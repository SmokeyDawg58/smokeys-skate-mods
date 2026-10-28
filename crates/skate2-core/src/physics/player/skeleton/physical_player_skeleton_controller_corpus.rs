//! Actual retained Skeleton collision modes and same publication refresh.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::SkeletonCollisionTransition,
    skeleton_controller::SkeletonControllerOrigin,
};
use std::sync::Arc;

#[expect(
    clippy::too_many_lines,
    reason = "one actual owner through standard/world-only/restore and publication transitions"
)]
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let owner = Arc::clone(world.physical_body_member(skeleton).unwrap().owner());
    assert!(
        owner
            .skeleton_controller()
            .lock()
            .unwrap()
            .current()
            .is_none()
    );
    world
        .try_construct_pair_skeleton_controller(board, skeleton)
        .unwrap();
    let current = owner
        .skeleton_controller()
        .lock()
        .unwrap()
        .current()
        .unwrap();
    assert_eq!(current.value, 0);
    assert_eq!(current.origin, SkeletonControllerOrigin::Constructor);
    let geometry: Vec<_> = world
        .skeleton_publication(skeleton)
        .unwrap()
        .records()
        .iter()
        .map(|r| r.volume.clone())
        .collect();
    let before = world.backend().rigid_body_snapshot();
    let normal = world.skeleton(skeleton).unwrap();
    assert_eq!(normal.use_physics_truck_tilt(), Some(true));
    let prior_blends = normal
        .current_animation()
        .ik_runtime()
        .unwrap()
        .state()
        .on_deck_blends()
        .map(f32::to_bits);
    let prior_contacts = *normal
        .current_animation()
        .ik_runtime()
        .unwrap()
        .state()
        .contact_states();
    world
        .try_ground_entry_skeleton_ik_and_tilt(board, skeleton)
        .unwrap();
    let normal = world.skeleton(skeleton).unwrap();
    assert_eq!(normal.use_physics_truck_tilt(), Some(false));
    let ik = normal.current_animation().ik_runtime().unwrap().state();
    assert!(ik.do_ik());
    assert_eq!(ik.on_deck_blends().map(f32::to_bits), prior_blends);
    assert_eq!(*ik.contact_states(), prior_contacts);
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    for (transition, state, group, world_only) in [
        (SkeletonCollisionTransition::Set(6), 6, 6, false),
        (SkeletonCollisionTransition::StartWorldOnly, 2, 3, true),
        (SkeletonCollisionTransition::StopWorldOnly, 6, 6, false),
    ] {
        let frame = world
            .backend()
            .collision_body_pool()
            .frame_identity()
            .unwrap();
        assert!(
            world
                .skeleton_publication(skeleton)
                .unwrap()
                .has_bounds_for_frame(frame)
        );
        world
            .try_pair_skeleton_collision_transition(board, skeleton, transition)
            .unwrap();
        assert!(
            !world
                .skeleton_publication(skeleton)
                .unwrap()
                .has_bounds_for_frame(frame)
        );
        let controller = owner.skeleton_controller().lock().unwrap();
        assert_eq!(controller.current().unwrap().value, state);
        assert_eq!(controller.world_only().unwrap().value, world_only);
        drop(controller);
        let normal = world.skeleton(skeleton).unwrap();
        assert_eq!(normal.state().assembly_group, group);
        assert!(
            normal
                .state()
                .part_user_data
                .iter()
                .all(|&value| value == group)
        );
        assert_eq!(normal.state().volume_headers[0].flags & 1, 0);
        assert_eq!(normal.state().disable_counts[0], Some(0));
        assert!(!normal.post_physics_records().proxy_enabled());
        world.try_refresh_skaters().unwrap();
        let publication = world.skeleton_publication(skeleton).unwrap();
        assert!(publication.has_bounds_for_frame(frame));
        assert_eq!(publication.group(), group);
        assert!(
            publication
                .records()
                .iter()
                .all(|record| record.group == group)
        );
        assert_eq!(
            publication
                .records()
                .iter()
                .map(|r| r.volume.clone())
                .collect::<Vec<_>>(),
            geometry
        );
    }
    assert_eq!(world.backend().rigid_body_snapshot(), before);
    assert!(
        !owner
            .skeleton_controller()
            .lock()
            .unwrap()
            .stored_valid()
            .unwrap()
            .value
    );
    // Set(4) runs SetOffBoardCollision82b4f038 with the owner's default-true
    // OB skeleton cvars: limbs LightDMO, body Normal, core LW, and Parts 18/22
    // keep the group-7 userdata. Starting world-only from current=4 selects
    // mode1 on the same pair, which discriminates it from onboard mode2.
    world
        .try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(4),
        )
        .unwrap();
    {
        let normal = world.skeleton(skeleton).unwrap();
        let state = normal.state();
        assert_eq!(state.assembly_group, 7);
        let mut expected = [7; 24];
        for part in [17, 21, 15, 19, 16, 20] {
            expected[part] = 0x11;
        }
        for part in [5, 4, 3, 9, 8, 7, 2, 1] {
            expected[part] = 6;
        }
        for part in [6, 10, 11, 12, 13, 14, 23] {
            expected[part] = 0x12;
        }
        expected[0] = 6;
        assert_eq!(state.part_user_data, expected);
        assert_eq!(state.volume_headers[0].flags & 1, 1);
        assert_eq!(state.volume_headers[0].group_id, 0);
        assert!(normal.post_physics_records().proxy_enabled());
    }
    world
        .try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::StartWorldOnly,
        )
        .unwrap();
    let controller = owner.skeleton_controller().lock().unwrap();
    assert_eq!(controller.current().unwrap().value, 1);
    assert!(controller.world_only().unwrap().value);
    drop(controller);
    let normal = world.skeleton(skeleton).unwrap();
    assert_eq!(normal.state().assembly_group, 3);
    assert!(
        normal
            .state()
            .part_user_data
            .iter()
            .all(|&value| value == 3)
    );
    assert_eq!(normal.state().volume_headers[0].flags & 1, 1);
    assert_eq!(normal.state().volume_headers[0].group_id, 0);
    assert!(
        normal.state().volume_headers[1..]
            .iter()
            .all(|v| v.flags & 1 == 0)
    );
    assert!(
        normal
            .state()
            .disable_counts
            .iter()
            .all(|&count| count == Some(0))
    );
    assert!(normal.post_physics_records().proxy_enabled());
    println!(
        "actual SkeletonController constructor -> STANDARD6/group6 -> world-only2/group3 -> restored6/group6; same24 Parts/proxy/root counters/publication, stale until refresh, no rigid-body numeric changes; selected late stages, not full ground entry"
    );
}
