//! Selected actual motion sample/publication and SAME next-Update1 prior copy.
//! Earlier Fill stages and the prior-copy gaps remain explicit diagnostic scope.
use super::{PhysicalAssemblyWorld, phys_out_corpus::Gaps};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::{PhysOutMotionScalar, PhysOutMotionVector, PhysOutOrigin},
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
};

pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    processed: &mut SkeletonProcessedContact,
) {
    let body = world
        .board(board)
        .unwrap()
        .try_part_contact_spy_observation(world.backend(), 6)
        .unwrap();
    let expected = world
        .backend()
        .collision_body_pool()
        .try_contact_spy_linear_velocity(body)
        .unwrap();
    let angular = world
        .backend()
        .collision_body_pool()
        .try_phys_out_angular_velocity(body)
        .unwrap();
    let sample = world.try_sample_pair_board_motion(board, skeleton).unwrap();
    world
        .try_publish_pair_board_motion(board, skeleton, sample, &mut HostSkeletonCollisionNumeric)
        .unwrap();
    world
        .try_with_pair_phys_out(board, skeleton, |output| {
            let velocity = output
                .motion_vector(PhysOutMotionVector::DeckVelocity)
                .unwrap();
            assert_eq!(velocity.origin, PhysOutOrigin::Publication);
            assert_eq!(
                velocity.value.xyz.map(f32::to_bits),
                expected.map(f32::to_bits)
            );
            assert_eq!(velocity.value.w, None);
            let omega = output
                .motion_vector(PhysOutMotionVector::AngularVelocity)
                .unwrap();
            assert_eq!(omega.value.xyz.map(f32::to_bits), angular.map(f32::to_bits));
            assert_eq!(omega.value.w, None);
            let speed = output
                .motion_scalar(PhysOutMotionScalar::DeckSpeed)
                .unwrap();
            let expected_speed = expected
                .into_iter()
                .map(|v| f64::from(v).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!((f64::from(speed.value) - expected_speed).abs() < 0.0001);
            processed
                .try_copy_prior_output(output, &mut Gaps::default())
                .unwrap();
            assert_eq!(processed.prior_output().velocity().unwrap(), velocity);
            assert_eq!(processed.prior_output().angular_velocity().unwrap(), omega);
            assert_eq!(processed.prior_output().deck_speed().unwrap(), speed);
        })
        .unwrap();
    println!(
        "actual Part6 velocity {expected:?} and angular velocity {angular:?} -> same pair motion publications -> same processed prior values; earlier Fill/Update1 gaps explicitly diagnostic, no full phase claim"
    );
}
