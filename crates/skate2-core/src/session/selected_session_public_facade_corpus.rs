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

/// A mission prop through the editor: X, the Mission subcategory (the eighth), A takes its
/// Speaker, A places it where it is held: it stays exactly there (static), Skate 3 physics takes
/// its cut collision as a body and the world runs on; A picks it back up, Y deletes it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_places_a_mission_speaker() {
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
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    assert!(session.test_editor().open, "LB + D-pad Left did not open the editor");
    run(&mut session, XboxPadSample { left_y: i16::MAX, ..idle }, 40);
    let mut buttons = vec![0x4000];
    // Mission stage is the Props tab's seventeenth subcategory (after Skate 2's seven, Skate 3's
    // seven, Characters and Marquee models).
    buttons.extend([0x0002; 16]);
    buttons.extend([0x1000, 0x1000]);
    for buttons in buttons {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    let editor = session.test_editor();
    let (name, _) = editor.held.unwrap_or_else(|| panic!("nothing taken: {editor:?}"));
    assert_eq!(name, "Speaker");
    run(&mut session, press(0x0001), 20);
    let held = session.test_editor().held.unwrap().1;
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert_eq!(editor.placed.len(), 1, "not placed: {editor:?}");
    let placements = crate::ui::object_dropper::mission_placements();
    assert_eq!(placements.len(), 1, "Skate 3 physics has no mission prop");
    eprintln!("park editor mission RESULT placed at {:?}, {} collision triangles", &editor.placed[0][12..15], placements[0].shape.triangles.len());
    run(&mut session, idle, 120);
    let settled = session.test_editor().placed[0];
    assert_eq!(settled, held, "the static mission prop moved");
    let editor = session.test_editor();
    assert_eq!(editor.target, Some(0), "the crosshair is not on the speaker");
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    assert!(crate::ui::object_dropper::mission_placements().is_empty(), "picked up, the speaker still collides");
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    assert_eq!(crate::ui::object_dropper::mission_placements().len(), 1, "placed again, it does not collide");
    run(&mut session, idle, 5);
    assert_eq!(session.test_editor().target, Some(0), "the crosshair lost the speaker");
    run(&mut session, press(0x8000), 1);
    run(&mut session, idle, 1);
    assert!(session.test_editor().placed.is_empty() && crate::ui::object_dropper::mission_placements().is_empty(), "Y did not delete it");
}

/// Mission props are Skate 2 ground (run without a Skate 3 gameplay.json): the world line test
/// meets a placed speaker's top. (A prop dropped here rests asleep where it is put, so a falling
/// box cannot show the contacts; the board and skeleton read the same triangles.)
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mission_speaker_is_skate2_ground() {
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
    run(&mut session, idle, 30);
    let (skater, _) = session.test_skater_root();
    let (x, z) = (skater[0] + 4.0, skater[2]);
    let ground = session.test_line_down(x, z, skater[1] + 5.0, skater[1] - 5.0).expect("ground under the test spot");
    let at = crate::ui::object_dropper::prop_transform([x, ground, z], 0.0);
    assert_eq!(session.test_drop_prop("Speaker", at), "placed");
    run(&mut session, idle, 2);
    let top = session.test_line_down(x, z, ground + 5.0, ground - 1.0).expect("no hit over the speaker");
    eprintln!("mission skate2 RESULT ground {ground:.3}, line hit over the speaker {top:.3} (+{:.3} m)", top - ground);
    assert!((top - ground - 1.55).abs() < 0.1, "the line test passed through the speaker: hit {:.3} m above the ground", top - ground);
    // Contacts: with the editor open (it reads placed props' live transforms), a box dropped
    // onto the speaker lands on its top, and one beside it on the ground.
    let press = |buttons: u16| XboxPadSample { buttons, ..idle };
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    assert_eq!(session.test_drop_prop("Cardboard box", crate::ui::object_dropper::prop_transform([x, ground + 3.0, z], 0.0)), "placed");
    assert_eq!(session.test_drop_prop("Cardboard box", crate::ui::object_dropper::prop_transform([x + 3.0, ground + 3.0, z], 0.0)), "placed");
    run(&mut session, idle, 150);
    let placed = session.test_editor().placed;
    let (on, beside) = (placed[1][13] - ground, placed[2][13] - ground);
    eprintln!("mission skate2 RESULT box over the speaker rests at +{on:.3} m, box beside it at +{beside:.3} m");
    assert!(beside < 0.5, "the control box did not fall: +{beside:.3} m");
    assert!(on > 1.3, "the box fell through the speaker to +{on:.3} m");
}

/// Prop settings and copies (Skate 3 physics): a kicker set static and placed in the air stays
/// there while a dynamic one falls; LB + A places the held prop and keeps a copy in hand.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_static_props_and_copies() {
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
    let (skater, _) = session.test_skater_root();
    let (x, z) = (skater[0] + 4.0, skater[2]);
    let ground = session.test_line_down(x, z, skater[1] + 5.0, skater[1] - 5.0).expect("ground");
    // The editor open: it reads where placed props are now.
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    session.test_set_prop_settings("Kicker (medium)", true, false);
    assert_eq!(session.test_drop_prop("Kicker (medium)", crate::ui::object_dropper::prop_transform([x, ground + 1.0, z], 0.0)), "placed (static)");
    session.test_set_prop_settings("Kicker (medium)", false, false);
    assert_eq!(session.test_drop_prop("Kicker (medium)", crate::ui::object_dropper::prop_transform([x + 4.0, ground + 1.0, z], 0.0)), "placed");
    run(&mut session, idle, 120);
    let placed = session.test_editor().placed;
    let (fixed, free) = (placed[0][13] - ground, placed[1][13] - ground);
    eprintln!("park editor settings RESULT static kicker at +{fixed:.3} m, dynamic kicker at +{free:.3} m after 2 s");
    assert!((fixed - 1.0).abs() < 0.01, "the static kicker moved to +{fixed:.3} m");
    assert!(free < 0.5, "the dynamic kicker did not fall: +{free:.3} m");
    // Take a kicker; LB + A places it and keeps a copy.
    for buttons in [0x4000, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x1000, 0x0002, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    assert!(session.test_editor().held.is_some(), "nothing taken");
    run(&mut session, press(0x0100), 1);
    run(&mut session, press(0x0100 | 0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert_eq!(editor.placed.len(), 3, "LB + A did not place it");
    assert!(editor.held.is_some_and(|held| held.0 == "Kicker (medium)"), "LB + A kept no copy: {editor:?}");
}
/// Skate 3 props are dynamic like the bank's: a Skate 3 plastic barrier and bench dropped 1 m
/// up fall onto the street, one set Static stays where it was put, and the dropped ones are
/// drawn (as living-world DMOs under their Skate 3 IDs) with no static collision body.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_props_are_dynamic() {
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
    let (skater, _) = session.test_skater_root();
    let (x, z) = (skater[0] + 4.0, skater[2]);
    let ground = session.test_line_down(x, z, skater[1] + 5.0, skater[1] - 5.0).expect("ground");
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let at = |dx: f32| crate::ui::object_dropper::prop_transform([x + dx, ground + 1.0, z], 0.0);
    assert_eq!(session.test_drop_prop("Barrier plastic", at(0.0)), "placed");
    assert_eq!(session.test_drop_prop("Bench flat", at(10.0)), "placed");
    session.test_set_prop_settings("Barrier plastic", true, false);
    assert_eq!(session.test_drop_prop("Barrier plastic", at(20.0)), "placed (static)");
    run(&mut session, idle, 120);
    let placed = session.test_editor().placed;
    let heights: Vec<f32> = placed.iter().map(|t| t[13] - ground).collect();
    let ids = [0x2c701707000601de_u64, 0x2c701707000f00df];
    let drawn = session.try_object_dropper_visual_snapshot().unwrap().dynamic_objects.iter().filter(|object| ids.contains(&object.asset_id)).count();
    let static_bodies = crate::ui::object_dropper::mission_placements().len();
    eprintln!("park editor skate3 props RESULT heights over the street after 2 s {heights:?}; drawn {drawn}; static collision bodies {static_bodies}");
    assert!(heights[0] < 0.5 && heights[1] < 0.5, "a dynamic Skate 3 prop did not fall: {heights:?}");
    assert!((heights[2] - 1.0).abs() < 0.01, "the static Skate 3 prop moved: {heights:?}");
    assert_eq!(drawn, 3, "the Skate 3 props are not all drawn");
    assert_eq!(static_bodies, 0, "a Skate 3 prop was placed as a static park piece");
}

/// The four Skate 3 props whose CModel is a mesh table (the two bins, the balloon and the
/// guard rail) are dynamic too: dropped 1 m up they fall onto the street and are drawn, with
/// no static collision body.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_mesh_table_props_are_dynamic() {
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
    let (skater, _) = session.test_skater_root();
    let props = [
        ("Garbage bin blue", 0x2c701707000f00e1_u64),
        ("Garbage can reclaimed", 0x2c701707000f00fd),
        ("Miracle whip balloon", 0x2c70170700030578),
        ("Guard rail grind", 0x2c701707000600d9),
    ];
    for (_, id) in props {
        assert!(crate::ui::object_dropper::is_dynamic_park_prop(id), "{id:#x} is not a living-world DMO model");
    }
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let mut grounds = Vec::new();
    for (index, (name, _)) in props.iter().enumerate() {
        let (x, z) = (skater[0] + 4.0 + 12.0 * index as f32, skater[2]);
        let ground = session.test_line_down(x, z, skater[1] + 5.0, skater[1] - 5.0).expect("ground");
        assert_eq!(session.test_drop_prop(name, crate::ui::object_dropper::prop_transform([x, ground + 1.0, z], 0.0)), "placed", "{name}");
        grounds.push(ground);
    }
    run(&mut session, idle, 180);
    let placed = session.test_editor().placed;
    let heights: Vec<f32> = placed.iter().zip(&grounds).map(|(t, ground)| t[13] - ground).collect();
    let ids: Vec<u64> = props.iter().map(|(_, id)| *id).collect();
    let drawn = session.try_object_dropper_visual_snapshot().unwrap().dynamic_objects.iter().filter(|object| ids.contains(&object.asset_id)).count();
    let static_bodies = crate::ui::object_dropper::mission_placements().len();
    eprintln!("park editor mesh-table props RESULT heights over the street after 3 s {heights:?}; drawn {drawn}; static collision bodies {static_bodies}");
    assert!(heights.iter().all(|height| (-0.1..0.5).contains(height)), "a mesh-table prop did not land on the street: {heights:?}");
    assert_eq!(drawn, props.len(), "the mesh-table props are not all drawn");
    assert_eq!(static_bodies, 0, "a mesh-table prop was placed as a static park piece");
}

/// The held prop's orbit camera stays above the ground: zoomed right out and looking up from
/// under the prop (which would put it below the street), it comes in to the street's surface.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_orbit_camera_stays_above_ground() {
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
    let (skater, _) = session.test_skater_root();
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    run(&mut session, XboxPadSample { left_y: i16::MAX, ..idle }, 40);
    for buttons in [0x4000, 0x0002, 0x0002, 0x0002, 0x0002, 0x0002, 0x1000, 0x0002, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    assert!(session.test_editor().held.is_some(), "nothing taken");
    // D-pad Left: zoom right out.
    run(&mut session, press(0x0004), 120);
    // The street the skater stands on, and the camera's clearance over whatever is under it (none
    // found: it is over open space, a building's far side or the like).
    let street = session.test_line_down(skater[0], skater[2], skater[1] + 1.5, skater[1] - 5.0).expect("no street under the skater");
    let (mut lowest, mut clearance) = (f32::MAX, f32::MAX);
    for right_y in [i16::MAX, i16::MIN] {
        for _ in 0..12 {
            run(&mut session, XboxPadSample { right_y, ..idle }, 10);
            let camera = session.test_editor().camera.unwrap();
            lowest = lowest.min(camera[1] - street);
            if let Some(under) = session.test_line_down(camera[0], camera[2], camera[1] + 0.05, camera[1] - 20.0) {
                clearance = clearance.min(camera[1] - under);
            }
        }
    }
    eprintln!("park editor camera RESULT lowest {lowest:.3} m above the street, at least {clearance:.3} m over what is under it");
    assert!(lowest > 0.25, "the orbit camera went {lowest} m under the street");
    assert!(clearance > 0.25, "the orbit camera came within {clearance} m of what is under it");
}

/// B on a held prop and another taken from the list: the new one goes exactly where the last
/// one was (moved off its spawn point first: its middle over that one's, on the same base height
/// and turn), the camera looking the same way; a mega piece taken
/// so has the camera backed off out of it. A held character's camera orbits its hips as drawn.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_swaps_in_place_and_follows_hips() {
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
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let origin = |t: [f32; 16]| [t[12], t[13], t[14]];
    session.test_take("Barrier plastic");
    run(&mut session, idle, 2);
    // Moved off where it spawned, and turned a little.
    run(&mut session, XboxPadSample { left_x: i16::MAX / 2, left_y: i16::MAX / 3, ..idle }, 15);
    run(&mut session, XboxPadSample { left_trigger: 255, right_trigger: 255, left_x: i16::MAX / 3, ..idle }, 10);
    run(&mut session, idle, 2);
    let before = session.test_editor();
    let (_, was) = before.held.expect("nothing held");
    let was_middle = before.held_middle.unwrap().1;
    for name in ["Bench flat", "Mega quater pipe 90 corner"] {
        run(&mut session, press(0x2000), 1);
        run(&mut session, idle, 2);
        assert!(session.test_editor().list_open, "B did not go back to the list");
        session.test_take(name);
        run(&mut session, idle, 2);
        let after = session.test_editor();
        let (_, now) = after.held.expect("nothing taken");
        // Its middle over the last one's, on the same base height.
        let now_middle = after.held_middle.unwrap().1;
        let moved = [now_middle[0] - was_middle[0], origin(now)[1] - origin(was)[1], now_middle[2] - was_middle[2]].iter().fold(0.0f32, |m, d| m.max(d.abs()));
        let turned = (0..12).map(|lane| (now[lane] - was[lane]).abs()).fold(0.0f32, f32::max);
        let [a, b] = [before.look.unwrap(), after.look.unwrap()];
        let look = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos().to_degrees();
        let backed = {
            let [c, d] = [before.camera.unwrap(), after.camera.unwrap()];
            ((c[0] - d[0]).powi(2) + (c[1] - d[1]).powi(2) + (c[2] - d[2]).powi(2)).sqrt()
        };
        eprintln!("park editor swap RESULT {name}: placed {moved:.4} m from the last one, turn differs {turned:.4}, camera looks {look:.2} deg away and is {backed:.2} m from where it was");
        assert!(moved < 1e-3 && turned < 1e-4, "{name} did not take the last one's spot");
        assert!(look < 0.01, "the camera swung round for {name}");
        if name == "Mega quater pipe 90 corner" {
            let piece = crate::ui::object_dropper::PARK_PIECES[crate::ui::object_dropper::park_piece(0x2c701707000303e6).unwrap()];
            let camera = after.camera.unwrap();
            let d = [camera[0] - now[12], camera[1] - now[13], camera[2] - now[14]];
            let local: [f32; 3] = std::array::from_fn(|c| d[0] * now[c * 4] + d[1] * now[c * 4 + 1] + d[2] * now[c * 4 + 2]);
            let inside = (0..3).all(|axis| local[axis] > piece.min[axis] && local[axis] < piece.max[axis]);
            eprintln!("park editor swap RESULT mega piece: camera at {local:?} in its model space, inside {inside}");
            assert!(!inside, "the camera is inside the mega piece");
        } else {
            assert!(backed < 1.0, "the camera backed off for a prop no bigger");
        }
    }
    // Closing the list forgets the spot: the next one goes back to the crosshair.
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 1);
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 1);
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 2);
    assert!(!session.test_editor().list_open, "the list did not close");
    // A held character: the camera looks at its hips as drawn.
    session.test_take("Chris Cole");
    let mut waited = 0;
    while session.test_character_hips().is_empty() && waited < 600 {
        run(&mut session, idle, 10);
        waited += 10;
    }
    let mut worst = 0.0f32;
    let mut heights = Vec::new();
    for _ in 0..6 {
        run(&mut session, idle, 10);
        let editor = session.test_editor();
        let hips = session.test_character_hips()[0];
        let (camera, look) = (editor.camera.unwrap(), editor.look.unwrap());
        let to = [hips[0] - camera[0], hips[1] - camera[1], hips[2] - camera[2]];
        let length = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
        let off = ((to[0] * look[0] + to[1] * look[1] + to[2] * look[2]) / length).clamp(-1.0, 1.0).acos().to_degrees();
        worst = worst.max(off);
        heights.push(hips[1] - editor.held.unwrap().1[13]);
    }
    eprintln!("park editor follow RESULT camera at most {worst:.2} deg off the hips (hips {heights:?} m over the placement)");
    assert!(worst < 3.0, "the camera does not look at the character's hips");
    // Start, D-pad Down to the clip row: the prop settings page steps through the viewer's clips
    // (D-pad Right) until one puts the hips elsewhere; the camera follows them with the page still open.
    for buttons in [0x0010, 0x0002] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    let start = session.test_character_hips()[0];
    let (mut worst, mut travel, mut steps) = (0.0f32, 0.0f32, 0);
    while travel < 0.2 && steps < 20 {
        run(&mut session, press(0x0008), 1);
        run(&mut session, idle, 1);
        steps += 1;
        for _ in 0..12 {
            run(&mut session, idle, 10);
            let editor = session.test_editor();
            let Some(&hips) = session.test_character_hips().first() else { continue };
            let (camera, look) = (editor.camera.unwrap(), editor.look.unwrap());
            let to = [hips[0] - camera[0], hips[1] - camera[1], hips[2] - camera[2]];
            let along = to[0] * look[0] + to[1] * look[1] + to[2] * look[2];
            let miss = [to[0] - look[0] * along, to[1] - look[1] * along, to[2] - look[2] * along];
            worst = worst.max((miss[0] * miss[0] + miss[1] * miss[1] + miss[2] * miss[2]).sqrt());
            travel = travel.max(((hips[0] - start[0]).powi(2) + (hips[1] - start[1]).powi(2) + (hips[2] - start[2]).powi(2)).sqrt());
        }
    }
    let open = session.test_editor().prop_settings_open;
    eprintln!("park editor follow RESULT settings open {open}, {steps} clips on: camera line at most {worst:.3} m off the hips, which moved up to {travel:.3} m");
    assert!(open && travel >= 0.2, "no clip moved the hips with the prop settings open");
    assert!(worst < 0.05, "the camera did not follow the hips with the settings open");
    // Back to the default clip; the page closes.
    for _ in 0..steps {
        run(&mut session, press(0x0004), 1);
        run(&mut session, idle, 1);
    }
    run(&mut session, press(0x2000), 1);
    run(&mut session, idle, 1);
}

/// Taken from the list, the biggest mega ramp (the 24 x 9 x 24 m quarter pipe corner) starts
/// with the camera outside its footprint, looking at it; and a held piece fine-tuned with LT + RT stops snapping (snapping would
/// square its turn and move it) until the left stick moves it again.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_big_pieces_and_fine_tuning() {
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
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    // The mega piece.
    let piece = crate::ui::object_dropper::PARK_PIECES[crate::ui::object_dropper::park_piece(0x2c701707000303e6).unwrap()];
    session.test_take("Mega quater pipe 90 corner");
    run(&mut session, idle, 2);
    let editor = session.test_editor();
    let (name, held) = editor.held.expect("nothing taken");
    let camera = editor.camera.unwrap();
    // The camera in the piece's model space.
    let d = [camera[0] - held[12], camera[1] - held[13], camera[2] - held[14]];
    let local: [f32; 3] = std::array::from_fn(|c| d[0] * held[c * 4] + d[1] * held[c * 4 + 1] + d[2] * held[c * 4 + 2]);
    // Over (or under) its footprint: inside it, or looking straight down into it.
    let over = [0, 2].iter().all(|&axis| local[axis] > piece.min[axis] && local[axis] < piece.max[axis]);
    let middle: [f32; 3] = std::array::from_fn(|axis| (piece.min[axis] + piece.max[axis]) * 0.5);
    let toward: [f32; 3] = std::array::from_fn(|c| held[12 + c] + held[c] * middle[0] + held[4 + c] * middle[1] + held[8 + c] * middle[2] - camera[c]);
    let size: Vec<f32> = (0..3).map(|axis| piece.max[axis] - piece.min[axis]).collect();
    eprintln!("park editor big piece RESULT {name} ({size:?} m): camera at {local:?} in its model space, over its footprint {over}, {:.1} m from its middle", (toward[0] * toward[0] + toward[1] * toward[1] + toward[2] * toward[2]).sqrt());
    assert!(!over, "the camera starts over the mega piece's footprint");
    session.test_let_go();
    // Fine-tuning: a street pad placed, a second held beside it snaps; LT + RT with the left stick
    // turns it finely and it stops snapping; the left stick alone moves it and it snaps again.
    let (skater, _) = session.test_skater_root();
    let at = |p: [f32; 3]| crate::ui::object_dropper::prop_transform(p, 0.0);
    let base = [skater[0] + 4.0, skater[1] - 0.5, skater[2]];
    assert_eq!(session.test_drop_prop("Pad street (large, Downtown)", at(base)), "placed");
    assert!(session.test_hold_snapped("Pad street (large, Downtown)", at([base[0] + 6.4, base[1], base[2]])).is_some(), "no snap to fine-tune");
    run(&mut session, XboxPadSample { left_trigger: 255, right_trigger: 255, left_x: i16::MAX / 3, ..idle }, 20);
    let fine = session.test_editor();
    run(&mut session, idle, 3);
    let released = session.test_editor();
    run(&mut session, XboxPadSample { left_x: -i16::MAX / 3, ..idle }, 2);
    run(&mut session, idle, 2);
    let moved = session.test_editor();
    let turned = fine.held.map(|(_, t)| t[0].clamp(-1.0, 1.0).acos().to_degrees());
    eprintln!("park editor fine-tune RESULT turned {turned:?} deg; snapped while fine-tuning {}, after letting go {}, after moving {}", fine.snapped.is_some(), released.snapped.is_some(), moved.snapped.is_some());
    assert!(turned.is_some_and(|t| t > 0.2), "LT + RT did not turn it");
    assert!(fine.snapped.is_none() && released.snapped.is_none(), "a fine-tuned piece snapped");
    assert!(moved.snapped.is_some(), "moved again, it did not snap again");
}

/// Wall snapping: a street pad held a little off the nearest wall to the start, turned 5
/// degrees off it and a little over the street, squares up to the wall, comes flush against it
/// and rests on the street.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_snaps_skate3_pieces_to_walls() {
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
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let (skater, _) = session.test_skater_root();
    let street = session.test_line_down(skater[0], skater[2], skater[1] + 1.5, skater[1] - 5.0).expect("no street");
    // The nearest wall around the start, 2 m up.
    let eye = [skater[0], street + 2.0, skater[2]];
    let (wall, normal) = (0..72)
        .filter_map(|step| {
            let angle = step as f32 * 5f32.to_radians();
            let end = [eye[0] + 40.0 * angle.sin(), eye[1], eye[2] + 40.0 * angle.cos()];
            session.test_first_surface(eye, end).filter(|(_, normal)| normal[1].abs() < 0.05)
        })
        .min_by(|a, b| {
            let d = |p: [f32; 3]| (p[0] - eye[0]).hypot(p[2] - eye[2]);
            d(a.0).total_cmp(&d(b.0))
        })
        .expect("no wall within 40 m");
    let facing = |n: [f32; 3]| {
        let length = n[0].hypot(n[2]);
        [n[0] / length, 0.0, n[2] / length]
    };
    // The wall's normal faces the eye.
    let n = facing(normal);
    let n = if n[0] * (eye[0] - wall[0]) + n[2] * (eye[2] - wall[2]) < 0.0 { [-n[0], 0.0, -n[2]] } else { n };
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "aa_padStreet_lrg_Downtown").unwrap();
    // Model +x into the wall, turned 5 degrees.
    let tilt = 5f32.to_radians();
    let into = [-n[0], 0.0, -n[2]];
    let x = [into[0] * tilt.cos() - into[2] * tilt.sin(), 0.0, into[0] * tilt.sin() + into[2] * tilt.cos()];
    let z = [-x[2], 0.0, x[0]];
    let mid_z = (piece.min[2] + piece.max[2]) * 0.5;
    let ground = session.test_line_down(wall[0] + n[0] * 4.0, wall[2] + n[2] * 4.0, street + 1.5, street - 3.0).expect("no street by the wall");
    let origin = [
        wall[0] - x[0] * (piece.max[0] + 0.5) - z[0] * mid_z,
        ground - piece.min[1] + 0.3,
        wall[2] - x[2] * (piece.max[0] + 0.5) - z[2] * mid_z,
    ];
    let held = [x[0], 0.0, x[2], 0.0, 0.0, 1.0, 0.0, 0.0, z[0], 0.0, z[2], 0.0, origin[0], origin[1], origin[2], 1.0];
    let snapped = session.test_hold_snapped("Pad street (large, Downtown)", held).expect("no snap by the wall");
    let note = session.test_editor().note;
    let squared = snapped[0] * into[0] + snapped[2] * into[2];
    // The +x face's middle, and the wall out from it.
    let face = |lane: usize| snapped[12 + lane] + snapped[lane] * piece.max[0] + snapped[4 + lane] * (piece.min[1] + piece.max[1]) * 0.5 + snapped[8 + lane] * mid_z;
    let face = [face(0), face(1), face(2)];
    let out = [snapped[0], snapped[1], snapped[2]];
    let gap = session
        .test_first_surface([face[0] - out[0], face[1], face[2] - out[2]], [face[0] + out[0] * 2.0, face[1], face[2] + out[2] * 2.0])
        .map(|(hit, _)| (hit[0] - face[0]) * out[0] + (hit[2] - face[2]) * out[2]);
    let rest = snapped[13] + piece.min[1];
    eprintln!("park editor wall snap RESULT wall at {wall:?} normal {n:?}; squared {squared:.5}; gap {gap:?}; base {rest:.3} on street {ground:.3}; note {note:?}");
    assert!(squared > 0.9999, "not squared to the wall: {squared}");
    assert!(gap.is_some_and(|gap| gap.abs() < 0.02), "not flush: gap {gap:?}");
    assert!((rest - ground).abs() < 0.3, "not on the street: base {rest}, street {ground}");
}

/// The park editor gives back what it takes: placing a character (animated, posed collision),
/// a Skate 3 piece, a bank prop and a mission prop, snapping a held piece, then taking them all
/// away, over and over, leaves nothing behind and the process's private memory flat (once the
/// game's own buffers have filled); and characters placed on 12
/// different animation files leave none of those files loaded once they are gone.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_memory_stays_flat() {
    use crate::input_pad::XboxPadSample;
    fn private_bytes() -> f64 {
        let script = format!("(Get-Process -Id {}).PrivateMemorySize64", std::process::id());
        let out = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).output().expect("powershell");
        String::from_utf8_lossy(&out.stdout).trim().parse::<f64>().expect("private bytes") / (1024.0 * 1024.0)
    }
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
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let (skater, _) = session.test_skater_root();
    let at = |dx: f32, dz: f32| crate::ui::object_dropper::prop_transform([skater[0] + dx, skater[1] - 0.5, skater[2] + dz], 0.0);
    // The game's own buffers fill over its first ~5,000 ticks (private memory then holds):
    // past that, only what the editor keeps shows.
    run(&mut session, idle, 6000);
    let mut cycle = |session: &mut OriginalFacade| {
        for (name, dx, dz) in [("Chris Cole", 3.0, 0.0), ("Pad street (large, Downtown)", 6.0, 4.0), ("Kicker (medium)", -3.0, 0.0), ("Speaker", 0.0, -4.0)] {
            assert_eq!(session.test_drop_prop(name, at(dx, dz)), "placed", "{name}");
        }
        run(session, idle, 30);
        let _ = session.test_hold_snapped("Pad street (large, Downtown)", at(12.5, 4.0));
        session.test_remove_all();
        run(session, idle, 5);
    };
    for _ in 0..5 {
        cycle(&mut session);
    }
    let before = private_bytes();
    for _ in 0..30 {
        cycle(&mut session);
    }
    let after = private_bytes();
    let per_cycle = (after - before) / 30.0;
    let (characters, placements) = (session.test_placed_characters(), crate::ui::object_dropper::mission_placements().len());
    // Animation files: 12 characters, each on its own file, placed then taken away.
    let files: Vec<&str> = (1..crate::ui::object_dropper::ANIMATION_FILES.len()).step_by(40).take(12).map(crate::ui::object_dropper::animation_file_name).collect();
    for file in &files {
        session.test_set_animation("Chris Cole", file);
        assert_eq!(session.test_drop_prop("Chris Cole", at(3.0, 0.0)), "placed");
        run(&mut session, idle, 3);
        session.test_remove_all();
    }
    session.test_set_animation("Chris Cole", files[0]);
    assert_eq!(session.test_drop_prop("Chris Cole", at(3.0, 0.0)), "placed");
    let loaded = session.test_animation_files_loaded();
    eprintln!("park editor memory RESULT private {before:.1} MB -> {after:.1} MB over 30 cycles ({per_cycle:.3} MB per cycle), {characters} characters and {placements} placements left; animation files loaded after 12 tried: {loaded}");
    assert_eq!((characters, placements), (0, 0), "taken away, still held");
    assert!(per_cycle < 0.5, "the park editor keeps {per_cycle} MB per cycle");
    assert_eq!(loaded, 1, "animation files no character plays stay loaded");
}

/// What the park editor's things cost per game tick: 900 ticks with nothing placed, then 900
/// with a typical park in a ring around the skater (Skate 3 pieces, bank props, a mission prop,
/// marquee characters, both kinds of dropped board); each run's tick times. (Simulation only:
/// drawing is the window's.)
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_tick_cost() {
    use crate::input_pad::XboxPadSample;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut run = |session: &mut OriginalFacade, count: usize| -> Vec<f64> {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        let mut times = Vec::with_capacity(count);
        for _ in 0..count {
            pad.scripted_controller_zero = Some(idle);
            let started = std::time::Instant::now();
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times
    };
    let summary = |mut times: Vec<f64>| {
        times.sort_by(f64::total_cmp);
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        (mean, times[times.len() * 95 / 100], times[times.len() - 1])
    };
    let rebuilds = || crate::skate3_native::physics::player::grinds::REBUILDS.lock().map(|r| *r).unwrap_or_default();
    run(&mut session, 600);
    let (count, ms) = rebuilds();
    let before = summary(run(&mut session, 900));
    let (count_b, ms_b) = rebuilds();
    let rebuilt_before = (count_b - count, ms_b - ms);
    let (skater, _) = session.test_skater_root();
    // TICK_COST_THINGS: a comma-separated list instead (one kind at a time, to find the cost).
    let listed = std::env::var("TICK_COST_THINGS").ok();
    let things: Vec<&str> = listed.as_deref().map_or_else(
        || vec![
            "Pad street (large, Downtown)", "Ledge (large, Downtown)", "Ledge (large, Industrial)", "Kicker (medium)",
            "Bench", "Pylon", "Hay bale", "Traffic barrier", "Speaker", "Chris Cole", "Chris Cole", "board", "grind board",
        ],
        |list| list.split(',').collect(),
    );
    let mut placed = Vec::new();
    for (index, name) in things.iter().enumerate() {
        let angle = index as f32 / things.len() as f32 * std::f32::consts::TAU;
        let distance = if ["Pad", "Ledge", "Kicker"].iter().any(|big| name.starts_with(big)) { 10.0 } else { 4.0 };
        let (x, z) = (skater[0] + angle.cos() * distance, skater[2] + angle.sin() * distance);
        let y = session.test_line_down(x, z, skater[1] + 3.0, skater[1] - 5.0).unwrap_or(skater[1]);
        let result = match *name {
            "board" => if session.test_drop_board(crate::ui::object_dropper::prop_transform([x, y + 0.1, z], 0.0)) { "placed" } else { "no board" },
            "grind board" => if session.test_drop_grind_board(crate::ui::object_dropper::prop_transform([x, y + 0.1, z], 0.0)) { "placed" } else { "no board" },
            _ => session.test_drop_prop(name, crate::ui::object_dropper::prop_transform([x, y, z], 0.0)),
        };
        placed.push((*name, result));
    }
    run(&mut session, 600);
    let (count, ms) = rebuilds();
    let after = summary(run(&mut session, 900));
    let (count_a, ms_a) = rebuilds();
    let rebuilt_after = (count_a - count, ms_a - ms);
    // Sampled over a second: a board that sleeps stays frozen.
    let mut asleep = vec![0; session.test_boards_frozen().len()];
    for _ in 0..60 {
        run(&mut session, 1);
        for (count, frozen) in asleep.iter_mut().zip(session.test_boards_frozen()) {
            *count += usize::from(frozen);
        }
    }
    eprintln!("park editor tick cost RESULT boards frozen in how many of 60 ticks: {asleep:?}");
    eprintln!("park editor tick cost RESULT nothing placed: mean {:.3} ms, p95 {:.3}, max {:.3}, grind world rebuilds {rebuilt_before:?}; with {placed:?}: mean {:.3} ms, p95 {:.3}, max {:.3}, grind world rebuilds {rebuilt_after:?}", before.0, before.1, before.2, after.0, after.1, after.2);
}

/// Skate 3 pieces are grindable: a placed ledge adds its own grind splines, at the placement,
/// to Skate 3 physics' grind world and to Skate 2 physics' grind query, and taking it away takes
/// them out again.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_pieces_are_grindable() {
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
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let (skater, _) = session.test_skater_root();
    let street = session.test_line_down(skater[0] + 5.0, skater[2], skater[1] + 1.5, skater[1] - 5.0).expect("no street");
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "ab_ledge_lrg_Downtown").unwrap();
    // Turned a quarter, so model and world axes differ.
    let at = crate::ui::object_dropper::prop_transform([skater[0] + 5.0, street - piece.min[1], skater[2]], std::f32::consts::FRAC_PI_2);
    let before = session.test_skate3_grind_segments();
    assert_eq!(session.test_drop_prop("Ledge (large, Downtown)", at), "placed");
    run(&mut session, idle, 3);
    let after = session.test_skate3_grind_segments();
    // In the piece's own frame every primitive lies in its bounds.
    let local = |p: [f32; 4]| -> [f32; 3] {
        let d = [p[0] - at[12], p[1] - at[13], p[2] - at[14]];
        std::array::from_fn(|c| d[0] * at[c * 4] + d[1] * at[c * 4 + 1] + d[2] * at[c * 4 + 2])
    };
    let inside = |p: [f32; 3]| (0..3).all(|axis| p[axis] > piece.min[axis] - 0.05 && p[axis] < piece.max[axis] + 0.05);
    let world_min: [f32; 3] = std::array::from_fn(|axis| at[12 + axis] - 8.0);
    let world_max: [f32; 3] = std::array::from_fn(|axis| at[12 + axis] + 8.0);
    let skate2 = session.test_skate2_grind_query(world_min, world_max);
    let on_piece = |segments: &[([f32; 4], [f32; 4])]| segments.iter().filter(|(a, b)| inside(local(*a)) && inside(local(*b))).count();
    let skate3_added = match (&before, &after) {
        (Some(before), Some(after)) => Some(after.len() as i64 - before.len() as i64),
        _ => None,
    };
    let skate3_on = after.as_deref().map(on_piece);
    let skate2_on = on_piece(&skate2);
    let top = skate2.iter().filter(|(a, b)| {
        let (a, b) = (local(*a), local(*b));
        (a[1] - piece.max[1]).abs() < 0.1 && (b[1] - piece.max[1]).abs() < 0.1 && ((a[0] - b[0]).hypot(a[2] - b[2])) > 0.5
    }).count();
    assert_eq!(session.test_drop_prop("Ledge (large, Downtown)", at), "placed");
    session.test_remove_all();
    run(&mut session, idle, 3);
    let removed = session.test_skate3_grind_segments().map(|segments| on_piece(&segments));
    let skate2_removed = on_piece(&session.test_skate2_grind_query(world_min, world_max));
    eprintln!("park editor grind RESULT ledge top {:.2}: Skate 3 added {skate3_added:?} ({skate3_on:?} on the piece), Skate 2 query {} ({skate2_on} on the piece, {top} along its top); taken away: Skate 3 {removed:?}, Skate 2 {skate2_removed}", piece.max[1], skate2.len());
    assert!(skate2_on > 0 && top > 0, "Skate 2 physics finds no grind on the ledge");
    if let Some(on) = skate3_on {
        assert!(on > 0 && skate3_added.is_some_and(|added| added as usize == on), "Skate 3 grind world: {skate3_added:?} added, {on} on the piece");
        assert_eq!(removed, Some(0), "Skate 3 still grinds a removed ledge");
    }
    assert_eq!(skate2_removed, 0, "Skate 2 still grinds a removed ledge");
}

/// Rolls the skater 3 pushes along the street, then places Skate 3 pieces across its line with
/// `place(session, forward, right, street, now)` and rides on for 150 ticks, ollieing when
/// `ollie_at` metres from `front` (None: no ollie): each tick's physics state, how far along the
/// line past `front` the board's deck is, and its height over the street.
fn ride_at_pieces(
    place: impl FnOnce(&mut OriginalFacade, [f32; 3], [f32; 3], f32, [f32; 3]) -> [f32; 2],
    ollie_at: Option<f32>,
) -> Vec<(i32, f32, f32)> {
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
    run(&mut session, idle, 180);
    let (start, _) = session.test_skater_root();
    // Pushes (A with D-pad Up).
    for _ in 0..3 {
        run(&mut session, XboxPadSample { buttons: 0x1000 | 0x0001, ..idle }, 12);
        run(&mut session, idle, 20);
    }
    let (now, _) = session.test_skater_root();
    let forward = {
        let d = [now[0] - start[0], now[2] - start[2]];
        let length = d[0].hypot(d[1]);
        [d[0] / length, 0.0, d[1] / length]
    };
    let right = [-forward[2], 0.0, forward[0]];
    let street = session.test_line_down(now[0], now[2], now[1] + 1.5, now[1] - 5.0).expect("no street");
    let front = place(&mut session, forward, right, street, now);
    let ollie = session.controlled_right_pattern_points_for_test("Ollie");
    let mut ollied = None;
    let mut trace = Vec::new();
    for tick in 0..150 {
        let (skater, _) = session.test_skater_root();
        let to_front = (front[0] - skater[0]) * forward[0] + (front[1] - skater[2]) * forward[2];
        let sample = match ollied {
            None if ollie_at.is_some_and(|at| to_front < at) => {
                ollied = Some(tick);
                crate::input_pad::device::tests::scripted_right_point(ollie[0])
            }
            Some(at) if tick - at < 2 => crate::input_pad::device::tests::scripted_right_point(ollie[0]),
            Some(at) if (3..5).contains(&(tick - at)) => crate::input_pad::device::tests::scripted_right_point(ollie[1]),
            _ => idle,
        };
        run(&mut session, sample, 1);
        let (state, deck) = session.test_physics_state();
        trace.push((state, (deck[0] - front[0]) * forward[0] + (deck[2] - front[1]) * forward[2], deck[1] - street));
    }
    trace
}

/// The model-to-world transform (a rotation: x cross y is z) that lays a piece's model -z along
/// `forward`, its model x on `right`, with its model point (`x`, `y`, 0) at `at`.
fn piece_along(forward: [f32; 3], right: [f32; 3], at: [f32; 3], x: f32, y: f32) -> [f32; 16] {
    let axis_x = [right[0], 0.0, right[2]];
    let axis_z = [-forward[0], 0.0, -forward[2]];
    [
        axis_x[0], 0.0, axis_x[2], 0.0, 0.0, 1.0, 0.0, 0.0, axis_z[0], 0.0, axis_z[2], 0.0,
        at[0] - axis_x[0] * x, at[1] - y, at[2] - axis_x[2] * x, 1.0,
    ]
}

/// A placed Skate 3 ledge is ground: the skater rolls at a ledge set low on its line (its edge on
/// the line, its top 0.3 m up), ollies onto it, grinds (physics states 4xx and the nonspecific
/// 701) and rides away. The ollie is tried 3.2, 3.6 and 4 m out (Skate 3 physics catches the
/// first, Skate 2 physics the later ones); one of them must grind cleanly.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_ledge_is_ground() {
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "ab_ledge_lrg_Downtown").unwrap();
    let mut clean = false;
    for ollie_at in [3.2, 3.6, 4.0] {
        let trace = ride_at_pieces(
            |session, forward, right, street, now| {
                let front = [now[0] + forward[0] * 8.0, street + 0.3, now[2] + forward[2] * 8.0];
                assert_eq!(session.test_drop_prop("Ledge (large, Downtown)", piece_along(forward, right, front, 0.0, piece.max[1])), "placed");
                [front[0], front[2]]
            },
            Some(ollie_at),
        );
        let grinding: Vec<&(i32, f32, f32)> = trace.iter().filter(|(state, _, _)| (400..500).contains(state) || *state == 701).collect();
        let span = grinding.first().zip(grinding.last()).map_or(0.0, |(a, b)| b.1 - a.1);
        let states: Vec<i32> = trace.iter().map(|(state, _, _)| *state).fold(Vec::new(), |mut seen, state| {
            if seen.last() != Some(&state) {
                seen.push(state);
            }
            seen
        });
        let (end, along, _) = *trace.last().unwrap();
        eprintln!("park editor grind ride RESULT ollie {ollie_at} m out: states {states:?}; grinding {} ticks over {span:.2} m; at the end state {end}, {along:.2} m past the ledge's start", grinding.len());
        clean |= grinding.len() >= 10 && span > 1.0 && !states.contains(&300) && end == 100 && along > 7.0;
    }
    assert!(clean, "no ollie grinds the placed ledge and rides away");
}

/// Your board (grind): a dropped copy of the player's board whose deck edges are grind splines.
/// Once it rests, both physics' grind queries find its two edges, each tail to nose along the
/// deck's side (three stretches: up the tail, the flat, up the nose); taking it away takes them
/// out. (Grinding it was play-tested: a scripted ollie clears or falls short of a board.)
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_grind_board_is_ground() {
    use crate::input_pad::XboxPadSample;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut run = |session: &mut OriginalFacade, count: usize| {
        let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
        for _ in 0..count {
            pad.scripted_controller_zero = Some(idle);
            session.try_tick(&mut pad, frame, &mut clamp).unwrap();
        }
    };
    run(&mut session, 180);
    let (skater, _) = session.test_skater_root();
    let street = session.test_line_down(skater[0] + 3.0, skater[2], skater[1] + 1.5, skater[1] - 5.0).expect("no street");
    let at = [skater[0] + 3.0, street + 0.1, skater[2]];
    // Turned a quarter, so deck and world axes differ.
    let deck = [0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, at[0], at[1], at[2], 1.0];
    assert!(session.test_drop_grind_board(deck));
    run(&mut session, 60);
    let rest = session.test_dropped_boards()[0].1.expect("board deck");
    let low = [at[0] - 1.0, street - 1.0, at[2] - 1.0];
    let high = [at[0] + 1.0, street + 1.0, at[2] + 1.0];
    let near = |p: &[f32; 4]| (0..3).all(|axis| p[axis] > low[axis] && p[axis] < high[axis]);
    // Deck space: x across, y up, z along.
    let local = |p: &[f32; 4]| -> [f32; 3] {
        let d = [p[0] - rest[12], p[1] - rest[13], p[2] - rest[14]];
        std::array::from_fn(|c| d[0] * rest[c * 4] + d[1] * rest[c * 4 + 1] + d[2] * rest[c * 4 + 2])
    };
    // Each side's stretches (its x sign), and how far along the deck they reach each way.
    let on_edges = |segments: &[([f32; 4], [f32; 4])]| -> Vec<(f32, f32, f32)> {
        let edge = [[0.091, 0.0275, -0.385], [0.1125, 0.0075, -0.295], [0.1125, 0.0075, 0.295], [0.091, 0.0275, 0.385]];
        let on = |p: [f32; 3]| edge.iter().any(|e: &[f32; 3]| (p[0].abs() - e[0]).abs() < 2e-3 && (p[1] - e[1]).abs() < 2e-3 && (p[2] - e[2]).abs() < 2e-3);
        let mut sides: Vec<(f32, f32, f32)> = Vec::new();
        for (a, b) in segments.iter().filter(|(a, b)| near(a) && near(b)) {
            let (a, b) = (local(a), local(b));
            assert!(on(a) && on(b) && a[0].signum() == b[0].signum(), "stretch off the deck's edge: {a:?} {b:?}");
            match sides.iter_mut().find(|side| side.0 == a[0].signum()) {
                Some(side) => {
                    side.1 = side.1.min(a[2].min(b[2]));
                    side.2 = side.2.max(a[2].max(b[2]));
                }
                None => sides.push((a[0].signum(), a[2].min(b[2]), a[2].max(b[2]))),
            }
        }
        sides
    };
    let skate2 = on_edges(&session.test_skate2_grind_query(low, high));
    let skate3 = session.test_skate3_grind_segments().map(|segments| on_edges(&segments));
    session.test_remove_all();
    run(&mut session, 3);
    let skate2_after = session.test_skate2_grind_query(low, high).iter().filter(|(a, b)| near(a) && near(b)).count();
    let skate3_after = session.test_skate3_grind_segments().map(|segments| segments.iter().filter(|(a, b)| near(a) && near(b)).count());
    eprintln!("grind board RESULT deck at {:?}: Skate 2 edges {skate2:?}, Skate 3 edges {skate3:?}; taken away: Skate 2 {skate2_after}, Skate 3 {skate3_after:?}", [rest[12], rest[13] - street, rest[14]]);
    let tail_to_nose = |sides: &[(f32, f32, f32)]| sides.len() == 2 && sides.iter().all(|side| side.1 < -0.38 && side.2 > 0.38);
    assert!(tail_to_nose(&skate2), "Skate 2 grind query: {skate2:?}");
    if let Some(skate3) = skate3 {
        assert!(tail_to_nose(&skate3), "Skate 3 grind world: {skate3:?}");
        assert_eq!(skate3_after, Some(0), "Skate 3 still grinds a removed board");
    }
    assert_eq!(skate2_after, 0, "Skate 2 still grinds a removed board");
}

/// Placed Skate 3 pieces collide the way they face: ridden at from either end, Funbox A's ramps
/// (it spans 18 x 1.5 x 6 m, ramps at both ends along its length) carry the board up onto it;
/// from either side its 1.5 m side wall stops the skater at it.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_ramps_are_ridden() {
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "ab_funboxA_Downtown").unwrap();
    let mut results = Vec::new();
    for quarter in 0..4 {
        // Quarters 1 and 3 meet a ramp (the piece's length across the line), 0 and 2 a side.
        let near = if quarter % 2 == 1 { piece.max[0] - piece.min[0] } else { piece.max[2] - piece.min[2] } * 0.5;
        let trace = ride_at_pieces(
            |session, forward, _, street, now| {
                let yaw = forward[0].atan2(forward[2]) + quarter as f32 * std::f32::consts::FRAC_PI_2;
                let middle = [(piece.min[0] + piece.max[0]) * 0.5, (piece.min[2] + piece.max[2]) * 0.5];
                let centre = [now[0] + forward[0] * (8.0 + near), now[2] + forward[2] * (8.0 + near)];
                let mut at = crate::ui::object_dropper::prop_transform([0.0, street - piece.min[1], 0.0], yaw);
                at[12] = centre[0] - (at[0] * middle[0] + at[8] * middle[1]);
                at[14] = centre[1] - (at[2] * middle[0] + at[10] * middle[1]);
                assert_eq!(session.test_drop_prop("Funbox a (Downtown)", at), "placed");
                [now[0] + forward[0] * 8.0, now[2] + forward[2] * 8.0]
            },
            None,
        );
        let top = trace.iter().map(|(_, _, h)| *h).fold(f32::MIN, f32::max);
        let furthest = trace.iter().map(|(_, along, _)| *along).fold(f32::MIN, f32::max);
        results.push((quarter, top, furthest));
    }
    eprintln!("park editor ramps RESULT (quarter, deck top over the street, furthest past the near face) {results:?}");
    for (quarter, top, furthest) in results {
        if quarter % 2 == 1 {
            assert!(top > 1.0, "the board did not ride up the funbox ramp (quarter {quarter}): top {top}");
        } else {
            assert!(furthest < 0.3, "the skater rode {furthest} m into the funbox's side (quarter {quarter})");
        }
    }
}

/// A ground-snapped piece has no lip: Funbox A held 0.3 m over the street snaps down onto it
/// and, placed there, is ridden up from either end with the board rolling over its toe as
/// smoothly as over a toe exactly flush with the street, and far more smoothly than over one
/// left 2 cm up (which stops the board low on the ramp: 0.49 m up against 1.55 flush in Skate 3
/// physics, 0.23 against 1.2 in Skate 2).
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_snapped_ramp_has_no_lip() {
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "ab_funboxA_Downtown").unwrap();
    // The ride up the ramp from one end with the funbox snapped (`None`) or `Some(height)` over
    // the street: (snapped height over the street, toe bump, deck top).
    let ride = |quarter: i32, lift: Option<f32>| {
        let mut height = 0.0;
        let trace = ride_at_pieces(
            |session, forward, _, street, now| {
                let yaw = forward[0].atan2(forward[2]) + quarter as f32 * std::f32::consts::FRAC_PI_2;
                let middle = [(piece.min[0] + piece.max[0]) * 0.5, (piece.min[2] + piece.max[2]) * 0.5];
                let near = (piece.max[0] - piece.min[0]) * 0.5;
                let centre = [now[0] + forward[0] * (8.0 + near), now[2] + forward[2] * (8.0 + near)];
                let mut at = crate::ui::object_dropper::prop_transform([0.0, street + lift.unwrap_or(0.3), 0.0], yaw);
                at[12] = centre[0] - (at[0] * middle[0] + at[8] * middle[1]);
                at[14] = centre[1] - (at[2] * middle[0] + at[10] * middle[1]);
                if lift.is_none() {
                    at = session.test_hold_snapped("Funbox a (Downtown)", at).expect("no ground snap");
                    session.test_let_go();
                }
                height = at[13] - street;
                assert_eq!(session.test_drop_prop("Funbox a (Downtown)", at), "placed");
                [now[0] + forward[0] * 8.0, now[2] + forward[2] * 8.0]
            },
            None,
        );
        let window: Vec<&(i32, f32, f32)> = trace.iter().filter(|(_, along, _)| (-1.0..1.5).contains(along)).collect();
        let bump = window.windows(3).map(|w| (w[2].2 - 2.0 * w[1].2 + w[0].2).abs()).fold(0.0_f32, f32::max);
        let top = trace.iter().map(|(_, _, h)| *h).fold(f32::MIN, f32::max);
        (height, bump, top)
    };
    for quarter in [1, 3] {
        let snapped = ride(quarter, None);
        let flush = ride(quarter, Some(0.0));
        let lip = ride(quarter, Some(0.02));
        eprintln!("park editor no lip RESULT quarter {quarter} (height over the street, toe bump, deck top): snapped {snapped:?}, flush {flush:?}, 2 cm lip {lip:?}");
        assert!(snapped.0.abs() < 0.005, "snapped {} m off the street", snapped.0);
        // A lip stops the board low on the ramp (Skate 2 physics: 0.23 m up against 1.2 flush).
        assert!(lip.2 < flush.2 * 0.6, "the ride does not show a 2 cm lip: {lip:?} against flush {flush:?}");
        assert!(snapped.2 > flush.2 * 0.95 && snapped.1 <= flush.1 * 1.2 + 0.001, "the board caught on the snapped ramp's toe: {snapped:?} (flush {flush:?}, lip {lip:?})");
    }
}

/// Placed static shapes are solid from outside: ridden into, a Skate 3 street pad's wall (3 m
/// tall) stops the skater before it, and so does the mission speaker.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_skate3_pieces_are_solid() {
    let pad = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "aa_padStreet_lrg_Downtown").unwrap();
    let trace = ride_at_pieces(
        |session, forward, right, street, now| {
            // The pad spans model x 0..6: its middle on the line, its near face 8 m ahead.
            let front = [now[0] + forward[0] * 8.0, street, now[2] + forward[2] * 8.0];
            assert_eq!(session.test_drop_prop("Pad street (large, Downtown)", piece_along(forward, right, front, 3.0, pad.min[1])), "placed");
            [front[0], front[2]]
        },
        None,
    );
    let furthest = trace.iter().map(|(_, along, _)| *along).fold(f32::MIN, f32::max);
    // The speaker's box base middle 0.6 m past `front`: its near face about 0.06 m past it.
    let trace = ride_at_pieces(
        |session, forward, right, street, now| {
            let front = [now[0] + forward[0] * 8.0, street, now[2] + forward[2] * 8.0];
            let mut at = piece_along(forward, right, front, 0.0, 0.0);
            at[12] += forward[0] * 0.6;
            at[14] += forward[2] * 0.6;
            assert_eq!(session.test_drop_prop("Speaker", at), "placed");
            [front[0], front[2]]
        },
        None,
    );
    let speaker = trace.iter().map(|(_, along, _)| *along).fold(f32::MIN, f32::max);
    eprintln!("park editor solid RESULT the deck got {furthest:.2} m past the pad's near face, {speaker:.2} m past `front` at the speaker");
    assert!(furthest < 0.0, "the skater rode {furthest} m into the street pad");
    assert!(speaker < 0.0, "the skater rode into the speaker ({speaker} m)");
}

/// Skate 3 snapping in the game: a street pad placed on the ground, a second one held a little
/// off its side snaps flush beside it, one held over it sits on its top, and a Skate 3 prop
/// never snaps.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_snaps_skate3_pieces() {
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
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let (skater, _) = session.test_skater_root();
    let at = |p: [f32; 3]| crate::ui::object_dropper::prop_transform(p, 0.0);
    let base = [skater[0] + 4.0, skater[1] - 0.5, skater[2]];
    assert_eq!(session.test_drop_prop("Pad street (large, Downtown)", at(base)), "placed");
    let side = session.test_hold_snapped("Pad street (large, Downtown)", at([base[0] + 6.5, base[1] + 0.2, base[2] - 0.3])).expect("no side snap");
    let top = session.test_hold_snapped("Pad street (large, Downtown)", at([base[0] + 0.4, base[1] + 3.6, base[2] + 0.2])).expect("no stack snap");
    let prop = session.test_hold_snapped("Large quarter pipe", at([base[0] + 6.5, base[1], base[2]]));
    eprintln!("park editor snap RESULT base {base:?}; side {:?}; top {:?}; Skate 3 prop {:?}", &side[12..15], &top[12..15], prop.map(|t| [t[12], t[13], t[14]]));
    let close = |t: [f32; 16], want: [f32; 3]| (0..3).all(|lane| (t[12 + lane] - want[lane]).abs() < 0.01);
    assert!(close(side, [base[0] + 6.0, base[1], base[2]]), "side snap off");
    assert!(close(top, [base[0], base[1] + 3.0, base[2]]), "stack snap off");
    assert!(prop.is_none(), "a Skate 3 prop snapped");
    // Away from the placed pad, on a flat stretch of street (its footprint's corners and middle
    // within 1 cm): a pad held a little over it rests on it, one held high stays where it is.
    let piece = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "aa_padStreet_lrg_Downtown").unwrap();
    let (spot, surface) = [[-15.0, 0.0], [0.0, -15.0], [-15.0, -15.0], [0.0, 15.0], [15.0, 15.0], [-15.0, 15.0]]
        .into_iter()
        .find_map(|[dx, dz]| {
            let spot = [base[0] + dx, base[2] + dz];
            let heights: Vec<f32> = [[piece.min[0], piece.min[2]], [piece.max[0], piece.min[2]], [piece.min[0], piece.max[2]], [piece.max[0], piece.max[2]], [(piece.min[0] + piece.max[0]) * 0.5, (piece.min[2] + piece.max[2]) * 0.5]]
                .into_iter()
                .map(|[x, z]| session.test_line_down(spot[0] + x, spot[1] + z, skater[1] + 5.0, skater[1] - 5.0))
                .collect::<Option<Vec<f32>>>()?;
            let (low, high) = heights.iter().fold((f32::MAX, f32::MIN), |(l, h), &y| (l.min(y), h.max(y)));
            (high - low < 0.01).then_some((spot, heights[4]))
        })
        .expect("no flat street near the start");
    let low = session.test_hold_snapped("Pad street (large, Downtown)", at([spot[0], surface + 0.6, spot[1]]));
    let high = session.test_hold_snapped("Pad street (large, Downtown)", at([spot[0], surface + 2.5, spot[1]]));
    eprintln!("park editor ground snap RESULT surface {surface}; low base {:?}; high {:?}", low.map(|t| t[13]), high.map(|t| t[13]));
    let rest = low.expect("no ground snap")[13];
    assert!((rest - surface).abs() < 0.01, "ground snap off: base {rest}, street {surface}");
    assert!(high.is_none(), "a pad held 2.5 m up snapped");
}

/// Skate 3 park pieces (parkassets.big beside the game's archives): the object list's Terrain
/// tab takes the first premade piece into the editor's hands, drawn as a DMO under its catalog
/// ID; placed, it is static, collides (its own collision mesh), and draws where it was put.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_places_skate3_pieces() {
    use crate::input_pad::XboxPadSample;
    use crate::ui::object_dropper::{PARK_PIECES, park_piece};
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
    run(&mut session, idle, 180);
    let (skater, _) = session.test_skater_root();
    let (x, z) = (skater[0] + 4.0, skater[2]);
    let ground = session.test_line_down(x, z, skater[1] + 30.0, skater[1] - 30.0).unwrap_or_else(|| panic!("no ground under {x:.1}, {z:.1} (skater at {skater:?})"));
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    assert!(session.test_editor().open, "LB + D-pad Left did not open the editor");
    // X opens the list on Props; RT goes round to Terrain; A opens Premade, A takes its first.
    run(&mut session, press(0x4000), 1);
    run(&mut session, idle, 1);
    run(&mut session, XboxPadSample { right_trigger: 255, ..idle }, 1);
    run(&mut session, idle, 1);
    for _ in 0..2 {
        run(&mut session, press(0x1000), 1);
        run(&mut session, idle, 1);
    }
    let editor = session.test_editor();
    let (name, held) = editor.held.unwrap_or_else(|| panic!("nothing taken from Terrain: {editor:?}"));
    assert_eq!(name, "Quarterpipe preset (Downtown)");
    let piece = PARK_PIECES[park_piece(0x2c70170700030297).unwrap()];
    let drawn = |session: &OriginalFacade| {
        session.try_object_dropper_visual_snapshot().unwrap().dynamic_objects.iter().filter(|object| object.asset_id == piece.id).count()
    };
    assert_eq!(drawn(&session), 1, "the held piece is not drawn");
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let editor = session.test_editor();
    assert_eq!(editor.placed.len(), 1, "not placed: {editor:?}");
    let placements = crate::ui::object_dropper::mission_placements();
    assert_eq!(placements.len(), 1, "the piece has no collision body");
    eprintln!("park editor skate3 RESULT {} placed, {} collision triangles", piece.recipe, placements[0].shape.triangles.len());
    run(&mut session, idle, 60);
    assert_eq!(session.test_editor().placed[0], held, "the static piece moved");
    assert_eq!(drawn(&session), 1, "the placed piece is not drawn");
    // A second piece by name beside the skater: the world line test lands on it.
    let pad_piece = PARK_PIECES[park_piece(0x2c701707000301a5).unwrap()];
    let at = crate::ui::object_dropper::prop_transform([x, ground, z], 0.0);
    assert_eq!(session.test_drop_prop("Pad street (large, Downtown)", at), "placed");
    run(&mut session, idle, 2);
    let middle = [(pad_piece.min[0] + pad_piece.max[0]) * 0.5, (pad_piece.min[2] + pad_piece.max[2]) * 0.5];
    let (mx, mz) = (x + at[0] * middle[0] + at[8] * middle[1], z + at[2] * middle[0] + at[10] * middle[1]);
    let under = session.test_line_down(mx, mz, ground + pad_piece.max[1] + 5.0, ground - 5.0).expect("no hit over the street pad");
    eprintln!("park editor skate3 RESULT street pad: ground {ground:.3}, line hit over its middle {under:.3} (+{:.3} m, piece top +{:.3})", under - ground, pad_piece.max[1]);
    assert!(under - ground > 0.05, "the line test passed through the street pad");
    assert_eq!(crate::ui::object_dropper::mission_placements().len(), 2);
}


/// The Terrain list shows each Skate 3 piece once, not once per theme, and takes it in the
/// editor settings' theme (Start over the list; the settings are set back to Downtown after).
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT, with Skate 3's parkassets.big in data\\content"]
fn park_editor_lists_themes_once() {
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
    let menu = || crate::ui::object_dropper::menu().expect("no editor panel");
    run(&mut session, idle, 180);
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    assert!(session.test_editor().open, "LB + D-pad Left did not open the editor");
    // X, RT round to Terrain, Down to Street: 33 rows, not its 99 entries.
    run(&mut session, press(0x4000), 1);
    run(&mut session, idle, 1);
    run(&mut session, XboxPadSample { right_trigger: 255, ..idle }, 1);
    run(&mut session, idle, 1);
    run(&mut session, press(0x0002), 1);
    run(&mut session, idle, 1);
    let groups = menu();
    eprintln!("park editor themes RESULT Terrain groups {:?}", groups.rows);
    assert_eq!(groups.rows.iter().find(|row| row.0 == "Street").and_then(|row| row.1), Some(33), "{:?}", groups.rows);
    run(&mut session, press(0x1000), 1);
    run(&mut session, idle, 1);
    let items = menu();
    eprintln!("park editor themes RESULT {} {:?} {:?}", items.title, items.rows, items.info);
    assert!(items.title.ends_with("(1/33)") && items.rows[0].0 == "Pad street (large)" && items.rows[1].0 == "Pad street (medium)", "{items:?}");
    assert!(items.info[0].contains("Downtown"), "{items:?}");
    // Start over the list: the settings, last row (Up wraps) is the theme; Right is Industrial.
    let theme = |session: &mut OriginalFacade| session.test_editor().settings[8];
    for buttons in [0x0010, 0x0001, 0x0008] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    assert!(session.test_editor().settings_open && theme(&mut session) == 1, "theme not Industrial: {:?}", session.test_editor());
    // B back to the list, Down to the medium pad, A takes it in Industrial.
    for buttons in [0x2000, 0x0002, 0x1000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    let editor = session.test_editor();
    assert_eq!(editor.held.map(|held| held.0), Some("Pad street (medium, Industrial)"), "{editor:?}");
    // Delete it, and the theme back to Downtown (X sets the default).
    for buttons in [0x8000, 0x0010, 0x4000, 0x2000] {
        run(&mut session, press(buttons), 1);
        run(&mut session, idle, 1);
    }
    assert_eq!(theme(&mut session), 0);
}

/// Marquee characters and models: Chris Cole and the podium placed beside the skater stream in
/// and draw; his idle clip moves his hips, the podium stays on its first frame.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_places_marquee_characters() {
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
    // Off the board (Y) and settled, as the other editor tests stand.
    run(&mut session, idle, 30);
    run(&mut session, XboxPadSample { buttons: 0x8000, ..idle }, 1);
    run(&mut session, idle, 150);
    let clips = session.test_character_clips();
    eprintln!("park editor characters RESULT {} viewer clips: {clips:?}", clips.len());
    let (skater, _) = session.test_skater_root();
    let ground = session.test_line_down(skater[0] + 3.0, skater[2], skater[1] + 5.0, skater[1] - 5.0).expect("ground");
    assert_eq!(session.test_drop_prop("Chris Cole", crate::ui::object_dropper::prop_transform([skater[0] + 3.0, ground, skater[2]], 0.0)), "placed");
    assert_eq!(session.test_drop_prop("Podium", crate::ui::object_dropper::prop_transform([skater[0] - 3.0, ground, skater[2]], 0.0)), "placed");
    if std::env::var_os("PARK_EDITOR_PROBE").is_some() {
        for name in ["Big Black's SUV", "Chair and table", "Pen", "Railing cap", "Tally board"] {
            eprintln!("park editor probe {name}: {}", session.test_drop_prop(name, crate::ui::object_dropper::prop_transform([skater[0], ground, skater[2] + 4.0], 0.0)));
        }
        run(&mut session, idle, 300);
        eprintln!("park editor probe RESULT drawn {}", session.test_character_hips().len());
        return;
    }
    let mut waited = 0;
    while session.test_character_hips().len() < 2 && waited < 600 {
        run(&mut session, idle, 10);
        waited += 10;
    }
    let first = session.test_character_hips();
    eprintln!("park editor characters RESULT {} drawn after {waited} ticks: hips {first:?}", first.len());
    assert_eq!(first.len(), 2, "the characters' models did not stream in");
    // Played in place, a character's bones move while its hips hold over the placement.
    let posed = session.test_character_pose();
    run(&mut session, idle, 45);
    let later = session.test_character_pose();
    let moved = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
    let (cole, podium) = (moved(&posed[0], &later[0]), moved(&posed[1], &later[1]));
    eprintln!("park editor characters RESULT bones moved: Chris Cole {cole:.4}, podium {podium:.4}");
    assert!(cole > 0.001, "Chris Cole's idle clip did not play");
    assert!(podium < 1e-5, "the podium moved");
    // Placed and drawn, each is solid: a line down over the podium meets its top.
    let top = session.test_line_down(skater[0] - 3.0, skater[2], ground + 6.0, ground - 1.0).expect("no hit over the podium");
    eprintln!("park editor characters RESULT line over the podium hits +{:.3} m", top - ground);
    assert!(top - ground > 0.2, "the podium is not solid");
    // Their collision is their posed mesh, not a box.
    let counts: Vec<usize> = crate::ui::object_dropper::mission_placements().iter().map(|placement| placement.shape.triangles.len()).collect();
    let head = session.test_line_down(skater[0] + 3.0, skater[2], ground + 6.0, ground - 1.0).map(|hit| hit - ground);
    eprintln!("park editor characters RESULT collision triangles {counts:?}; line over Chris Cole hits {head:?}");
    assert!(counts.iter().all(|&count| count > 12), "a character's collision is still a box: {counts:?}");
    // Big Black's SUV, a marquee recipe in the vehicle effect, draws too.
    assert_eq!(session.test_drop_prop("Big Black's SUV", crate::ui::object_dropper::prop_transform([skater[0], ground, skater[2] + 6.0], 0.0)), "placed");
    let mut waited = 0;
    while session.test_character_hips().len() < 3 && waited < 600 {
        run(&mut session, idle, 10);
        waited += 10;
    }
    assert_eq!(session.test_character_hips().len(), 3, "Big Black's SUV did not draw");
}

/// A NIS scene's clip on a placed character plays in place: its hips stay over the placement.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_scene_animation_plays_in_place() {
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
    run(&mut session, idle, 30);
    run(&mut session, XboxPadSample { buttons: 0x8000, ..idle }, 1);
    run(&mut session, idle, 150);
    let (skater, _) = session.test_skater_root();
    let at = [skater[0] + 3.0, skater[1], skater[2]];
    let file = session.test_set_animation("Chris Cole", "goto_slappy");
    assert_eq!(session.test_drop_prop("Chris Cole", crate::ui::object_dropper::prop_transform(at, 0.0)), "placed");
    let mut farthest = 0.0f32;
    let mut drawn = 0;
    for _ in 0..12 {
        run(&mut session, idle, 10);
        if let Some(hips) = session.test_character_hips().first() {
            drawn += 1;
            farthest = farthest.max((hips[0] - at[0]).hypot(hips[2] - at[2]));
        }
    }
    eprintln!("park editor scene animation RESULT {file}: drawn {drawn} times, hips at most {farthest:.3} m from the placement");
    assert!(drawn > 0, "the character never drew");
    assert!(farthest < 1.0, "the clip carried the character {farthest} m away");
}

/// Every animation file the editor offers, tried on Chris Cole's skeleton.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn park_editor_animation_census() {
    use crate::input_pad::XboxPadSample;
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let idle = XboxPadSample { buttons: 0, left_trigger: 0, right_trigger: 0, left_x: 0, left_y: 0, right_x: 0, right_y: 0 };
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    for _ in 0..30 {
        pad.scripted_controller_zero = Some(idle);
        session.try_tick(&mut pad, frame, &mut clamp).unwrap();
    }
    let (ok, clips, bad) = session.test_animation_census();
    eprintln!("park editor animations RESULT {ok} files play ({clips} clips), {} do not: {bad:?}", bad.len());
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
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    assert!(session.test_editor().open, "LB + D-pad Left did not open the editor");
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

/// PC park editor (Object Dropper), driven by the pad as a player would: LB + D-pad Left opens it over a
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
    // LB, then LB + D-pad Left.
    run(&mut session, press(0x0100), 2);
    run(&mut session, press(0x0100 | 0x0004), 1);
    run(&mut session, idle, 2);
    let editor = session.test_editor();
    assert!(editor.open, "LB + D-pad Left did not open the editor");
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
            run(&mut session, XboxPadSample { buttons: 0x0100 | 0x0004, ..idle }, 1);
            assert!(session.test_editor().open, "LB + D-pad Left did not open the editor");
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
    assert_eq!(magic, b"SK2PCR04");
    assert!(with > 0 && same, "the saved replay lost the board");
}
