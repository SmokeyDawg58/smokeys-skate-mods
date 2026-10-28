{
    // PC Object Dropper: every offered prop spawns as a static body and goes again, and a
    // kicker dropped ahead of the source skater is ridden up.
    session.try_with_selected_queued_frame(|frame, _, _| {
        frame.actor.try_set_pinned(false).unwrap();
        frame.world.local_skater_identity.as_ref().unwrap().backend().finish_loading().unwrap();
    }).unwrap();
    let props = crate::ui::object_dropper::PROPS;
    let mut failed = Vec::new();
    for (index, &(_, name, guid)) in props.iter().enumerate() {
        // A copy of the player's board is a riderless board, not a bank prop (its own test).
        if guid == crate::ui::object_dropper::YOUR_BOARD { continue; }
        let mut at = test_spawn;
        at[12] += 40.0 + 4.0 * index as f32;
        // As the game drops one: on a locator of its own, object spawned on it, then removed.
        let locator = 0x5043_4452_0000_0000 + index as u64;
        let (added, user, kept, gone) = session.try_with_selected_queued_frame(|frame, _, _| {
            let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
            let added = living.try_drop_on_locator(frame.world, frame.actor, frame.cvars, recipes, locator, guid, &at).unwrap();
            let (user, kept) = {
                let owner = std::sync::Arc::clone(frame.world.grind_services.living_world());
                let living = owner.lock().unwrap();
                let found = living.dmo_locators.as_deref().unwrap().locator(locator);
                (found.map_or(0, |l| l.user_object_id), found.map(|l| l.transform))
            };
            living.try_remove_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, &[locator]).unwrap();
            let gone = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(frame.world, &[locator]).unwrap()[0].is_none();
            (added, user, kept, gone)
        }).unwrap();
        eprintln!("dropper prop {name} {guid:016x} -> locator added={added} object={user} at={:?} removed={gone}", kept.map(|t| [t[12], t[13], t[14]]));
        if !added || user == 0 || !gone { failed.push(name); }
    }
    assert!(failed.is_empty(), "props that would not drop: {failed:?}");

    // Push test, then ride test: settle, drop a light cardboard box on the skater's line and push
    // into it (it must move: dropped props are dynamic bodies), take it away, then drop a large
    // kicker ahead and push into it (the board must roll up it). DROPPER_NATIVE: Skate 2 physics.
    let settings = if std::env::var_os("DROPPER_NATIVE").is_some() {
        GameplaySettings { skate3: Default::default(), ..settings }
    } else {
        settings
    };
    let kicker = props.iter().find(|(_, name, _)| *name == "Kicker (large)").unwrap().2;
    let mut start_y = None;
    let mut highest = f32::MIN;
    let mut kicker_id = None;
    let mut settled = 0;
    let mut dropped_at = None;
    let skate3 = std::env::var_os("DROPPER_NATIVE").is_none();
    // (props held as Skate 3 ground, Skate 3 grind primitives) after the kicker, and around a rail drop.
    let mut kicker_counts = None;
    let mut rail_counts = (None, None);
    let mut rail_segments = (Vec::new(), Vec::new());
    let mut rail_transform = None;
    let light = props.iter().find(|(_, name, _)| *name == "Cardboard box").unwrap().2;
    const LIGHT_LOCATOR: u64 = 0x5043_4452_0000_1001;
    const LIGHT_PHASE: usize = 240;
    let mut light_at: Option<usize> = None;
    let mut light_start: Option<[f32; 16]> = None;
    let mut light_moved = 0.0_f32;
    let mut light_turned = 0.0_f32;
    // Seat a drop `ahead` metres along the skater's heading (and `lateral` off it) on the ground.
    let place = |frame: &mut crate::selected_session_frame::SelectedSessionFrame<'_>,
                 ground: &crate::static_ground_pool::StaticGroundPool, ahead: f32, lateral: f32| {
        let root = {
            let physical = frame.actor.cached_physical_allocation().unwrap();
            let output = physical.owner().phys_out().lock().unwrap();
            crate::ui::object_dropper::root_lanes(&output.try_skeleton_output().unwrap().skeleton_root_matrix)
        };
        let mut transform = crate::ui::object_dropper::drop_transform(root);
        transform[12] += transform[8] * ahead - transform[0] * lateral;
        transform[14] += transform[10] * ahead - transform[2] * lateral;
        if let Some(on_ground) = crate::skater_manager_adjust_spawn_position::try_adjust_spawn_position(
            frame.world, ground, frame.cvars, [transform[12], transform[13], transform[14], 1.0]).unwrap() {
            transform[12..15].copy_from_slice(&on_ground[..3]);
        }
        if let Some(player) = frame.actor.cached_physical_allocation().unwrap().owner().skate3.lock().unwrap().as_ref() {
            transform[13] = player.test_ground_height([transform[12], root[13] + 1.0, transform[14]]).unwrap();
        }
        (transform, root)
    };
    for step in 0..1200 {
        if light_at.is_none() && settled >= 30 {
            light_at = Some(step);
            light_start = session.try_with_selected_queued_frame(|mut frame, ground, _| {
                let (transform, root) = place(&mut frame, ground, 0.0, 0.0);
                eprintln!("dropper light prop at {:?} skater root {:?}", &transform[12..15], &root[12..15]);
                let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
                assert!(living.try_drop_on_locator(frame.world, frame.actor, frame.cvars, recipes, LIGHT_LOCATOR, light, &transform).unwrap(),
                    "the light prop would not drop");
                crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(frame.world, &[LIGHT_LOCATOR]).unwrap()[0]
            }).unwrap();
            assert!(light_start.is_some(), "the light prop has no locator");
        }
        if light_at.is_some_and(|at| step > at && step <= at + LIGHT_PHASE) {
            let start = light_start.unwrap();
            let now = session.try_with_selected_queued_frame(|frame, _, _| {
                crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(frame.world, &[LIGHT_LOCATOR]).unwrap()[0]
            }).unwrap().expect("the light prop vanished");
            light_moved = light_moved.max(((now[12] - start[12]).powi(2) + (now[13] - start[13]).powi(2) + (now[14] - start[14]).powi(2)).sqrt());
            light_turned = light_turned.max((0..3).map(|c| 1.0 - (now[c * 4] * start[c * 4] + now[c * 4 + 1] * start[c * 4 + 1] + now[c * 4 + 2] * start[c * 4 + 2])).fold(0.0, f32::max));
            if step % 15 == 0 { eprintln!("dropper light prop step={step} at {:?} moved {light_moved:.3} m", &now[12..15]); }
            if light_at.is_some_and(|at| step == at + LIGHT_PHASE) {
                // Out of the way before the kicker run.
                session.try_with_selected_queued_frame(|frame, _, _| {
                    let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
                    living.try_remove_dropped_locators(frame.world, frame.actor, frame.cvars, recipes, &[LIGHT_LOCATOR]).unwrap();
                }).unwrap();
            }
        }
        if skate3 && dropped_at.is_some_and(|at| [at + 5, at + 430, at + 440].contains(&step)) {
            let counts = session.try_with_selected_queued_frame(|frame, _, _| {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let player = physical.owner().skate3.lock().unwrap();
                player.as_ref().map(|player| (player.test_prop_world_counts(), player.test_grind_segments()))
            }).unwrap();
            let (counts, segments) = counts.map_or((None, Vec::new()), |(counts, segments)| (Some(counts), segments));
            eprintln!("dropper skate3 prop world step={step} (ground props, grind primitives)={counts:?}");
            let at = dropped_at.unwrap();
            if step == at + 5 { kicker_counts = counts; }
            if step == at + 430 { rail_counts.0 = counts; rail_segments.0 = segments; }
            else if step == at + 440 { rail_counts.1 = counts; rail_segments.1 = segments; }
        }
        if skate3 && dropped_at.is_some_and(|at| step == at + 431) {
            let rail = props.iter().find(|(_, name, _)| *name == "Rail (long, low)").unwrap().2;
            let id = session.try_with_selected_queued_frame(|frame, ground, _| {
                let root = {
                    let physical = frame.actor.cached_physical_allocation().unwrap();
                    let output = physical.owner().phys_out().lock().unwrap();
                    crate::ui::object_dropper::root_lanes(&output.try_skeleton_output().unwrap().skeleton_root_matrix)
                };
                let mut transform = crate::ui::object_dropper::drop_transform(root);
                transform[12] += transform[8] * 6.0;
                transform[14] += transform[10] * 6.0;
                if let Some(on_ground) = crate::skater_manager_adjust_spawn_position::try_adjust_spawn_position(
                    frame.world, ground, frame.cvars, [transform[12], transform[13], transform[14], 1.0]).unwrap() {
                    transform[12..15].copy_from_slice(&on_ground[..3]);
                }
                // The spawn adjustment is a skater's standing height, above the ground. Seat the
                // prop on the actual collision surface (Skate 3's world holds the same terrain).
                if let Some(player) = frame.actor.cached_physical_allocation().unwrap().owner().skate3.lock().unwrap().as_ref() {
                    let surface = player.test_ground_height([transform[12], root[13] + 1.0, transform[14]]).unwrap();
                    eprintln!("dropper prop ground {surface} (spawn adjustment {})", transform[13]);
                    transform[13] = surface;
                }
                let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
                (living.try_spawn_dropped(frame.world, frame.actor, frame.cvars, recipes, rail, &transform, false).unwrap(), transform)
            }).unwrap();
            assert!(id.0.is_some(), "rail would not drop");
            rail_transform = Some(id.1);
        }
        if dropped_at.is_none() && light_at.is_some_and(|at| step == at + LIGHT_PHASE + 1) {
            dropped_at = Some(step);
            kicker_id = session.try_with_selected_queued_frame(|frame, ground, _| {
                let root = {
                    let physical = frame.actor.cached_physical_allocation().unwrap();
                    let output = physical.owner().phys_out().lock().unwrap();
                    crate::ui::object_dropper::root_lanes(&output.try_skeleton_output().unwrap().skeleton_root_matrix)
                };
                let mut transform = crate::ui::object_dropper::drop_transform(root);
                // Far enough ahead for twelve pushes: the 1.1 m kicker needs ~2.5 m/s at its toe to rise 0.3 m.
                transform[12] += transform[8] * 7.0;
                transform[14] += transform[10] * 7.0;
                // The large kicker's model origin is its side edge: its ramp spans local x 0..1.86 m.
                // Centre it on the skater's line so the board meets the ramp, not the side panel.
                let lateral = std::env::var("DROPPER_LATERAL").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.93);
                transform[12] -= transform[0] * lateral;
                transform[14] -= transform[2] * lateral;
                if std::env::var_os("DROPPER_FLIP").is_some() { for i in [0, 2, 8, 10] { transform[i] = -transform[i]; } }
                if let Some(on_ground) = crate::skater_manager_adjust_spawn_position::try_adjust_spawn_position(
                    frame.world, ground, frame.cvars, [transform[12], transform[13], transform[14], 1.0]).unwrap() {
                    transform[12..15].copy_from_slice(&on_ground[..3]);
                }
                // The spawn adjustment is a skater's standing height, above the ground. Seat the
                // prop on the actual collision surface (Skate 3's world holds the same terrain).
                if let Some(player) = frame.actor.cached_physical_allocation().unwrap().owner().skate3.lock().unwrap().as_ref() {
                    let surface = player.test_ground_height([transform[12], root[13] + 1.0, transform[14]]).unwrap();
                    eprintln!("dropper prop ground {surface} (spawn adjustment {})", transform[13]);
                    transform[13] = surface;
                }
                eprintln!("dropper kicker at {:?} skater root {:?}", &transform[12..15], &root[12..15]);
                let (living, recipes) = (frame.living_world.unwrap(), frame.living_world_recipes.as_ref().unwrap());
                // As the game drops it: an ordinary object on a dropper locator of its own.
                living.try_drop_on_locator(frame.world, frame.actor, frame.cvars, recipes, 0x5043_4452_0000_1000, kicker, &transform)
                    .unwrap().then_some(0x5043_4452_0000_1000_u64)
            }).unwrap();
            assert!(kicker_id.is_some(), "kicker would not drop");
        }
        let sample = crate::input_pad::XboxPadSample {
            buttons: if dropped_at.is_some_and(|at| [10, 35, 60, 85, 110, 135, 160, 185, 210, 235, 260, 285].iter().any(|d| (at + d..at + d + 4).contains(&step)))
                || (dropped_at.is_none() && light_at.is_some_and(|at| [10, 35, 60].iter().any(|d| (at + d..at + d + 4).contains(&step)))) { 0x1000 } else { 0 },
            ..neutral
        };
        pad.scripted_controller_zero = Some(sample);
        let input = session.bound_input.as_mut().unwrap();
        input.poll_and_cache(pad).unwrap();
        assert!(input.try_preupdate_enqueue(pad,
            &session.player.as_ref().unwrap().selected.owners.loader, true, true, &sync).unwrap());
        let raw = input.raw_controller();
        session.try_with_selected_queued_frame(|mut frame, ground, queued| {
            frame.tick.configure_skate3(settings, raw);
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
               &mut HostReckoningNumeric, None, None).unwrap();
            let (state, board, speed) = {
                let physical = frame.actor.cached_physical_allocation().unwrap();
                let native = physical.owner().phys_out().lock().unwrap();
                let state = native.try_state_output().unwrap().physics_state;
                let deck = native.try_camera_board_output().unwrap().deck_position.xyz;
                (state, deck, native.try_system_centre_and_damped_velocity().unwrap().1.xyz)
            };
            let y = board[1];
            if state == 100 && dropped_at.is_none() { settled += 1; } else if dropped_at.is_none() { settled = 0; }
            if dropped_at.is_some() && start_y.is_none() { start_y = Some(y); }
            if dropped_at.is_some() && state != 300 { highest = highest.max(y); }
            if dropped_at.is_some_and(|at| step == at + 420) {
                // The kicker is a dynamic body too: report how far riding up it pushed it.
                let kicker = crate::selected_living_world_dmo::SelectedLivingWorldDmo::dropped_locator_transforms(
                    frame.world, &[0x5043_4452_0000_1000]).unwrap()[0];
                eprintln!("dropper kicker after the ride at {:?}", kicker.map(|t| [t[12], t[13], t[14]]));
            }
            if dropped_at.is_some_and(|at| step > at + 420) { return; }
            if step % 15 == 0 {
                eprintln!("dropper ride step={step} state={state} board={board:?} speed={:.2}", speed[0].hypot(speed[2]));
            }
        }).unwrap();
    }
    let rise = highest - start_y.unwrap();
    eprintln!("dropper push RESULT light prop moved {light_moved:.3} m, turned {light_turned:.4} (1-cos)");
    eprintln!("dropper ride RESULT board rose {rise:.3} m over the kicker");
    if skate3 {
        eprintln!("dropper skate3 RESULT kicker {kicker_counts:?} rail before/after {rail_counts:?}");
        assert!(kicker_counts.is_some_and(|(props, _)| props >= 1), "the kicker is not a Skate 3 mesh body: {kicker_counts:?}");
        assert!(light_moved > 0.05 || light_turned > 0.01,
            "the board ran into the light prop but it did not move: {light_moved:.3} m, turned {light_turned:.4}");
        let (Some(before), Some(after)) = rail_counts else { panic!("Skate 3 player absent around the rail drop") };
        assert!(after.1 > before.1, "the dropped rail added no Skate 3 grind primitives: {rail_counts:?}");
        // Every added primitive lies on the rail in its own frame (its mesh spans local
        // x +-0.52, y 0..0.75, z -7.65..1.29), and some run along its top bar.
        let t = rail_transform.unwrap();
        let local = |p: [f32; 4]| -> [f32; 3] {
            let d = [p[0] - t[12], p[1] - t[13], p[2] - t[14]];
            std::array::from_fn(|c| d[0] * t[c * 4] + d[1] * t[c * 4 + 1] + d[2] * t[c * 4 + 2])
        };
        let bits = |(a, b): &([f32; 4], [f32; 4])| (a.map(f32::to_bits), b.map(f32::to_bits));
        let added: Vec<_> = rail_segments.1.iter().filter(|s| !rail_segments.0.iter().any(|o| bits(o) == bits(s))).collect();
        assert_eq!(added.len(), after.1 - before.1, "the drop changed existing grind primitives");
        let mut along_top = 0;
        for (start, end) in &added {
            let (a, b) = (local(*start), local(*end));
            eprintln!("dropper rail grind segment local {a:?} -> {b:?}");
            for p in [a, b] {
                assert!(p[0].abs() < 0.6 && (-0.05..0.8).contains(&p[1]) && (-7.8..1.4).contains(&p[2]),
                    "a dropped rail grind segment is off the rail: local {a:?} -> {b:?}");
            }
            if [a, b].iter().all(|p| (0.6..0.8).contains(&p[1])) && (a[2] - b[2]).abs() > 0.5 {
                along_top += 1;
            }
        }
        assert!(along_top > 0, "no dropped rail grind segment runs along its top bar");
    }
    assert!(rise > 0.3, "the board never rode up the dropped kicker: rose {rise:.3} m");
}
