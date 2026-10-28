//! Controlled first skating view over the retained core session.
//! Original A_RobsPark map meshes are preloaded through the inspection backend.
//! Spawn scheduling, PROXY activation and restored camera presentation remain pending.
use crate::{
    gpu_renderer::{GpuObject, GpuRenderer, RenderResult},
    live_skating_view::LiveSkatingView,
    render_inspection::Inspection,
    render_window::{RenderWindowClient, RenderWindowOptions},
    windows_juice_platform::{WindowsJuiceConnection, WindowsJuicePlatform},
    windows_pad_platform::WindowsPadPlatform,
};
use skate2_core::{
    JuiceHostOption, SelectedBootProgress, SelectedCommandStartPolicy,
    SelectedControlledBootPolicy, SelectedFrameNumerics, SelectedGestureAllocation,
    SelectedGravityPolicy, SelectedLiveSession, SelectedQueuedFrameOutcome,
    SelectedQueuedFramePolicy, SelectedRootWPolicy, SelectedSpawnPolicy,
    collection_resident_streamer::CollectionId,
    game::{CommonBackEndSeed, WorldStreamSpec},
    player::DEFAULT_MARQUEE_ID,
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::WorldStreamEffects,
};
use std::{
    convert::Infallible,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use winit::event::WindowEvent;

const FIXED_STEP: Duration = Duration::from_nanos(16_666_667);
type WindowsSession = SelectedLiveSession<fn() -> Result<u64, Infallible>, WindowsJuiceConnection>;

/// A process-local host identity mapping. Its value is not an Xbox thread ID.
fn host_animation_thread() -> Result<u64, Infallible> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    Ok(hasher.finish())
}

/// Observes the real stream callbacks without creating a second world owner.
struct StreamLog;
impl WorldStreamEffects for StreamLog {
    fn add_high_collection(&mut self, id: u64, x: u32, z: u32) {
        eprintln!("static-map test stream: high collection {id:016x} at ({x},{z})");
    }
    fn finalize_world_bounds(&mut self) {
        eprintln!("static-map test stream: world bounds finalized");
    }
}

pub(crate) fn run() -> RenderResult<()> {
    let args: Vec<_> = std::env::args().collect();
    let option = |name: &str| -> RenderResult<Option<&str>> {
        args.iter()
            .position(|argument| argument == name)
            .map(|index| {
                args.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("{name} requires a value").into())
            })
            .transpose()
    };
    let frame_limit = option("--live-frames")?.map(str::parse).transpose()?;
    let completion_limit: Option<u64> =
        option("--live-completions")?.map(str::parse).transpose()?;
    let capture = option("--live-capture")?.map(PathBuf::from);
    if frame_limit.is_some() && completion_limit.is_some() {
        return Err("choose --live-frames or --live-completions, not both".into());
    }
    if completion_limit == Some(0) {
        return Err("completion limit must be positive".into());
    }
    if capture.is_some() && frame_limit.is_none() && completion_limit.is_none() {
        return Err("--live-capture requires --live-frames or --live-completions".into());
    }
    let root = PathBuf::from(
        std::env::var_os("SKATE2_GAME_ROOT")
            .ok_or("set SKATE2_GAME_ROOT to the external original game directory")?,
    );
    let mounts = Arc::new(crate::mount_reference_archives(&root)?);
    let map = Inspection::load(&mounts)?;
    let mut pad = WindowsPadPlatform;
    let mut juice = WindowsJuicePlatform::new(None);
    let juice_ini = std::fs::read(root.join("data").join("juice").join("Juice_m.ini"))?;
    // The retained native-owner graph is large. Keep it at one stable heap
    // address before moving the window client into winit's event callback.
    let mut session = Box::new(WindowsSession::new(Arc::clone(&mounts), &root));
    let first_test = SelectedControlledBootPolicy {
        spawn_policy: SelectedSpawnPolicy::AuthoredSelectedLocation,
        marquee: DEFAULT_MARQUEE_ID,
        actor_flags: 1,
        capsule_resource_allocated: true,
        camera_set: CommonBackEndSeed::debug_reference().camera_set.to_owned(),
        terrain_catalog: WorldStreamCatalog::new(),
        terrain_spec: WorldStreamSpec::derive("BAM", None)?,
        unlock_seed: EnvironmentUnlockSeed {
            locations_lock_status_raw: 0,
            contexts_in_native_iteration_order: Vec::new(),
        },
        collection: CollectionId(0x823c_4449_fb31_63b4),
        dmo_update_in_parallel: false,
        arena_userdata: 0,
        presentation_lod: 0,
        presentation_submission_capacity: 65536,
        presentation_gather_capacity: 65536,
        presentation_provider: 0,
        // This still transports an opaque source W lane as a first-test
        // policy; it is not a recovered native skater-W producer.
        presentation_root_w: SelectedRootWPolicy::ReadSkaterOpaque { bits: 0 },
        camera_submit_mask: 3,
        gravity_policy: SelectedGravityPolicy::VaultSkateboardDefault,
        command_start: SelectedCommandStartPolicy::NativeStaticFields,
    };
    let progress = session.try_begin_controlled(
        &mut pad,
        &mut juice,
        Some(&juice_ini),
        JuiceHostOption::DisableBeforeStart,
        host_animation_thread as fn() -> Result<u64, Infallible>,
        StreamLog,
        first_test,
        &mut |_, _, _| Ok::<_, Infallible>(SelectedGestureAllocation::Host),
        &mut || panic!("source spawn orthogonality assertion"),
        &mut || panic!("hat-enable host answer is not reconstructed"),
    )?;
    eprintln!(
        "CONTROLLED FIRST TEST: boot={progress:?}; original static RobsPark map, diagnostic materials, provisional CameraMan projection, native spawn/status and enabled PROXY activation pending"
    );
    let completed_counter = Arc::new(AtomicU64::new(0));
    let presented_completion_counter = Arc::new(AtomicU64::new(0));
    let client = LiveSkatingClient {
        view: LiveSkatingView::new(map),
        session,
        pad,
        ready: false,
        last_progress: Some(progress),
        previous: Instant::now(),
        accumulator: Duration::ZERO,
        completed_frames: 0,
        completion_limit,
        completed_counter: Arc::clone(&completed_counter),
        presented_completion_counter: Arc::clone(&presented_completion_counter),
        no_command_reported: false,
        // Wait for the window's actual focus event before admitting focus-gain input.
        focused: false,
    };
    crate::render_window::run_with_options(
        client,
        RenderWindowOptions {
            frame_limit,
            capture,
            title: Some(
                "Skate 2 - CONTROLLED gamepad test / static original map / provisional camera after first frame",
            ),
        },
    )?;
    let completed = completed_counter.load(Ordering::Relaxed);
    if frame_limit.is_some() && completed == 0 {
        return Err("bounded live test presented no completed selected skating frame".into());
    }
    if let Some(limit) = completion_limit {
        let presented_completion = presented_completion_counter.load(Ordering::Relaxed);
        if presented_completion < limit {
            return Err(format!(
                "bounded live test prepared {completed} and presented {presented_completion} of {limit} completed selected skating frames"
            )
            .into());
        }
    }
    Ok(())
}

struct LiveSkatingClient {
    view: LiveSkatingView,
    session: Box<WindowsSession>,
    pad: WindowsPadPlatform,
    ready: bool,
    last_progress: Option<SelectedBootProgress>,
    previous: Instant,
    accumulator: Duration,
    completed_frames: u64,
    completion_limit: Option<u64>,
    completed_counter: Arc<AtomicU64>,
    presented_completion_counter: Arc<AtomicU64>,
    no_command_reported: bool,
    focused: bool,
}

impl RenderWindowClient for LiveSkatingClient {
    fn initialize(&mut self, renderer: &GpuRenderer) -> RenderResult<Vec<GpuObject>> {
        self.view.initialize(renderer)
    }

    fn window_event(&mut self, event: &WindowEvent) -> RenderResult<()> {
        if let WindowEvent::Focused(focused) = event {
            self.focused = *focused;
        }
        Ok(())
    }

    fn completion_reached(&self) -> bool {
        self.completion_limit
            .is_some_and(|limit| self.completed_frames >= limit)
    }

    fn completion_bound_active(&self) -> bool {
        self.completion_limit.is_some()
    }

    fn record_presented_completion(&mut self) {
        self.presented_completion_counter
            .store(self.completed_frames, Ordering::Relaxed);
    }

    fn frame(
        &mut self,
        renderer: &GpuRenderer,
        objects: &mut Vec<GpuObject>,
        now: Instant,
    ) -> RenderResult<()> {
        // A capture may need another redraw after a surface timeout. Retain
        // the completed visual objects and never advance to the next tick.
        if self.completion_reached() {
            self.previous = now;
            return Ok(());
        }
        if !self.ready {
            let progress = self.session.try_poll_controlled_boot()?;
            if self.last_progress != Some(progress) {
                eprintln!("controlled first-test boot={progress:?}");
                self.last_progress = Some(progress);
            }
            if progress != SelectedBootProgress::ControlledFrameReady {
                self.previous = now;
                return Ok(());
            }
            self.ready = true;
            self.previous = now;
            self.accumulator = FIXED_STEP;
            eprintln!(
                "controlled frame ready; fixed host simulation step 1/60, source-selected original-data input/Actor/physics/camera/palette; camera view and map shading remain diagnostic"
            );
        } else {
            self.accumulator += now.duration_since(self.previous);
            self.previous = now;
        }
        while self.accumulator >= FIXED_STEP {
            self.accumulator -= FIXED_STEP;
            let outcome = self.session.try_tick(
                &mut self.pad,
                SelectedQueuedFramePolicy {
                    dt: 1.0 / 60.0,
                    lod: 0,
                    wait_for_bs_job: true,
                    can_simulate: true,
                    has_focus: self.focused,
                    ground_group: 0,
                    allow_multiple_writes: true,
                    spy_flags: 4,
                    numerics: SelectedFrameNumerics::ExistingHostApproximation,
                    physics_flags3: true,
                    park_grind_preparation: true,
                    exclude_grind_fill: true,
                    no_pipeline_culling: true,
                },
                &mut || Err::<i32, _>("hands clamp service has no first-test host answer"),
            )?;
            match outcome {
                SelectedQueuedFrameOutcome::NoCommands {
                    update_mask,
                    delivered,
                } => {
                    if !self.no_command_reported {
                        eprintln!(
                            "controlled input produced no simulation command: mask={update_mask} delivered={delivered}"
                        );
                        self.no_command_reported = true;
                    }
                }
                SelectedQueuedFrameOutcome::Completed {
                    update_mask,
                    delivered,
                    missing_receivers,
                } => {
                    if missing_receivers != 0 {
                        return Err(format!(
                            "selected queued frame missed {missing_receivers} input receivers"
                        )
                        .into());
                    }
                    let visual = self.session.try_visual_snapshot()?;
                    self.view.update(
                        renderer,
                        objects,
                        now,
                        visual.provisional_clip_from_world,
                        visual.palette,
                        visual.materials,
                    )?;
                    self.completed_frames += 1;
                    self.completed_counter
                        .store(self.completed_frames, Ordering::Relaxed);
                    self.no_command_reported = false;
                    if self.completed_frames == 1 || self.completed_frames.is_multiple_of(60) {
                        eprintln!(
                            "controlled frame {}: queued mask={update_mask} delivered={delivered} CameraMan update={}; static map and selected character share this producer projection",
                            self.completed_frames, visual.camera_update_counter
                        );
                    }
                    // The window has not presented this completed frame yet.
                    // Stop catch-up here so frame N+1 cannot fail first.
                    if self.completion_reached() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
