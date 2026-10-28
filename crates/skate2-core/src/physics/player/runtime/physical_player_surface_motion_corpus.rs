//! Actual retained pair state -> selected surface motion -> SAME next prior copy.
//! Toolkit flip flag and Reckoning state inputs remain explicit diagnostics.
use super::{PhysicalAssemblyWorld, phys_out_corpus::Gaps};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_phys_out::{PhysOutMotionScalar, PhysOutMotionVector, PhysOutOrigin},
    physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
    reckoning_numeric::HostReckoningNumeric,
    skeleton_collision_numeric::HostSkeletonCollisionNumeric,
    skeleton_processed_contact::{SkeletonProcessedContact, SkeletonProcessedContactBoard},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
struct Board<'a> {
    world: &'a PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
}
impl SkeletonProcessedContactBoard for Board<'_> {
    type Error = String;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        crate::skateboard_body_mass::try_skateboard_body_mass(
            self.world.board(self.board).map_err(|e| e.to_string())?,
            self.world.backend(),
        )
        .map_err(|e| format!("actual body mass: {e:?}"))
    }
    fn deck_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let matrix = self
            .world
            .board(self.board)
            .map_err(|e| e.to_string())?
            .try_live_part_transform_host(self.world.backend(), 6)
            .map_err(|e| e.to_string())?;
        // The actual mapped Part transform supplies XYZ; imported native W is
        // conservatively unknown, not inferred from the host matrix padding.
        Ok(SkeletonRootMatrix::from_columns(std::array::from_fn(
            |column| SkeletonRootVector {
                xyz: [
                    matrix[column * 4],
                    matrix[column * 4 + 1],
                    matrix[column * 4 + 2],
                ],
                w: None,
            },
        )))
    }
    fn flags_8c4(&self) -> u32 {
        0 // Explicit controlled BoardFlipped=false; current flag producer remains open.
    }
}
/// Sample actual raw/effective deck prefix before the support fixture's next solve.
pub(crate) fn prepare(
    world: &PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    processed: &mut SkeletonProcessedContact,
) {
    processed
        .try_sample_toolkit_prefix(&mut Board { world, board })
        .unwrap();
}
pub(crate) fn check(
    world: &mut PhysicalAssemblyWorld,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    phase: &PhysicalPlayerReckoningPhase,
) {
    let owner = std::sync::Arc::clone(world.physical_body_member(board).unwrap().owner());
    let ground = owner
        .reckoning()
        .lock()
        .unwrap()
        .try_completed(phase)
        .unwrap()
        .ground_normal();
    let sample = world.try_sample_pair_board_motion(board, skeleton).unwrap();
    world
        .try_publish_pair_board_motion(board, skeleton, sample, &mut HostSkeletonCollisionNumeric)
        .unwrap();
    world
        .try_publish_pair_board_surface_motion(board, skeleton, phase, &mut HostReckoningNumeric)
        .unwrap();
    let processed_owner = std::sync::Arc::clone(owner.reckoning().lock().unwrap().processed());
    let mut processed = processed_owner.lock().unwrap();
    world.try_with_pair_phys_out(board, skeleton, |output| {
        let velocity = output.motion_vector(PhysOutMotionVector::DeckVelocity).unwrap();
        let surface = output.motion_vector(PhysOutMotionVector::SurfaceVelocity).unwrap();
        let speed = output.motion_scalar(PhysOutMotionScalar::SurfaceSpeed).unwrap();
        let angle = output.motion_scalar(PhysOutMotionScalar::SurfaceAngle).unwrap();
        assert_eq!(surface.origin, PhysOutOrigin::Publication);
        assert_eq!(speed.origin, PhysOutOrigin::Publication);
        assert_eq!(angle.origin, PhysOutOrigin::Publication);
        assert_eq!(surface.value.w, None);
        let dot: f64 = (0..3).map(|i| f64::from(velocity.value.xyz[i]) * f64::from(ground.xyz[i])).sum();
        let expected: [f64; 3] = std::array::from_fn(|i| f64::from(velocity.value.xyz[i]) - f64::from(ground.xyz[i]) * dot);
        for (actual, expected) in surface.value.xyz.into_iter().zip(expected) {
            assert!((f64::from(actual) - expected).abs() < 0.0001);
        }
        let length = expected.into_iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!((f64::from(speed.value) - length).abs() < 0.0001);
        assert!(angle.value.is_finite());
        processed.try_copy_prior_output(output, &mut Gaps::default()).unwrap();
        assert_eq!(processed.prior_output().surface_velocity().unwrap(), surface);
        assert_eq!(processed.prior_output().surface_speed().unwrap(), speed);
        assert!(processed.completed_deck_transform().is_none(), "next prior copy invalidates old toolkit readiness");
        println!("same actual Reckoning normal {ground:?}, retained pre-solve raw deck and published velocity {:?} -> surface {:?}, speed {}, angle {} -> SAME processed prior records; controlled Reckoning/flip inputs, no full Fill/Actor claim", velocity.value, surface.value, speed.value, angle.value);
    }).unwrap();
    drop(processed);
    super::prepare_corpus::check(world, board, skeleton);
    super::ground_position_corpus::check(world, board, phase);
}
