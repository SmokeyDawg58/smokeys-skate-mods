//! `SkeletonIK::CalculateExternalIKOffset`, 0x82b48d10..0x82b49534.
//! External preparation mutates the existing state before common initial
//! transforms. Full pivot compositions and partial W lanes are retained.

use super::{
    SkeletonIkState,
    initial::{SkeletonIkInitialCvars, SkeletonIkRecordRead},
};
use crate::{
    animation::AnimationMatrix,
    reckoning_numeric::{
        ReckoningClampError, ReckoningNumeric, cross_full4, try_asin, try_clamp, try_length,
    },
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skateboard_spawn_math::skateboard_spawn_rigid_inverse,
    skeleton_ik::SkeletonIkDataIn,
    skeleton_ik_targets::SkeletonIkTargets,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[cfg(test)]
#[path = "skeleton_ik_external_tests.rs"]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkExternalError<C, N, R> {
    InvalidEntry { entry: usize },
    MissingPrimaryPart { entry: usize },
    RecordUnavailable(SkeletonIkRecordRead),
    InvalidMode { entry: usize, mode: u32 },
    Cvar(C),
    Numeric(N),
    Trigonometry(R),
    Clamp(ReckoningClampError<N>),
}

impl SkeletonIkState {
    #[expect(
        clippy::too_many_lines,
        reason = "ordered external target update and failure prefixes"
    )]
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn try_calculate_external_offset<
        C: SkeletonIkInitialCvars,
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        entry: usize,
        data: &SkeletonIkDataIn,
        targets: &SkeletonIkTargets,
        previous: &[AnimationMatrix],
        cvars: &mut C,
        numeric: &mut N,
        trigonometry: &mut R,
    ) -> Result<(), SkeletonIkExternalError<C::Error, N::Error, R::Error>> {
        use SkeletonIkExternalError as Error;
        if entry >= 4 {
            return Err(Error::InvalidEntry { entry });
        }
        let part = targets.parts()[entry].ok_or(Error::MissingPrimaryPart { entry })?;
        let original = previous.get(part.index()).ok_or(Error::RecordUnavailable(
            SkeletonIkRecordRead::Primary {
                entry,
                part: part.index(),
            },
        ))?;
        let animated = SkeletonRootMatrix::known(*original);
        let world = data.animation_to_world.compose(&animated);
        let mut rotation = SkeletonRootMatrix::known(super::IDENTITY);
        let blend = self.external_target_normal_blends[entry];
        if blend > 0.0 {
            let axis = cross(
                SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                self.external_target_normals[entry],
            );
            let length = try_length(scratch(axis), numeric).map_err(Error::Numeric)?;
            if length > f32::from_bits(0x3d4c_cccd) {
                let angle = try_asin(length, numeric).map_err(Error::Numeric)?;
                let estimate = numeric
                    .reciprocal_estimate(length)
                    .map_err(Error::Numeric)?;
                let maximum = cvars.maximum_external_normal_angle().map_err(Error::Cvar)?;
                let inverse = refine_drive_reciprocal(length, estimate);
                let lower = if -maximum - angle >= 0.0 {
                    -maximum
                } else {
                    angle
                };
                let bounded = if maximum - lower >= 0.0 {
                    lower
                } else {
                    maximum
                };
                let unit = axis.xyz.map(|component| inverse * component);
                let (sine, cosine) =
                    try_sin_cos(bounded * blend, trigonometry).map_err(Error::Trigonometry)?;
                rotation = normal_rotation(unit, sine, cosine);
            }
            if !self.external_target_normal_set[entry] {
                let speed = cvars.target_normal_blend_speed().map_err(Error::Cvar)?;
                let decayed = blend - speed;
                self.external_target_normal_blends[entry] = decayed;
                self.external_target_normal_blends[entry] =
                    if -decayed >= 0.0 { 0.0 } else { decayed };
            }
        }
        if self.modes[entry] == 3 {
            let blend = self.target_blends[entry];
            let delta = self.external_target_local_deltas[entry];
            let difference = subtract(
                self.external_target_positions_local[entry],
                animated.columns()[3],
            );
            let blended = binary(difference, delta, |new, old| {
                new.mul_add(blend, old * (1.0 - blend))
            });
            let step = subtract(blended, delta);
            let maximum = cvars.maximum_local_delta().map_err(Error::Cvar)?;
            let clamped = clamp(step, maximum, numeric).map_err(Error::Clamp)?;
            let local = add(animated.columns()[3], blended);
            self.external_target_local_deltas[entry] = add(delta, clamped);
            self.external_target_positions_local[entry] = local;
            self.external_target_positions_world[entry] =
                transform_point(&data.animation_to_world, local);
        } else {
            let mode = self.modes[entry];
            if mode != 2 {
                return Err(Error::InvalidMode { entry, mode });
            }
            self.external_target_positions_local[entry] = transform_point(
                &data.world_to_animation,
                self.external_target_positions_world[entry],
            );
        }
        let displacement = subtract(
            self.external_target_positions_world[entry],
            world.columns()[3],
        );
        let maximum = cvars.maximum_world_offset().map_err(Error::Cvar)?;
        let offset = clamp(displacement, maximum, numeric).map_err(Error::Clamp)?;
        let position = add(world.columns()[3], offset);
        let negative = map(world.columns()[3], |value| {
            f32::from_bits(value.to_bits() ^ 0x8000_0000)
        });
        let pivot = rotation.compose(&SkeletonRootMatrix::translation(negative));
        let moved = SkeletonRootMatrix::translation(position).compose(&pivot);
        let external = moved.compose(&world);
        self.part_to_world_external[entry] = external;
        if let Some(parent) = targets.parents()[entry] {
            let parent_row = previous
                .get(parent.index())
                .ok_or(Error::RecordUnavailable(SkeletonIkRecordRead::Parent {
                    entry,
                    part: parent.index(),
                }))?;
            let inverse = SkeletonRootMatrix::known(skateboard_spawn_rigid_inverse(original));
            let relation = inverse.compose(&SkeletonRootMatrix::known(*parent_row));
            self.parent_to_world_external[entry] = external.compose(&relation);
        }
        Ok(())
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
fn map(value: SkeletonRootVector, operation: impl Fn(f32) -> f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: value.xyz.map(&operation),
        w: value.w.map(operation),
    }
}
fn binary(
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    operation: impl Fn(f32, f32) -> f32,
) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|axis| operation(a.xyz[axis], b.xyz[axis])),
        w: a.w.zip(b.w).map(|(a, b)| operation(a, b)),
    }
}
fn add(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    binary(a, b, |a, b| a + b)
}
fn subtract(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    binary(a, b, |a, b| a - b)
}
fn cross(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    let mut result = SkeletonRootVector::known(cross_full4(scratch(a), scratch(b)));
    if a.w.is_none() || b.w.is_none() {
        result.w = None;
    }
    result
}
fn clamp<N: ReckoningNumeric>(
    value: SkeletonRootVector,
    maximum: f32,
    numeric: &mut N,
) -> Result<SkeletonRootVector, ReckoningClampError<N::Error>> {
    let mut result = SkeletonRootVector::known(try_clamp(scratch(value), maximum, numeric)?);
    if value.w.is_none() {
        result.w = None;
    }
    Ok(result)
}
pub(crate) fn transform_point(
    matrix: &SkeletonRootMatrix,
    point: SkeletonRootVector,
) -> SkeletonRootVector {
    let columns = matrix.columns();
    let x = binary(columns[0], columns[3], |x, p| x.mul_add(point.xyz[0], p));
    let xy = binary(columns[1], x, |y, p| y.mul_add(point.xyz[1], p));
    binary(columns[2], xy, |z, p| z.mul_add(point.xyz[2], p))
}

/// 0x82b48fc8..0x82b4903c. The [0,5,0,0] permutations make W=X.
pub(crate) fn normal_rotation([x, y, z]: [f32; 3], sine: f32, cosine: f32) -> SkeletonRootMatrix {
    let complement = 1.0 - cosine;
    let qx = complement * x;
    let qy = complement * y;
    let qz = complement * z;
    let sx = sine * x;
    let sy = sine * y;
    let sz = sine * z;
    let xx = qx.mul_add(x, cosine);
    let yx = qy * x - sz;
    let zx = qz.mul_add(x, sy);
    SkeletonRootMatrix::from_columns([
        SkeletonRootVector::known([xx, qx.mul_add(y, sz), qx * z - sy, xx]),
        SkeletonRootVector::known([yx, qy.mul_add(y, cosine), qy.mul_add(z, sx), yx]),
        SkeletonRootVector::known([zx, qz * y - sx, qz.mul_add(z, cosine), zx]),
        SkeletonRootVector::known([0.0; 4]),
    ])
}
