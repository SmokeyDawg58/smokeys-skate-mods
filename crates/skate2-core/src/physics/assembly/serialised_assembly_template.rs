//! Owner-bound source templates for `EA::Physics::SerialisedAssembly`.
//!
//! `SerialisedAssembly::Create` (`0x82e0c850..0x82e0cf2c`) receives a live
//! `rw::physics::Assembly`, follows its `AssemblyDefinition`, and visits Parts
//! in definition order. The native object has fixed storage for 26 body
//! pointers and does not guard the Part loop, so `numParts <= 26` is an input
//! prerequisite rather than a truncation rule.
//!
//! This module resolves that immutable arena-backed source chain and keeps the
//! distinct Create/Update volume-selection walks and mutable `VolumeData`
//! semantics. Live body occurrence/header publication is staged privately in
//! the adjacent Create owner; keeping both private prevents them from being
//! mistaken for a production-connected native 0x1780-byte runtime snapshot.

#[cfg(test)]
use crate::world::part_get_transform;
use crate::{
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_rigid_body_pool::SimulationInertia,
    world::{CollisionAggregateRef, CollisionArena},
};
use skate2_formats::{
    collision::{
        ASSEMBLY_DEFINITION_TYPE_ID, ASSEMBLY_TYPE_ID, Assembly, AssemblyDefinition,
        AssemblyPartInertia, AssemblyPartMaterial, BIT_TABLE_TYPE_ID, BitTable, CollisionError,
        ObjectType, Volume, VolumeType,
    },
    pegasus::ObjectId,
    rw4::DictionaryEntry,
};
use std::{error::Error, fmt, sync::Arc};

#[cfg(test)]
use skate2_formats::collision::AssemblyPartDefinition;

/// Fixed Part/body-pointer capacity embedded in `SerialisedAssembly`.
pub(crate) const SERIALISED_ASSEMBLY_PART_CAPACITY: usize = 26;
/// Fixed `VolumeData` array capacity, independently checked per emission.
pub(crate) const SERIALISED_ASSEMBLY_VOLUME_CAPACITY: usize = 26;
/// Literal stored by Create rather than its ignored padding formal.
pub(crate) const SERIALISED_ASSEMBLY_VOLUME_PADDING: f32 = 0.05;

const ASSEMBLY_PART_DEFINITION_STRIDE: u32 = 0xa0;
const ASSEMBLY_PART_INERTIA_OFFSET: u32 = 0x10;
const ASSEMBLY_PART_INERTIA_SIZE: u32 = 0x30;

/// Owner-qualified location of one immutable Definition-embedded `Inertia`.
///
/// Native `Part::Initialize` retains the address of the `Inertia` at
/// `PartDefinition +0x10`. This source preserves the equivalent arena backing
/// identity without inventing an absolute Xenon resource base.
#[derive(Clone)]
pub(crate) struct SerialisedAssemblyPartInertiaSource {
    owner: Arc<SimulationNormalArenaPlan>,
    resource_index: usize,
    resource_offset: u32,
}

impl fmt::Debug for SerialisedAssemblyPartInertiaSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SerialisedAssemblyPartInertiaSource")
            .field("resource_index", &self.resource_index)
            .field("resource_offset", &self.resource_offset)
            .finish_non_exhaustive()
    }
}

impl PartialEq for SerialisedAssemblyPartInertiaSource {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_backing_inertia(other)
    }
}

impl Eq for SerialisedAssemblyPartInertiaSource {}

impl SerialisedAssemblyPartInertiaSource {
    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        owner: Arc<SimulationNormalArenaPlan>,
        resource_index: usize,
        resource_offset: u32,
    ) -> Self {
        Self {
            owner,
            resource_index,
            resource_offset,
        }
    }

    pub(super) fn owner(&self) -> &Arc<SimulationNormalArenaPlan> {
        &self.owner
    }

    pub(super) const fn resource_index(&self) -> usize {
        self.resource_index
    }

    pub(super) const fn resource_offset(&self) -> u32 {
        self.resource_offset
    }

    pub(super) fn bytes(&self) -> &[u8] {
        let start = usize::try_from(self.resource_offset)
            .expect("validated u32 inertia offset must fit the host");
        let end = start
            + usize::try_from(ASSEMBLY_PART_INERTIA_SIZE)
                .expect("fixed inertia size must fit the host");
        &self.owner.static_plan().asset().decoded.resources.resources[self.resource_index]
            [start..end]
    }

    /// Tests native-pointer-equivalent immutable backing identity.
    #[must_use]
    pub(super) fn is_same_backing_inertia(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.resource_index == other.resource_index
            && self.resource_offset == other.resource_offset
    }
}

/// One definition-ordered Part and its resolved aggregate-root occurrence.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyTemplatePart {
    pub(crate) volume_object_id: ObjectId,
    pub(crate) root_index: usize,
    pub(crate) inertia_source: SerialisedAssemblyPartInertiaSource,
    pub(crate) inertia: AssemblyPartInertia,
    pub(crate) inverse_body_transform: [f32; 16],
    pub(crate) material: AssemblyPartMaterial,
    pub(crate) has_inverse_body_transform: u32,
    pub(crate) is_static_body: u32,
}

/// Semantic host Inertia paired with the exact Definition address source from
/// the same immutable Part ordinal.
///
/// The first three inverse-tensor lanes and following scalar fields are the
/// recovered semantic payload. The unassigned fourth serialized tensor lane
/// remains preserved only by `source`'s complete `0x30` backing span.
#[derive(Clone, Debug)]
pub(crate) struct SerialisedAssemblyRuntimeInertia {
    semantic: Arc<SimulationInertia>,
    source: SerialisedAssemblyPartInertiaSource,
}

impl SerialisedAssemblyRuntimeInertia {
    fn from_template_part(part: &SerialisedAssemblyTemplatePart) -> Self {
        let inertia = part.inertia;
        Self {
            semantic: Arc::new(SimulationInertia {
                inverse_tensor: [
                    inertia.inverse_tensor[0],
                    inertia.inverse_tensor[1],
                    inertia.inverse_tensor[2],
                ],
                inverse_mass: inertia.inverse_mass,
                spherical: inertia.spherical,
                max_velocity: inertia.max_velocity,
                max_omega: inertia.max_omega,
                linear_drag: inertia.linear_drag,
                angular_drag: inertia.angular_drag,
            }),
            source: part.inertia_source.clone(),
        }
    }

    pub(crate) fn semantic(&self) -> &Arc<SimulationInertia> {
        &self.semantic
    }

    pub(crate) const fn source(&self) -> &SerialisedAssemblyPartInertiaSource {
        &self.source
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        semantic: Arc<SimulationInertia>,
        source: SerialisedAssemblyPartInertiaSource,
    ) -> Self {
        Self { semantic, source }
    }
}

/// Immutable Create fields sourced from one selected Volume occurrence.
///
/// Live material, Part transform, group/frozen state, and the body pointer are
/// absent because Create reads those from live Parts and bodies. This staged
/// owner reads enabled from the retained inline Volume snapshot; mutation of
/// the native Volume allocation between template parsing and Create remains a
/// production-bridge boundary.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyVolumeSource {
    pub(crate) part_ordinal: usize,
    pub(crate) rigid_body_index: u16,
    pub(crate) volume_type: VolumeType,
    pub(crate) volume: Volume,
    pub(crate) padding: f32,
    pub(crate) surface_id: u32,
}

/// One Volume occurrence visited by `SerialisedAssembly::Update`.
///
/// Update reads only the Volume enabled bit after selection; the complete
/// Volume is retained here solely to make occurrence identity and corpus
/// validation mutation-resistant. Unlike Create, Update does not select a
/// top-level `ClusteredMesh` aggregate wrapper.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyVolumeUpdateSource {
    pub(crate) part_ordinal: u16,
    pub(crate) volume: Volume,
}

/// Owner-qualified immutable snapshot of an arena `rw::BitTable`.
///
/// Native Assembly/Header fields borrow the shared arena allocation through a
/// const pointer; mutation through another owner/alias would remain visible.
/// Rust instead retains the arena owner, `ObjectId`, and one shared parsed
/// snapshot and does not model that later alias visibility.
///
/// A LivingWorld model's `kLWModelCullingTable` (`LWModel::
/// CreateCollisionCullingTable` 0x82e63e40) is an Inventory resource, not an
/// arena object: its owner is the arena whose HiLOD Definition CreateFromMesh
/// 0x82d33c9c points at it, and it has no `ObjectId`.
#[derive(Clone, Debug)]
pub(crate) struct SerialisedAssemblyCullingTableSource {
    owner: Arc<SimulationNormalArenaPlan>,
    object_id: Option<ObjectId>,
    table: Arc<BitTable>,
}

impl PartialEq for SerialisedAssemblyCullingTableSource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.object_id == other.object_id
            && self.table == other.table
    }
}

impl Eq for SerialisedAssemblyCullingTableSource {}

impl SerialisedAssemblyCullingTableSource {
    pub(super) fn owner(&self) -> &Arc<SimulationNormalArenaPlan> {
        &self.owner
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for live SerialisedAssembly culling publication"
        )
    )]
    pub(super) const fn object_id(&self) -> Option<ObjectId> {
        self.object_id
    }

    pub(super) fn table(&self) -> &BitTable {
        &self.table
    }

    /// The shared LivingWorld model table stored into `owner`'s HiLOD Definition.
    pub(crate) fn living_world_model(
        owner: Arc<SimulationNormalArenaPlan>,
        table: Arc<BitTable>,
    ) -> Self {
        Self {
            owner,
            object_id: None,
            table,
        }
    }
}

/// Part-0 fields `LWDynamicObjectModel::ApplyAttribulatorOffsets` 0x82d31e28
/// rewrites in the shared bank `PartDefinition`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyLivingWorldPart {
    pub(crate) inertia: AssemblyPartInertia,
    pub(crate) inverse_body_transform: [f32; 16],
    pub(crate) has_inverse_body_transform: u32,
    pub(crate) material: AssemblyPartMaterial,
}

/// Immutable, owner-bound inputs needed by a later native-shaped Create pass.
#[derive(Debug)]
pub(crate) struct SerialisedAssemblyTemplate {
    owner: Arc<SimulationNormalArenaPlan>,
    assembly_object_id: ObjectId,
    definition_object_id: ObjectId,
    assembly: Assembly,
    definition: AssemblyDefinition,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for live SerialisedAssembly culling publication"
        )
    )]
    culling_table: Option<SerialisedAssemblyCullingTableSource>,
    parts: Vec<SerialisedAssemblyTemplatePart>,
    volume_sources: Vec<SerialisedAssemblyVolumeSource>,
    update_volume_sources: Vec<SerialisedAssemblyVolumeUpdateSource>,
}

impl SerialisedAssemblyTemplate {
    /// Resolves one local Assembly and all definition-ordered Part volumes.
    ///
    /// Errors are host-side publication refusals. The reference assumes fixed
    /// up local pointers and sufficient fixed storage rather than returning
    /// these failures from `SerialisedAssembly::Create`.
    pub(crate) fn try_new(
        owner: Arc<SimulationNormalArenaPlan>,
        assembly_object_id: ObjectId,
    ) -> Result<Self, SerialisedAssemblyTemplateError> {
        let assembly_entry = local_typed_entry(
            &owner,
            assembly_object_id,
            SerialisedAssemblyLink::Assembly,
            ASSEMBLY_TYPE_ID,
        )?;
        let assembly = Assembly::parse(object_bytes(&owner, assembly_entry)?)
            .map_err(SerialisedAssemblyTemplateError::AssemblyParse)?;
        let definition_object_id = assembly.definition;
        let definition_entry = local_typed_entry(
            &owner,
            definition_object_id,
            SerialisedAssemblyLink::Definition,
            ASSEMBLY_DEFINITION_TYPE_ID,
        )?;
        let definition = AssemblyDefinition::parse_in_resource(
            resource_bytes(&owner, definition_entry)?,
            definition_entry.object_offset,
        )
        .map_err(SerialisedAssemblyTemplateError::DefinitionParse)?;
        let culling_table = resolve_culling_table(&owner, &assembly)?;

        let parsed_count = definition.parts.len();
        let definition_count = usize::try_from(definition.part_count)
            .map_err(|_| SerialisedAssemblyTemplateError::PartCountOutsideHost)?;
        let assembly_count = usize::try_from(assembly.part_count)
            .map_err(|_| SerialisedAssemblyTemplateError::PartCountOutsideHost)?;
        if assembly_count != definition_count || definition_count != parsed_count {
            return Err(SerialisedAssemblyTemplateError::PartCountMismatch {
                assembly: assembly.part_count,
                definition: definition.part_count,
                parsed: parsed_count,
            });
        }
        if parsed_count > SERIALISED_ASSEMBLY_PART_CAPACITY {
            return Err(SerialisedAssemblyTemplateError::PartCapacityExceeded {
                count: parsed_count,
                capacity: SERIALISED_ASSEMBLY_PART_CAPACITY,
            });
        }

        let mut parts = Vec::new();
        parts.try_reserve_exact(parsed_count).map_err(|_| {
            SerialisedAssemblyTemplateError::PartStorageAllocation {
                requested: parsed_count,
            }
        })?;
        for (part_ordinal, definition_part) in definition.parts.iter().enumerate() {
            let inertia_source = definition_part_inertia_source(
                &owner,
                definition_entry,
                definition.parts_offset,
                part_ordinal,
            )?;
            let volume_entry = local_typed_entry(
                &owner,
                definition_part.volume,
                SerialisedAssemblyLink::PartVolume { part_ordinal },
                ObjectType::Volume as u32,
            )?;
            let root_index = owner
                .static_plan()
                .asset()
                .collision
                .aggregate_roots
                .iter()
                .position(|root| root.dictionary_index == definition_part.volume.entry_index())
                .ok_or(
                    SerialisedAssemblyTemplateError::MissingProjectedPartVolume {
                        part_ordinal,
                        object_id: definition_part.volume.0,
                    },
                )?;
            debug_assert_eq!(volume_entry.type_id, ObjectType::Volume as u32);
            parts.push(SerialisedAssemblyTemplatePart {
                volume_object_id: definition_part.volume,
                root_index,
                inertia_source,
                inertia: definition_part.inertia,
                inverse_body_transform: definition_part.inverse_body_transform,
                material: definition_part.material,
                has_inverse_body_transform: definition_part.has_inverse_body_transform,
                is_static_body: definition_part.is_static_body,
            });
        }
        let volume_sources = build_volume_sources(&owner.static_plan().asset().collision, &parts)?;
        let update_volume_sources =
            build_update_volume_sources(&owner.static_plan().asset().collision, &parts)?;

        Ok(Self {
            owner,
            assembly_object_id,
            definition_object_id,
            assembly,
            definition,
            culling_table,
            parts,
            volume_sources,
            update_volume_sources,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_parts_and_constraint_counts_for_test(
        owner: Arc<SimulationNormalArenaPlan>,
        assembly_object_id: ObjectId,
        assembly_joint_count: u32,
        definition_joint_count: u32,
        assembly_drive_count: u32,
        definition_drive_count: u32,
        parts: Vec<SerialisedAssemblyTemplatePart>,
    ) -> Self {
        assert!(
            parts
                .iter()
                .all(|part| Arc::ptr_eq(part.inertia_source.owner(), &owner)),
            "test Part sources share the exact template owner"
        );
        let part_count = u32::try_from(parts.len()).expect("test Part count fits u32");
        let definition_parts = parts
            .iter()
            .map(|part| AssemblyPartDefinition {
                volume: part.volume_object_id,
                inertia: part.inertia,
                inverse_body_transform: part.inverse_body_transform,
                material: part.material,
                has_inverse_body_transform: part.has_inverse_body_transform,
                is_static_body: part.is_static_body,
            })
            .collect();
        Self {
            owner,
            assembly_object_id,
            definition_object_id: ObjectId(0),
            assembly: Assembly {
                culling_table: None,
                definition: ObjectId(0),
                part_list_offset: 0,
                joint_list_offset: 0,
                drive_list_offset: 0,
                world_index: 0,
                group_id: 0,
                part_count,
                joint_count: assembly_joint_count,
                drive_count: assembly_drive_count,
                user_data: 0,
                flags: 0,
                user_flags: 0,
            },
            definition: AssemblyDefinition {
                parts_offset: 0,
                part_count,
                joint_count: definition_joint_count,
                joint_limits_offset: 0,
                joint_frames_offset: 0,
                joint_parts_offset: 0,
                drive_count: definition_drive_count,
                drive_dynamics_offset: 0,
                drive_frames_offset: 0,
                drive_parts_offset: 0,
                culling_table: None,
                hashed_part_names_offset: 0,
                parts: definition_parts,
            },
            culling_table: None,
            parts,
            volume_sources: Vec::new(),
            update_volume_sources: Vec::new(),
        }
    }

    /// Builds every serialized Assembly object in dictionary order.
    pub(crate) fn all_in_plan(
        owner: &Arc<SimulationNormalArenaPlan>,
    ) -> Result<Vec<Self>, SerialisedAssemblyTemplateError> {
        let indexes = owner
            .static_plan()
            .asset()
            .decoded
            .arena
            .dictionary
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (entry.type_id == ASSEMBLY_TYPE_ID).then_some(index))
            .collect::<Vec<_>>();
        let mut templates = Vec::new();
        templates.try_reserve_exact(indexes.len()).map_err(|_| {
            SerialisedAssemblyTemplateError::TemplateStorageAllocation {
                requested: indexes.len(),
            }
        })?;
        for index in indexes {
            let raw = u32::try_from(index).map_err(|_| {
                SerialisedAssemblyTemplateError::DictionaryIndexOutsideObjectId { index }
            })?;
            if raw > 0x003f_ffff {
                return Err(
                    SerialisedAssemblyTemplateError::DictionaryIndexOutsideObjectId { index },
                );
            }
            templates.push(Self::try_new(Arc::clone(owner), ObjectId(raw))?);
        }
        Ok(templates)
    }

    pub(crate) fn parts(&self) -> &[SerialisedAssemblyTemplatePart] {
        &self.parts
    }

    /// Mints the inseparable semantic/source Inertia pair owned by one actual
    /// Definition-order Part in this template.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for the Definition-backed DynamicAssembly runtime-template bridge"
        )
    )]
    pub(crate) fn runtime_inertia(
        &self,
        part_ordinal: usize,
    ) -> Option<SerialisedAssemblyRuntimeInertia> {
        self.parts
            .get(part_ordinal)
            .map(SerialisedAssemblyRuntimeInertia::from_template_part)
    }

    pub(crate) fn volume_sources(&self) -> &[SerialisedAssemblyVolumeSource] {
        &self.volume_sources
    }

    pub(crate) fn update_volume_sources(&self) -> &[SerialisedAssemblyVolumeUpdateSource] {
        &self.update_volume_sources
    }

    pub(crate) fn assembly(&self) -> &Assembly {
        &self.assembly
    }

    pub(crate) fn definition(&self) -> &AssemblyDefinition {
        &self.definition
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "reserved for live SerialisedAssembly culling publication"
        )
    )]
    pub(crate) fn culling_table(&self) -> Option<&SerialisedAssemblyCullingTableSource> {
        self.culling_table.as_ref()
    }

    /// Fresh Assembly initialization 0x82ae0ddc..0x82ae0de4 selects the
    /// Definition table, independently of the serialized Assembly header.
    pub(crate) fn try_definition_culling_table(
        &self,
    ) -> Result<Option<SerialisedAssemblyCullingTableSource>, SerialisedAssemblyTemplateError> {
        resolve_culling_object(&self.owner, self.definition.culling_table)
    }

    pub(crate) const fn assembly_object_id(&self) -> ObjectId {
        self.assembly_object_id
    }

    pub(crate) const fn definition_object_id(&self) -> ObjectId {
        self.definition_object_id
    }

    pub(crate) fn root_volume(&self, part_ordinal: usize) -> &Volume {
        let part = &self.parts[part_ordinal];
        &self.owner.static_plan().asset().collision.aggregate_roots[part.root_index].volume
    }

    pub(crate) fn owner(&self) -> &Arc<SimulationNormalArenaPlan> {
        &self.owner
    }

    /// The LivingWorld model's view of this pipe HiLOD Definition after its
    /// in-place Part-0 edit. Native edits the shared bank Definition; every
    /// consumer of this model reads the edited copy. Volumes, the Definition's
    /// serialized culling ID and the Inertia address source are unchanged.
    pub(crate) fn living_world_model_copy(&self, part0: SerialisedAssemblyLivingWorldPart) -> Self {
        let mut parts = self.parts.clone();
        let mut definition = self.definition.clone();
        for (part, definition_part) in parts.iter_mut().zip(definition.parts.iter_mut()).take(1) {
            part.inertia = part0.inertia;
            part.inverse_body_transform = part0.inverse_body_transform;
            part.has_inverse_body_transform = part0.has_inverse_body_transform;
            part.material = part0.material;
            definition_part.inertia = part0.inertia;
            definition_part.inverse_body_transform = part0.inverse_body_transform;
            definition_part.has_inverse_body_transform = part0.has_inverse_body_transform;
            definition_part.material = part0.material;
        }
        Self {
            owner: Arc::clone(&self.owner),
            assembly_object_id: self.assembly_object_id,
            definition_object_id: self.definition_object_id,
            assembly: self.assembly.clone(),
            definition,
            culling_table: self.culling_table.clone(),
            parts,
            volume_sources: self.volume_sources.clone(),
            update_volume_sources: self.update_volume_sources.clone(),
        }
    }
}

fn resolve_culling_table(
    owner: &Arc<SimulationNormalArenaPlan>,
    assembly: &Assembly,
) -> Result<Option<SerialisedAssemblyCullingTableSource>, SerialisedAssemblyTemplateError> {
    resolve_culling_object(owner, assembly.culling_table)
}

fn resolve_culling_object(
    owner: &Arc<SimulationNormalArenaPlan>,
    object_id: Option<ObjectId>,
) -> Result<Option<SerialisedAssemblyCullingTableSource>, SerialisedAssemblyTemplateError> {
    object_id
        .map(|object_id| {
            let entry = local_typed_entry(
                owner,
                object_id,
                SerialisedAssemblyLink::CullingTable,
                BIT_TABLE_TYPE_ID,
            )?;
            let table = BitTable::parse(object_bytes(owner, entry)?)
                .map_err(SerialisedAssemblyTemplateError::CullingTableParse)?;
            Ok(SerialisedAssemblyCullingTableSource {
                owner: Arc::clone(owner),
                object_id: Some(object_id),
                table: Arc::new(table),
            })
        })
        .transpose()
}

/// Live Part values read by `SerialisedAssembly::Create` and `Update`.
///
/// Both callers run after `Assembly::AddToSimulation`, so the body pose and
/// body-state word are mandatory here even though `Part::GetTransform` itself
/// has a detached branch.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the next DMO live-assembly publication seam"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyLivePartState {
    pub(crate) part_transform: [f32; 16],
    pub(crate) body_transform: [f32; 16],
    pub(crate) material: AssemblyPartMaterial,
    pub(crate) user_data: u32,
    pub(crate) body_state: u32,
}

/// Complete semantic fields of one native 0xe0-byte `VolumeData` record.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SerialisedAssemblyVolumeDataState {
    pub(crate) rigid_body_index: u16,
    pub(crate) volume_type: VolumeType,
    pub(crate) volume: Volume,
    pub(crate) padding: f32,
    pub(crate) material: AssemblyPartMaterial,
    pub(crate) surface_id: u32,
    pub(crate) transform: [f32; 16],
    pub(crate) enabled: bool,
    pub(crate) group: u32,
    pub(crate) frozen: bool,
}

/// Owner-qualified `VolumeData` storage across Create and later Update calls.
///
/// This deliberately excludes the `SerialisedAssembly` header, AABB, body
/// pointer array, and Params. It is therefore not a native-layout claim.
#[derive(Clone, Debug)]
pub(crate) struct SerialisedAssemblyVolumeDataSet {
    owner: Arc<SimulationNormalArenaPlan>,
    assembly_object_id: ObjectId,
    definition_object_id: ObjectId,
    records: Vec<SerialisedAssemblyVolumeDataState>,
}

impl PartialEq for SerialisedAssemblyVolumeDataSet {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.assembly_object_id == other.assembly_object_id
            && self.definition_object_id == other.definition_object_id
            && self.records == other.records
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the next DMO live-assembly publication seam"
    )
)]
impl SerialisedAssemblyVolumeDataSet {
    /// Copies immutable Create fields and the five live Part/body fields.
    ///
    /// Reference ranges: inlined CPU Create at
    /// `0x82e0c8f0..0x82e0ce28`; equivalent `VolumeData::Initialize` at
    /// `0x82e0b690..0x82e0b75c`.
    pub(super) fn try_create(
        template: &SerialisedAssemblyTemplate,
        live_parts: &[SerialisedAssemblyLivePartState],
    ) -> Result<Self, SerialisedAssemblyVolumeDataStateError> {
        validate_live_part_count(template, live_parts)?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(template.volume_sources.len())
            .map_err(
                |_| SerialisedAssemblyVolumeDataStateError::StorageAllocation {
                    requested: template.volume_sources.len(),
                },
            )?;
        for source in &template.volume_sources {
            let part = &live_parts[source.part_ordinal];
            let template_part = &template.parts[source.part_ordinal];
            records.push(SerialisedAssemblyVolumeDataState {
                rigid_body_index: source.rigid_body_index,
                volume_type: source.volume_type,
                volume: source.volume.clone(),
                padding: source.padding,
                material: part.material,
                surface_id: source.surface_id,
                transform: live_part_transform(template_part, part),
                enabled: source.volume.flags & 1 != 0,
                group: part.user_data,
                frozen: part.body_state & 2 != 0,
            });
        }
        Ok(Self {
            owner: Arc::clone(template.owner()),
            assembly_object_id: template.assembly_object_id,
            definition_object_id: template.definition_object_id,
            records,
        })
    }

    /// Rewrites exactly enabled, material, transform, group, and frozen.
    ///
    /// `live_volume_flags` is parallel to the already-proven Update selection
    /// sequence. Topology and pointer stability across Create/Update are native
    /// external preconditions. All safe-host validation happens before writes;
    /// intermediate concurrent visibility of the native scalar/vector stores
    /// is not claimed.
    pub(super) fn try_update(
        &mut self,
        template: &SerialisedAssemblyTemplate,
        live_parts: &[SerialisedAssemblyLivePartState],
        live_volume_flags: &[u32],
    ) -> Result<(), SerialisedAssemblyVolumeDataStateError> {
        self.validate_update_shape(template, live_volume_flags)?;
        validate_live_part_count(template, live_parts)?;

        for ((record, source), flags) in self
            .records
            .iter_mut()
            .zip(&template.update_volume_sources)
            .zip(live_volume_flags)
        {
            let part_ordinal = usize::from(source.part_ordinal);
            let part = &live_parts[part_ordinal];
            let template_part = &template.parts[part_ordinal];
            record.enabled = flags & 1 != 0;
            record.material = part.material;
            record.transform = live_part_transform(template_part, part);
            record.group = part.user_data;
            record.frozen = part.body_state & 2 != 0;
        }
        Ok(())
    }

    /// Pure preflight used before any fallible live-body/address-domain read.
    pub(super) fn validate_update_shape(
        &self,
        template: &SerialisedAssemblyTemplate,
        live_volume_flags: &[u32],
    ) -> Result<(), SerialisedAssemblyVolumeDataStateError> {
        if !Arc::ptr_eq(&self.owner, template.owner())
            || self.assembly_object_id != template.assembly_object_id
            || self.definition_object_id != template.definition_object_id
        {
            return Err(SerialisedAssemblyVolumeDataStateError::TemplateMismatch);
        }
        if live_volume_flags.len() != template.update_volume_sources.len() {
            return Err(
                SerialisedAssemblyVolumeDataStateError::UpdateVolumeFlagCount {
                    selected: template.update_volume_sources.len(),
                    flags: live_volume_flags.len(),
                },
            );
        }
        if template.update_volume_sources.len() > self.records.len() {
            return Err(
                SerialisedAssemblyVolumeDataStateError::UpdateOutsideCreateStorage {
                    created: self.records.len(),
                    updates: template.update_volume_sources.len(),
                },
            );
        }

        Ok(())
    }

    pub(super) fn records(&self) -> &[SerialisedAssemblyVolumeDataState] {
        &self.records
    }

    #[cfg(test)]
    pub(crate) fn records_mut_for_test(&mut self) -> &mut Vec<SerialisedAssemblyVolumeDataState> {
        &mut self.records
    }
}

fn validate_live_part_count(
    template: &SerialisedAssemblyTemplate,
    live_parts: &[SerialisedAssemblyLivePartState],
) -> Result<(), SerialisedAssemblyVolumeDataStateError> {
    if live_parts.len() != template.parts.len() {
        return Err(SerialisedAssemblyVolumeDataStateError::LivePartCount {
            template: template.parts.len(),
            live: live_parts.len(),
        });
    }
    Ok(())
}

fn live_part_transform(
    template: &SerialisedAssemblyTemplatePart,
    live: &SerialisedAssemblyLivePartState,
) -> [f32; 16] {
    let inverse_body_ltm =
        (template.has_inverse_body_transform != 0).then_some(&template.inverse_body_transform);
    let Some(inverse_body_ltm) = inverse_body_ltm else {
        return live.body_transform;
    };
    compose_serialised_part_transform(live.body_transform, *inverse_body_ltm)
}

/// Four-lane operation graph used by the attached/inverse-body branch of
/// `Part::GetTransform`. Unlike the public geometric affine helper, native
/// stores every computed W lane, including packed pointer/scalar words from
/// the body columns. This transcription retains ordinary-finite association;
/// exceptional Xenon VMX/FPSCR behavior remains an explicit hardware boundary.
fn compose_serialised_part_transform(outer: [f32; 16], inner: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for column in 0..3 {
        for row in 0..4 {
            let x = outer[row] * inner[column * 4];
            let xy = outer[4 + row].mul_add(inner[column * 4 + 1], x);
            result[column * 4 + row] = outer[8 + row].mul_add(inner[column * 4 + 2], xy);
        }
    }
    for row in 0..4 {
        let x = outer[row].mul_add(inner[12], outer[12 + row]);
        let xy = outer[4 + row].mul_add(inner[13], x);
        result[12 + row] = outer[8 + row].mul_add(inner[14], xy);
    }
    result
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "reserved for the next DMO live-assembly publication seam"
    )
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SerialisedAssemblyVolumeDataStateError {
    LivePartCount { template: usize, live: usize },
    UpdateVolumeFlagCount { selected: usize, flags: usize },
    UpdateOutsideCreateStorage { created: usize, updates: usize },
    TemplateMismatch,
    StorageAllocation { requested: usize },
}

impl fmt::Display for SerialisedAssemblyVolumeDataStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LivePartCount { template, live } => write!(
                formatter,
                "SerialisedAssembly has {template} template Parts but {live} live Parts"
            ),
            Self::UpdateVolumeFlagCount { selected, flags } => write!(
                formatter,
                "SerialisedAssembly Update selected {selected} Volumes but received {flags} live flag words"
            ),
            Self::UpdateOutsideCreateStorage { created, updates } => write!(
                formatter,
                "SerialisedAssembly created {created} VolumeData records but Update selected {updates}"
            ),
            Self::TemplateMismatch => {
                formatter.write_str("SerialisedAssembly Update template differs from Create")
            }
            Self::StorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not reserve {requested} VolumeData records"
                )
            }
        }
    }
}

impl Error for SerialisedAssemblyVolumeDataStateError {}

fn build_volume_sources(
    arena: &CollisionArena,
    parts: &[SerialisedAssemblyTemplatePart],
) -> Result<Vec<SerialisedAssemblyVolumeSource>, SerialisedAssemblyTemplateError> {
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(SERIALISED_ASSEMBLY_VOLUME_CAPACITY)
        .map_err(
            |_| SerialisedAssemblyTemplateError::VolumeStorageAllocation {
                requested: SERIALISED_ASSEMBLY_VOLUME_CAPACITY,
            },
        )?;
    for (part_ordinal, part) in parts.iter().enumerate() {
        let root = arena.aggregate_roots.get(part.root_index).ok_or(
            SerialisedAssemblyTemplateError::MissingProjectedPartVolume {
                part_ordinal,
                object_id: part.volume_object_id.0,
            },
        )?;
        match require_volume_type(part_ordinal, &root.volume)? {
            VolumeType::Aggregate => match root
                .aggregate
                .ok_or(SerialisedAssemblyTemplateError::MissingRootAggregate { part_ordinal })?
            {
                CollisionAggregateRef::SimplePrimitiveSet(set_index) => {
                    append_simple_mapped_array_children(
                        arena,
                        part_ordinal,
                        set_index,
                        &mut sources,
                    )?;
                }
                CollisionAggregateRef::Mesh(_) => {
                    append_volume_source(part_ordinal, &root.volume, &mut sources)?;
                }
                CollisionAggregateRef::PrimitiveSet(_) => {}
            },
            _ => append_volume_source(part_ordinal, &root.volume, &mut sources)?,
        }
    }
    Ok(sources)
}

fn append_simple_mapped_array_children(
    arena: &CollisionArena,
    part_ordinal: usize,
    set_index: usize,
    sources: &mut Vec<SerialisedAssemblyVolumeSource>,
) -> Result<(), SerialisedAssemblyTemplateError> {
    let set = arena.simple_primitive_sets.get(set_index).ok_or(
        SerialisedAssemblyTemplateError::InvalidAggregateIndex {
            aggregate: CollisionAggregateRef::SimplePrimitiveSet(set_index),
        },
    )?;
    append_constructed_mapped_volume_sources(arena, part_ordinal, &set.volumes, sources)
}

/// Source Create/Update child selection also applies to programmatically built
/// LivingWorld SimpleMappedArrays. Share this walk instead of synthesizing an
/// arena object for the outer constructed allocation.
pub(crate) fn append_constructed_mapped_volume_sources<'a>(
    arena: &CollisionArena,
    part_ordinal: usize,
    children: impl IntoIterator<Item = &'a Volume>,
    sources: &mut Vec<SerialisedAssemblyVolumeSource>,
) -> Result<(), SerialisedAssemblyTemplateError> {
    visit_mapped_children(arena, part_ordinal, children, |volume| {
        append_volume_source(part_ordinal, volume, sources)
    })
}

fn visit_mapped_children<'a>(
    arena: &CollisionArena,
    part_ordinal: usize,
    children: impl IntoIterator<Item = &'a Volume>,
    mut emit: impl FnMut(&Volume) -> Result<(), SerialisedAssemblyTemplateError>,
) -> Result<(), SerialisedAssemblyTemplateError> {
    for child in children {
        match require_volume_type(part_ordinal, child)? {
            VolumeType::Aggregate => match resolve_aggregate_target(arena, child.shape_words[0])? {
                CollisionAggregateRef::Mesh(_) => {
                    emit(child)?;
                }
                CollisionAggregateRef::SimplePrimitiveSet(grandchild_set_index) => {
                    let grandchildren = &arena
                        .simple_primitive_sets
                        .get(grandchild_set_index)
                        .ok_or(SerialisedAssemblyTemplateError::InvalidAggregateIndex {
                            aggregate: CollisionAggregateRef::SimplePrimitiveSet(
                                grandchild_set_index,
                            ),
                        })?
                        .volumes;
                    for grandchild in grandchildren {
                        if require_volume_type(part_ordinal, grandchild)? != VolumeType::Aggregate {
                            emit(grandchild)?;
                        }
                    }
                }
                CollisionAggregateRef::PrimitiveSet(_) => {}
            },
            _ => emit(child)?,
        }
    }
    Ok(())
}

pub(crate) fn append_volume_source(
    part_ordinal: usize,
    volume: &Volume,
    sources: &mut Vec<SerialisedAssemblyVolumeSource>,
) -> Result<(), SerialisedAssemblyTemplateError> {
    let volume_type = require_volume_type(part_ordinal, volume)?;
    if sources.len() < SERIALISED_ASSEMBLY_VOLUME_CAPACITY {
        sources.push(SerialisedAssemblyVolumeSource {
            part_ordinal,
            rigid_body_index: u16::try_from(part_ordinal)
                .expect("validated SerialisedAssembly Part ordinal fits u16"),
            volume_type,
            volume: volume.clone(),
            padding: SERIALISED_ASSEMBLY_VOLUME_PADDING,
            surface_id: volume.surface_id,
        });
    }
    Ok(())
}

/// Replays the separate Update walk at `0x82e0cf30..0x82e0d2c8`.
///
/// The child and grandchild paths match Create, but the aggregate-root branch
/// admits only `SimpleMappedArray`. In particular, a root `ClusteredMesh` wrapper
/// is not selected even though Create emitted it.
fn build_update_volume_sources(
    arena: &CollisionArena,
    parts: &[SerialisedAssemblyTemplatePart],
) -> Result<Vec<SerialisedAssemblyVolumeUpdateSource>, SerialisedAssemblyTemplateError> {
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(SERIALISED_ASSEMBLY_VOLUME_CAPACITY)
        .map_err(
            |_| SerialisedAssemblyTemplateError::VolumeStorageAllocation {
                requested: SERIALISED_ASSEMBLY_VOLUME_CAPACITY,
            },
        )?;
    for (part_ordinal, part) in parts.iter().enumerate() {
        let root = arena.aggregate_roots.get(part.root_index).ok_or(
            SerialisedAssemblyTemplateError::MissingProjectedPartVolume {
                part_ordinal,
                object_id: part.volume_object_id.0,
            },
        )?;
        match require_volume_type(part_ordinal, &root.volume)? {
            VolumeType::Aggregate => {
                if let CollisionAggregateRef::SimplePrimitiveSet(set_index) = root
                    .aggregate
                    .ok_or(SerialisedAssemblyTemplateError::MissingRootAggregate { part_ordinal })?
                {
                    append_update_simple_mapped_array_children(
                        arena,
                        part_ordinal,
                        set_index,
                        &mut sources,
                    )?;
                }
            }
            _ => append_update_volume_source(part_ordinal, &root.volume, &mut sources),
        }
    }
    Ok(sources)
}

fn append_update_simple_mapped_array_children(
    arena: &CollisionArena,
    part_ordinal: usize,
    set_index: usize,
    sources: &mut Vec<SerialisedAssemblyVolumeUpdateSource>,
) -> Result<(), SerialisedAssemblyTemplateError> {
    let set = arena.simple_primitive_sets.get(set_index).ok_or(
        SerialisedAssemblyTemplateError::InvalidAggregateIndex {
            aggregate: CollisionAggregateRef::SimplePrimitiveSet(set_index),
        },
    )?;
    visit_mapped_children(arena, part_ordinal, &set.volumes, |volume| {
        append_update_volume_source(part_ordinal, volume, sources);
        Ok(())
    })
}

fn append_update_volume_source(
    part_ordinal: usize,
    volume: &Volume,
    sources: &mut Vec<SerialisedAssemblyVolumeUpdateSource>,
) {
    if sources.len() < SERIALISED_ASSEMBLY_VOLUME_CAPACITY {
        sources.push(SerialisedAssemblyVolumeUpdateSource {
            part_ordinal: u16::try_from(part_ordinal)
                .expect("validated SerialisedAssembly Part ordinal fits u16"),
            volume: volume.clone(),
        });
    }
}

fn require_volume_type(
    part_ordinal: usize,
    volume: &Volume,
) -> Result<VolumeType, SerialisedAssemblyTemplateError> {
    volume
        .volume_type()
        .ok_or(SerialisedAssemblyTemplateError::UnknownVolumeType {
            part_ordinal,
            raw: volume.vtable_offset,
        })
}

fn resolve_aggregate_target(
    arena: &CollisionArena,
    raw_target: u32,
) -> Result<CollisionAggregateRef, SerialisedAssemblyTemplateError> {
    if raw_target == 0 {
        return Err(SerialisedAssemblyTemplateError::NullAggregateTarget);
    }
    let target = ObjectId(raw_target);
    if target.section_index() != 0 {
        return Err(SerialisedAssemblyTemplateError::NonLocalAggregateTarget {
            object_id: raw_target,
            section: target.section_index(),
        });
    }
    let dictionary_index = target.entry_index();
    let mut found = None;
    for (index, set) in arena.primitive_sets.iter().enumerate() {
        if set.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::PrimitiveSet(index),
                dictionary_index,
            )?;
        }
    }
    for (index, set) in arena.simple_primitive_sets.iter().enumerate() {
        if set.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::SimplePrimitiveSet(index),
                dictionary_index,
            )?;
        }
    }
    for (index, mesh) in arena.meshes.iter().enumerate() {
        if mesh.dictionary_index == dictionary_index {
            record_aggregate_match(
                &mut found,
                CollisionAggregateRef::Mesh(index),
                dictionary_index,
            )?;
        }
    }
    found.ok_or(SerialisedAssemblyTemplateError::MissingAggregateTarget { dictionary_index })
}

fn record_aggregate_match(
    found: &mut Option<CollisionAggregateRef>,
    candidate: CollisionAggregateRef,
    dictionary_index: u32,
) -> Result<(), SerialisedAssemblyTemplateError> {
    if found.replace(candidate).is_some() {
        return Err(SerialisedAssemblyTemplateError::AmbiguousAggregateTarget { dictionary_index });
    }
    Ok(())
}

fn local_typed_entry(
    owner: &SimulationNormalArenaPlan,
    object_id: ObjectId,
    link: SerialisedAssemblyLink,
    expected_type: u32,
) -> Result<&DictionaryEntry, SerialisedAssemblyTemplateError> {
    if object_id.0 == 0 {
        return Err(SerialisedAssemblyTemplateError::NullObjectReference { link });
    }
    if object_id.section_index() != 0 {
        return Err(SerialisedAssemblyTemplateError::NonLocalObjectReference {
            link,
            object_id: object_id.0,
            section: object_id.section_index(),
        });
    }
    let entry = owner
        .static_plan()
        .asset()
        .decoded
        .arena
        .dictionary
        .get(
            usize::try_from(object_id.entry_index())
                .map_err(|_| SerialisedAssemblyTemplateError::DictionaryIndexOutsideHost)?,
        )
        .ok_or(SerialisedAssemblyTemplateError::MissingObjectReference {
            link,
            object_id: object_id.0,
        })?;
    if entry.type_id != expected_type {
        return Err(SerialisedAssemblyTemplateError::WrongObjectType {
            link,
            object_id: object_id.0,
            expected: expected_type,
            actual: entry.type_id,
        });
    }
    Ok(entry)
}

fn resource_bytes<'a>(
    owner: &'a SimulationNormalArenaPlan,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], SerialisedAssemblyTemplateError> {
    owner
        .static_plan()
        .asset()
        .decoded
        .resources
        .resources
        .get(entry.resource_index)
        .map(Vec::as_slice)
        .ok_or(SerialisedAssemblyTemplateError::MissingResource {
            resource_index: entry.resource_index,
        })
}

fn definition_part_inertia_source(
    owner: &Arc<SimulationNormalArenaPlan>,
    definition_entry: &DictionaryEntry,
    parts_offset: u32,
    part_ordinal: usize,
) -> Result<SerialisedAssemblyPartInertiaSource, SerialisedAssemblyTemplateError> {
    let ordinal = u32::try_from(part_ordinal).map_err(|_| {
        SerialisedAssemblyTemplateError::InertiaSourceOffsetOverflow { part_ordinal }
    })?;
    let part_offset = ordinal
        .checked_mul(ASSEMBLY_PART_DEFINITION_STRIDE)
        .ok_or(SerialisedAssemblyTemplateError::InertiaSourceOffsetOverflow { part_ordinal })?;
    // The serialized pointer is a 32-bit displacement from the Definition
    // object. Native fixup adds it in the 32-bit address domain, so this one
    // addition deliberately wraps and permits arrays placed before the header.
    let parts_start = definition_entry.object_offset.wrapping_add(parts_offset);
    let resource_offset = parts_start
        .checked_add(part_offset)
        .and_then(|offset| offset.checked_add(ASSEMBLY_PART_INERTIA_OFFSET))
        .ok_or(SerialisedAssemblyTemplateError::InertiaSourceOffsetOverflow { part_ordinal })?;
    let resource_end = resource_offset
        .checked_add(ASSEMBLY_PART_INERTIA_SIZE)
        .ok_or(SerialisedAssemblyTemplateError::InertiaSourceOffsetOverflow { part_ordinal })?;
    let resource = owner
        .static_plan()
        .asset()
        .decoded
        .resources
        .resources
        .get(definition_entry.resource_index)
        .ok_or(SerialisedAssemblyTemplateError::MissingResource {
            resource_index: definition_entry.resource_index,
        })?;
    let start = usize::try_from(resource_offset).map_err(|_| {
        SerialisedAssemblyTemplateError::InertiaSourceRangeOutsideResource {
            part_ordinal,
            resource_index: definition_entry.resource_index,
            resource_offset,
            resource_size: resource.len(),
        }
    })?;
    let end = usize::try_from(resource_end).map_err(|_| {
        SerialisedAssemblyTemplateError::InertiaSourceRangeOutsideResource {
            part_ordinal,
            resource_index: definition_entry.resource_index,
            resource_offset,
            resource_size: resource.len(),
        }
    })?;
    if resource.get(start..end).is_none() {
        return Err(
            SerialisedAssemblyTemplateError::InertiaSourceRangeOutsideResource {
                part_ordinal,
                resource_index: definition_entry.resource_index,
                resource_offset,
                resource_size: resource.len(),
            },
        );
    }

    Ok(SerialisedAssemblyPartInertiaSource {
        owner: Arc::clone(owner),
        resource_index: definition_entry.resource_index,
        resource_offset,
    })
}

fn object_bytes<'a>(
    owner: &'a SimulationNormalArenaPlan,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], SerialisedAssemblyTemplateError> {
    owner
        .static_plan()
        .asset()
        .decoded
        .arena
        .object_bytes(
            &owner.static_plan().asset().decoded.resources.resources,
            entry,
        )
        .ok_or(
            SerialisedAssemblyTemplateError::ObjectRangeOutsideResource {
                resource_index: entry.resource_index,
                object_offset: entry.object_offset,
                size: entry.size,
                type_id: entry.type_id,
            },
        )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SerialisedAssemblyLink {
    Assembly,
    Definition,
    CullingTable,
    PartVolume { part_ordinal: usize },
}

impl fmt::Display for SerialisedAssemblyLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assembly => formatter.write_str("Assembly"),
            Self::Definition => formatter.write_str("AssemblyDefinition"),
            Self::CullingTable => formatter.write_str("Assembly culling table"),
            Self::PartVolume { part_ordinal } => write!(formatter, "Part {part_ordinal} volume"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SerialisedAssemblyTemplateError {
    DictionaryIndexOutsideObjectId {
        index: usize,
    },
    DictionaryIndexOutsideHost,
    NullObjectReference {
        link: SerialisedAssemblyLink,
    },
    NonLocalObjectReference {
        link: SerialisedAssemblyLink,
        object_id: u32,
        section: u16,
    },
    MissingObjectReference {
        link: SerialisedAssemblyLink,
        object_id: u32,
    },
    WrongObjectType {
        link: SerialisedAssemblyLink,
        object_id: u32,
        expected: u32,
        actual: u32,
    },
    MissingResource {
        resource_index: usize,
    },
    ObjectRangeOutsideResource {
        resource_index: usize,
        object_offset: u32,
        size: u32,
        type_id: u32,
    },
    AssemblyParse(CollisionError),
    DefinitionParse(CollisionError),
    CullingTableParse(CollisionError),
    PartCountOutsideHost,
    PartCountMismatch {
        assembly: u32,
        definition: u32,
        parsed: usize,
    },
    PartCapacityExceeded {
        count: usize,
        capacity: usize,
    },
    InertiaSourceOffsetOverflow {
        part_ordinal: usize,
    },
    InertiaSourceRangeOutsideResource {
        part_ordinal: usize,
        resource_index: usize,
        resource_offset: u32,
        resource_size: usize,
    },
    MissingProjectedPartVolume {
        part_ordinal: usize,
        object_id: u32,
    },
    UnknownVolumeType {
        part_ordinal: usize,
        raw: u32,
    },
    MissingRootAggregate {
        part_ordinal: usize,
    },
    InvalidAggregateIndex {
        aggregate: CollisionAggregateRef,
    },
    NullAggregateTarget,
    NonLocalAggregateTarget {
        object_id: u32,
        section: u16,
    },
    MissingAggregateTarget {
        dictionary_index: u32,
    },
    AmbiguousAggregateTarget {
        dictionary_index: u32,
    },
    PartStorageAllocation {
        requested: usize,
    },
    TemplateStorageAllocation {
        requested: usize,
    },
    VolumeStorageAllocation {
        requested: usize,
    },
}

impl fmt::Display for SerialisedAssemblyTemplateError {
    #[allow(clippy::too_many_lines)] // Keep every typed publication refusal explicit.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DictionaryIndexOutsideObjectId { index } => write!(
                formatter,
                "Assembly dictionary index {index} does not fit a local arena object ID"
            ),
            Self::DictionaryIndexOutsideHost => {
                formatter.write_str("arena object index does not fit host address space")
            }
            Self::NullObjectReference { link } => write!(formatter, "{link} reference is null"),
            Self::NonLocalObjectReference {
                link,
                object_id,
                section,
            } => write!(
                formatter,
                "{link} reference {object_id:#010x} uses nonlocal section {section}"
            ),
            Self::MissingObjectReference { link, object_id } => write!(
                formatter,
                "{link} reference {object_id:#010x} has no dictionary entry"
            ),
            Self::WrongObjectType {
                link,
                object_id,
                expected,
                actual,
            } => write!(
                formatter,
                "{link} reference {object_id:#010x} has type {actual:#010x}, expected {expected:#010x}"
            ),
            Self::MissingResource { resource_index } => {
                write!(
                    formatter,
                    "Assembly source resource {resource_index} is missing"
                )
            }
            Self::ObjectRangeOutsideResource {
                resource_index,
                object_offset,
                size,
                type_id,
            } => write!(
                formatter,
                "Assembly source type {type_id:#010x} at {object_offset:#x}+{size:#x} is outside resource {resource_index}"
            ),
            Self::AssemblyParse(source) => {
                write!(formatter, "Assembly header is invalid: {source}")
            }
            Self::DefinitionParse(source) => {
                write!(formatter, "AssemblyDefinition is invalid: {source}")
            }
            Self::CullingTableParse(source) => {
                write!(formatter, "Assembly culling table is invalid: {source}")
            }
            Self::PartCountOutsideHost => {
                formatter.write_str("Assembly Part count does not fit host address space")
            }
            Self::PartCountMismatch {
                assembly,
                definition,
                parsed,
            } => write!(
                formatter,
                "Assembly Part counts differ: runtime {assembly}, definition {definition}, parsed {parsed}"
            ),
            Self::PartCapacityExceeded { count, capacity } => write!(
                formatter,
                "Assembly has {count} Parts but SerialisedAssembly has fixed capacity {capacity}"
            ),
            Self::InertiaSourceOffsetOverflow { part_ordinal } => write!(
                formatter,
                "Part {part_ordinal} Definition inertia offset exceeds the 32-bit arena resource domain"
            ),
            Self::InertiaSourceRangeOutsideResource {
                part_ordinal,
                resource_index,
                resource_offset,
                resource_size,
            } => write!(
                formatter,
                "Part {part_ordinal} Definition inertia at {resource_offset:#x}+{ASSEMBLY_PART_INERTIA_SIZE:#x} is outside {resource_size:#x}-byte resource {resource_index}"
            ),
            Self::MissingProjectedPartVolume {
                part_ordinal,
                object_id,
            } => write!(
                formatter,
                "Part {part_ordinal} volume {object_id:#010x} is absent from the retained collision projection"
            ),
            Self::UnknownVolumeType { part_ordinal, raw } => write!(
                formatter,
                "Part {part_ordinal} selected unknown Volume type {raw}"
            ),
            Self::MissingRootAggregate { part_ordinal } => write!(
                formatter,
                "Part {part_ordinal} aggregate root has no resolved aggregate"
            ),
            Self::InvalidAggregateIndex { aggregate } => {
                write!(
                    formatter,
                    "Assembly aggregate index is invalid: {aggregate:?}"
                )
            }
            Self::NullAggregateTarget => {
                formatter.write_str("Assembly child aggregate target is null")
            }
            Self::NonLocalAggregateTarget { object_id, section } => write!(
                formatter,
                "Assembly child aggregate target {object_id:#010x} uses nonlocal section {section}"
            ),
            Self::MissingAggregateTarget { dictionary_index } => write!(
                formatter,
                "Assembly child aggregate target dictionary entry {dictionary_index} is missing"
            ),
            Self::AmbiguousAggregateTarget { dictionary_index } => write!(
                formatter,
                "Assembly child aggregate target dictionary entry {dictionary_index} is ambiguous"
            ),
            Self::PartStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} Assembly Parts")
            }
            Self::TemplateStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not reserve {requested} Assembly templates"
                )
            }
            Self::VolumeStorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} Volume sources")
            }
        }
    }
}

impl Error for SerialisedAssemblyTemplateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::AssemblyParse(source)
            | Self::DefinitionParse(source)
            | Self::CullingTableParse(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        serialised_assembly_bbox::{
            SerialisedAssemblyBBoxMinMaxNumeric, SerialisedAssemblyPartBBoxNumeric,
        },
        serialised_assembly_create::{
            SerialisedAssemblyCreatePartInput, SerialisedAssemblyLiveAssemblyInput,
            SerialisedAssemblyPackedBodyMotionWords, SerialisedAssemblyPackedBodyTransformWords,
            SerialisedAssemblyPackedBodyWordAuthority, SerialisedAssemblyPackedBodyWordProvider,
            SerialisedAssemblyParamsState, SimulationPackedBodyWordProviderError,
            try_prepare_serialised_assembly_create,
            try_prepare_serialised_assembly_create_from_xenon,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_rigid_body_pool::{SimulationInertia, SimulationRigidBodyPool},
        simulation_xenon_address_image::{
            SimulationArenaXenonAddressBinding, SimulationPackedBodyWordImageDomain,
            SimulationRigidBodyAddInertiaInput, SimulationRigidBodyAddInertiaOwner,
            SimulationXenonAddressImage,
        },
        world::{CollisionMesh, CollisionRoot, CollisionSimplePrimitiveSet},
    };
    use skate2_formats::{
        collision::ClusteredMesh,
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{convert::Infallible, ops::Range};

    const VOLUME_OFFSET: usize = 0x40;
    const BIT_TABLE_OFFSET: usize = 0x20;
    const DEFINITION_OFFSET: usize = 0xc0;
    const PARTS_OFFSET: usize = 0x100;
    const ASSEMBLY_OFFSET: usize = 0x1200;

    #[derive(Default)]
    struct OrdinaryFiniteNumeric;

    impl SerialisedAssemblyBBoxMinMaxNumeric for OrdinaryFiniteNumeric {
        type Error = Infallible;

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

    impl SerialisedAssemblyPartBBoxNumeric for OrdinaryFiniteNumeric {
        fn sum_three_products(
            &mut self,
            left: [f32; 4],
            right: [f32; 4],
        ) -> Result<f32, Self::Error> {
            Ok((left[0] * right[0] + left[1] * right[1]) + left[2] * right[2])
        }

        fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
            Ok(value.sqrt().recip())
        }
    }

    struct NeverNumeric;

    impl SerialisedAssemblyBBoxMinMaxNumeric for NeverNumeric {
        type Error = Infallible;

        fn minimum(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<[f32; 4], Self::Error> {
            panic!("numeric callback must not run before culling preflight")
        }

        fn maximum(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<[f32; 4], Self::Error> {
            panic!("numeric callback must not run before culling preflight")
        }
    }

    impl SerialisedAssemblyPartBBoxNumeric for NeverNumeric {
        fn sum_three_products(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            panic!("numeric callback must not run before culling preflight")
        }

        fn reciprocal_square_root_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("numeric callback must not run before culling preflight")
        }
    }

    struct FixedPackedWords {
        authority: SerialisedAssemblyPackedBodyWordAuthority,
        transform_calls: Vec<crate::simulation_rigid_body_pool::SimulationRigidBodyObservation>,
        motion_calls: Vec<crate::simulation_rigid_body_pool::SimulationRigidBodyObservation>,
    }

    impl Default for FixedPackedWords {
        fn default() -> Self {
            Self {
                authority: SerialisedAssemblyPackedBodyWordAuthority::for_test(1),
                transform_calls: Vec::new(),
                motion_calls: Vec::new(),
            }
        }
    }

    impl SerialisedAssemblyPackedBodyWordProvider for FixedPackedWords {
        type Error = Infallible;

        fn authority(&self) -> SerialisedAssemblyPackedBodyWordAuthority {
            self.authority
        }

        fn validate_context(
            &self,
            _: &Arc<SimulationNormalArenaPlan>,
            _: &SimulationRigidBodyPool,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn packed_transform_words(
            &mut self,
            body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        ) -> Result<SerialisedAssemblyPackedBodyTransformWords, Self::Error> {
            let ordinal = u16::try_from(self.transform_calls.len()).unwrap();
            self.transform_calls.push(body);
            Ok(SerialisedAssemblyPackedBodyTransformWords {
                stasis: (2.0_f32 + f32::from(ordinal)).to_bits(),
                inertia: (3.0_f32 + f32::from(ordinal)).to_bits(),
                solver_delta_row: (7.0_f32 + f32::from(ordinal)).to_bits(),
            })
        }

        fn packed_motion_words(
            &mut self,
            body: crate::simulation_rigid_body_pool::SimulationRigidBodyObservation,
        ) -> Result<SerialisedAssemblyPackedBodyMotionWords, Self::Error> {
            let ordinal = u16::try_from(self.motion_calls.len()).unwrap();
            self.motion_calls.push(body);
            Ok(SerialisedAssemblyPackedBodyMotionWords {
                right_link: (11.0_f32 + f32::from(ordinal)).to_bits(),
                left_link: (13.0_f32 + f32::from(ordinal)).to_bits(),
            })
        }
    }

    fn single_body_fixture(
        state: u32,
    ) -> (
        SimulationRigidBodyPool,
        [SerialisedAssemblyCreatePartInput; 1],
    ) {
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let transform = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let handle = pool
            .try_add_rigid_body(transform, Some(&inertia), state)
            .unwrap()
            .unwrap();
        let body = pool.try_observe(handle).unwrap();
        (
            pool,
            [SerialisedAssemblyCreatePartInput {
                body,
                part_transform: transform,
                material: AssemblyPartMaterial {
                    static_friction: 1.0,
                    dynamic_friction: 1.0,
                    restitution: 0.0,
                },
                user_data: 0,
            }],
        )
    }

    fn dictionary_entry(object_offset: usize, size: usize, type_id: u32) -> DictionaryEntry {
        DictionaryEntry {
            resource_index: 0,
            object_offset: u32::try_from(object_offset).unwrap(),
            relocation_offset: 0,
            size: u32::try_from(size).unwrap(),
            alignment: 16,
            type_index: 0,
            type_id,
            object_range: Range {
                start: object_offset,
                end: object_offset + size,
            },
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn source_plan(
        part_count: u32,
        assembly_part_count: u32,
        definition_id: ObjectId,
        part_volume_id: ObjectId,
    ) -> Arc<SimulationNormalArenaPlan> {
        source_plan_with_bit_table(
            part_count,
            assembly_part_count,
            definition_id,
            part_volume_id,
            &BitTable {
                rows: 1,
                columns: 1,
                array_size: 1,
                words: vec![0x8000_0000],
            },
        )
    }

    /// Synthetic complete-volume fixture for generic body-only admission.
    /// Intentionally distinguishes fresh Definition culling from stored Assembly culling.
    pub(crate) fn body_only_template_for_test(part_count: u32) -> SerialisedAssemblyTemplate {
        let owner = source_plan(part_count, part_count, ObjectId(2), ObjectId(1));
        let mut template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        template.assembly.joint_count = 0;
        template.assembly.drive_count = 0;
        template.definition.joint_count = 0;
        template.definition.drive_count = 0;
        template.definition.culling_table = None;
        template
    }

    #[allow(clippy::too_many_lines)]
    fn source_plan_with_bit_table(
        part_count: u32,
        assembly_part_count: u32,
        definition_id: ObjectId,
        part_volume_id: ObjectId,
        bit_table: &BitTable,
    ) -> Arc<SimulationNormalArenaPlan> {
        source_plan_with_layout(
            part_count,
            assembly_part_count,
            definition_id,
            part_volume_id,
            bit_table,
            DEFINITION_OFFSET,
            PARTS_OFFSET,
        )
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn source_plan_with_layout(
        part_count: u32,
        assembly_part_count: u32,
        definition_id: ObjectId,
        part_volume_id: ObjectId,
        bit_table: &BitTable,
        definition_offset: usize,
        parts_offset: usize,
    ) -> Arc<SimulationNormalArenaPlan> {
        let mut resource = vec![0_u8; 0x1300];

        for (index, value) in [bit_table.rows, bit_table.columns, bit_table.array_size]
            .into_iter()
            .chain(bit_table.words.iter().copied())
            .enumerate()
        {
            let offset = BIT_TABLE_OFFSET + index * 4;
            resource[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }

        for (index, bits) in (0_u32..16).map(|index| (index, 0x3f00_0000 + index)) {
            let offset = VOLUME_OFFSET + usize::try_from(index).unwrap() * 4;
            resource[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }
        resource[VOLUME_OFFSET + 64..VOLUME_OFFSET + 68]
            .copy_from_slice(&(skate2_formats::collision::VolumeType::Sphere as u32).to_be_bytes());
        for (word_index, bits) in [
            0x0123_4567_u32,
            0x89ab_cdef,
            0xfedc_ba98,
            0x3f40_0000,
            0x1357_2468,
            0x2468_1357,
            0x8000_0001,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = VOLUME_OFFSET + 68 + word_index * 4;
            resource[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }

        let definition_relative_parts = u32::try_from(parts_offset)
            .unwrap()
            .wrapping_sub(u32::try_from(definition_offset).unwrap());
        resource[definition_offset..definition_offset + 4]
            .copy_from_slice(&definition_relative_parts.to_be_bytes());
        resource[definition_offset + 4..definition_offset + 8]
            .copy_from_slice(&part_count.to_be_bytes());
        resource[definition_offset + 8..definition_offset + 12]
            .copy_from_slice(&7_u32.to_be_bytes());
        resource[definition_offset + 24..definition_offset + 28]
            .copy_from_slice(&9_u32.to_be_bytes());
        resource[definition_offset + 40..definition_offset + 44]
            .copy_from_slice(&ObjectId(1).0.to_be_bytes());
        resource[definition_offset + 44..definition_offset + 48]
            .copy_from_slice(&0x5566_7788_u32.to_be_bytes());
        for part_ordinal in 0..usize::try_from(part_count).unwrap() {
            let part = parts_offset + part_ordinal * 0xa0;
            resource[part..part + 4].copy_from_slice(&part_volume_id.0.to_be_bytes());
            for (index, bits) in [
                0x8000_0000_u32,
                0x7fc1_2345,
                0xffc5_4321,
                0x3f80_0000,
                0x3f00_0000,
                0xbf00_0000,
                0x7f7f_ffff,
                0xff7f_ffff,
                0x3dca_feba,
                0xbdca_feba,
            ]
            .into_iter()
            .enumerate()
            {
                resource[part + 16 + index * 4..part + 20 + index * 4]
                    .copy_from_slice(&bits.to_be_bytes());
            }
            for index in 0_u32..16 {
                let offset = part + 64 + usize::try_from(index).unwrap() * 4;
                resource[offset..offset + 4].copy_from_slice(&(0x3e80_0000 + index).to_be_bytes());
            }
            for (index, bits) in [0x8000_0000_u32, 0x7fc9_abcd, 0x3e4c_cccd]
                .into_iter()
                .enumerate()
            {
                resource[part + 128 + index * 4..part + 132 + index * 4]
                    .copy_from_slice(&bits.to_be_bytes());
            }
            resource[part + 140..part + 144].copy_from_slice(&0x8000_0001_u32.to_be_bytes());
            resource[part + 144..part + 148].copy_from_slice(&0x89ab_cdef_u32.to_be_bytes());
        }

        resource[ASSEMBLY_OFFSET..ASSEMBLY_OFFSET + 4]
            .copy_from_slice(&ObjectId(4).0.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 4..ASSEMBLY_OFFSET + 8]
            .copy_from_slice(&definition_id.0.to_be_bytes());
        for (word_index, bits) in [
            0x1111_1111_u32,
            0x2222_2222,
            0x3333_3333,
            0x4444_4444,
            0x5555_5555,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = ASSEMBLY_OFFSET + 8 + word_index * 4;
            resource[offset..offset + 4].copy_from_slice(&bits.to_be_bytes());
        }
        resource[ASSEMBLY_OFFSET + 28..ASSEMBLY_OFFSET + 32]
            .copy_from_slice(&assembly_part_count.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 32..ASSEMBLY_OFFSET + 36].copy_from_slice(&7_u32.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 36..ASSEMBLY_OFFSET + 40].copy_from_slice(&9_u32.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 40..ASSEMBLY_OFFSET + 44]
            .copy_from_slice(&0xdead_beef_u32.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 44..ASSEMBLY_OFFSET + 46]
            .copy_from_slice(&0x1357_u16.to_be_bytes());
        resource[ASSEMBLY_OFFSET + 46..ASSEMBLY_OFFSET + 48]
            .copy_from_slice(&0x2468_u16.to_be_bytes());

        let dictionary = vec![
            dictionary_entry(0, 0, 0),
            dictionary_entry(VOLUME_OFFSET, 0x60, ObjectType::Volume as u32),
            dictionary_entry(definition_offset, 0x30, ASSEMBLY_DEFINITION_TYPE_ID),
            dictionary_entry(ASSEMBLY_OFFSET, 0x30, ASSEMBLY_TYPE_ID),
            dictionary_entry(
                BIT_TABLE_OFFSET,
                12 + bit_table.words.len() * 4,
                BIT_TABLE_TYPE_ID,
            ),
        ];
        let descriptor = ResourceDescriptor {
            size: u32::try_from(resource.len()).unwrap(),
            alignment: 16,
        };
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
            entry_count: u32::try_from(dictionary.len()).unwrap(),
            used_entry_count: u32::try_from(dictionary.len()).unwrap(),
            alignment: 16,
            virtual_size: u32::try_from(resource.len()).unwrap(),
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor; 5],
            resources_used: [descriptor; 5],
            resource_offsets: [0; 5],
            arena_group_offset: 0,
            dictionary,
        };
        let decoded = DecodedArenaAsset::from_parts_for_test(
            0x0123_4567_89ab_cdef,
            WORLD_STREAM_ARENA_ASSET_TYPE,
            ArenaResources {
                resources: [resource, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            },
            arena,
        );
        let collision = SimulationCollisionAsset::from_decoded(decoded).unwrap();
        Arc::new(SimulationNormalArenaPlan::from_collision_asset(collision).unwrap())
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one source fixture verifies the complete Definition-order Part payload"
    )]
    fn retains_owner_and_complete_definition_ordered_part_source() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let weak_owner = Arc::downgrade(&owner);
        let template =
            SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        drop(owner);

        assert!(weak_owner.upgrade().is_some());
        assert_eq!(template.assembly_object_id(), ObjectId(3));
        assert_eq!(template.definition_object_id(), ObjectId(2));
        let culling = template.culling_table().expect("culling table");
        assert!(Arc::ptr_eq(culling.owner(), template.owner()));
        assert_eq!(culling.object_id(), Some(ObjectId(4)));
        assert_eq!(
            culling.table(),
            &BitTable {
                rows: 1,
                columns: 1,
                array_size: 1,
                words: vec![0x8000_0000],
            }
        );
        assert_eq!(template.assembly().part_count, 1);
        assert_eq!(template.definition().part_count, 1);
        let [part] = template.parts() else {
            panic!("expected one Part")
        };
        assert_eq!(part.volume_object_id, ObjectId(1));
        assert_eq!(part.root_index, 0);
        assert!(Arc::ptr_eq(part.inertia_source.owner(), template.owner()));
        assert_eq!(part.inertia_source.resource_index(), 0);
        assert_eq!(part.inertia_source.resource_offset(), 0x110);
        assert_eq!(part.inertia_source.bytes().len(), 0x30);
        assert_eq!(
            &part.inertia_source.bytes()[40..],
            &[0; 8],
            "the retained span includes Inertia padding"
        );
        assert_ne!(
            &part.inertia_source.bytes()[32..],
            &template
                .owner()
                .static_plan()
                .asset()
                .decoded
                .resources
                .resources[0][PARTS_OFFSET + 0x40..PARTS_OFFSET + 0x50],
            "the 0x30-byte source must not bleed into inverseBodyTransform at +0x40"
        );
        assert_eq!(
            part.inertia.inverse_tensor.map(f32::to_bits),
            [0x8000_0000, 0x7fc1_2345, 0xffc5_4321, 0x3f80_0000]
        );
        assert_eq!(
            [
                part.material.static_friction.to_bits(),
                part.material.dynamic_friction.to_bits(),
                part.material.restitution.to_bits(),
            ],
            [0x8000_0000, 0x7fc9_abcd, 0x3e4c_cccd]
        );
        assert_eq!(part.has_inverse_body_transform, 0x8000_0001);
        assert_eq!(part.is_static_body, 0x89ab_cdef);
        let runtime_inertia = template.runtime_inertia(0).expect("first Part");
        assert!(template.runtime_inertia(1).is_none());
        assert_eq!(
            runtime_inertia.semantic().inverse_tensor.map(f32::to_bits),
            [
                part.inertia.inverse_tensor[0].to_bits(),
                part.inertia.inverse_tensor[1].to_bits(),
                part.inertia.inverse_tensor[2].to_bits(),
            ]
        );
        assert_eq!(
            [
                runtime_inertia.semantic().inverse_mass.to_bits(),
                runtime_inertia.semantic().spherical.to_bits(),
                runtime_inertia.semantic().max_velocity.to_bits(),
                runtime_inertia.semantic().max_omega.to_bits(),
                runtime_inertia.semantic().linear_drag.to_bits(),
                runtime_inertia.semantic().angular_drag.to_bits(),
            ],
            [
                part.inertia.inverse_mass.to_bits(),
                part.inertia.spherical.to_bits(),
                part.inertia.max_velocity.to_bits(),
                part.inertia.max_omega.to_bits(),
                part.inertia.linear_drag.to_bits(),
                part.inertia.angular_drag.to_bits(),
            ]
        );
        assert!(
            runtime_inertia
                .source()
                .is_same_backing_inertia(&part.inertia_source)
        );
        let root = template.root_volume(0);
        assert_eq!(root.vtable_offset, 1);
        assert_eq!(root.shape_words, [0x0123_4567, 0x89ab_cdef, 0xfedc_ba98]);
        assert_eq!(root.radius.to_bits(), 0x3f40_0000);
        assert_eq!(root.group_id, 0x1357_2468);
        assert_eq!(root.surface_id, 0x2468_1357);
        assert_eq!(root.flags, 0x8000_0001);
    }

    #[test]
    fn inertia_sources_preserve_stride_location_and_owner_identity() {
        let owner = source_plan(2, 2, ObjectId(2), ObjectId(1));
        let first = SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        let alias = SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        let foreign = SerialisedAssemblyTemplate::try_new(
            source_plan(2, 2, ObjectId(2), ObjectId(1)),
            ObjectId(3),
        )
        .unwrap();

        assert_eq!(first.parts()[0].inertia_source.resource_offset(), 0x110);
        assert_eq!(first.parts()[1].inertia_source.resource_offset(), 0x1b0);
        assert_eq!(
            first.parts()[0].inertia_source.bytes(),
            first.parts()[1].inertia_source.bytes(),
            "equal source bytes at distinct Part locations must not imply aliasing"
        );
        assert_eq!(
            first.parts()[1].inertia_source.resource_offset()
                - first.parts()[0].inertia_source.resource_offset(),
            ASSEMBLY_PART_DEFINITION_STRIDE
        );
        assert!(
            first.parts()[0]
                .inertia_source
                .is_same_backing_inertia(&alias.parts()[0].inertia_source)
        );
        assert!(
            !first.parts()[0]
                .inertia_source
                .is_same_backing_inertia(&first.parts()[1].inertia_source)
        );
        assert!(
            !first.parts()[0]
                .inertia_source
                .is_same_backing_inertia(&foreign.parts()[0].inertia_source)
        );
    }

    #[test]
    fn inertia_source_resolves_part_allocation_before_definition_header() {
        const EARLIER_PARTS_OFFSET: usize = 0x200;
        const LATER_DEFINITION_OFFSET: usize = 0x500;
        let owner = source_plan_with_layout(
            1,
            1,
            ObjectId(2),
            ObjectId(1),
            &BitTable {
                rows: 1,
                columns: 1,
                array_size: 1,
                words: vec![0x8000_0000],
            },
            LATER_DEFINITION_OFFSET,
            EARLIER_PARTS_OFFSET,
        );

        let template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        let [part] = template.parts() else {
            panic!("expected one Part")
        };
        assert_eq!(
            part.inertia_source.resource_offset(),
            u32::try_from(EARLIER_PARTS_OFFSET + 0x10).unwrap()
        );
        assert_eq!(
            part.inertia_source.bytes(),
            &template
                .owner()
                .static_plan()
                .asset()
                .decoded
                .resources
                .resources[0][EARLIER_PARTS_OFFSET + 0x10..EARLIER_PARTS_OFFSET + 0x40]
        );
    }

    #[test]
    fn inertia_source_refuses_checked_suffix_overflow_and_outside_span() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let mut entry = dictionary_entry(0, 0x30, ASSEMBLY_DEFINITION_TYPE_ID);
        entry.object_offset = u32::MAX;
        assert_eq!(
            definition_part_inertia_source(&owner, &entry, 0, 0).unwrap_err(),
            SerialisedAssemblyTemplateError::InertiaSourceOffsetOverflow { part_ordinal: 0 }
        );

        entry.object_offset = 0x12c0;
        let exact_tail = definition_part_inertia_source(&owner, &entry, 0, 0).unwrap();
        assert_eq!(exact_tail.resource_offset(), 0x12d0);
        assert_eq!(exact_tail.bytes().len(), 0x30);

        entry.object_offset = 0x12c1;
        assert_eq!(
            definition_part_inertia_source(&owner, &entry, 0, 0).unwrap_err(),
            SerialisedAssemblyTemplateError::InertiaSourceRangeOutsideResource {
                part_ordinal: 0,
                resource_index: 0,
                resource_offset: 0x12d1,
                resource_size: 0x1300,
            }
        );

        let mut foreign_resource = dictionary_entry(0, 0x30, ASSEMBLY_DEFINITION_TYPE_ID);
        foreign_resource.resource_index = 1;
        assert_eq!(
            definition_part_inertia_source(&owner, &foreign_resource, 0, 0).unwrap_err(),
            SerialisedAssemblyTemplateError::InertiaSourceRangeOutsideResource {
                part_ordinal: 0,
                resource_index: 1,
                resource_offset: 0x10,
                resource_size: 0,
            },
            "the Definition entry, not the Assembly entry, selects the source resource"
        );
    }

    #[test]
    fn culling_preflight_retains_then_refuses_insufficient_stored_words() {
        let owner = source_plan_with_bit_table(
            1,
            1,
            ObjectId(2),
            ObjectId(1),
            &BitTable {
                rows: 1,
                columns: 33,
                array_size: 1,
                words: vec![0xdead_beef],
            },
        );
        let template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        let culling = template.culling_table().expect("culling table");
        assert_eq!(culling.table().array_size, 1);
        assert_eq!(culling.table().words, vec![0xdead_beef]);
        let (pool, parts) = single_body_fixture(3);
        let mut packed = FixedPackedWords::default();
        let rejected = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: Some(culling),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut NeverNumeric,
            &mut packed,
        );
        assert!(matches!(
            rejected,
            Err(crate::serialised_assembly_create::SerialisedAssemblyCreateError::CullingWordCount {
                stored: 1,
                dimensions: 2
            })
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());

        let mut numeric = OrdinaryFiniteNumeric;
        let mut prepared = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 7,
                culling_table: None,
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        )
        .unwrap();
        let before_header = prepared.header().clone();
        let before_records = prepared.volume_data().clone();
        packed.transform_calls.clear();
        packed.motion_calls.clear();
        let rejected = prepared.try_update(
            &template,
            &pool,
            &parts,
            &[1],
            SerialisedAssemblyLiveAssemblyInput {
                group: 9,
                culling_table: Some(culling),
            },
            &mut packed,
        );
        assert!(matches!(
            rejected,
            Err(crate::serialised_assembly_create::SerialisedAssemblyUpdateError::CullingWordCount {
                stored: 1,
                dimensions: 2
            })
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());
        assert_eq!(prepared.header(), &before_header);
        assert_eq!(prepared.volume_data(), &before_records);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one real-provider story keeps Create, both authority axes, arena refusal, and Update adjacent"
    )]
    fn production_wrappers_keep_the_real_provider_bound_through_create_and_update() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let template =
            SerialisedAssemblyTemplate::try_new(Arc::clone(&owner), ObjectId(3)).unwrap();
        let binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let mut inertia_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&pool, binding).unwrap();
        let runtime_inertia = template.runtime_inertia(0).unwrap();
        let transform = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ];
        let body = inertia_owner
            .try_add(
                &mut pool,
                transform,
                SimulationRigidBodyAddInertiaInput::Definition(&runtime_inertia),
                4,
            )
            .unwrap()
            .unwrap();
        let parts = [SerialisedAssemblyCreatePartInput {
            body: body.observation(),
            part_transform: transform,
            material: AssemblyPartMaterial {
                static_friction: 1.0,
                dynamic_friction: 1.0,
                restitution: 0.0,
            },
            user_data: 0,
        }];
        let image = SimulationXenonAddressImage::try_new(0x1000_0080, 1, 0, 0).unwrap();
        let image_domain = SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        let mut numeric = OrdinaryFiniteNumeric;
        let mut prepared = try_prepare_serialised_assembly_create_from_xenon(
            &template,
            &pool,
            &inertia_owner,
            &image_domain,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 7,
                culling_table: template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
        )
        .unwrap();
        let create_header = prepared.header().clone();
        let create_volume_data = prepared.volume_data().clone();

        let replacement_domain =
            SimulationPackedBodyWordImageDomain::try_new(&pool, image).unwrap();
        assert_eq!(
            prepared.try_update_from_xenon(
                &template,
                &pool,
                &inertia_owner,
                &replacement_domain,
                &parts,
                &[1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 8,
                    culling_table: template.culling_table(),
                },
            ),
            Err(crate::serialised_assembly_create::SerialisedAssemblyUpdateError::PackedBodyWordAuthorityMismatch)
        );
        assert_eq!(prepared.header(), &create_header);
        assert_eq!(prepared.volume_data(), &create_volume_data);

        let replacement_binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x2000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let replacement_inertia_owner =
            SimulationRigidBodyAddInertiaOwner::try_new(&pool, replacement_binding).unwrap();
        assert_eq!(
            prepared.try_update_from_xenon(
                &template,
                &pool,
                &replacement_inertia_owner,
                &image_domain,
                &parts,
                &[1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 8,
                    culling_table: template.culling_table(),
                },
            ),
            Err(crate::serialised_assembly_create::SerialisedAssemblyUpdateError::PackedBodyWordAuthorityMismatch)
        );
        assert_eq!(prepared.header(), &create_header);
        assert_eq!(prepared.volume_data(), &create_volume_data);

        let foreign_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        assert!(matches!(
            try_prepare_serialised_assembly_create_from_xenon(
                &template,
                &foreign_pool,
                &inertia_owner,
                &image_domain,
                &parts,
                SerialisedAssemblyLiveAssemblyInput {
                    group: 8,
                    culling_table: None,
                },
                SerialisedAssemblyParamsState::native_default(),
                &mut numeric,
            ),
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyCreateError::PackedBodyWords(
                    SimulationPackedBodyWordProviderError::ForeignImageBodyPool
                )
            )
        ));
        let foreign_pool_binding = Arc::new(
            SimulationArenaXenonAddressBinding::try_new(
                Arc::clone(&owner),
                [0x3000_0000, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        let foreign_pool_inertia =
            SimulationRigidBodyAddInertiaOwner::try_new(&foreign_pool, foreign_pool_binding)
                .unwrap();
        assert!(matches!(
            try_prepare_serialised_assembly_create_from_xenon(
                &template,
                &pool,
                &foreign_pool_inertia,
                &image_domain,
                &parts,
                SerialisedAssemblyLiveAssemblyInput {
                    group: 8,
                    culling_table: None,
                },
                SerialisedAssemblyParamsState::native_default(),
                &mut numeric,
            ),
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyCreateError::PackedBodyWords(
                    SimulationPackedBodyWordProviderError::ForeignInertiaBodyPool
                )
            )
        ));

        let foreign_owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let foreign_template =
            SerialisedAssemblyTemplate::try_new(foreign_owner, ObjectId(3)).unwrap();
        assert!(matches!(
            try_prepare_serialised_assembly_create_from_xenon(
                &foreign_template,
                &pool,
                &inertia_owner,
                &image_domain,
                &parts,
                SerialisedAssemblyLiveAssemblyInput {
                    group: 8,
                    culling_table: None,
                },
                SerialisedAssemblyParamsState::native_default(),
                &mut numeric,
            ),
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyCreateError::PackedBodyWords(
                    SimulationPackedBodyWordProviderError::ForeignArenaOwner
                )
            )
        ));

        prepared
            .try_update_from_xenon(
                &template,
                &pool,
                &inertia_owner,
                &image_domain,
                &parts,
                &[1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 9,
                    culling_table: template.culling_table(),
                },
            )
            .unwrap();
        assert_eq!(prepared.header().group, 9);
        assert_eq!(prepared.rigid_bodies(), &[body.observation()]);
    }

    #[test]
    fn nonnull_zero_dimension_and_oversized_culling_keep_identity_and_derived_size() {
        for (table, expected_size) in [
            (
                BitTable {
                    rows: 0,
                    columns: 99,
                    array_size: 0,
                    words: Vec::new(),
                },
                12,
            ),
            (
                BitTable {
                    rows: 1,
                    columns: 1,
                    array_size: 2,
                    words: vec![0x1111_1111, 0x2222_2222],
                },
                16,
            ),
        ] {
            let owner = source_plan_with_bit_table(1, 1, ObjectId(2), ObjectId(1), &table);
            let template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
            assert_eq!(template.culling_table().unwrap().table(), &table);
            let (pool, parts) = single_body_fixture(3);
            let mut numeric = OrdinaryFiniteNumeric;
            let mut packed = FixedPackedWords::default();
            let prepared = try_prepare_serialised_assembly_create(
                &template,
                &pool,
                &parts,
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0,
                    culling_table: template.culling_table(),
                },
                SerialisedAssemblyParamsState::native_default(),
                &mut numeric,
                &mut packed,
            )
            .unwrap();
            assert_eq!(
                prepared.header().culling_table,
                template.culling_table().cloned()
            );
            assert_eq!(prepared.header().culling_table_size, expected_size);
        }
    }

    #[test]
    fn packed_callbacks_follow_selected_transform_and_primitive_motion_reads() {
        let owner = source_plan(2, 2, ObjectId(2), ObjectId(1));
        let mut template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        template.volume_sources[0].volume_type = VolumeType::Aggregate;
        template.volume_sources[0].volume.vtable_offset = VolumeType::Aggregate as u32;

        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let transforms = [
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 1.0,
            ],
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 20.0, 0.0, 0.0, 1.0,
            ],
        ];
        let observations = transforms.map(|transform| {
            let handle = pool
                .try_add_rigid_body(transform, Some(&inertia), 3)
                .unwrap()
                .unwrap();
            pool.try_observe(handle).unwrap()
        });
        let parts: [SerialisedAssemblyCreatePartInput; 2] =
            std::array::from_fn(|ordinal| SerialisedAssemblyCreatePartInput {
                body: observations[ordinal],
                part_transform: transforms[ordinal],
                material: AssemblyPartMaterial {
                    static_friction: 1.0,
                    dynamic_friction: 1.0,
                    restitution: 0.0,
                },
                user_data: u32::try_from(ordinal).unwrap(),
            });
        let mut numeric = OrdinaryFiniteNumeric;
        let mut packed = FixedPackedWords::default();
        let prepared = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: None,
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        )
        .unwrap();
        assert_eq!(prepared.rigid_bodies(), &observations);
        assert_eq!(packed.transform_calls, observations);
        assert_eq!(packed.motion_calls, vec![observations[1]]);
        assert_eq!(
            prepared.header().aabb,
            crate::octree::AabbWords {
                min: [1_100_919_603, 3_190_187_540, 3_190_187_540, 1_094_276_010],
                max: [1_101_745_357, 1_068_813_526, 1_068_813_526, 1_096_129_366],
            }
        );

        template.update_volume_sources.remove(0);
        let mut update_words = FixedPackedWords::default();
        let mut prepared = prepared;
        prepared
            .try_update(
                &template,
                &pool,
                &parts,
                &[1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0,
                    culling_table: None,
                },
                &mut update_words,
            )
            .unwrap();
        assert_eq!(update_words.transform_calls, vec![observations[1]]);
        assert!(update_words.motion_calls.is_empty());

        let owner = source_plan(2, 2, ObjectId(2), ObjectId(1));
        let mut template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        template.volume_sources.remove(0);
        let mut packed = FixedPackedWords::default();
        let prepared = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: None,
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        )
        .unwrap();
        assert_eq!(prepared.header().num_volumes, 1);
        assert_eq!(prepared.rigid_bodies(), &observations);
        assert_eq!(packed.transform_calls, vec![observations[1]]);
        assert_eq!(packed.motion_calls, vec![observations[1]]);
    }

    #[test]
    fn dictionary_scan_and_native_part_capacity_are_independent() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let templates = SerialisedAssemblyTemplate::all_in_plan(&owner).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].assembly_object_id(), ObjectId(3));

        let oversized = source_plan(27, 27, ObjectId(2), ObjectId(1));
        assert_eq!(
            SerialisedAssemblyTemplate::try_new(oversized, ObjectId(3)).unwrap_err(),
            SerialisedAssemblyTemplateError::PartCapacityExceeded {
                count: 27,
                capacity: 26,
            }
        );
    }

    #[test]
    fn rejects_count_nonlocal_and_typed_link_mutations() {
        let mismatched = source_plan(1, 2, ObjectId(2), ObjectId(1));
        assert_eq!(
            SerialisedAssemblyTemplate::try_new(mismatched, ObjectId(3)).unwrap_err(),
            SerialisedAssemblyTemplateError::PartCountMismatch {
                assembly: 2,
                definition: 1,
                parsed: 1,
            }
        );

        let nonlocal = source_plan(1, 1, ObjectId((3 << 22) | 2), ObjectId(1));
        assert_eq!(
            SerialisedAssemblyTemplate::try_new(nonlocal, ObjectId(3)).unwrap_err(),
            SerialisedAssemblyTemplateError::NonLocalObjectReference {
                link: SerialisedAssemblyLink::Definition,
                object_id: (3 << 22) | 2,
                section: 3,
            }
        );

        let wrong_part_type = source_plan(1, 1, ObjectId(2), ObjectId(2));
        assert_eq!(
            SerialisedAssemblyTemplate::try_new(wrong_part_type, ObjectId(3)).unwrap_err(),
            SerialisedAssemblyTemplateError::WrongObjectType {
                link: SerialisedAssemblyLink::PartVolume { part_ordinal: 0 },
                object_id: 2,
                expected: ObjectType::Volume as u32,
                actual: ASSEMBLY_DEFINITION_TYPE_ID,
            }
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one source-shaped Create/Update record story keeps immutable and live assertions adjacent"
    )]
    fn volume_data_create_and_update_keep_immutable_and_live_fields_separate() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        let part_transform = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 90.0, 80.0, 70.0, 1.0,
        ];
        let body_transform = [
            0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        let initial = SerialisedAssemblyLivePartState {
            part_transform,
            body_transform,
            material: AssemblyPartMaterial {
                static_friction: f32::from_bits(0x8000_0000),
                dynamic_friction: f32::from_bits(0x7fc1_2345),
                restitution: f32::from_bits(0xffc5_4321),
            },
            user_data: 0x89ab_cdef,
            body_state: 3,
        };
        let mut data = SerialisedAssemblyVolumeDataSet::try_create(&template, &[initial]).unwrap();
        let [created] = data.records() else {
            panic!("expected one Create-selected VolumeData record")
        };
        let source = &template.volume_sources()[0];
        assert_eq!(created.rigid_body_index, source.rigid_body_index);
        assert_eq!(created.volume_type, source.volume_type);
        assert_eq!(created.volume, source.volume);
        assert_eq!(created.padding.to_bits(), 0x3d4c_cccd);
        assert_eq!(created.surface_id, source.surface_id);
        assert_eq!(
            [
                created.material.static_friction.to_bits(),
                created.material.dynamic_friction.to_bits(),
                created.material.restitution.to_bits(),
            ],
            [0x8000_0000, 0x7fc1_2345, 0xffc5_4321]
        );
        assert_eq!(
            created.transform.map(f32::to_bits),
            part_get_transform(
                initial.part_transform,
                Some(&initial.body_transform),
                Some(&template.parts()[0].inverse_body_transform),
            )
            .map(f32::to_bits)
        );
        assert!(created.enabled);
        assert_eq!(created.group, initial.user_data);
        assert!(created.frozen);

        let immutable_before = (
            created.rigid_body_index,
            created.volume_type,
            created.volume.clone(),
            created.padding.to_bits(),
            created.surface_id,
        );
        let changed = SerialisedAssemblyLivePartState {
            part_transform: [f32::from_bits(0x7fc0_5555); 16],
            body_transform: [
                1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -2.0, -3.0, -4.0, 1.0,
            ],
            material: AssemblyPartMaterial {
                static_friction: -1.0,
                dynamic_friction: 2.0,
                restitution: 3.0,
            },
            user_data: 0x1357_2468,
            body_state: 1,
        };
        data.try_update(&template, &[changed], &[0x8000_0000])
            .unwrap();
        let [updated] = data.records() else {
            panic!("expected one updated VolumeData record")
        };
        assert_eq!(
            (
                updated.rigid_body_index,
                updated.volume_type,
                updated.volume.clone(),
                updated.padding.to_bits(),
                updated.surface_id,
            ),
            immutable_before
        );
        assert_eq!(updated.material, changed.material);
        assert_eq!(
            updated.transform.map(f32::to_bits),
            part_get_transform(
                changed.part_transform,
                Some(&changed.body_transform),
                Some(&template.parts()[0].inverse_body_transform),
            )
            .map(f32::to_bits)
        );
        assert!(!updated.enabled);
        assert_eq!(updated.group, changed.user_data);
        assert!(!updated.frozen);

        let before_error = data.clone();
        assert_eq!(
            data.try_update(&template, &[changed], &[]),
            Err(
                SerialisedAssemblyVolumeDataStateError::UpdateVolumeFlagCount {
                    selected: 1,
                    flags: 0,
                }
            )
        );
        assert_eq!(data, before_error);

        let other_owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let other = SerialisedAssemblyTemplate::try_new(other_owner, ObjectId(3)).unwrap();
        assert_eq!(
            data.try_update(&other, &[changed], &[1]),
            Err(SerialisedAssemblyVolumeDataStateError::TemplateMismatch)
        );
        assert_eq!(data, before_error);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one source-shaped Create/Update ownership story keeps immutable-field assertions adjacent"
    )]
    fn staged_cpu_create_binds_part_order_bodies_header_and_borrowed_culling() {
        let owner = source_plan(2, 2, ObjectId(2), ObjectId(1));
        let mut template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        let inverse = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 4.0, 6.0, 8.0, 1.0,
        ];
        for part in &mut template.parts {
            part.inverse_body_transform = inverse;
            part.has_inverse_body_transform = 1;
        }
        let mut pool = SimulationRigidBodyPool::try_new(2).unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let first_transform = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        let second_transform = [
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -4.0, -5.0, -6.0, 1.0,
        ];
        let first = pool
            .try_add_rigid_body(first_transform, Some(&inertia), 1)
            .unwrap()
            .unwrap();
        let second = pool
            .try_add_rigid_body(second_transform, Some(&inertia), 2)
            .unwrap()
            .unwrap();
        let first = pool.try_observe(first).unwrap();
        let second = pool.try_observe(second).unwrap();
        let material = AssemblyPartMaterial {
            static_friction: 1.0,
            dynamic_friction: 2.0,
            restitution: 3.0,
        };
        let parts = [
            SerialisedAssemblyCreatePartInput {
                body: first,
                part_transform: first_transform,
                material,
                user_data: 0x1111_1111,
            },
            SerialisedAssemblyCreatePartInput {
                body: second,
                part_transform: second_transform,
                material,
                user_data: 0x2222_2222,
            },
        ];
        let params = SerialisedAssemblyParamsState {
            edge_cos_bend_normal_threshold: f32::from_bits(0x8000_0000),
        };
        let mut numeric = OrdinaryFiniteNumeric;
        let mut packed = FixedPackedWords::default();
        let prepared = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &parts,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0x89ab_cdef,
                culling_table: template.culling_table(),
            },
            params,
            &mut numeric,
            &mut packed,
        )
        .unwrap();

        assert!(Arc::ptr_eq(prepared.owner(), template.owner()));
        assert_eq!(prepared.definition_object_id(), ObjectId(2));
        assert_eq!(prepared.rigid_bodies(), &[first, second]);
        assert_eq!(packed.transform_calls, vec![first, second]);
        assert_eq!(packed.motion_calls, vec![first, second]);
        assert_eq!(
            prepared.params().edge_cos_bend_normal_threshold.to_bits(),
            0x8000_0000
        );
        let header = prepared.header();
        assert_eq!(header.num_volumes, 2);
        assert_eq!(header.num_rigid_bodies, 2);
        assert_eq!(header.num_static, 1);
        assert_eq!(header.group, 0x89ab_cdef);
        assert_eq!(header.culling_table, template.culling_table().cloned());
        assert_eq!(header.culling_table_size, 16);
        assert!(!header.all_parts_frozen);
        assert_eq!(header.assembly_object_id, ObjectId(3));
        assert_ne!(header.aabb.min, [0; 4]);
        assert_eq!(
            prepared
                .volume_data()
                .records()
                .iter()
                .map(|record| record.rigid_body_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            prepared
                .volume_data()
                .records()
                .iter()
                .map(|record| {
                    [
                        record.transform[3],
                        record.transform[7],
                        record.transform[11],
                        record.transform[15],
                    ]
                    .map(f32::to_bits)
                })
                .collect::<Vec<_>>(),
            vec![
                [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0, 33.0_f32.to_bits()],
                [3.0_f32.to_bits(), 4.0_f32.to_bits(), 0, 44.0_f32.to_bits()],
            ]
        );

        let immutable_header = (
            header.num_volumes,
            header.num_rigid_bodies,
            header.assembly_object_id,
            header.aabb,
        );
        let immutable_params = prepared.params();
        let immutable_bodies = prepared.rigid_bodies().to_vec();
        let changed_material = AssemblyPartMaterial {
            static_friction: -1.0,
            dynamic_friction: -2.0,
            restitution: -3.0,
        };
        let changed_parts = parts.map(|part| SerialisedAssemblyCreatePartInput {
            material: changed_material,
            user_data: 0x7654_3210,
            ..part
        });
        let mut update_words = FixedPackedWords::default();
        let mut prepared = prepared;
        prepared
            .try_update(
                &template,
                &pool,
                &changed_parts,
                &[0, 0],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0x0102_0304,
                    culling_table: None,
                },
                &mut update_words,
            )
            .unwrap();
        let updated_header = prepared.header();
        assert_eq!(
            (
                updated_header.num_volumes,
                updated_header.num_rigid_bodies,
                updated_header.assembly_object_id,
                updated_header.aabb,
            ),
            immutable_header
        );
        assert_eq!(prepared.params(), immutable_params);
        assert_eq!(prepared.rigid_bodies(), immutable_bodies);
        assert_eq!(updated_header.num_static, 1);
        assert_eq!(updated_header.group, 0x0102_0304);
        assert_eq!(updated_header.culling_table, None);
        assert_eq!(updated_header.culling_table_size, 0);
        assert!(!updated_header.all_parts_frozen);
        assert_eq!(update_words.transform_calls, vec![first, second]);
        assert!(update_words.motion_calls.is_empty());
        assert!(
            prepared
                .volume_data()
                .records()
                .iter()
                .all(|record| !record.enabled
                    && record.material == changed_material
                    && record.group == 0x7654_3210)
        );

        let before_preflight_header = prepared.header().clone();
        let before_preflight_volume_data = prepared.volume_data().clone();
        let mut bad_shape_words = FixedPackedWords::default();
        assert!(matches!(
            prepared.try_update(
                &template,
                &pool,
                &changed_parts,
                &[],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0,
                    culling_table: None,
                },
                &mut bad_shape_words,
            ),
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyUpdateError::VolumeData(
                    SerialisedAssemblyVolumeDataStateError::UpdateVolumeFlagCount {
                        selected: 2,
                        flags: 0
                    }
                )
            )
        ));
        assert!(bad_shape_words.transform_calls.is_empty());
        assert!(bad_shape_words.motion_calls.is_empty());
        assert_eq!(prepared.header(), &before_preflight_header);
        assert_eq!(prepared.volume_data(), &before_preflight_volume_data);

        let mut foreign_domain_words = FixedPackedWords {
            authority: SerialisedAssemblyPackedBodyWordAuthority::for_test(2),
            ..FixedPackedWords::default()
        };
        assert!(matches!(
            prepared.try_update(
                &template,
                &pool,
                &changed_parts,
                &[1, 1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0,
                    culling_table: None,
                },
                &mut foreign_domain_words,
            ),
            Err(crate::serialised_assembly_create::SerialisedAssemblyUpdateError::PackedBodyWordAuthorityMismatch)
        ));
        assert!(foreign_domain_words.transform_calls.is_empty());
        assert!(foreign_domain_words.motion_calls.is_empty());
        assert_eq!(prepared.header(), &before_preflight_header);
        assert_eq!(prepared.volume_data(), &before_preflight_volume_data);

        let before_rejected_update = prepared.volume_data().clone();
        let swapped_parts = [changed_parts[1], changed_parts[0]];
        let mut rejected_words = FixedPackedWords::default();
        assert!(matches!(
            prepared.try_update(
                &template,
                &pool,
                &swapped_parts,
                &[1, 1],
                SerialisedAssemblyLiveAssemblyInput {
                    group: 0,
                    culling_table: template.culling_table(),
                },
                &mut rejected_words,
            ),
            Err(crate::serialised_assembly_create::SerialisedAssemblyUpdateError::BodyOccurrenceChanged {
                ordinal: 0
            })
        ));
        assert!(rejected_words.transform_calls.is_empty());
        assert!(rejected_words.motion_calls.is_empty());
        assert_eq!(prepared.volume_data(), &before_rejected_update);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one refusal test keeps callback-free publication policies adjacent"
    )]
    fn staged_cpu_create_refuses_foreign_culling_body_and_no_bbox_policy() {
        let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let mut template = SerialisedAssemblyTemplate::try_new(owner, ObjectId(3)).unwrap();
        let other_owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
        let other_template = SerialisedAssemblyTemplate::try_new(other_owner, ObjectId(3)).unwrap();
        let mut pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let inertia = Arc::new(SimulationInertia::native_default());
        let transform = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let body = pool
            .try_add_rigid_body(transform, Some(&inertia), 1)
            .unwrap()
            .unwrap();
        let observation = pool.try_observe(body).unwrap();
        let input = [SerialisedAssemblyCreatePartInput {
            body: observation,
            part_transform: transform,
            material: AssemblyPartMaterial {
                static_friction: 1.0,
                dynamic_friction: 1.0,
                restitution: 0.0,
            },
            user_data: 0,
        }];
        let mut numeric = OrdinaryFiniteNumeric;
        let mut packed = FixedPackedWords::default();
        let foreign_culling = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &input,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: other_template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        );
        assert!(matches!(
            foreign_culling,
            Err(crate::serialised_assembly_create::SerialisedAssemblyCreateError::CullingOwnerMismatch)
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());

        let mut foreign_pool = SimulationRigidBodyPool::try_new(1).unwrap();
        let foreign = foreign_pool
            .try_add_rigid_body(transform, Some(&inertia), 1)
            .unwrap()
            .unwrap();
        let foreign = foreign_pool.try_observe(foreign).unwrap();
        let foreign_input = [SerialisedAssemblyCreatePartInput {
            body: foreign,
            ..input[0]
        }];
        let foreign_body = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &foreign_input,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        );
        assert!(matches!(
            foreign_body,
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyCreateError::Body(
                    crate::simulation_rigid_body_pool::SimulationRigidBodyPoolError::ForeignHandle
                )
            )
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());

        template.volume_sources[0].volume_type = VolumeType::Aggregate;
        template.volume_sources[0].volume.vtable_offset = VolumeType::Aggregate as u32;
        let no_bbox = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &input,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut numeric,
            &mut packed,
        );
        assert!(matches!(
            no_bbox,
            Err(crate::serialised_assembly_create::SerialisedAssemblyCreateError::NoDeterministicCpuAabb {
                selected_volumes: 1
            })
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());

        template.volume_sources[0].volume_type = VolumeType::Null;
        template.volume_sources[0].volume.vtable_offset = VolumeType::Null as u32;
        let null_volume = try_prepare_serialised_assembly_create(
            &template,
            &pool,
            &input,
            SerialisedAssemblyLiveAssemblyInput {
                group: 0,
                culling_table: template.culling_table(),
            },
            SerialisedAssemblyParamsState::native_default(),
            &mut NeverNumeric,
            &mut packed,
        );
        assert!(matches!(
            null_volume,
            Err(
                crate::serialised_assembly_create::SerialisedAssemblyCreateError::NullVolume {
                    selected_ordinal: 0
                }
            )
        ));
        assert!(packed.transform_calls.is_empty());
        assert!(packed.motion_calls.is_empty());
    }

    fn marked_volume(volume_type: VolumeType, target: u32, marker: u32) -> Volume {
        Volume {
            transform: std::array::from_fn(|index| {
                f32::from_bits(marker.wrapping_add(u32::try_from(index).unwrap()))
            }),
            vtable_offset: volume_type as u32,
            shape_words: [target, marker ^ 0xaaaa_aaaa, marker ^ 0x5555_5555],
            radius: f32::from_bits(marker),
            group_id: marker.wrapping_add(1),
            surface_id: marker.wrapping_add(2),
            flags: 0,
        }
    }

    fn template_part(root_index: usize) -> SerialisedAssemblyTemplatePart {
        SerialisedAssemblyTemplatePart {
            volume_object_id: ObjectId(u32::try_from(root_index + 1).unwrap()),
            root_index,
            inertia_source: {
                let owner = source_plan(1, 1, ObjectId(2), ObjectId(1));
                definition_part_inertia_source(
                    &owner,
                    &dictionary_entry(DEFINITION_OFFSET, 0x30, ASSEMBLY_DEFINITION_TYPE_ID),
                    u32::try_from(PARTS_OFFSET - DEFINITION_OFFSET).unwrap(),
                    0,
                )
                .unwrap()
            },
            inertia: AssemblyPartInertia {
                inverse_tensor: [0.0; 4],
                inverse_mass: 0.0,
                spherical: 0.0,
                max_velocity: 0.0,
                max_omega: 0.0,
                linear_drag: 0.0,
                angular_drag: 0.0,
            },
            inverse_body_transform: [0.0; 16],
            material: AssemblyPartMaterial {
                static_friction: 0.0,
                dynamic_friction: 0.0,
                restitution: 0.0,
            },
            has_inverse_body_transform: 0,
            is_static_body: 0,
        }
    }

    fn empty_clustered_mesh(dictionary_index: u32) -> CollisionMesh {
        CollisionMesh {
            dictionary_index,
            bounding_box_min: [0.0; 3],
            bounding_box_max: [0.0; 3],
            triangles: Vec::new(),
            clustered_mesh: ClusteredMesh {
                bounding_box_min: [0.0; 4],
                bounding_box_max: [0.0; 4],
                aggregate_vtable_offset: 0,
                tag_bit_count: 0,
                volume_count: 0,
                aggregate_padding: 0,
                kd_tree_offset: 0,
                cluster_data_offset: 0,
                vertex_compression_granularity: 0.0,
                cluster_flags: 0,
                group_id_size: 0,
                surface_id_size: 0,
                cluster_count: 0,
                maximum_cluster_count: 0,
                unit_count: 0,
                maximum_unit_count: 0,
                allocation_size: 0,
                default_group_id: 0,
                default_surface_id: 0,
                default_edge_angle: 0,
            },
            object_bytes: Vec::new(),
        }
    }

    #[test]
    fn create_volume_walk_preserves_the_native_two_level_shape_and_disabled_records() {
        let root_sma = marked_volume(VolumeType::Aggregate, 10, 0x1000_0000);
        let root_mesh = marked_volume(VolumeType::Aggregate, 20, 0x2000_0000);
        let direct = marked_volume(VolumeType::Sphere, 0, 0x3000_0000);
        let mesh_child = marked_volume(VolumeType::Aggregate, 20, 0x4000_0000);
        let nested_child = marked_volume(VolumeType::Aggregate, 11, 0x5000_0000);
        let grandchild = marked_volume(VolumeType::Box, 0, 0x6000_0000);
        let skipped_deep_aggregate = marked_volume(VolumeType::Aggregate, 20, 0x7000_0000);
        let arena = CollisionArena {
            id: 1,
            grind_assets: Vec::new(),
            aggregate_roots: vec![
                CollisionRoot {
                    dictionary_index: 1,
                    volume: root_sma,
                    aggregate: Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
                },
                CollisionRoot {
                    dictionary_index: 2,
                    volume: root_mesh,
                    aggregate: Some(CollisionAggregateRef::Mesh(0)),
                },
            ],
            primitive_sets: Vec::new(),
            simple_primitive_sets: vec![
                CollisionSimplePrimitiveSet {
                    dictionary_index: 10,
                    bounding_box_min: [0.0; 3],
                    bounding_box_max: [0.0; 3],
                    volumes: vec![direct, mesh_child, nested_child],
                },
                CollisionSimplePrimitiveSet {
                    dictionary_index: 11,
                    bounding_box_min: [0.0; 3],
                    bounding_box_max: [0.0; 3],
                    volumes: vec![grandchild, skipped_deep_aggregate],
                },
            ],
            meshes: vec![empty_clustered_mesh(20)],
            collision_models: Vec::new(),
            instances: Vec::new(),
        };
        let sources = build_volume_sources(&arena, &[template_part(0), template_part(1)]).unwrap();
        assert_eq!(sources.len(), 4);
        assert_eq!(
            sources
                .iter()
                .map(|source| source.volume.radius.to_bits())
                .collect::<Vec<_>>(),
            [0x3000_0000, 0x4000_0000, 0x6000_0000, 0x2000_0000]
        );
        assert_eq!(
            sources
                .iter()
                .map(|source| (
                    source.part_ordinal,
                    source.rigid_body_index,
                    source.volume_type
                ))
                .collect::<Vec<_>>(),
            [
                (0, 0, VolumeType::Sphere),
                (0, 0, VolumeType::Aggregate),
                (0, 0, VolumeType::Box),
                (1, 1, VolumeType::Aggregate),
            ]
        );
        assert!(sources.iter().all(|source| source.volume.flags == 0));
        assert!(sources.iter().all(|source| {
            source.padding.to_bits() == 0x3d4c_cccd && source.surface_id == source.volume.surface_id
        }));

        let update_sources =
            build_update_volume_sources(&arena, &[template_part(0), template_part(1)]).unwrap();
        assert_eq!(update_sources.len(), 3);
        assert_eq!(
            update_sources
                .iter()
                .map(|source| (source.part_ordinal, source.volume.radius.to_bits()))
                .collect::<Vec<_>>(),
            [(0, 0x3000_0000), (0, 0x4000_0000), (0, 0x6000_0000),]
        );
    }

    #[test]
    fn volume_cap_truncates_emissions_but_not_later_traversal() {
        let mut volumes = (0_u32..27)
            .map(|index| marked_volume(VolumeType::Sphere, 0, index + 1))
            .collect::<Vec<_>>();
        volumes.push(marked_volume(VolumeType::Aggregate, 99, 0xfeed_beef));
        let mut arena = CollisionArena {
            id: 1,
            grind_assets: Vec::new(),
            aggregate_roots: vec![CollisionRoot {
                dictionary_index: 1,
                volume: marked_volume(VolumeType::Aggregate, 10, 0),
                aggregate: Some(CollisionAggregateRef::SimplePrimitiveSet(0)),
            }],
            primitive_sets: Vec::new(),
            simple_primitive_sets: vec![CollisionSimplePrimitiveSet {
                dictionary_index: 10,
                bounding_box_min: [0.0; 3],
                bounding_box_max: [0.0; 3],
                volumes,
            }],
            meshes: Vec::new(),
            collision_models: Vec::new(),
            instances: Vec::new(),
        };
        assert_eq!(
            build_volume_sources(&arena, &[template_part(0)]).unwrap_err(),
            SerialisedAssemblyTemplateError::MissingAggregateTarget {
                dictionary_index: 99,
            }
        );
        arena.simple_primitive_sets[0].volumes.pop();
        let sources = build_volume_sources(&arena, &[template_part(0)]).unwrap();
        assert_eq!(sources.len(), 26);
        assert_eq!(sources[0].volume.radius.to_bits(), 1);
        assert_eq!(sources[25].volume.radius.to_bits(), 26);

        arena.simple_primitive_sets[0].volumes.push(marked_volume(
            VolumeType::Aggregate,
            99,
            0xfeed_beef,
        ));
        assert_eq!(
            build_update_volume_sources(&arena, &[template_part(0)]).unwrap_err(),
            SerialisedAssemblyTemplateError::MissingAggregateTarget {
                dictionary_index: 99,
            }
        );
        arena.simple_primitive_sets[0].volumes.pop();
        let update_sources = build_update_volume_sources(&arena, &[template_part(0)]).unwrap();
        assert_eq!(update_sources.len(), 26);
        assert_eq!(update_sources[0].volume.radius.to_bits(), 1);
        assert_eq!(update_sources[25].volume.radius.to_bits(), 26);
    }
}
