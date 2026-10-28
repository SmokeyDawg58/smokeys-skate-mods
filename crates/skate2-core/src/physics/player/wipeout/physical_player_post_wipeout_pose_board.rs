//! UpdatePoseBuffer82b4de30..ee98 board visuals, on the SAME local pose.
//! Child of `physical_player_post_wipeout_pose` (`post_wipeout_pose_live`).
use super::{PoseBeforeBoardVisuals, SkeletonNormalAssembly};
use crate::{
    actor_construction::anim_out_phys_in::{
        AnimOutPhysInOwner,
        pose_lanes::{PoseAccessError, PoseBuffer},
    },
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError,
    reckoning_numeric::{ReckoningNumeric, try_normalize_safe, try_signed_angle},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_body_construction::SkateboardBodyConstruction,
    skateboard_truck_drives::SkateboardTruckDrives,
    skeleton_assembly_construction::constructor::SkeletonConstructorRuntime,
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

pub(crate) struct PoseBoardOwners<'a> {
    pub(crate) pose: &'a mut AnimOutPhysInOwner,
    pub(crate) board: &'a SkateboardAssemblySession,
    pub(crate) backend: &'a SkateboardSimulationBackend,
    pub(crate) board_construction: &'a SkateboardBodyConstruction,
    pub(crate) trucks: &'a SkateboardTruckDrives,
    pub(crate) constructor: &'a SkeletonConstructorRuntime,
    /// Actual GlobalAttributeCollection+ec `physics_skeleton.default`.
    pub(crate) physics_skeleton: &'a AttributeRuntimeInstance,
}
#[derive(Debug)]
pub(crate) enum PoseBoardError<N, S> {
    Missing(&'static str), Pose(PoseAccessError), Part(DynamicAssemblyPartTransformError),
    Attribute(AttributeRuntimeError), Numeric(N), SinCos(S),
    Root(crate::dynamic_assembly_session::skeleton_normal::part_transform_query::SkeletonPartTransformQueryError),
    TiltAxisAssertion { front: bool },
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for PoseBoardError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "pose board missing {field}"),
            Self::Pose(e) => e.fmt(f),
            Self::Part(e) => write!(f, "pose board Part: {e:?}"),
            Self::Attribute(e) => write!(f, "pose board attribute: {e:?}"),
            Self::Numeric(e) => write!(f, "pose board numeric: {e:?}"),
            Self::SinCos(e) => write!(f, "pose board SinCos: {e:?}"),
            Self::Root(e) => e.fmt(f),
            Self::TiltAxisAssertion { front } => {
                write!(f, "pose truck axis assertion front={front}")
            }
        }
    }
}

const ZERO: Vector = Vector::known([0.0; 4]);
const J: Vector = Vector::known([0.0, 1.0, 0.0, 0.0]);
const K: Vector = Vector::known([0.0, 0.0, 1.0, 0.0]);
const NEGATIVE_I: Vector = Vector::known([-1.0, -0.0, -0.0, -0.0]);

fn direction(frame: &Matrix, vector: Vector) -> Vector {
    let [x, y, z, _] = *frame.columns();
    let result = x.map(|v| vector.xyz[0] * v);
    let result = y.zip(result, |v, r| vector.xyz[1].mul_add(v, r));
    z.zip(result, |v, r| vector.xyz[2].mul_add(v, r))
}
fn rotation_x(sin: f32, cos: f32) -> Matrix {
    // vperm constant82fd6e90 selects [outer.x,inner.y,outer.x,outer.x].
    // The subsequent Z insertion leaves X.w=1. It is an inner coefficient
    // matrix, whose W lanes are not consumed by native affine composition.
    Matrix::known([
        1.0, 0.0, 0.0, 1.0, 0.0, cos, sin, 0.0, 0.0, -sin, cos, 0.0, 0.0, 0.0, 0.0, 0.0,
    ])
}
#[expect(
    clippy::cast_precision_loss,
    reason = "native FCFID/FRSP converts truncated turns back to single precision"
)]
fn normalize_pose_angle<N: ReckoningNumeric>(angle: f32, numeric: &mut N) -> Result<f32, N::Error> {
    let pi = f32::from_bits(0x4049_0fdb);
    let tau = f32::from_bits(0x40c9_0fdb);
    if (-pi..pi).contains(&angle) {
        return Ok(angle);
    }
    let turns = numeric.truncate_i32(angle * f32::from_bits(0x3e22_f983))? as f32;
    // FNMSUBS negates AFTER the fused subtraction, including exact-zero sign.
    let remainder = -(turns.mul_add(tau, -angle));
    Ok(
        if remainder.partial_cmp(&pi) != Some(std::cmp::Ordering::Less) {
            remainder - tau
        } else if !matches!(
            remainder.partial_cmp(&-pi),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ) {
            remainder + tau
        } else {
            remainder
        },
    )
}

impl SkeletonNormalAssembly {
    /// Requires the actual predecessor token after the24 global/local loops.
    /// This mutates only the actual six named LOCAL bone matrices, in source
    /// order; source leaves the corresponding globals at their earlier values.
    #[expect(
        clippy::too_many_lines,
        reason = "source ordering of six visual publications and preceding live reads"
    )]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "consume native continuation and exclusive pose borrow"
    )]
    pub(crate) fn try_post_wipeout_pose_board<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
        &self,
        before: PoseBeforeBoardVisuals,
        owners: PoseBoardOwners<'_>,
        numeric: &mut N,
        sin_cos: &mut S,
    ) -> Result<(), PoseBoardError<N::Error, S::Error>> {
        use PoseBoardError as E;
        // Only orientation of these inverse truck transforms is consumed.
        let front = owners
            .board
            .try_attached_part_body_transform(owners.backend, 4)
            .map_err(E::Part)?
            .inverse();
        let back = owners
            .board
            .try_attached_part_body_transform(owners.backend, 5)
            .map_err(E::Part)?
            .inverse();
        let mut wheels = [before.deck; 4];
        for (part, wheel) in wheels.iter_mut().enumerate() {
            *wheel = owners
                .board
                .try_attached_part_body_transform(owners.backend, part)
                .map_err(E::Part)?;
        }
        let wheel_axes = [
            direction(&front, wheels[0].columns()[1]),
            direction(&front, wheels[1].columns()[1]),
            direction(&back, wheels[2].columns()[1]),
            direction(&back, wheels[3].columns()[1]),
        ];
        let mut wheel_rotations = [before.deck; 4];
        for (ordinal, (axis, rotation)) in
            wheel_axes.into_iter().zip(&mut wheel_rotations).enumerate()
        {
            let angle = try_signed_angle(axis, J, K, numeric).map_err(E::Numeric)?;
            let angle = if ordinal >= 2 {
                f32::from_bits(angle.to_bits() ^ 0x8000_0000)
            } else {
                angle
            };
            let (sin, cos) = try_sin_cos(angle, sin_cos).map_err(E::SinCos)?;
            *rotation = rotation_x(sin, cos);
        }
        let attribute = |key| {
            owners
                .physics_skeleton
                .try_scalar_word(key, 0)
                .map(|value| f32::from_bits(value.unwrap_or(0)))
                .map_err(E::Attribute)
        };
        let angle_scale = attribute(0xecdb_b05b_b9a9_c877)?;
        let angle_cap = attribute(0x770b_3abe_0e17_c90d)?;
        let wobble_scale = attribute(0x023c_bbcd_aebd_c56b)?;
        let back_wind = Matrix::known(
            *owners
                .trucks
                .current_back()
                .ok_or(E::Missing("current back truck+1ed0"))?,
        )
        .columns()[0];
        let front_wind = Matrix::known(
            *owners
                .trucks
                .current_front()
                .ok_or(E::Missing("current front truck+1e90"))?,
        )
        .columns()[0];
        let use_physics = self
            .use_physics_truck_tilt
            .ok_or(E::Missing("UsePhysicsTruckTilt3e41"))?;
        let (front_axis, back_axis) = if use_physics {
            let root = self.try_query_part_transform(0).map_err(E::Root)?.inverse();
            let difference = wheels[0].columns()[3].zip(wheels[1].columns()[3], |a, b| a - b);
            let axis = try_normalize_safe(difference, ZERO, numeric).map_err(E::Numeric)?;
            let back_axis = direction(&root, axis);
            if 0.5
                > numeric
                    .dot_vector(back_axis, back_axis)
                    .map_err(E::Numeric)?
            {
                return Err(E::TiltAxisAssertion { front: false });
            }
            let difference = wheels[3].columns()[3].zip(wheels[2].columns()[3], |a, b| a - b);
            let axis = try_normalize_safe(difference, ZERO, numeric).map_err(E::Numeric)?;
            let front_axis = direction(&root, axis);
            if 0.5
                > numeric
                    .dot_vector(front_axis, front_axis)
                    .map_err(E::Numeric)?
            {
                return Err(E::TiltAxisAssertion { front: true });
            }
            (front_axis, back_axis)
        } else {
            let front = Matrix::known(
                *owners
                    .trucks
                    .current_front()
                    .ok_or(E::Missing("front truck tilt"))?,
            )
            .columns()[2]
                .map(|v| -v);
            let back = Matrix::known(
                *owners
                    .trucks
                    .current_back()
                    .ok_or(E::Missing("back truck tilt"))?,
            )
            .columns()[2];
            (front, back)
        };
        let angle =
            try_signed_angle(front_axis, NEGATIVE_I, back_wind, numeric).map_err(E::Numeric)?;
        let front_angle = normalize_pose_angle(angle, numeric).map_err(E::Numeric)?;
        let angle =
            try_signed_angle(back_axis, NEGATIVE_I, front_wind, numeric).map_err(E::Numeric)?;
        let back_angle = normalize_pose_angle(angle, numeric).map_err(E::Numeric)?;
        let outer = owners
            .constructor
            .outer
            .as_ref()
            .ok_or(E::Missing("Skeleton outer wobble fields"))?;
        let wobble = outer.deck_wobble_tilt * wobble_scale;
        // Two FNMSUBS: negate the rounded fused subtraction, then two FSELs.
        let back_tilt = clamp_tilt(-(back_angle.mul_add(angle_scale, -wobble)), angle_cap);
        let front_tilt = clamp_tilt(-(front_angle.mul_add(angle_scale, wobble)), angle_cap);
        let (sin, cos) = try_sin_cos(front_tilt, sin_cos).map_err(E::SinCos)?;
        let front_rotation = rotation_x(sin, cos);
        let (sin, cos) = try_sin_cos(back_tilt, sin_cos).map_err(E::SinCos)?;
        let back_rotation = rotation_x(sin, cos);
        let compression = owners
            .board_construction
            .wheel_compressions()
            .current()
            .ok_or(E::Missing("average front/back wheel compression"))?;
        let cap = attribute(0x1ba7_a9d7_c680_ffee)?;
        let front_offset = compression_offset(compression[0], outer.deck_wobble_squish, cap);
        let back_offset = compression_offset(compression[1], outer.deck_wobble_squish, cap);
        let bone = |slot: usize| {
            owners.constructor.init.named_bones[slot]
                .and_then(|v| usize::try_from(v).ok())
                .ok_or(E::Missing("named truck/wheel bone"))
        };
        let front_bone = bone(1)?;
        let old = owners
            .pose
            .pose_matrix_typed(PoseBuffer::Local, front_bone)
            .map_err(E::Pose)?;
        let value = Matrix::translation(Vector::known([0.0, front_offset, 0.0, 0.0]))
            .compose(&old.compose(&back_rotation));
        owners
            .pose
            .set_pose_matrix_typed(PoseBuffer::Local, front_bone, &value)
            .map_err(E::Pose)?;
        let back_bone = bone(0)?;
        let old = owners
            .pose
            .pose_matrix_typed(PoseBuffer::Local, back_bone)
            .map_err(E::Pose)?;
        let value = Matrix::translation(Vector::known([0.0, back_offset, 0.0, 0.0]))
            .compose(&old.compose(&front_rotation));
        owners
            .pose
            .set_pose_matrix_typed(PoseBuffer::Local, back_bone, &value)
            .map_err(E::Pose)?;
        for (slot, rotation) in wheel_rotations.iter().enumerate() {
            let wheel = bone(slot + 2)?;
            let old = owners
                .pose
                .pose_matrix_typed(PoseBuffer::Local, wheel)
                .map_err(E::Pose)?;
            owners
                .pose
                .set_pose_matrix_typed(PoseBuffer::Local, wheel, &old.compose(rotation))
                .map_err(E::Pose)?;
        }
        Ok(())
    }
}

fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn clamp_tilt(value: f32, cap: f32) -> f32 {
    let lower = fsel(-cap - value, -cap, value);
    fsel(cap - lower, lower, cap)
}
fn compression_offset(compression: f32, squish: f32, cap: f32) -> f32 {
    // Native BO4/BI24 skips replacement when !LT, including unordered.
    let compression = if compression < f32::from_bits(0x3a83_126f) {
        0.0
    } else {
        compression
    };
    let value = compression - squish;
    let lower = fsel(-value, 0.0, value);
    fsel(cap - lower, lower, cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_turn_reduction_retains_negated_exact_zero() {
        let angle = f32::from_bits(0x40c9_0fdb);
        let result =
            normalize_pose_angle(angle, &mut crate::reckoning_numeric::HostReckoningNumeric)
                .unwrap();
        assert_eq!(result.to_bits(), (-0.0f32).to_bits());
    }

    #[test]
    fn unordered_compression_reaches_fsel_and_selects_the_cap() {
        assert_eq!(
            compression_offset(f32::NAN, -0.25, 0.5).to_bits(),
            0.5f32.to_bits()
        );
        assert_eq!(
            compression_offset(0.8, 0.1, 0.5).to_bits(),
            0.5f32.to_bits()
        );
        assert!(compression_offset(0.8, 0.1, f32::NAN).is_nan());
        assert_eq!(clamp_tilt(f32::NAN, 0.5).to_bits(), 0.5f32.to_bits());
    }

    #[test]
    fn wheel_rotation_preserves_full_native_outer_lane_dependency() {
        let mut old = Matrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0, 3.0, 4.0, 0.0,
        ]);
        old.set_column(
            0,
            Vector {
                xyz: [1.0, 0.0, 0.0],
                w: None,
            },
        );
        let result = old.compose(&rotation_x(1.0, 0.0));
        assert_eq!(
            result.columns()[1].xyz.map(f32::to_bits),
            [0.0f32, 0.0, 1.0].map(f32::to_bits)
        );
        assert_eq!(
            result.columns()[2].xyz.map(f32::to_bits),
            [0.0f32, -1.0, 0.0].map(f32::to_bits)
        );
        assert_eq!(
            result.columns()[3].xyz.map(f32::to_bits),
            [2.0f32, 3.0, 4.0].map(f32::to_bits)
        );
        assert!(result.columns().iter().all(|column| column.w.is_none()));
    }
}
