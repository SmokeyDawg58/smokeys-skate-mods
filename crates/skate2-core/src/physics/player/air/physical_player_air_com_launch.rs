//! Same retained Air.GetComTrajectoryLaunchInfo82d84050.
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences, air_com_launch::ComLaunchFloat},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_trajectory_owners::{
        RetainedTrajectoryLaunchInfo,
        skeleton_launch_info::{SkeletonLaunchInfoError, try_fill_launch_info},
    },
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum AirComLaunchError {
    MissingAir,
    Poisoned,
    Snapshot(SkeletonLaunchInfoError),
    Cvar(InputCvarError),
}
impl std::fmt::Display for AirComLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAir => f.write_str("AirComLaunchError: MissingAir"),
            Self::Poisoned => f.write_str("AirComLaunchError: Poisoned"),
            Self::Snapshot(error) => write!(f, "AirComLaunchError Snapshot: {error:?}"),
            Self::Cvar(error) => write!(f, "AirComLaunchError Cvar: {error:?}"),
        }
    }
}

/// The caller already validates the same Air pair. This helper deliberately
/// takes the second full Skeleton snapshot before reading Air+60 velocity.
pub(crate) fn try_fill_com_launch_info(
    constructor: &SkeletonConstructorRuntime,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    trajectory: &WallRideTrajectoryAttributes,
    output: &mut RetainedTrajectoryLaunchInfo,
) -> Result<(), AirComLaunchError> {
    try_fill_launch_info(constructor, owner, trajectory, output)
        .map_err(AirComLaunchError::Snapshot)?;
    let velocity = owner
        .physics_state()
        .lock()
        .map_err(|_| AirComLaunchError::Poisoned)?
        .air
        .as_ref()
        .ok_or(AirComLaunchError::MissingAir)?
        .fields
        .com_trajectory
        .velocity;
    finish_com_launch(output, velocity, &owner.inputs().cvars).map_err(AirComLaunchError::Cvar)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "source sth stores low sixteen bits of actual signed cvar value"
)]
fn finish_com_launch(
    output: &mut RetainedTrajectoryLaunchInfo,
    velocity: Vector,
    refs: &InputCvarReferences,
) -> Result<(), InputCvarError> {
    // kUpdateTime65@82341f44 and VECTOR3_GRAVITY initializer82f76360.
    let step = f32::from_bits(0x3c88_8889);
    let gravity = Vector::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]);
    output.start_velocity = gravity.zip(velocity, |g, v| g.mul_add(step, v));
    output.player_jumped = true;
    //84114 captures board target;84118 loads COM before trajectory target8411c.
    let board_pointer = refs.try_capture_com_launch_float(ComLaunchFloat::BoardDelta)?;
    let first_com = output.com_position;
    let trajectory_pointer = refs.try_capture_com_launch_float(ComLaunchFloat::TrajectoryDelta)?;
    let board_delta = board_pointer.read()?;
    let trajectory_delta = trajectory_pointer.read()?;
    output.trajectory_start_override = offset_y(first_com, trajectory_delta);
    output.board_position_override = offset_y(output.com_position, board_delta);
    output.use_start_override = true;
    let scalar_pointer = refs.try_capture_com_launch_float(ComLaunchFloat::ConeScalar)?;
    let old_z = output.cone_angle_z;
    let old_x = output.cone_angle_x;
    let scalar = scalar_pointer.read()?;
    output.cone_angle_x = old_x * scalar;
    output.cone_angle_z = old_z * scalar;
    output.num_trajectories = refs.try_com_launch_count()? as u16;
    Ok(())
}
fn offset_y(com: Vector, delta: f32) -> Vector {
    com.zip(Vector::known([0.0, delta, 0.0, 0.0]), |a, b| a + b)
}

#[cfg(test)]
#[path = "physical_player_air_com_launch_tests.rs"]
mod tests;
