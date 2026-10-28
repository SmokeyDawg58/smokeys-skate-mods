//! Original-data raw-pad grab and explicit bail probes through the public frame owner.
//! Button and trigger packets enter the same device/map/command/Actor path as play.
use super::boot_original_first_visual;
use crate::{
    SelectedQueuedFrameOutcome, input_pad::XboxPadSample,
    physical_player_physics_state::CurrentPhysicalState,
};

fn packet(buttons: u16, left_trigger: u8, right_trigger: u8) -> XboxPadSample {
    XboxPadSample {
        buttons,
        left_trigger,
        right_trigger,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    }
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_y_dismount_walk_and_remount_connected_route() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut saw_offboard = false;
    let mut remounted = false;
    let mut before_walk = None;
    let mut after_walk = None;
    for step in 0..420 {
        pad.scripted_controller_zero = Some(XboxPadSample {
            buttons: if step == 32 || step == 270 { 0x8000 } else { 0 },
            left_y: if (100..220).contains(&step) {
                24_000
            } else {
                0
            },
            ..packet(0, 0, 0)
        });
        let outcome = session
            .try_tick(&mut pad, frame, &mut clamp)
            .unwrap_or_else(|error| {
                panic!(
                    "dismount/walk/remount frame{step}: {error:?}; physical={:?}",
                    session.controlled_physical_state_for_test()
                )
            });
        assert!(matches!(
            outcome,
            SelectedQueuedFrameOutcome::Completed { .. }
        ));
        let state = session.controlled_physical_state_for_test();
        saw_offboard |= matches!(
            state,
            CurrentPhysicalState::OffBoard
                | CurrentPhysicalState::OffBoardAir
                | CurrentPhysicalState::OffBoardLocomotion
        );
        remounted |= step > 270 && saw_offboard && state == CurrentPhysicalState::Ground;
        if step == 99 {
            before_walk = Some(session.controlled_skater_position_for_test());
        }
        if step == 219 {
            after_walk = Some(session.controlled_skater_position_for_test());
        }
    }
    assert!(
        saw_offboard,
        "Y never dismounted through physical offboard state"
    );
    let a = before_walk.unwrap();
    let b = after_walk.unwrap();
    let travel = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
    assert!(
        travel > 0.3,
        "left stick never moved actual offboard skater: {a:?}->{b:?}"
    );
    assert!(remounted, "second Y never returned to onboard Ground");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_left_stick_reaches_kickturn_through_shared_speed_conditions() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut states = Vec::new();
    let mut entered_at = None;
    let mut exited_after_release = false;
    let mut board_turn_samples = Vec::new();
    for step in 0..45 {
        // Packet axes come from the first reached hands-on Kickturn boundary.
        pad.scripted_controller_zero = Some(XboxPadSample {
            left_x: if (8..26).contains(&step) { -21_930 } else { 0 },
            left_y: if (8..26).contains(&step) { 2_322 } else { 0 },
            ..packet(0, 0, 0)
        });
        session
            .try_tick(&mut pad, frame, &mut clamp)
            .unwrap_or_else(|error| {
                panic!("left-stick frame {step}: {error:?}; states {states:?}")
            });
        let action = session.controlled_action_observation_for_test(b"KickTurn");
        states.push(action.0);
        if matches!(step, 7 | 8 | 9 | 16 | 25 | 26 | 30) {
            board_turn_samples.push((step, session.controlled_board_turn_for_test()));
        }
        if action.0 == 50 && entered_at.is_none() {
            entered_at = Some(step);
        }
        if step >= 26 && entered_at.is_some() && action.0 != 50 {
            exited_after_release = true;
        }
    }
    assert!(
        entered_at.is_some(),
        "raw left stick did not enter Action Kickturn50: {states:?}"
    );
    assert!(
        exited_after_release,
        "Kickturn did not exit after raw left-stick release: {states:?}"
    );
    assert!(
        board_turn_samples.iter().any(|(step, sample)| {
            (9..26).contains(step)
                && sample.as_ref().is_some_and(|(_, _, balance, spin, _, _)| {
                    balance.abs() > 0.0 && spin.abs() > 0.0
                })
        }),
        "Kickturn never delivered nonzero Balance and Spin to board physics: {board_turn_samples:?}",
    );
    let board_before = board_turn_samples
        .iter()
        .find(|(step, _)| *step == 7)
        .unwrap()
        .1
        .as_ref()
        .unwrap()
        .5;
    let board_held = board_turn_samples
        .iter()
        .find(|(step, _)| *step == 25)
        .unwrap()
        .1
        .as_ref()
        .unwrap()
        .5;
    // Compare the board's orientation, not merely the selected state or an intent.
    // This tolerance rejects stationary numerical drift; it is not game tuning.
    assert!(
        (board_held[0] - board_before[0]).abs() > 0.1,
        "held Kickturn did not rotate the physical board: {board_turn_samples:?}"
    );
    println!("raw left-stick Kickturn entered at {entered_at:?} and exited; states {states:?}");
    println!("board turn samples: {board_turn_samples:?}");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_partial_left_trigger_reaches_air_grab_intent() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let ollie = session.controlled_right_pattern_points_for_test("Ollie");
    assert_eq!(ollie.len(), 2);
    let mut trace = Vec::new();
    let mut grabbed_in_air = false;
    for step in 0..60 {
        let before = session.controlled_physical_state_for_test();
        let grab = step >= 11 && before == CurrentPhysicalState::Air && !grabbed_in_air;
        pad.scripted_controller_zero = if grab {
            // An analog half trigger is an air grab, but not a full ground grab.
            Some(packet(0, 128, 0))
        } else {
            match step {
                6 | 7 => Some(crate::input_pad::device::tests::scripted_right_point(
                    ollie[0],
                )),
                9 | 10 => Some(crate::input_pad::device::tests::scripted_right_point(
                    ollie[1],
                )),
                _ => None,
            }
        };
        let result = session
            .try_tick(&mut pad, frame, &mut clamp)
            .unwrap_or_else(|error| {
                panic!("air-grab frame {step}, before {before:?}: {error:?}; trace {trace:?}")
            });
        assert!(matches!(
            result,
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let action = session.controlled_action_observation_for_test(b"LeftAirGrab");
        let ground = session.controlled_action_observation_for_test(b"LeftGroundGrab");
        let motion = session.controlled_motion_observation_for_test(b"WantsLeftAirGrab");
        let after = session.controlled_physical_state_for_test();
        trace.push((step, before, grab, action, ground, motion, after));
        if step == 9 {
            let ollie_action = session.controlled_action_observation_for_test(b"Ollie");
            assert_eq!(
                ollie_action.1[1],
                Some(1.0_f32.to_bits()),
                "trace {trace:?}"
            );
        }
        if grab {
            assert_eq!(action.1[1], Some(1.0_f32.to_bits()), "trace {trace:?}");
            assert_eq!(ground.1[1], None, "trace {trace:?}");
            grabbed_in_air = true;
            break;
        }
    }
    assert!(
        grabbed_in_air,
        "Ollie never supplied an air grab window: {trace:?}"
    );
    println!("raw partial-left-trigger air grab trace: {trace:?}");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_partial_left_trigger_holds_air_grab_through_action_and_motion() {
    replay_held_air_grab(false);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_partial_right_trigger_holds_air_grab_through_action_and_motion() {
    replay_held_air_grab(true);
}

fn replay_held_air_grab(right: bool) {
    let air_intent: &[u8] = if right {
        b"RightAirGrab"
    } else {
        b"LeftAirGrab"
    };
    let ground_intent: &[u8] = if right {
        b"RightGroundGrab"
    } else {
        b"LeftGroundGrab"
    };
    let wants_intent: &[u8] = if right {
        b"WantsRightAirGrab"
    } else {
        b"WantsLeftAirGrab"
    };
    let grab_intent: &[u8] = if right { b"BSGrab" } else { b"FSGrab" };
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let ollie = session.controlled_right_pattern_points_for_test("Ollie");
    assert_eq!(ollie.len(), 2);
    let mut trace = Vec::new();
    let mut reached_action_left_grab = false;
    let mut reached_motion_fs_grab = false;
    for step in 0..80 {
        let before = session.controlled_physical_state_for_test();
        let holding_grab = (27..=46).contains(&step);
        pad.scripted_controller_zero = if holding_grab {
            // Fill 0x825fbbc0 distinguishes a positive air trigger from a
            // full ground trigger. Keep the raw half trigger across the
            // InAir.Idle selection and subsequent Motion updates.
            Some(if right {
                packet(0, 0, 128)
            } else {
                packet(0, 128, 0)
            })
        } else {
            match step {
                6 | 7 => Some(crate::input_pad::device::tests::scripted_right_point(
                    ollie[0],
                )),
                9 | 10 => Some(crate::input_pad::device::tests::scripted_right_point(
                    ollie[1],
                )),
                _ => None,
            }
        };
        let result = session.try_tick(&mut pad, frame, &mut clamp).unwrap_or_else(|error| {
            let action = session.controlled_action_observation_for_test(air_intent);
            let motion = session.controlled_motion_observation_for_test(wants_intent);
            let physical = session.controlled_physical_state_for_test();
            let current_animation = session.controlled_current_animation_for_test();
            panic!("air-grab frame {step}, holding {holding_grab}, before {before:?}: {error:?}; partial Action {action:?}, Motion {motion:?}, physical {physical:?}, current animation {current_animation:?}; completed trace {trace:?}")
        });
        assert!(matches!(
            result,
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let action = session.controlled_action_observation_for_test(air_intent);
        let ground = session.controlled_action_observation_for_test(ground_intent);
        let motion = session.controlled_motion_observation_for_test(wants_intent);
        let fs_grab = session.controlled_motion_observation_for_test(grab_intent);
        let after = session.controlled_physical_state_for_test();
        let visual_counter = session.try_visual_snapshot().unwrap().camera_update_counter;
        trace.push((
            step,
            before,
            holding_grab,
            action,
            ground,
            motion,
            fs_grab,
            after,
            visual_counter,
        ));
        assert_eq!(visual_counter, (step + 2) as u32, "trace {trace:?}");
        if step == 9 {
            let ollie_action = session.controlled_action_observation_for_test(b"Ollie");
            assert_eq!(
                ollie_action.1[1],
                Some(1.0_f32.to_bits()),
                "trace {trace:?}"
            );
        }
        if step == 26 {
            assert_eq!(action.0, 43, "expected InAir.Idle before LT: {trace:?}");
            assert_eq!(after, CurrentPhysicalState::Air, "trace {trace:?}");
            println!(
                "air-grab pre-LT current animation: {:?}",
                session.controlled_current_animation_for_test()
            );
        }
        if step == 27 {
            assert_eq!(
                session.controlled_grab_busy_for_test(),
                Some(if right { [1, 0] } else { [0, 1] }),
                "HandBusy must increment only the selected retained Motion counter"
            );
        }
        if holding_grab {
            assert_eq!(action.1[1], Some(1.0_f32.to_bits()), "trace {trace:?}");
            assert_eq!(ground.1[1], None, "trace {trace:?}");
        }
        reached_action_left_grab |= action.0 == if right { 146 } else { 138 };
        reached_motion_fs_grab |= motion.0 == if right { 658 } else { 647 };
    }
    assert!(
        reached_action_left_grab,
        "held LT never selected Action LeftGrab leaf138: {trace:?}"
    );
    assert!(
        reached_motion_fs_grab,
        "held LT never selected Motion FSGrabInto647: {trace:?}"
    );
    println!("raw partial-left-trigger held air grab trace: {trace:?}");
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_trigger_and_both_stick_clicks_reach_explicit_bail_request() {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let mut trace = Vec::new();
    let mut wipeout_ground = None;
    let mut action_wipeout = None;
    let mut recovery_snapshots = Vec::new();
    let mut last_recovery = session.controlled_wipeout_recovery_for_test();
    let mut teleport_entry = None;
    let mut recovered_ground = None;
    let mut exit_effects = None;
    let mut after_teleport_entry = Vec::new();
    for step in 0..480 {
        let before_effects =
            (step >= 180).then(|| session.controlled_wipeout_exit_effects_for_test());
        // XInput bit 6/7 are the left/right stick clicks. The original cvar
        // enables a request on their newly pressed edge with both full triggers.
        pad.scripted_controller_zero = (step == 8).then_some(packet(0x00c0, 255, 255));
        let result = match session.try_tick(&mut pad, frame, &mut clamp) {
            Ok(result) => result,
            Err(error) => panic!("explicit-bail frame {step}: {error:?}; trace {trace:?}"),
        };
        assert!(matches!(
            result,
            SelectedQueuedFrameOutcome::Completed {
                update_mask: 3,
                missing_receivers: 0,
                ..
            }
        ));
        let action = session.controlled_action_observation_for_test(b"WipeOutRequest");
        let motion = session.controlled_motion_observation_for_test(b"WipeOutRequest");
        let physical = session.controlled_physical_state_for_test();
        if teleport_entry.is_some() && physical == CurrentPhysicalState::Ground {
            recovered_ground.get_or_insert(step);
        }
        trace.push((step, action, motion, physical));
        let recovery = session.controlled_wipeout_recovery_for_test();
        if let Some(snapshot) = recovery {
            let changed = if let Some(previous) = last_recovery {
                previous.auto_teleport_pending != snapshot.auto_teleport_pending
                    || previous.request_teleport != snapshot.request_teleport
            } else {
                false
            };
            if matches!(step, 8 | 76 | 179 | 299) || changed {
                recovery_snapshots.push((step, snapshot));
            }
        }
        last_recovery = recovery;
        assert_eq!(
            session.try_visual_snapshot().unwrap().camera_update_counter,
            (step + 2) as u32,
        );
        if step == 8 {
            assert_eq!(action.1[1], Some(1.0_f32.to_bits()), "trace {trace:?}");
        }
        if physical == crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround {
            wipeout_ground.get_or_insert(step);
        }
        if action.0 == 23 {
            action_wipeout.get_or_insert(step);
        }
        if physical == CurrentPhysicalState::Teleporting {
            if teleport_entry.is_none() {
                teleport_entry = session.controlled_teleporting_entry_for_test();
                exit_effects = before_effects
                    .map(|before| (before, session.controlled_wipeout_exit_effects_for_test()));
            } else {
                after_teleport_entry.push((
                    step,
                    physical,
                    session.controlled_teleporting_entry_for_test(),
                ));
            }
        }
    }
    assert_eq!(wipeout_ground, Some(8), "trace {trace:?}");
    assert_eq!(action_wipeout, Some(9), "trace {trace:?}");
    assert!(
        trace.iter().any(|entry| entry.0 == 76 && entry.2.0 == 30),
        "Motion recovery state30 was never selected: {trace:?}"
    );
    let first_state_after_wipeout = trace
        .iter()
        .find(|(step, _, _, state)| {
            *step > 8
                && *state
                    != crate::physical_player_physics_state::CurrentPhysicalState::WipeoutGround
        })
        .map(|(step, _, _, state)| (*step, *state));
    println!(
        "raw explicit bail: WipeoutGround step {wipeout_ground:?}, Action Wipeout step {action_wipeout:?}, Motion recovery step 76, first physical exit {first_state_after_wipeout:?}, final completed physical {:?}; teleport entry {teleport_entry:?}, after entry {after_teleport_entry:?}, exit effects {exit_effects:?}, recovered Ground at {recovered_ground:?}; recovery {recovery_snapshots:?}",
        trace.last().unwrap().3,
    );
    assert_eq!(
        teleport_entry,
        Some((
            Some(300),
            Some(300),
            Some(702),
            Some(702),
            false,
            Some(1),
            Some(0),
            Some(0),
            true
        )),
        "source transition/Enter was not reached in 480 frames; recovery {recovery_snapshots:?}"
    );
    let ((before_wheel, before_clears), (after_wheel, after_clears)) =
        exit_effects.expect("transition frame board/request observations");
    assert!(
        before_wheel,
        "Wipeout board mode was not active before Exit"
    );
    assert!(!after_wheel, "SetStandard did not clear free-wheel mode");
    assert_eq!(
        after_clears,
        before_clears.wrapping_add(1),
        "Update2 did not clear the Wipeout ledger exactly once"
    );
    assert!(
        after_teleport_entry.iter().any(|(step, _, _)| *step >= 258),
        "same-session replay did not reach a frame after Teleporting entry"
    );
    assert!(
        recovered_ground.is_some(),
        "bail never recovered onto the actual board in480 ticks"
    );
}

/// One boot and one uninterrupted device/command/Actor/physics session. The
/// stage-local timings come from the existing individual raw-pad probes.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_mixed_controls_move_trick_grab_bail_and_recover() {
    mixed_controls_move_trick_grab_bail_and_recover(-21_930, 2_322, false);
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn original_raw_full_left_steering_retains_wheel_axes() {
    mixed_controls_move_trick_grab_bail_and_recover(i16::MIN, 0, true);
}

fn mixed_controls_move_trick_grab_bail_and_recover(left_x: i16, left_y: i16, steering_only: bool) {
    let (mut session, mut pad, frame) = boot_original_first_visual();
    let mut clamp = || Err::<i32, _>("hands clamp service is not mounted by this test");
    let ollie = session.controlled_right_pattern_points_for_test("Ollie");
    let flip = session.controlled_right_pattern_points_for_test("Kickflip");
    assert_eq!(ollie.len(), 2);
    assert!(flip.len() >= 2);
    let mut total_ticks = 0_u32;
    for (stage, duration) in [
        ("push", 120),
        ("steer", 45),
        ("crouch_ollie", 163),
        ("kickflip", 75),
        ("grab", 80),
        ("bail_recovery", 480),
        ("recovered_push", 120),
    ] {
        // This focused wheel-joint regression stays near spawn. The existing
        // uninterrupted tricks/bail regression below still runs every stage.
        if steering_only && stage == "crouch_ollie" {
            return;
        }
        assert!(
            matches!(
                session.controlled_physical_state_for_test(),
                CurrentPhysicalState::Ground | CurrentPhysicalState::GroundAnimation
            ),
            "{stage} must begin on the actual board after the preceding release"
        );
        if stage == "grab" {
            assert_eq!(
                session.controlled_grab_busy_for_test(),
                Some([0, 0]),
                "grab stage must begin with both retained hands released"
            );
        }
        let board_start = session.controlled_board_kinematics_for_test().0;
        let skater_start = session.controlled_skater_position_for_test();
        let mut peak_y = board_start[1];
        let mut minimum_up = 1.0_f32;
        let mut saw_air = false;
        let mut landed = false;
        let mut named = false;
        let mut released = false;
        let mut steering = false;
        let mut steer_start = None;
        let mut steer_end = None;
        let mut grab_busy = false;
        let mut grab_released = false;
        let mut wipeout = false;
        let mut teleport = false;
        let mut recovered = false;
        let mut exit_effects = None;
        let mut trace = std::collections::VecDeque::new();
        for step in 0..duration {
            // An explicit neutral packet releases all controls between gestures.
            let mut sample = packet(0, 0, 0);
            match stage {
                "push" | "recovered_push" if (32..44).contains(&step) => {
                    sample.buttons = 1 | 0x1000;
                }
                "steer" if (8..26).contains(&step) => {
                    sample.left_x = left_x;
                    sample.left_y = left_y;
                }
                "crouch_ollie" if (32..62).contains(&step) => {
                    sample = crate::input_pad::device::tests::scripted_right_point(ollie[0]);
                }
                "crouch_ollie" if (63..65).contains(&step) => {
                    sample = crate::input_pad::device::tests::scripted_right_point(ollie[1]);
                }
                "kickflip" if (6..24).contains(&step) => {
                    sample = crate::input_pad::device::tests::scripted_right_point(
                        flip[(step - 6).min(flip.len() - 1)],
                    );
                }
                "grab" if step == 6 || step == 7 => {
                    sample = crate::input_pad::device::tests::scripted_right_point(ollie[0]);
                }
                "grab" if step == 9 || step == 10 => {
                    sample = crate::input_pad::device::tests::scripted_right_point(ollie[1]);
                }
                "grab" if (27..=46).contains(&step) => sample.left_trigger = 128,
                "bail_recovery" if step == 8 => sample = packet(0x00c0, 255, 255),
                _ => {}
            }
            pad.scripted_controller_zero = Some(sample);
            let before_effects = (stage == "bail_recovery")
                .then(|| session.controlled_wipeout_exit_effects_for_test());
            let outcome = session.try_tick(&mut pad, frame, &mut clamp)
                .unwrap_or_else(|error| panic!(
                    "mixed {stage} local={step} total={total_ticks}: {error:?}; last completed {trace:?}"
                ));
            assert!(matches!(
                outcome,
                SelectedQueuedFrameOutcome::Completed {
                    update_mask: 3,
                    missing_receivers: 0,
                    ..
                }
            ));
            total_ticks += 1;
            let physical = session.controlled_physical_state_for_test();
            let board = session.controlled_board_kinematics_for_test();
            let skater = session.controlled_skater_position_for_test();
            let turn = session
                .controlled_board_turn_for_test()
                .expect("completed board transform");
            assert!(board.0.iter().chain(skater.iter()).all(|v| v.is_finite()));
            assert!(turn.5.iter().all(|v| v.is_finite()));
            assert_eq!(
                session.try_visual_snapshot().unwrap().camera_update_counter,
                total_ticks + 1
            );
            peak_y = peak_y.max(board.0[1]);
            minimum_up = minimum_up.min(turn.5[5]);
            landed |= saw_air && physical == CurrentPhysicalState::Ground;
            saw_air |= physical == CurrentPhysicalState::Air;
            let intent: &[u8] = match stage {
                "push" | "recovered_push" => b"RightPush",
                "steer" => b"KickTurn",
                "crouch_ollie" => b"Ollie",
                "kickflip" => b"Kickflip",
                "grab" => b"LeftAirGrab",
                _ => b"WipeOutRequest",
            };
            let action = session.controlled_action_observation_for_test(intent);
            named |= action.1[1] == Some(1.0_f32.to_bits());
            let release_at = match stage {
                "push" | "recovered_push" => 44,
                "steer" => 26,
                "crouch_ollie" => 65,
                "kickflip" => 24,
                "grab" => 47,
                _ => 9,
            };
            released |= step >= release_at && named && action.1[1] != Some(1.0_f32.to_bits());
            if stage == "steer" {
                if matches!(step, 7 | 8 | 12 | 16 | 20 | 25 | 26 | 44) {
                    eprintln!(
                        "moving steering tick{step}: controls/deck={turn:?}; trucks={:?}",
                        session.controlled_truck_turn_for_test()
                    );
                }
                if step == 7 {
                    steer_start = Some(turn.5);
                }
                if step == 25 {
                    steer_end = Some(turn.5);
                    if steering_only {
                        let (_, _, parts) = session.controlled_truck_turn_for_test().unwrap();
                        for wheel in 0..4 {
                            let truck = 4 + wheel / 2;
                            let axle_alignment: f32 = (8..11)
                                .map(|lane| parts[wheel][lane] * parts[truck][lane])
                                .sum();
                            // Physical axle constraint, with solver tolerance;
                            // this is not a captured native numerical oracle.
                            assert!(
                                axle_alignment.abs() > 0.99,
                                "wheel{wheel} escaped truck{truck} axle: {axle_alignment}"
                            );
                        }
                    }
                }
                steering |= (8..26).contains(&step)
                    && [turn.0, turn.1, turn.2, turn.3]
                        .iter()
                        .any(|v| v.abs() > 0.001);
            }
            if stage == "grab" {
                let busy = session
                    .controlled_grab_busy_for_test()
                    .expect("retained grab owner");
                grab_busy |= (27..=46).contains(&step) && busy[1] > 0 && busy[0] == 0;
                grab_released |= step >= 47 && grab_busy && busy == [0, 0];
            }
            if stage == "bail_recovery" {
                wipeout |= physical == CurrentPhysicalState::WipeoutGround;
                if physical == CurrentPhysicalState::Teleporting && !teleport {
                    assert!(wipeout, "teleport must follow the requested wipeout");
                    exit_effects = Some((
                        before_effects.unwrap(),
                        session.controlled_wipeout_exit_effects_for_test(),
                    ));
                    teleport = true;
                }
                recovered |= teleport && physical == CurrentPhysicalState::Ground;
            }
            if trace.len() == 24 {
                trace.pop_front();
            }
            trace.push_back((step, physical, action.0, board, skater));
        }
        let board_end = session.controlled_board_kinematics_for_test();
        let skater_end = session.controlled_skater_position_for_test();
        let distance =
            |a: [f32; 3], b: [f32; 3]| ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        match stage {
            "push" | "recovered_push" => assert!(
                named
                    && released
                    && distance(board_start, board_end.0) > 0.3
                    && distance(skater_start, skater_end) > 0.3
                    && board_end.2 > 0.2,
                "{stage} did not move both physical owners after press/release: {trace:?}"
            ),
            "steer" => {
                let a = steer_start.unwrap();
                let b = steer_end.unwrap();
                if steering_only {
                    let heading_change = (a[8] * b[10] - a[10] * b[8])
                        .atan2(a[8] * b[8] + a[10] * b[10])
                        .abs();
                    assert!(
                        heading_change > 0.1,
                        "full steering changed lean but barely changed heading: {heading_change}"
                    );
                }
                let changed = [0, 1, 2, 4, 5, 6, 8, 9, 10]
                    .iter()
                    .any(|&i| (a[i] - b[i]).abs() > 0.1);
                assert!(
                    steering && changed,
                    "moving steering did not rotate board: {trace:?}"
                );
                assert_ne!(
                    session
                        .controlled_action_observation_for_test(b"KickTurn")
                        .0,
                    50,
                    "steering release left Kickturn active"
                );
            }
            "crouch_ollie" | "kickflip" | "grab" => {
                assert!(
                    named && released && saw_air && landed && peak_y - board_start[1] > 0.05,
                    "{stage}: named={named} released={released} air={saw_air} landed={landed} rise={}; {trace:?}",
                    peak_y - board_start[1]
                );
                if stage == "kickflip" {
                    assert!(
                        minimum_up < 0.0,
                        "board never inverted during flip: {trace:?}"
                    );
                }
                if stage == "grab" {
                    assert!(
                        grab_busy && grab_released,
                        "grab ownership did not activate/release: {trace:?}"
                    );
                }
            }
            "bail_recovery" => {
                assert!(
                    named && released && wipeout && teleport && recovered,
                    "explicit bail did not recover: {trace:?}"
                );
                let ((before_free, before_clear), (after_free, after_clear)) =
                    exit_effects.unwrap();
                assert!(before_free && !after_free);
                assert_eq!(after_clear, before_clear.wrapping_add(1));
            }
            _ => unreachable!(),
        }
        println!(
            "mixed {stage} completed {duration} ticks, total={total_ticks}, board={:?}, skater={skater_end:?}, rise={}",
            board_end,
            peak_y - board_start[1]
        );
    }
}
