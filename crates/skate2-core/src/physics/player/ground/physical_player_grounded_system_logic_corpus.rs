//! Measured original-terrain placement followed by genuine selected Update2.
//! Reuses the existing support fixture's rays/radii/SetTransform algorithm.
//! This is a controlled placement, not recovery of the native spawn scheduler.
use crate::physics_state_ids::PHYSICS_STATE_PHYSICS_GROUND;
use super::{PhysicalAssemblyWorld, SimulationSkaterAssemblyHandle};
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
    input_cvar_runtime::InputCvarReferences, skateboard_spawn::SkateboardSpawnServices,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM, skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

struct Placement;
impl SkateboardSpawnServices for Placement {
    fn current_orthogonal_tolerance(&mut self) -> f32 {
        0.01
    }
    fn orthogonality_assertion_failed(&mut self) {
        panic!("original terrain fixture requires orthogonal board pose");
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one measured terrain placement and actual state/request producer chain"
)]
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    hook: &SimulationHookAssemblyHandle,
    refs: &mut InputCvarReferences,
) {
    let owner = world
        .skaters
        .as_mut()
        .unwrap()
        .try_with_jump_pair(&world.backend, board, skeleton, |_, owner| {
            Arc::clone(owner)
        })
        .unwrap();
    assert!(!owner.wipeout().lock().unwrap().requests_initialized());
    assert_eq!(
        owner.physics_state().lock().unwrap().current,
        crate::physical_player_physics_state::CurrentPhysicalState::Ground
    );
    assert_eq!(
        owner
            .skeleton_controller()
            .lock()
            .unwrap()
            .current()
            .unwrap()
            .value,
        6
    );
    assert!(
        world
            .skeleton(skeleton)
            .unwrap()
            .current_animation()
            .ik_runtime()
            .unwrap()
            .state()
            .do_ik()
    );
    let ground = crate::simulation_contact_jacobian::ground_preparation::corpus::mounted_robs_park_ground_for_board_test();
    // The existing original-ground/contact corpora independently prove this
    // Rob's Park patch. Every wheel's actual supporting plane is queried below.
    let mut requested = FRESH_VOLUME_TRANSFORM;
    requested[12..15].copy_from_slice(&[588.8617, -54.431, 996.5048]);
    world
        .try_set_registered_board_transform(board, hook, requested, &mut Placement)
        .unwrap();
    let frames: [_; 5] = [0, 1, 2, 3, 6].map(|part| {
        world
            .board(board)
            .unwrap()
            .try_live_part_transform_host(world.backend(), part)
            .unwrap()
    });
    let mut lift = f32::NEG_INFINITY;
    for (part, frame) in frames.iter().enumerate().take(4) {
        let volume = world
            .board(board)
            .unwrap()
            .template()
            .collision()
            .wheels
            .parts()[part]
            .volume();
        assert_eq!(
            volume.volume_type(),
            Some(skate2_formats::collision::VolumeType::Sphere)
        );
        let radius = volume.radius;
        assert!(radius.is_finite() && radius > 0.0);
        let point = |y| SkeletonRootVector {
            xyz: [frame[12], y, frame[14]],
            w: None,
        };
        let hit = world
            .try_world_line_test(
                &ground,
                CollisionLineInput {
                    start: point(frame[13] + 2.0),
                    end: point(frame[13] - 2.0),
                    fatness: 0.0,
                },
                "original Ground Update2 support geometry",
                &mut HostCollisionLineNumeric,
            )
            .unwrap()
            .unwrap_or_else(|| panic!("original terrain unavailable below wheel {part}"));
        assert!(hit.normal.xyz[1] > 0.9);
        let desired_y = hit.position.xyz[1] + radius * 0.9 / hit.normal.xyz[1];
        let candidate = desired_y - frame[13];
        assert!(candidate.is_finite());
        lift = lift.max(candidate);
        println!(
            "Ground fixture wheel{part}: terrain={:?}, normal={:?}, radius={radius}, lift={candidate}",
            hit.position, hit.normal
        );
    }
    requested = frames[4];
    requested[13] += lift;
    world
        .try_set_registered_board_transform(board, hook, requested, &mut Placement)
        .unwrap();
    let up = owner.reckoning().lock().unwrap().reckoning().unwrap().up();
    world
        .try_start_board_wheel_lines(board, &ground, up, &mut HostCollisionLineNumeric)
        .unwrap();
    let hits = world.wheel_line_results_for_test(board);
    world
        .try_end_board_wheel_lines(board, &ground, &mut HostCollisionLineNumeric)
        .unwrap();
    let height = world
        .try_board_wheel_line_state::<std::convert::Infallible>(board)
        .unwrap()
        .lock()
        .unwrap()
        .height
        .unwrap();
    let before = owner.state_changer().lock().unwrap().fields.unwrap();
    let result = world.try_update_pair_system_logic(board, skeleton, refs);
    println!(
        "placed original Ground Update2: actual hits={:?}, height={height:?}, result={result:?}",
        hits.iter().map(Option::is_some).collect::<Vec<_>>()
    );
    assert!(hits[..4].iter().any(Option::is_some));
    result.expect("measured original geometry must support selected same-Ground path");
    let after = owner.state_changer().lock().unwrap().fields.unwrap();
    assert_eq!(after.current, Some(PHYSICS_STATE_PHYSICS_GROUND));
    assert_eq!(
        after.post_grind_jump_counter,
        before.post_grind_jump_counter.wrapping_add(1)
    );
    assert!(owner.wipeout().lock().unwrap().requests_initialized());
    assert_eq!(owner.wipeout().lock().unwrap().state_bits().unwrap().4, 0);
    assert_eq!(
        owner.physics_state().lock().unwrap().current,
        crate::physical_player_physics_state::CurrentPhysicalState::Ground
    );
}
