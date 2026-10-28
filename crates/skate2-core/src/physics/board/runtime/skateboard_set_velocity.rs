//! Actual all-Part velocity replacement, 0x82b244d8..0x82b24528.
use crate::{
    dynamic_assembly_session::part_contact_spy::DynamicAssemblyPartContactSpyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Debug)]
pub(crate) enum SkateboardSetVelocityError {
    Part {
        index: usize,
        source: DynamicAssemblyPartContactSpyError,
    },
    Body {
        index: usize,
        source: SimulationRigidBodyPoolError,
    },
}

#[derive(Debug)]
pub(crate) enum TargetPositionError {
    Transform(crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError),
    Velocity(SkateboardSetVelocityError),
}
/// Skateboard::TargetPositionWithVelocity82b2a038 shared by KnownAir and
/// Controller retrieval. The current Processed DT is sampled by the caller.
pub(crate) fn try_target_position_with_velocity(
    board: &SkateboardAssemblySession,
    backend: &mut SkateboardSimulationBackend,
    dt: f32,
    target: &SkeletonRootVector,
) -> Result<(), TargetPositionError> {
    let inverse_dt = 1.0 / dt;
    let body = board
        .try_attached_part_body_transform(backend, 6)
        .map_err(TargetPositionError::Transform)?;
    let velocity = target
        .zip(body.columns()[3], |a, b| a - b)
        .map(|v| v * inverse_dt);
    try_set_skateboard_velocity(board, backend, &velocity).map_err(TargetPositionError::Velocity)
}
impl std::fmt::Display for SkateboardSetVelocityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Part { index, source } => write!(f, "set velocity Part {index}: {source}"),
            Self::Body { index, source } => write!(f, "set velocity body {index}: {source}"),
        }
    }
}
/// This exclusive backend / immutable input borrow rules out source alias mutation.
/// Each Part is resolved just before its store; no all-body preflight or rollback.
pub(crate) fn try_set_skateboard_velocity(
    board: &SkateboardAssemblySession,
    backend: &mut SkateboardSimulationBackend,
    velocity: &SkeletonRootVector,
) -> Result<(), SkateboardSetVelocityError> {
    let mut index = 0;
    while index < board.template().part_payloads().len() {
        // Native input W is overwritten by the existing body link before store.
        let xyz = velocity.xyz;
        let body = board
            .try_part_contact_spy_observation(backend, index)
            .map_err(|source| SkateboardSetVelocityError::Part { index, source })?;
        backend
            .try_set_part_linear_velocity(body, xyz)
            .map_err(|source| SkateboardSetVelocityError::Body { index, source })?;
        index += 1;
    }
    Ok(())
}
