//! SetBumpCoefficients.Begin82cb1600: ordered live reads and actual two writes.
use crate::{
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_normalize_safe},
    skeleton_root_transforms::SkeletonRootVector,
};
pub(crate) const X_SCALE: u64 = 0x796a_2241_00aa_f608;
pub(crate) const MINIMUM: u64 = 0x2eda_fe74_f922_c817;
pub(crate) const MAXIMUM: u64 = 0xb8ca_c55c_4fdd_8cf7;
pub(crate) const BASE: u64 = 0xc405_a27b_b4dd_bd5e;
pub(crate) trait BumpCoefficientSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn set_attribute(&mut self, index: usize, value: f32) -> Result<(), Self::Error>;
}
#[expect(
    clippy::neg_multiply,
    reason = "native mirrored branch multiplies by negative one; retain floating operation"
)]
pub(crate) fn begin<S: BumpCoefficientSource, N: ReckoningNumeric<Error = S::Error>>(
    source: &mut S,
    numeric: &mut N,
) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let mut vector = source.acceleration()?;
    vector.xyz[0] *= source.scalar(X_SCALE)?;
    let squared = numeric.dot_vector(vector, vector)?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    let length = length_from_squared_and_estimate(squared, estimate);
    let minimum = source.scalar(MINIMUM)?;
    let maximum = source.scalar(MAXIMUM)?;
    let denominator = maximum - minimum;
    let numerator = length - minimum;
    let ratio = numerator / denominator;
    let first = if -ratio >= 0. { 0. } else { ratio };
    let clamped = if 1. - first >= 0. { first } else { 1. };
    let base = source.scalar(BASE)?;
    let normalized = try_normalize_safe(
        vector,
        SkeletonRootVector {
            xyz: [0.; 3],
            w: Some(0.),
        },
        numeric,
    )?;
    let amplitude = (1. - base).mul_add(clamped, base);
    let mut coefficient = normalized.map(|value| value * amplitude);
    if source.mirrored()? {
        coefficient = coefficient.map(|value| value * -1.);
    }
    source.set_attribute(0, coefficient.xyz[0])?;
    source.set_attribute(1, coefficient.xyz[2])?;
    Ok(())
}
#[cfg(test)]
#[path = "motion_set_bump_coefficients_tests.rs"]
mod tests;
