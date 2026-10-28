//! Actual absorption82b28c60 and pump-force82b28ed8 consumers of retained Pumping.
use crate::{
    reckoning_numeric::ReckoningNumeric, reckoning_numeric::try_normalize_safe_with_refinement,
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skateboard_foot_forces::SkateboardFootForce, skeleton_root_transforms::SkeletonRootVector,
};

pub(crate) const ABSORPTION_SCALE: u64 = 0xe486_4a94_53f4_1775;
pub(crate) const NO_INTENTION_SCALE: u64 = 0x99a7_684b_304b_a44d;
pub(crate) const ENABLED: u64 = 0xb9ca_bbb8_8c47_204b;

pub(crate) trait AbsorptionSource {
    type Error;
    type Pointer;
    fn crouch(&mut self) -> Result<f32, Self::Error>;
    fn crouch_delta(&mut self) -> Result<f32, Self::Error>;
    fn capture_scale(&mut self) -> Result<Self::Pointer, Self::Error>;
    fn load_scale(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn absorption(&mut self) -> Result<f32, Self::Error>;
    fn draw_absorption(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}

/// The noncalculating crouch arm still requires the actual overlay singleton.
/// On the other arm its debug cvar read precedes the fsel/result decision.
pub(crate) fn try_absorption<S: AbsorptionSource>(source: &mut S) -> Result<f32, S::Error> {
    let crouch = source.crouch()?;
    if crouch == 0.0 || source.crouch_delta()? < 0.0 {
        let pointer = source.capture_scale()?;
        let scale = source.load_scale(&pointer)?;
        let absorption = source.absorption()?;
        let product = absorption * scale;
        let draw = source.draw_absorption()?;
        // fneg/fsel8d38/3c: NaN preserves product, either zero chooses +0.
        let result = if -product >= 0.0 { 0.0 } else { product };
        if draw {
            source.require_overlay()?;
        }
        Ok(result)
    } else {
        source.require_overlay()?;
        Ok(0.0)
    }
}

pub(crate) trait PumpForceSource {
    type Error;
    type Pointer;
    fn player_controlled_pump(&mut self) -> Result<bool, Self::Error>;
    fn acceleration(&mut self) -> Result<f32, Self::Error>;
    fn draw_intention(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn load_scalar(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn load_enabled(&mut self, pointer: &Self::Pointer) -> Result<bool, Self::Error>;
    fn mass(&mut self) -> Result<f32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn z_axis_velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PumpForceError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for PumpForceError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(f, "pump force source: {error:?}"),
            Self::Numeric(error) => write!(f, "pump force numeric: {error:?}"),
        }
    }
}

pub(crate) fn try_pump<S: PumpForceSource, N: ReckoningNumeric>(
    source: &mut S,
    numeric: &mut N,
    output: &mut SkateboardFootForce,
) -> Result<(), PumpForceError<S::Error, N::Error>> {
    try_pump_with_refinement(source, numeric, output, refine_drive_reciprocal_sqrt)
}
#[expect(
    clippy::assign_op_pattern,
    reason = "native multiplication operand order"
)]
pub(crate) fn try_pump_with_refinement<S: PumpForceSource, N: ReckoningNumeric>(
    source: &mut S,
    numeric: &mut N,
    output: &mut SkateboardFootForce,
    refine: impl FnOnce(f32, f32) -> f32,
) -> Result<(), PumpForceError<S::Error, N::Error>> {
    use PumpForceError as E;
    let intended = source.player_controlled_pump().map_err(E::Source)?;
    let mut acceleration = source.acceleration().map_err(E::Source)?;
    if !intended {
        if source.draw_intention().map_err(E::Source)? {
            source.require_overlay().map_err(E::Source)?;
        }
        let pointer = source.capture(NO_INTENTION_SCALE).map_err(E::Source)?;
        let factor = source.load_scalar(&pointer).map_err(E::Source)?;
        acceleration = factor * acceleration;
    }
    let mass = source.mass().map_err(E::Source)?;
    let impulse = mass * acceleration;
    let dt = source.time_delta().map_err(E::Source)?;
    let mut magnitude = impulse / dt;
    let pointer = source.capture(ENABLED).map_err(E::Source)?;
    if !source.load_enabled(&pointer).map_err(E::Source)? {
        magnitude = 0.0;
    }
    if std::env::var_os("SKATE2_TRACE_VERT").is_some() {
        eprintln!(
            "vert pump force: intended={intended} acceleration={acceleration} mass={mass} dt={dt} magnitude={magnitude}"
        );
    }
    // Source stores90a0/a4/a8 occur only after both tuning consumers and divide.
    output.world_space_force = SkeletonRootVector::known([0.0; 4]);
    output.local_space_position = SkeletonRootVector::known([0.0; 4]);
    let velocity = source.z_axis_velocity().map_err(E::Source)?;
    // Disabled force still executes normalization and the full-lane multiply.
    let direction = try_normalize_safe_with_refinement(
        velocity,
        SkeletonRootVector::known([0.0; 4]),
        numeric,
        refine,
    )
    .map_err(E::Numeric)?;
    output.world_space_force = SkeletonRootVector {
        xyz: direction.xyz.map(|lane| lane * magnitude),
        w: direction.w.map(|lane| lane * magnitude),
    };
    Ok(())
}

#[cfg(test)]
#[path = "skateboard_pumping_forces_tests.rs"]
mod tests;
