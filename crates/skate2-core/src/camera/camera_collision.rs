//! Actual camera collision owner allocated at `CreateCameraManInstances82e3a578`.
use crate::{
    actor_construction::ActorConstruction, camera_subject::ZERO,
    physical_player_assembly::PhysicalAssemblyWorld, skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone, Copy)]
pub(crate) struct CameraLineResult {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
    pub(crate) parameter: f32,
    pub(crate) collided: bool,
}
impl CameraLineResult {
    /// The actual constructor null-result object; never a fabricated query result.
    pub(crate) const fn constructed() -> Self {
        Self {
            position: ZERO,
            normal: ZERO,
            parameter: 0.0,
            collided: false,
        }
    }
}
#[path = "camera_collision_query.rs"]
pub(crate) mod query;
pub(crate) struct CameraCollision {
    // Host-interrupted native acquisitions survive until the real Release.
    pub(crate) pending_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch>,
    // The session retains the camera, so its native non-owning island link is weak.
    pub(crate) world: Weak<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) result: CameraLineResult,
    pub(crate) actor: Option<Arc<ActorConstruction>>,
}
impl CameraCollision {
    pub(crate) fn constructed(world: Weak<Mutex<PhysicalAssemblyWorld>>) -> Self {
        Self {
            pending_batch: None,
            world,
            result: CameraLineResult::constructed(),
            actor: None,
        }
    }
}
