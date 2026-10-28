//! `SkeletonBody::SetRagdollJointLimits` 0x82b3bd08..0x82b3bef0.
//! Each dense row reads both live local-joints scalars before its first store.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    ragdoll::SkeletonPartIndex,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits, SimulationJointPool},
    skeleton_attributes::SkeletonBodyAttributeData,
    skeleton_data::SkeletonData,
    skeleton_joint_attributes::SkeletonJointAttributeProvider,
    skeleton_normal_joint_limits::{SkeletonJointLimitBinding, SkeletonNormalJointLimitsError},
};

const SWING_SCALE: u64 = 0xa57c_f13f_d8f9_5cf1;
const TWIST_SCALE: u64 = 0x4eee_4c25_b561_406c;

#[derive(Debug)]
pub(crate) enum RagdollJointLimitsError<C> {
    Attribute {
        child: SkeletonPartIndex,
        source: AttributeRuntimeError,
    },
    Limits(SkeletonNormalJointLimitsError<C>),
}

impl<C: std::fmt::Debug> std::fmt::Display for RagdollJointLimitsError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute { child, source } => {
                write!(f, "ragdoll Joint {child:?} attribute: {source}")
            }
            Self::Limits(source) => write!(f, "ragdoll Joint limits: {source:?}"),
        }
    }
}

pub(crate) fn try_set_ragdoll_joint_limits<C>(
    data: &SkeletonData,
    attributes: &SkeletonBodyAttributeData,
    reader: &mut SkeletonJointAttributeProvider,
    pool: &mut SimulationJointPool<SimulationJointFrames, SimulationJointLimits>,
    bindings: &[SkeletonJointLimitBinding],
    cosine: &mut impl FnMut(f32) -> Result<f32, C>,
) -> Result<(), RagdollJointLimitsError<C>> {
    use RagdollJointLimitsError as E;
    use SkeletonNormalJointLimitsError as L;
    let mut row = 0;
    for child in SkeletonPartIndex::ALL {
        if data.parents()[child.index()].is_none() {
            continue;
        }
        let bone = &data.parameters().bones[child.index()];
        let attribute = attributes
            .joints
            .get(row)
            .ok_or(E::Limits(L::AttributeRowUnavailable { child, row }))?;
        let swing_scale = reader
            .joint_float(SWING_SCALE)
            .map_err(|source| E::Attribute { child, source })?;
        // Raw bdfc/be18: two rounded multiplies, then source fsel minimum.
        let swing = minimum_angle((attribute.swing_ragdoll * bone.swing) * swing_scale);
        let twist_scale = reader
            .joint_float(TWIST_SCALE)
            .map_err(|source| E::Attribute { child, source })?;
        // Raw be50/be68: bone*live multiplier precedes cached joint factor.
        let twist = (bone.twist * twist_scale) * attribute.twist_ragdoll;
        let binding = bindings
            .get(row)
            .ok_or(E::Limits(L::MissingInitializedRow { child, row }))?;
        let mut limits = binding
            .limits
            .try_read()
            .map_err(|source| E::Limits(L::Pool { child, row, source }))?;
        limits.swing_a = swing;
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| E::Limits(L::Pool { child, row, source }))?;
        let swing_cosine =
            cosine(swing).map_err(|source| E::Limits(L::Cosine { child, row, source }))?;
        let twist = minimum_angle(twist);
        limits.swing_c = swing_cosine;
        limits.twist_a = twist;
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| E::Limits(L::Pool { child, row, source }))?;
        limits.twist_c =
            cosine(twist).map_err(|source| E::Limits(L::Cosine { child, row, source }))?;
        pool.try_write_mutable_limits(binding.joint, &binding.limits, limits)
            .map_err(|source| E::Limits(L::Pool { child, row, source }))?;
        row += 1;
    }
    Ok(())
}

fn minimum_angle(value: f32) -> f32 {
    let epsilon = f32::from_bits(0x3c23_d70a);
    if value - epsilon >= 0.0 {
        value
    } else {
        epsilon
    }
}
