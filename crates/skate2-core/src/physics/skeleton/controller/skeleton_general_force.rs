//! `GeneralUpdate` gravity displacement and per-Part force arithmetic.
//! References: 0x82b5bb80..0x82b5bc3c and 0x82b545e0..0x82b546c0.
//! The actual owner supplies captured body fields and commits force XYZ then
//! cooldown0 only after success. This kernel creates no body or force authority.

use crate::reckoning_numeric::ReckoningNumeric;

/// Native `kUpdateTime` at 0x82261f04, independent of either processed timestep.
pub(crate) const UPDATE_TIMESTEP: f32 = f32::from_bits(0x3c88_8889);

/// Source known full4 vector and three separately rounded products. Gravity is
/// retained processed+964, not the Simulation pool's gravity vector.
pub(crate) fn displacement_from_gravity(gravity: f32) -> [f32; 4] {
    let displacement = [0.0, -gravity, 0.0, 0.0];
    let half = displacement.map(|lane| lane * f32::from_bits(0x3f00_0000));
    let first_time = half.map(|lane| lane * UPDATE_TIMESTEP);
    first_time.map(|lane| lane * UPDATE_TIMESTEP)
}

/// `ApplyDisplacementToPart`: inverse mass and OLD force XYZ have already been
/// captured before the one reciprocal service. No zero/nonfinite skip or seed
/// fallback exists. Failed estimation leaves the caller's body untouched.
/// Host f32 FMA preserves this graph without claiming exact Xenon estimate,
/// denormal, NaN-payload or exceptional signed-zero behavior.
pub(crate) fn try_accumulate_force<N: ReckoningNumeric>(
    displacement: [f32; 4],
    captured_inverse_mass: f32,
    captured_old_force: [f32; 3],
    numeric: &mut N,
) -> Result<[f32; 3], N::Error> {
    let seed = numeric.reciprocal_estimate(UPDATE_TIMESTEP)?;
    let first_residual = (-seed).mul_add(UPDATE_TIMESTEP, 1.0);
    let first = seed.mul_add(first_residual, seed);
    let second_residual = (-first).mul_add(UPDATE_TIMESTEP, 1.0);
    let reciprocal = first.mul_add(second_residual, first);
    let scaled = displacement.map(|lane| reciprocal * lane);
    let delta = scaled.map(|lane| lane * captured_inverse_mass);
    Ok(std::array::from_fn(|lane| {
        captured_old_force[lane] + delta[lane]
    }))
}

#[cfg(test)]
#[path = "skeleton_general_force_tests.rs"]
mod tests;
