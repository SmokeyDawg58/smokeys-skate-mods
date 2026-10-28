//! Retained `HandPlantManager` constructor82dc1748 and live launch/trajectory/IK
//! on the actual Sim allocation; no second handplant owner.
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    physical_player_footplant::RetainedTrajectory,
    physical_player_grind_manager::types::GrindPrim,
};
use std::sync::Arc;

#[path = "physical_player_handplant_trajectory.rs"]
pub(crate) mod trajectory;
use trajectory::{HandPlantTrajectoryOutResult, HandPlantTrajectorySelector, SurfaceQuery};
#[path = "physical_player_handplant_geometry.rs"]
pub(crate) mod geometry;
#[path = "physical_player_handplant_ik.rs"]
pub(crate) mod ik;
#[path = "physical_player_handplant_manager.rs"]
pub(crate) mod manager;
#[path = "physical_player_handplant_position.rs"]
pub(crate) mod position;
#[path = "physical_player_handplant_values.rs"]
pub(crate) mod values;

pub(crate) const POSITION_CLASS: u64 = 0x55b5_9400_75f7_8842;
pub(crate) const MANAGER_CLASS: u64 = 0x0f38_0840_37cb_9175;
pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

pub(crate) struct HandPlantPositionResult {
    pub(crate) found: bool,
    pub(crate) position: [f32; 4],
    pub(crate) direction_vs_up: i32,
    pub(crate) primitive: GrindPrim,
}
pub(crate) struct HandPlantIkData {
    pub(crate) blend: f32,
    pub(crate) distance: [f32; 4],
    pub(crate) flags: u32,
}
pub(crate) struct HandPlantAnimationInfo {
    pub(crate) position: [f32; 4],
    pub(crate) estimated_time_to_apex: f32,
    pub(crate) flags: u32,
}
pub(crate) struct HandPlantManager {
    pub(crate) position_attributes: AttributeRuntimeInstance,
    pub(crate) trajectory_selector: HandPlantTrajectorySelector,
    pub(crate) animation: HandPlantAnimationInfo,
    pub(crate) position_result: HandPlantPositionResult,
    pub(crate) trajectory_result: HandPlantTrajectoryOutResult,
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) trajectory_into: RetainedTrajectory,
    pub(crate) trajectory_out: RetainedTrajectory,
    pub(crate) trajectory_out_min: RetainedTrajectory,
    pub(crate) trajectory_physics_start: RetainedTrajectory,
    pub(crate) grind_spline: GrindPrim,
    pub(crate) up_vectors: [[f32; 4]; 3],
    pub(crate) matrices: [[f32; 16]; 4],
    pub(crate) bezier: [[f32; 4]; 4],
    pub(crate) grind_normal_out: [f32; 4],
    pub(crate) parametric_time: [f32; 4],
    pub(crate) parametric_time_at_apex: [f32; 4],
    pub(crate) estimated_time_to_apex: [f32; 4],
    pub(crate) real_time: [f32; 4],
    pub(crate) time_from_apex_to_ground: [f32; 4],
    pub(crate) heading_flip_scalar: [f32; 4],
    pub(crate) trajectory_ready: bool,
    pub(crate) surface_id: u32,
    pub(crate) ik: HandPlantIkData,
}
impl HandPlantManager {
    /// Only the actual Sim allocation proves zero in untouched embedded cells.
    /// The surface allocator runs at the embedded selector's source call site.
    pub(crate) fn new_sim(
        vault: &Arc<AttributeRuntimeVault>,
        allocate_surface: impl FnOnce() -> Option<SurfaceQuery>,
    ) -> Self {
        let position_attributes = vault.instance(POSITION_CLASS, DEFAULT_COLLECTION);
        let trajectory_selector = HandPlantTrajectorySelector::new_sim(vault, allocate_surface);
        let position_result = HandPlantPositionResult {
            found: false,
            position: [0.0; 4],
            direction_vs_up: 0,
            primitive: GrindPrim::default(),
        };
        let trajectory_result = HandPlantTrajectoryOutResult::new_sim();
        let attributes = vault.instance(MANAGER_CLASS, DEFAULT_COLLECTION);
        let mut result = Self {
            position_attributes,
            trajectory_selector,
            animation: HandPlantAnimationInfo {
                position: [0.0; 4],
                estimated_time_to_apex: f32::MAX,
                flags: 0,
            },
            position_result,
            trajectory_result,
            attributes,
            trajectory_into: RetainedTrajectory::constructed(),
            trajectory_out: RetainedTrajectory::constructed(),
            trajectory_out_min: RetainedTrajectory::constructed(),
            trajectory_physics_start: RetainedTrajectory::constructed(),
            grind_spline: GrindPrim::default(),
            up_vectors: [[0.0; 4]; 3],
            matrices: [[0.0; 16]; 4],
            bezier: [[0.0; 4]; 4],
            grind_normal_out: [0.0; 4],
            parametric_time: [0.0; 4],
            parametric_time_at_apex: [0.0; 4],
            estimated_time_to_apex: [0.0; 4],
            real_time: [0.0; 4],
            time_from_apex_to_ground: [0.0; 4],
            heading_flip_scalar: [0.0; 4],
            trajectory_ready: false,
            surface_id: 0,
            ik: HandPlantIkData {
                blend: 0.0,
                distance: [-1.0; 4],
                flags: 0,
            },
        };
        result.reset();
        result
    }
    /// Reset82db5c78 preserves selector/futures, trajectories, IK, matrices,
    /// primitive provenance, and animation flag bit30.
    pub(crate) fn reset(&mut self) {
        self.trajectory_ready = false;
        self.animation.flags &= 0x4fff_ffff;
        self.animation.estimated_time_to_apex = f32::MAX;
        self.animation.position = [0.0; 4];
        self.position_result.found = false;
        self.position_result.position = [0.0; 4];
        self.position_result.direction_vs_up = 0;
        self.parametric_time = [-1.0; 4];
        self.parametric_time_at_apex = [-1.0; 4];
        self.estimated_time_to_apex = [-1.0; 4];
        self.real_time = [-1.0; 4];
        self.time_from_apex_to_ground = [0.0; 4];
        self.heading_flip_scalar = [0.0; 4];
        self.surface_id = 0;
        self.grind_normal_out = [0.0; 4];
    }
}

#[cfg(test)]
pub(crate) fn check_original(
    value: &HandPlantManager,
    predictor: &Arc<
        crate::simulation_assembly_backend::constraint_session::prediction::PhysicalPredictionOwner,
    >,
) {
    assert!(!value.position_attributes.is_null());
    assert!(!value.attributes.is_null());
    assert!(!value.trajectory_selector.attributes.is_null());
    assert!(!value.trajectory_ready && !value.trajectory_selector.ready);
    assert_eq!(
        value.parametric_time.map(f32::to_bits),
        [(-1.0_f32).to_bits(); 4]
    );
    assert_eq!(value.time_from_apex_to_ground.map(f32::to_bits), [0; 4]);
    assert_eq!(
        value.ik.distance.map(f32::to_bits),
        [(-1.0_f32).to_bits(); 4]
    );
    for query in &value.trajectory_selector.queries {
        assert!(!query.ready);
        assert_eq!(
            query.result.trajectory.time.map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        assert_eq!(
            query
                .result
                .position
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [0; 4]
        );
    }
    let surface = value.trajectory_selector.surface_query.as_ref().unwrap();
    assert!(Arc::ptr_eq(surface.predictor.as_ref().unwrap(), predictor));
    assert!(!surface.query.ready);
    assert!(surface.query.collision_position.is_none());
    assert_eq!(predictor.pending_count().unwrap(), 0);
}
