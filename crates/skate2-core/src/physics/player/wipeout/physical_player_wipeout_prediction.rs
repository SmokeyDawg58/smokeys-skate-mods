//! WipeoutGround.Update82d9b31c..b3d0, using its retained future and World queue.
use crate::{
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_wipeout_state::WipeoutGroundFields,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::prediction::runtime::{
        PredictionRuntimeError, future::PredictionTarget,
    },
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

/// `PredictionResults.TimeToCollision`82d83ca0 checks abs(X)<100 before return.
pub(crate) fn try_consume_wipeout_prediction<N>(
    fields: &mut WipeoutGroundFields,
) -> Result<(), PredictionRuntimeError<N>> {
    use PredictionRuntimeError as E;
    if !fields.prediction.ready {
        return Ok(());
    }
    let time = fields
        .prediction
        .collision_time
        .ok_or(E::Missing("ready Wipeout collision time"))?;
    if time.iter().all(|v| *v >= 0.0) {
        if time[0].abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
            return Err(E::Missing("TimeToCollision abs(X)<100"));
        }
        fields.estimated_time_to_collision = time[0];
    }
    Ok(())
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_wipeout_prediction<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<(), PredictionRuntimeError<N::Error>> {
        use PredictionRuntimeError as E;
        {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let wipeout = state
                .wipeout_ground
                .as_mut()
                .ok_or(E::Missing("WipeoutGround"))?;
            try_consume_wipeout_prediction(&mut wipeout.fields)?;
        }
        // No state guard survives submission into the very same future.
        let predictor = self
            .predictor
            .value
            .as_ref()
            .ok_or(E::Missing("World predictor"))?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let input = {
            let p = processed.lock().map_err(|_| E::Poisoned)?;
            BallisticTrajectoryInput {
                position: p
                    .prior_output()
                    .centre_of_mass()
                    .ok_or(E::Missing("Processed COM250"))?
                    .value,
                velocity: p
                    .prior_output()
                    .centre_of_mass_velocity()
                    .ok_or(E::Missing("Processed velocity260"))?
                    .value,
                gravity: SkeletonRootVector::known(predictor.gravity),
                radius: f32::from_bits(0x3e99_999a),
                duration: 2.0,
                maximum_error_start: 1.0,
                maximum_error_end: 1.0,
            }
        };
        predictor.try_add_query(
            owner,
            PredictionTarget::WipeoutGround,
            input,
            false,
            self.world_line_queries
                .as_mut()
                .ok_or(E::Missing("World queries"))?,
            ground,
            self.dmos.as_ref().ok_or(E::Missing("DMO collection"))?,
            numeric,
        )
    }
}
