//! ComputeRightingForces82dc7eb8 numeric suffix. The owner stores the freshly
//! sampled non-Raw animation target before calling this suffix.
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed, try_signed_angle, wrap_turns_to_radians},
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine},
    skeleton_root_transforms::SkeletonRootVector as V,
};

#[derive(Debug)]
pub(crate) enum RightingError<N, R, A> {
    Numeric(N),
    Rounding(R),
    Attribute(A),
}

pub(super) fn normalize<N: ReckoningNumeric>(v: V, numeric: &mut N) -> Result<V, N::Error> {
    let squared = numeric.dot_vector(v, v)?;
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    Ok(normalize_finish(v, squared, seed))
}

/// Shared unsafe normalization finish; caller may publish source stores after estimate.
pub(crate) fn normalize_finish(v: V, squared: f32, seed: f32) -> V {
    let square = seed * seed;
    let half = seed * 0.5;
    let residual = -(squared.mul_add(square, -1.0));
    let first = half.mul_add(residual, seed);
    let square = first * first;
    let half = first * 0.5;
    let residual = -(squared.mul_add(square, -1.0));
    let inverse = half.mul_add(residual, first);
    v.map(|x| x * inverse)
}

fn wrapped(angle: f32) -> f32 {
    wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983))
}

/// Output order is yaw, pitch, roll. A later failure retains already written
/// outputs. No epsilon fallback is present in either inline normalization.
pub(crate) fn try_compute<N: ReckoningNumeric, R: ReckoningSinCosNumeric, A>(
    effective_forward: V,
    mut target_right: V,
    mut target_forward: V,
    ground_normal: V,
    output: &mut [V; 3],
    mut scalar: impl FnMut(u64) -> Result<f32, A>,
    numeric: &mut N,
    rounding: &mut R,
) -> Result<(), RightingError<N::Error, R::Error, A>> {
    use RightingError as E;
    let vertical = numeric
        .dot_vector(effective_forward, ground_normal)
        .map_err(E::Numeric)?;
    let projected = effective_forward.zip(ground_normal.map(|x| x * vertical), |a, b| a - b);
    let forward = normalize(projected, numeric).map_err(E::Numeric)?;
    let facing = numeric
        .dot_vector(target_forward, forward)
        .map_err(E::Numeric)?;
    if 0.0 > facing {
        // Native XOR sign masks, including signed zero/NaN payloads.
        target_right = target_right.map(|x| f32::from_bits(x.to_bits() ^ 0x8000_0000));
        target_forward = target_forward.map(|x| f32::from_bits(x.to_bits() ^ 0x8000_0000));
    }
    let vertical = numeric
        .dot_vector(target_forward, ground_normal)
        .map_err(E::Numeric)?;
    let flat_target = target_forward.zip(ground_normal.map(|x| x * vertical), |a, b| a - b);
    let yaw = wrapped(
        try_signed_angle(flat_target, forward, ground_normal, numeric).map_err(E::Numeric)?,
    );
    let strength = scalar(0x727c_82ba_a1ba_d755).map_err(E::Attribute)?;
    let sine = try_sine(yaw, rounding).map_err(E::Rounding)?;
    // The multiplication by sine precedes multiplication by strength.
    output[0] = ground_normal.map(|x| (x * sine) * strength);
    let pitch_axis =
        normalize(cross_typed(ground_normal, target_forward), numeric).map_err(E::Numeric)?;
    let offset = scalar(0x87f6_6c6a_6add_cb53).map_err(E::Attribute)? * f32::from_bits(0x3c8e_fa35);
    let pitch = wrapped(
        try_signed_angle(target_forward, flat_target, pitch_axis, numeric).map_err(E::Numeric)?,
    ) - offset;
    let strength = scalar(0x1e06_1522_ab8e_a5c6).map_err(E::Attribute)?;
    let sine = try_sine(pitch, rounding).map_err(E::Rounding)?;
    output[1] = pitch_axis.map(|x| (x * sine) * strength);
    let roll = wrapped(
        try_signed_angle(target_right, pitch_axis, target_forward, numeric).map_err(E::Numeric)?,
    );
    let strength = scalar(0x3f87_0094_5bad_38fe).map_err(E::Attribute)?;
    let sine = try_sine(roll, rounding).map_err(E::Rounding)?;
    output[2] = target_forward.map(|x| (x * sine) * strength);
    Ok(())
}
