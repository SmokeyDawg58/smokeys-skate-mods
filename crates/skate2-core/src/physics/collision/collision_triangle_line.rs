//! Native thin/fat triangle line intersection and ordered-fatness wrapper.
//!
//! Sources: `82acecf0..82acee64` and `82adf658..82adf804`. Numeric reductions,
//! estimate seeds, denormals and fused-negative exceptional signs retain the
//! declared `ReckoningNumeric` host boundary. Result writes preserve source
//! prefixes on geometric misses and numerical refusal.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "ground query awaits the live World query owner")
)]

use std::{cmp::Ordering, fmt};

use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
    skeleton_root_transforms::SkeletonRootVector,
};

#[path = "collision_fat_triangle_line.rs"]
mod fat;
#[cfg(test)]
#[path = "collision_triangle_line_tests.rs"]
mod tests;

const DETERMINANT_MINIMUM: f32 = f32::from_bits(0x322b_cc77);
const NEGATIVE_TOLERANCE: f32 = f32::from_bits(0xb727_c5ac);

/// Ordered vertices and the original segment delta, not its second endpoint.
#[derive(Debug)]
pub(crate) struct CollisionTriangleLineInput {
    pub(crate) start: SkeletonRootVector,
    pub(crate) delta: SkeletonRootVector,
    pub(crate) vertices: [SkeletonRootVector; 3],
}

/// Only fields written by the native helper. `None` means not yet written;
/// an initialized vector can independently have an unknown native W lane.
/// The untouched native volume pointer/reference are not synthesized.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct CollisionTriangleLineOutput {
    pub(crate) position: Option<SkeletonRootVector>,
    pub(crate) normal: Option<SkeletonRootVector>,
    pub(crate) volume_parameter: Option<[f32; 4]>,
    pub(crate) line_parameter: Option<f32>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CollisionTriangleLineError<E> {
    Numeric(E),
    UnreadySuccess { field: &'static str },
}

impl<E: fmt::Debug> fmt::Display for CollisionTriangleLineError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "triangle line numeric operation: {error:?}"),
            Self::UnreadySuccess { field } => {
                write!(f, "successful fat triangle has no source-produced {field}")
            }
        }
    }
}

impl<E: fmt::Debug> std::error::Error for CollisionTriangleLineError<E> {}

/// `ThinTriangleLineSegIntersect`, 0x82acecf0. A late segment rejection leaves
/// the unscaled parameter numerator stored; earlier misses store nothing.
pub(crate) fn try_thin_triangle_line<N: ReckoningNumeric>(
    input: &CollisionTriangleLineInput,
    output: &mut CollisionTriangleLineOutput,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let [first, second, third] = input.vertices;
    let edge1 = subtract(second, first);
    let edge2 = subtract(third, first);
    let perpendicular = cross(input.delta, edge2);
    let determinant = numeric.dot3(scratch(edge1), scratch(perpendicular))?;
    // BO4/BI25: reject when GT is not set, including unordered.
    if determinant.partial_cmp(&DETERMINANT_MINIMUM) != Some(Ordering::Greater) {
        return Ok(false);
    }

    let from_vertex = subtract(input.start, first);
    let lower = determinant * NEGATIVE_TOLERANCE;
    let u = numeric.dot3(scratch(from_vertex), scratch(perpendicular))?;
    let upper = determinant - lower;
    if u < lower || u > upper {
        return Ok(false);
    }

    let other_perpendicular = cross(from_vertex, edge1);
    let v = numeric.dot3(scratch(input.delta), scratch(other_perpendicular))?;
    if v < lower || v + u > upper {
        return Ok(false);
    }

    let numerator = numeric.dot3(scratch(edge2), scratch(other_perpendicular))?;
    output.line_parameter = Some(numerator);
    if numerator < lower || numerator > upper {
        return Ok(false);
    }

    // Native scalar fdivs, not an estimate or reciprocal refinement.
    let inverse = 1.0_f32 / determinant;
    let parameter = numerator * inverse;
    output.line_parameter = Some(parameter);
    let scaled_u = u * inverse;
    let scaled_v = v * inverse;
    output.position = Some(SkeletonRootVector {
        xyz: std::array::from_fn(|lane| {
            input.delta.xyz[lane].mul_add(parameter, input.start.xyz[lane])
        }),
        w: input
            .delta
            .w
            .zip(input.start.w)
            .map(|(delta, start)| delta.mul_add(parameter, start)),
    });
    output.volume_parameter = Some([scaled_u, scaled_v, 0.0, 0.0]);
    Ok(true)
}

/// `TriangleLineSegIntersect`, 0x82adf658.
/// Raw normal publication precedes both wrapper numeric callbacks. The mesh
/// caller still executes this work before selecting its separate GP normal.
pub(crate) fn try_triangle_line<N: ReckoningNumeric>(
    input: &CollisionTriangleLineInput,
    line_fatness: f32,
    triangle_fatness: f32,
    output: &mut CollisionTriangleLineOutput,
    numeric: &mut N,
) -> Result<bool, CollisionTriangleLineError<N::Error>> {
    let sum = line_fatness + triangle_fatness;
    if sum == 0.0
        && !try_thin_triangle_line(input, output, numeric)
            .map_err(CollisionTriangleLineError::Numeric)?
    {
        return Ok(false);
    }

    let [first, second, third] = input.vertices;
    let raw_normal = cross(subtract(first, second), subtract(first, third));
    output.normal = Some(raw_normal);
    let squared = numeric
        .dot3(scratch(raw_normal), scratch(raw_normal))
        .map_err(CollisionTriangleLineError::Numeric)?;
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .map_err(CollisionTriangleLineError::Numeric)?;
    let inverse = refine_drive_reciprocal_sqrt(squared, seed);
    output.normal = Some(SkeletonRootVector {
        xyz: raw_normal.xyz.map(|lane| lane * inverse),
        w: raw_normal.w.map(|lane| lane * inverse),
    });
    if sum != 0.0 {
        if !fat::intersect(input, sum, output, numeric)
            .map_err(CollisionTriangleLineError::Numeric)?
        {
            return Ok(false);
        }
        let position = output
            .position
            .ok_or(CollisionTriangleLineError::UnreadySuccess { field: "position" })?;
        let normal = output
            .normal
            .ok_or(CollisionTriangleLineError::UnreadySuccess { field: "normal" })?;
        output.position = Some(subtract(position, fat::scale(normal, line_fatness)));
    }
    Ok(true)
}

fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| a.xyz[lane] - b.xyz[lane]),
        w: a.w.zip(b.w).map(|(a, b)| a - b),
    }
}

fn cross(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    let value = cross_full4(scratch(a), scratch(b));
    SkeletonRootVector {
        xyz: [value[0], value[1], value[2]],
        w: a.w.zip(b.w).map(|_| value[3]),
    }
}

// Unknown W scratch is consumed only by XYZ dot/cross lanes. Published W is
// derived independently above; this zero is never stored as native output W.
fn scratch(vector: SkeletonRootVector) -> [f32; 4] {
    [
        vector.xyz[0],
        vector.xyz[1],
        vector.xyz[2],
        vector.w.unwrap_or(0.0),
    ]
}
