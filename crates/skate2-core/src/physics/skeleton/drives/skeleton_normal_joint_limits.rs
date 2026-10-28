//! Live dense-limit writes from `SkeletonBody::SetNormalJointLimits`
//! (0x82b3bb90..0x82b3bd04), using the same storage retained by actual pool slots.
//! This is not the rest of `SetUpNormal` or completed humanoid assembly admission.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "normal skeleton assembly admission awaits remaining mutable owners"
    )
)]

use crate::{
    ragdoll::SkeletonPartIndex,
    simulation_joint_pool::{
        SimulationJointFrames, SimulationJointHandle, SimulationJointLimits, SimulationJointPool,
        SimulationJointPoolError, SimulationMutableJointLimits,
    },
    skeleton_attributes::SkeletonBodyAttributeData,
    skeleton_data::SkeletonData,
};

#[cfg(test)]
#[path = "skeleton_normal_joint_limits_tests.rs"]
mod tests;

/// A real live Joint and its exact shared source definition row. These are
/// host capabilities, not serialized addresses or fabricated body membership.
pub(crate) struct SkeletonJointLimitBinding {
    pub(crate) joint: SimulationJointHandle,
    pub(crate) limits: SimulationMutableJointLimits,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkeletonNormalJointLimitsError<E> {
    MissingInitializedRow {
        child: SkeletonPartIndex,
        row: usize,
    },
    AttributeRowUnavailable {
        child: SkeletonPartIndex,
        row: usize,
    },
    Pool {
        child: SkeletonPartIndex,
        row: usize,
        source: SimulationJointPoolError,
    },
    Cosine {
        child: SkeletonPartIndex,
        row: usize,
        source: E,
    },
}

/// No enable, `BallJoint`, emission-map or declared-count gate belongs here.
/// Successful earlier rows and the current row's completed stores survive an
/// error. A missing initialized row is a checked-host refusal, not an invented
/// value for native unwritten limit storage. Numeric callbacks may observe
/// shared values but cannot mutate or replace the exclusively borrowed pool.
/// Adjacent swing-cosine/twist-angle scalar stores are one locked Copy update;
/// their native concurrent intermediate visibility is not a host guarantee.
pub(crate) fn try_set_normal_joint_limits<E>(
    data: &SkeletonData,
    attributes: &SkeletonBodyAttributeData,
    pool: &mut SimulationJointPool<SimulationJointFrames, SimulationJointLimits>,
    bindings: &[SkeletonJointLimitBinding],
    cosine: &mut impl FnMut(f32) -> Result<f32, E>,
) -> Result<(), SkeletonNormalJointLimitsError<E>> {
    use SkeletonNormalJointLimitsError as Error;
    let mut row = 0;
    for child in SkeletonPartIndex::ALL {
        if data.parents()[child.index()].is_none() {
            continue;
        }
        // Validated SkeletonData cannot contain the source's parent bone -1.
        let bone = &data.parameters().bones[child.index()];
        let attribute = attributes
            .joints
            .get(row)
            .ok_or(Error::AttributeRowUnavailable { child, row })?;
        let swing = attribute.swing_angle * bone.swing;
        let twist = bone.twist * attribute.twist_angle;
        let binding = bindings
            .get(row)
            .ok_or(Error::MissingInitializedRow { child, row })?;
        // Private Copy storage has no mutation callbacks. Exact owner identity
        // and live occurrence are checked before the first source store.
        let mut limits =
            binding
                .limits
                .try_read()
                .map_err(|source| Error::Pool { child, row, source })?;
        limits.swing_a = minimum_angle(swing);
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| Error::Pool { child, row, source })?;
        let swing_cosine =
            cosine(limits.swing_a).map_err(|source| Error::Cosine { child, row, source })?;
        let twist_angle = minimum_angle(twist);
        limits.swing_c = swing_cosine;
        limits.twist_a = twist_angle;
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| Error::Pool { child, row, source })?;
        let twist_cosine =
            cosine(twist_angle).map_err(|source| Error::Cosine { child, row, source })?;
        limits.twist_c = twist_cosine;
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| Error::Pool { child, row, source })?;
        row += 1;
    }
    Ok(())
}

fn minimum_angle(product: f32) -> f32 {
    let epsilon = f32::from_bits(0x3c23_d70a);
    if product - epsilon >= 0.0 {
        product
    } else {
        epsilon
    }
}
