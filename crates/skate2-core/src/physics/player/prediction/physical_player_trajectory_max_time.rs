//! `TrajectorySelector::CalcMaxTimeForTrajectory`82dc0118.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
    skeleton_root_transforms::SkeletonRootVector,
};

#[cfg(test)]
#[path = "physical_player_trajectory_max_time_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum MaximumTimeError<N> {
    Attribute(AttributeRuntimeError),
    Numeric(N),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    NonPositiveDiscriminant,
}
impl<N: std::fmt::Debug> std::fmt::Display for MaximumTimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trajectory maximum time: {self:?}")
    }
}

pub(crate) fn try_maximum_time<N: ReckoningNumeric>(
    trajectory: &WallRideTrajectoryAttributes,
    references: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
    start_velocity: SkeletonRootVector,
    numeric: &mut N,
) -> Result<f32, MaximumTimeError<N::Error>> {
    // Generated global owner installs DefaultDataArea(0x24c) when null.
    // Read cap before the separate FETCH, preserving source failure order.
    let layout = trajectory.capture().map_err(MaximumTimeError::Attribute)?;
    let bytes = layout
        .as_ref()
        .map_or(&[0; 0x24c][..], AttributeRuntimeData::bytes);
    let cap = f32::from_be_bytes(bytes[0x1ac..0x1b0].try_into().expect("captured layout"));
    let parameter = trajectory
        .try_scalar(0x6367_3fb6_da09_2b7e)
        .map_err(MaximumTimeError::Attribute)?;
    calculate(
        cap,
        parameter,
        start_velocity.xyz[1],
        numeric,
        || references.try_trajectory_debug_draw(),
        overlay,
    )
}

fn calculate<N: ReckoningNumeric>(
    cap: f32,
    parameter: f32,
    velocity_y: f32,
    numeric: &mut N,
    debug: impl FnOnce() -> Result<bool, InputCvarError>,
    overlay: &DebugOverlaySlot,
) -> Result<f32, MaximumTimeError<N::Error>> {
    // Raw01a0/01b4 negate twice;01b8 rounded product then01bc FMSUBS.
    let product = (-(-parameter)) * f32::from_bits(0xc19c_cccd);
    let discriminant = velocity_y.mul_add(velocity_y, -product);
    if discriminant.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(MaximumTimeError::NonPositiveDiscriminant);
    }
    let estimate = numeric
        .reciprocal_sqrt_estimate(discriminant)
        .map_err(MaximumTimeError::Numeric)?;
    // Source target/value0254/025c after estimate022c, before final refinement.
    let draw = debug().map_err(MaximumTimeError::Cvar)?;
    let length = length_from_squared_and_estimate(discriminant, estimate);
    let duration = (-velocity_y - length) * f32::from_bits(0xbdd0_fac6);
    // FSEL02b4: unordered subtraction selects duration, not a Rust min.
    let result = if duration - cap >= 0.0 { cap } else { duration };
    if draw {
        overlay.require().map_err(MaximumTimeError::Overlay)?;
    }
    Ok(result)
}
