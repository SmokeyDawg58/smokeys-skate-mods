//! Original-data cross-owner session gate. All native owners are constructed
//! through one `SelectedSession`; the pad, Juice, stream and frame values are
//! explicit first-test fixture policies.
use super::Platform as DisabledJuicePlatform;
use crate::{
    actor_construction::selected_tick::{
        SelectedQueuedTickOutcome, SelectedTickInputDelivery, SelectedTickNumerics,
        SelectedTickRequest, SelectedTickStage,
    },
    actor_face_playback::presentation::frame::{
        RootPositionWTransport, SubmittedPresentationFrame,
        selected_runtime::{
            SelectedPresentationPolicy, SelectedPresentationRuntime, SelectedPresentationStage,
        },
    },
    collection_resident_streamer::CollectionId,
    game::{CommonBackEndSeed, WorldStreamPoll, WorldStreamSpec},
    gesture_trick_mapping::MappingAllocation,
    input_pad::device::tests::Platform as PadPlatform,
    juice_trick_input::boot::JuiceHostOption,
    native_skater_random::{NativeRandomGate, SimulationCommandRandom},
    physical_player_assembly::PhysicalAssemblyWorld,
    player::{DEFAULT_MARQUEE_ID, PlayerLoadPhase},
    reckoning_numeric::HostReckoningNumeric,
    reckoning_sin_cos::HostReckoningSinCosNumeric,
    selected_session_boot_owner::{ControlledPlayerPolicy, SelectedSession},
    selected_sim_command_queue::{SelectedSimCommandQueue, SimCommandCounters},
    simulation_assembly_backend::constraint_session::{
        ground_pre_solve::{GroundPreparationGrindPolicy, HostGroundPreparationNumeric},
        skateboard_ground::{SkateboardGroundFrameInput, SkateboardGroundFrameSchedule},
        update5_managers::GrindManagerFillPolicy,
    },
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
    skateboard_attributes::SkateboardAttributes,
    world_stream_catalog::WorldStreamCatalog,
    world_stream_runtime::WorldStreamEffects,
};
use std::{
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Default)]
struct StreamCounts {
    high: usize,
    finalized: usize,
}
struct StreamObservations(Arc<Mutex<StreamCounts>>);
impl WorldStreamEffects for StreamObservations {
    fn add_high_collection(&mut self, _id: u64, _x: u32, _z: u32) {
        self.0.lock().unwrap().high += 1;
    }
    fn finalize_world_bounds(&mut self) {
        self.0.lock().unwrap().finalized += 1;
    }
}
#[expect(
    clippy::unnecessary_wraps,
    reason = "the selected thread provider's fallible callback signature is explicit"
)]
fn selected_thread() -> Result<u64, Infallible> {
    Ok(1)
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "one original-data test crosses the retained boot, terrain, input, camera and frame owners"
)]
fn original_session_empty_and_queued_frame_reach_same_camera_and_palette() {
    let root = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"));
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let mut session =
        SelectedSession::<fn() -> Result<u64, Infallible>, u32>::new(Arc::clone(&mounts), &root);
    let mut pad = PadPlatform::new();
    let mut juice_platform = DisabledJuicePlatform::new();
    juice_platform.initialized = false;
    let juice_ini = std::fs::read(root.join("data").join("juice").join("Juice_m.ini"))
        .expect("original Juice config");
    session
        .try_boot_prefix(
            &mut pad,
            &mut juice_platform,
            Some(&juice_ini),
            JuiceHostOption::DisableBeforeStart,
        )
        .unwrap();
    let assets = session.assets.as_ref().expect("one asset owner");
    assert!(Arc::ptr_eq(&assets.mounts, &mounts));
    let spawn = assets.spawn;
    let vault = Arc::clone(&assets.vault);
    let cvars = assets.cvars.clone();
    let layout = vault
        .physics(SkateboardAttributes::DEFAULT_COLLECTION)
        .try_physics_layout()
        .unwrap();
    let gravity = std::array::from_fn(|axis| {
        f32::from_be_bytes(layout.raw_bytes[axis * 4..axis * 4 + 4].try_into().unwrap())
    });
    session
        .world_handle()
        .unwrap()
        .lock()
        .unwrap()
        .backend_mut()
        .set_gravity(gravity);
    session
        .try_continue_controlled_player(
            ControlledPlayerPolicy::HostSpawnWhileRegistrationPending,
            spawn,
            DEFAULT_MARQUEE_ID,
            1,
            selected_thread,
        )
        .unwrap();
    let player = session.player.as_ref().unwrap();
    assert!(matches!(
        player.selected.owners.loader.phase(),
        PlayerLoadPhase::Registering { .. }
    ));
    let actor = Arc::clone(player.selected.actor.as_ref().unwrap());
    session
        .try_continue_physical(
            &mut |_, _, _| Ok::<_, Infallible>(MappingAllocation::Host),
            &mut HostReckoningNumeric,
            true,
            &mut || panic!("native spawn orthogonality assertion"),
            &mut || panic!("original hat0"),
        )
        .unwrap();
    session
        .try_continue_controlled_presentation::<Infallible>()
        .unwrap();

    let counts = Arc::new(Mutex::new(StreamCounts::default()));
    session
        .try_select_terrain(
            WorldStreamCatalog::new(),
            WorldStreamSpec::derive("BAM", None).unwrap(),
            StreamObservations(Arc::clone(&counts)),
            // Empty contexts/zero lock are the original constructor state in
            // the terrain corpus; this is explicit test policy here.
            EnvironmentUnlockSeed {
                locations_lock_status_raw: 0,
                contexts_in_native_iteration_order: Vec::new(),
            },
            CollectionId(0x823c_4449_fb31_63b4),
            false,
            0,
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        assert!(Instant::now() < deadline, "BAM stream timed out");
        match session.terrain.as_mut().unwrap().try_poll_stream().unwrap() {
            WorldStreamPoll::Pending => std::thread::yield_now(),
            WorldStreamPoll::CompletedThisPoll => break,
        }
    }
    assert!(counts.lock().unwrap().high > 0);
    assert_eq!(counts.lock().unwrap().finalized, 1);
    session.terrain.as_mut().unwrap().try_request().unwrap();
    loop {
        assert!(Instant::now() < deadline, "normal resident timed out");
        if session.terrain.as_mut().unwrap().try_poll().unwrap() {
            break;
        }
        std::thread::yield_now();
    }
    let world_handle = Arc::clone(session.world_handle().unwrap());
    assert!(
        session
            .terrain
            .as_ref()
            .unwrap()
            .ground_pool_for(&world_handle)
            .unwrap()
            .published_volume_count()
            > 0
    );
    session
        .try_admit_selected_camera_and_roster(CommonBackEndSeed::debug_reference().camera_set)
        .unwrap();
    {
        let world = world_handle.lock().unwrap();
        let roster = world.entity_roster.as_ref().unwrap();
        assert!(roster.contains(&actor).unwrap());
        assert!(Arc::ptr_eq(
            &world
                .local_skater_identity
                .as_ref()
                .unwrap()
                .try_local_actor()
                .unwrap()
                .unwrap(),
            &actor,
        ));
    }

    let queue = SelectedSimCommandQueue::construct_offline_command_fields(
        SimCommandCounters::from_static_initializer(),
    );
    let random =
        SimulationCommandRandom::construct_field(NativeRandomGate::from_static_initializer());
    session
        .try_bind_selected_input(queue, random)
        .unwrap_or_else(|error| {
            panic!(
                "production input bind: {:?}",
                std::mem::discriminant(&error)
            )
        });
    session
        .try_prepare_selected_queued_frame(
            SelectedPresentationPolicy {
                lod: 0,
                submission_capacity: 65536,
                gather_capacity: 65536,
                provider: 0,
                root_w: RootPositionWTransport::ReadSkaterOpaque { bits: 0 },
            },
            3,
        )
        .unwrap();

    let (mut sync, mut feedback, overlay) = {
        let world = world_handle.lock().unwrap();
        (
            Arc::clone(world.physical_world_services().unwrap().game_info()),
            world.try_live_contact_feedback(&cvars).unwrap(),
            world.debug_overlay.clone(),
        )
    };
    let mut recognizer =
        crate::game_input_recognizer_runtime_services::GameInputRuntimeServices::new(
            &cvars, &vault, &overlay, None,
        );
    let mut numeric = SelectedTickNumerics {
        input: HostReckoningNumeric,
        actor: HostReckoningNumeric,
        pre: HostGroundPreparationNumeric::default(),
        line: crate::collision_line_numeric::HostCollisionLineNumeric,
        vector: HostReckoningNumeric,
        rounding: HostReckoningSinCosNumeric,
        truck: crate::skateboard_truck_drives::HostSkateboardTruckDriveNumeric,
        collision: crate::skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    };
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut empty_stages = 0;
    let mut observe_empty = |_: SelectedTickStage, _: &PhysicalAssemblyWorld| empty_stages += 1;
    let empty = session
        .try_with_selected_queued_frame(|mut frame, ground, queued| {
            frame.try_step_queued::<_, _, _, _, _, _, Infallible>(
                SelectedTickRequest {
                    input: queued,
                    dt: 1.0 / 60.0,
                    lod: 0,
                    wait_for_bs_job: true,
                    physics: SkateboardGroundFrameInput {
                        schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool: ground,
                        ground_group: 0,
                        pipeline_culling: None,
                        allow_multiple_writes: true,
                        spy_flags: 4,
                    },
                    pre_grind: GroundPreparationGrindPolicy::ParkedByUser,
                    post_grind: GrindManagerFillPolicy::ExcludeForFirstSkatingTest,
                },
                &mut numeric,
                &mut sync,
                &mut recognizer,
                &mut feedback,
                &mut clamp,
                &mut HostReckoningNumeric,
                Some(&mut observe_empty),
                None,
            )
        })
        .unwrap()
        .unwrap();
    assert!(
        matches!(empty, SelectedQueuedTickOutcome::NoCommands(report) if report.update_mask == 0 && report.delivered == 0)
    );
    assert_eq!(empty_stages, 0);
    assert!(
        session
            .presentation_runtime
            .as_ref()
            .unwrap()
            .palette()
            .is_err()
    );

    let game_info = Arc::clone(&sync);
    let input = session.bound_input.as_mut().unwrap();
    input.poll_and_cache(&mut pad).unwrap();
    assert!(
        input
            .try_preupdate_enqueue(
                &mut pad,
                &session.player.as_ref().unwrap().selected.owners.loader,
                true,
                true,
                &game_info,
            )
            .unwrap()
    );
    let mut tick_stages = 0;
    let mut published = 0;
    let mut observe_tick = |_: SelectedTickStage, _: &PhysicalAssemblyWorld| tick_stages += 1;
    let mut observe_presentation =
        |stage: SelectedPresentationStage,
         _: &SelectedPresentationRuntime,
         _: &SubmittedPresentationFrame| {
            if matches!(stage, SelectedPresentationStage::Published) {
                published += 1;
            }
            Ok::<(), Infallible>(())
        };
    let completed = session
        .try_with_selected_queued_frame(|mut frame, ground, queued| {
            frame.try_step_queued(
                SelectedTickRequest {
                    input: queued,
                    dt: 1.0 / 60.0,
                    lod: 0,
                    wait_for_bs_job: true,
                    physics: SkateboardGroundFrameInput {
                        schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                        ground_pool: ground,
                        ground_group: 0,
                        pipeline_culling: None,
                        allow_multiple_writes: true,
                        spy_flags: 4,
                    },
                    pre_grind: GroundPreparationGrindPolicy::ParkedByUser,
                    post_grind: GrindManagerFillPolicy::ExcludeForFirstSkatingTest,
                },
                &mut numeric,
                &mut sync,
                &mut recognizer,
                &mut feedback,
                &mut clamp,
                &mut HostReckoningNumeric,
                Some(&mut observe_tick),
                Some(&mut observe_presentation),
            )
        })
        .unwrap()
        .unwrap();
    let SelectedQueuedTickOutcome::Completed(completed) = completed else {
        panic!("queued hardware sample did not reach selected frame");
    };
    assert!(tick_stages > 0);
    assert_eq!(published, 1);
    assert!(
        session
            .presentation_runtime
            .as_ref()
            .unwrap()
            .palette()
            .is_ok()
    );
    assert!(
        session
            .presentation_runtime
            .as_ref()
            .unwrap()
            .materials()
            .is_ok()
    );
    match &completed.input {
        SelectedTickInputDelivery::Queued { commands, .. } => {
            assert_eq!(
                (
                    commands.update_mask,
                    commands.delivered,
                    commands.missing_receivers
                ),
                (3, 2, 0)
            );
        }
        SelectedTickInputDelivery::Immediate(_) => panic!("expected production queued owner"),
    }
    let camera = world_handle
        .lock()
        .unwrap()
        .local_skater_identity
        .as_ref()
        .unwrap()
        .try_camera_producer_projection()
        .unwrap();
    assert_eq!(camera.camera_update_counter, 1);
    assert_eq!(
        camera.near_plane.to_bits(),
        cvars.try_camera_near_clip_distance().unwrap().to_bits()
    );
}
