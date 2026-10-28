//! UNREGISTERED complete StraightenOutTorque82b28560..28820.
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed},
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
pub(crate) trait StraightenSource {
    type Error;
    type Surface;
    fn wheel_hardness(&mut self) -> Result<f32, Self::Error>;
    fn wheel_minimum(&mut self) -> Result<f32, Self::Error>;
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error>;
    fn surface_duration(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error>;
    fn heading_graph(&mut self, ratio: f32) -> Result<f32, Self::Error>;
    /// 286a8: Processed+160 `mBodyZAxis`, not the flattened +170 direction.
    fn body_z(&mut self) -> Result<Vector, Self::Error>;
    fn velocity(&mut self) -> Result<Vector, Self::Error>;
    fn surface_strength(&mut self, surface: &Self::Surface) -> Result<f32, Self::Error>;
    fn ground_normal(&mut self) -> Result<Vector, Self::Error>;
    fn turn(&mut self) -> Result<f32, Self::Error>;
    fn maximum_turn(&mut self) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum StraightenError<S, N> {
    Source(S),
    Numeric(N),
    WheelMinimumAssertion { bits: u32 },
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for StraightenError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "straighten input: {e:?}"),
            Self::Numeric(e) => write!(f, "straighten numeric: {e:?}"),
            Self::WheelMinimumAssertion { bits } => {
                write!(f, "straighten wheel minimum outside [0,1]: {bits:08x}")
            }
        }
    }
}
fn source_select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= 0.0 { yes } else { no }
}
pub(crate) fn try_straighten<S: StraightenSource, N: ReckoningNumeric>(
    source: &mut S,
    time: f32,
    numeric: &mut N,
    output: &mut Vector,
) -> Result<(), StraightenError<S::Error, N::Error>> {
    use StraightenError as E;
    let hardness = source.wheel_hardness().map_err(E::Source)?;
    let minimum = source.wheel_minimum().map_err(E::Source)?;
    // BO4/BI25 second assertion branch includes unordered: NaN is not rejected.
    if minimum < 0.0 || minimum > 1.0 {
        return Err(E::WheelMinimumAssertion {
            bits: minimum.to_bits(),
        });
    }
    let one_minus = 1.0 - minimum;
    let surface = source.capture_surface().map_err(E::Source)?;
    let blended = one_minus.mul_add(hardness, minimum);
    let time = source_select(-time, 0.0, time);
    let duration = source.surface_duration(&surface).map_err(E::Source)?;
    let duration = blended * duration;
    let bounded = source_select(duration - time, time, duration);
    let ratio = bounded / duration;
    let curve = source.heading_graph(ratio).map_err(E::Source)?;
    let surface = source.capture_surface().map_err(E::Source)?; //286a4, fresh selection
    let direction = source.body_z().map_err(E::Source)?;
    let velocity = source.velocity().map_err(E::Source)?;
    let forward = numeric
        .dot_vector(direction, velocity)
        .map_err(E::Numeric)?;
    let strength = source.surface_strength(&surface).map_err(E::Source)? * curve;
    let direction = if forward < 0.0 {
        direction.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000))
    } else {
        direction
    };
    let normal = source.ground_normal().map_err(E::Source)?;
    let turn = source.turn().map_err(E::Source)?;
    let angular = numeric
        .dot_vector(cross_typed(direction, velocity), normal)
        .map_err(E::Numeric)?;
    let magnitude = angular * strength;
    let against = angular * turn;
    *output = normal.map(|v| v * magnitude); //28764, before optional late attribute
    if against < 0.0 {
        let absolute = turn.abs();
        let maximum = source.maximum_turn().map_err(E::Source)?;
        let lower = source_select(-absolute, 0.0, absolute);
        let upper = source_select(maximum - lower, lower, maximum);
        let ratio = upper / maximum;
        let scale = 1.0 - ratio;
        *output = output.map(|v| v * scale);
    }
    Ok(())
}

#[cfg(test)]
#[path = "skateboard_straighten_torque_tests.rs"]
mod tests;
