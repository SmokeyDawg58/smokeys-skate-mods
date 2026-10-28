//! `UpdatePredictedSkateboardPos82d84b88`; queue-based hook displacement.
use crate::{
    reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector as Vector,
};
pub(crate) trait GroundForcePredictionSource {
    type Error;
    fn mass(&mut self) -> Result<f32, Self::Error>;
    fn time_delta(&mut self) -> Result<f32, Self::Error>;
    fn combined_forces(&mut self) -> Result<Vector, Self::Error>;
    fn ground_normal(&mut self) -> Result<Vector, Self::Error>;
    fn add_offset_to_hooks(&mut self, offset: Vector) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum GroundForcePredictionError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for GroundForcePredictionError<S, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "Ground force prediction source: {e:?}"),
            Self::Numeric(e) => write!(f, "Ground force prediction numeric: {e:?}"),
        }
    }
}
pub(crate) fn try_predict_force_offset<S: GroundForcePredictionSource, N: ReckoningNumeric>(
    exiting_wallride: bool,
    source: &mut S,
    numeric: &mut N,
) -> Result<(), GroundForcePredictionError<S::Error, N::Error>> {
    use GroundForcePredictionError as E;
    if exiting_wallride {
        return Ok(());
    }
    let mass = source.mass().map_err(E::Source)?;
    let dt = source.time_delta().map_err(E::Source)?;
    let estimate = numeric.reciprocal_estimate(mass).map_err(E::Numeric)?;
    let residual = -estimate.mul_add(mass, -1.0);
    let first = estimate.mul_add(residual, estimate); //4bf0 before actual queue call
    let force = source.combined_forces().map_err(E::Source)?;
    let residual = -first.mul_add(mass, -1.0); //4c18 before normal load
    let normal = source.ground_normal().map_err(E::Source)?;
    let inverse = first.mul_add(residual, first);
    let inverse_mass_dt = inverse * dt;
    let velocity = force.map(|v| v * inverse_mass_dt);
    let projection = numeric.dot_vector(velocity, normal).map_err(E::Numeric)?;
    let normal_component = normal.map(|v| v * projection);
    let tangent = velocity.zip(normal_component, |a, b| a - b);
    let offset = tangent.map(|v| v * dt);
    source.add_offset_to_hooks(offset).map_err(E::Source)
}

#[cfg(test)]
#[path = "physical_player_ground_force_prediction_tests.rs"]
mod tests;
