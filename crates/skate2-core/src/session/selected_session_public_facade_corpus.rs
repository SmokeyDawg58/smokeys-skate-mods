//! Original-data public facade smoke gate. This uses the same mounted corpus,
//! pad and disabled Juice fixtures as the retained cross-owner test; all boot
//! and frame answers below are explicit controlled first-test host policy.
#[path = "selected_session_grab_bail_corpus.rs"]
mod grab_bail_corpus;
use super::Platform as DisabledJuicePlatform;
use crate::{
    SelectedBootProgress, SelectedCommandStartPolicy, SelectedControlledBootPolicy,
    SelectedFrameNumerics, SelectedGestureAllocation, SelectedGravityPolicy, SelectedLiveError,
    SelectedLivePending, SelectedLiveSession, SelectedLiveStage, SelectedQueuedFrameOutcome,
    SelectedQueuedFramePolicy, SelectedRootWPolicy, SelectedSpawnPolicy,
    collection_resident_streamer::CollectionId,
    game::{CommonBackEndSeed, WorldStreamSpec},
    input_pad::device::tests::Platform as PadPlatform,
    juice_trick_input::boot::JuiceHostOption,
    player::DEFAULT_MARQUEE_ID,
    simulation_environment_unlock_router::EnvironmentUnlockSeed,
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
    reason = "the selected thread callback has a fallible native boundary"
)]
fn selected_thread() -> Result<u64, Infallible> {
    Ok(1)
}

type OriginalFacade = SelectedLiveSession<fn() -> Result<u64, Infallible>, u32>;
#[expect(
    clippy::too_many_lines,
    reason = "original-data boot and first visual proof"
)]
fn boot_original_first_visual() -> (Box<OriginalFacade>, PadPlatform, SelectedQueuedFramePolicy) {
    let root = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("original game root"));
    let mounts =
        Arc::new(crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_corpus());
    let mut session = Box::new(OriginalFacade::new(mounts, &root));
    assert!(matches!(
        session.try_poll_controlled_boot(
            &mut |_, _, _| Ok::<_, Infallible>(SelectedGestureAllocation::Host),
            &mut || panic!("native spawn orthogonality assertion"),
            &mut || panic!("original hat0"),
        ),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::BootNotStarted
        ))
    ));
    assert!(matches!(
        session.try_visual_snapshot(),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::NoCompletedFrame
        ))
    ));

    let mut pad = PadPlatform::new();
    let mut juice = DisabledJuicePlatform::new();
    juice.initialized = false;
    // Optional, as the PC client reads it (an absent file is None).
    let juice_ini = std::fs::read(root.join("data").join("juice").join("Juice_m.ini")).ok();
    let counts = Arc::new(Mutex::new(StreamCounts::default()));
    let boot = SelectedControlledBootPolicy {
        spawn_policy: SelectedSpawnPolicy::AuthoredSelectedLocation,
        marquee: DEFAULT_MARQUEE_ID,
        actor_flags: 1,
        capsule_resource_allocated: true,
        camera_set: CommonBackEndSeed::debug_reference().camera_set.to_owned(),
        terrain_catalog: WorldStreamCatalog::new(),
        terrain_spec: WorldStreamSpec::derive("BAM", None).unwrap(),
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
        presentation_root_w: SelectedRootWPolicy::ReadSkaterOpaque { bits: 0 },
        camera_submit_mask: 3,
        gravity_policy: SelectedGravityPolicy::VaultSkateboardDefault,
        command_start: SelectedCommandStartPolicy::NativeStaticFields,
    };
    assert_eq!(
        session
            .try_begin_controlled(
                &mut pad,
                &mut juice,
                juice_ini.as_deref(),
                JuiceHostOption::DisableBeforeStart,
                selected_thread,
                StreamObservations(Arc::clone(&counts)),
                boot,
            )
            .unwrap(),
        SelectedBootProgress::TerrainStreamPending
    );
    assert!(!session.controlled_frame_ready());
    assert_eq!(
        session.controlled_player_and_pair_present_for_test(),
        (false, false)
    );
    assert!(matches!(
        session.try_visual_snapshot(),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::NoCompletedFrame
        ))
    ));

    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        assert!(Instant::now() < deadline, "selected terrain boot timed out");
        match session
            .try_poll_controlled_boot(
                &mut |_, _, _| Ok::<_, Infallible>(SelectedGestureAllocation::Host),
                &mut || panic!("native spawn orthogonality assertion"),
                &mut || panic!("original hat0"),
            )
            .unwrap()
        {
            SelectedBootProgress::TerrainStreamPending
            | SelectedBootProgress::TerrainResidentPending => {
                assert_eq!(
                    session.controlled_player_and_pair_present_for_test(),
                    (false, false)
                );
                std::thread::yield_now();
            }
            SelectedBootProgress::ControlledFrameReady => break,
            SelectedBootProgress::DmoBankPending => std::thread::yield_now(),
        }
    }
    assert!(session.controlled_frame_ready());
    assert_eq!(
        session.controlled_player_and_pair_present_for_test(),
        (true, true)
    );
    // Island gravity must be selected before AddRigidBody copies it into
    // every newly admitted board body's first force. A later pool-only setter
    // would leave those bodies with Simulation::Initialize's -600 instead.
    {
        let (gravity, forces) = session.controlled_initial_body_forces_for_test();
        let expected = [0_u32, 0xc11c_cccd, 0];
        assert_eq!(gravity.map(f32::to_bits), expected);
        assert_eq!(forces.len(), 7);
        for force in forces {
            assert_eq!(force.map(f32::to_bits), expected);
        }
    }
    assert!(counts.lock().unwrap().high > 0);
    assert_eq!(counts.lock().unwrap().finalized, 1);
    assert!(matches!(
        session.try_poll_controlled_boot(
            &mut |_, _, _| Ok::<_, Infallible>(SelectedGestureAllocation::Host),
            &mut || panic!("native spawn orthogonality assertion"),
            &mut || panic!("original hat0"),
        ),
        Err(SelectedLiveError::AlreadyAttempted)
    ));

    let frame = SelectedQueuedFramePolicy {
        dt: 1.0 / 60.0,
        lod: 0,
        wait_for_bs_job: true,
        can_simulate: true,
        has_focus: true,
        ground_group: 0,
        allow_multiple_writes: true,
        spy_flags: 4,
        numerics: SelectedFrameNumerics::ExistingHostApproximation,
        physics_flags3: true,
        park_grind_preparation: true,
        exclude_grind_fill: true,
    };
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    assert!(matches!(
        session.try_tick(
            &mut pad,
            SelectedQueuedFramePolicy {
                physics_flags3: false,
                ..frame
            },
            &mut clamp
        ),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::UnsourcedFramePolicy("physical schedule")
        ))
    ));
    assert!(matches!(
        session.try_visual_snapshot(),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::NoCompletedFrame
        ))
    ));
    let result = session.try_tick(&mut pad, frame, &mut clamp).unwrap();
    assert!(matches!(
        result,
        SelectedQueuedFrameOutcome::Completed {
            update_mask: 3,
            delivered: 2,
            missing_receivers: 0
        }
    ));
    {
        let view = session.try_visual_snapshot().unwrap();
        assert_eq!(view.camera_update_counter, 1);
        assert!(
            view.provisional_clip_from_world
                .iter()
                .all(|value| value.is_finite())
        );
        let _same_frame_palette = view.palette;
        let _same_frame_materials = view.materials;
    }

    (session, pad, frame)
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "one public owner must cross original-data boot, terrain, input, camera and visual frame"
)]
fn original_selected_live_facade_reaches_one_queued_visual_frame() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");

    // Keep the same public owner through an authored raw-pad Kickflip. The
    // lower-level 120-tick corpus already proves this gesture's detailed
    // mapping; this checks the new facade boundary through the actual Actor,
    // post-solve and selected visual publication on every tick.
    let points = session.controlled_right_pattern_points_for_test("Kickflip");
    assert!(points.len() >= 2);
    let start = 6;
    let end = start + points.len();
    let mut matched = false;
    for tick in 1..=end + 1 {
        pad.scripted_controller_zero = if (start..end).contains(&tick) {
            Some(crate::input_pad::device::tests::scripted_right_point(
                points[tick - start],
            ))
        } else if tick == end {
            Some(crate::input_pad::device::tests::scripted_right_point(
                *points.last().unwrap(),
            ))
        } else {
            None
        };
        assert!(matches!(
            session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| panic!("public facade frame {tick}: {error:?}")),
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let (state, [trick, kickflip, held]) =
            session.controlled_action_observation_for_test(b"Kickflip");
        if (start..end).contains(&tick) && kickflip == Some(1.0_f32.to_bits()) {
            assert_eq!(trick, Some(1.0_f32.to_bits()));
            assert_eq!(state, 52);
            matched = true;
        }
        if tick == end {
            assert!(
                matched,
                "authored Kickflip never reached public facade Action"
            );
            assert_eq!(held, Some(1.0_f32.to_bits()));
        }
        if tick == end + 1 {
            assert_eq!(state, 94);
            assert_eq!(held, None);
        }
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            (tick + 1) as u32,
        );
    }

    // The original Ollie pattern has two chronological right-stick points.
    // Replay the same start, neutral, end, held-end and release cadence that
    // the retained recognizer corpus uses, through this public frame owner.
    let ollie = session.controlled_right_pattern_points_for_test("Ollie");
    assert_eq!(ollie.len(), 2);
    let mut visual_counter = (end + 2) as u32;
    let mut ollie_motion_states = Vec::new();
    for step in 0..12 {
        pad.scripted_controller_zero = match step {
            6 | 7 => Some(crate::input_pad::device::tests::scripted_right_point(
                ollie[0],
            )),
            9 | 10 => Some(crate::input_pad::device::tests::scripted_right_point(
                ollie[1],
            )),
            _ => None,
        };
        assert!(matches!(
            session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| panic!("public facade Ollie step {step}: {error:?}")),
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let (_, [trick, named, held]) = session.controlled_action_observation_for_test(b"Ollie");
        let (motion_state, motion_intents) =
            session.controlled_motion_observation_for_test(b"Ollie");
        ollie_motion_states.push((step, motion_state, motion_intents));
        if step == 9 {
            assert_eq!(trick, Some(1.0_f32.to_bits()));
            assert_eq!(named, Some(1.0_f32.to_bits()));
            assert_eq!(held, None);
        }
        visual_counter += 1;
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            visual_counter,
        );
    }
    println!("scripted Ollie Motion states/intents: {ollie_motion_states:?}");

    // A reached controller-zero poll failure is terminal. A later tick must
    // not replay input or frame writes.
    pad.fail_index = Some(0);
    assert!(matches!(
        session.try_tick(&mut pad, frame, &mut clamp),
        Err(SelectedLiveError::Failed {
            stage: SelectedLiveStage::DevicePoll,
            ..
        })
    ));
    let calls_after_failure = pad.calls.len();
    assert!(matches!(
        session.try_tick(&mut pad, frame, &mut clamp),
        Err(SelectedLiveError::AlreadyFailed(
            SelectedLiveStage::DevicePoll
        ))
    ));
    assert_eq!(pad.calls.len(), calls_after_failure);
    assert!(matches!(
        session.try_visual_snapshot(),
        Err(SelectedLiveError::Pending(
            SelectedLivePending::NoCompletedFrame
        ))
    ));
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_fresh_ollie_motion_route() {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let points = session.controlled_right_pattern_points_for_test("Ollie");
    assert_eq!(points.len(), 2);
    let mut action_matched = false;
    let mut trace = Vec::new();
    for step in 0..60 {
        pad.scripted_controller_zero = match step {
            6 | 7 => Some(crate::input_pad::device::tests::scripted_right_point(
                points[0],
            )),
            9 | 10 => Some(crate::input_pad::device::tests::scripted_right_point(
                points[1],
            )),
            _ => None,
        };
        let outcome = session.try_tick(&mut pad, frame, &mut clamp).unwrap_or_else(|error| {
            let selected_action = session.controlled_action_observation_for_test(b"Ollie");
            let selected_motion = session.controlled_motion_observation_for_test(b"Ollie");
            let selected_physical = session.controlled_physical_state_for_test();
            let selected_stance = session.controlled_relative_stance_for_test();
            let jump_gate = session.controlled_jump_gate_for_test();
            panic!("fresh Ollie frame {step}: {error:?}; selected action/motion/physical/stance/jump-gate: {selected_action:?}/{selected_motion:?}/{selected_physical:?}/{selected_stance:?}/{jump_gate:?}; completed trace {trace:?}");
        });
        assert!(matches!(
            outcome,
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let action = session.controlled_action_observation_for_test(b"Ollie");
        let (_, [trick, ollie, _]) = action;
        if ollie == Some(1.0_f32.to_bits()) {
            assert_eq!(trick, Some(1.0_f32.to_bits()));
            action_matched = true;
        }
        let motion = session.controlled_motion_observation_for_test(b"Ollie");
        let physical = session.controlled_physical_state_for_test();
        let stance = session.controlled_relative_stance_for_test();
        let jump_gate = session.controlled_jump_gate_for_test();
        trace.push((step, action, motion, physical, stance, jump_gate));
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            (step + 2) as u32,
        );
    }
    assert!(action_matched, "fresh Ollie never reached Action");
    let ollie_motion = trace
        .iter()
        .position(|(_, _, motion, _, _, _)| motion.0 == 789)
        .expect("fresh Ollie never selected Motion FromAntic789");
    let ground_animation = trace
        .iter()
        .position(|(_, _, _, physical, _, _)| *physical == CurrentPhysicalState::GroundAnimation)
        .expect("fresh Ollie never entered physical GroundAnimation103");
    let jumped = trace
        .iter()
        .position(|(_, _, _, _, _, ((bits, known), ground))| {
            bits & known & 0x0040_0000 != 0
                && ground.is_some_and(|(jump, this_frame)| {
                    jump == Some(true) && this_frame == Some(true)
                })
        })
        .expect("fresh Ollie never emitted StartJump and launched");
    let air = trace
        .iter()
        .position(|(_, _, _, physical, _, _)| *physical == CurrentPhysicalState::KnownAir)
        .expect("flatground Ollie never acquired a landing trajectory (KnownAir201)");
    let _landed = trace
        .iter()
        .skip(air + 1)
        .position(|(_, _, _, physical, _, _)| *physical == CurrentPhysicalState::Ground)
        .expect("fresh Ollie never landed back on Ground100");
    assert!(ollie_motion < ground_animation && ground_animation < jumped && jumped < air);
    println!("fresh scripted Ollie Action/Motion/physical/stance/jump-gate trace: {trace:?}");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_kickflip_long_hold_route() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let points = session.controlled_right_pattern_points_for_test("Kickflip");
    assert!(points.len() >= 2);

    // Replay the authored points once, then keep the last authored endpoint
    // before releasing it. The optional end step probes shorter releases
    // through the same retained recognizer and graph/physics schedule.
    let start = 6;
    let hold_end = std::env::var("SKATE2_KICKFLIP_HOLD_END_STEP")
        .ok()
        .map(|value| value.parse::<usize>().expect("numeric hold-end step"))
        .unwrap_or(start + points.len() + 180);
    assert!(hold_end > start + points.len());
    let release_end = hold_end + 30;
    let mut saw_match = false;
    let mut saw_hold = false;
    let mut trace = Vec::new();
    for step in 0..release_end {
        pad.scripted_controller_zero = if (start..start + points.len()).contains(&step) {
            Some(crate::input_pad::device::tests::scripted_right_point(
                points[step - start],
            ))
        } else if (start + points.len()..hold_end).contains(&step) {
            Some(crate::input_pad::device::tests::scripted_right_point(
                *points.last().unwrap(),
            ))
        } else {
            None
        };
        let outcome = session.try_tick(&mut pad, frame, &mut clamp).unwrap_or_else(|error| {
            let action = session.controlled_action_observation_for_test(b"Kickflip");
            let motion = session.controlled_motion_observation_for_test(b"Kickflip");
            let motion_hold = session.controlled_motion_observation_for_test(b"KickflipHold").1[1];
            let physical = session.controlled_physical_state_for_test();
            let runout = session.controlled_runout_request_for_test();
            let offboard_flags = session.controlled_offboard_processed_flags_for_test();
            panic!(
                "Kickflip long hold step {step}: {error:?}; Action/Motion/KickflipHold/physical/runout/processed8cc8d0: {action:?}/{motion:?}/{motion_hold:?}/{physical:?}/{runout:?}/{offboard_flags:?}; completed trace: {trace:?}"
            );
        });
        assert!(
            matches!(
                outcome,
                SelectedQueuedFrameOutcome::Completed {
                    update_mask: 3,
                    missing_receivers: 0,
                    ..
                }
            ),
            "Kickflip long hold step {step}: {outcome:?}"
        );
        let action = session.controlled_action_observation_for_test(b"Kickflip");
        let motion = session.controlled_motion_observation_for_test(b"Kickflip");
        let motion_hold = session
            .controlled_motion_observation_for_test(b"KickflipHold")
            .1[1];
        let physical = session.controlled_physical_state_for_test();
        if matches!(step, 42 | 43) {
            println!(
                "Kickflip runout source step {step}: {:?}",
                session.controlled_runout_request_for_test()
            );
        }
        saw_match |= action.1[1] == Some(1.0_f32.to_bits());
        saw_hold |= action.1[2] == Some(1.0_f32.to_bits());
        trace.push((step, action, motion, motion_hold, physical));
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            (step + 2) as u32,
        );
    }
    assert!(saw_match, "Kickflip never reached Action; trace: {trace:?}");
    assert!(
        saw_hold,
        "Kickflip never produced HoldPattern; trace: {trace:?}"
    );
    println!("scripted Kickflip long hold Action/Motion/physical trace: {trace:?}");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_flatground_flip_family_routes() {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let release_step = std::env::var("SKATE2_FLIP_FAMILY_RELEASE_STEP")
        .ok()
        .map(|value| value.parse::<usize>().expect("numeric family release step"))
        .unwrap_or(24);
    let mut failures = Vec::new();
    for (name, left_ground, first_out, family) in [
        ("Kickflip", 792, 1079, 793..=795),
        ("Heelflip", 799, 1087, 800..=802),
        ("N_Kickflip", 871, 1095, 872..=874),
        ("N_Heelflip", 878, 1103, 879..=881),
    ] {
        let (mut session, mut pad, frame) = boot_original_first_visual();
        let points = session.controlled_right_pattern_points_for_test(name);
        assert!(points.len() >= 2, "{name} authored pattern");
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        let mut trace = Vec::new();
        let mut saw_cycle = false;
        let mut saw_takeoff = false;
        let mut saw_first_out = false;
        let mut saw_air = false;
        let mut saw_landing = false;
        let mut failed = false;
        let start_y = session.controlled_board_kinematics_for_test().0[1];
        let mut peak_y = start_y;
        let mut minimum_deck_up = 1.0_f32;
        for step in 0..75 {
            pad.scripted_controller_zero = if (6..release_step).contains(&step) {
                Some(crate::input_pad::device::tests::scripted_right_point(
                    points[(step - 6).min(points.len() - 1)],
                ))
            } else {
                None
            };
            if let Err(error) = session.try_tick(&mut pad, frame, &mut clamp) {
                failures.push(format!("{name} step {step}: {error:?}; trace {trace:?}"));
                failed = true;
                break;
            }
            let action = session.controlled_action_observation_for_test(name.as_bytes());
            let motion = session.controlled_motion_observation_for_test(name.as_bytes());
            let physical = session.controlled_physical_state_for_test();
            peak_y = peak_y.max(session.controlled_board_kinematics_for_test().0[1]);
            let (_, _, _, _, _, board_transform) = session
                .controlled_board_turn_for_test()
                .expect("actual Part6 transform after completed flip tick");
            minimum_deck_up = minimum_deck_up.min(board_transform[5]);
            saw_cycle |= family.contains(&motion.0);
            saw_takeoff |= motion.0 == left_ground;
            saw_first_out |= motion.0 == first_out;
            saw_landing |= saw_air && physical == CurrentPhysicalState::Ground;
            saw_air |= physical == CurrentPhysicalState::KnownAir;
            trace.push((step, action.0, motion.0, physical));
        }
        // Authored LeftGround has both direct first-Out and held-cycle routes.
        // The default short release must complete takeoff, first Out and landing;
        // the explicit long-hold regression additionally requires a cycle.
        let route_complete = saw_takeoff
            && saw_air
            && saw_landing
            && peak_y - start_y > 0.05
            && minimum_deck_up < 0.0
            && if release_step < 38 {
                saw_first_out
            } else {
                saw_cycle
            };
        if !failed && !route_complete {
            failures.push(format!("{name}: takeoff={saw_takeoff}, out={saw_first_out}, cycle={saw_cycle}, air={saw_air}, landing={saw_landing}, board rise={}, minimum deck up={minimum_deck_up}; {trace:?}", peak_y-start_y));
        }
        println!(
            "{name} takeoff={saw_takeoff}, out={saw_first_out}, cycle={saw_cycle}, air={saw_air}, landing={saw_landing}, board rise={}, minimum deck up={minimum_deck_up}",
            peak_y - start_y
        );
    }
    assert!(
        failures.is_empty(),
        "flip-family replay failures:\n{}",
        failures.join("\n")
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_grounded_right_push_route() {
    grounded_push_route(0x1000, b"RightPush", 44, false);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_grounded_held_x_push_route() {
    grounded_push_route(0x4000, b"LeftPush", 212, false);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_grounded_held_a_push_route() {
    grounded_push_route(0x1000, b"RightPush", 212, false);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_grounded_turned_held_a_push_route() {
    grounded_push_route(0x1000, b"RightPush", 212, true);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_diagonal_anticipation_route() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut seen = std::collections::BTreeSet::new();
    // Reproduce bottom-right, then the other corners through real pad input.
    for (x, y) in [
        (23_000, -23_000),
        (-23_000, -23_000),
        (23_000, 23_000),
        (-23_000, 23_000),
    ] {
        for tick in 0..112 {
            pad.scripted_controller_zero =
                (32..80)
                    .contains(&tick)
                    .then_some(crate::input_pad::XboxPadSample {
                        buttons: 0,
                        left_trigger: 0,
                        right_trigger: 0,
                        left_x: 0,
                        left_y: 0,
                        right_x: x,
                        right_y: y,
                    });
            session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| {
                    panic!("diagonal ({x},{y}) tick {tick}: {error:?}; seen {seen:?}")
                });
            let action = session
                .controlled_action_observation_for_test(b"AnticMag")
                .0;
            if (164..=179).contains(&action) {
                seen.insert(action);
            }
        }
    }
    assert!(
        seen.len() >= 4,
        "diagonal inputs must reach four directional leaves: {seen:?}"
    );
    println!("diagonal anticipation leaves: {seen:?}");
}

fn grounded_push_route(button: u16, push_intent: &[u8], release_tick: usize, turn_first: bool) {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    if turn_first {
        let initial = session.controlled_board_turn_for_test().unwrap().5;
        let mut turned = false;
        for step in 0..240 {
            pad.scripted_controller_zero = Some(crate::input_pad::XboxPadSample {
                buttons: 0,
                left_trigger: 0,
                right_trigger: 0,
                left_x: -21_930,
                left_y: 2_322,
                right_x: 0,
                right_y: 0,
            });
            session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| panic!("pre-push turn {step}: {error:?}"));
            let current = session.controlled_board_turn_for_test().unwrap().5;
            let dot = initial[8] * current[8] + initial[10] * current[10];
            if dot < -0.1 {
                eprintln!(
                    "pre-push heading crossed90deg at {step}: initial={initial:?}, current={current:?}"
                );
                turned = true;
                break;
            }
        }
        assert!(turned, "stationary left-stick turn never crossed90deg");
        pad.scripted_controller_zero = None;
        for _ in 0..12 {
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    }
    let initial_camera_counter = session.try_visual_snapshot().unwrap().camera_update_counter;
    let push = crate::input_pad::XboxPadSample {
        buttons: button,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    };
    let mut trace = Vec::new();
    let mut board_samples = Vec::new();
    let mut push_chain_samples = Vec::new();
    let mut saw_new_push = false;
    let mut saw_right_push = false;
    let push_observation_end = release_tick.max(120);
    for step in 0..push_observation_end {
        if step == 32 {
            assert!(
                matches!(
                    session.controlled_physical_state_for_test(),
                    CurrentPhysicalState::Ground | CurrentPhysicalState::GroundAnimation
                ),
                "the raw push press must start from selected physical ground"
            );
        }
        pad.scripted_controller_zero = (32..release_tick).contains(&step).then_some(push);
        assert!(matches!(
            session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| panic!("right push frame {step}: {error:?}; {trace:?}")),
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let (action, [_, new_push, _]) = session.controlled_action_observation_for_test(b"NewPush");
        let (_, [_, right_push, _]) = session.controlled_action_observation_for_test(push_intent);
        let motion = session.controlled_motion_observation_for_test(b"Pushing");
        let physical = session.controlled_physical_state_for_test();
        saw_new_push |= new_push == Some(1.0_f32.to_bits());
        saw_right_push |= right_push == Some(1.0_f32.to_bits());
        if step == 32 {
            println!(
                "first push frame push_contact tree attribute: {:?}",
                session.controlled_tree_attribute_for_test("push_contact")
            );
        }
        trace.push((step, action, motion, physical, new_push, right_push));
        if step >= 31 {
            let push_chain = session
                .controlled_push_chain_for_test()
                .unwrap_or_else(|error| panic!("push chain frame {step}: {error}"));
            // A continuously held input must retain the native positive holding
            // cap through PushCycle. A reversed Vault key previously read zero.
            if step < release_tick && matches!(motion.0, 1282 | 1283) {
                assert!(
                    f32::from_bits(push_chain.0[1]) > 0.0,
                    "held push lost its strength at frame {step}: {push_chain:?}"
                );
            }
            let contact = session.controlled_tree_attribute_for_test("push_contact");
            push_chain_samples.push((step, motion.0, contact, push_chain));
        }
        if matches!(step, 31 | 32 | 43 | 44 | 51 | 80 | 119) || step + 1 == push_observation_end {
            board_samples.push((step, session.controlled_board_kinematics_for_test()));
        }
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            initial_camera_counter + step as u32 + 1,
        );
    }
    assert!(
        saw_new_push && saw_right_push,
        "raw button {button:04x} never produced push intents"
    );
    let start = board_samples[0].1;
    let end = board_samples.last().expect("final board sample").1;
    let travel = start
        .0
        .iter()
        .zip(end.0.iter())
        .map(|(a, b)| (b - a).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(
        travel > 0.3 && end.2 > 0.2,
        "original Vault-backed push did not accelerate the selected board: travel={travel}, end deck speed={}; samples={board_samples:?}",
        end.2
    );
    println!("scripted grounded right push trace: {trace:?}");
    println!("scripted grounded right push board samples: {board_samples:?}");
    println!("scripted grounded right push source chain: {push_chain_samples:?}");
    // The live failure occurred after 372 completed ticks. Preserve the
    // acceleration assertions above, then exercise real moving breadcrumbs.
    pad.scripted_controller_zero = None;
    for step in push_observation_end..480 {
        let result = session
            .try_tick(&mut pad, frame, &mut clamp)
            .unwrap_or_else(|error| panic!("post-push continuation frame {step}: {error:?}"));
        assert!(matches!(
            result,
            SelectedQueuedFrameOutcome::Completed { .. }
        ));
    }
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_crouch_hold_release_routes() {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let mut measured_rises = Vec::new();
    for (name, hold) in [("Ollie", 2), ("Ollie", 30), ("Nollie", 2), ("Nollie", 30)] {
        let (mut session, mut pad, frame) = boot_original_first_visual();
        let points = session.controlled_right_pattern_points_for_test(name);
        assert_eq!(points.len(), 2, "{name} authored pattern");
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        let start_y = session.controlled_board_kinematics_for_test().0[1];
        let mut peak_y = start_y;
        let mut saw_air = false;
        let mut landed = false;
        let mut trace = Vec::new();
        let flick = 32 + hold + 1;
        for step in 0..flick + 100 {
            pad.scripted_controller_zero = if (32..32 + hold).contains(&step) {
                Some(crate::input_pad::device::tests::scripted_right_point(
                    points[0],
                ))
            } else if (flick..flick + 2).contains(&step) {
                Some(crate::input_pad::device::tests::scripted_right_point(
                    points[1],
                ))
            } else {
                None
            };
            let result = session
                .try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| {
                    panic!("{name} hold={hold} frame={step}: {error:?}; {trace:?}")
                });
            assert!(matches!(
                result,
                SelectedQueuedFrameOutcome::Completed { .. }
            ));
            let physical = session.controlled_physical_state_for_test();
            let motion = session
                .controlled_motion_observation_for_test(name.as_bytes())
                .0;
            let y = session.controlled_board_kinematics_for_test().0[1];
            peak_y = peak_y.max(y);
            landed |= saw_air && physical == CurrentPhysicalState::Ground;
            saw_air |= physical == CurrentPhysicalState::KnownAir;
            trace.push((step, motion, physical, y));
        }
        assert!(
            saw_air && landed && peak_y - start_y > 0.05,
            "{name} hold={hold}: air={saw_air} landed={landed} height={}; {trace:?}",
            peak_y - start_y
        );
        println!(
            "{name} crouch {hold} ticks: physical board rise={}",
            peak_y - start_y
        );
        measured_rises.push((name, hold, peak_y - start_y));
    }
    for pair in measured_rises.chunks_exact(2) {
        assert_eq!(pair[0].0, pair[1].0);
        assert!(
            pair[1].2 > pair[0].2 + 0.1,
            "long crouch did not produce a distinctly higher physical jump: {pair:?}"
        );
    }
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_selected_live_facade_known_air_ollie_spin() {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let points = session.controlled_right_pattern_points_for_test("Ollie");
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut previous_heading: Option<f32> = None;
    let mut rotation = 0.0_f32;
    let mut peak_rotation = 0.0_f32;
    let mut known_air = false;
    let mut landed = false;
    for step in 0..170 {
        let airborne =
            session.controlled_physical_state_for_test() == CurrentPhysicalState::KnownAir;
        let mut sample =
            crate::input_pad::device::tests::scripted_right_point(if (32..62).contains(&step) {
                points[0]
            } else if (63..65).contains(&step) {
                points[1]
            } else {
                [0.0, 0.0]
            });
        if airborne {
            sample.left_x = i16::MAX;
        }
        pad.scripted_controller_zero = Some(sample);
        session
            .try_tick(&mut pad, frame, &mut clamp)
            .unwrap_or_else(|e| panic!("spin frame {step}: {e:?}"));
        let state = session.controlled_physical_state_for_test();
        if state == CurrentPhysicalState::KnownAir {
            known_air = true;
            let (_, _, _, _, _, board) = session.controlled_board_turn_for_test().unwrap();
            let heading = board[8].atan2(board[10]);
            if let Some(previous) = previous_heading {
                let delta = heading - previous;
                rotation += delta.sin().atan2(delta.cos());
                peak_rotation = peak_rotation.max(rotation.abs());
            }
            previous_heading = Some(heading);
        }
        landed |= known_air && state == CurrentPhysicalState::Ground;
    }
    println!(
        "KnownAir high ollie held left stick: {} degrees",
        peak_rotation.to_degrees()
    );
    assert!(
        known_air && landed,
        "spin must acquire a landing prediction and return to Ground"
    );
    assert!(
        peak_rotation > 3.0,
        "held high-ollie rotation should reach approximately180 degrees, got {}",
        peak_rotation.to_degrees()
    );
}

/// PC Object Dropper: a dropped board gets the on-foot prop menu, joins the session marker,
/// and using the marker puts it back at rest where it was added; removed, it stays put; the
/// menu's upright rights it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn object_dropper_board_joins_the_session_marker() {
    use crate::physical_player_physics_state::CurrentPhysicalState;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    assert!(session.test_has_session_marker(), "free skate has no session marker");
    let mut ticks = |session: &mut OriginalFacade, buttons: u16, count: usize| {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        for _ in 0..count {
            pad.scripted_controller_zero = Some(crate::input_pad::XboxPadSample {
                buttons,
                left_trigger: 0,
                right_trigger: 0,
                left_x: 0,
                left_y: 0,
                right_x: 0,
                right_y: 0,
            });
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    };
    // Y steps off the board.
    ticks(&mut session, 0, 30);
    ticks(&mut session, 0x8000, 1);
    ticks(&mut session, 0, 150);
    let state = session.controlled_physical_state_for_test();
    assert!(
        matches!(state, CurrentPhysicalState::OffBoard | CurrentPhysicalState::OffBoardLocomotion),
        "not on foot: {state:?}"
    );
    let (root, forward) = session.test_skater_root();
    let length = forward[0].hypot(forward[2]);
    let (fx, fz) = (forward[0] / length, forward[2] / length);
    // Beside the skater (its side axis), dropped from a little above its root.
    let at = [root[0] + fz * 0.9, root[1] + 0.3, root[2] - fx * 0.9];
    let drop = [fz, 0.0, -fx, 0.0, 0.0, 1.0, 0.0, 0.0, fx, 0.0, fz, 0.0, at[0], at[1], at[2], 1.0];
    assert!(session.test_drop_board(drop), "no room for the board");
    ticks(&mut session, 0, 150);
    let (object, deck, marker) = session.test_dropped_boards()[0];
    let deck = deck.expect("board deck");
    eprintln!("board marker: skater {root:?}, board deck {:?} up_y {}", &deck[12..15], deck[5]);
    assert!(marker.is_none());
    assert!(session.test_prop_menu().is_none(), "board menu shown without RB held");
    // RB held, as for a living-world prop.
    ticks(&mut session, 0x0200, 2);
    let menu = session.test_prop_menu().expect("no prop menu beside the dropped board");
    assert_eq!(menu.object, object, "the menu is for another object");
    assert!(!menu.params[3], "board already in the marker");

    // Add it to the marker: the saved transform is where it rests.
    session.test_queue_prop_action(object, 2);
    ticks(&mut session, 0x0200, 2);
    let saved = session.test_dropped_boards()[0].2.expect("board not added to the marker");
    let rest = session.test_dropped_boards()[0].1.unwrap();
    assert!((saved[12] - rest[12]).hypot(saved[14] - rest[14]) < 0.05, "saved {saved:?} vs rest {rest:?}");
    assert!(session.test_prop_menu().expect("menu").params[3], "menu does not show membership");

    let away = |saved: [f32; 16]| {
        let mut moved = saved;
        moved[12] += fz * 3.0;
        moved[14] -= fx * 3.0;
        moved[13] += 0.3;
        moved
    };
    let distance = |deck: [f32; 16]| (deck[12] - saved[12]).hypot(deck[14] - saved[14]);
    session.test_place_dropped_board(object, away(saved));
    ticks(&mut session, 0, 90);
    let moved = session.test_dropped_boards()[0].1.unwrap();
    assert!(distance(moved) > 2.0, "board did not move away: {}", distance(moved));

    // Using the marker puts it back, at rest.
    session.test_restore_marker_props();
    ticks(&mut session, 0, 60);
    let back = session.test_dropped_boards()[0].1.unwrap();
    ticks(&mut session, 0, 30);
    let later = session.test_dropped_boards()[0].1.unwrap();
    let drift = (later[12] - back[12]).hypot(later[14] - back[14]);
    eprintln!("board marker RESULT restore: {:.3} m from saved, up_y {:.4}, drift over 30 frames {drift:.4} m (was {:.3} m away)",
        distance(back), back[5], distance(moved));
    assert!(distance(back) < 0.05 && back[5] > 0.99 && drift < 0.01, "restore missed: {back:?}");

    // Removed from the marker, it stays where it is.
    session.test_queue_prop_action(object, 3);
    ticks(&mut session, 0, 2);
    assert!(session.test_dropped_boards()[0].2.is_none(), "board still in the marker");
    session.test_place_dropped_board(object, away(saved));
    ticks(&mut session, 0, 60);
    session.test_restore_marker_props();
    ticks(&mut session, 0, 60);
    let left = session.test_dropped_boards()[0].1.unwrap();
    eprintln!("board marker RESULT removed: {:.3} m from saved", distance(left));
    assert!(distance(left) > 2.0, "a removed board was restored");

    // Upright: tipped on its side beside the skater, the menu rights it.
    let mut tipped = saved;
    tipped[0..3].copy_from_slice(&[0.0, 1.0, 0.0]);
    tipped[4..7].copy_from_slice(&[-fz, 0.0, fx]);
    tipped[13] += 0.2;
    session.test_place_dropped_board(object, tipped);
    ticks(&mut session, 0, 90);
    let lying = session.test_dropped_boards()[0].1.unwrap();
    ticks(&mut session, 0x0200, 2);
    let menu = session.test_prop_menu().expect("no menu by the tipped board");
    assert!(!menu.params[0], "upright disabled for a tipped board (up_y {})", lying[5]);
    session.test_queue_prop_action(object, 0);
    ticks(&mut session, 0, 90);
    let righted = session.test_dropped_boards()[0].1.unwrap();
    eprintln!("board marker RESULT upright: up_y {:.4} -> {:.4}", lying[5], righted[5]);
    assert!(righted[5] > 0.99, "upright left it at up_y {}", righted[5]);
}

/// Online the living world spawns nothing (a network game's reset clears census bit 7): the park
/// editor's props still come out, and one the reset culled comes back.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_props_show_in_a_network_game() {
    use crate::input_pad::XboxPadSample;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut run = |session: &mut OriginalFacade, sample: XboxPadSample, count: usize| {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        for _ in 0..count {
            pad.scripted_controller_zero = Some(sample);
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    };
    let press = |buttons: u16| XboxPadSample { buttons, ..idle };
    run(&mut session, idle, 30);
    run(&mut session, press(0x8000), 1);
    run(&mut session, idle, 150);
    session.test_network_game_reset();
    run(&mut session, idle, 5);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x2000), 1);
    run(&mut session, idle, 2);
    assert!(session.test_editor().open, "LB + B did not open the editor");
    run(&mut session, XboxPadSample { left_y: i16::MAX, ..idle }, 40);
    for buttons in [0x4000, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x1000, 0x0002, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    run(&mut session, press(0x0001), 20);
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert_eq!(editor.placed.len(), 1, "not placed: {editor:?}");
    let placed = editor.placed[0];
    assert_eq!(session.test_dropped_objects_out(), vec![true], "the placed prop has no object in a network game");
    run(&mut session, idle, 120);
    let settled = session.test_editor().placed[0];
    eprintln!("park editor online RESULT placed at y {:.2}, settled at y {:.2}", placed[13], settled[13]);
    assert!(placed[13] - settled[13] > 0.15, "the placed prop did not fall in a network game");
    // Another network game reset culls it; the online tick's respawn brings it back.
    session.test_network_game_reset();
    run(&mut session, idle, 2);
    let culled = session.test_dropped_objects_out();
    session.test_respawn_online_props();
    let back = session.test_dropped_objects_out();
    eprintln!("park editor online RESULT after reset {culled:?}, after respawn {back:?}");
    assert_eq!(back, vec![true], "the culled prop did not come back");
}

/// PC park editor (Object Dropper), driven by the pad as a player would: LB + B opens it over a
/// running world, X lists Props by subcategory, A takes Ramps' Kicker (medium) in front of the
/// crosshair; D-pad Up raises it, RT + a stick flick turns it 90 degrees, A places it and it
/// falls while the skater stands still; A picks it back up from under the crosshair, A places it
/// again, Y deletes it, B leaves.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_takes_places_picks_up_and_deletes() {
    use crate::input_pad::XboxPadSample;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut run = |session: &mut OriginalFacade, sample: XboxPadSample, count: usize| {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        for _ in 0..count {
            pad.scripted_controller_zero = Some(sample);
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    };
    let press = |buttons: u16| XboxPadSample { buttons, ..idle };
    // Off the board (Y), standing.
    run(&mut session, idle, 30);
    run(&mut session, press(0x8000), 1);
    run(&mut session, idle, 150);
    let (skater, _) = session.test_skater_root();
    // LB, then LB + B.
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x2000), 1);
    run(&mut session, idle, 2);
    let editor = session.test_editor();
    assert!(editor.open, "LB + B did not open the editor");
    // Fly ahead of the skater so the prop lands clear of it.
    run(&mut session, XboxPadSample { left_y: i16::MAX, ..idle }, 40);
    // X, the subcategories: Ramps is the sixth; A opens it, its second item is Kicker (medium).
    for buttons in [0x4000, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x1000, 0x0002, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    let editor = session.test_editor();
    let (name, taken) = editor.held.unwrap_or_else(|| panic!("nothing taken: {editor:?}"));
    assert_eq!(name, "Kicker (medium)");
    assert!(!editor.list_open);
    // Raise it a little, then RT + a flick right turns it 90 degrees.
    run(&mut session, press(0x0001), 20);
    let raised = session.test_editor().held.unwrap().1;
    let (local, middle_before) = session.test_editor().held_middle.unwrap();
    run(&mut session, XboxPadSample { right_trigger: 255, ..idle }, 3);
    run(&mut session, XboxPadSample { right_trigger: 255, left_x: i16::MAX, ..idle }, 3);
    run(&mut session, XboxPadSample { right_trigger: 255, ..idle }, 3);
    let spinning = session.test_editor().held.unwrap().1;
    run(&mut session, idle, 20);
    let turned = session.test_editor().held.unwrap().1;
    let middle_after = session.test_editor().held_middle.unwrap().1;
    let drift = (0..3).map(|lane| (middle_after[lane] - middle_before[lane]).powi(2)).sum::<f32>().sqrt();
    let origin_moved = (turned[12] - raised[12]).hypot(turned[14] - raised[14]);
    eprintln!("park editor RESULT middle (model space) {local:?}: moved {drift:.4} m by the quarter turn, origin moved {origin_moved:.3} m");
    assert!(drift < 0.005, "the quarter turn moved the prop's middle {drift} m");
    let angle = (raised[8] * turned[8] + raised[10] * turned[10]).clamp(-1.0, 1.0).acos().to_degrees();
    eprintln!("park editor RESULT taken at {:?}, raised {:.2} m, turned {angle:.1} deg",
        &taken[12..15], raised[13] - taken[13]);
    assert!(raised[13] - taken[13] > 0.2, "D-pad Up did not raise it");
    assert!((angle - 90.).abs() < 1., "RT + flick turned it {angle} degrees");
    let partway = (raised[8] * spinning[8] + raised[10] * spinning[10]).clamp(-1.0, 1.0).acos().to_degrees();
    assert!(partway > 5. && partway < 85., "the quarter turn did not spin through: {partway} degrees after 3 frames");
    // A places it: it falls.
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert!(editor.held.is_none() && editor.placed.len() == 1, "not placed: {editor:?}");
    let placed = editor.placed[0];
    run(&mut session, idle, 120);
    let editor = session.test_editor();
    let settled = editor.placed[0];
    let (now, _) = session.test_skater_root();
    let stood = (now[0] - skater[0]).hypot(now[2] - skater[2]);
    eprintln!("park editor RESULT placed at y {:.2}, settled at y {:.2} (fell {:.2} m), skater moved {stood:.2} m, target {:?}",
        placed[13], settled[13], placed[13] - settled[13], editor.target);
    assert!(placed[13] - settled[13] > 0.15, "the placed prop did not fall");
    assert!(stood < 0.3, "the skater moved {stood} m with the editor open");
    // D-pad Down lowers the camera onto where it fell; A picks it back up from under the crosshair.
    run(&mut session, press(0x0002), 15);
    let editor = session.test_editor();
    assert_eq!(editor.target, Some(0), "the crosshair is not on the placed prop");
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert!(editor.held.is_some_and(|held| held.0 == "Kicker (medium)") && editor.placed.is_empty(), "not picked up: {editor:?}");
    // B while holding: back in the object list on the Kicker, the prop put back where it lay.
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert!(editor.held.is_none() && editor.list_open && editor.list_items, "B did not go back to the list: {editor:?}");
    assert_eq!(editor.placed.len(), 1, "B did not put the picked-up prop back");
    let back = (editor.placed[0][12] - settled[12]).hypot(editor.placed[0][14] - settled[14]);
    assert!(back < 0.05, "the prop went back {back} m from where it lay");
    // B, B out of the list; Y deletes the placed prop under the crosshair.
    for _ in 0..2 {
        run(&mut session, press(0x2000), 1);
        run(&mut session, idle, 1);
    }
    let editor = session.test_editor();
    assert!(!editor.list_open && editor.target == Some(0), "not back on the placed prop: {editor:?}");
    run(&mut session, press(0x8000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert!(editor.placed.is_empty(), "Y did not delete it: {editor:?}");
    // X, A (Ramps), A (its first item, Kicker (large)): in hand again; Y deletes the held prop.
    for buttons in [0x4000, 0x1000, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    assert!(session.test_editor().held.is_some_and(|held| held.0.starts_with("Kicker")), "not taken again: {:?}", session.test_editor());
    run(&mut session, press(0x8000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert!(editor.held.is_none() && editor.placed.is_empty() && !editor.list_open, "Y did not delete the held prop: {editor:?}");
    // Start: the settings page; D-pad Right raises the camera speed, X sets it back, B closes.
    run(&mut session, press(0x0010), 1);
    run(&mut session, idle, 1);
    assert!(session.test_editor().settings_open, "Start did not open the settings");
    let before = session.test_editor().settings[0];
    run(&mut session, press(0x0008), 1);
    run(&mut session, idle, 1);
    let raised = session.test_editor().settings[0];
    run(&mut session, press(0x4000), 1);
    run(&mut session, idle, 1);
    let reset = session.test_editor().settings[0];
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 1);
    eprintln!("park editor RESULT settings camera speed {before}% -> {raised}% -> X {reset}%");
    assert!(raised == before + 10 && reset == 100 && !session.test_editor().settings_open, "settings page");
    // B leaves once the pad is let go.
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 2);
    assert!(!session.test_editor().open, "B did not leave the editor");
}

/// PC park editor: a skater riding when it opens puts its foot down (the editor holds B for it,
/// as a player holding B does) and stops, where a skater left coasting keeps rolling.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_brakes_a_riding_skater() {
    use crate::input_pad::XboxPadSample;
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    // 0 coast, 1 editor open, 2 the pad holds B.
    let ride = |how: u8| -> (f32, f32) {
        let (mut session, mut pad, frame) = boot_original_first_visual();
        let mut run = |session: &mut OriginalFacade, sample: XboxPadSample, count: usize| {
            let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
            for _ in 0..count {
                pad.scripted_controller_zero = Some(sample);
                session.try_tick(&mut pad, frame, &mut clamp).unwrap();
            }
        };
        run(&mut session, idle, 30);
        // Three pushes (A with D-pad Up, as the push demo).
        for _ in 0..3 {
            run(&mut session, XboxPadSample { buttons: 0x1000 | 0x0001, ..idle }, 12);
            run(&mut session, idle, 20);
        }
        let speed = |session: &mut OriginalFacade, run: &mut dyn FnMut(&mut OriginalFacade, XboxPadSample, usize)| {
            let (a, _) = session.test_skater_root();
            run(session, idle, 10);
            let (b, _) = session.test_skater_root();
            (b[0] - a[0]).hypot(b[2] - a[2]) * 6.0
        };
        let before = speed(&mut session, &mut run);
        if how == 1 {
            run(&mut session, XboxPadSample { buttons: 0x0100, ..idle }, 1);
            run(&mut session, XboxPadSample { buttons: 0x0100 | 0x2000, ..idle }, 1);
            assert!(session.test_editor().open, "LB + B did not open the editor");
        }
        run(&mut session, XboxPadSample { buttons: if how == 2 { 0x2000 } else { 0 }, ..idle }, 150);
        let after = speed(&mut session, &mut run);
        (before, after)
    };
    let (coast_before, coast_after) = ride(0);
    let (brake_before, brake_after) = ride(1);
    let (held_before, held_after) = ride(2);
    eprintln!("park editor RESULT coasting {coast_before:.2} -> {coast_after:.2} m/s; editor open {brake_before:.2} -> {brake_after:.2} m/s; B held {held_before:.2} -> {held_after:.2} m/s");
    assert!(brake_before > 1.0, "the pushes did not get the skater rolling: {brake_before} m/s");
    assert!(brake_after < 0.3, "the skater kept rolling under the editor: {brake_after} m/s");
    assert!(coast_after > brake_after + 0.5, "coasting slowed as much as braking: {coast_after} vs {brake_after} m/s");
}

/// PC Object Dropper in the replay editor: a dropped copy of the player's board is recorded
/// with every frame (its deck and the player's), the replay editor draws it at the recorded
/// place, and a saved replay keeps it (version 2) and loads it back unchanged.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn replay_editor_draws_dropped_boards() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = crate::input_pad::XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut run = |session: &mut OriginalFacade, count: usize| {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        for _ in 0..count {
            pad.scripted_controller_zero = Some(idle);
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    };
    run(&mut session, 30);
    let (root, forward) = session.test_skater_root();
    let length = forward[0].hypot(forward[2]);
    let (fx, fz) = (forward[0] / length, forward[2] / length);
    let at = [root[0] + fz * 1.5, root[1] + 0.3, root[2] - fx * 1.5];
    let drop = [fz, 0.0, -fx, 0.0, 0.0, 1.0, 0.0, 0.0, fx, 0.0, fz, 0.0, at[0], at[1], at[2], 1.0];
    assert!(session.test_drop_board(drop), "no room for the board");
    run(&mut session, 120);
    let live = session.test_dropped_boards()[0].1.expect("board deck");
    session.test_open_replay_editor();
    let (local, boards) = session.test_replay_editor_boards_at_end();
    eprintln!("replay boards RESULT editor draws {} board(s), local deck {}, live deck {:?}, replayed {:?}",
        boards.len(), local.is_some(), &live[12..15], boards.first().map(|deck| deck[12..15].to_vec()));
    assert_eq!(boards.len(), 1, "the replay editor draws no dropped board");
    assert!(local.is_some(), "no local board deck recorded");
    let off = (boards[0][12] - live[12]).hypot(boards[0][14] - live[14]);
    assert!(off < 0.05, "the replayed board is {off} m from where it was");
    let (magic, with, total, same) = session.test_replay_round_trip_boards();
    eprintln!("replay boards RESULT saved as {:?}: {with} of {total} frames hold the board, loaded back equal {same}",
        String::from_utf8_lossy(&magic));
    assert_eq!(magic, b"SK2PCR02");
    assert!(with > 0 && same, "the saved replay lost the board");
}
