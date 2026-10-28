//! Real retained Skeleton assembly construction from the paired player's inputs.
//!
//! Source `Skeleton::Init` (0x82b62710) enters `SkeletonData::Init` first;
//! `SkeletonBody` (0x82b62200) then creates collision data before `InitAttribData`
//! and its geometry/joints/culling/Assembly. The enclosing physical constructor
//! invokes this producer only AFTER its real board has completed construction.
//! This module neither admits a member nor invents IK, head-tracking, Actor,
//! processed-input, Body/State pointers, or the omitted whole Skeleton services.
//!
//! Existing leaf producers retain their checked-host preflight/allocation and
//! immutable Vault policies, including absence of partial leaf outputs on error.
//! Every completed output is retained here. Host sqrt/reciprocal/trig/dot policies
//! are explicit approximations, not Xenon bit-parity. The actual owner retains
//! bouncy/human input bytes; their downstream behavior is not implemented here.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained skeleton producer awaits paired physical construction"
    )
)]

use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    simulation_volume_integral::HostSimulationVolumeIntegralNumeric,
    skeleton_assembly::SkeletonAssemblyBindingError,
    skeleton_attributes::{
        SkeletonAttributeLoadError, SkeletonAttributes, SkeletonBodyAttributeData,
        SkeletonDriveAttributes,
    },
    skeleton_body_attributes::{DEFAULT_COLLECTION, SkeletonBodyAttributes},
    skeleton_body_definition::{SkeletonBodyDefinition, SkeletonBodyDefinitionError},
    skeleton_body_joints::{HostSkeletonJointNumeric, SkeletonBodyJoints, SkeletonJointError},
    skeleton_body_mass::HostSkeletonBodyMassNumeric,
    skeleton_collision_attributes::SkeletonCollisionAttributes,
    skeleton_collision_data::SkeletonCollisionData,
    skeleton_culling::{
        SkeletonCullingError, SkeletonCullingInventory, SkeletonCullingKey, SkeletonCullingResource,
    },
    skeleton_data::{SkeletonData, SkeletonDataError, SkeletonPoseView},
    skeleton_hat::{HatAttributes, HatLayout, HostHatNumeric},
    skeleton_joint_attributes::SkeletonJointAttributeProvider,
};
use std::{convert::Infallible, fmt, sync::Arc};

#[path = "../movement/skeleton_constructor_runtime.rs"]
pub(crate) mod constructor;

#[cfg(test)]
#[path = "skeleton_assembly_construction_tests.rs"]
mod tests;

/// Current operation, retained on failure; not a resumable native program counter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkeletonAssemblyConstructionPhase {
    Fresh,
    Data,
    CollisionData,
    Attributes,
    HatLayout,
    Body,
    Joints,
    NormalCulling,
    RagdollCulling,
    Assembly,
    Complete,
    HandedOff,
}

#[derive(Debug)]
pub(crate) enum SkeletonAssemblyConstructionError {
    WrongPhase(SkeletonAssemblyConstructionPhase),
    Data(SkeletonDataError),
    InitIndices(constructor::init_state::SkeletonInitIndicesError),
    CollisionData(AttributeRuntimeError),
    Attributes(SkeletonAttributeLoadError<AttributeRuntimeError>),
    HatLayout(AttributeRuntimeError),
    Body(SkeletonBodyDefinitionError<AttributeRuntimeError, Infallible, Infallible>),
    Joints(SkeletonJointError<AttributeRuntimeError, Infallible>),
    Culling(SkeletonCullingError),
    Assembly(SkeletonAssemblyBindingError),
}

impl fmt::Display for SkeletonAssemblyConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPhase(phase) => write!(f, "skeleton construction cannot run in {phase:?}"),
            Self::Data(error) => write!(f, "skeleton mapping: {error}"),
            Self::InitIndices(error) => write!(f, "skeleton initialization: {error}"),
            Self::CollisionData(error) => write!(f, "skeleton collision data: {error}"),
            Self::Attributes(error) => write!(f, "skeleton attributes: {error:?}"),
            Self::HatLayout(error) => write!(f, "skeleton hat layout: {error}"),
            Self::Body(error) => write!(f, "skeleton body: {error:?}"),
            Self::Joints(error) => write!(f, "skeleton joints: {error:?}"),
            Self::Culling(error) => write!(f, "skeleton culling: {error}"),
            Self::Assembly(error) => write!(f, "skeleton assembly: {error}"),
        }
    }
}
impl std::error::Error for SkeletonAssemblyConstructionError {}

/// Owns the same paired physical input lifetime before and after assembly
/// handoff. The caller transfers this remaining owner WITH its actual assembly,
/// including failed admission/removal. No arbitrary prebuilt Body is accepted.
pub(crate) struct SkeletonAssemblyConstruction {
    pub(crate) constructor: constructor::SkeletonConstructorRuntime,
    owner: Arc<PhysicalPlayerAssemblyOwner>,
    island_body_id: Option<u32>,
    attempted: bool,
    phase: SkeletonAssemblyConstructionPhase,
    data: Option<Arc<SkeletonData>>,
    collision_reader: Option<SkeletonCollisionAttributes>,
    collision_data: Option<SkeletonCollisionData>,
    attribute_reader: Option<SkeletonAttributes>,
    attributes: Option<Arc<SkeletonBodyAttributeData>>,
    body_reader: Option<SkeletonBodyAttributes>,
    joint_reader: Option<SkeletonJointAttributeProvider>,
    hat_reader: Option<HatAttributes>,
    hat_layout: Option<HatLayout>,
    body: Option<Arc<SkeletonBodyDefinition>>,
    joints: Option<SkeletonBodyJoints>,
    normal_culling: Option<Arc<SkeletonCullingResource>>,
    ragdoll_culling: Option<Arc<SkeletonCullingResource>>,
    assembly: Option<SkeletonNormalAssembly>,
}

pub(crate) type SkeletonRagdollConstructionInputs<'a> = (
    u8,
    Arc<SkeletonCullingResource>,
    &'a mut SkeletonBodyAttributes,
    &'a mut SkeletonJointAttributeProvider,
);

impl SkeletonAssemblyConstruction {
    pub(crate) fn postphysics_inputs_mut(
        &mut self,
    ) -> Option<(
        &mut constructor::SkeletonOuterRemainder,
        &mut SkeletonBodyAttributes,
    )> {
        Some((self.constructor.outer.as_mut()?, self.body_reader.as_mut()?))
    }

    /// Same constructor inputs/readers retained across Assembly admission.
    pub(crate) fn ragdoll_inputs_mut(&mut self) -> Option<SkeletonRagdollConstructionInputs<'_>> {
        Some((
            self.constructor.bindings.as_ref()?.bouncy_ragdoll?,
            Arc::clone(self.ragdoll_culling.as_ref()?),
            self.body_reader.as_mut()?,
            self.joint_reader.as_mut()?,
        ))
    }

    pub(crate) fn new(owner: Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            constructor: constructor::SkeletonConstructorRuntime::new(),
            owner,
            island_body_id: None,
            attempted: false,
            phase: SkeletonAssemblyConstructionPhase::Fresh,
            data: None,
            collision_reader: None,
            collision_data: None,
            attribute_reader: None,
            attributes: None,
            body_reader: None,
            joint_reader: None,
            hat_reader: None,
            hat_layout: None,
            body: None,
            joints: None,
            normal_culling: None,
            ragdoll_culling: None,
            assembly: None,
        }
    }

    /// The inventory belongs to the SAME selected host world; it is not freshly
    /// invented per skeleton. Failed find/create retains earlier inventory and
    /// completed constructor outputs. The hat callback performs both original
    /// live enable reads only when the actual nonzero-hat shape path reaches them.
    #[expect(
        clippy::too_many_lines,
        reason = "one native-ordered construction retaining every successful producer and provider"
    )]
    pub(crate) fn try_construct(
        &mut self,
        inventory: &mut SkeletonCullingInventory,
        hat_enabled: &mut impl FnMut() -> bool,
    ) -> Result<(), SkeletonAssemblyConstructionError> {
        use SkeletonAssemblyConstructionError as Error;
        use SkeletonAssemblyConstructionPhase as Phase;
        if self.attempted {
            return Err(Error::WrongPhase(self.phase));
        }
        self.attempted = true;
        let inputs = self.owner.inputs();
        self.phase = Phase::Data;
        self.data = Some(Arc::new(
            SkeletonData::try_new(
                Arc::clone(&inputs.hierarchy),
                Arc::clone(&inputs.parameters),
                SkeletonPoseView {
                    hierarchy: inputs.pose_hierarchy.as_ref(),
                    matrices: inputs.pose_matrices.as_ref(),
                },
            )
            .map_err(Error::Data)?,
        ));

        if let Some(current) = self.constructor.current_animation.as_mut() {
            current.bind_initialized_skeleton_data(Arc::clone(
                self.data.as_ref().expect("completed Data.Init"),
            ));
            current.initialize_ik_targets();
        }
        self.constructor
            .init
            .try_initialize(self.data.as_ref().expect("completed Data.Init"))
            .map_err(Error::InitIndices)?;

        self.phase = Phase::CollisionData;
        self.collision_reader = Some(SkeletonCollisionAttributes::bind(&inputs.vault));
        self.collision_data = Some(
            SkeletonCollisionData::try_new(
                self.collision_reader
                    .as_mut()
                    .expect("retained collision reader"),
            )
            .map_err(Error::CollisionData)?,
        );

        self.phase = Phase::Attributes;
        self.attribute_reader = Some(SkeletonAttributes::bind(&inputs.vault, DEFAULT_COLLECTION));
        self.attributes = Some(Arc::new(
            SkeletonBodyAttributeData::try_read(
                self.attribute_reader
                    .as_mut()
                    .expect("retained attribute reader"),
                SkeletonDriveAttributes::constructor_value(),
            )
            .map_err(Error::Attributes)?,
        ));
        self.body_reader = Some(SkeletonBodyAttributes::bind(
            &inputs.vault,
            DEFAULT_COLLECTION,
        ));
        self.joint_reader = Some(SkeletonJointAttributeProvider::bind(
            &inputs.vault,
            DEFAULT_COLLECTION,
        ));
        self.hat_reader = Some(HatAttributes::bind(&inputs.vault, inputs.hat_type));
        self.phase = Phase::HatLayout;
        if inputs.hat_type != 0 {
            self.hat_layout = Some(
                self.hat_reader
                    .as_ref()
                    .expect("retained hat reader")
                    .try_layout()
                    .map_err(Error::HatLayout)?,
            );
        }
        self.phase = Phase::Body;
        self.body = Some(Arc::new(
            SkeletonBodyDefinition::try_create(
                Arc::clone(self.data.as_ref().expect("completed mapping")),
                Arc::clone(self.attributes.as_ref().expect("completed attributes")),
                self.body_reader.as_mut().expect("retained body reader"),
                inputs.hat_type,
                self.hat_layout.as_ref(),
                hat_enabled,
                &mut HostSkeletonBodyMassNumeric,
                &mut HostSimulationVolumeIntegralNumeric,
                &mut HostHatNumeric,
            )
            .map_err(Error::Body)?,
        ));
        self.phase = Phase::Joints;
        let body = self.body.as_ref().expect("completed actual body");
        self.joints = Some(
            SkeletonBodyJoints::try_create(
                &body.data,
                &body.attributes,
                body.allocated_joint_count,
                self.joint_reader.as_mut().expect("retained joint reader"),
                &mut HostSkeletonJointNumeric,
            )
            .map_err(Error::Joints)?,
        );
        self.phase = Phase::NormalCulling;
        self.normal_culling = Some(
            inventory
                .try_get_or_create(SkeletonCullingKey::Normal)
                .map_err(Error::Culling)?,
        );
        self.phase = Phase::RagdollCulling;
        self.ragdoll_culling = Some(
            inventory
                .try_get_or_create(SkeletonCullingKey::Ragdoll)
                .map_err(Error::Culling)?,
        );
        self.phase = Phase::Assembly;
        let joints = self.joints.as_ref().expect("completed actual joints");
        // Existing binder consumes its inputs on a checked failure. Inert
        // construction copies preserve this completed prefix. After success
        // originals are immediately removed: never two admitted mutable states.
        let candidate_joints = SkeletonBodyJoints {
            joints: joints.joints.clone(),
            allocated_count: joints.allocated_count,
            emitted_for_part: joints.emitted_for_part,
        };
        let mut assembly = SkeletonNormalAssembly::try_create(
            Arc::clone(body),
            candidate_joints,
            Arc::clone(
                self.normal_culling
                    .as_ref()
                    .expect("completed normal culling"),
            ),
            self.collision_data
                .as_ref()
                .expect("completed collision data")
                .clone(),
        )
        .map_err(Error::Assembly)?;
        assembly.retain_jump_adjustments(
            self.constructor
                .jump_adjustments
                .take()
                .expect("same selected adjustment storage"),
        );
        assembly.retain_constructor_runtime(&mut self.constructor, &self.owner.inputs().vault);
        self.collision_data = None;
        self.joints = None;
        self.assembly = Some(assembly);
        self.phase = Phase::Complete;
        Ok(())
    }

    pub(crate) fn owner(&self) -> &Arc<PhysicalPlayerAssemblyOwner> {
        &self.owner
    }
    /// Source Body ID survives membership cleanup and returned-candidate ownership.
    pub(crate) fn island_body_id(&self) -> Option<u32> {
        self.island_body_id
    }
    pub(crate) fn set_island_body_id(&mut self, id: u32) {
        self.island_body_id = Some(id);
    }
    pub(crate) fn phase(&self) -> SkeletonAssemblyConstructionPhase {
        self.phase
    }
    pub(crate) fn data(&self) -> Option<&Arc<SkeletonData>> {
        self.data.as_ref()
    }
    pub(crate) fn attributes(&self) -> Option<&Arc<SkeletonBodyAttributeData>> {
        self.attributes.as_ref()
    }
    pub(crate) fn body(&self) -> Option<&Arc<SkeletonBodyDefinition>> {
        self.body.as_ref()
    }
    pub(crate) fn assembly(&self) -> Option<&SkeletonNormalAssembly> {
        self.assembly.as_ref()
    }
    pub(crate) fn body_reader_mut(&mut self) -> Option<&mut SkeletonBodyAttributes> {
        self.body_reader.as_mut()
    }
    pub(crate) fn body_reader(&self) -> Option<&SkeletonBodyAttributes> {
        self.body_reader.as_ref()
    }
    pub(crate) fn initial_culling(&self) -> Option<&Arc<SkeletonCullingResource>> {
        self.normal_culling.as_ref()
    }
    pub(crate) fn ragdoll_culling(&self) -> Option<&Arc<SkeletonCullingResource>> {
        self.ragdoll_culling.as_ref()
    }

    /// One-way transfer into the real candidate; this retained construction and
    /// its shared physical owner must accompany the assembly through membership.
    pub(crate) fn take_assembly(
        &mut self,
    ) -> Result<SkeletonNormalAssembly, SkeletonAssemblyConstructionError> {
        if self.phase != SkeletonAssemblyConstructionPhase::Complete {
            return Err(SkeletonAssemblyConstructionError::WrongPhase(self.phase));
        }
        let assembly = self
            .assembly
            .take()
            .expect("successful construction retains actual assembly");
        self.phase = SkeletonAssemblyConstructionPhase::HandedOff;
        Ok(assembly)
    }
}
