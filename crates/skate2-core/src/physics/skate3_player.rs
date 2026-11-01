//! Live-player owner for the explicitly requested PC simulation mod.
//!
//! The actual Skate 2 physical assembly owns this selected solver and controller.
//! The original solver slots stay dormant while completed, read-only views
//! serve the character, world queries and replay. No second game runtime runs.
use skate3_core::{input::xbox::XboxState, physics::{board::BodyId, skeleton_animation_record::{AnimationPartTransform, compose_affine}}};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_rigid_body_pool::SimulationRigidBodyObservation,
    static_ground_pool::{ResidentGroundGeometry, StaticGroundPool},
};
use crate::skate3_native::{
    BodyOutput, BodyReaction, SkeletonPartOutput, FrameOutput, RawController, Scene, SceneTriangle, SceneTriangleFeatures,
};

#[path = "skate3_terrain.rs"]
pub(crate) mod terrain;
#[path = "skate3_grinds.rs"]
pub(crate) mod grinds;
#[path = "skate3_grabs.rs"]
mod grabs;
#[path = "player/skate3_exit.rs"]
pub(crate) mod exit;
#[path = "player/skate3_tuning.rs"]
pub(crate) mod tuning;
#[path = "player/skate3_actor_input.rs"]
pub(crate) mod actor_input;
#[path = "player/skate3_camera_rate.rs"]
mod camera_rate;
#[path = "player/skate3_spawn.rs"]
mod spawn;

pub(crate) static PLAYER_ASSETS: &[u8] = include_bytes!(env!("SKATE3_EMBEDDED_PACK"));

/// Immutable data from this executable's fixed asset pack. Runtime profile
/// overrides clone this source; no mutable tuning or query state is shared.
pub(crate) fn embedded_collections()->Result<std::sync::Arc<skate_data::collections::Collections>,String> {
    static DATA:std::sync::OnceLock<Result<std::sync::Arc<skate_data::collections::Collections>,String>>=std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let started=std::time::Instant::now();
        let root=skate_data::asset_io::mount(PLAYER_ASSETS).map_err(|error|error.to_string())?;
        let mount_ms=started.elapsed().as_secs_f64()*1000.;
        let collections_started=std::time::Instant::now();
        let data=std::sync::Arc::new(skate_data::collections::Collections::load(root)?);
        crate::diagnostics::test_log::event("skate3_embedded_collections",format_args!(
            "immutable=true mount_ms={mount_ms:.3} collections_ms={:.3} total_ms={:.3}",
            collections_started.elapsed().as_secs_f64()*1000.,started.elapsed().as_secs_f64()*1000.));
        Ok(data)
    }).as_ref().map(std::sync::Arc::clone).map_err(Clone::clone)
}

pub(crate) fn embedded_animation_source()->Result<std::sync::Arc<crate::skate3_native::skater_animation::AnimationSource>,String> {
    let root=skate_data::asset_io::mount(PLAYER_ASSETS).map_err(|e|e.to_string())?;
    Ok(std::sync::Arc::clone(&player_assets(root)?.animation))
}

struct PreparedPlayerTick {
    actions:skate3_core::input::gameplay_map::GameplayActions,
    input_available:bool,
    frame:Option<super::frame::PreparedFrame>,
}

struct PlayerAssets {
    graphs: std::sync::Arc<crate::skate3_native::graph_runtime::StockGraphs>,
    animation: std::sync::Arc<crate::skate3_native::skater_animation::AnimationSource>,
}
fn player_assets(root: &std::path::Path) -> Result<&'static PlayerAssets, String> {
    // Only immutable graph/bank/pose data is shared. Every player still owns
    // its own graph state, physics, camera, input history and scoring runtime.
    static ASSETS: std::sync::OnceLock<Result<PlayerAssets, String>> = std::sync::OnceLock::new();
    ASSETS.get_or_init(|| {
        let manifest = skate_data::GameAssets::load_physics(root).map_err(|error|error.to_string())?;
        Ok(PlayerAssets {
            graphs: std::sync::Arc::new(crate::skate3_native::graph_runtime::StockGraphs::load(root, &manifest)?),
            animation: crate::skate3_native::skater_animation::AnimationSource::load(root)?,
        })
    }).as_ref().map_err(Clone::clone)
}

/// The two engines' authored mass frames can place their mirror COM at
/// different offsets. Transport the solved rigid velocity to that actual point.
fn velocity_at(source: &crate::skate3_native::BodyOutput, target: &[f32; 16]) -> [f32; 3] {
    let r: [f32; 3] = std::array::from_fn(|i| target[12 + i] - source.centre_of_mass[i]);
    let w = source.omega;
    [
        source.velocity[0] + w[1] * r[2] - w[2] * r[1],
        source.velocity[1] + w[2] * r[0] - w[0] * r[2],
        source.velocity[2] + w[0] * r[1] - w[1] * r[0],
    ]
}

#[derive(Clone, Copy)]
struct HostTeleport {
    transform: [f32; 16],
    on_board: bool,
    generation: Option<u32>,
    foot_forward: Option<u32>,
}

/// One-time transfer at solver selection, using the current actor's published
/// physical root and velocity. Each subsequent tick has exactly one solver.
pub(crate) struct PlayerEntry {
    pub transform: [f32; 16],
    pub velocity: [f32; 3],
    pub on_board: bool,
    pub state: u32,
    pub holding_board: bool,
    pub filtered: Option<skate3_core::physics::filtered_state::FilteredStateOutput>,
    pub camera_basis: Option<[[f32; 3]; 3]>,
}

struct OffboardReplacement { saw_offboard: bool, pending_toggle: bool }

pub(crate) struct Skate3Player {
    root: std::path::PathBuf,
    graphs: std::sync::Arc<crate::skate3_native::graph_runtime::StockGraphs>,
    physics: super::Skate3PhysicsState,
    skater: super::Skate3ControllerState,
    controls: super::PlayerControls,
    input: crate::skate3_native::input::ControllerInput,
    entry_camera_basis: Option<[[f32; 3]; 3]>,
    camera: crate::skate3_native::camera::CameraRuntime,
    airtime_slow_motion: bool,
    hips_body: usize,
    grinds: grinds::HostGrinds,
    grabs: grabs::HostGrabs,
    host_cvars: Option<crate::input_cvar_runtime::InputCvarReferences>,
    host_bodies: crate::simulation_assembly_backend::constraint_session::skate3_world::HostBodies,
    sources: Vec<ResidentGroundGeometry>,
    terrain: terrain::PreparedTerrain,
    board: Vec<SimulationRigidBodyObservation>,
    board_frames: Vec<[f32; 16]>,
    skeleton_frames: Vec<[f32; 16]>,
    skeleton: Vec<SimulationRigidBodyObservation>,
    skeleton_bones: Vec<(String, [f32; 16])>,
    output: std::sync::Arc<FrameOutput>,
    queued_teleport: Option<HostTeleport>,
    awaiting_teleport: Option<HostTeleport>,
    completed_teleport: Option<HostTeleport>,
    advanced_this_tick: bool,
    prepared_tick: Option<PreparedPlayerTick>,
    waiting_for_terrain: bool,
    teleport_generation: Option<u32>,
    world_body_count: usize,
    offboard_replacement: Option<OffboardReplacement>,
    actor_input: actor_input::HostActorInput,
    input_focused: bool,
    /// PC: bridge single-tick misses while holding the prop menu open.
    interactable_hold: Option<(u32, u32)>,
}

const INTERACTABLE_HOLD_TICKS: u32 = 30;
fn hold_interactable(hold: &mut Option<(u32, u32)>, published: &mut (u32, bool), grab_world: bool) {
    if !grab_world {
        *hold = None;
    } else if published.1 {
        *hold = Some((published.0, 0));
    } else if let Some((object, missed)) = hold {
        *missed += 1;
        if *missed > INTERACTABLE_HOLD_TICKS { *hold = None; }
        else { *published = (*object, true); }
    }
}

// Diagnostic-only capture of the already decoded static collision triangles.
fn diagnostic_ground_export(sources: &[ResidentGroundGeometry]) {
    use std::io::Write;
    let Some(path)=std::env::var_os("SKATE2_DIAGNOSTIC_GROUND_EXPORT") else {return;};
    let result=(||->std::io::Result<()> {
        let path=std::path::PathBuf::from(path);
        if let Some(parent)=path.parent(){std::fs::create_dir_all(parent)?;}
        let mut out=std::io::BufWriter::new(std::fs::File::create(path)?);
        for source in sources {
            for triangle in &source.triangles {
                for point in triangle.vertices {for coordinate in point {out.write_all(&coordinate.to_le_bytes())?;}}
            }
        }
        out.flush()
    })();
    if let Err(error)=result {eprintln!("ground diagnostic export failed: {error}");}
}

fn scene(sources: &[ResidentGroundGeometry], spawn: [f32; 3]) -> Result<Scene, String> {
    let mut triangles = Vec::new();
    for source in sources {
        if source.triangles.len() != source.features.len() {
            return Err("Skate 2 resident ground has incomplete native features".into());
        }
        for (triangle, feature) in source.triangles.iter().zip(&source.features) {
            let surface = triangle.surface_id.unwrap_or(0);
            triangles.push(SceneTriangle {
                points: triangle.vertices,
                // S3 WorldLineHit.tag is the packed RW surface itself. S2's
                // GP user tag (surface << 16 | group) has a different contract.
                surface: u32::from(surface),
                packed_surface: surface,
                native_features: Some(SceneTriangleFeatures {
                    flags: feature.flags,
                    edge_cosines: feature.edge_cosines,
                }),
                native_edges: None,
            });
        }
    }
    if triangles.is_empty() {
        return Err("Skate 3 player requires actual Skate 2 terrain".into());
    }
    Ok(Scene {
        spawn,
        triangles,
        ..Default::default()
    })
}

impl Skate3Player {
    pub(crate) fn load(
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        profile: &str,
        entry: Option<PlayerEntry>,
    ) -> Result<Self, String> {
        let board_bodies = world
            .board_publication(board)
            .map_err(|e| e.to_string())?
            .rigid_bodies()
            .to_vec();
        let skeleton_bodies = world
            .skeleton_publication(skeleton)
            .map_err(|e| e.to_string())?
            .rigid_bodies()
            .to_vec();
        if board_bodies.len() != 7 || skeleton_bodies.len() != 24 {
            return Err(
                "Skate 3 player requires the retained seven-part board and 24-part skeleton".into(),
            );
        }
        let deck = world
            .board(board)
            .map_err(|e| e.to_string())?
            .try_live_part_transform_host(world.backend(), 6)
            .map_err(|e| e.to_string())?;
        let identity = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        let board_frames = (0..7)
            .map(|i| {
                world
                    .board(board)
                    .map_err(|e| e.to_string())?
                    .try_skate3_mirror_body_transform(world.backend(), i, identity)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let skeleton_frames = (0..24)
            .map(|i| {
                world
                    .skeleton(skeleton)
                    .map_err(|e| e.to_string())?
                    .session()
                    .try_skate3_mirror_body_transform(world.backend(), i, identity)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let velocity = world
            .backend()
            .try_external_body_snapshot(board_bodies[6])?
            .velocity;
        let sources = ground.resident_geometry().map_err(|e| e.to_string())?;
        diagnostic_ground_export(&sources);
        let root = skate_data::asset_io::mount(PLAYER_ASSETS).map_err(|e| e.to_string())?;
        let scene = scene(&sources, [deck[12], deck[13], deck[14]])?;
        let _asset_load = crate::diagnostics::test_log::span("skate3_asset_graph_load");
        let difficulty = crate::skate3_native::difficulty::Difficulty::parse(profile)?;
        let assets = player_assets(root)?;
        let graphs = std::sync::Arc::clone(&assets.graphs);
        let mut physics = super::Skate3PhysicsState::load_with_host_scene(root, &scene, difficulty)?;
        let terrain=terrain::PreparedTerrain::seed(&sources,&physics.world,
            physics.settings.floor_material)?;
        let mut skater = super::Skate3ControllerState::load_for_world(root, &graphs, &physics, difficulty.profile_key(),
            Some(std::sync::Arc::clone(&assets.animation)))?;
        let controls = super::PlayerControls::load(root)?;
        let camera = crate::skate3_native::camera::CameraRuntime::load(root)?;
        let hips_bone = skater.animation.evaluator.frames.bone_names.iter()
            .position(|name| name.eq_ignore_ascii_case("Hips")).ok_or("Native movement rig lacks Hips")?;
        let hips_body = skater.animated_skeleton.bone_indices.iter().position(|&bone| bone == hips_bone)
            .ok_or("Native movement physical rig lacks Hips body")?;
        let entry = entry.unwrap_or(PlayerEntry { transform: deck, velocity, on_board: true, state: 100, holding_board: false, filtered: None, camera_basis: None });
        if !entry.on_board {
            // The first graph tick must see the outgoing solver's completed
            // offboard observations. Zero/ground constructor observations would
            // select an onboard graph and immediately undo the manual reset.
            // These facts seed graph input only; the source's ordinary reset,
            // state selection and solver still own the first physical tick.
            skater.player_input.physical.state.category_12 = 500;
            skater.player_input.physical.state.state_16 = entry.state;
            skater.player_input.physical.off_board.flag_311 = u8::from(entry.holding_board);
            let filtered = entry.filtered.ok_or("Offboard entry requires the outgoing filtered publication")?;
            skater.player_input.physical.filtered_state_0 = filtered.category as u32;
            skater.player_state.filtered_output = Some(filtered);
            skater.player_state.state_flags[75 - 52] = true;
        }
        let entry_target = spawn::target(&physics.world, unflatten(entry.transform), entry.on_board)?;
        skater.player_input.request_teleport_ex(entry_target, Some(entry.velocity))?;
        skater.teleport_state.request_manual(entry_target, entry.on_board);
        let mut grinds = grinds::HostGrinds::default();
        grinds.sync(world, &mut physics, &mut skater)?;
        let data = &world
            .skeleton(skeleton)
            .map_err(|e| e.to_string())?
            .state()
            .binding
            .body()
            .data;
        let skeleton_bones = data
            .bone_ids()
            .iter()
            .enumerate()
            .map(|(part, &bone)| {
                (
                    data.hierarchy().bone_names[bone].clone(),
                    data.bone_to_volume()[part],
                )
            })
            .collect();
        let bodies: Vec<_> = board_bodies
            .iter()
            .chain(&skeleton_bodies)
            .copied()
            .collect();
        let output = std::sync::Arc::new(export_frame(&physics, &skater, &camera, hips_body));
        // Validate names before taking solver ownership. Positional guesses
        // between the two rigs are deliberately not admitted.
        let result = Self {
            root: root.to_owned(), graphs, physics, skater, controls,
            input: Default::default(), entry_camera_basis: entry.camera_basis, camera, airtime_slow_motion: false, hips_body,
            grinds,
            grabs: Default::default(), host_cvars: None,
            host_bodies: Default::default(),
            sources, terrain,
            board: board_bodies,
            board_frames,
            skeleton_frames,
            skeleton: skeleton_bodies,
            skeleton_bones,
            output,
            queued_teleport: None,
            awaiting_teleport: Some(HostTeleport {
                transform: entry.transform,
                on_board: entry.on_board,
                generation: None,
                foot_forward: None,
            }),
            completed_teleport: None,
            advanced_this_tick: false,
            prepared_tick: None,
            waiting_for_terrain: false,
            teleport_generation: None,
            world_body_count: 0,
            offboard_replacement: None,
            actor_input: Default::default(),
            interactable_hold: None, input_focused: true,
        };
        result.character_physical_pose()?;
        world.backend_mut().try_select_external_player(&bodies)?;
        Ok(result)
    }

    /// Moving bodies answered by their authored mesh, and grind primitives in total.
    #[cfg(test)]
    pub(crate) fn test_prop_world_counts(&self) -> (usize, usize) {
        (self.physics.network_proxies.meshes.len(), self.physics.grind_world.primitives().len())
    }

    /// Every Skate 3 grind primitive's start and end, in provider order.
    #[cfg(test)]
    pub(crate) fn test_grind_segments(&self) -> Vec<([f32; 4], [f32; 4])> {
        self.physics.grind_world.primitives().iter().map(|p| (p.start, p.end)).collect()
    }

    /// Host runs this after its world solve, before source-player stepping.
    pub(crate) fn sync_world_bodies(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        collide_other_skaters: bool,
    ) -> Result<(), String> {
        world.try_refresh_skate3_sources()?;
        let bodies = world.try_skate3_bodies(
            &mut self.host_bodies, collide_other_skaters, self.physics.settings.floor_material)?;
        self.world_body_count = bodies.solids.len();
        self.set_moving_mesh_bodies(bodies.solids, bodies.meshes)
    }



    pub(crate) fn prepare_animation_tick(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        dt: f32,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) -> Result<bool, String> {
        if self.prepared_tick.is_some(){return Err("Source animation prepared twice in one host tick".into());}
        if !dt.is_finite() || dt <= 0. {
            return Err("Invalid host simulation interval".into());
        }
        let _test_stage = crate::diagnostics::test_log::span("skate3_player_animation_prepare");
        self.completed_teleport = None;
        self.advanced_this_tick = false;
        //A retained player can outlive the streamed collision during travel.
        //Do not decode/replace it with an empty world or consume a queued
        //marker request before the destination has resident collision.
        if ground.published_volume_count() == 0 {
            if !self.waiting_for_terrain {
                crate::diagnostics::test_log::event("skate3_scene_wait",format_args!(
                    "tick={} reason=no_resident_terrain phase=player_step queued_generation={:?} root={:?}",
                    self.output.tick,self.queued_teleport.and_then(|target|target.generation),self.output.root_transform[3]));
            }
            self.waiting_for_terrain = true;
            return Ok(false);
        }
        if self.waiting_for_terrain {
            crate::diagnostics::test_log::event("skate3_scene_resume",format_args!(
                "tick={} phase=player_step volumes={}",self.output.tick,ground.published_volume_count()));
            self.waiting_for_terrain = false;
        }
        // Dynamic props (Object Dropper and city DMOs) are moving bodies whose
        // authored meshes answer ground queries each tick (sync_world_bodies);
        // only the streamed static ground is rebuilt here.
        if !ground.matches_resident_geometry(&self.sources) {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_refresh");
            let sources = {
                let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_decode");
                ground.resident_geometry_reusing(std::mem::take(&mut self.sources)).map_err(|e| e.to_string())?
            };
            let prepared_world={
                let _stage=crate::diagnostics::test_log::timed_span("skate3_terrain_world_build");
                self.terrain.world(&sources,self.physics.settings.floor_material)?
            };
            self.replace_collision_world(prepared_world)?;
            crate::diagnostics::test_log::event("skate3_terrain_refresh", format_args!(
                "tick={} sources={} triangles={}", self.output.tick, sources.len(),
                sources.iter().map(|source|source.triangles.len()).sum::<usize>()));
            diagnostic_ground_export(&sources);
            self.sources = sources;
        }
        {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_grind_refresh");
            self.grinds.sync(world, &mut self.physics, &mut self.skater)?;
        }
        {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_grab_refresh");
            self.physics.offboard_grab_scene=self.grabs.sync(world,&self.physics.world,self.host_cvars.as_ref())?;
        }
        // A constructor reset cannot consume the real dismount edge. Retain it
        // through that reset, then deliver it once to the source input graph.
        let mut raw = raw;
        if let Some(replacement) = &mut self.offboard_replacement {
            if replacement.pending_toggle {
                if let Some(pad) = &mut raw { pad.buttons &= !(1 << 15); }
                if self.skater.player_input.pending_teleport().is_none()
                    && self.skater.teleport_state.manual_on_board().is_none()
                    && self.output.state == 100 {
                    if let Some(pad) = &mut raw {
                        pad.buttons |= 1 << 15;
                        replacement.pending_toggle = false;
                    }
                }
            }
        }
        let raw = raw.map(|pad| RawController {
            buttons: pad.buttons,
            triggers: [pad.left_trigger, pad.right_trigger],
            left: [pad.left_x, pad.left_y],
            right: [pad.right_x, pad.right_y],
        });
        self.collect_controller(raw);
        // The actual game's simulation timer owns this physical player, just
        // as it owns the other players, cars, score and career state. A second
        // accumulator here could repeat/skip publications or burst after a
        // slow-motion rate change. Each admitted world tick advances it once.
            // A newer explicit Actor reset supersedes an old manual board-family
            // continuation once its input matrix has been consumed. Waiting for
            // that continuation can deadlock online travel after an offboard reset.
            if self.skater.player_input.pending_teleport().is_none()
                // An onboard constructor/reset reaches Ground in two ticks.
                // Let its existing Teleport leaf exit before replacing it.
                // Offboard continuations remain supersedable (pinned online
                // actors can prevent that dismount from ever completing).
                && self.skater.teleport_state.manual_on_board() != Some(true)
                && let Some(teleport) = self.queued_teleport.take()
            {
                // Restore each native Actor reset's requested foot in the same
                // graph dispatch that consumes its transform and board state.
                if let Some(foot) = teleport.foot_forward {
                    self.skater.animation.restore_foot_forward(foot != 0);
                    // restore_foot_forward previews the native reset pose with
                    // a copied request; the graph still consumes its original.
                    // This also covers a new target during an active Teleport.
                }
                crate::diagnostics::test_log::event("skate3_spawn_apply", format_args!(
                    "tick={} generation={:?} foot={:?} natural={} on_board={} transform={:?}",
                    self.physics.ticks,teleport.generation,teleport.foot_forward,
                    self.skater.animation.natural_stance(),teleport.on_board,teleport.transform));
                self.host_teleport(unflatten(teleport.transform), teleport.on_board)?;
                self.awaiting_teleport = Some(teleport);
            }
        let (actions,input_available)=self.sample_solver_input()?;
        let frame=super::frame::prepare(&mut self.physics,&mut self.skater,&mut self.controls,&self.graphs,&mut self.camera)?;
        self.prepared_tick=Some(PreparedPlayerTick {actions,input_available,frame});
        Ok(true)
    }

    pub(crate) fn advance(
        &mut self,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        dt: f32,
        raw: Option<crate::input_pad::XboxPadSample>,
    ) -> Result<(), String> {
        let _test_stage=crate::diagnostics::test_log::span("skate3_player_solve");
        if self.prepared_tick.is_none() && !self.prepare_animation_tick(world,ground,dt,raw)? {return Ok(());}
        {
            let PreparedPlayerTick {mut actions,input_available,frame}=self.prepared_tick.take().ok_or("Missing prepared source tick")?;
            if let Some(prepared)=frame {
                let _stage=crate::diagnostics::test_log::timed_span("skate3_source_tick");
                super::frame::finish(&mut self.physics,&mut self.skater,&mut self.controls,&mut actions,
                    input_available,&mut self.camera,prepared)?;
            }
            let mut output=export_frame(&self.physics,&self.skater,&self.camera,self.hips_body);
            hold_interactable(&mut self.interactable_hold, &mut output.gameplay.interactable_object,
                self.skater.player_input.processed.flags_2476 & (1 << 22) != 0);
            self.output=std::sync::Arc::new(output);
            self.advanced_this_tick = true;
            if !self.teleport_pending() {
                self.completed_teleport = self.awaiting_teleport.take().or(self.completed_teleport);
                if let Some(target) = self.completed_teleport {
                    let requested = unflatten(target.transform);
                    let bone = |name: &str| self.output.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name))
                        .map(|i|self.output.world_bones[i][3]);
                    let feet = bone("LeftToeBase").zip(bone("RightToeBase")).map(|(left,right)| {
                        let sign=if target.foot_forward==Some(0) {-1.0} else {1.0};
                        sign*((left[0]-right[0])*requested[2][0]+(left[2]-right[2])*requested[2][2])
                    });
                    crate::diagnostics::test_log::event("skate3_spawn_complete",format_args!(
                        "tick={} generation={:?} foot={:?} actual_foot={} natural={} on_board={} requested_at={:?} root_at={:?} board_at={:?} leading_foot_projection={feet:?}",
                        self.output.tick,target.generation,target.foot_forward,self.skater.animation.checkpoint_stance(),
                        self.skater.animation.natural_stance(),target.on_board,requested[2],self.output.effective_root_transform()[2],self.output.board_transform[2]));
                }
            }
            let reactions = self.take_body_reactions();
            world.try_apply_skate3_reactions(&self.host_bodies, reactions)?;
            // Camera effects are a separate mod preference. Suppression leaves
            // native story/script rates alone; no baseline reset is sent.
            camera_rate::submit(&world.sim_rate, &self.camera.simulation_rate_requests,
                self.airtime_slow_motion, self.output.tick)?;
        }
        if crate::diagnostics::test_log::enabled() {
            let output = &self.output;
            let spawn_frames=self.skater.player_input.processed.frames_since_teleport_2584;
            if spawn_frames<12 && !output.offboard {
                let rendered=["Left_WheelFront","Right_WheelFront","Left_WheelBack","Right_WheelBack"].map(|name|
                    output.bone_names.iter().position(|bone|bone.eq_ignore_ascii_case(name)).map(|bone|output.world_bones[bone][3]));
                crate::diagnostics::test_log::event("skate3_spawn_wheels",format_args!(
                    "tick={} frames_since_reset={spawn_frames} state={} generation={:?} physical_gaps_m={:?} rendered_centres={rendered:?} contacts={:?}",
                    output.tick,output.state,self.awaiting_teleport.and_then(|target|target.generation),
                    self.test_spawn_wheel_gaps(),output.physical.collision.wheel_contact_3296_3299));
            }
            let finite_pose = output.world_bones.iter().flatten().flatten().all(|v| v.is_finite());
            crate::diagnostics::test_log::event("skate3_physics", format_args!("tick={} state={} category={} offboard={} wiping_out={} root={:?} root_velocity={:?} root_omega={:?} board={:?} board_velocity={:?} centre_of_mass={:?} collision={:?} ground={:?} bones={} finite_pose={} teleport_pending={} mirrored_world_bodies={} rails={}",
                output.tick, output.state, output.category, output.offboard, output.wiping_out,
                output.root_transform, output.root_velocity, output.root_omega, output.board_transform,
                output.board_velocity, output.centre_of_mass, output.physical.collision,
                output.physical.ground, output.world_bones.len(), finite_pose,
                self.teleport_pending(), self.world_body_count, self.grind_primitive_count()));
            if output.state==601 || (output.state==300 && output.tick%6==0) {
                crate::diagnostics::test_log::event("skate3_control_path",format_args!(
                    "tick={} state={} camera_axes={:?} derived_left={:?} state83={} body_adjust={:?} bail_control={:?}",
                    output.tick,output.state,self.controls.sampled_camera_axes(),
                    [self.controls.controller.words()[7],self.controls.controller.words()[8]].map(f32::from_bits),
                    output.gameplay.state_flags[83-52],self.skater.animation_input.extra.body_adjust,
                    self.skater.animation_input.extra.wipeout_control));
            }
            if output.state==300 && output.tick%6==0 {
                let w=&self.skater.wipeout_state.state;
                crate::diagnostics::test_log::event("skate3_hom_control",format_args!(
                    "tick={} online={} focused={} profile={} gesture={:?} control={:?} response={} responses={} speed={} over={} flags2484={:08x}",
                    output.tick,self.skater.scoring.online_game,self.input_focused,w.profile,
                    self.skater.animation_input.extra.wipeout_gesture,self.skater.animation_input.extra.wipeout_control,
                    w.response_strength(),w.response_count,w.maximum_speed,w.over,self.skater.player_input.processed.flags_2484));
            }
            if output.tick % 60 == 0 {
                crate::diagnostics::test_log::event("skate3_physics_full", format_args!("tick={} input={:?}", output.tick, output.physical));
            }
            if !finite_pose { crate::diagnostics::test_log::anomaly("nonfinite_animation", &format!("tick={} state={} category={}", output.tick, output.state, output.category)); }
        }
        self.publish_body_views(world)
    }

    fn character_physical_pose(&self) -> Result<Vec<([f32; 16], &crate::skate3_native::BodyOutput)>, String> {
        self.skeleton_bones
            .iter()
            .map(|(name, bone_to_volume)| {
                let part = self
                    .output
                    .skeleton_parts
                    .iter()
                    .find(|part| part.bone_name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| format!("Skate 3 pose lacks Skate 2 physical bone {name}"))?;
                let bone = flatten(part.bone_transform);
                Ok((
                    crate::skateboard_spawn_math::skateboard_spawn_compose(&bone, bone_to_volume),
                    &part.body,
                ))
            })
            .collect()
    }

    fn publish_body_views(&self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        let skeleton = self.character_physical_pose()?;
        let mut views = Vec::with_capacity(self.board.len() + self.skeleton.len());
        // Board part matrices/rates are the solved seven native bodies, not a
        // rigid board transform applied to independently rotating trucks.
        for ((body, frame), solved) in self
            .board
            .iter()
            .zip(&self.board_frames)
            .zip(&self.output.board_parts)
        {
            let transform = crate::skateboard_spawn_math::skateboard_spawn_compose(
                &flatten(solved.transform),
                frame,
            );
            views.push((*body, transform, velocity_at(solved, &transform), solved.omega));
        }
        for ((body, frame), (transform, solved)) in self
            .skeleton
            .iter()
            .zip(&self.skeleton_frames)
            .zip(skeleton)
        {
            let transform =
                crate::skateboard_spawn_math::skateboard_spawn_compose(&transform, frame);
            views.push((*body, transform, velocity_at(solved, &transform), solved.omega));
        }
        world.backend_mut().try_publish_mode_body_views(&views)
    }

    pub(crate) fn grind_primitive_count(&self) -> usize {
        self.physics.grind_world.primitives().len()
    }
    pub(crate) fn queue_teleport(
        &mut self,
        transform: [f32; 16],
        on_board: bool,
        generation: u32,
        foot_forward: u32,
    ) -> Result<(), String> {
        if foot_forward > 1 { return Err("Invalid native teleport leading foot".into()); }
        if transform.iter().any(|v| !v.is_finite()) {
            return Err("Non-finite host teleport".into());
        }
        self.queued_teleport = Some(HostTeleport {
            transform,
            on_board,
            generation: Some(generation),
            foot_forward: Some(foot_forward),
        });
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn test_offboard_motion_trace(&self)->String {
        let s=&self.skater;
        let hands=["LEFTARM","LEFTFOREARM","LEFTHAND","RIGHTARM","RIGHTFOREARM","RIGHTHAND"].map(|name|{
            let bone=s.animation.evaluator.frames.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name)).unwrap();
            let physical=s.animated_skeleton.bone_indices.iter().position(|b|*b==bone).unwrap();
            let world=compose_affine(&s.animated_skeleton.roots.animation_to_world,&s.animation.packet.hierarchy[bone]);
            let actual=self.output.skeleton_parts[physical].bone_transform;
            let error=(0..3).map(|i|(actual[3][i]-world[3][i]).powi(2)).sum::<f32>().sqrt();
            (name,world,actual,error)
        });
        let danger=s.animation.attributes.entries().iter().filter(|a|
            a.name==skate3_core::animation::skeleton_input::name::encode(b"DangerZone"))
            .copied().collect::<Vec<_>>();
        let ik=["LEFTARM","LEFTFOREARM","LEFTHAND","RIGHTARM","RIGHTFOREARM","RIGHTHAND"].map(|name| {
            let bone=s.animation.evaluator.frames.bone_names.iter().position(|n|n.eq_ignore_ascii_case(name)).unwrap();
            let part=s.animated_skeleton.bone_indices.iter().position(|b|*b==bone).unwrap();
            let target=compose_affine(&s.animated_skeleton.roots.animation_to_world,&s.skeleton_input.drive_frames[part]);
            let physical=s.skeleton.record.pose[part];
            let distance=(0..3).map(|i|(physical[3][i]-target[3][i]).powi(2)).sum::<f32>().sqrt();
            let max_basis=(0..3).flat_map(|i|(0..3).map(move |j|(physical[i][j]-target[i][j]).abs())).fold(0.0f32,f32::max);
            let drive=s.skeleton_drives.bones[part].as_ref().map(|d|(d.dynamics.mode,d.dynamics.strengths));
            let actual=self.output.skeleton_parts[part].bone_transform;
            let rendered=self.output.world_bones[bone];
            let render_error=(0..4).flat_map(|i|(0..3).map(move |j|(actual[i][j]-rendered[i][j]).abs())).fold(0.0f32,f32::max);
            (name,distance,max_basis,drive,render_error)
        });
        format!("state={} root={:?} forward={:?} velocity={:?} turn={} axes={:?} biped={:?} board_held={} flags={:08x?} pinned={} {} hands={hands:?} ik={ik:?} collision_weight={} collision_mode={:?} intent={:?} danger={danger:?}",
            self.output.state,self.output.root_transform[3],self.output.root_transform[2],
            s.biped_ground.controller.state.motion.velocity_480,s.biped_ground.controller.state.motion.angular_velocity_688,
            self.controls.sampled_camera_axes(),[s.animation_input.extra.biped_world_x,s.animation_input.extra.biped_world_z],s.player_input.physical.off_board.flag_311,
            [s.player_input.processed.flags_2472,s.player_input.processed.flags_2476,s.player_input.processed.flags_2480],
            self.actor_input.pinned,self.test_input_trace(),s.collision_feedback.drive_weight,(s.skeleton_collision.partial_ragdoll,s.skeleton_collision.is_ragdoll),s.biped_ground.controller.state.intent)
    }
    pub(crate) fn use_actor_gesture_channels(&mut self) {self.physics.set_gesture_preferences(None);}
    pub(crate) fn prepared_movement(&self)->Option<(&[String],&[i32],&[skate3_core::animation::output::Sqt])> {
        self.prepared_tick.as_ref()?.frame.as_ref()?;
        Some((&self.skater.animation.evaluator.frames.bone_names,&self.skater.animation.evaluator.frames.parents,&self.skater.animation.pose))
    }
    pub(crate) fn prepared_gesture_context(&self)->Option<crate::skate3_gesture_playback::Context> {
        self.prepared_tick.as_ref()?.frame.as_ref()?;
        let s=&self.skater;let motion=&s.animation.motion;
        Some(crate::skate3_gesture_playback::Context {
            busy_hands:motion.hand_services.busy_hands,
            filtered:motion.condition_inputs.physical_state.as_ref()?.category,
            hippy:s.player_input.processed.flags_2480&0x1000!=0,
            holding_board:motion.playback_context.board_available?,
            offboard:s.teleport_state.manual_on_board().map_or(s.player_input.physical.state.category_12==500,|on_board|!on_board),
            distance_to_cog:s.physical_feedback.crouching.animation_height_72,
            blocked:self.output.wiping_out || ["RetrieveBoard","Shove","WipeoutPushOff"].into_iter().any(|name|motion.animation.channels.has(name)) ||
                ["OB_Mount","OB_Dismount"].into_iter().any(|name|motion.animation.motion_intents.contains_key(name)),
            skater_animation_flags:s.animation.gesture_flags(),
        })
    }
    pub(crate) fn capture_composed_actor_pose(&mut self,actor:&crate::actor_construction::ActorConstruction)->Result<(),String> {
        if self.prepared_movement().is_none(){return Ok(());}
        let (local,hierarchy)=actor.try_composed_body_pose()?;
        if local.len()!=self.skater.animation.packet.local.len() || hierarchy.len()!=local.len() {
            return Err("Actor composed pose differs from its selected physical player".into());
        }
        self.skater.animation.packet.local=local;
        self.skater.animation.packet.hierarchy=hierarchy;
        Ok(())
    }
    pub(crate) fn advanced_this_tick(&self) -> bool { self.advanced_this_tick }
    pub(crate) fn completed_teleport(&self) -> Option<[f32; 16]> {
        if !self.advanced_this_tick { return None; }
        self.completed_teleport.map(|teleport| teleport.transform).or_else(|| {
            // Automatic source bail recovery has no host Actor generation.
            // Its actual completed external reset still notifies S2's mission
            // and HUD consumers through their ordinary teleport publication.
            (self.awaiting_teleport.is_none() && self.output.teleported && self.output.state != 702).then(||
                flatten(self.skater.player_input.processed.matrix_1536.map(|row| row.map(f32::from_bits))))
        })
    }
    pub(crate) fn acknowledge_teleport(
        &self,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<(), String> {
        if let Some(generation) = self
            .completed_teleport
            .and_then(|teleport| teleport.generation)
        {
            actor
                .try_acknowledge_skate3_teleport(generation)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn sync_teleport(
        &mut self,
        actor: &crate::actor_construction::ActorConstruction,
    ) -> Result<(), String> {
        let (generation, transform, on_board, pending) = actor
            .try_skate3_teleport_publication()
            .map_err(|e| e.to_string())?;
        if (self.teleport_generation.is_none() && pending)
            || self
                .teleport_generation
                .is_some_and(|old| old != generation)
        {
            self.queue_teleport(
                transform
                    .known_lanes()
                    .ok_or("Incomplete host teleport matrix")?,
                on_board,
                generation,
                actor.try_last_set_foot_forward().map_err(|error|error.to_string())?,
            )?;
        }
        self.teleport_generation = Some(generation);
        Ok(())
    }
    pub(crate) fn diagnostic_controls(&self) -> (bool, [f32; 4], f32) {
        (self.input_focused,
            std::array::from_fn(|i| f32::from_bits(self.controls.controller.words()[7+i])),
            self.output.effective_root_transform()[2][0].atan2(self.output.effective_root_transform()[2][2]))
    }
    pub(crate) fn diagnostic_transition(&self) -> String {
        let s = &self.skater;
        let motion = s.animation.motion_controller.frame.current
            .and_then(|i|self.graphs.motion.binding.states.get(i)).map(|s|s.name.as_str());
        format!("state={} pinned={} axes={:?} turn={} motion={motion:?} MGturn={:?} source_trick={:?} turn_rate={} source_score={} source_active={} holder={:?} landing={:?}",
            self.output.state,self.actor_input.pinned,self.diagnostic_controls().1,
            s.animation_input.fields.turn,s.animation.motion.animation.motion_intents.get("Turn").copied(),
            s.scoring.trick_name(),s.ground_animation.turn_rate,s.scoring.sequence_score(),s.scoring.sequence_active(),s.scoring.session.holder.snapshot,s.landing_quality)
    }
    pub(crate) fn set_input_focus(&mut self, focused: bool) { self.input_focused = focused; }
    pub(crate) fn capture_actor_input(&mut self, actor: &crate::actor_construction::ActorConstruction) -> Result<(),String> {
        let mut policy=actor.try_skate3_actor_input().map_err(|error|error.to_string())?;
        if !self.input_focused {
            policy.pinned = true;
            policy.pinned_gestures = false;
        }
        if policy!=self.actor_input {
            crate::diagnostics::test_log::event("skate3_actor_policy",format_args!(
                "tick={} pinned={} pinned_gestures={} prevent_manual_respawn={}",
                self.physics.ticks,policy.pinned,policy.pinned_gestures,policy.prevent_manual_respawn));
        }
        self.host_cvars=Some(actor.cached_physical_allocation().map_err(|e|e.to_string())?.owner().inputs().cvars.clone());
        self.actor_input=policy;
        self.controls.host_actor_input=policy;
        // These are semantic S2 Actor policies, not a bitwise alias of S3's
        // unrelated Actor1908 input-filter word.
        self.physics.animation_profile.force_braking=self.actor_input.pinned;
        self.physics.animation_profile.prevent_manual_respawn=self.actor_input.prevent_manual_respawn;
        Ok(())
    }
    pub(crate) fn capture_actor_physics(&mut self, actor: &crate::actor_construction::ActorConstruction) -> Result<(),String> {
        let input=actor.published_animation_physics_input().map_err(|error|error.to_string())?;
        let input=input.lock().map_err(|_|"native Actor physics input lock")?;
        let profile=&mut self.physics.animation_profile;
        if [profile.truck_tightness,profile.wheel_hardness].map(f32::to_bits)!=
            [input.truck_tightness,input.wheel_hardness].map(f32::to_bits) {
            crate::diagnostics::test_log::event("skate3_equipment",format_args!(
                "tick={} truck_tightness={} wheel_hardness={}",
                self.physics.ticks,input.truck_tightness,input.wheel_hardness));
        }
        if input.flags.living_world_impulse_active {
            crate::diagnostics::test_log::event("skate3_world_impulse",format_args!(
                "tick={} direction={:?} source=skate2_actor",self.physics.ticks,input.living_world_impulse));
        }
        profile.truck_tightness=input.truck_tightness;
        profile.wheel_hardness=input.wheel_hardness;
        profile.prevent_manual_respawn=input.flags.prevent_manual_respawn;
        profile.ignore_respawn_reset_button=u8::from(input.flags.ignore_respawn_reset_button);
        profile.force_braking=input.flags.force_braking || !self.input_focused;
        profile.external_impulse=input.flags.living_world_impulse_active.then_some(input.living_world_impulse.map(f32::to_bits));
        Ok(())
    }
    pub(crate) fn request_offboard_replacement(&mut self) {
        self.offboard_replacement = Some(OffboardReplacement { saw_offboard: false, pending_toggle: true });
    }
    pub(crate) fn observe_offboard_replacement(&mut self) {
        if let Some(replacement) = &mut self.offboard_replacement {
            replacement.saw_offboard |= self.output.offboard;
        } else if self.output.offboard || self.dismount_transition_active() {
            self.offboard_replacement = Some(OffboardReplacement { saw_offboard: self.output.offboard, pending_toggle: false });
        }
    }
    fn dismount_transition_active(&self) -> bool {
        // Authored AG/MG publications, including the accepted transition before
        // physics first publishes Offboard. No animation-name/timer heuristic.
        let animation=&self.skater.animation;
        animation.action.motion_intents.contains_key("OB_Dismount") ||
            animation.attributes.entries().iter().any(|attribute|
                [b"SkaterOffBoard".as_slice(),b"OB_Unmounting",b"IdleDismounting"].iter().any(|name|
                    attribute.name==skate3_core::animation::skeleton_input::name::encode(name)))
    }
    pub(crate) fn keep_offboard_replacement(&self) -> bool {
        self.offboard_replacement.as_ref().is_some_and(|replacement| {
            if replacement.pending_toggle { true }
            else if replacement.saw_offboard { self.output.state != 100 || self.output.offboard }
            else { self.output.offboard || self.dismount_transition_active() }
        })
    }
    pub(crate) fn begin_ready_offboard_replacement(&mut self) {
        if !self.keep_offboard_replacement() {
            self.offboard_replacement = Some(OffboardReplacement { saw_offboard: self.output.offboard, pending_toggle: false });
        }
    }
    /// S2 FootForward RIGHT0/LEFT1 matches S3's checkpoint enum; natural
    /// stance uses GOOFY1/REGULAR0. The customization comes from the actual
    /// profile; each marker/breadcrumb independently requests its leading foot.
    pub(crate) fn configure_character_stance(&mut self, regular: bool, foot_forward: u32, entering: bool) -> Result<(),String> {
        if foot_forward > 1 { return Err("Invalid native character leading foot".into()); }
        self.skater.animation.set_natural_stance(u32::from(!regular));
        if entering { self.skater.animation.restore_foot_forward(foot_forward != 0); }
        Ok(())
    }
    /// GetCACSettings82590D20/82590B50: profile refresh does not invent a
    /// posture request; authored Play Begin owns that construction latch.
    pub(crate) fn configure_animation_style(&mut self, settings: skate2_formats::skater_profile::PcSkate3Style) -> Result<(),String> {
        self.skater.animation.set_style_preferences(settings)
    }
    pub(crate) fn set_online_scoring(&mut self, online: bool) { self.skater.scoring.online_game = online; }
    pub(crate) fn set_camera_type(&mut self, value: u32) { self.camera.set_camera_type(value); }
    pub(crate) fn set_camera_invert_look(&mut self, value: [bool; 2]) { self.camera.set_invert_look(value); }
    pub(crate) fn set_airtime_slow_motion(&mut self, enabled: bool) { self.airtime_slow_motion = enabled; }
    /// Diagnostic only: signed wheel-sphere gap against the actual loaded
    /// collision surface. Queries do not move a body or modify solver state.
    pub(crate) fn test_spawn_wheel_gaps(&self) -> Result<[f32;4],String> {
        use skate3_core::math::Vector3;
        let poses=self.physics.board.part_transforms();
        let mut gaps=[0.;4];
        for i in 0..4 {
            let p=poses[i].translation;
            let hit=self.physics.world.query_thin_line(
                Vector3::new(p.x,p.y+0.5,p.z),Vector3::new(p.x,p.y-0.5,p.z))
                .map_err(str::to_owned)?.ok_or_else(||format!("wheel{i} has no collision ground near {p:?}"))?;
            let q=hit.geometry.position;let n=hit.geometry.normal;
            gaps[i]=(p.x-q.x)*n.x+(p.y-q.y)*n.y+(p.z-q.z)*n.z-self.physics.settings.wheel_radius;
        }
        Ok(gaps)
    }
    #[cfg(test)]
    pub(crate) fn test_ground_height(&self,point:[f32;3])->Result<f32,String> {
        use skate3_core::math::Vector3;
        self.physics.world.query_thin_line(Vector3::new(point[0],point[1]+50.,point[2]),
            Vector3::new(point[0],point[1]-50.,point[2])).map_err(str::to_owned)?
            .map(|hit|hit.geometry.position.y).ok_or_else(||format!("marker fixture has no resident terrain at {point:?}"))
    }
    #[cfg(test)]
    pub(crate) fn test_spawn_stance(&self) -> (u32, i32) {
        (self.skater.animation.checkpoint_stance(), self.skater.animation.natural_stance())
    }
    #[cfg(test)]
    pub(crate) fn test_boneless_trace(&self) -> String {
        let f=&self.skater.boneless;
        let p=&self.skater.player_input.processed;
        format!("toe={} right={} anchor={:?} com={:?} velocity={:?} prepared={:?} flags={:08x}/{:08x} {}",
            f.toe,f.right,f.anchor,p.vectors_544_560_592_608[2].map(f32::from_bits),
            p.vectors_544_560_592_608[3].map(f32::from_bits),p.prepared_jump_704.map(f32::from_bits),
            p.flags_2468,p.flags_2480,self.test_input_trace())
    }
    #[cfg(test)]
    pub(crate) fn test_bail_control_probe(&self)->([f32;2],Option<[f32;2]>) {
        (self.skater.animation_input.extra.wipeout_control,self.controls.sampled_camera_axes())
    }
    #[cfg(test)]
    pub(crate) fn test_plant_control_probe(&self)->([f32;2],[f32;4],Option<[f32;2]>) {
        (self.skater.animation_input.extra.body_adjust,
            self.skater.footplant.test_release_velocity(),self.controls.sampled_camera_axes())
    }
    #[cfg(test)]
    pub(crate) fn test_footplant_trace(&self) -> String {
        let f=&self.skater.footplant;
        let p=&self.skater.player_input.processed;
        format!("enabled={} toe={:?} candidate={} hit={} perform={} active={} requested={} time={} duration={} blend={} flags={:08x}/{:08x}/{:08x} {}",
            f.enabled,f.selected_toe,f.candidate,f.hit,f.perform,f.contact_active,f.requested,
            f.contact_time,f.scalar_596,f.target_blend,p.flags_2476,p.flags_2480,p.flags_2484,self.test_input_trace())
    }
    #[cfg(test)]
    pub(crate) fn test_grab_scene(&self,world:&PhysicalAssemblyWorld)->Result<(usize,usize,usize),String> {
        self.grabs.test_audit(world,&self.physics.world,&self.physics.offboard_grab_scene)
    }
    #[cfg(test)]
    pub(crate) fn test_living_world_impulse(&self) -> (bool,[u32;4]) {
        (self.skater.player_input.processed.flags_2468 & 4 != 0,self.skater.player_input.processed.vector_1520)
    }
    #[cfg(test)]
    pub(crate) fn test_actor_input(&self) -> (bool,bool,Vec<String>,[f32;2]) {
        (self.actor_input.pinned,self.physics.animation_profile.prevent_manual_respawn,
            self.controls.named_intents.keys().cloned().collect(),
            [self.physics.animation_profile.truck_tightness,self.physics.animation_profile.wheel_hardness])
    }
    #[cfg(test)]
    pub(crate) fn test_input_trace(&self) -> String {
        let action = self.skater.animation.action_controller.frame.current
            .and_then(|index|self.graphs.action.binding.states.get(index)).map(|state|state.name.as_str());
        let motion = self.skater.animation.motion_controller.frame.current
            .and_then(|index|self.graphs.motion.binding.states.get(index)).map(|state|state.name.as_str());
        let intents: Vec<_> = self.controls.intents.iter().filter(|intent|intent.value != 0.)
            .map(|intent|(intent.name,intent.value)).collect();
        format!("buttons={:#x} stick={:?} intents={intents:?} AG={action:?} MG={motion:?} capabilities={:#x}",
            self.controls.controller.words()[13], &self.controls.controller.words()[7..11],
            self.skater.player_input.physical.scoring.capabilities_204)
    }
    #[cfg(test)]
    pub(crate) fn camera_shot(&self) -> &str { self.camera.selected_shot() }
    pub(crate) fn output_frame(&self) -> std::sync::Arc<FrameOutput> { std::sync::Arc::clone(&self.output) }
    pub(crate) fn output(&self) -> &FrameOutput {
        &self.output
    }
    #[cfg(test)]
    pub(crate) fn world_body_count(&self) -> usize {
        self.world_body_count
    }
    /// A user-selected physics change returns this Actor to its existing S2
    /// assembly. The matching Actor reset applies these rates once, after its
    /// real body resets and before Ground.Enter samples the deck speed.
    pub(crate) fn return_to_skate2(
        self,
        world: &mut PhysicalAssemblyWorld,
        actor: &crate::actor_construction::ActorConstruction,
        owner: &std::sync::Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), String> {
        let mut rates = Vec::with_capacity(self.board.len() + self.skeleton.len());
        for body in self.board.iter().chain(&self.skeleton) {
            let view = world.backend().try_external_body_snapshot(*body)?;
            rates.push(exit::BodyRate { body: *body, centre: view.center_of_mass,
                velocity: view.velocity, omega: view.omega });
        }
        let transform = crate::skeleton_root_transforms::SkeletonRootMatrix::known(flatten(self.output.effective_root_transform()));
        let on_board = !self.output.offboard;
        let mut pending = owner.skate3_exit.lock().map_err(|_| "native exit lock")?;
        actor.try_publish_teleport(&transform, on_board).map_err(|error|error.to_string())?;
        let generation = actor.try_teleport_publication().map_err(|error|error.to_string())?.0;
        *pending = Some(exit::PlayerExit { generation, rates });
        drop(pending);
        world.backend_mut().try_return_external_player()?;
        world.try_return_native_skater_root(board, skeleton, transform, owner)?;
        self.release(world)
    }
    pub(crate) fn release(self, world: &mut PhysicalAssemblyWorld) -> Result<(), String> {
        world.backend_mut().try_select_external_player(&[])?;
        // Source camera requests have a one-tick lifetime. Let the native
        // countdown expire them without cancelling a story/script rate.
        Ok(())
    }
}

/// Preserve S3's request lifetime through S2's existing timer message API.
pub(crate) fn host_sim_rate_request(request: skate3_core::camera::SimulationRateRequest) -> crate::simulation_sim_rate::SetSimRate {
    let ticks = request.ticks as i32;
    crate::simulation_sim_rate::SetSimRate {
        sim_time: request.timestep,
        ticks_of_slow_motion: if request.timestep == f32::from_bits(0x3c88_8889) { 0 }
            else if ticks > 0 { ticks }
            // S3's fallback stores180 directly; S2's API adds one.
            else { 179 },
    }
}

pub(crate) fn flatten(matrix: [[f32; 4]; 4]) -> [f32; 16] {
    std::array::from_fn(|i| matrix[i / 4][i % 4])
}
fn unflatten(matrix: [f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|c| std::array::from_fn(|r| matrix[c * 4 + r]))
}

impl Skate3Player {
    pub fn period(&self) -> std::time::Duration {
        self.physics.period()
    }

    /// Replace the current tick's host-owned NPC/car/prop snapshots. A body has
    /// exactly one identity and one reaction recipient; remote-player prediction
    /// and multiplayer clock rules are not enabled by this host boundary.
    pub fn set_moving_bodies(
        &mut self,
        bodies: Vec<skate_dynamics::SolidBody>,
    ) -> Result<(), String> {
        self.set_moving_mesh_bodies(bodies, Vec::new())
    }

    /// `set_moving_bodies`, where each of `meshes` names one of `bodies` whose
    /// authored collision mesh (native triangles at its current placement)
    /// answers the board, skeleton, wheel-line and nearby-triangle queries.
    /// Those bodies stay finite-mass solver proxies and receive reactions.
    pub(crate) fn set_moving_mesh_bodies(
        &mut self,
        bodies: Vec<skate_dynamics::SolidBody>,
        meshes: Vec<crate::simulation_assembly_backend::constraint_session::skate3_world::Skate3MeshBody>,
    ) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for body in &bodies {
            if body.id == 0
                || !ids.insert(body.id)
                || !body.inverse_mass.is_finite()
                || body.inverse_mass < 0.
                || !body.center_of_mass.is_finite()
                || !body.linvel.is_finite()
                || !body.angvel.is_finite()
                || !body.inertia_rotation.is_finite()
                || !body.inverse_inertia.is_finite()
            {
                return Err(
                    "Moving bodies require unique nonzero IDs and finite physical state".into(),
                );
            }
        }
        let mut proxies = super::network::Proxies::default();
        let meshes: Vec<_> = meshes
            .into_iter()
            .filter_map(|mesh| {
                let body = bodies.iter().find(|body| body.id == mesh.id)?;
                crate::skate3_native::modding::moving_queries::MeshQuery::new(body, mesh.triangles)
            })
            .collect();
        // The same snapshots must answer the source engine's wheel, biped and
        // trajectory queries as well as its solid solver contacts.
        self.physics.set_external_queries(Some(std::sync::Arc::new(
            crate::skate3_native::modding::moving_queries::MovingQueries::new(bodies.clone(), meshes.clone()),
        )));
        for body in bodies {
            let id = body.id;
            proxies.append_solid(body, &self.physics, &self.skater, true);
            if let Some(mesh) = meshes.iter().find(|mesh| mesh.id == id)
                && let Some(&(index, _)) = proxies.solids.last().filter(|(_, solid)| solid.id == id)
            {
                proxies.meshes.push((index, mesh.clone()));
            }
        }
        self.physics.network_proxies = proxies;
        Ok(())
    }

    pub fn set_external_queries(
        &mut self,
        queries: std::sync::Arc<dyn skate3_core::physics::board_world::ExternalQueries>,
    ) {
        self.physics.set_external_queries(Some(queries));
    }

    /// Bind actual authored grab records to their actual collision meshes.
    /// Identities are supplied by the host's object registry, never inferred.
    pub fn set_grab_objects(
        &mut self,
        objects: Vec<skate3_core::player::offboard::grab_scene::Object>,
        mesh_assemblies: Vec<(u32, u32)>,
    ) -> Result<(), String> {
        let bindings = mesh_assemblies
            .into_iter()
            .map(|(mesh, assembly)| super::offboard::grab_scene::MeshAssembly { mesh, assembly })
            .collect();
        let registry =
            super::offboard::grab_scene::Registry::new(&self.physics.world, objects, bindings)?;
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    /// Drain once after the tick, then apply each correction to the matching
    /// Skate 2 world body before its next publication.
    pub fn take_body_reactions(&mut self) -> Vec<BodyReaction> {
        std::mem::take(&mut self.physics.network_proxies.dynamics_deltas)
            .into_iter()
            .map(
                |(
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                )| BodyReaction {
                    id,
                    linear_velocity_delta,
                    angular_velocity_delta,
                    position_delta,
                    orientation_delta,
                },
            )
            .collect()
    }

    pub(crate) fn set_profile(&mut self, profile: &str) -> Result<(), String> {
        let _asset_load = crate::diagnostics::test_log::span("skate3_asset_graph_load");
        let difficulty = crate::skate3_native::difficulty::Difficulty::parse(profile)?;
        self.physics.set_difficulty(difficulty);
        Ok(())
    }

    /// Skate 2's tuning on the source solver. Read the original host Vault;
    /// shared settings and the push/jump limits keep their actual provenance.
    /// S3-only settings inherit its authored base collection, not Normal.
    pub(crate) fn set_skate2_profile(
        &mut self,
        vault: &std::sync::Arc<crate::attribute_runtime::AttributeRuntimeVault>,
    ) -> Result<(), String> {
        use skate2_formats::attrib::string_hash64 as hash;
        let mut data = (*embedded_collections()?).clone();
        let mut fields = std::collections::BTreeMap::new();
        let mut field = |target: &str, class: &str, source: u64| -> Result<(), String> {
            let word = vault.instance(hash(class), hash("default"))
                .try_scalar_word(source, 0).map_err(|error|error.to_string())?
                .ok_or_else(||format!("Original Skate 2 {class}.{source:016X} missing"))?;
            if !f32::from_bits(word).is_finite() { return Err(format!("Invalid original Skate 2 {class}.{source:016X}")); }
            fields.insert(target.into(), skate_data::collections::Field {
                type_name: "EA::Reflection::Float".into(), data: format!("{word:08X}"), array: None,
            });
            Ok(())
        };
        // S2 Toolkit_CalcPushAcceleration82b272f8 has one change cap at all speeds.
        for target in ["MaxPushDVStart", "MaxPushDVEnd"] { field(target, "physics_push", 0x76fe_ee54_a3ad_2af5)?; }
        // S2 GroundJumpAttribute minimum/maximum share physical height units.
        for target in ["JumpMinHeight", "Hash_BE3F74F978D777E5"] { field(target, "physics_jump", 0xbb9e_9baf_ba1b_29e6)?; }
        field("JumpMaxHeight", "physics_jump", 0x0b24_54c6_6641_0860)?;
        for (target, class) in [
            ("PumpEffectFactor", "physics_pumping"),
            ("PumpEffectFactorAbsorption", "physics_pumping"),
            ("UnintentionalPumpScalar", "physics_pumping"),
            ("Wipeout_GroundXZAcceleration", "physics_wipeout"),
            ("MaxAutoBodySpinSpeed", "physics_airstates"),
        ] { field(target, class, hash(target))?; }
        crate::diagnostics::test_log::event("difficulty_profile", format_args!("selected=skate2 solver=skate3 source=original_skate2_vault fields={fields:?} base=authored_default"));
        data.override_profile("physics_mode", "test", "default", fields)?;
        self.skater.reload_difficulty(&data)?;
        self.physics.set_difficulty(crate::skate3_native::difficulty::Difficulty::Custom);
        Ok(())
    }

    /// Prepare the complete difficulty table before replacing the active slot.

    pub(crate) fn set_motorized_profile(&mut self, base: &str) -> Result<(), String> {
        let _ = crate::skate3_native::difficulty::Difficulty::parse(base)?;
        let mut data = (*embedded_collections()?).clone();
        let target = data
            .field("physics_mode", "motorized", "MotorTopSpeed")?
            .clone();
        // Match Motorized landing protection while inheriting the base handling.
        let landing = data.field("physics_mode", "motorized", "WipeoutCheckForBadLanding")?.clone();
        let enabled = skate_data::collections::Field {
            type_name: "EA::Reflection::Bool".into(),
            data: "01".into(),
            array: None,
        };
        data.override_profile(
            "physics_mode",
            "test",
            base,
            [
                ("MotorEnabled".into(), enabled),
                ("MotorTopSpeed".into(), target),
                ("WipeoutCheckForBadLanding".into(), landing),
            ]
            .into_iter()
            .collect(),
        )?;
        self.skater.reload_difficulty(&data)?;
        self.physics
            .set_difficulty(crate::skate3_native::difficulty::Difficulty::Custom);
        Ok(())
    }

    pub fn replace_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let map = scene.map()?;
        let world = scene.world(self.physics.settings.floor_material)?;
        let grind = crate::skate3_native::grind_world::StaticProvider::new(Some(&map))?;
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world.replace_static_geometry(world);
        self.physics.grind_world = std::sync::Arc::new(grind);
        self.skater
            .trajectory
            .bind_grind_world(std::sync::Arc::clone(&self.physics.grind_world));
        self.physics.offboard_grab_scene = registry;
        Ok(())
    }

    pub fn replace_collision_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let world = {
            let _stage = crate::diagnostics::test_log::timed_span("skate3_terrain_world_build");
            scene.world(self.physics.settings.floor_material)?
        };
        self.replace_collision_world(world)
    }

    fn replace_collision_world(&mut self, world: skate3_core::physics::board_world::BoardWorld) -> Result<(),String> {
        let registry = super::offboard::grab_scene::Registry::new(&world, Vec::new(), Vec::new())?;
        self.physics.world.replace_static_geometry(world);
        self.physics.offboard_grab_scene = registry;
        self.skater.offboard_grab.invalidate();
        Ok(())
    }



    pub fn travel(
        &mut self,
        transform: AnimationPartTransform,
        velocity: [f32; 3],
    ) -> Result<(), String> {
        self.skater.travel(transform, Some(velocity))
    }
    pub fn host_teleport(
        &mut self,
        transform: AnimationPartTransform,
        on_board: bool,
    ) -> Result<(), String> {
        // Actor's external-reset packet selects702; its animation graph then
        // restores GivenStance before State61 performs the physical placement.
        // An extra immediate request_teleport_ex resets with the outgoing pose,
        // before that graph, and can turn the new board/leading foot backwards.
        self.interactable_hold = None;
        let mut physical_target = spawn::target(&self.physics.world, transform, on_board)?;
        // Actor targets are effective world facing. Convert once here, after
        // restore_foot_forward, rather than sampling an animation blend packet
        // whose mirror can differ on702's two reset/ground dispatches.
        if on_board && self.skater.animation.stance().1 {
            for axis in [0, 2] { physical_target[axis] = physical_target[axis].map(|v| -v); }
        }
        self.skater.teleport_state.request_manual(physical_target, on_board);
        Ok(())
    }
    pub fn teleport_pending(&self) -> bool {
        self.skater.player_input.pending_teleport().is_some()
            || self.skater.teleport_state.manual_on_board().is_some()
    }

    /// Collect every host sample even when no native fixed tick is due yet.
    pub fn collect_controller(&mut self, raw: Option<RawController>) {
        self.skater.scoring.darkcatch_button_held = raw.is_some_and(|pad|pad.buttons & 0x0200 != 0);
        crate::skate3_native::input::collect_embedded(
            &mut self.input,
            raw.map(|raw| XboxState {
                buttons: raw.buttons,
                triggers: raw.triggers,
                left: raw.left,
                right: raw.right,
            }),
        );
    }



    fn sample_solver_input(&mut self)->Result<(skate3_core::input::gameplay_map::GameplayActions,bool),String> {
        let tick = crate::skate3_native::input::publish_embedded(&mut self.input);
        let mut actions = tick.actions();
        // Same preceding Ground publication as the native input listener.
        // Host policy uses the active character's regular (non-mongo) foot,
        // derived from the existing regular/switch push classification.
        self.controls.preferences.automatic_push_enabled=self.skater.player_state.ground_output.as_ref()
            .is_some_and(|g|g.intents_52.selected_mode_below_speed_threshold_58);
        self.controls.preferences.automatic_push_right=self.output.gameplay.stance[2]!=self.output.gameplay.stance[3];
        // A menu is not the native Actor pin. Consume the neutral sample so
        // old stick values cannot survive while the pin suppresses Fill.
        let retained_policy = self.controls.host_actor_input;
        if !self.input_focused { self.controls.host_actor_input.pinned = false; }
        let sampled = self.controls.update_for_physics_with_entry_camera(
            &mut actions, &self.physics, &self.skater, &self.camera, self.entry_camera_basis,
        );
        self.controls.host_actor_input = retained_policy;
        sampled?;
        self.controls.publish_gestures(
            self.physics.difficulty_index(),
            self.skater.player_input.physical.state.state_16,
        );
        self.actor_input.apply(&mut self.controls, self.output.wiping_out);
        Ok((actions,tick.controller_available()))
    }

    pub(crate) fn advance_solver_tick(&mut self) -> Result<FrameOutput, String> {
        let (mut actions,input_available)=self.sample_solver_input()?;
        super::frame::advance(
            &mut self.physics,
            &mut self.skater,
            &mut self.controls,
            &self.graphs,
            &mut actions,
            input_available,
            &mut self.camera,
        )?;
        Ok(export_frame(&self.physics, &self.skater, &self.camera, self.hips_body))
    }


}

    fn export_frame(physics: &super::Skate3PhysicsState, skater: &super::Skate3ControllerState, camera: &crate::skate3_native::camera::CameraRuntime, hips_body: usize) -> FrameOutput {
        let deck = physics.board.part_transforms()[BodyId::Deck.index()];
        let board_transform = [
            [
                deck.basis.columns[0][0],
                deck.basis.columns[0][1],
                deck.basis.columns[0][2],
                0.,
            ],
            [
                deck.basis.columns[1][0],
                deck.basis.columns[1][1],
                deck.basis.columns[1][2],
                0.,
            ],
            [
                deck.basis.columns[2][0],
                deck.basis.columns[2][1],
                deck.basis.columns[2][2],
                0.,
            ],
            [
                deck.translation.x,
                deck.translation.y,
                deck.translation.z,
                1.,
            ],
        ];
        let velocity = physics.board.bodies()[BodyId::Deck.index()]
            .rates
            .linear_velocity;
        let root = skater.animated_skeleton.roots.animation_to_world;
        let physical = &skater.player_input.physical;
        let skeleton_transforms = skater.skeleton.part_transforms();
        let score = &skater.animation.motion.score_packet;
        let packet = &skater.animation.packet;
        let contact = &skater.collision_feedback;
        FrameOutput {
            teleported: skater.teleported_this_tick,
            gameplay: crate::skate3_native::world_types::GameplayOutput {
                state_flags: skater.player_state.state_flags,
                interactable_object: (skater.player_input.processed.object_2464,
                    skater.player_input.processed.flags_2480 & (1 << 20) != 0),
                animation_turn: skater.animation_input.fields.turn,
                landing_quality: skater.landing_quality,
                jump_strength: skater.animation_input.extra.jump_strength,
                hips_ground_start: skater.hips_ground_start,
                hips_ground: skater.player_input.player.hips_line_test_1488,
                deck_ground: physics.riding.probes.deck,
                foot_surfaces: [skater.player_input.processed.left_surface_2596, skater.player_input.processed.right_surface_2600],
                deck_tilt: skater.ground.steering.deck_tilt,
                wheel_hardness: skater.player_input.processed.scalar_2764,
                trick: score.trick_names.first.or_else(|| score.grab.map(|value| value.0)),
                // ScoringGrabs publishes a continuous descriptor separately
                // from ScoringTrick. S2 ProcessAnimData82dfc358 admits that
                // descriptor through bit25; otherwise ordinary source grabs
                // animate correctly but never enter the native score holder.
                trick_flags: score.flags | if score.grab.is_some() { 0x0200_0000 } else { 0 },
                trick_tweak: score.grab.or(score.handplant).map_or([0.; 2], |value| value.1),
                wipeout_gesture: skater.animation_input.extra.wipeout_gesture,
                stance: [packet.riding_fakie, packet.mirrored, packet.riding_switch, packet.regular_stance],
                board_ground: physics.riding.ground.clone(),
                wheel_lines: physics.riding.wheel_lines,
                bone_contacts: contact.bones,
                bone_velocity_changes: std::array::from_fn(|i| skater.skeleton.record.velocity_changes[i]),
                contact_regions: contact.regions,
                contact_flags: contact.flags,
                specific_contacts: contact.specific,
                wipeout_times: contact.wipeout_times,
                entity_impacts: [contact.maximum_group_8_force, contact.maximum_skater_force, contact.maximum_group_11_force],
                feet: skater.foot_physical.output,
            },
            tick: physics.ticks,
            physical: *physical,
            centre_of_mass: physical.reckoning.vector_64.map(f32::from_bits),
            damped_centre_of_mass: skater.centre_of_mass_output,
            state: physical.state.state_16,
            category: physical.state.category_12,
            offboard: physical.state.category_12 == 500,
            wiping_out: skater.skeleton_collision.is_ragdoll,
            board_transform,
            board_velocity: [velocity.x, velocity.y, velocity.z],
            board_parts: std::array::from_fn(|i| {
                let part = physics.board.part_transforms()[i];
                let rates = &physics.board.bodies()[i].rates;
                BodyOutput {
                    transform: [
                        [
                            part.basis.columns[0][0],
                            part.basis.columns[0][1],
                            part.basis.columns[0][2],
                            0.,
                        ],
                        [
                            part.basis.columns[1][0],
                            part.basis.columns[1][1],
                            part.basis.columns[1][2],
                            0.,
                        ],
                        [
                            part.basis.columns[2][0],
                            part.basis.columns[2][1],
                            part.basis.columns[2][2],
                            0.,
                        ],
                        [
                            part.translation.x,
                            part.translation.y,
                            part.translation.z,
                            1.,
                        ],
                    ],
                    centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                    velocity: [
                        rates.linear_velocity.x,
                        rates.linear_velocity.y,
                        rates.linear_velocity.z,
                    ],
                    omega: [
                        rates.angular_velocity.x,
                        rates.angular_velocity.y,
                        rates.angular_velocity.z,
                    ],
                }
            }),
            skeleton_parts: skater
                .animated_skeleton
                .bone_indices
                .iter()
                .enumerate()
                .map(|(part, &bone)| {
                    let transform = skeleton_transforms[part];
                    let rates = &skater.skeleton.bodies()[part].rates;
                    SkeletonPartOutput {
                        bone_name: skater.animation.evaluator.frames.bone_names[bone].clone(),
                        body: BodyOutput {
                            transform,
                            centre_of_mass: [rates.position.x, rates.position.y, rates.position.z],
                            velocity: [
                                rates.linear_velocity.x,
                                rates.linear_velocity.y,
                                rates.linear_velocity.z,
                            ],
                            omega: [
                                rates.angular_velocity.x,
                                rates.angular_velocity.y,
                                rates.angular_velocity.z,
                            ],
                        },
                        bone_transform: compose_affine(
                            &transform,
                            &skate3_core::physics::skeleton_root::inverse_rigid(
                                &skater.animated_skeleton.physics_frames[part],
                            ),
                        ),
                    }
                })
                .collect(),
            root_velocity: {
                let v = skater.skeleton.bodies()[hips_body]
                    .rates
                    .linear_velocity;
                [v.x, v.y, v.z]
            },
            root_omega: {
                let v = skater.skeleton.bodies()[hips_body]
                    .rates
                    .angular_velocity;
                [v.x, v.y, v.z]
            },
            camera: camera.frame,

            root_transform: root,
            bone_names: skater.animation.evaluator.frames.bone_names.clone(),
            world_bones: skater
                .render_pose
                .iter()
                .map(|bone| compose_affine(&root, bone))
                .collect(),
        }
    }

#[cfg(test)]
mod style_tests {
    use super::*;
    #[test]
    #[ignore = "requires embedded original Skate 3 animation banks"]
    fn embedded_styles_and_postures_change_authored_motion_without_resetting_stance() {
        use skate3_core::animation::{playback::{PlayAnimation,PlayAnimationInstance,TransitionSettings},playback_tree::{Evaluation,PoseCommand}};
        let root=skate_data::asset_io::mount(PLAYER_ASSETS).unwrap();
        let data=embedded_collections().unwrap();
        let assets=player_assets(root).unwrap();
        let mut animation=crate::skate3_native::skater_animation::SkaterAnimation::from_source(&data,&assets.graphs,b"",std::sync::Arc::clone(&assets.animation)).unwrap();
        let operation=PlayAnimation {
            animation:"S_R_PUSHLSP_HSTR_N_0_CYC1".into(),switch_animation:None,mirror_animation:None,no_board_animation:None,
            playback_speed:1.,apply_posture:true,parameters:vec![],
            transition:TransitionSettings {kind:1,seconds:0.,under:0,matching:0,use_channels_from_weights:false},
        };
        for natural in [0,1] {
            animation.set_natural_stance(natural);
            let mut poses=Vec::new();
            for style in 0..4 {for posture in 0..4 {
                let expected=skate2_formats::skater_profile::PcSkate3Style {style,posture};
                animation.set_style_preferences(expected).unwrap();
                let context=animation.motion.playback_context.pro_skater;
                // The game's per-tick stance refresh must not clear the choice.
                animation.set_natural_stance(natural);
                assert_eq!(animation.motion.playback_context.pro_skater,context);
                assert_eq!(animation.natural_stance(),natural as i32);
                assert_eq!(animation.motion.animation.posture.profile(),posture);
                let motion=&mut animation.motion;
                PlayAnimationInstance::default().begin(&operation,&mut motion.playback_context,&mut motion.animation).unwrap();
                motion.animation.apply_parameters().unwrap();
                let length=motion.animation.current_length().unwrap();
                motion.animation.advance(length*0.35,1.);
                let commands=motion.animation.evaluate_pose(Evaluation {cull_threshold:0.0001,update_history:false}).unwrap();
                let pose=animation.evaluator.evaluate(&commands).unwrap();
                assert!(pose.iter().flat_map(|v|v.scale.into_iter().chain(v.rotation).chain(v.translation)).all(f32::is_finite));
                if let Some(baseline)=poses.first() {assert!(&pose!=baseline,"style {style}, posture {posture}, stance {natural} matched default");}
                let names:Vec<_>=commands.iter().filter_map(|command|match command {PoseCommand::Clip {name,..}|PoseCommand::Pose {name}=>Some(name.as_str()),_=>None}).collect();
                eprintln!("style133 stance={natural} style={style} posture={posture} authored={names:?}");
                poses.push(pose);
            }}
        }
        let before=animation.motion.playback_context.pro_skater;
        assert!(animation.set_style_preferences(skate2_formats::skater_profile::PcSkate3Style {style:4,posture:0}).is_err());
        assert_eq!(animation.motion.playback_context.pro_skater,before);
    }
}
