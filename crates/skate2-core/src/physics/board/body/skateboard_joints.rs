//! Authored six-joint definitions (`CreateJoints`, 0x82b2d0e8) and independently
//! sampled level truck transforms (`CalcTruckTransforms`, 0x82b2c390).
//! These values do not admit a board or supply its missing drive/input path.

#![cfg_attr(
    not(test),
    expect(dead_code, reason = "authored joints await whole-board admission")
)]

use crate::{
    dynamic_assembly_session::DynamicAssemblyJointDefinition,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    skateboard_attributes::{SkateboardAttributeClass as Class, SkateboardAttributeRead},
    world::try_simulation_quaternion_candidate,
};
use std::{fmt, sync::Arc};

#[cfg(test)]
#[path = "skateboard_joints_tests.rs"]
mod tests;

const ANGLE: u64 = 0x7ca2_0106_5c78_f1cb;
const DECK_LENGTH: u64 = 0xc94d_6509_1dcb_828b;
const POSITIVE_OFFSET: u64 = 0xe02d_cbd0_7830_d3a5;
const NEGATIVE_OFFSET: u64 = 0xa69c_ca4e_15db_a369;
const VERTICAL: u64 = 0x2be0_8dd7_a920_7622;
const TWIST_ARC: u64 = 0xe8b3_4c12_d753_b164;
const TWIST_VELOCITY: u64 = 0x870b_a43e_d433_77d6;
const WHEEL_HALF_WIDTH: u64 = 0x5a84_c435_dc34_0af8;
const AXLE_VELOCITY: u64 = 0x1a90_ba4d_1195_7560;
const DTR: f32 = f32::from_bits(0x3c8e_fa35);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const TIMESTEP: f32 = f32::from_bits(0x3c88_8889);
const ANGULAR_FREQUENCY: f32 = f32::from_bits(0x426f_ffff);
const IDENTITY_AXES: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

pub(crate) type SkateboardJointDefinition =
    DynamicAssemblyJointDefinition<Arc<SimulationJointFrames>, Arc<SimulationJointLimits>>;

/// Intrinsic/library boundary only. Source refinements and candidate selection
/// remain authored calculations, not provider-selected replacement algorithms.
pub(crate) trait SkateboardJointNumeric {
    type Error;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error>;
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error>;
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error>;
}

/// Portable host estimates/trigonometry, not captured Xenon bit parity.
pub(crate) struct HostSkateboardJointNumeric;

impl SkateboardJointNumeric for HostSkateboardJointNumeric {
    type Error = std::convert::Infallible;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(angle.sin_cos())
    }
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        Ok(angle.cos())
    }
    fn quaternion_reciprocal_square_root_estimate(
        &mut self,
        magnitudes: [f32; 4],
    ) -> Result<[f32; 4], Self::Error> {
        Ok(magnitudes.map(|value| value.sqrt().recip()))
    }
    fn timestep_reciprocal_estimate(&mut self, timestep: f32) -> Result<f32, Self::Error> {
        Ok(timestep.recip())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardJointBuildError<A, N> {
    Attribute(A),
    Numeric(N),
}

impl<A: fmt::Display, N: fmt::Display> fmt::Display for SkateboardJointBuildError<A, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "board joint attribute read: {error}"),
            Self::Numeric(error) => write!(f, "board joint numeric operation: {error}"),
        }
    }
}

impl<A: std::error::Error + 'static, N: std::error::Error + 'static> std::error::Error
    for SkateboardJointBuildError<A, N>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Attribute(error) => Some(error),
            Self::Numeric(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkateboardTruckTransforms {
    /// PDB `mFrontTruckTransformLevel`, +0x1e10: negative longitudinal position.
    pub(crate) front: [f32; 16],
    /// PDB `mBackTruckTransformLevel`, +0x1e50: positive longitudinal position.
    pub(crate) back: [f32; 16],
}

/// `CreateJoints`, 0x82b2d0e8..0x82b2e080. The enable-byte lookup belongs to
/// Init before definition allocation, not this unconditional six-record leaf.
#[expect(
    clippy::too_many_lines,
    reason = "preserve one readable source-ordered sequence of interleaved attribute and numeric callbacks"
)]
pub(crate) fn try_create_skateboard_joints<
    A: SkateboardAttributeRead,
    N: SkateboardJointNumeric,
>(
    attributes: &mut A,
    numeric: &mut N,
) -> Result<[SkateboardJointDefinition; 6], SkateboardJointBuildError<A::Error, N::Error>> {
    use SkateboardJointBuildError::{Attribute, Numeric};

    let first_y = rotation_y(numeric.sine_cosine(flip_sign(HALF_PI)).map_err(Numeric)?);
    let first_angle = attributes
        .read_float(Class::Trucks, ANGLE)
        .map_err(Attribute)?;
    let first_x = rotation_x(
        numeric
            .sine_cosine(flip_sign(first_angle * DTR))
            .map_err(Numeric)?,
    );
    let first_rotation = compose_x_y(first_x, first_y);
    let length_pointer = attributes
        .lookup(Class::Deck, DECK_LENGTH)
        .map_err(Attribute)?;
    let offset_pointer = attributes
        .lookup(Class::Trucks, POSITIVE_OFFSET)
        .map_err(Attribute)?;
    let offset = attributes.float(&offset_pointer).map_err(Attribute)?;
    let length = attributes.float(&length_pointer).map_err(Attribute)?;
    let positive_z = length.mul_add(0.5, offset);
    let mut first = make_frames(IDENTITY_AXES, first_rotation, numeric).map_err(Numeric)?;
    let vertical = attributes
        .read_float(Class::Trucks, VERTICAL)
        .map_err(Attribute)?;
    first.position_b = [0.0, vertical, positive_z, 0.0];

    let second_y = rotation_y(numeric.sine_cosine(HALF_PI).map_err(Numeric)?);
    let second_angle = attributes
        .read_float(Class::Trucks, ANGLE)
        .map_err(Attribute)?;
    let second_x = rotation_x(numeric.sine_cosine(second_angle * DTR).map_err(Numeric)?);
    let second_rotation = compose_x_y(second_x, second_y);
    let offset_pointer = attributes
        .lookup(Class::Trucks, NEGATIVE_OFFSET)
        .map_err(Attribute)?;
    let length_pointer = attributes
        .lookup(Class::Deck, DECK_LENGTH)
        .map_err(Attribute)?;
    let offset = attributes.float(&offset_pointer).map_err(Attribute)?;
    let negated_offset = flip_sign(offset);
    let length = attributes.float(&length_pointer).map_err(Attribute)?;
    let negative_z = flip_sign(length.mul_add(0.5, flip_sign(negated_offset)));
    let mut second = make_frames(IDENTITY_AXES, second_rotation, numeric).map_err(Numeric)?;
    let vertical = attributes
        .read_float(Class::Trucks, VERTICAL)
        .map_err(Attribute)?;
    second.position_b = [0.0, vertical, negative_z, 0.0];

    let truck_v_prism = prism_velocity(numeric).map_err(Numeric)?;
    let arc = attributes
        .read_float(Class::Trucks, TWIST_ARC)
        .map_err(Attribute)?
        * DTR;
    let arc_cosine = numeric.cosine(arc).map_err(Numeric)?;
    let velocity = attributes
        .read_float(Class::Trucks, TWIST_VELOCITY)
        .map_err(Attribute)?;
    let truck_limits = SimulationJointLimits {
        p_prism: [0.0; 4],
        v_prism: truck_v_prism,
        v_twist: (velocity * DTR) * ANGULAR_FREQUENCY,
        v_swing: 0.0,
        swing_a: 0.0,
        twist_a: arc,
        swing_c: 1.0,
        twist_c: arc_cosine,
        raw_swing_type: 0,
        raw_twist_type: 1,
    };

    let axle_rotation = rotation_x(numeric.sine_cosine(HALF_PI).map_err(Numeric)?);
    let mut positive_axle = make_frames(axle_rotation, axle_rotation, numeric).map_err(Numeric)?;
    let width = attributes
        .read_float(Class::Wheels, WHEEL_HALF_WIDTH)
        .map_err(Attribute)?;
    positive_axle.position_b = [0.0, 0.0, width, 0.0];
    let mut negative_axle = make_frames(axle_rotation, axle_rotation, numeric).map_err(Numeric)?;
    let width = attributes
        .read_float(Class::Wheels, WHEEL_HALF_WIDTH)
        .map_err(Attribute)?;
    negative_axle.position_b = [0.0, 0.0, flip_sign(width), 0.0];
    let axle_v_prism = prism_velocity(numeric).map_err(Numeric)?;
    let velocity = attributes
        .read_float(Class::Wheels, AXLE_VELOCITY)
        .map_err(Attribute)?;
    let axle_limits = SimulationJointLimits {
        p_prism: [0.0; 4],
        v_prism: axle_v_prism,
        v_twist: 0.0,
        v_swing: (velocity * DTR) * ANGULAR_FREQUENCY,
        swing_a: 0.0,
        twist_a: 0.0,
        swing_c: 1.0,
        twist_c: 1.0,
        // CreateJoints82b2de84 loads3; df0c/df78/dfdc/e048 store it to
        // JointLimits+38. Value4 is FREE in JointJacobian::Build, not AXLE.
        raw_swing_type: 3,
        raw_twist_type: 0,
    };
    // Native definition storage has six distinct frame and limit slots. Equal
    // copied values do not imply shared pointer identity between ordinals.
    Ok([
        joint(6, 4, first, truck_limits),
        joint(6, 5, second, truck_limits),
        joint(4, 0, positive_axle, axle_limits),
        joint(4, 1, negative_axle, axle_limits),
        joint(5, 2, positive_axle, axle_limits),
        joint(5, 3, negative_axle, axle_limits),
    ])
}

/// `CalcTruckTransforms`, 0x82b2c390..0x82b2c998. This separate sampling sequence
/// is not replaced by the `CreateJoints` inputs, even for an immutable provider.
pub(crate) fn try_calc_truck_transforms<A: SkateboardAttributeRead, N: SkateboardJointNumeric>(
    attributes: &mut A,
    numeric: &mut N,
) -> Result<SkateboardTruckTransforms, SkateboardJointBuildError<A::Error, N::Error>> {
    use SkateboardJointBuildError::{Attribute, Numeric};
    let length_pointer = attributes
        .lookup(Class::Deck, DECK_LENGTH)
        .map_err(Attribute)?;
    let offset_pointer = attributes
        .lookup(Class::Trucks, POSITIVE_OFFSET)
        .map_err(Attribute)?;
    let length = attributes.float(&length_pointer).map_err(Attribute)?;
    let offset = attributes.float(&offset_pointer).map_err(Attribute)?;
    let positive_z = length.mul_add(0.5, offset);
    let offset_pointer = attributes
        .lookup(Class::Trucks, NEGATIVE_OFFSET)
        .map_err(Attribute)?;
    let length_pointer = attributes
        .lookup(Class::Deck, DECK_LENGTH)
        .map_err(Attribute)?;
    let offset = attributes.float(&offset_pointer).map_err(Attribute)?;
    let negated_offset = flip_sign(offset);
    let length = attributes.float(&length_pointer).map_err(Attribute)?;
    let negative_z = flip_sign(length.mul_add(0.5, flip_sign(negated_offset)));

    let first_y = rotation_y(numeric.sine_cosine(flip_sign(HALF_PI)).map_err(Numeric)?);
    let angle = attributes
        .read_float(Class::Trucks, ANGLE)
        .map_err(Attribute)?;
    let first_x = rotation_x(
        numeric
            .sine_cosine(flip_sign(angle * DTR))
            .map_err(Numeric)?,
    );
    let back_rotation = compose_x_y(first_x, first_y);
    let vertical = attributes
        .read_float(Class::Trucks, VERTICAL)
        .map_err(Attribute)?;
    let back = matrix(back_rotation, [0.0, vertical, positive_z, 0.0]);

    let second_y = rotation_y(numeric.sine_cosine(HALF_PI).map_err(Numeric)?);
    let angle = attributes
        .read_float(Class::Trucks, ANGLE)
        .map_err(Attribute)?;
    let second_x = rotation_x(numeric.sine_cosine(angle * DTR).map_err(Numeric)?);
    let front_rotation = compose_x_y(second_x, second_y);
    let vertical = attributes
        .read_float(Class::Trucks, VERTICAL)
        .map_err(Attribute)?;
    let front = matrix(front_rotation, [0.0, vertical, negative_z, 0.0]);
    Ok(SkateboardTruckTransforms { front, back })
}

fn joint(
    parent_part: i32,
    child_part: i32,
    frames: SimulationJointFrames,
    limits: SimulationJointLimits,
) -> SkateboardJointDefinition {
    DynamicAssemblyJointDefinition {
        parent_part,
        child_part,
        frames: Arc::new(frames),
        limits: Arc::new(limits),
    }
}

fn make_frames<N: SkateboardJointNumeric>(
    child: [[f32; 4]; 3],
    parent: [[f32; 4]; 3],
    numeric: &mut N,
) -> Result<SimulationJointFrames, N::Error> {
    let quaternion_a = quaternion(child, numeric)?;
    let quaternion_b = quaternion(parent, numeric)?;
    let quaternion_l = quaternion(parent, numeric)?;
    Ok(SimulationJointFrames {
        quaternion_a,
        position_a: [0.0; 4],
        quaternion_b,
        position_b: [0.0; 4],
        quaternion_l,
    })
}

fn quaternion<N: SkateboardJointNumeric>(
    axes: [[f32; 4]; 3],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    try_simulation_quaternion_candidate(matrix(axes, [0.0; 4]), |magnitudes| {
        numeric.quaternion_reciprocal_square_root_estimate(magnitudes)
    })
}

fn prism_velocity<N: SkateboardJointNumeric>(numeric: &mut N) -> Result<[f32; 4], N::Error> {
    let mut estimate = numeric.timestep_reciprocal_estimate(TIMESTEP)?;
    for _ in 0..2 {
        let residual = flip_sign(estimate).mul_add(TIMESTEP, 1.0);
        estimate = estimate.mul_add(residual, estimate);
    }
    Ok([estimate * 0.0; 4])
}

fn flip_sign(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

fn rotation_y((sine, cosine): (f32, f32)) -> [[f32; 4]; 3] {
    [
        [cosine, 0.0, flip_sign(sine), cosine],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, sine],
    ]
}

fn rotation_x((sine, cosine): (f32, f32)) -> [[f32; 4]; 3] {
    [
        [1.0, 0.0, 0.0, 1.0],
        [0.0, cosine, sine, 0.0],
        [0.0, flip_sign(sine), cosine, 0.0],
    ]
}

fn compose_x_y(x: [[f32; 4]; 3], y: [[f32; 4]; 3]) -> [[f32; 4]; 3] {
    y.map(|column| {
        std::array::from_fn(|lane| {
            let first = x[0][lane] * column[0];
            let second = x[1][lane].mul_add(column[1], first);
            x[2][lane].mul_add(column[2], second)
        })
    })
}

fn matrix(axes: [[f32; 4]; 3], position: [f32; 4]) -> [f32; 16] {
    std::array::from_fn(|index| {
        if index < 12 {
            axes[index / 4][index % 4]
        } else {
            position[index - 12]
        }
    })
}
