//! Staged source ownership for body-only DMO Assembly Part attachment.
//!
//! This module binds one exact dictionary-filtered Assembly template to one
//! source-aware Simulation backend, then owns only the proven Part lifecycle.
//! Initial Part transforms and requested body state remain explicit caller
//! inputs rather than claimed live DMO values. No body handle, routed payload,
//! attachment batch, allocator, catalog, world index, or Create authority is
//! exposed.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged owner awaits the production DMO spawn caller"
    )
)]

#[path = "../solver/simulation_dmo_locator_registry.rs"]
mod locator_registry;
#[path = "../solver/simulation_dmo_bank_collection_binding.rs"]
mod main_bank_collection_binding;
#[path = "../solver/simulation_dmo_bank_model_registration_batch.rs"]
mod model_registration_batch;
#[path = "../solver/simulation_dmo_model_registry.rs"]
mod model_registry;
#[path = "../solver/simulation_dmo_registration_manager.rs"]
mod registration_manager;
#[path = "../solver/simulation_dmo_registration_runtime.rs"]
mod registration_runtime;
#[path = "../../streaming/dmo_bank_resident_model_catalog.rs"]
mod resident_model_catalog;

#[path = "simulation_dmo_dynamic_pool.rs"]
pub(crate) mod dynamic_pool;

#[cfg(test)]
#[path = "../solver/simulation_dmo_pool_corpus.rs"]
pub(crate) mod pool_corpus;

use locator_registry::SimulationDmoLocatorRegistry;
#[allow(
    unused_imports,
    reason = "staged mutation capabilities await the normal-arena registration adapter"
)]
pub(crate) use locator_registry::{
    SimulationDmoLocatorRegisterError, SimulationDmoLocatorRegistryInitError,
    SimulationDmoRegisteredLocatorOccurrenceError,
};
pub use main_bank_collection_binding::{
    MainDmoSimulationDeferredDeregistrationAcknowledgementError,
    MainDmoSimulationDeferredDeregistrationQueueError,
    MainDmoSimulationDeferredDeregistrationQueueFailure, MainDmoSimulationDeferredDispatch,
    MainDmoSimulationDeferredDispatchError, MainDmoSimulationDeferredMarkerKind,
    MainDmoSimulationDeferredProducer, MainDmoSimulationDeferredProducerFailure,
    MainDmoSimulationDeferredProducerFailureKind, MainDmoSimulationDeferredProducerUpdate,
    MainDmoSimulationDeferredRegistrationDriver,
};
#[allow(
    unused_imports,
    reason = "staged source batch awaits the live DMO registration adapter"
)]
pub(crate) use model_registration_batch::{
    SimulationDmoBankModelRegistrationBatch, SimulationDmoBankModelRegistrationBatchError,
    SimulationDmoBankModelRegistrationPassError, SimulationDmoBankSnapshotConsumer,
};
use model_registry::SimulationDmoModelRegistry;
#[allow(
    unused_imports,
    reason = "staged mutation capabilities await the normal-arena registration adapter"
)]
pub(crate) use model_registry::{
    SimulationDmoModelCurrentWinner, SimulationDmoModelRegisterError,
    SimulationDmoModelRegistrationSource, SimulationDmoModelRegistrationSourceError,
    SimulationDmoModelRegistryInitError, SimulationDmoModelRegistryResolveError,
    SimulationDmoModelRegistryResolver,
};
#[allow(
    unused_imports,
    reason = "staged manager owner awaits the live DMO registration adapter"
)]
pub(crate) use registration_manager::{
    SimulationDmoRegistrationManager, SimulationDmoRegistrationManagerInitError,
    SimulationDmoRegistrationManagerResolveError,
};
#[allow(
    unused_imports,
    reason = "remaining staged synchronous pass exports await the outer collection coordinator"
)]
pub(crate) use registration_runtime::{
    SimulationDmoBankModelDeregisterController, SimulationDmoBankModelDeregisterPass,
    SimulationDmoBankModelDeregisterPassError, SimulationDmoBankModelRegisterController,
    SimulationDmoBankModelRegisterPass, SimulationDmoBankModelRegisterPassError,
    SimulationDmoRegistrationRuntime, SimulationDmoRegistrationRuntimeInitError,
};
pub(crate) use resident_model_catalog::DmoBankResidentStrongModelSource;
pub use resident_model_catalog::{
    DmoBankResidentModelCatalog, DmoBankResidentModelCatalogError,
    DmoBankResidentModelRoutePosition,
};

use crate::{
    dmo_bank_processed_asset::{DmoBankProcessedArenaAsset, DmoSerialisedAssemblySelectError},
    dynamic_assembly_session::{
        DynamicAssemblyBodyOnlyPartAttachError, DynamicAssemblyBodyOnlyPartDetachError,
        DynamicAssemblyBodyOnlyPartSession, DynamicAssemblyPartAttachmentTemplate,
        DynamicAssemblySessionCreateError,
    },
    owned_resident_collection::OwnedResidentCollectionError,
    serialised_assembly_template::SerialisedAssemblyTemplate,
    simulation_assembly_backend::{
        SerialisedAssemblyPartAttachmentBindError, SimulationAssemblyBackend,
    },
    simulation_assembly_payload::SimulationAssemblyInertia,
    simulation_dmo_arena_plan::SimulationDmoLocatorOccurrence,
    simulation_joint_pool::{SimulationJointFrames, SimulationJointLimits},
    simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority,
};
use skate2_formats::{collision::AssemblyPartMaterial, pegasus::ObjectId};
use std::{error::Error, fmt, sync::Arc};

type SourceBackend =
    SimulationAssemblyBackend<AssemblyPartMaterial, SimulationJointFrames, SimulationJointLimits>;
type SourceAttachment =
    DynamicAssemblyPartAttachmentTemplate<SimulationAssemblyInertia, AssemblyPartMaterial>;

/// Effect-free refusal before a selected DMO Assembly Part owner is published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoAssemblyPartOwnerError {
    Selection(DmoSerialisedAssemblySelectError),
    TemplateOwnerMismatch { object_id: ObjectId, ordinal: usize },
    Attachment(SerialisedAssemblyPartAttachmentBindError),
}

impl fmt::Display for SimulationDmoAssemblyPartOwnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Selection(source) => write!(formatter, "could not select DMO Assembly: {source}"),
            Self::TemplateOwnerMismatch { object_id, ordinal } => write!(
                formatter,
                "DMO Assembly {:#010x} at ordinal {ordinal} belongs to another arena owner",
                object_id.0
            ),
            Self::Attachment(source) => {
                write!(formatter, "could not bind DMO Assembly Parts: {source}")
            }
        }
    }
}

impl Error for SimulationDmoAssemblyPartOwnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Selection(source) => Some(source),
            Self::Attachment(source) => Some(source),
            Self::TemplateOwnerMismatch { .. } => None,
        }
    }
}

/// Attach refusal at the source-bound, backend-affine Part-session boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoAssemblyPartAttachError {
    AlreadyAttached,
    BackendAuthorityMismatch,
    PartUnavailable { ordinal: usize },
}

impl fmt::Display for SimulationDmoAssemblyPartAttachError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyAttached => formatter.write_str("DMO Assembly Parts are attached"),
            Self::BackendAuthorityMismatch => {
                formatter.write_str("DMO Assembly Parts are bound to another backend authority")
            }
            Self::PartUnavailable { ordinal } => {
                write!(formatter, "DMO Assembly Part {ordinal} could not attach")
            }
        }
    }
}

impl Error for SimulationDmoAssemblyPartAttachError {}

/// Immutable selected DMO/template/batch association for one backend domain.
pub(crate) struct SimulationDmoAssemblyPartOwner {
    asset: Arc<DmoBankProcessedArenaAsset>,
    template_ordinal: usize,
    assembly_object_id: ObjectId,
    /// A LivingWorld model's HiLOD Definition after its in-place Part-0 edit
    /// (`LWDynamicObjectModel::ApplyAttribulatorOffsets` 0x82d31e28); every
    /// Assembly created from that shared Definition reads the edited copy.
    template_override: Option<Arc<SerialisedAssemblyTemplate>>,
    attachment: Arc<SourceAttachment>,
    backend_authority: SimulationRigidBodyPoolAuthority,
}

impl SimulationDmoAssemblyPartOwner {
    pub(crate) fn template(&self) -> &SerialisedAssemblyTemplate {
        if let Some(template) = &self.template_override {
            return template;
        }
        let (ordinal, template) = self
            .asset
            .try_serialised_assembly_by_object_id(self.assembly_object_id)
            .expect("selected immutable DMO template stays present and unique");
        debug_assert_eq!(ordinal, self.template_ordinal);
        template
    }

    /// `cLWDynamicObjectCollision::InitCollisionBodies` 0x82d3aa28 creates
    /// every body of one LivingWorld model from its edited HiLOD Definition.
    /// The edited view must share the selected source Assembly's arena owner.
    pub(crate) fn try_new_living_world(
        asset: Arc<DmoBankProcessedArenaAsset>,
        assembly_object_id: ObjectId,
        template: Arc<SerialisedAssemblyTemplate>,
        backend: &SourceBackend,
    ) -> Result<Arc<Self>, SimulationDmoAssemblyPartOwnerError> {
        let (template_ordinal, selected) = asset
            .try_serialised_assembly_by_object_id(assembly_object_id)
            .map_err(SimulationDmoAssemblyPartOwnerError::Selection)?;
        let asset_plan = asset.plan_owner();
        if !Arc::ptr_eq(selected.owner(), &asset_plan)
            || !Arc::ptr_eq(template.owner(), &asset_plan)
            || template.assembly_object_id() != assembly_object_id
        {
            return Err(SimulationDmoAssemblyPartOwnerError::TemplateOwnerMismatch {
                object_id: assembly_object_id,
                ordinal: template_ordinal,
            });
        }
        let attachment = backend
            .try_bind_serialised_part_attachment_template(&template)
            .map_err(SimulationDmoAssemblyPartOwnerError::Attachment)?;
        let backend_authority = backend.authority();
        drop(asset_plan);
        Ok(Arc::new(Self {
            asset,
            template_ordinal,
            assembly_object_id,
            template_override: Some(template),
            attachment,
            backend_authority,
        }))
    }

    /// Selects, defensively owner-checks, and effect-free binds one exact ID.
    pub(crate) fn try_new(
        source: DmoBankResidentStrongModelSource,
        backend: &SourceBackend,
    ) -> Result<Arc<Self>, SimulationDmoAssemblyPartOwnerError> {
        source.try_bind_part_owner(backend)
    }

    fn try_new_from_validated_route(
        asset: Arc<DmoBankProcessedArenaAsset>,
        assembly_object_id: ObjectId,
        backend: &SourceBackend,
    ) -> Result<Arc<Self>, SimulationDmoAssemblyPartOwnerError> {
        let (template_ordinal, template) = asset
            .try_serialised_assembly_by_object_id(assembly_object_id)
            .map_err(SimulationDmoAssemblyPartOwnerError::Selection)?;
        let asset_plan = asset.plan_owner();
        if !Arc::ptr_eq(template.owner(), &asset_plan) {
            return Err(SimulationDmoAssemblyPartOwnerError::TemplateOwnerMismatch {
                object_id: assembly_object_id,
                ordinal: template_ordinal,
            });
        }
        let attachment = backend
            .try_bind_serialised_part_attachment_template(template)
            .map_err(SimulationDmoAssemblyPartOwnerError::Attachment)?;
        let backend_authority = backend.authority();
        drop(asset_plan);
        Ok(Arc::new(Self {
            asset,
            template_ordinal,
            assembly_object_id,
            template_override: None,
            attachment,
            backend_authority,
        }))
    }

    /// Constructs detached mutable Part state without touching the backend.
    pub(crate) fn try_begin_session(
        self: &Arc<Self>,
        initial_part_transforms: Vec<[f32; 16]>,
    ) -> Result<SimulationDmoAssemblyPartSession, DynamicAssemblySessionCreateError> {
        Ok(SimulationDmoAssemblyPartSession {
            owner: Arc::clone(self),
            parts: DynamicAssemblyBodyOnlyPartSession::try_new(
                Arc::clone(&self.attachment),
                initial_part_transforms,
            )?,
        })
    }
}

/// Opaque mutable Part lifecycle retaining its exact selected source owner.
pub(crate) struct SimulationDmoAssemblyPartSession {
    owner: Arc<SimulationDmoAssemblyPartOwner>,
    parts: DynamicAssemblyBodyOnlyPartSession<SourceBackend>,
}

impl SimulationDmoAssemblyPartSession {
    pub(crate) const fn is_attached(&self) -> bool {
        self.parts.is_attached()
    }

    pub(crate) fn part_transforms(&self) -> impl ExactSizeIterator<Item = &[f32; 16]> {
        self.parts.part_transforms()
    }

    pub(crate) fn owner(&self) -> &Arc<SimulationDmoAssemblyPartOwner> {
        &self.owner
    }

    /// `Part::GetTransform` 0x826c3480 of every Part: attached bodies through
    /// the source copyback rule, detached Parts from their cache.
    pub(crate) fn world_part_transforms(&self, backend: &SourceBackend) -> Vec<[f32; 16]> {
        self.parts.world_part_transforms(backend)
    }

    pub(crate) fn try_grab_part_snapshot(
        &self,
        backend: &SourceBackend,
        index: usize,
    ) -> Result<
        crate::dynamic_assembly_session::source_parts::SourcePartGrabSnapshot,
        crate::dynamic_assembly_session::source_parts::DynamicAssemblySourcePartError,
    > {
        self.parts.try_grab_part_snapshot(backend, index)
    }

    /// Attaches after `AlreadyAttached` and bound-backend preflight.
    pub(crate) fn attach(
        &mut self,
        backend: &mut SourceBackend,
        requested_body_state: u32,
    ) -> Result<(), SimulationDmoAssemblyPartAttachError> {
        if self.parts.is_attached() {
            return Err(SimulationDmoAssemblyPartAttachError::AlreadyAttached);
        }
        if backend.authority() != self.owner.backend_authority {
            return Err(SimulationDmoAssemblyPartAttachError::BackendAuthorityMismatch);
        }
        self.parts
            .attach(backend, requested_body_state)
            .map_err(|source| match source {
                DynamicAssemblyBodyOnlyPartAttachError::AlreadyAttached => {
                    SimulationDmoAssemblyPartAttachError::AlreadyAttached
                }
                DynamicAssemblyBodyOnlyPartAttachError::PartUnavailable { ordinal } => {
                    SimulationDmoAssemblyPartAttachError::PartUnavailable { ordinal }
                }
            })
    }

    /// Detaches after `NotAttached`, bound-backend, then attached-authority checks.
    pub(crate) fn detach(
        &mut self,
        backend: &mut SourceBackend,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartDetachError> {
        if !self.parts.is_attached() {
            return Err(DynamicAssemblyBodyOnlyPartDetachError::NotAttached);
        }
        if backend.authority() != self.owner.backend_authority {
            return Err(DynamicAssemblyBodyOnlyPartDetachError::BackendAuthorityMismatch);
        }
        self.parts.detach(backend)
    }
}

/// Effect-free refusal while consuming one locator occurrence into its exact
/// resident model and backend-bound Part owner.
#[derive(Debug)]
pub(crate) enum SimulationDmoResolvedLocatorPartOwnerError {
    ManagerMissing,
    ResidentExpired,
    ResidentAccess(OwnedResidentCollectionError),
    CatalogMissingModel,
    ExactWinnerMismatch,
    AssetExpired,
    PartOwner(SimulationDmoAssemblyPartOwnerError),
}

impl fmt::Display for SimulationDmoResolvedLocatorPartOwnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ManagerMissing => formatter.write_str("DMO model has no manager winner"),
            Self::ResidentExpired => formatter.write_str("resident DMO model catalog expired"),
            Self::ResidentAccess(source) => {
                write!(
                    formatter,
                    "resident DMO model catalog access failed: {source}"
                )
            }
            Self::CatalogMissingModel => {
                formatter.write_str("manager DMO model is absent from the candidate catalog")
            }
            Self::ExactWinnerMismatch => formatter
                .write_str("manager DMO model winner differs from the candidate catalog route"),
            Self::AssetExpired => {
                formatter.write_str("DMO locator model lost its exact resident asset")
            }
            Self::PartOwner(source) => {
                write!(
                    formatter,
                    "could not bind resolved DMO locator Parts: {source}"
                )
            }
        }
    }
}

impl Error for SimulationDmoResolvedLocatorPartOwnerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentAccess(source) => Some(source),
            Self::PartOwner(source) => Some(source),
            _ => None,
        }
    }
}

/// Session-construction refusal after exact locator/model ownership is sealed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoResolvedLocatorPartSessionCreateError {
    PartSession(DynamicAssemblySessionCreateError),
}

impl fmt::Display for SimulationDmoResolvedLocatorPartSessionCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PartSession(source) => {
                write!(
                    formatter,
                    "could not construct resolved DMO Part session: {source}"
                )
            }
        }
    }
}

impl Error for SimulationDmoResolvedLocatorPartSessionCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PartSession(source) => Some(source),
        }
    }
}

/// Opaque association retaining both the exact locator occurrence and its
/// exact resident model's backend-bound Part owner.
pub(crate) struct SimulationDmoResolvedLocatorPartOwner {
    locator: SimulationDmoLocatorOccurrence,
    parts: Arc<SimulationDmoAssemblyPartOwner>,
}

impl SimulationDmoResolvedLocatorPartOwner {
    /// Consumes this occurrence and its private raw model key. Catalog expiry
    /// and exact winner identity are checked before Part-owner/backend binding.
    /// The supplied resident catalog is only a safe-host candidate; native
    /// query and `SpawnStatic` chronology remain outside this staged owner.
    fn try_new(
        locator: SimulationDmoLocatorOccurrence,
        models: &SimulationDmoModelRegistry,
        catalog: &DmoBankResidentModelCatalog,
        backend: &SourceBackend,
    ) -> Result<Arc<Self>, SimulationDmoResolvedLocatorPartOwnerError> {
        let resolver = models.resolver();
        let (locator, winner) = locator
            .try_select_current_model(resolver)
            .map_err(|source| match source {
                SimulationDmoModelRegistryResolveError::MissingWinner => {
                    SimulationDmoResolvedLocatorPartOwnerError::ManagerMissing
                }
            })?;
        let model = catalog
            .try_resolve_current_winner(&winner)
            .map_err(|source| match source {
                resident_model_catalog::DmoBankResidentModelResolveError::ResidentExpired => {
                    SimulationDmoResolvedLocatorPartOwnerError::ResidentExpired
                }
                resident_model_catalog::DmoBankResidentModelResolveError::ResidentAccess(
                    source,
                ) => SimulationDmoResolvedLocatorPartOwnerError::ResidentAccess(source),
                resident_model_catalog::DmoBankResidentModelResolveError::MissingModel => {
                    SimulationDmoResolvedLocatorPartOwnerError::CatalogMissingModel
                }
                resident_model_catalog::DmoBankResidentModelResolveError::RegisteredWinnerMismatch => {
                    SimulationDmoResolvedLocatorPartOwnerError::ExactWinnerMismatch
                }
                resident_model_catalog::DmoBankResidentModelResolveError::AssetExpired => {
                    SimulationDmoResolvedLocatorPartOwnerError::AssetExpired
                }
            })?;
        let parts = SimulationDmoAssemblyPartOwner::try_new(model, backend)
            .map_err(SimulationDmoResolvedLocatorPartOwnerError::PartOwner)?;
        Ok(Arc::new(Self { locator, parts }))
    }

    /// Constructs detached mutable Part state without a backend effect.
    pub(crate) fn try_begin_session(
        self: &Arc<Self>,
        initial_part_transforms: Vec<[f32; 16]>,
    ) -> Result<
        SimulationDmoResolvedLocatorPartSession,
        SimulationDmoResolvedLocatorPartSessionCreateError,
    > {
        let parts = self
            .parts
            .try_begin_session(initial_part_transforms)
            .map_err(SimulationDmoResolvedLocatorPartSessionCreateError::PartSession)?;
        Ok(SimulationDmoResolvedLocatorPartSession {
            owner: Arc::clone(self),
            parts,
        })
    }
}

/// Opaque body-only session retaining the sealed locator/model association.
pub(crate) struct SimulationDmoResolvedLocatorPartSession {
    owner: Arc<SimulationDmoResolvedLocatorPartOwner>,
    parts: SimulationDmoAssemblyPartSession,
}

impl SimulationDmoResolvedLocatorPartSession {
    pub(crate) const fn is_attached(&self) -> bool {
        self.parts.is_attached()
    }

    pub(crate) fn part_transforms(&self) -> impl ExactSizeIterator<Item = &[f32; 16]> {
        self.parts.part_transforms()
    }

    pub(crate) fn attach(
        &mut self,
        backend: &mut SourceBackend,
        requested_body_state: u32,
    ) -> Result<(), SimulationDmoAssemblyPartAttachError> {
        self.parts.attach(backend, requested_body_state)
    }

    pub(crate) fn detach(
        &mut self,
        backend: &mut SourceBackend,
    ) -> Result<(), DynamicAssemblyBodyOnlyPartDetachError> {
        self.parts.detach(backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        serialised_assembly_template::{
            SerialisedAssemblyPartInertiaSource, SerialisedAssemblyTemplate,
            SerialisedAssemblyTemplatePart,
        },
        simulation_assembly_backend::{
            SerialisedAssemblySimulationBindError, SimulationAssemblyBackendAddFailure,
            source::SimulationAssemblySourceError,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::SimulationNormalArenaPlan,
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        simulation_xenon_address_image::{
            SimulationArenaXenonAddressBinding, SimulationXenonAddressImage,
        },
        world::{CollisionArena, CollisionPrimitiveSet},
    };
    use skate2_formats::{
        collision::AssemblyPartInertia,
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    fn arena_owner() -> Arc<SimulationNormalArenaPlan> {
        let resources = [
            vec![0; 0x100],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let descriptors = std::array::from_fn(|index| ResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let arena = Arena {
            file_header: FileHeader {
                magic: *b"\x89RW4xb2\0\r\n\x1a\n",
                is_big_endian: true,
                pointer_size_bits: 32,
                pointer_alignment: 4,
                unused: 0,
                major_version: *b"454\0",
                minor_version: *b"000\0",
                build_number: 0,
            },
            id: 0x8646_a1f7,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 16,
            virtual_size: 0x100,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: descriptors,
            resources_used: descriptors,
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let decoded = DecodedArenaAsset {
            asset_id: 0x0123_4567_89ab_cdef,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            asset_resource_descriptors,
            resources: ArenaResources { resources },
            arena,
        };
        let collision = SimulationCollisionAsset {
            decoded,
            collision: CollisionArena {
                id: 0x8646_a1f7,
                grind_assets: Vec::new(),
                aggregate_roots: Vec::new(),
                primitive_sets: Vec::<CollisionPrimitiveSet>::new(),
                simple_primitive_sets: Vec::new(),
                meshes: Vec::new(),
                collision_models: Vec::new(),
                instances: Vec::new(),
            },
        };
        let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
            collision,
            Vec::new(),
        ));
        Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
    }

    fn binding(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_zero_base: u32,
    ) -> Arc<SimulationArenaXenonAddressBinding> {
        Arc::new(
            SimulationArenaXenonAddressBinding::try_new(owner, [resource_zero_base, 0, 0, 0, 0])
                .unwrap(),
        )
    }

    fn backend(
        capacity: usize,
        routes: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> SourceBackend {
        let image = SimulationXenonAddressImage::try_new(
            0x1000_0080,
            u32::try_from(capacity).unwrap(),
            0,
            0,
        )
        .unwrap();
        let mut backend = SourceBackend::try_new(capacity, 0, 0).unwrap();
        backend.try_bind_source_arenas(image, routes).unwrap();
        backend
    }

    fn part(
        owner: Arc<SimulationNormalArenaPlan>,
        inertia_offset: u32,
        inverse_marker: f32,
    ) -> SerialisedAssemblyTemplatePart {
        let mut inverse = IDENTITY;
        inverse[12] = inverse_marker;
        SerialisedAssemblyTemplatePart {
            volume_object_id: ObjectId(1),
            root_index: 0,
            inertia_source: SerialisedAssemblyPartInertiaSource::from_parts_for_test(
                owner,
                0,
                inertia_offset,
            ),
            inertia: AssemblyPartInertia {
                inverse_tensor: [1.0, 2.0, 3.0, 4.0],
                inverse_mass: 0.25,
                spherical: 1.0,
                max_velocity: 10.0,
                max_omega: 11.0,
                linear_drag: 0.1,
                angular_drag: 0.2,
            },
            inverse_body_transform: inverse,
            material: AssemblyPartMaterial {
                static_friction: 1.0,
                dynamic_friction: 0.5,
                restitution: 0.25,
            },
            has_inverse_body_transform: u32::from(inverse_marker != 0.0),
            is_static_body: 0,
        }
    }

    fn template(
        owner: Arc<SimulationNormalArenaPlan>,
        object_id: ObjectId,
        parts: Vec<SerialisedAssemblyTemplatePart>,
    ) -> SerialisedAssemblyTemplate {
        SerialisedAssemblyTemplate::from_parts_and_constraint_counts_for_test(
            owner, object_id, 0, 0, 0, 0, parts,
        )
    }

    fn asset(
        owner: Arc<SimulationNormalArenaPlan>,
        templates: Vec<SerialisedAssemblyTemplate>,
    ) -> Arc<DmoBankProcessedArenaAsset> {
        Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
            owner, templates,
        ))
    }

    fn model_source(
        asset: Arc<DmoBankProcessedArenaAsset>,
        assembly_object_id: ObjectId,
    ) -> DmoBankResidentStrongModelSource {
        DmoBankResidentStrongModelSource::from_parts_for_test(
            0x0102_0304_0506_0708,
            asset,
            assembly_object_id,
        )
    }

    #[test]
    fn exact_zero_object_id_selects_second_dictionary_filtered_template() {
        let owner = arena_owner();
        let asset = asset(
            Arc::clone(&owner),
            vec![
                template(
                    Arc::clone(&owner),
                    ObjectId(7),
                    vec![part(Arc::clone(&owner), 0x11, 0.0)],
                ),
                template(
                    Arc::clone(&owner),
                    ObjectId(0),
                    vec![part(Arc::clone(&owner), 0x10, 0.0)],
                ),
            ],
        );
        let mut backend = backend(1, vec![(Arc::clone(&owner), binding(owner, 0x2000))]);
        let selected = SimulationDmoAssemblyPartOwner::try_new(
            model_source(Arc::clone(&asset), ObjectId(0)),
            &backend,
        )
        .unwrap();
        assert_eq!(selected.template_ordinal, 1);
        assert_eq!(selected.assembly_object_id, ObjectId(0));
        assert!(Arc::ptr_eq(&selected.asset, &asset));
        let mut session = selected.try_begin_session(vec![IDENTITY]).unwrap();
        session
            .attach(&mut backend, 4)
            .expect("selected A payload must not cross-pair with invalid B");
        session.detach(&mut backend).unwrap();
    }

    #[test]
    fn selector_uses_full_raw_id_and_freezes_nonlocal_missing_duplicate_precedence() {
        let owner = arena_owner();
        let duplicate_id = ObjectId(7);
        let asset = asset(
            Arc::clone(&owner),
            vec![
                template(Arc::clone(&owner), duplicate_id, Vec::new()),
                template(Arc::clone(&owner), ObjectId(0), Vec::new()),
                template(Arc::clone(&owner), duplicate_id, Vec::new()),
            ],
        );
        assert_eq!(
            asset
                .try_serialised_assembly_by_object_id(ObjectId((1 << 22) | 7))
                .unwrap_err(),
            DmoSerialisedAssemblySelectError::NonLocal {
                object_id: ObjectId((1 << 22) | 7),
                section: 1,
            }
        );
        assert_eq!(
            asset
                .try_serialised_assembly_by_object_id(ObjectId(9))
                .unwrap_err(),
            DmoSerialisedAssemblySelectError::Missing {
                object_id: ObjectId(9),
            }
        );
        assert_eq!(
            asset
                .try_serialised_assembly_by_object_id(duplicate_id)
                .unwrap_err(),
            DmoSerialisedAssemblySelectError::Duplicate {
                object_id: duplicate_id,
                first_ordinal: 0,
                second_ordinal: 2,
            }
        );
    }

    #[test]
    fn byte_equal_cross_arena_source_pair_is_refused_atomically() {
        let registered = arena_owner();
        let foreign = arena_owner();
        assert!(!Arc::ptr_eq(&registered, &foreign));
        let foreign_asset = asset(
            Arc::clone(&foreign),
            vec![template(
                Arc::clone(&foreign),
                ObjectId(3),
                vec![part(Arc::clone(&foreign), 0x10, 0.0)],
            )],
        );
        let backend = backend(
            1,
            vec![(Arc::clone(&registered), binding(registered, 0x2000))],
        );

        assert!(matches!(
            SimulationDmoAssemblyPartOwner::try_new(
                model_source(foreign_asset, ObjectId(3)),
                &backend,
            ),
            Err(SimulationDmoAssemblyPartOwnerError::Attachment(
                SerialisedAssemblyPartAttachmentBindError::Route {
                    ordinal: 0,
                    source: SerialisedAssemblySimulationBindError::ArenaNotRegistered,
                }
            ))
        ));
        assert_eq!(backend.last_add_failure(), None);
    }

    #[test]
    fn backend_affinity_and_repeated_calls_precede_every_effect() {
        let owner = arena_owner();
        let asset = asset(
            Arc::clone(&owner),
            vec![template(
                Arc::clone(&owner),
                ObjectId(3),
                vec![part(Arc::clone(&owner), 0x10, 0.0)],
            )],
        );
        let mut backend_a = backend(
            1,
            vec![(Arc::clone(&owner), binding(Arc::clone(&owner), 0x2000))],
        );
        let mut backend_b = backend(1, vec![(Arc::clone(&owner), binding(owner, 0x4000))]);
        let source =
            SimulationDmoAssemblyPartOwner::try_new(model_source(asset, ObjectId(3)), &backend_a)
                .unwrap();
        let mut session = source.try_begin_session(vec![IDENTITY]).unwrap();

        assert_eq!(
            session.detach(&mut backend_b),
            Err(DynamicAssemblyBodyOnlyPartDetachError::NotAttached)
        );
        assert_eq!(
            session.attach(&mut backend_b, 4),
            Err(SimulationDmoAssemblyPartAttachError::BackendAuthorityMismatch)
        );
        assert!(!session.is_attached());
        assert_eq!(backend_a.last_add_failure(), None);
        assert_eq!(backend_b.last_add_failure(), None);
        session.attach(&mut backend_a, 4).unwrap();
        assert_eq!(
            session.attach(&mut backend_b, 4),
            Err(SimulationDmoAssemblyPartAttachError::AlreadyAttached)
        );
        assert_eq!(
            session.detach(&mut backend_b),
            Err(DynamicAssemblyBodyOnlyPartDetachError::BackendAuthorityMismatch)
        );
        assert!(session.is_attached());
        session.detach(&mut backend_a).unwrap();
        assert_eq!(
            session.detach(&mut backend_a),
            Err(DynamicAssemblyBodyOnlyPartDetachError::NotAttached)
        );
    }

    #[test]
    fn later_add_failure_rolls_back_and_preserves_diagnostic() {
        let owner = arena_owner();
        let asset = asset(
            Arc::clone(&owner),
            vec![
                template(
                    Arc::clone(&owner),
                    ObjectId(3),
                    vec![
                        part(Arc::clone(&owner), 0x10, 0.0),
                        part(Arc::clone(&owner), 0x11, 0.0),
                    ],
                ),
                template(
                    Arc::clone(&owner),
                    ObjectId(4),
                    vec![
                        part(Arc::clone(&owner), 0x10, 0.0),
                        part(Arc::clone(&owner), 0x20, 0.0),
                    ],
                ),
            ],
        );
        let mut backend = backend(2, vec![(Arc::clone(&owner), binding(owner, 0x2000))]);
        let failing = SimulationDmoAssemblyPartOwner::try_new(
            model_source(Arc::clone(&asset), ObjectId(3)),
            &backend,
        )
        .unwrap();
        let mut failing_session = failing.try_begin_session(vec![IDENTITY, IDENTITY]).unwrap();
        assert_eq!(
            failing_session.attach(&mut backend, 4),
            Err(SimulationDmoAssemblyPartAttachError::PartUnavailable { ordinal: 1 })
        );
        assert!(!failing_session.is_attached());
        let Some(SimulationAssemblyBackendAddFailure::SourceRigidBody(error)) =
            backend.last_add_failure()
        else {
            panic!("expected retained source add failure");
        };
        assert_eq!(
            error.cause(),
            &SimulationAssemblySourceError::Inertia(
                crate::simulation_xenon_address_image::SimulationRigidBodyAddInertiaOwnerError::Binding(
                    crate::simulation_xenon_address_image::SimulationArenaXenonAddressBindingError::MisalignedInertiaAddress {
                        address: 0x2011,
                        required_alignment: 0x10,
                    },
                )
            )
        );

        let valid =
            SimulationDmoAssemblyPartOwner::try_new(model_source(asset, ObjectId(4)), &backend)
                .unwrap();
        let mut valid_session = valid.try_begin_session(vec![IDENTITY, IDENTITY]).unwrap();
        valid_session
            .attach(&mut backend, 4)
            .expect("prior Part and routed outcome were rolled back");
        valid_session.detach(&mut backend).unwrap();
    }

    #[test]
    fn source_session_retains_exact_asset_owner_and_supports_zero_parts() {
        let owner = arena_owner();
        let asset = asset(
            Arc::clone(&owner),
            vec![template(Arc::clone(&owner), ObjectId(3), Vec::new())],
        );
        let weak_asset = Arc::downgrade(&asset);
        let mut backend = backend(0, Vec::new());
        let source = SimulationDmoAssemblyPartOwner::try_new(
            model_source(Arc::clone(&asset), ObjectId(3)),
            &backend,
        )
        .unwrap();
        let mut session = source.try_begin_session(Vec::new()).unwrap();
        assert_eq!(session.part_transforms().len(), 0);
        session.attach(&mut backend, 4).unwrap();
        assert!(session.is_attached());
        session.detach(&mut backend).unwrap();
        assert_eq!(backend.last_add_failure(), None);

        drop(asset);
        drop(source);
        assert!(weak_asset.upgrade().is_some());
        drop(session);
        assert!(weak_asset.upgrade().is_none());
    }
}
