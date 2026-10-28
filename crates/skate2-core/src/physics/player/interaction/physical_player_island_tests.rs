use super::*;
use crate::physical_player_assembly::PhysicalAssemblyWorld;

#[expect(
    clippy::too_many_lines,
    reason = "controlled private corruptions are restored against one real mounted owner"
)]
pub(crate) fn check_registration_defenses(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    let before = world.backend.rigid_body_snapshot();
    let counter = world.physical_bodies.next_body_id;
    let member_index = world.physical_bodies.member_index(board).unwrap();
    let id = world.physical_bodies.members[member_index].body_id;
    let expected = world.physical_bodies.members[member_index].requested_state;
    assert!(
        matches!(world.try_add_physical_body_membership(board, expected ^ 1),
        Err(SimulationPhysicalBodyError::RequestedStateMismatch { expected: actual_expected, actual })
            if actual_expected == expected && actual == expected ^ 1)
    );
    assert_eq!(world.physical_bodies.next_body_id, counter);
    assert_eq!(world.physical_bodies.members[member_index].body_id, id);
    assert_eq!(world.backend.rigid_body_snapshot(), before);

    let observation = world
        .skaters
        .as_ref()
        .unwrap()
        .try_physical_part_observation(&world.backend, board, 0)
        .unwrap();
    // Only the private map is synthetically corrupted; both original and
    // replacement references below are actual admitted component handles.
    let original = world.physical_bodies.targets.remove(&observation).unwrap();
    assert!(world.physical_body_member(board).is_err());
    assert!(world.try_mark_physical_board_completed(board).is_err());
    world.physical_bodies.targets.insert(observation, original);

    let original_part = world.physical_bodies.targets[&observation]
        .physical()
        .unwrap()
        .part;
    world
        .physical_bodies
        .targets
        .get_mut(&observation)
        .unwrap()
        .physical_mut()
        .unwrap()
        .part = 1;
    assert!(matches!(
        world.physical_body_member(board),
        Err(SimulationPhysicalBodyError::TargetMismatch)
    ));
    assert!(matches!(
        world.try_mark_physical_board_completed(board),
        Err(SimulationPhysicalBodyError::TargetMismatch)
    ));
    world
        .physical_bodies
        .targets
        .get_mut(&observation)
        .unwrap()
        .physical_mut()
        .unwrap()
        .part = original_part;

    let other = world
        .physical_bodies
        .members
        .iter()
        .find(|member| !member.handle.same_as(board))
        .unwrap()
        .handle
        .clone();
    world
        .physical_bodies
        .targets
        .get_mut(&observation)
        .unwrap()
        .physical_mut()
        .unwrap()
        .member = other;
    assert!(world.physical_body_member(board).is_err());
    assert!(world.try_mark_physical_board_completed(board).is_err());
    world
        .physical_bodies
        .targets
        .get_mut(&observation)
        .unwrap()
        .physical_mut()
        .unwrap()
        .member = board.clone();

    let original_state = world
        .backend
        .collision_body_pool()
        .try_raw_state(observation)
        .unwrap();
    assert_eq!(original_state & !0xf, 0);
    world
        .skaters
        .as_ref()
        .unwrap()
        .try_remove_physical_part_target(&mut world.backend, board, 0)
        .unwrap();
    assert!(matches!(
        world.physical_body_target(observation),
        Err(SimulationPhysicalBodyError::TargetSpyBitMissing { part: 0, .. })
    ));
    assert!(matches!(
        world.physical_body_member(board),
        Err(SimulationPhysicalBodyError::TargetSpyBitMissing { part: 0, .. })
    ));
    assert!(matches!(
        world.try_mark_physical_board_completed(board),
        Err(SimulationPhysicalBodyError::TargetSpyBitMissing { part: 0, .. })
    ));
    world
        .skaters
        .as_ref()
        .unwrap()
        .try_register_physical_part(&mut world.backend, board, 0)
        .unwrap();
    assert_eq!(world.backend.rigid_body_snapshot(), before);
    assert!(
        world
            .physical_body_member(board)
            .unwrap()
            .registration_complete()
    );
    assert_eq!(world.physical_bodies.next_body_id, counter);
}

/// AFTER actual board drives have been released: exercise source cleanup
/// while real bodies remain live, then re-register them for ordinary teardown.
/// This extra cleanup/register cycle is a controlled diagnostic, not spawn.
pub(crate) fn check_cleanup_progress(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
) {
    let before = world.backend.rigid_body_snapshot();
    let registered = world
        .physical_body_member(board)
        .unwrap()
        .registered_parts();
    let total_targets = world.physical_bodies.target_count();
    let count = world
        .skaters
        .as_ref()
        .unwrap()
        .try_physical_part_count(&world.backend, board)
        .unwrap();
    assert_eq!(registered, count);
    world
        .try_prepare_physical_body_cleanup(board, SimulationPhysicalBodyKind::Board)
        .unwrap();
    let member = world.physical_body_member(board).unwrap();
    assert_eq!(member.registered_parts(), 0);
    assert_eq!(member.cleanup_completed_parts(), Some(count));
    assert!(!member.registration_complete());
    assert_eq!(world.physical_bodies.target_count(), total_targets - count);
    for part in 0..count {
        let observation = world
            .skaters
            .as_ref()
            .unwrap()
            .try_physical_part_observation(&world.backend, board, part)
            .unwrap();
        assert_eq!(
            world
                .backend
                .collision_body_pool()
                .try_raw_state(observation)
                .unwrap()
                & 8,
            0
        );
        assert!(world.physical_body_target(observation).unwrap().is_none());
    }
    world.try_register_physical_body_targets(board).unwrap();
    let member = world.physical_body_member(board).unwrap();
    assert_eq!(member.registered_parts(), count);
    assert_eq!(member.cleanup_completed_parts(), None);
    assert!(member.registration_complete());
    assert_eq!(world.physical_bodies.target_count(), total_targets);
    assert_eq!(world.backend.rigid_body_snapshot(), before);
}
