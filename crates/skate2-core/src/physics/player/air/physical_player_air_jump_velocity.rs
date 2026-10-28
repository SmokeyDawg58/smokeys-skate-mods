//! Actual `PhysicsAir::CalcVelFromJump`82d8af08; same retained Processed owner.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences, air_jump::AirJumpFloat},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::{ReckoningClampError, ReckoningNumeric, try_clamp},
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
#[derive(Debug)]
pub(crate) enum AirJumpVelocityError<N> {
    Missing(&'static str),
    Poisoned,
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    FramesAssertion { frames: i32, limit: i32 },
    GravityAssertion { bits: u32 },
    Numeric(N),
    Clamp(ReckoningClampError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for AirJumpVelocityError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirJumpVelocityError: Poisoned"),
            Self::Missing(error) => write!(f, "AirJumpVelocityError Missing: {error:?}"),
            Self::Cvar(error) => write!(f, "AirJumpVelocityError Cvar: {error:?}"),
            Self::Overlay(error) => write!(f, "AirJumpVelocityError Overlay: {error:?}"),
            Self::Numeric(error) => write!(f, "AirJumpVelocityError Numeric: {error:?}"),
            Self::Clamp(error) => write!(f, "AirJumpVelocityError Clamp: {error:?}"),
            Self::FramesAssertion { frames, limit } => {
                write!(f, "jump frames {frames} exceeds limit {limit}")
            }
            Self::GravityAssertion { bits } => write!(f, "jump gravity assertion {bits:08x}"),
        }
    }
}

fn debug<N>(
    refs: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
) -> Result<(), AirJumpVelocityError<N>> {
    if refs
        .try_draw_physics_air_debug_info()
        .map_err(AirJumpVelocityError::Cvar)?
    {
        overlay.require().map_err(AirJumpVelocityError::Overlay)?;
    }
    Ok(())
}
/// The result is caller-local scratch. No retained velocity is published until
/// the caller consumes this successfully completed source return.
#[expect(
    clippy::cast_precision_loss,
    reason = "native signed i32 to f32 conversion via fcfid/frsp"
)]
pub(crate) fn try_calculate<N: ReckoningNumeric>(
    owner: &PhysicalPlayerAssemblyOwner,
    frames: i32,
    overlay: &DebugOverlaySlot,
    n: &mut N,
) -> Result<Vector, AirJumpVelocityError<N::Error>> {
    use AirJumpVelocityError as E;
    let refs = &owner.inputs().cvars;
    let limit = refs.try_physics_air_jump_fix_frames().map_err(E::Cvar)?;
    if frames >= limit {
        return Err(E::FramesAssertion { frames, limit });
    }
    let processed = std::sync::Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .processed(),
    );
    let (dt, gravity) = {
        let p = processed.lock().map_err(|_| E::Poisoned)?;
        (
            p.physics_state_fields()
                .time_delta
                .ok_or(E::Missing("DT93c"))?,
            p.gravity().ok_or(E::Missing("gravity964"))?.value,
        )
    };
    let elapsed = dt * (frames as f32);
    if gravity.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) {
        return Err(E::GravityAssertion {
            bits: gravity.to_bits(),
        });
    }
    let intended = processed
        .lock()
        .map_err(|_| E::Poisoned)?
        .physics_state_fields()
        .state_intended_jump_velocity
        .ok_or(E::Missing("intended jump velocity350"))?;
    let min_pointer = refs
        .try_capture_air_jump_float(AirJumpFloat::MinimumYError)
        .map_err(E::Cvar)?;
    let current = processed
        .lock()
        .map_err(|_| E::Poisoned)?
        .prior_output()
        .velocity()
        .ok_or(E::Missing("velocity190"))?
        .value;
    let min_y = min_pointer.read().map_err(E::Cvar)?;
    let mut result = intended.zip(Vector::known([0.0, gravity * elapsed, 0.0, 0.0]), |a, b| {
        a + b
    });
    let difference = current.zip(result, |a, b| a - b);
    let keep_y = difference.xyz[1] > min_y; // splatted ordered VCMPGT; unordered chooses current Y.
    debug(refs, overlay)?;
    if !keep_y {
        result.xyz[1] = current.xyz[1];
    }
    let mut planar_difference = difference;
    planar_difference.xyz[1] = 0.0;
    let max_pointer = refs
        .try_capture_air_jump_float(AirJumpFloat::MaximumError)
        .map_err(E::Cvar)?;
    let squared = n
        .dot_vector(planar_difference, planar_difference)
        .map_err(E::Numeric)?;
    let maximum = max_pointer.read().map_err(E::Cvar)?;
    //40980014: BO4 BI24 branches when LT is clear, INCLUDING unordered.
    if requires_planar_correction(squared, maximum) {
        result = correct_planar(result, current, n)?;
    }
    debug(refs, overlay)?;
    Ok(result)
}
fn requires_planar_correction(squared: f32, maximum: f32) -> bool {
    squared.partial_cmp(&(maximum * maximum)) != Some(std::cmp::Ordering::Less)
}
fn correct_planar<N: ReckoningNumeric>(
    mut result: Vector,
    mut current: Vector,
    n: &mut N,
) -> Result<Vector, AirJumpVelocityError<N::Error>> {
    use AirJumpVelocityError as E;
    let mut planar = result;
    planar.xyz[1] = 0.0;
    let squared = n.dot_vector(planar, planar).map_err(E::Numeric)?;
    let estimate = n.reciprocal_sqrt_estimate(squared).map_err(E::Numeric)?;
    current.xyz[1] = 0.0;
    // Standard VNMSUB negates after the fused subtraction; preserve its zero sign.
    let first = (estimate * 0.5).mul_add(-(squared.mul_add(estimate * estimate, -1.0)), estimate);
    let second = (first * 0.5).mul_add(-(squared.mul_add(first * first, -1.0)), first);
    let maximum = if squared == 0.0 {
        0.0
    } else {
        squared * second
    };
    let bounded = try_clamp(current, maximum, n).map_err(E::Clamp)?;
    result.xyz[0] = bounded.xyz[0];
    result.xyz[2] = bounded.xyz[2];
    Ok(result)
}
#[cfg(test)]
#[path = "physical_player_air_jump_velocity_tests.rs"]
mod tests;
