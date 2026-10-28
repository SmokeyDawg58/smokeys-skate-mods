//! Concrete `SkeletonBody::InitJoints` (0x82b464f8) frame/limit producer.
//! Output is the initialized dense prefix of the earlier22/0 allocation. This
//! neither fills unwritten tails nor rewrites the source declared joint count.
//! Host errors publish no partial definition; native unchecked failure and
//! intermediate stores are outside this immutable construction boundary.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "joints await normal body admission")
)]
use crate::{
    dynamic_assembly_session::DynamicAssemblyJointDefinition,
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    simulation_drive_jacobian::drive_basis_host,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    skateboard_spawn_math::{skateboard_spawn_compose, skateboard_spawn_rigid_inverse},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_attributes::SkeletonBodyAttributeData,
    skeleton_data::SkeletonData,
    skeleton_joint_attributes::{SkeletonJointAttributeRead, SkeletonJointByteRoute},
    world::try_simulation_quaternion_candidate,
};
use skate2_formats::andale::AndalePhysicsBone;

#[cfg(test)]
#[path = "skeleton_body_joints_tests.rs"]
mod tests;

const ENABLE: u64 = 0xdff9_7208_0e90_f82f;
const FREE_SWING: u64 = 0x9cf0_c7c3_8917_9e4c;
const FREE_TWIST: u64 = 0x2654_99fa_ea27_836a;
const SWING_VELOCITY: u64 = 0x71cc_8102_bc85_277b;
const TWIST_VELOCITY: u64 = 0x1b84_fd92_5541_2c0f;
const TIMESTEP: f32 = f32::from_bits(0x3c88_8889);
const ANGLE_EPSILON: f32 = f32::from_bits(0x3c23_d70a);
const FREQUENCY: f32 = f32::from_bits(0x426f_ffff);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonJointQuaternionSite {
    ParentAngular,
    ChildAngular,
    ParentLinear,
}

/// Pure intrinsic services: no reentrant mutation of source owners. Source
/// interleaves parent/child candidate refinements; sequential pure calculations
/// have the same values and seed-call order, not the same intermediate stores.
pub(crate) trait SkeletonJointNumeric {
    type Error;
    fn quaternion_estimate(
        &mut self,
        site: SkeletonJointQuaternionSite,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error>;
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error>;
}

pub(crate) struct HostSkeletonJointNumeric;
impl SkeletonJointNumeric for HostSkeletonJointNumeric {
    type Error = std::convert::Infallible;
    fn quaternion_estimate(
        &mut self,
        _site: SkeletonJointQuaternionSite,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(magnitudes.map(|value| value.sqrt().recip()))
    }
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error> {
        Ok(timestep.recip())
    }
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        Ok(angle.cos())
    }
}

pub(crate) type SkeletonJointDefinition =
    DynamicAssemblyJointDefinition<SimulationJointFrames, SimulationJointLimits>;

#[derive(Debug)]
pub(crate) struct SkeletonBodyJoints {
    pub(crate) joints: Vec<SkeletonJointDefinition>,
    pub(crate) allocated_count: usize,
    pub(crate) emitted_for_part: [Option<usize>; SKELETON_PART_COUNT],
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonJointError<A, N> {
    Allocation,
    Attribute(A),
    Numeric(N),
    AttributeRowUnavailable {
        child: SkeletonPartIndex,
        row: usize,
    },
    Capacity {
        child: SkeletonPartIndex,
        allocated: usize,
    },
    ParentReverseMappingUnavailable {
        child: SkeletonPartIndex,
        parent_bone: usize,
    },
}

impl SkeletonBodyJoints {
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered frame, attribute and emission operations"
    )]
    pub(crate) fn try_create<R: SkeletonJointAttributeRead, N: SkeletonJointNumeric>(
        data: &SkeletonData,
        attributes: &SkeletonBodyAttributeData,
        allocated_count: usize,
        reader: &mut R,
        numeric: &mut N,
    ) -> Result<Self, SkeletonJointError<R::Error, N::Error>> {
        use SkeletonJointError as Error;
        let mut output = Self {
            joints: Vec::new(),
            allocated_count,
            emitted_for_part: [None; SKELETON_PART_COUNT],
        };
        output
            .joints
            .try_reserve_exact(allocated_count)
            .map_err(|_| Error::Allocation)?;
        for child in SkeletonPartIndex::ALL {
            let Some(parent) = data.parents()[child.index()] else {
                continue;
            };
            let parent_bone = data.bone_ids()[parent.index()];
            let frames = try_frames(
                &data.parameters().bones[child.index()],
                &data.parameters().bones[parent.index()],
                &data.physics_pose()[child.index()],
                &data.physics_pose()[parent.index()],
                numeric,
            )
            .map_err(Error::Numeric)?;
            let velocity = reader.cached_prism_velocity().map_err(Error::Attribute)?;
            let mut reciprocal = numeric
                .timestep_reciprocal_estimate(TIMESTEP)
                .map_err(Error::Numeric)?;
            for _ in 0..2 {
                let residual = (-reciprocal).mul_add(TIMESTEP, 1.0);
                reciprocal = reciprocal.mul_add(residual, reciprocal);
            }
            let mut limits = SimulationJointLimits {
                p_prism: [0.0; 4],
                v_prism: velocity.map(|lane| reciprocal * lane),
                v_twist: 0.0,
                v_swing: 0.0,
                swing_a: 0.0,
                twist_a: 0.0,
                swing_c: 1.0,
                twist_c: 1.0,
                raw_swing_type: 4,
                raw_twist_type: 2,
            };
            let enabled = reader
                .byte(SkeletonJointByteRoute::LocalJoints, ENABLE)
                .map_err(Error::Attribute)?;
            let row_index = output.joints.len();
            let limited_row =
                if enabled != 0 {
                    Some(attributes.joints.get(row_index).ok_or(
                        Error::AttributeRowUnavailable {
                            child,
                            row: row_index,
                        },
                    )?)
                } else {
                    None
                };
            if let Some(row) = limited_row.filter(|row| row.ball_joint == 0) {
                let bone = &data.parameters().bones[child.index()];
                let swing = bone.swing * row.swing_angle;
                let twist = row.twist_angle * bone.twist;
                limits.swing_a = minimum_angle(swing);
                limits.swing_c = numeric.cosine(limits.swing_a).map_err(Error::Numeric)?;
                limits.twist_a = minimum_angle(twist);
                limits.twist_c = numeric.cosine(limits.twist_a).map_err(Error::Numeric)?;
                limits.raw_swing_type = 1;
                limits.raw_twist_type = 1;
            }
            if reader
                .byte(SkeletonJointByteRoute::LocalJoints, FREE_SWING)
                .map_err(Error::Attribute)?
                != 0
            {
                limits.raw_swing_type = 4;
            }
            if reader
                .byte(SkeletonJointByteRoute::LocalJoints, FREE_TWIST)
                .map_err(Error::Attribute)?
                != 0
            {
                limits.raw_twist_type = 2;
            }
            limits.v_swing = reader
                .skeleton_float(SWING_VELOCITY)
                .map_err(Error::Attribute)?
                * FREQUENCY;
            limits.v_twist = reader
                .skeleton_float(TWIST_VELOCITY)
                .map_err(Error::Attribute)?
                * FREQUENCY;
            if reader
                .byte(SkeletonJointByteRoute::GlobalAnimation, ENABLE)
                .map_err(Error::Attribute)?
                != 0
            {
                // Native writes parent from the LAST-WRITE reverse map, not
                // from the nearest-ancestor Part used by the frame calculation.
                let emitted_parent = data
                    .part_from_bone()
                    .get(parent_bone)
                    .copied()
                    .flatten()
                    .ok_or(Error::ParentReverseMappingUnavailable { child, parent_bone })?;
                if output.joints.len() >= allocated_count {
                    return Err(Error::Capacity {
                        child,
                        allocated: allocated_count,
                    });
                }
                output.emitted_for_part[child.index()] = Some(output.joints.len());
                output.joints.push(SkeletonJointDefinition {
                    parent_part: emitted_parent as i32,
                    child_part: child as i32,
                    frames,
                    limits,
                });
            }
        }
        Ok(output)
    }
}

fn minimum_angle(product: f32) -> f32 {
    if product - ANGLE_EPSILON >= 0.0 {
        product
    } else {
        ANGLE_EPSILON
    }
}

fn authored_frame(quaternion: [f32; 4], position: [f32; 4]) -> [f32; 16] {
    let axes = drive_basis_host(quaternion);
    std::array::from_fn(|index| {
        if index < 12 {
            axes[index / 4][index % 4]
        } else {
            position[index - 12]
        }
    })
}

fn transpose_rotation(frame: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        if index < 12 && index % 4 < 3 {
            frame[(index % 4) * 4 + index / 4]
        } else {
            0.0
        }
    })
}

fn transform_zero(frame: &[f32; 16]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let x = 0.0_f32.mul_add(frame[lane], frame[12 + lane]);
        let xy = 0.0_f32.mul_add(frame[4 + lane], x);
        0.0_f32.mul_add(frame[8 + lane], xy)
    })
}

/// Inlined frame graph 0x82b467e8..46ef0. The relative pose supplies parent
/// POSITION, not its angular frame. Authored parent orientation supplies that
/// frame. Explicit zero-point products preserve nonfinite/signed-zero behavior.
fn try_frames<N: SkeletonJointNumeric>(
    child: &AndalePhysicsBone,
    parent: &AndalePhysicsBone,
    child_pose: &[f32; 16],
    parent_pose: &[f32; 16],
    numeric: &mut N,
) -> Result<SimulationJointFrames, N::Error> {
    let relative =
        skateboard_spawn_compose(&skateboard_spawn_rigid_inverse(parent_pose), child_pose);
    let child_inverse =
        skateboard_spawn_rigid_inverse(&authored_frame(child.orientation, child.position));
    let parent_inverse =
        skateboard_spawn_rigid_inverse(&authored_frame(parent.orientation, parent.position));
    let parent_orientation =
        transpose_rotation(&authored_frame(child.parent_orientation, [0.0; 4]));
    let joint = authored_frame(child.joint_orientation, [0.0; 4]);
    let parent_rotation = skateboard_spawn_compose(&parent_inverse, &parent_orientation);
    let parent_frame = skateboard_spawn_compose(&parent_rotation, &joint);
    let child_frame = skateboard_spawn_compose(&child_inverse, &joint);
    let parent_relative = skateboard_spawn_compose(&parent_inverse, &relative);
    let position_a = transform_zero(&child_inverse);
    let position_b = transform_zero(&parent_relative);
    let quaternion_b = try_simulation_quaternion_candidate(parent_frame, |values| {
        numeric.quaternion_estimate(SkeletonJointQuaternionSite::ParentAngular, values)
    })?;
    let quaternion_a = try_simulation_quaternion_candidate(child_frame, |values| {
        numeric.quaternion_estimate(SkeletonJointQuaternionSite::ChildAngular, values)
    })?;
    let quaternion_l = try_simulation_quaternion_candidate(FRESH_VOLUME_TRANSFORM, |values| {
        numeric.quaternion_estimate(SkeletonJointQuaternionSite::ParentLinear, values)
    })?;
    Ok(SimulationJointFrames {
        quaternion_a,
        position_a,
        quaternion_b,
        position_b,
        quaternion_l,
    })
}
