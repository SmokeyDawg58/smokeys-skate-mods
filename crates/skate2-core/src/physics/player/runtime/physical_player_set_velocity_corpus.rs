//! Actual all-Part setter followed by the next real original-ground solve.
use super::PhysicalAssemblyWorld;
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::PhysOutMotionVector,
    simulation_assembly_backend::constraint_session::{
        contact_feedback::ControlledNoFreezing, skateboard_ground::SkateboardGroundFrameInput,
    },
    static_ground_pool::StaticGroundPool,
};
pub(super) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    ground: &StaticGroundPool,
) {
    let velocity = world
        .try_with_pair_phys_out(board, skeleton, |output| {
            output
                .motion_vector(PhysOutMotionVector::DeckVelocity)
                .unwrap()
        })
        .unwrap();
    let count = world.board(board).unwrap().template().part_payloads().len();
    assert_eq!(count, 7);
    assert!(world.contact_spies().unwrap().is_some());
    world.check_late_board_velocity_failure(board);
    world
        .try_set_board_velocity(board, &velocity.value)
        .unwrap();
    for part in 0..count {
        let body = world
            .board(board)
            .unwrap()
            .try_part_contact_spy_observation(world.backend(), part)
            .unwrap();
        let actual = world
            .backend()
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .unwrap();
        assert_eq!(
            actual.map(f32::to_bits),
            velocity.value.xyz.map(f32::to_bits)
        );
    }
    assert!(world.contact_spies().is_err());
    assert!(world.try_sample_pair_board_motion(board, skeleton).is_err());
    assert!(
        world
            .try_publish_pair_skeleton_com(board, skeleton)
            .is_err()
    );
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            assert_eq!(
                output
                    .motion_vector(PhysOutMotionVector::DeckVelocity)
                    .unwrap(),
                velocity
            );
        })
        .unwrap();
    world
        .try_step_authored_ground(
            SkateboardGroundFrameInput {
                schedule: crate::simulation_assembly_backend::constraint_session::skateboard_ground::SkateboardGroundFrameSchedule::IsolatedCollisionComponent,
                ground_pool: ground,
                ground_group: 0,
                pipeline_culling: None,
                allow_multiple_writes: true,
                spy_flags: 4,
            },
            f32::from_bits(0x3c88_8889),
            50,
            &mut ControlledNoFreezing,
        )
        .unwrap();
    assert!(world.contact_spies().unwrap().is_some());
    world.try_sample_pair_board_motion(board, skeleton).unwrap();
    println!(
        "actual SetVelocity writes all7 board bodies, preserves historical output, rejects stale spies/motion/COM, and next original-ground solve restores current motion; supplied previous velocity is diagnostic, not active jump event"
    );
}
