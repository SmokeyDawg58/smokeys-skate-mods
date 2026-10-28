//! IK range clamp (0x82e496d0) and line frames (0x82b3e030/0x82b3e1d8).
//! Existing typed vectors/matrices carry full W knowledge. Host dot/estimate,
//! denormal, NaN-payload and exact-zero-sign limitations remain explicit.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "pre-physics wrapper owns line-frame integration")
)]

use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4, normalize_from_squared_and_estimate},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skateboard_spawn_math::skateboard_spawn_rigid_inverse,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_ik_line_frame_tests.rs"]
mod tests;

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
const NORMALIZE_EPSILON: f32 = f32::from_bits(0x3586_37bd);
const FRAME_QUALITY_MINIMUM: f32 = f32::from_bits(0x3780_0000);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkClampError<E> {
    NegativeRange { bits: u32 },
    NonpositiveLength { bits: u32 },
    Numeric(E),
}

/// 0x82e496d0..0x82e49854. Both assertions and numeric failures precede the
/// only output store. NaN range passes the first ordered-negative assertion.
pub(crate) fn try_clamp_vector_within_range<N: ReckoningNumeric>(
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    range: f32,
    output: &mut SkeletonRootVector,
    numeric: &mut N,
) -> Result<(), SkeletonIkClampError<N::Error>> {
    use SkeletonIkClampError as Error;
    if range < 0.0 {
        return Err(Error::NegativeRange {
            bits: range.to_bits(),
        });
    }
    let delta = subtract(a, b);
    let squared = numeric
        .dot3(scratch(delta), scratch(delta))
        .map_err(Error::Numeric)?;
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(Error::Numeric)?;
    let inverse = refine_drive_reciprocal_sqrt(squared, seed);
    let product = squared * inverse;
    let length = if squared == 0.0 { 0.0 } else { product };
    if range >= length {
        *output = a;
    } else {
        if length.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(Error::NonpositiveLength {
                bits: length.to_bits(),
            });
        }
        // Native scalar fdivs, followed by a full-lane fused delta*factor+B.
        let factor = range / length;
        *output = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| delta.xyz[lane].mul_add(factor, b.xyz[lane])),
            w: delta
                .w
                .zip(b.w)
                .map(|(delta, base)| delta.mul_add(factor, base)),
        };
    }
    Ok(())
}

/// 0x82b3e030..0x82b3e1d8. Normalize Z, X, Y; then quality dots X, Z, Y.
/// A false geometric result publishes I/J/K/P0; a numeric failure writes nothing.
pub(crate) fn try_create_matrix_from_line_and_normal<N: ReckoningNumeric>(
    start: SkeletonRootVector,
    end: SkeletonRootVector,
    normal: SkeletonRootVector,
    output: &mut SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let line = subtract(end, start);
    let raw_side = cross(line, normal);
    let raw_normal = cross(raw_side, line);
    let z = normalize_safe(raw_normal, numeric)?;
    let x = normalize_safe(raw_side, numeric)?;
    let y = normalize_safe(line, numeric)?;
    let x_squared = numeric.dot3(scratch(x), scratch(x))?;
    let z_squared = numeric.dot3(scratch(z), scratch(z))?;
    let y_squared = numeric.dot3(scratch(y), scratch(y))?;
    let zx = z_squared * x_squared;
    let quality = zx * y_squared;
    let success = quality > FRAME_QUALITY_MINIMUM;
    let columns = if success {
        [x, y, z, start]
    } else {
        *SkeletonRootMatrix::known(IDENTITY).columns()
    };
    publish(output, columns, [0, 1, 2, 3]);
    Ok(success)
}

/// 0x82b3e1d8..0x82b3e384. The first false frame skips every second-frame
/// callback. Numeric failure preserves the caller's old output, even if the
/// first local frame succeeded. Success commits Z/Y/X/P in native order.
pub(crate) fn try_calculate_matrix_from_line<N: ReckoningNumeric>(
    old_line: [SkeletonRootVector; 2],
    old_normal: SkeletonRootVector,
    new_line: [SkeletonRootVector; 2],
    new_normal: SkeletonRootVector,
    output: &mut SkeletonRootMatrix,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let mut old_frame = SkeletonRootMatrix::known(IDENTITY);
    let mut new_frame = SkeletonRootMatrix::known(IDENTITY);
    if !try_create_matrix_from_line_and_normal(
        old_line[0],
        old_line[1],
        old_normal,
        &mut old_frame,
        numeric,
    )? || !try_create_matrix_from_line_and_normal(
        new_line[0],
        new_line[1],
        new_normal,
        &mut new_frame,
        numeric,
    )? {
        publish(
            output,
            *SkeletonRootMatrix::known(IDENTITY).columns(),
            [0, 1, 2, 3],
        );
        return Ok(false);
    }
    // The accepted inverse consumes XYZ and derives all inverse W lanes;
    // old frame's unknown W is never fabricated into a retained known input.
    let inverse =
        SkeletonRootMatrix::known(skateboard_spawn_rigid_inverse(&old_frame.xyz_scratch()));
    let result = new_frame.compose(&inverse);
    publish(output, *result.columns(), [2, 1, 0, 3]);
    Ok(true)
}

fn publish(output: &mut SkeletonRootMatrix, values: [SkeletonRootVector; 4], order: [usize; 4]) {
    let mut columns = *output.columns();
    for column in order {
        columns[column] = values[column];
        *output = SkeletonRootMatrix::from_columns(columns);
    }
}

/// Typed adapter for accepted `NormalizeSafe` at 0x824b18e0. Its exact fallback
/// recovers known-zero W; a successful normalization preserves unknown W even
/// if a controlled host estimate happens to produce a zero scaling factor.
pub(crate) fn normalize_safe<N: ReckoningNumeric>(
    value: SkeletonRootVector,
    numeric: &mut N,
) -> Result<SkeletonRootVector, N::Error> {
    let lanes = scratch(value);
    let squared = numeric.dot3(lanes, lanes)?;
    let difference = squared - NORMALIZE_EPSILON * NORMALIZE_EPSILON;
    if difference > 0.0 {
        let seed = numeric.reciprocal_sqrt_estimate(squared)?;
        let result = normalize_from_squared_and_estimate(lanes, squared, seed);
        Ok(SkeletonRootVector {
            xyz: [result[0], result[1], result[2]],
            w: value.w.map(|_| result[3]),
        })
    } else {
        Ok(ZERO)
    }
}

fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}
fn cross(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    let lanes = cross_full4(scratch(left), scratch(right));
    SkeletonRootVector {
        xyz: [lanes[0], lanes[1], lanes[2]],
        w: left.w.zip(right.w).map(|_| lanes[3]),
    }
}
fn scratch(value: SkeletonRootVector) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}
