//! Native shot easing, quaternion interpolation, and normal-shot orientation.
//! Shared host dot reduction/estimate policies are not hardware bit oracles.
use crate::{
    reckoning_numeric::{ReckoningNumeric, host_dot4, try_acos},
    reckoning_sin_cos::sine::try_sine,
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_cosine, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
};

#[derive(Debug)]
pub(crate) enum ShotMathError<N, T> {
    Numeric(N),
    Trigonometry(T),
}
impl<N: std::fmt::Display, T: std::fmt::Display> std::fmt::Display for ShotMathError<N, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "shot numeric: {error}"),
            Self::Trigonometry(error) => write!(f, "shot trigonometry: {error}"),
        }
    }
}

pub(crate) fn ease<T: ReckoningSinCosNumeric>(amount: f32, trig: &mut T) -> Result<f32, T::Error> {
    let angle = amount.mul_add(f32::from_bits(0x4049_0fdb), -f32::from_bits(0x3fc9_0fdb));
    Ok(try_sine(angle, trig)?.mul_add(0.5, 0.5))
}

/// `InterpolateFloat82e3d850`; unlike quaternion interpolation, instant/hold
/// bypass trigonometry entirely.
pub(super) fn scalar<T: ReckoningSinCosNumeric>(
    first: f32,
    second: f32,
    style: u32,
    amount: f32,
    trig: &mut T,
) -> Result<f32, T::Error> {
    match style {
        0 => Ok((second - first).mul_add(ease(amount, trig)?, first)),
        1 => Ok(second),
        _ => Ok(first),
    }
}

/// `UpdateNormalShot82e400e8`: all three native `SinCos` calls, including +0,
/// precede the sole quaternion publication by the caller.
pub(super) fn normal_orientation<T: ReckoningSinCosNumeric>(
    elevation: f32,
    heading: f32,
    trig: &mut T,
) -> Result<[f32; 4], T::Error> {
    let (se, ce) = try_sin_cos(elevation * 0.5, trig)?;
    let (sh, ch) = try_sin_cos(heading * 0.5, trig)?;
    let (sz, cz) = try_sin_cos(0.0 * 0.5, trig)?;
    let p = ce * cz;
    let q = se * sz;
    let r = ce * sz;
    let s = se * cz;
    Ok([
        ch * s - sh * r,
        ch.mul_add(q, sh * p),
        ch * r - sh * s,
        ch.mul_add(p, sh * q),
    ])
}

/// `InterpolateQuaternion82e40268`: preserve two independent reciprocal
/// estimates and refinement placement around the two sine calls.
pub(crate) fn quaternion<N: ReckoningNumeric, T: ReckoningSinCosNumeric>(
    first: [f32; 4],
    second: [f32; 4],
    style: u32,
    amount: f32,
    numeric: &mut N,
    trig: &mut T,
) -> Result<[f32; 4], ShotMathError<N::Error, T::Error>> {
    use ShotMathError::Trigonometry;
    let amount = ease(amount, trig).map_err(Trigonometry)?;
    if style != 0 {
        return Ok(if style == 1 { second } else { first });
    }
    quaternion_core(first, second, amount, numeric, trig)
}

/// Same post-easing graph also in `FrameComposer.DoFraming82e3f350`.
pub(crate) fn quaternion_core<N: ReckoningNumeric, T: ReckoningSinCosNumeric>(
    mut first: [f32; 4],
    second: [f32; 4],
    amount: f32,
    numeric: &mut N,
    trig: &mut T,
) -> Result<[f32; 4], ShotMathError<N::Error, T::Error>> {
    use ShotMathError::{Numeric, Trigonometry};
    let mut dot = host_dot4(first, second);
    if dot < 0.0 {
        first = first.map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000));
        dot = f32::from_bits(dot.to_bits() ^ 0x8000_0000);
    }
    let threshold = try_cosine(f32::from_bits(0x3db2_b8c3), trig).map_err(Trigonometry)?;
    if dot > threshold {
        let positive = host_dot4(first, second) > 0.0;
        let mixed = std::array::from_fn(|lane| {
            if positive {
                (second[lane] - first[lane]).mul_add(amount, first[lane])
            } else {
                -((second[lane] + first[lane]).mul_add(amount, -first[lane]))
            }
        });
        let squared = host_dot4(mixed, mixed);
        let estimate = numeric.reciprocal_sqrt_estimate(squared).map_err(Numeric)?;
        let inverse = refine_drive_reciprocal_sqrt(squared, estimate);
        return Ok(mixed.map(|lane| lane * inverse));
    }
    let theta = try_acos(dot, numeric).map_err(Numeric)?;
    let sine = try_sine(theta, trig).map_err(Trigonometry)?;
    let first_seed = numeric.reciprocal_estimate(sine).map_err(Numeric)?;
    let second_seed = numeric.reciprocal_estimate(sine).map_err(Numeric)?;
    let first_inverse = refine_once(sine, first_seed);
    let second_inverse = refine_once(sine, second_seed);
    let second_sine = try_sine(amount * theta, trig).map_err(Trigonometry)?;
    let second_inverse = refine_once(sine, second_inverse);
    let second_weight = second_inverse * second_sine;
    let second_product = second.map(|lane| lane * second_weight);
    let first_sine = try_sine((1.0 - amount) * theta, trig).map_err(Trigonometry)?;
    let first_inverse = refine_once(sine, first_inverse);
    let first_weight = first_inverse * first_sine;
    Ok(std::array::from_fn(|lane| {
        first[lane].mul_add(first_weight, second_product[lane])
    }))
}

fn refine_once(value: f32, estimate: f32) -> f32 {
    let residual = (-estimate).mul_add(value, 1.0);
    estimate.mul_add(residual, estimate)
}

#[cfg(test)]
#[path = "camera_shot_runtime_math_tests.rs"]
mod tests;
