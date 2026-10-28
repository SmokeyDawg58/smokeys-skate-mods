//! Native-shaped swept bounds used by `SerialisedAssembly` collision jobs.
//!
//! `EA::Physics::CalculatePartBBox` (`0x82e0b760..0x82e0b96c`) first asks a
//! concrete `rw::collision::Volume` for its transformed bounds, then expands
//! them for one predicted body step. This module reconstructs the five
//! concrete primitive `GetBBox` implementations and the subsequent swept
//! value graph. `CalculatePartBBoxUnion`
//! (`0x82e0b970..0x82e0b9fc`) is retained as a disjoint safe-host scan.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the live SerialisedAssembly publication owner"
    )
)]

use crate::octree::AabbWords;
use skate2_formats::collision::{Volume, VolumeType};

const PART_BBOX_TIMESTEP: f32 = f32::from_bits(0x3c88_8889);
const PART_BBOX_FINAL_EXPANSION: f32 = f32::from_bits(0x3f86_6666);
const FLOAT_ABSOLUTE_MASK: u32 = 0x7fff_ffff;

/// Four native words loaded from each three-float `RigidBody` motion field.
///
/// The fourth words are adjacent PDB members (`mRight`, `mLeft`, `mKine`, and
/// `mCool`) rather than proven vector components. They remain explicit because
/// the VMX loads and later vector arithmetic consume them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SerialisedAssemblyBodyMotion {
    pub(super) velocity_and_right: [f32; 4],
    pub(super) omega_and_left: [f32; 4],
    pub(super) force_and_kine: [f32; 4],
    pub(super) torque_and_cool: [f32; 4],
}

/// Numeric seam for native `vminfp128` and `vmaxfp128`.
///
/// The operation order and operands are recovered exactly. A future Xenon
/// oracle supplies exceptional NaN, signed-zero, denormal, and FPSCR behavior.
pub(super) trait SerialisedAssemblyBBoxMinMaxNumeric {
    type Error;

    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error>;

    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error>;
}

/// Remaining Xenon-only reductions used by `CalculatePartBBox`.
pub(super) trait SerialisedAssemblyPartBBoxNumeric:
    SerialisedAssemblyBBoxMinMaxNumeric
{
    /// Executes one native `vmsum3fp128` with the exact vector operands.
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error>;

    /// Executes the pre-refinement `vrsqrtefp128` seed operation.
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error>;
}

/// Executable host arithmetic for the recovered primitive/swept bounds graph.
///
/// Reference operation sites: `CalculatePartBBox`, `0x82E0B760..0x82E0B96C`.
/// Ordinary finite comparisons use the native operands; equal or unordered
/// comparisons select the right operand as an explicit host policy. XYZ dot
/// products round each product, then sum `(x + y) + z`. The seed is the host
/// `1 / sqrt(value)`; the existing graph still executes both recovered fused
/// refinement rounds and its later zero selection.
///
/// This provider does not establish Xenon `vmsum3fp128` reduction rounding,
/// `vrsqrtefp128` estimate bits, signed-zero/NaN selection, denormal handling,
/// or FPSCR/exception behavior, even when the resulting bounds look similar.
/// It never rejects, clamps, or replaces nonfinite operands. All four lanes
/// participate in min/max; only the first three participate in dot products.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct HostSerialisedAssemblyBBoxNumeric;

impl SerialisedAssemblyBBoxMinMaxNumeric for HostSerialisedAssemblyBBoxNumeric {
    type Error = core::convert::Infallible;

    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(|lane| {
            if left[lane] < right[lane] {
                left[lane]
            } else {
                right[lane]
            }
        }))
    }

    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(std::array::from_fn(|lane| {
            if left[lane] > right[lane] {
                left[lane]
            } else {
                right[lane]
            }
        }))
    }
}

impl SerialisedAssemblyPartBBoxNumeric for HostSerialisedAssemblyBBoxNumeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        let x_product = left[0] * right[0];
        let y_product = left[1] * right[1];
        let z_product = left[2] * right[2];
        let xy_sum = x_product + y_product;
        Ok(xy_sum + z_product)
    }

    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(value.sqrt().recip())
    }
}

/// Reconstructs the concrete primitive `Volume::GetBBox` dispatch currently
/// proven from the native vtables.
///
/// The optional matrix is the parent transform passed to `GetBBox`; native
/// code transforms the volume's stored vectors only when this pointer is
/// non-null. `tight` is ignored by Sphere, Triangle, Box, and Capsule, while
/// any nonzero value selects Cylinder's component-wise path.
///
/// Vector operations retain native source association and all four lanes.
/// Individual scalar/vector arithmetic is an ordinary-finite host
/// transcription; reciprocal-square-root estimates and min/max remain behind
/// the numeric seam, and exact Xenon exceptional/FPSCR behavior stays open.
/// Native Triangle temporarily stores and reloads its reduced box before
/// radius expansion; this owned-return form claims only nonaliased output and
/// does not model overlapping `this`/output pointers.
pub(crate) fn try_get_primitive_bbox<N: SerialisedAssemblyPartBBoxNumeric>(
    volume: &Volume,
    parent_transform: Option<&[f32; 16]>,
    tight: u32,
    numeric: &mut N,
) -> Result<Option<AabbWords>, N::Error> {
    let result = match volume.volume_type() {
        Some(VolumeType::Sphere) => sphere_bbox(volume, parent_transform),
        Some(VolumeType::Triangle) => triangle_bbox(volume, parent_transform, numeric)?,
        Some(VolumeType::Box) => box_bbox(volume, parent_transform),
        Some(VolumeType::Capsule) => capsule_bbox(volume, parent_transform),
        Some(VolumeType::Cylinder) => cylinder_bbox(volume, parent_transform, tight, numeric)?,
        Some(VolumeType::Null | VolumeType::Aggregate) | None => {
            return Ok(None);
        }
    };
    Ok(Some(result))
}

/// Complete supported-primitive path through
/// `EA::Physics::CalculatePartBBox`.
///
/// The native helper always passes a non-null transform and literal tight
/// value one to the Volume vtable before consuming the returned shape box.
/// Recovered Create explicitly bypasses Aggregate. Null also remains `None`
/// here, but the staged owner refuses it as malformed input because native
/// would attempt its non-Aggregate vtable path.
pub(super) fn try_calculate_primitive_part_bbox<N: SerialisedAssemblyPartBBoxNumeric>(
    volume: &Volume,
    transform: &[f32; 16],
    body: SerialisedAssemblyBodyMotion,
    numeric: &mut N,
) -> Result<Option<AabbWords>, N::Error> {
    let Some(shape_bounds) = try_get_primitive_bbox(volume, Some(transform), 1, numeric)? else {
        return Ok(None);
    };
    try_expand_part_bbox_after_shape(shape_bounds, body, numeric).map(Some)
}

fn sphere_bbox(volume: &Volume, parent_transform: Option<&[f32; 16]>) -> AabbWords {
    let stored_center = affine_column(volume.transform, 3);
    let center = parent_transform.map_or(stored_center, |transform| {
        transform_affine_point(*transform, stored_center)
    });
    let radius = splat(volume.radius);
    AabbWords::from_f32(vector_sub(center, radius), vector_add(center, radius))
}

fn triangle_bbox<N: SerialisedAssemblyBBoxMinMaxNumeric>(
    volume: &Volume,
    parent_transform: Option<&[f32; 16]>,
    numeric: &mut N,
) -> Result<AabbWords, N::Error> {
    let stored_points = std::array::from_fn(|column| affine_column(volume.transform, column));
    let points = parent_transform.map_or(stored_points, |transform| {
        transform_triangle_points(*transform, stored_points)
    });
    let minimum_one_two = numeric.minimum(points[1], points[2])?;
    let maximum_one_two = numeric.maximum(points[1], points[2])?;
    let minimum = numeric.minimum(points[0], minimum_one_two)?;
    let maximum = numeric.maximum(points[0], maximum_one_two)?;
    let radius = splat(volume.radius);
    Ok(AabbWords::from_f32(
        vector_sub(minimum, radius),
        vector_add(maximum, radius),
    ))
}

fn box_bbox(volume: &Volume, parent_transform: Option<&[f32; 16]>) -> AabbWords {
    let stored_frame = std::array::from_fn(|column| affine_column(volume.transform, column));
    let [x_axis, y_axis, z_axis, center] = parent_transform.map_or(stored_frame, |transform| {
        transform_box_frame(*transform, stored_frame)
    });
    let [half_x, half_y, half_z] = volume.shape_words.map(f32::from_bits);
    let y_extent = vector_mul(absolute_bits(y_axis), splat(half_y));
    let after_x = vector_fused_multiply_add(absolute_bits(x_axis), splat(half_x), y_extent);
    let after_z = vector_fused_multiply_add(absolute_bits(z_axis), splat(half_z), after_x);
    let extent = vector_add(after_z, splat(volume.radius));
    AabbWords::from_f32(vector_sub(center, extent), vector_add(center, extent))
}

fn capsule_bbox(volume: &Volume, parent_transform: Option<&[f32; 16]>) -> AabbWords {
    let stored_axis = affine_column(volume.transform, 2);
    let stored_center = affine_column(volume.transform, 3);
    let (axis, center) = parent_transform.map_or((stored_axis, stored_center), |transform| {
        transform_capsule_frame(*transform, stored_axis, stored_center)
    });
    let half_height = f32::from_bits(volume.shape_words[0]);
    let extent = vector_fused_multiply_add(
        absolute_bits(axis),
        splat(half_height),
        splat(volume.radius),
    );
    AabbWords::from_f32(vector_sub(center, extent), vector_add(center, extent))
}

fn cylinder_bbox<N: SerialisedAssemblyPartBBoxNumeric>(
    volume: &Volume,
    parent_transform: Option<&[f32; 16]>,
    tight: u32,
    numeric: &mut N,
) -> Result<AabbWords, N::Error> {
    let stored_axis = affine_column(volume.transform, 2);
    let stored_center = affine_column(volume.transform, 3);
    let (axis, center) = parent_transform.map_or((stored_axis, stored_center), |transform| {
        transform_cylinder_frame(*transform, stored_axis, stored_center)
    });
    let half_height = f32::from_bits(volume.shape_words[0]);
    let inner_radius = f32::from_bits(volume.shape_words[1]);

    let extent = if tight == 0 {
        let radial_extent = volume.radius + inner_radius;
        let axial_extent = volume.radius + half_height;
        let radial_squared = radial_extent * radial_extent;
        let diagonal_squared = axial_extent.mul_add(axial_extent, radial_squared);
        let estimate = numeric.reciprocal_square_root_estimate(diagonal_squared)?;
        let diagonal_is_zero = diagonal_squared == 0.0;
        let refined_once = refine_reciprocal_square_root(estimate, diagonal_squared);
        let refined_twice = refine_reciprocal_square_root(refined_once, diagonal_squared);
        let candidate = diagonal_squared * refined_twice;
        let diagonal = if diagonal_is_zero { 0.0 } else { candidate };
        [diagonal, diagonal, diagonal, 0.0]
    } else {
        try_cylinder_tight_extent(axis, volume.radius, inner_radius, half_height, numeric)?
    };

    Ok(AabbWords::from_f32(
        vector_sub(center, extent),
        vector_add(center, extent),
    ))
}

fn try_cylinder_tight_extent<N: SerialisedAssemblyPartBBoxNumeric>(
    axis: [f32; 4],
    outer_radius: f32,
    inner_radius: f32,
    half_height: f32,
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let radial_extent = outer_radius + inner_radius;
    let x_squared = axis[0] * axis[0];
    let y_squared = axis[1] * axis[1];
    let axial_extent = outer_radius + half_height;

    let x_difference = 1.0 - x_squared;
    let x_perpendicular_squared = absolute_scalar_bits(x_difference);
    let absolute_x = absolute_scalar_bits(axis[0]);
    let x_estimate_zero = numeric.reciprocal_square_root_estimate(x_perpendicular_squared)?;
    let x_is_zero = x_perpendicular_squared == 0.0;
    let x_estimate_zero_squared = x_estimate_zero * x_estimate_zero;
    let x_estimate_zero_half = x_estimate_zero * 0.5;
    let x_residual_zero = (-x_perpendicular_squared).mul_add(x_estimate_zero_squared, 1.0);

    let y_difference = 1.0 - y_squared;
    let x_axial = absolute_x * axial_extent;
    let x_estimate_one = x_estimate_zero_half.mul_add(x_residual_zero, x_estimate_zero);
    let x_estimate_one_squared = x_estimate_one * x_estimate_one;
    let x_estimate_one_half = x_estimate_one * 0.5;
    let y_perpendicular_squared = absolute_scalar_bits(y_difference);

    let z_squared = axis[2] * axis[2];
    let x_residual_one = (-x_perpendicular_squared).mul_add(x_estimate_one_squared, 1.0);
    let y_estimate_zero = numeric.reciprocal_square_root_estimate(y_perpendicular_squared)?;
    let absolute_y = absolute_scalar_bits(axis[1]);
    let y_is_zero = y_perpendicular_squared == 0.0;

    let z_difference = 1.0 - z_squared;
    let y_estimate_zero_squared = y_estimate_zero * y_estimate_zero;
    let x_estimate_two = x_estimate_one_half.mul_add(x_residual_one, x_estimate_one);
    let z_perpendicular_squared = absolute_scalar_bits(z_difference);
    let y_residual_zero = (-y_perpendicular_squared).mul_add(y_estimate_zero_squared, 1.0);
    let y_estimate_zero_half = y_estimate_zero * 0.5;
    let z_estimate_zero = numeric.reciprocal_square_root_estimate(z_perpendicular_squared)?;
    let z_is_zero = z_perpendicular_squared == 0.0;
    let absolute_z = absolute_scalar_bits(axis[2]);

    let y_axial = absolute_y * axial_extent;
    let x_candidate = x_perpendicular_squared * x_estimate_two;
    let y_estimate_one = y_estimate_zero_half.mul_add(y_residual_zero, y_estimate_zero);
    let z_estimate_zero_squared = z_estimate_zero * z_estimate_zero;
    let z_axial = absolute_z * axial_extent;
    let x_perpendicular = if x_is_zero { 0.0 } else { x_candidate };
    let y_estimate_one_squared = y_estimate_one * y_estimate_one;
    let y_estimate_one_half = y_estimate_one * 0.5;
    let x_extent = x_perpendicular.mul_add(radial_extent, x_axial);
    let z_estimate_zero_half = z_estimate_zero * 0.5;
    let z_residual_zero = (-z_perpendicular_squared).mul_add(z_estimate_zero_squared, 1.0);
    let y_residual_one = (-y_perpendicular_squared).mul_add(y_estimate_one_squared, 1.0);
    let z_estimate_one = z_estimate_zero_half.mul_add(z_residual_zero, z_estimate_zero);
    let y_estimate_two = y_estimate_one_half.mul_add(y_residual_one, y_estimate_one);
    let z_estimate_one_squared = z_estimate_one * z_estimate_one;
    let z_estimate_one_half = z_estimate_one * 0.5;
    let y_candidate = y_perpendicular_squared * y_estimate_two;
    let z_residual_one = (-z_perpendicular_squared).mul_add(z_estimate_one_squared, 1.0);
    let y_perpendicular = if y_is_zero { 0.0 } else { y_candidate };
    let y_extent = y_perpendicular.mul_add(radial_extent, y_axial);
    let z_estimate_two = z_estimate_one_half.mul_add(z_residual_one, z_estimate_one);
    let z_candidate = z_perpendicular_squared * z_estimate_two;
    let z_perpendicular = if z_is_zero { 0.0 } else { z_candidate };
    let z_extent = z_perpendicular.mul_add(radial_extent, z_axial);

    Ok([x_extent, y_extent, z_extent, x_squared])
}

/// Applies the post-`Volume::GetBBox` graph of `CalculatePartBBox`.
///
/// The reference unconditionally reads a non-null body and performs three
/// `vmsum3fp128` reductions plus one reciprocal-square-root estimate. Even a
/// zero angular magnitude reaches the estimate; the zero select occurs only
/// after both fused refinement rounds. Basic vector arithmetic below is an
/// ordinary-finite host expression of the recovered instruction graph. Exact
/// exceptional arithmetic and the meaning of the fourth lanes remain open.
fn try_expand_part_bbox_after_shape<N: SerialisedAssemblyPartBBoxNumeric>(
    shape_bounds: AabbWords,
    body: SerialisedAssemblyBodyMotion,
    numeric: &mut N,
) -> Result<AabbWords, N::Error> {
    let low = shape_bounds.min_f32();
    let high = shape_bounds.max_f32();
    let zero = [0.0; 4];
    let one = [1.0; 4];

    let extent_x = splat(high[0] - low[0]);
    let extent_y = splat(high[1] - low[1]);
    let extent_z = splat(high[2] - low[2]);
    let extent_y_minus_z = absolute_bits(vector_sub(extent_y, extent_z));
    let extent_x_minus_y = absolute_bits(vector_sub(extent_x, extent_y));
    let extent_z_minus_x = absolute_bits(vector_sub(extent_z, extent_x));
    let outer_shape_delta = numeric.maximum(extent_y_minus_z, extent_z_minus_x)?;
    let shape_delta = numeric.maximum(extent_x_minus_y, outer_shape_delta)?;

    let angular_step = vector_mul_scalar(body.omega_and_left, PART_BBOX_TIMESTEP);
    let torque_step = vector_mul_scalar(body.torque_and_cool, PART_BBOX_TIMESTEP);
    let torque_step_squared = vector_mul_scalar(torque_step, PART_BBOX_TIMESTEP);
    let angular_projection = numeric.sum_three_products(angular_step, torque_step_squared)?;
    let nonnegative_angular_projection = numeric.maximum(zero, splat(angular_projection))?;
    let clamped_angular_projection = numeric.minimum(one, nonnegative_angular_projection)?[0];
    let scaled_torque = vector_mul_scalar(body.torque_and_cool, clamped_angular_projection);
    let scaled_torque_step = vector_mul_scalar(scaled_torque, PART_BBOX_TIMESTEP);
    let predicted_angular =
        vector_fused_multiply_add(scaled_torque_step, splat(PART_BBOX_TIMESTEP), angular_step);

    let angular_squared = numeric.sum_three_products(predicted_angular, predicted_angular)?;
    let estimate = numeric.reciprocal_square_root_estimate(angular_squared)?;
    let refined_once = refine_reciprocal_square_root(estimate, angular_squared);
    let refined_twice = refine_reciprocal_square_root(refined_once, angular_squared);
    let angular_candidate = angular_squared * refined_twice;
    let selected_angular_candidate = if angular_squared == 0.0 {
        0.0
    } else {
        angular_candidate
    };
    let angular_amount = numeric.minimum(splat(selected_angular_candidate), one)?[0];
    let angular_padding = vector_mul_scalar(shape_delta, angular_amount);
    let angular_high = vector_add(high, angular_padding);
    let angular_low = vector_sub(low, angular_padding);

    let linear_step = vector_mul_scalar(body.velocity_and_right, PART_BBOX_TIMESTEP);
    let force_step = vector_mul_scalar(body.force_and_kine, PART_BBOX_TIMESTEP);
    let force_step_squared = vector_mul_scalar(force_step, PART_BBOX_TIMESTEP);
    let linear_projection = numeric.sum_three_products(linear_step, force_step_squared)?;
    let nonnegative_linear_projection = numeric.maximum(zero, splat(linear_projection))?;
    let clamped_linear_projection = numeric.minimum(one, nonnegative_linear_projection)?[0];
    let scaled_force = vector_mul_scalar(body.force_and_kine, clamped_linear_projection);
    let scaled_force_step = vector_mul_scalar(scaled_force, PART_BBOX_TIMESTEP);
    let predicted_linear =
        vector_fused_multiply_add(splat(PART_BBOX_TIMESTEP), scaled_force_step, linear_step);

    let swept_high = numeric.maximum(angular_high, vector_add(angular_high, predicted_linear))?;
    let swept_low = numeric.minimum(angular_low, vector_add(angular_low, predicted_linear))?;

    let center = vector_mul_scalar(vector_add(swept_high, swept_low), 0.5);
    let radius = vector_sub(swept_high, center);
    let expanded_radius = vector_mul_scalar(radius, PART_BBOX_FINAL_EXPANSION);
    Ok(AabbWords::from_f32(
        vector_sub(center, expanded_radius),
        vector_add(center, expanded_radius),
    ))
}

/// Disjoint safe-host form of `CalculatePartBBoxUnion`.
///
/// Native initializes the output to positive zero, accepts any nonzero valid
/// byte, copies the first valid box bit-for-bit, and reduces later boxes in
/// source order. Its output and list pointers are not annotated non-aliasing;
/// this typed slice intentionally does not claim that overlapping pointer case.
pub(super) fn try_union_part_bboxes<N: SerialisedAssemblyBBoxMinMaxNumeric>(
    entries: &[(AabbWords, u8)],
    numeric: &mut N,
) -> Result<AabbWords, N::Error> {
    let mut result = AabbWords::from_f32([0.0; 4], [0.0; 4]);
    let mut has_valid = false;
    for &(candidate, valid) in entries {
        if valid == 0 {
            continue;
        }
        if !has_valid {
            result = candidate;
            has_valid = true;
            continue;
        }
        let minimum = numeric.minimum(result.min_f32(), candidate.min_f32())?;
        let maximum = numeric.maximum(result.max_f32(), candidate.max_f32())?;
        result = AabbWords::from_f32(minimum, maximum);
    }
    Ok(result)
}

fn refine_reciprocal_square_root(estimate: f32, squared: f32) -> f32 {
    let estimate_squared = estimate * estimate;
    let half_estimate = estimate * 0.5;
    let residual = (-squared).mul_add(estimate_squared, 1.0);
    half_estimate.mul_add(residual, estimate)
}

fn splat(value: f32) -> [f32; 4] {
    [value; 4]
}

fn affine_column(transform: [f32; 16], column: usize) -> [f32; 4] {
    std::array::from_fn(|lane| transform[column * 4 + lane])
}

fn transform_affine_point(transform: [f32; 16], point: [f32; 4]) -> [f32; 4] {
    let transformed_x = vector_fused_multiply_add(
        splat(point[0]),
        affine_column(transform, 0),
        affine_column(transform, 3),
    );
    let transformed_xy =
        vector_fused_multiply_add(splat(point[1]), affine_column(transform, 1), transformed_x);
    vector_fused_multiply_add(splat(point[2]), affine_column(transform, 2), transformed_xy)
}

fn transform_triangle_points(transform: [f32; 16], points: [[f32; 4]; 3]) -> [[f32; 4]; 3] {
    let transformed_x = points.map(|point| {
        vector_fused_multiply_add(
            splat(point[0]),
            affine_column(transform, 0),
            affine_column(transform, 3),
        )
    });
    let transformed_xy: [[f32; 4]; 3] = std::array::from_fn(|index| {
        vector_fused_multiply_add(
            splat(points[index][1]),
            affine_column(transform, 1),
            transformed_x[index],
        )
    });
    std::array::from_fn(|index| {
        vector_fused_multiply_add(
            splat(points[index][2]),
            affine_column(transform, 2),
            transformed_xy[index],
        )
    })
}

fn transform_box_frame(transform: [f32; 16], frame: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let transform_x = affine_column(transform, 0);
    let transform_y = affine_column(transform, 1);
    let transform_z = affine_column(transform, 2);
    let transform_center = affine_column(transform, 3);

    let x_zero = vector_mul(splat(frame[0][0]), transform_x);
    let y_zero = vector_mul(splat(frame[1][0]), transform_x);
    let center_zero = vector_fused_multiply_add(splat(frame[3][0]), transform_x, transform_center);
    let z_zero = vector_mul(splat(frame[2][0]), transform_x);
    let x_one = vector_fused_multiply_add(splat(frame[0][1]), transform_y, x_zero);
    let y_one = vector_fused_multiply_add(splat(frame[1][1]), transform_y, y_zero);
    let center_one = vector_fused_multiply_add(splat(frame[3][1]), transform_y, center_zero);
    let z_one = vector_fused_multiply_add(splat(frame[2][1]), transform_y, z_zero);
    let x_axis = vector_fused_multiply_add(splat(frame[0][2]), transform_z, x_one);
    let y_axis = vector_fused_multiply_add(splat(frame[1][2]), transform_z, y_one);
    let center = vector_fused_multiply_add(splat(frame[3][2]), transform_z, center_one);
    let z_axis = vector_fused_multiply_add(splat(frame[2][2]), transform_z, z_one);
    [x_axis, y_axis, z_axis, center]
}

fn transform_capsule_frame(
    transform: [f32; 16],
    axis: [f32; 4],
    center: [f32; 4],
) -> ([f32; 4], [f32; 4]) {
    let transform_x = affine_column(transform, 0);
    let transform_y = affine_column(transform, 1);
    let transform_z = affine_column(transform, 2);
    let center_zero =
        vector_fused_multiply_add(splat(center[0]), transform_x, affine_column(transform, 3));
    let axis_zero = vector_mul(splat(axis[0]), transform_x);
    let center_one = vector_fused_multiply_add(splat(center[1]), transform_y, center_zero);
    let axis_one = vector_fused_multiply_add(splat(axis[1]), transform_y, axis_zero);
    let transformed_center = vector_fused_multiply_add(splat(center[2]), transform_z, center_one);
    let transformed_axis = vector_fused_multiply_add(splat(axis[2]), transform_z, axis_one);
    (transformed_axis, transformed_center)
}

fn transform_cylinder_frame(
    transform: [f32; 16],
    axis: [f32; 4],
    center: [f32; 4],
) -> ([f32; 4], [f32; 4]) {
    let transform_x = affine_column(transform, 0);
    let transform_y = affine_column(transform, 1);
    let transform_z = affine_column(transform, 2);
    let axis_zero = vector_mul(splat(axis[0]), transform_x);
    let center_zero =
        vector_fused_multiply_add(splat(center[0]), transform_x, affine_column(transform, 3));
    let axis_one = vector_fused_multiply_add(splat(axis[1]), transform_y, axis_zero);
    let center_one = vector_fused_multiply_add(splat(center[1]), transform_y, center_zero);
    let transformed_axis = vector_fused_multiply_add(splat(axis[2]), transform_z, axis_one);
    let transformed_center = vector_fused_multiply_add(splat(center[2]), transform_z, center_one);
    (transformed_axis, transformed_center)
}

fn absolute_bits(value: [f32; 4]) -> [f32; 4] {
    value.map(|lane| f32::from_bits(lane.to_bits() & FLOAT_ABSOLUTE_MASK))
}

fn absolute_scalar_bits(value: f32) -> f32 {
    f32::from_bits(value.to_bits() & FLOAT_ABSOLUTE_MASK)
}

fn vector_add(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] + right[lane])
}

fn vector_mul(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] * right[lane])
}

fn vector_sub(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| left[lane] - right[lane])
}

fn vector_mul_scalar(value: [f32; 4], scalar: f32) -> [f32; 4] {
    value.map(|lane| lane * scalar)
}

fn vector_fused_multiply_add(
    multiplier: [f32; 4],
    multiplicand: [f32; 4],
    addend: [f32; 4],
) -> [f32; 4] {
    std::array::from_fn(|lane| multiplier[lane].mul_add(multiplicand[lane], addend[lane]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;

    #[derive(Clone, Debug, PartialEq)]
    enum NumericCall {
        Minimum([[u32; 4]; 2]),
        Maximum([[u32; 4]; 2]),
        SumThreeProducts([[u32; 4]; 2]),
        ReciprocalSquareRootEstimate(u32),
    }

    #[derive(Default)]
    struct TraceNumeric {
        calls: Vec<NumericCall>,
        forced_sums: Vec<f32>,
        next_forced_sum: usize,
        forced_estimate: Option<f32>,
    }

    impl SerialisedAssemblyBBoxMinMaxNumeric for TraceNumeric {
        type Error = Infallible;

        fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
            self.calls.push(NumericCall::Minimum([
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ]));
            Ok(std::array::from_fn(|lane| {
                if left[lane] < right[lane] {
                    left[lane]
                } else {
                    right[lane]
                }
            }))
        }

        fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
            self.calls.push(NumericCall::Maximum([
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ]));
            Ok(std::array::from_fn(|lane| {
                if left[lane] > right[lane] {
                    left[lane]
                } else {
                    right[lane]
                }
            }))
        }
    }

    impl SerialisedAssemblyPartBBoxNumeric for TraceNumeric {
        fn sum_three_products(
            &mut self,
            left: [f32; 4],
            right: [f32; 4],
        ) -> Result<f32, Self::Error> {
            self.calls.push(NumericCall::SumThreeProducts([
                left.map(f32::to_bits),
                right.map(f32::to_bits),
            ]));
            let result = self
                .forced_sums
                .get(self.next_forced_sum)
                .copied()
                .unwrap_or_else(|| (left[0] * right[0] + left[1] * right[1]) + left[2] * right[2]);
            self.next_forced_sum += 1;
            Ok(result)
        }

        fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            self.calls
                .push(NumericCall::ReciprocalSquareRootEstimate(value.to_bits()));
            Ok(self.forced_estimate.unwrap_or_else(|| value.sqrt().recip()))
        }
    }

    fn zero_motion() -> SerialisedAssemblyBodyMotion {
        SerialisedAssemblyBodyMotion {
            velocity_and_right: [0.0; 4],
            omega_and_left: [0.0; 4],
            force_and_kine: [0.0; 4],
            torque_and_cool: [0.0; 4],
        }
    }

    fn volume(volume_type: VolumeType) -> Volume {
        Volume {
            transform: [0.0; 16],
            vtable_offset: volume_type as u32,
            shape_words: [0; 3],
            radius: 0.0,
            group_id: 0,
            surface_id: 0,
            flags: 0,
        }
    }

    #[test]
    fn host_numeric_minmax_compares_all_four_lanes() {
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let left = [1.0, -4.0, 5.0, -9.0];
        let right = [2.0, -3.0, 4.0, 7.0];
        assert_eq!(
            numeric.minimum(left, right).unwrap().map(f32::to_bits),
            [1.0_f32, -4.0, 4.0, -9.0].map(f32::to_bits)
        );
        assert_eq!(
            numeric.maximum(left, right).unwrap().map(f32::to_bits),
            [2.0_f32, -3.0, 5.0, 7.0].map(f32::to_bits)
        );
    }

    #[test]
    fn host_numeric_right_operand_tie_and_unordered_policy_is_explicit() {
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let nan = f32::from_bits(0x7fc1_2345);
        let left = [0.0, -0.0, nan, 3.0];
        let right = [-0.0, 0.0, 3.0, nan];
        for selected in [
            numeric.minimum(left, right).unwrap(),
            numeric.maximum(left, right).unwrap(),
        ] {
            assert_eq!(selected.map(f32::to_bits), right.map(f32::to_bits));
        }
    }

    #[test]
    fn host_numeric_dot_ignores_fourth_lane_and_freezes_host_rounding_policy() {
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let poison = f32::from_bits(0x7fc1_2345);
        let geometric = numeric
            .sum_three_products([2.0, 3.0, 4.0, poison], [5.0, 6.0, 7.0, poison])
            .unwrap();
        assert_eq!(geometric.to_bits(), 56.0_f32.to_bits());

        // Separately rounded products cancel; fusing the first product with
        // the second addend would leave the representable residue 2^-46.
        let factor = f32::from_bits(0x3f80_0001);
        let cancellation = -f32::from_bits(0x3f80_0002);
        let split = numeric
            .sum_three_products(
                [factor, cancellation, 0.0, poison],
                [factor, 1.0, 1.0, poison],
            )
            .unwrap();
        assert_eq!(split.to_bits(), 0);
        assert_eq!(factor.mul_add(factor, cancellation).to_bits(), 0x2880_0000);

        // This distinguishes (x+y)+z from x+(y+z).
        let associated = numeric
            .sum_three_products(
                [16_777_216.0, 1.0, -16_777_216.0, poison],
                [1.0, 1.0, 1.0, poison],
            )
            .unwrap();
        assert_eq!(associated.to_bits(), 0);
    }

    #[test]
    fn host_numeric_seed_keeps_zero_and_invalid_arithmetic_without_sanitizing() {
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        for (value, expected) in [
            (4.0, 0.5_f32),
            (0.0, f32::INFINITY),
            (-0.0, f32::NEG_INFINITY),
            (f32::INFINITY, 0.0),
        ] {
            assert_eq!(
                numeric
                    .reciprocal_square_root_estimate(value)
                    .unwrap()
                    .to_bits(),
                expected.to_bits()
            );
        }
        assert!(
            numeric
                .reciprocal_square_root_estimate(-4.0)
                .unwrap()
                .is_nan()
        );
    }

    #[test]
    fn host_numeric_runs_complete_sphere_and_cylinder_bounds_with_zero_motion() {
        // Translation W is zero here so every lane of the stationary sphere
        // has the same expected extent; it is an explicit input, not a fixup.
        let transform = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let mut sphere = volume(VolumeType::Sphere);
        sphere.radius = 1.0;
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let sphere_bounds =
            try_calculate_primitive_part_bbox(&sphere, &transform, zero_motion(), &mut numeric)
                .unwrap()
                .unwrap();
        assert_eq!(sphere_bounds, AabbWords::from_f32([-1.05; 4], [1.05; 4]));

        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8] = 1.0;
        cylinder.shape_words[0] = 2.5_f32.to_bits();
        cylinder.shape_words[1] = 1.5_f32.to_bits();
        cylinder.radius = 0.5;
        let cylinder_bounds =
            try_calculate_primitive_part_bbox(&cylinder, &transform, zero_motion(), &mut numeric)
                .unwrap()
                .unwrap();
        assert_eq!(
            cylinder_bounds,
            AabbWords::from_f32(
                [-3.149_999_9, -2.1, -2.1, -1.05],
                [3.149_999_9, 2.1, 2.1, 1.05]
            )
        );
    }

    #[test]
    fn host_numeric_motion_preserves_adjacent_words_through_four_lane_sweep() {
        let shape = AabbWords::from_f32([-1.0, -2.0, -3.0, -4.0], [1.0, 2.0, 3.0, 4.0]);
        let mut numeric = HostSerialisedAssemblyBBoxNumeric;
        let mut body = zero_motion();
        body.velocity_and_right[0] = 60.0;
        body.force_and_kine[0] = 3_600.0;
        let baseline = try_expand_part_bbox_after_shape(shape, body, &mut numeric).unwrap();

        // Angular W must not contaminate the three-product reductions.
        body.omega_and_left[3] = 120.0;
        body.torque_and_cool[3] = 3_600.0;
        assert_eq!(
            try_expand_part_bbox_after_shape(shape, body, &mut numeric).unwrap(),
            baseline
        );

        // Linear W must reach the sweep, including both velocity and force.
        for (right, kine) in [(60.0, 0.0), (0.0, 3_600.0)] {
            body.velocity_and_right[3] = right;
            body.force_and_kine[3] = kine;
            let moved = try_expand_part_bbox_after_shape(shape, body, &mut numeric).unwrap();
            assert_eq!(moved.min[..3], baseline.min[..3]);
            assert_eq!(moved.max[..3], baseline.max[..3]);
            assert!(f32::from_bits(moved.max[3]) > f32::from_bits(baseline.max[3]));
            assert!(f32::from_bits(moved.min[3]) < f32::from_bits(baseline.min[3]));
        }
    }

    #[test]
    fn sphere_uses_stored_center_or_parent_xyz_fmas_and_expands_every_lane() {
        let mut sphere = volume(VolumeType::Sphere);
        sphere.transform[12..16].copy_from_slice(&[2.0, -3.0, 4.0, -5.0]);
        sphere.radius = 0.5;
        let mut numeric = TraceNumeric::default();

        let stored = try_get_primitive_bbox(&sphere, None, u32::MAX, &mut numeric)
            .unwrap()
            .expect("sphere is supported");
        assert_eq!(
            stored,
            AabbWords::from_f32([1.5, -3.5, 3.5, -5.5], [2.5, -2.5, 4.5, -4.5])
        );

        let parent = [
            2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 43.0, 47.0,
            53.0,
        ];
        let transformed = try_get_primitive_bbox(&sphere, Some(&parent), 0, &mut numeric)
            .unwrap()
            .expect("sphere is supported");
        assert_eq!(
            transformed,
            AabbWords::from_f32([103.5, 125.5, 129.5, 157.5], [104.5, 126.5, 130.5, 158.5])
        );
        assert!(numeric.calls.is_empty());
    }

    #[test]
    fn parent_point_transform_matches_frozen_fused_rounding_words() {
        let from_words = |words: [u32; 4]| words.map(f32::from_bits);
        let parent_columns = [
            [0x3f80_0001, 0x3eaa_aaab, 0xbf00_0001, 0x3f00_0001],
            [0x3f00_0001, 0xbf80_0001, 0x3f40_0001, 0xbeaa_aaab],
            [0xbf40_0001, 0x3f20_0001, 0x3e80_0001, 0x3f80_0001],
            [0x4123_4567, 0xc0ab_cdef, 0x3f99_999a, 0x4000_0001],
        ];
        let parent: [f32; 16] =
            std::array::from_fn(|index| f32::from_bits(parent_columns[index / 4][index % 4]));
        let mut sphere = volume(VolumeType::Sphere);
        sphere.transform[12..16].copy_from_slice(&from_words([
            0x3f20_0001,
            0xbf40_0001,
            0x3e80_0001,
            0x7fc1_2345,
        ]));
        let expected = [0x4124_4567, 0xc088_2344, 0x3ec6_6664, 0x4034_0001];
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&sphere, Some(&parent), 0, &mut numeric)
            .unwrap()
            .expect("sphere is supported");

        assert_eq!(result.min, expected);
        assert_eq!(result.max, expected);
    }

    #[test]
    fn complete_primitive_part_path_gets_shape_before_swept_expansion() {
        let mut sphere = volume(VolumeType::Sphere);
        sphere.transform[12..16].copy_from_slice(&[0.0, 0.0, 0.0, -99.0]);
        sphere.radius = 1.0;
        let identity = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut numeric = TraceNumeric::default();

        let result =
            try_calculate_primitive_part_bbox(&sphere, &identity, zero_motion(), &mut numeric)
                .unwrap()
                .expect("sphere produces a part bbox");

        assert_eq!(
            result,
            AabbWords::from_f32(
                [-1.05, -1.05, -1.05, f32::from_bits(0xbd4c_ccc0)],
                [1.05, 1.05, 1.05, 2.05]
            )
        );
        assert_eq!(
            numeric
                .calls
                .iter()
                .filter(|call| matches!(call, NumericCall::ReciprocalSquareRootEstimate(_)))
                .count(),
            1
        );
    }

    #[test]
    fn triangle_reduces_points_one_two_then_point_zero_before_radius() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform[0..4].copy_from_slice(&[4.0, -1.0, 8.0, -0.0]);
        triangle.transform[4..8].copy_from_slice(&[-2.0, 6.0, 3.0, 10.0]);
        triangle.transform[8..12].copy_from_slice(&[1.0, 2.0, -5.0, 7.0]);
        triangle.radius = 0.25;
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&triangle, None, 9, &mut numeric)
            .unwrap()
            .expect("triangle is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([-2.25, -1.25, -5.25, -0.25], [4.25, 6.25, 8.25, 10.25])
        );
        let point_zero = [4.0f32, -1.0, 8.0, -0.0];
        let point_one = [-2.0f32, 6.0, 3.0, 10.0];
        let point_two = [1.0f32, 2.0, -5.0, 7.0];
        assert_eq!(
            numeric.calls,
            vec![
                NumericCall::Minimum([point_one.map(f32::to_bits), point_two.map(f32::to_bits)]),
                NumericCall::Maximum([point_one.map(f32::to_bits), point_two.map(f32::to_bits)]),
                NumericCall::Minimum([
                    point_zero.map(f32::to_bits),
                    [-2.0f32, 2.0, -5.0, 7.0].map(f32::to_bits),
                ]),
                NumericCall::Maximum([
                    point_zero.map(f32::to_bits),
                    [1.0f32, 6.0, 3.0, 10.0].map(f32::to_bits),
                ]),
            ]
        );
    }

    #[test]
    fn triangle_parent_transform_uses_xyz_fmas_for_all_four_lanes() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform[0..4].copy_from_slice(&[1.0, 2.0, 3.0, -100.0]);
        triangle.transform[4..8].copy_from_slice(&[-2.0, 4.0, -1.0, -200.0]);
        triangle.transform[8..12].copy_from_slice(&[5.0, -3.0, 2.0, -300.0]);
        let parent = [
            2.0, 0.0, 0.0, 5.0, 0.0, 3.0, 0.0, 11.0, 0.0, 0.0, 4.0, 13.0, 10.0, 20.0, 30.0, 17.0,
        ];
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&triangle, Some(&parent), 1, &mut numeric)
            .unwrap()
            .expect("triangle is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([6.0, 11.0, 26.0, 35.0], [20.0, 32.0, 42.0, 83.0])
        );
        assert_eq!(
            numeric.calls[0],
            NumericCall::Minimum([
                [6.0f32, 32.0, 26.0, 38.0].map(f32::to_bits),
                [20.0f32, 11.0, 38.0, 35.0].map(f32::to_bits),
            ])
        );
    }

    #[test]
    fn box_uses_y_x_z_radius_extent_order_and_preserves_w() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.transform[0..4].copy_from_slice(&[1.0, -2.0, -0.0, 4.0]);
        box_volume.transform[4..8].copy_from_slice(&[-5.0, 6.0, 7.0, -8.0]);
        box_volume.transform[8..12].copy_from_slice(&[9.0, -10.0, 11.0, 12.0]);
        box_volume.transform[12..16].copy_from_slice(&[100.0, 200.0, 300.0, 400.0]);
        box_volume.shape_words = [2.0f32.to_bits(), 3.0f32.to_bits(), 4.0f32.to_bits()];
        box_volume.radius = 0.5;
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&box_volume, None, 0, &mut numeric)
            .unwrap()
            .expect("box is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([46.5, 137.5, 234.5, 319.5], [153.5, 262.5, 365.5, 480.5])
        );
        assert_eq!(
            try_get_primitive_bbox(&box_volume, None, 1, &mut numeric).unwrap(),
            Some(result)
        );
        assert_eq!(
            try_get_primitive_bbox(&box_volume, None, u32::MAX, &mut numeric).unwrap(),
            Some(result)
        );
        assert!(numeric.calls.is_empty());
    }

    #[test]
    fn box_fuses_x_term_after_y_and_adds_radius_separately() {
        let factor = f32::from_bits(0x3f80_0001);
        let cancelled = -f32::from_bits(0x3f80_0002);
        let fused_residue = f32::from_bits(0x2880_0000);
        let mut box_volume = volume(VolumeType::Box);
        box_volume.transform[0] = factor;
        box_volume.transform[4] = 1.0;
        box_volume.shape_words = [factor.to_bits(), cancelled.to_bits(), 0.0f32.to_bits()];
        let mut numeric = TraceNumeric::default();

        let fused = try_get_primitive_bbox(&box_volume, None, 0, &mut numeric)
            .unwrap()
            .expect("box is supported");
        assert_eq!(
            fused,
            AabbWords::from_f32(
                [-fused_residue, 0.0, 0.0, 0.0],
                [fused_residue, 0.0, 0.0, 0.0]
            )
        );

        box_volume.transform[0] = 0.0;
        box_volume.transform[4] = 0.0;
        box_volume.transform[8..12].fill(factor);
        box_volume.shape_words[2] = factor.to_bits();
        box_volume.radius = cancelled;
        let separate_radius = try_get_primitive_bbox(&box_volume, None, 0, &mut numeric)
            .unwrap()
            .expect("box is supported");
        assert_eq!(separate_radius, AabbWords::from_f32([0.0; 4], [0.0; 4]));
    }

    #[test]
    fn box_parent_transform_ignores_stored_w_coefficients_but_publishes_parent_w() {
        let poison = f32::from_bits(0x7fc1_2345);
        let mut box_volume = volume(VolumeType::Box);
        box_volume.transform[0..4].copy_from_slice(&[1.0, 0.0, 0.0, poison]);
        box_volume.transform[4..8].copy_from_slice(&[0.0, 1.0, 0.0, poison]);
        box_volume.transform[8..12].copy_from_slice(&[0.0, 0.0, 1.0, poison]);
        box_volume.transform[12..16].copy_from_slice(&[1.0, 2.0, 3.0, poison]);
        box_volume.shape_words = [1.0f32.to_bits(); 3];
        let parent = [
            2.0, 0.0, 0.0, 5.0, 0.0, 3.0, 0.0, 7.0, 0.0, 0.0, 4.0, 11.0, 10.0, 20.0, 30.0, 13.0,
        ];
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&box_volume, Some(&parent), 0, &mut numeric)
            .unwrap()
            .expect("box is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([10.0, 23.0, 38.0, 42.0], [14.0, 29.0, 46.0, 88.0])
        );
    }

    #[test]
    fn capsule_uses_only_stored_z_axis_and_fused_half_height_radius_extent() {
        let mut capsule = volume(VolumeType::Capsule);
        capsule.transform[0..8].fill(f32::from_bits(0x7fc1_2345));
        capsule.transform[8..12].copy_from_slice(&[1.0, -2.0, -0.0, 4.0]);
        capsule.transform[12..16].copy_from_slice(&[10.0, 20.0, 30.0, 40.0]);
        capsule.shape_words[0] = 3.0f32.to_bits();
        capsule.radius = 0.5;
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&capsule, None, 0, &mut numeric)
            .unwrap()
            .expect("capsule is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([6.5, 13.5, 29.5, 27.5], [13.5, 26.5, 30.5, 52.5])
        );
        assert_eq!(
            try_get_primitive_bbox(&capsule, None, 1, &mut numeric).unwrap(),
            Some(result)
        );
        assert_eq!(
            try_get_primitive_bbox(&capsule, None, u32::MAX, &mut numeric).unwrap(),
            Some(result)
        );
        assert!(numeric.calls.is_empty());
    }

    #[test]
    fn capsule_half_height_radius_is_one_fused_operation() {
        let factor = f32::from_bits(0x3f80_0001);
        let cancelled = -f32::from_bits(0x3f80_0002);
        let fused_residue = f32::from_bits(0x2880_0000);
        let mut capsule = volume(VolumeType::Capsule);
        capsule.transform[8..12].fill(factor);
        capsule.shape_words[0] = factor.to_bits();
        capsule.radius = cancelled;
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&capsule, None, 0, &mut numeric)
            .unwrap()
            .expect("capsule is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([-fused_residue; 4], [fused_residue; 4])
        );
    }

    #[test]
    fn cylinder_tight_uses_xyz_estimates_and_native_w_extent() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8..12].copy_from_slice(&[1.0, 0.0, 0.0, 0.0]);
        cylinder.transform[12..16].copy_from_slice(&[10.0, 20.0, 30.0, 40.0]);
        cylinder.shape_words[0] = 2.5f32.to_bits();
        cylinder.shape_words[1] = 1.5f32.to_bits();
        cylinder.radius = 0.5;
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&cylinder, None, 2, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([7.0, 18.0, 28.0, 39.0], [13.0, 22.0, 32.0, 41.0])
        );
        assert_eq!(
            numeric
                .calls
                .iter()
                .filter_map(|call| match call {
                    NumericCall::ReciprocalSquareRootEstimate(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [0.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits()]
        );
    }

    #[test]
    fn cylinder_loose_uses_one_diagonal_estimate_and_zero_w_extent() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[12..16].copy_from_slice(&[10.0, 20.0, 30.0, 40.0]);
        cylinder.shape_words[0] = 2.0f32.to_bits();
        let mut numeric = TraceNumeric {
            forced_estimate: Some(0.5),
            ..TraceNumeric::default()
        };

        let result = try_get_primitive_bbox(&cylinder, None, 0, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([8.0, 18.0, 28.0, 40.0], [12.0, 22.0, 32.0, 40.0])
        );
        assert_eq!(
            numeric.calls,
            vec![NumericCall::ReciprocalSquareRootEstimate(4.0f32.to_bits())]
        );
    }

    #[test]
    fn cylinder_parent_transform_and_tight_w_match_native_axis_x_square() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8..12].copy_from_slice(&[0.0, 0.0, 1.0, -99.0]);
        cylinder.transform[12..16].copy_from_slice(&[2.0, 3.0, 4.0, -77.0]);
        cylinder.shape_words[0] = 1.0f32.to_bits();
        let parent = [
            0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0, 1.0, 0.0, 0.0, 4.0, 10.0, 20.0, 30.0, 5.0,
        ];
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&cylinder, Some(&parent), 1, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([13.0, 22.0, 33.0, 33.0], [15.0, 22.0, 33.0, 35.0])
        );
    }

    #[test]
    fn cylinder_tight_rsqrt_order_uses_absolute_one_minus_axis_square() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8..12].copy_from_slice(&[0.0, 0.5, 2.0, 0.0]);
        let mut numeric = TraceNumeric {
            forced_estimate: Some(0.9),
            ..TraceNumeric::default()
        };

        let _ = try_get_primitive_bbox(&cylinder, None, u32::MAX, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            numeric
                .calls
                .iter()
                .filter_map(|call| match call {
                    NumericCall::ReciprocalSquareRootEstimate(value) => Some(*value),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [1.0f32.to_bits(), 0.75f32.to_bits(), 3.0f32.to_bits()]
        );
    }

    #[test]
    fn cylinder_nonideal_seed_and_fractional_x_square_are_frozen() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8..12].copy_from_slice(&[0.5, 0.0, 0.0, 0.0]);
        cylinder.transform[15] = 7.0;
        cylinder.shape_words[1] = 1.0f32.to_bits();
        let mut numeric = TraceNumeric {
            forced_estimate: Some(0.9),
            ..TraceNumeric::default()
        };

        let tight = try_get_primitive_bbox(&cylinder, None, 1, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(tight.max[0], 0x3f5c_3766);
        assert_eq!(tight.min[0], 0xbf5c_3766);
        assert_eq!(tight.min[3], 6.75f32.to_bits());
        assert_eq!(tight.max[3], 7.25f32.to_bits());

        let mut loose_numeric = TraceNumeric {
            forced_estimate: Some(0.9),
            ..TraceNumeric::default()
        };
        let loose = try_get_primitive_bbox(&cylinder, None, 0, &mut loose_numeric)
            .unwrap()
            .expect("cylinder is supported");
        assert_eq!(loose.min[3], 7.0f32.to_bits());
        assert_eq!(loose.max[3], 7.0f32.to_bits());
    }

    #[test]
    fn cylinder_loose_rounds_radius_sums_before_square_and_fma() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.shape_words[0] = (-1.0f32).to_bits();
        cylinder.shape_words[1] = (-1.0f32).to_bits();
        cylinder.radius = f32::from_bits(0x3f80_0001);
        let mut numeric = TraceNumeric {
            forced_estimate: Some(1.0),
            ..TraceNumeric::default()
        };

        let _ = try_get_primitive_bbox(&cylinder, None, 0, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            numeric.calls,
            vec![NumericCall::ReciprocalSquareRootEstimate(0x2900_0000)]
        );
    }

    #[test]
    fn cylinder_negative_dimensions_are_not_sorted_or_clamped() {
        let mut cylinder = volume(VolumeType::Cylinder);
        cylinder.transform[8] = 1.0;
        cylinder.shape_words[0] = (-2.0f32).to_bits();
        cylinder.shape_words[1] = (-1.0f32).to_bits();
        let mut numeric = TraceNumeric::default();

        let result = try_get_primitive_bbox(&cylinder, None, 1, &mut numeric)
            .unwrap()
            .expect("cylinder is supported");

        assert_eq!(
            result,
            AabbWords::from_f32([2.0, 1.0, 1.0, -1.0], [-2.0, -1.0, -1.0, 1.0])
        );
    }

    #[test]
    fn exact_constants_zero_motion_still_estimates_then_expands_all_lanes() {
        assert_eq!(PART_BBOX_TIMESTEP.to_bits(), 0x3c88_8889);
        assert_eq!(PART_BBOX_FINAL_EXPANSION.to_bits(), 0x3f86_6666);
        let input = AabbWords::from_f32([-1.0, -2.0, -3.0, -4.0], [1.0, 2.0, 3.0, 4.0]);
        let mut numeric = TraceNumeric::default();

        let result = try_expand_part_bbox_after_shape(input, zero_motion(), &mut numeric)
            .expect("trace numeric is infallible");

        assert_eq!(
            result,
            AabbWords::from_f32(
                [-1.05, -2.1, -3.149_999_9, -4.2],
                [1.05, 2.1, 3.149_999_9, 4.2]
            )
        );
        assert!(
            numeric
                .calls
                .contains(&NumericCall::ReciprocalSquareRootEstimate(0.0f32.to_bits()))
        );
        assert_eq!(
            numeric
                .calls
                .iter()
                .filter(|call| matches!(call, NumericCall::SumThreeProducts(_)))
                .count(),
            3
        );
    }

    #[test]
    fn prediction_forwards_exact_motion_words_and_clamps_in_native_order() {
        let body = SerialisedAssemblyBodyMotion {
            velocity_and_right: [60.0, -120.0, 180.0, 240.0],
            omega_and_left: [1.0, -2.0, 3.0, -4.0],
            force_and_kine: [300.0, -360.0, 420.0, -480.0],
            torque_and_cool: [60.0, 120.0, -180.0, 240.0],
        };
        let mut numeric = TraceNumeric {
            forced_sums: vec![2.0, 4.0, -2.0],
            forced_estimate: Some(0.5),
            ..TraceNumeric::default()
        };
        let input = AabbWords::from_f32([-1.0; 4], [1.0; 4]);

        let _ = try_expand_part_bbox_after_shape(input, body, &mut numeric)
            .expect("trace numeric is infallible");

        let sums = numeric
            .calls
            .iter()
            .filter_map(|call| match call {
                NumericCall::SumThreeProducts(operands) => Some(*operands),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(sums.len(), 3);
        assert_eq!(
            sums[0][0],
            body.omega_and_left
                .map(|lane| (lane * PART_BBOX_TIMESTEP).to_bits())
        );
        assert_eq!(
            sums[0][1],
            body.torque_and_cool
                .map(|lane| { ((lane * PART_BBOX_TIMESTEP) * PART_BBOX_TIMESTEP).to_bits() })
        );
        assert_eq!(
            sums[2][0],
            body.velocity_and_right
                .map(|lane| (lane * PART_BBOX_TIMESTEP).to_bits())
        );
        assert_eq!(
            sums[2][1],
            body.force_and_kine
                .map(|lane| { ((lane * PART_BBOX_TIMESTEP) * PART_BBOX_TIMESTEP).to_bits() })
        );

        let angular_clamp_max = numeric
            .calls
            .iter()
            .position(|call| match call {
                NumericCall::Maximum(operands) => {
                    operands[0] == [0; 4] && operands[1] == [2.0f32.to_bits(); 4]
                }
                _ => false,
            })
            .expect("native nonnegative angular clamp call");
        let angular_clamp_min = numeric
            .calls
            .iter()
            .position(|call| match call {
                NumericCall::Minimum(operands) => {
                    operands[0] == [1.0f32.to_bits(); 4] && operands[1] == [2.0f32.to_bits(); 4]
                }
                _ => false,
            })
            .expect("native upper angular clamp call");
        assert!(angular_clamp_max < angular_clamp_min);
    }

    #[test]
    fn union_zero_all_invalid_and_first_valid_copy_are_bit_exact() {
        let poisoned = AabbWords {
            min: [0x7fc1_1111; 4],
            max: [0xffc2_2222; 4],
        };
        let first = AabbWords {
            min: [0x8000_0000, 1, 2, 3],
            max: [0x7fc1_2345, 4, 5, 6],
        };

        let mut numeric = TraceNumeric::default();
        assert_eq!(
            try_union_part_bboxes(&[], &mut numeric).unwrap(),
            AabbWords {
                min: [0; 4],
                max: [0; 4]
            }
        );
        assert_eq!(
            try_union_part_bboxes(&[(poisoned, 0), (first, 0)], &mut numeric).unwrap(),
            AabbWords {
                min: [0; 4],
                max: [0; 4]
            }
        );
        assert_eq!(
            try_union_part_bboxes(&[(poisoned, 0), (first, 0x80)], &mut numeric).unwrap(),
            first
        );
        assert!(numeric.calls.is_empty());
    }

    #[test]
    fn union_reduces_only_after_first_valid_in_source_order() {
        let first = AabbWords::from_f32([-1.0, -2.0, -3.0, -4.0], [1.0, 2.0, 3.0, 4.0]);
        let ignored = AabbWords::from_f32([-100.0; 4], [100.0; 4]);
        let second = AabbWords::from_f32([-5.0, -1.0, -7.0, -2.0], [6.0, 1.0, 8.0, 2.0]);
        let mut numeric = TraceNumeric::default();

        let result = try_union_part_bboxes(
            &[(ignored, 0), (first, 1), (ignored, 0), (second, 2)],
            &mut numeric,
        )
        .unwrap();

        assert_eq!(
            result,
            AabbWords::from_f32([-5.0, -2.0, -7.0, -4.0], [6.0, 2.0, 8.0, 4.0])
        );
        assert_eq!(numeric.calls.len(), 2);
        assert_eq!(
            numeric.calls[0],
            NumericCall::Minimum([first.min, second.min])
        );
        assert_eq!(
            numeric.calls[1],
            NumericCall::Maximum([first.max, second.max])
        );
    }
}
