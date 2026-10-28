{
    // PC Object Dropper riderless board: a copy of the player's board dropped
    // ~1 m above the ground ahead of the settled skater lands on its wheels and
    // rests, the skater pushes into it and kicks it, it is removed, and the
    // skater keeps skating. `native`: Skate 2 physics, else Skate 3 mode.
    let native = matches!(trick_probe, Some(PlayerProbe::RiderlessBoard(true)));
    let mode = if native { "skate2" } else { "skate3" };
    let settings = if native {
        GameplaySettings { skate3: Default::default(), ..settings }
    } else {
        settings
    };
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.actor.try_set_pinned(false).unwrap();
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
        eprintln!("riderless {mode} island_flags={} run_skater_skater={}",
            frame.world.island_flags(), frame.cvars.try_run_skater_skater_collision_pipeline().unwrap());
    }).unwrap();
    let ground_height = |ground: &crate::static_ground_pool::StaticGroundPool, x: f32, y: f32, z: f32| {
        ground.trajectory_sweep_sphere_segment([x, y + 3.0, z], [x, y - 10.0, z], 0.0)
            .unwrap().map(|hit| hit.position[1])
    };
    let mut settled = 0;
    let mut board = None;
    let mut spawned_at = None;
    // (step, deck matrix) every frame after the spawn.
    let mut history: Vec<(usize, [[f32; 16]; 7])> = Vec::new();
    let mut rest = None;
    let mut kicked = 0.0_f32;
    let mut removed_at = None;
    let mut after_speed = 0.0_f32;
    let mut skater_start = None;
    let mut closest = f32::MAX;
    let mut stacked = None;
    // Largest truck-to-deck up-vector angle (degrees) after the board settles.
    let mut truck_tilt = 0.0_f32;
    // RIDERLESS_SHOVE: no skater kick; the resting board is shoved straight
    // (each speed along deck X, then deck Z), 120 frames each, re-placed between.
    let shove = std::env::var("RIDERLESS_SHOVE").is_ok() || std::env::var("RIDERLESS_ROLL").is_ok();
    let roll = std::env::var("RIDERLESS_ROLL").ok().and_then(|v| v.parse::<f32>().ok());
    let mut rest_tilt: Option<[f32; 2]> = None;
    let mut roll_dev = 0.0_f32;
    let shoves: Vec<(usize, f32)> = [0usize, 2].iter().flat_map(|&axis| [2.0_f32].map(|v| (axis, v))).collect();
    let mut shove_rest: Option<[f32; 16]> = None;
    for step in 0..1600 {
        if board.is_none() && removed_at.is_none() && settled >= 30 {
            spawned_at = Some(step);
            board = Some(session.try_with_selected_queued_frame(|frame, ground, _| {
                let root = {
                    let physical = frame.actor.cached_physical_allocation().unwrap();
                    let output = physical.owner().phys_out().lock().unwrap();
                    crate::ui::object_dropper::root_lanes(&output.try_skeleton_output().unwrap().skeleton_root_matrix)
                };
                let ahead = crate::ui::object_dropper::drop_transform(root);
                // Broadside across the skater's line: deck X along it, Z across.
                let mut transform = [
                    ahead[8], ahead[9], ahead[10], 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    -ahead[0], -ahead[1], -ahead[2], 0.0,
                    root[12] + ahead[8] * 3.0, 0.0, root[14] + ahead[10] * 3.0, 1.0,
                ];
                let surface = ground_height(ground, transform[12], root[13], transform[14])
                    .expect("no ground under the riderless board");
                transform[13] = surface + 1.0;
                if std::env::var("RIDERLESS_STACK").is_ok() {
                    // Onto the standing player's own deck, same orientation, 0.4 m up.
                    let player = frame.world.board(frame.board).unwrap()
                        .try_live_part_transform_host(frame.world.backend(), 6).unwrap();
                    transform = player;
                    transform[13] += 0.4;
                }
                let vault = Arc::clone(&frame.actor.cached_physical_allocation().unwrap().owner().inputs().vault);
                let count = frame.world.riderless_board_count();
                let handle = frame.world.try_spawn_riderless_board(&vault, transform).unwrap();
                assert_eq!(frame.world.riderless_board_count(), count + 1);
                eprintln!("riderless {mode} MASS player board inv_mass={:?} riderless inv_mass={:?}",
                    frame.world.board_inverse_masses(frame.board), frame.world.riderless_inverse_masses(&handle));
                let parts = frame.world.riderless_board_part_transforms(&handle).unwrap();
                eprintln!("riderless {mode} spawn step={step} deck={:?} surface={surface} skater={:?}",
                    &parts[6][12..15], &root[12..15]);
                assert!((parts[6][13] - transform[13]).abs() < 1e-3, "deck not placed at the request");
                if std::env::var("RIDERLESS_STACK2").is_ok() {
                    // A second board 0.35 m above the first, same orientation: board on board.
                    let mut top = transform;
                    top[13] += 0.35;
                    let second = frame.world.try_spawn_riderless_board(&vault, top).unwrap();
                    stacked = Some(second);
                }
                // Teleport round trip: same place, zero motion.
                frame.world.try_place_riderless_board(&handle, transform).unwrap();
                handle
            }).unwrap());
        }
        // RIDERLESS_LATE_KICK: the board settles fully (several seconds) first.
        let late = if std::env::var("RIDERLESS_LATE_KICK").is_ok() { 450 } else { 0 };
        let kick_from = spawned_at.map_or(usize::MAX, |at| at + 150 + late);
        let remove_at = spawned_at.map_or(usize::MAX, |at| at + 600 + late);
        if step == remove_at {
            let handle = board.take().unwrap();
            session.try_with_selected_queued_frame(|frame, _, _| {
                frame.world.try_remove_riderless_board(&handle).unwrap();
                assert_eq!(frame.world.riderless_board_count(), 0);
                assert!(frame.world.riderless_board_part_transforms(&handle).is_err());
            }).unwrap();
            removed_at = Some(step);
        }
        if shove && std::env::var("RIDERLESS_STACK2").is_err() && board.is_some() && step >= kick_from {
            let handle = board.as_ref().unwrap();
            let k = (step - kick_from) / 150;
            let at = (step - kick_from) % 150;
            session.try_with_selected_queued_frame(|frame, _, _| {
                let parts = frame.world.riderless_board_part_transforms(handle).unwrap();
                let rest = *shove_rest.get_or_insert(parts[6]);
                if k < shoves.len() {
                    let (axis, v) = shoves[k];
                    if at == 0 {
                        frame.world.try_place_riderless_board(handle, rest).unwrap();
                    } else if at == 30 && roll.is_some() {
                        // Roll the deck about its long axis (Z), as a rider landing on one side.
                        let w = roll.unwrap();
                        let a = &rest[8..11];
                        frame.world.try_spin_riderless_deck(handle, [a[0] * w, a[1] * w, a[2] * w]).unwrap();
                    } else if at == 30 {
                        let a = &rest[axis * 4..axis * 4 + 3];
                        frame.world.try_shove_riderless_board(handle, [a[0] * v, a[1] * v, a[2] * v]).unwrap();
                    } else if at == 149 || at == 45 || at == 90 {
                        let d = (parts[6][12] - rest[12]).hypot(parts[6][14] - rest[14]);
                        eprintln!("riderless {mode} SHOVE axis={} v={v} after {} frames: moved {d:.3} m",
                            if axis == 0 { "X" } else { "Z" }, at - 30);
                    }
                }
            }).unwrap();
        }
        let pushing = !shove && std::env::var("RIDERLESS_STACK").is_err() && (step >= kick_from && step < remove_at
            && [10, 40, 70, 100, 130].iter().any(|d| (kick_from + d..kick_from + d + 4).contains(&step)))
            || removed_at.is_some_and(|at| [20, 60, 100].iter().any(|d| (at + d..at + d + 4).contains(&step)));
        let sample = crate::input_pad::XboxPadSample { buttons: if pushing { 0x1000 } else { 0 }, ..neutral };
        pad.scripted_controller_zero = Some(sample);
        let input = session.bound_input.as_mut().unwrap();
        input.poll_and_cache(pad).unwrap();
        assert!(input.try_preupdate_enqueue(pad,
            &session.player.as_ref().unwrap().selected.owners.loader, true, true, &sync).unwrap());
        let raw = input.raw_controller();
        session.try_with_selected_queued_frame(|mut frame, ground, queued| {
            frame.tick.configure_skate3(settings, raw);
            // Any ground-frame error (the riderless odd-count case) fails here.
            frame.try_step_queued::<_, _, _, _, _, _, Infallible>(SelectedTickRequest {
                input: queued, dt: 1. / 60., lod: 0, wait_for_bs_job: true,
                physics: SkateboardGroundFrameInput {
                    schedule: SkateboardGroundFrameSchedule::SinglePlayerPhysicsFlags3,
                    ground_pool: ground, ground_group: 0, pipeline_culling: None,
                    allow_multiple_writes: true, spy_flags: 4,
                },
                pre_grind: GroundPreparationGrindPolicy::RunRecoveredPrefix,
                post_grind: GrindManagerFillPolicy::RequireNative,
            }, &mut numeric, &mut sync, &mut recognizer, &mut feedback, &mut clamp,
               &mut HostReckoningNumeric, None, None).unwrap_or_else(|error| panic!("riderless {mode} step {step}: {error:?}"));
            let (state, deck, speed) = {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let native = physical.owner().phys_out().lock().unwrap();
                (native.try_state_output().unwrap().physics_state,
                    native.try_camera_board_output().unwrap().deck_position.xyz,
                    native.try_system_centre_and_damped_velocity().unwrap().1.xyz)
            };
            let speed = speed[0].hypot(speed[2]);
            if spawned_at.is_none() {
                if state == 100 { settled += 1; } else { settled = 0; }
                return;
            }
            if let Some(handle) = board.as_ref() {
                let parts = frame.world.riderless_board_part_transforms(handle).unwrap();
                assert!(parts.iter().flatten().all(|v| v.is_finite()), "riderless {mode} board not finite at {step}");
                let d = &parts[6];
                closest = closest.min((deck[0] - d[12]).hypot(deck[2] - d[14]));
                if step % 15 == 0 || (step >= kick_from && step % 5 == 0 && kicked < 0.05) {
                    eprintln!("riderless {mode} step={step} state={state} skater_deck={deck:?} speed={speed:.2} board_deck={:?} up={:?} frozen={:?} {}",
                        &d[12..15], &d[4..7], frame.world.riderless_board_frozen(handle).ok(), frame.world.probe_riderless_deck(handle));
                }
                if step >= kick_from - 30 {
                    for truck in [4, 5] {
                        let dot = (0..3).map(|i| parts[truck][4 + i] * d[4 + i]).sum::<f32>().clamp(-1.0, 1.0);
                        let angle = dot.acos().to_degrees();
                        let base = rest_tilt.get_or_insert([angle, angle])[truck - 4];
                        roll_dev = roll_dev.max((angle - base).abs());
                        truck_tilt = truck_tilt.max(dot.acos().to_degrees());
                    }
                }
                if std::env::var("RIDERLESS_STACK").is_ok() && step % 10 == 0 {
                    let player = frame.world.board(frame.board).unwrap()
                        .try_live_part_transform_host(frame.world.backend(), 6).unwrap();
                    let lift = (0..3).map(|i| (d[12 + i] - player[12 + i]) * player[4 + i]).sum::<f32>();
                    let slide = ((d[12] - player[12]).powi(2) + (d[14] - player[14]).powi(2)).sqrt();
                    eprintln!("riderless {mode} STACK step={step} deck_above_player_deck={lift:.4} m slide={slide:.3} m up_y={:.3}", d[5]);
                }
                if let Some(top) = stacked.as_ref() {
                    if step % 10 == 0 {
                        let t = frame.world.riderless_board_part_transforms(top).unwrap();
                        let lift = (0..3).map(|i| (t[6][12 + i] - d[12 + i]) * d[4 + i]).sum::<f32>();
                        let slide = ((t[6][12] - d[12]).powi(2) + (t[6][14] - d[14]).powi(2)).sqrt();
                        // Each top wheel's centre height above the bottom deck's centre plane.
                        // (along bottom deck X, along Z (length), up) of each top wheel centre.
                        let wheels: Vec<String> = (0..4).map(|w| {
                            let local = |axis: usize| (0..3).map(|i| (t[w][12 + i] - d[12 + i]) * d[axis * 4 + i]).sum::<f32>();
                            format!("({:.3},{:.3},{:.3})", local(0), local(2), local(1))
                        }).collect();
                        let own: Vec<String> = (0..4).map(|w| {
                            let local = |axis: usize| (0..3).map(|i| (parts[w][12 + i] - d[12 + i]) * d[axis * 4 + i]).sum::<f32>();
                            format!("({:.3},{:.3},{:.3})", local(0), local(2), local(1))
                        }).collect();
                        eprintln!("riderless {mode} STACK2W bottom_own_wheels={own:?}");
                        eprintln!("riderless {mode} STACK2 step={step} top_deck_above_bottom_deck={lift:.4} slide={slide:.3} top_up_y={:.3} top_wheels_above_bottom_deck={wheels:?}", t[6][5]);
                    }
                }
                if step % 60 == 0 {
                    eprintln!("riderless {mode} VOLUMES step={step} state={state} player_board={} dropped_board={}",
                        frame.world.board_volume_enables(frame.board), frame.world.riderless_volume_enables(handle));
                }
                history.push((step, parts));
                if step == kick_from - 1 {
                    let surface: Vec<_> = (0..4).map(|w| {
                        let p = &parts[w];
                        (p[13], ground_height(ground, p[12], p[13] + 0.5, p[14]).expect("no ground under a wheel"))
                    }).collect();
                    rest = Some((parts, surface));
                    skater_start = Some(deck);
                }
                if let Some((rest, _)) = rest.as_ref() {
                    kicked = kicked.max((d[12] - rest[6][12]).hypot(d[14] - rest[6][14]));
                }
            } else if removed_at.is_some() {
                after_speed = after_speed.max(speed);
                if step % 15 == 0 {
                    eprintln!("riderless {mode} after removal step={step} state={state} speed={speed:.2}");
                }
            }
        }).unwrap();
    }
    let late_kick = if std::env::var("RIDERLESS_LATE_KICK").is_ok() { 450 } else { 0 };
    let spawned = spawned_at.expect("the skater never settled to drop the board");
    let (rest_parts, wheels) = rest.expect("the riderless board never reached its rest sample");
    // Rest: deck up, wheels on the ground, still over the last 30 frames before the kick.
    let deck = &rest_parts[6];
    let up = deck[5];
    let window: Vec<_> = history.iter().filter(|(s, _)| *s + 30 >= spawned + 150 + late_kick && *s < spawned + 150 + late_kick).collect();
    let motion = window.iter().map(|(_, p)| (p[6][12] - deck[12]).hypot(p[6][13] - deck[13]).hypot(p[6][14] - deck[14]))
        .fold(0.0_f32, f32::max);
    let clearances: Vec<f32> = wheels.iter().map(|(y, g)| y - g).collect();
    let deck_height = deck[13] - wheels.iter().map(|(_, g)| *g).sum::<f32>() / 4.0;
    eprintln!("riderless {mode} RESULT rest: up_y={up:.4} deck_above_ground={deck_height:.3} wheel_centre_clearance={clearances:?} motion_last_30={motion:.4}");
    // The resting board creeps down the slight slope; a kick is a speed far above that creep.
    let creep = motion / 0.5;
    let kick_speed = history.windows(2).filter(|pair| pair[0].0 + 1 >= spawned + 150 + late_kick)
        .map(|pair| (pair[1].1[6][12] - pair[0].1[6][12]).hypot(pair[1].1[6][14] - pair[0].1[6][14]) * 60.0)
        .fold(0.0_f32, f32::max);
    eprintln!("riderless {mode} RESULT kick: board moved {kicked:.3} m, peak speed {kick_speed:.2} m/s vs resting creep {creep:.3} m/s (skater start {skater_start:?}, closest skater-board {closest:.2} m)");
    eprintln!("riderless {mode} RESULT trucks: max truck tilt from the deck {truck_tilt:.2} deg, max change from rest {roll_dev:.2} deg");
    eprintln!("riderless {mode} RESULT after removal: max skater speed {after_speed:.2} m/s");
    assert!(up > 0.95, "riderless {mode} board did not land deck up: up_y={up}");
    for clearance in &clearances {
        assert!((-0.02..0.12).contains(clearance), "riderless {mode} wheel not on the ground: centre clearances {clearances:?}");
    }
    assert!(deck_height > 0.0 && deck_height < 0.3, "riderless {mode} deck height above ground {deck_height}");
    assert!(motion < 0.02, "riderless {mode} board not at rest: moved {motion} m in 30 frames");
    assert!(kicked > 0.1 && kick_speed > 0.3_f32.max(10.0 * creep),
        "riderless {mode} board was not kicked: moved {kicked} m, peak {kick_speed} m/s, creep {creep} m/s");
    assert!(after_speed > 0.5, "riderless {mode} skater stopped skating after the removal: {after_speed} m/s");
}
