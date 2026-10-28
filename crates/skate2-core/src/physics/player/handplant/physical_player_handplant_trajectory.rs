//! Embedded handplant trajectory82dbeed0 and separately allocated `SurfaceQuery`.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_footplant::{FootPlantPrediction, RetainedTrajectory},
    physical_player_grind_manager::types::GrindPrim,
    physical_player_trajectory_owners::RetainedPredictionFuture,
    simulation_assembly_backend::constraint_session::prediction::PhysicalPredictionOwner,
};
use std::sync::Arc;
#[path = "physical_player_handplant_predictions.rs"]
pub(crate) mod predictions;
pub(crate) const TRAJECTORY_CLASS: u64 = 0x2656_db32_bef0_b68d;

pub(crate) struct SurfaceQuery {
    pub(crate) predictor: Option<Arc<PhysicalPredictionOwner>>,
    pub(crate) query: FootPlantPrediction,
}
impl SurfaceQuery {
    /// `PhysicsSystems` allocation is not zero-filled. Only trajectory and
    /// readiness are written by ctor82dbee50 and Reset82dcd038.
    pub(crate) fn new(predictor: Option<Arc<PhysicalPredictionOwner>>) -> Self {
        Self {
            predictor,
            query: FootPlantPrediction {
                ready: false,
                trajectory: RetainedTrajectory::constructed(),
                collision_position: None,
                collision_normal: None,
                ground_normal: None,
                collision_time: None,
                frame: None,
                surface: None,
                radius: None,
                max_error_start: None,
                max_error_end: None,
                draw_true: None,
            },
        }
    }
    pub(crate) fn reset(&mut self) {
        self.query.ready = false;
    }
}
pub(crate) struct HandPlantTrajectoryOutResult {
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) landing_normal: [f32; 4],
    pub(crate) time_to_land: f32,
    pub(crate) result_valid: bool,
    pub(crate) surface_id: i32,
}
impl HandPlantTrajectoryOutResult {
    pub(crate) fn new_sim() -> Self {
        Self {
            trajectory: RetainedTrajectory::constructed(),
            landing_normal: [0.0; 4],
            time_to_land: 0.0,
            result_valid: false,
            surface_id: 0,
        }
    }
}
pub(crate) struct HandPlantTrajectorySelector {
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) queries: [RetainedPredictionFuture; 6],
    pub(crate) winning_result: Option<usize>,
    pub(crate) trajectory_out: RetainedTrajectory,
    pub(crate) trajectory_into: RetainedTrajectory,
    pub(crate) grind_primitive: GrindPrim,
    pub(crate) hand_position: [f32; 4],
    pub(crate) reflect_normal: [f32; 4],
    pub(crate) landing_normal: [f32; 4],
    pub(crate) landing_position: [f32; 4],
    pub(crate) surface_id: i32,
    pub(crate) time_to_land: f32,
    pub(crate) result: HandPlantTrajectoryOutResult,
    pub(crate) surface_query: Option<SurfaceQuery>,
    pub(crate) ready: bool,
}
impl HandPlantTrajectorySelector {
    pub(crate) fn new_sim(
        vault: &Arc<AttributeRuntimeVault>,
        allocate_surface: impl FnOnce() -> Option<SurfaceQuery>,
    ) -> Self {
        let attributes = vault.instance(TRAJECTORY_CLASS, super::DEFAULT_COLLECTION);
        let queries = std::array::from_fn(|_| RetainedPredictionFuture::in_sim_allocation());
        let trajectory_out = RetainedTrajectory::constructed();
        let trajectory_into = RetainedTrajectory::constructed();
        let result = HandPlantTrajectoryOutResult::new_sim();
        let surface_query = allocate_surface();
        let mut value = Self {
            attributes,
            queries,
            winning_result: None,
            trajectory_out,
            trajectory_into,
            grind_primitive: GrindPrim::default(),
            hand_position: [0.0; 4],
            reflect_normal: [0.0; 4],
            landing_normal: [0.0; 4],
            landing_position: [0.0; 4],
            surface_id: 0,
            time_to_land: 0.0,
            result,
            surface_query,
            ready: false,
        };
        value.reset();
        value
    }
    /// Reset82db5eb0 preserves futures/readiness, winning pointer and trajectories.
    pub(crate) fn reset(&mut self) {
        self.time_to_land = 0.0;
        self.surface_id = 0;
        self.ready = false;
        self.landing_normal = [0.0, 1.0, 0.0, 0.0];
        self.hand_position = [0.0; 4];
        self.reflect_normal = [0.0, 1.0, 0.0, 0.0];
        self.landing_position = [0.0; 4];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_reset_preserves_nullable_binding_and_unwritten_result_cells() {
        let mut value = SurfaceQuery::new(None);
        assert!(value.predictor.is_none());
        assert!(value.query.collision_position.is_none());
        value.query.ready = true;
        value.query.trajectory.position = [7.0; 4].into();
        value.query.surface = Some(29);
        value.reset();
        assert!(!value.query.ready);
        assert_eq!(
            value
                .query
                .trajectory
                .position
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [7.0_f32.to_bits(); 4]
        );
        assert_eq!(value.query.surface, Some(29));
        assert!(value.query.collision_position.is_none());
    }
}
