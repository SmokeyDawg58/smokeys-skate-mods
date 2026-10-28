//! Pre-normal binding of actual Skeleton definitions to the existing Assembly.
//!
//! `SkeletonBody::Init` calls `CreateAssembly` at 0x82b620d8. Component
//! initialization (0x82ae0bf8) retains definition-backed inertia and joint
//! limits, and `Part::Initialize` (0x82ae8fc8) initializes every stored Part
//! matrix to I/J/K/zero. The physics pose is NOT initial admission placement.
//! This owner stops before island membership, group/userdata, zero-material
//! selection, spy registration and `SetUpNormal`. It is not a complete Body Init.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "pre-normal skeleton awaits its physical owner")
)]

use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyJointDefinition, DynamicAssemblyPartAttachmentMetadata,
        DynamicAssemblyPartAttachmentTemplate, DynamicAssemblyPartPayload,
        DynamicAssemblyRuntimeTemplate, DynamicAssemblySession, DynamicAssemblySessionCreateError,
        DynamicAssemblyTemplateError,
    },
    ragdoll::SKELETON_PART_COUNT,
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_assembly_payload::{SimulationAssemblyInertia, SimulationAssemblyJointLimits},
    simulation_joint_pool::{
        SimulationJointFrames, SimulationJointLimits, SimulationMutableJointLimits,
    },
    simulation_rigid_body_pool::SimulationMutableInertia,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_body_definition::SkeletonBodyDefinition,
    skeleton_body_joints::SkeletonBodyJoints,
    skeleton_culling::{SkeletonCullingKey, SkeletonCullingResource},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

#[cfg(test)]
#[path = "skeleton_assembly_tests.rs"]
pub(crate) mod tests;

pub(crate) type SkeletonPreNormalSimulationBackend =
    SimulationAssemblyBackend<AssemblyPartMaterial, SimulationJointFrames, SimulationJointLimits>;
pub(crate) type SkeletonPreNormalAssemblySession =
    DynamicAssemblySession<SkeletonPreNormalSimulationBackend, SkeletonAssemblyBinding>;

/// Actual geometry remains in the body owner, including the hat's nested
/// allocation. Mutable cells are shared with the real pools, not detached
/// normal-mode snapshots. Their first admission binds them to one pool domain.
/// The original mass caches and packed tensor W remain in `body`; no packed
/// Xenon inertia address is manufactured by these semantic owners.
#[derive(Debug)]
pub(crate) struct SkeletonAssemblyBinding {
    body: Arc<SkeletonBodyDefinition>,
    inertias: [SimulationMutableInertia; SKELETON_PART_COUNT],
    joint_limits: Vec<SimulationMutableJointLimits>,
    emitted_for_part: [Option<usize>; SKELETON_PART_COUNT],
    initial_culling: Arc<SkeletonCullingResource>,
}

impl SkeletonAssemblyBinding {
    pub(crate) fn body(&self) -> &Arc<SkeletonBodyDefinition> {
        &self.body
    }

    pub(crate) fn inertias(&self) -> &[SimulationMutableInertia; SKELETON_PART_COUNT] {
        &self.inertias
    }

    pub(crate) fn joint_limits(&self) -> &[SimulationMutableJointLimits] {
        &self.joint_limits
    }

    pub(crate) fn emitted_for_part(&self) -> &[Option<usize>; SKELETON_PART_COUNT] {
        &self.emitted_for_part
    }

    /// Definition culling selected at 0x82b620d0, then retained by component
    /// initialization at 0x82ae0de4. This is not mutable current-mode state or
    /// a claim that this table has reached the collision consumer yet.
    pub(crate) fn initial_culling(&self) -> &Arc<SkeletonCullingResource> {
        &self.initial_culling
    }
}

#[derive(Debug)]
pub(crate) enum SkeletonAssemblyBindingError {
    Allocation,
    WrongCullingResource {
        actual: SkeletonCullingKey,
    },
    JointAllocationMismatch {
        body: usize,
        joints: usize,
    },
    UninitializedJointAllocation {
        allocated: usize,
        initialized: usize,
    },
    MissingInverseBodyFrame {
        part: usize,
    },
    Template(DynamicAssemblyTemplateError),
    Session(DynamicAssemblySessionCreateError),
}

impl std::fmt::Display for SkeletonAssemblyBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allocation => write!(formatter, "could not allocate Skeleton assembly binding"),
            Self::WrongCullingResource { actual } => write!(
                formatter,
                "Skeleton initial culling must be Normal, received {actual:?}"
            ),
            Self::JointAllocationMismatch { body, joints } => write!(
                formatter,
                "Skeleton body allocated {body} joints but joint producer retained {joints}"
            ),
            Self::UninitializedJointAllocation {
                allocated,
                initialized,
            } => write!(
                formatter,
                "Skeleton joint allocation has {initialized} initialized records for {allocated} slots"
            ),
            Self::MissingInverseBodyFrame { part } => write!(
                formatter,
                "Skeleton Part {part} has an inverse-frame flag without initialized storage"
            ),
            Self::Template(source) => write!(formatter, "Skeleton template: {source}"),
            Self::Session(source) => write!(formatter, "Skeleton session: {source}"),
        }
    }
}

impl std::error::Error for SkeletonAssemblyBindingError {}

/// Binds accepted source-produced body/joint definitions in native array order.
/// A source allocation with unwritten joint tails is deliberately refused;
/// filling it with identity joints or shrinking its declared count is not
/// source behavior. Allocation/presence refusals are host preflight policy.
/// The caller subsequently uses the ordinary session attachment transaction.
/// Caller invariant: `joints` was produced from this SAME body's `SkeletonData`,
/// attributes and allocation choice. Count/index checks cannot establish that
/// source provenance. The supplied normal resource is the caller's existing
/// inventory allocation; this function neither creates nor recomputes it.
///
/// No tuning is fetched here and no mass is recomputed. Mutable inertia begins
/// with the producer's CURRENT mass state, after its debug adjustments, rather
/// than either saved mass cache or `PartDefinition`'s earlier default inertia.
pub(crate) fn try_create_pre_normal_skeleton_assembly(
    body: Arc<SkeletonBodyDefinition>,
    joints: SkeletonBodyJoints,
    initial_culling: Arc<SkeletonCullingResource>,
) -> Result<SkeletonPreNormalAssemblySession, SkeletonAssemblyBindingError> {
    use SkeletonAssemblyBindingError as Error;
    if initial_culling.key() != SkeletonCullingKey::Normal {
        return Err(Error::WrongCullingResource {
            actual: initial_culling.key(),
        });
    }
    if body.allocated_joint_count != joints.allocated_count {
        return Err(Error::JointAllocationMismatch {
            body: body.allocated_joint_count,
            joints: joints.allocated_count,
        });
    }
    if joints.joints.len() != joints.allocated_count {
        return Err(Error::UninitializedJointAllocation {
            allocated: joints.allocated_count,
            initialized: joints.joints.len(),
        });
    }
    let mut metadata = Vec::new();
    let mut payloads = Vec::new();
    let mut bound_joints = Vec::new();
    let mut joint_limits = Vec::new();
    let mut initial = Vec::new();
    metadata
        .try_reserve_exact(SKELETON_PART_COUNT)
        .map_err(|_| Error::Allocation)?;
    payloads
        .try_reserve_exact(SKELETON_PART_COUNT)
        .map_err(|_| Error::Allocation)?;
    initial
        .try_reserve_exact(SKELETON_PART_COUNT)
        .map_err(|_| Error::Allocation)?;
    bound_joints
        .try_reserve_exact(joints.joints.len())
        .map_err(|_| Error::Allocation)?;
    joint_limits
        .try_reserve_exact(joints.joints.len())
        .map_err(|_| Error::Allocation)?;
    for (part, definition) in body.parts.iter().enumerate() {
        let mass = &definition.mass;
        if mass.has_inverse_body_transform && mass.inverse_body_transform_storage.is_none() {
            return Err(Error::MissingInverseBodyFrame { part });
        }
        metadata.push(DynamicAssemblyPartAttachmentMetadata {
            // Absent native storage is not initialized here: this canonical
            // host value is unreachable through the source presence gate.
            inverse_body_transform: mass
                .inverse_body_transform_storage
                .unwrap_or(FRESH_VOLUME_TRANSFORM),
            has_inverse_body_transform: u32::from(mass.has_inverse_body_transform),
            // PartDefinition::Initialize explicitly stores zero at 82ae90d8.
            is_static_body: 0,
        });
    }
    let inertias = std::array::from_fn(|part| {
        SimulationMutableInertia::new_packed(
            body.parts[part].mass.inertia,
            body.parts[part].mass.inverse_tensor_w,
        )
    });
    for (part, inertia) in body.parts.iter().zip(&inertias) {
        payloads.push(DynamicAssemblyPartPayload {
            inertia: SimulationAssemblyInertia::mutable(inertia.clone()),
            material: part.material,
        });
        initial.push(FRESH_VOLUME_TRANSFORM);
    }
    for joint in joints.joints {
        let limits = SimulationMutableJointLimits::new(joint.limits);
        bound_joints.push(DynamicAssemblyJointDefinition {
            parent_part: joint.parent_part,
            child_part: joint.child_part,
            frames: Arc::new(joint.frames),
            limits: SimulationAssemblyJointLimits::mutable(limits.clone()),
        });
        joint_limits.push(limits);
    }
    let attachment = DynamicAssemblyPartAttachmentTemplate::try_from_parts(metadata, payloads)
        .expect("one metadata and payload record per actual Skeleton Part");
    let binding = Arc::new(SkeletonAssemblyBinding {
        body,
        inertias,
        joint_limits,
        emitted_for_part: joints.emitted_for_part,
        initial_culling,
    });
    let template = DynamicAssemblyRuntimeTemplate::try_from_owned_parts(
        binding,
        Arc::new(attachment),
        bound_joints,
        Vec::new(),
    )
    .map_err(Error::Template)?;
    SkeletonPreNormalAssemblySession::try_new(Arc::new(template), initial).map_err(Error::Session)
}
