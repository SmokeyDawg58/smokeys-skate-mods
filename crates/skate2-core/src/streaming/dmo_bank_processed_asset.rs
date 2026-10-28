//! Catalog-checked processed arenas for the main DMO model bank.
//!
//! The bank is serialized as a Presentation stream, but the recovered hull
//! callback forwards its completed collection to `Simulation::RegArena` with
//! raw userdata `0x0a`. This module admits only the supplied reference
//! manifest's parsed corpus fingerprint. It does not relax the ordinary
//! Simulation-only collision builder session, register managers, reproduce
//! hull scheduling, or infer native failures.

use crate::{
    catalog_resident_load_factory::CollectionAssetBuilderSessionFactory,
    collection_resident_streamer::CollectionId,
    decoded_arena_asset::{
        DecodedArenaAssetBuilder, DecodedArenaAssetError, WORLD_STREAM_ARENA_ASSET_TYPE,
    },
    game_stream::StreamKind,
    owned_resident_collection::ResidentAssetIdentity,
    resident_collection_load_driver::{CollectionAssetBuildInput, CollectionAssetBuilder},
    serialised_assembly_template::SerialisedAssemblyTemplate,
    simulation_collision_asset::{SimulationCollisionAsset, SimulationCollisionAssetError},
    simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationNormalArenaPlanError},
    world_stream_catalog::{
        AssetRegistrationSnapshot, CatalogError, CollectionRegistrationSnapshot,
        FileRegistrationPhase, WorldStreamCatalog,
    },
};
use skate2_formats::pegasus::ObjectId;
use std::{error::Error, fmt, sync::Arc};

pub const DMO_BANK_COLLECTION_ID: u64 = 0xe878_78ef_d7f5_0339;
pub const DMO_BANK_PARENT_ID: u64 = 0xe171_e1ab_fedb_69ed;
pub const DMO_BANK_STAMP: u64 = 0x0000_0000_94d1_4dc9;
pub const DMO_BANK_ASSET_COUNT: u32 = 125;
pub const DMO_BANK_ASSET_ID_SIGNATURE: u64 = 0x9a86_1845_e82e_e292;
pub const DMO_BANK_ASSET_DESCRIPTOR_SIGNATURE: u64 = 0x20d5_18b8_2728_d8d5;

const DMO_BANK_MANIFEST_STEM: &str = "data\\content\\world\\dmo\\DMO\\DMO_Pres";
const DMO_BANK_COLLECTION_PATH: &str = "data\\content\\world\\dmo\\DMO\\cPres_Global.xsf";
const DMO_BANK_COLLECTION_NAME: &str = "cPres_Global";
const DMO_BANK_ASSET_COUNT_USIZE: usize = 125;
const DMO_BANK_PAYLOAD_OFFSET: u32 = 0x80;
const DMO_BANK_PAYLOAD_SIZE: u32 = 0x2f_3180;
const DMO_BANK_FIRST_ASSET_ID: u64 = 0x91d7_2407_5ae4_4972;
const DMO_BANK_LAST_ASSET_ID: u64 = 0xd309_6f13_cf1c_0aab;
const DMO_BANK_SERIALIZED_BYTES: u64 = 7_117_012;
const DMO_BANK_MAX_SERIALIZED_BYTES: u32 = 1_382_292;
const DMO_BANK_RESOURCE_BYTES: [u64; 5] = [945_452, 0, 6_171_560, 0, 0];
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Stable semantic fingerprint of owner-bound Assembly source templates in
/// one processed DMO arena. This exposes validation metrics, not the private
/// incomplete template itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmoSerialisedAssemblySourceSummary {
    pub assembly_count: usize,
    pub part_count: usize,
    pub joint_count: usize,
    pub drive_count: usize,
    pub joint_count_mismatches: usize,
    pub drive_count_mismatches: usize,
    pub volume_count: usize,
    pub signature: u64,
    pub inertia_source_signature: u64,
    pub update_volume_count: usize,
    pub create_update_selection_mismatches: usize,
    pub update_signature: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DmoSerialisedAssemblyConstraintCounts {
    joints: usize,
    drives: usize,
    joint_mismatches: usize,
    drive_mismatches: usize,
}

fn count_serialised_assembly_constraints(
    counts: impl IntoIterator<Item = (u32, u32, u32, u32)>,
) -> DmoSerialisedAssemblyConstraintCounts {
    counts.into_iter().fold(
        DmoSerialisedAssemblyConstraintCounts::default(),
        |mut totals, (runtime_joints, definition_joints, runtime_drives, definition_drives)| {
            totals.joints += definition_joints as usize;
            totals.drives += definition_drives as usize;
            totals.joint_mismatches += usize::from(runtime_joints != definition_joints);
            totals.drive_mismatches += usize::from(runtime_drives != definition_drives);
            totals
        },
    )
}

/// One fully owned processed DMO-bank arena with no registration effects.
///
/// Reference consumer: `Simulation::RegArena` at `0x828a01e0`.
#[derive(Debug)]
pub struct DmoBankProcessedArenaAsset {
    plan: Arc<SimulationNormalArenaPlan>,
    serialised_assemblies: Vec<SerialisedAssemblyTemplate>,
}

/// Safe-host refusal while selecting one exact local Assembly object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DmoSerialisedAssemblySelectError {
    NonLocal {
        object_id: ObjectId,
        section: u16,
    },
    Missing {
        object_id: ObjectId,
    },
    Duplicate {
        object_id: ObjectId,
        first_ordinal: usize,
        second_ordinal: usize,
    },
}

impl fmt::Display for DmoSerialisedAssemblySelectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonLocal { object_id, section } => write!(
                formatter,
                "DMO Assembly object {:#010x} names nonlocal section {section}",
                object_id.0
            ),
            Self::Missing { object_id } => write!(
                formatter,
                "DMO Assembly object {:#010x} is not retained",
                object_id.0
            ),
            Self::Duplicate {
                object_id,
                first_ordinal,
                second_ordinal,
            } => write!(
                formatter,
                "DMO Assembly object {:#010x} occurs at template ordinals {first_ordinal} and {second_ordinal}",
                object_id.0
            ),
        }
    }
}

impl Error for DmoSerialisedAssemblySelectError {}

impl DmoBankProcessedArenaAsset {
    /// Selects one dictionary-filtered template by exact raw local `ObjectId`.
    pub(crate) fn try_serialised_assembly_by_object_id(
        &self,
        object_id: ObjectId,
    ) -> Result<(usize, &SerialisedAssemblyTemplate), DmoSerialisedAssemblySelectError> {
        let section = object_id.section_index();
        if section != 0 {
            return Err(DmoSerialisedAssemblySelectError::NonLocal { object_id, section });
        }
        let mut found = None;
        for (ordinal, template) in self.serialised_assemblies.iter().enumerate() {
            if template.assembly_object_id() != object_id {
                continue;
            }
            if let Some((first_ordinal, _)) = found {
                return Err(DmoSerialisedAssemblySelectError::Duplicate {
                    object_id,
                    first_ordinal,
                    second_ordinal: ordinal,
                });
            }
            found = Some((ordinal, template));
        }
        found.ok_or(DmoSerialisedAssemblySelectError::Missing { object_id })
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_test(
        plan: Arc<SimulationNormalArenaPlan>,
        serialised_assemblies: Vec<SerialisedAssemblyTemplate>,
    ) -> Self {
        assert!(
            serialised_assemblies
                .iter()
                .all(|template| Arc::ptr_eq(template.owner(), &plan)),
            "test Assembly templates share the exact DMO plan owner"
        );
        Self {
            plan,
            serialised_assemblies,
        }
    }

    #[must_use]
    pub fn plan(&self) -> &SimulationNormalArenaPlan {
        &self.plan
    }

    /// Clones the owning plan authority needed by a later prepared adapter.
    #[must_use]
    pub fn plan_owner(&self) -> Arc<SimulationNormalArenaPlan> {
        Arc::clone(&self.plan)
    }

    /// Number of arena Assembly objects whose complete Part source chain was
    /// resolved before this asset was published.
    #[must_use]
    pub fn serialised_assembly_count(&self) -> usize {
        self.serialised_assemblies.len()
    }

    /// Total definition-ordered Parts retained by the Assembly templates.
    #[must_use]
    pub fn serialised_assembly_part_count(&self) -> usize {
        self.serialised_assemblies
            .iter()
            .map(|template| template.parts().len())
            .sum()
    }

    /// Total Definition-owned Joint entries required by the source templates.
    #[must_use]
    pub fn serialised_assembly_joint_count(&self) -> usize {
        self.serialised_assembly_constraint_counts().joints
    }

    /// Total Definition-owned Drive entries required by the source templates.
    #[must_use]
    pub fn serialised_assembly_drive_count(&self) -> usize {
        self.serialised_assembly_constraint_counts().drives
    }

    /// Runtime/Definition Joint-count disagreements retained by the source.
    #[must_use]
    pub fn serialised_assembly_joint_count_mismatches(&self) -> usize {
        self.serialised_assembly_constraint_counts()
            .joint_mismatches
    }

    /// Runtime/Definition Drive-count disagreements retained by the source.
    #[must_use]
    pub fn serialised_assembly_drive_count_mismatches(&self) -> usize {
        self.serialised_assembly_constraint_counts()
            .drive_mismatches
    }

    fn serialised_assembly_constraint_counts(&self) -> DmoSerialisedAssemblyConstraintCounts {
        count_serialised_assembly_constraints(self.serialised_assemblies.iter().map(|template| {
            (
                template.assembly().joint_count,
                template.definition().joint_count,
                template.assembly().drive_count,
                template.definition().drive_count,
            )
        }))
    }

    /// Number of Create-selected Volume occurrences after the independent
    /// fixed-26 emission cap has been applied per Assembly.
    #[must_use]
    pub fn serialised_assembly_volume_count(&self) -> usize {
        self.serialised_assemblies
            .iter()
            .map(|template| template.volume_sources().len())
            .sum()
    }

    /// Number of Volume occurrences visited by the distinct Update walk.
    #[must_use]
    pub fn serialised_assembly_update_volume_count(&self) -> usize {
        self.serialised_assemblies
            .iter()
            .map(|template| template.update_volume_sources().len())
            .sum()
    }

    /// Positional Create/Update occurrence differences in this arena.
    #[must_use]
    pub fn serialised_assembly_create_update_selection_mismatches(&self) -> usize {
        self.serialised_assemblies
            .iter()
            .map(|template| {
                let create = template.volume_sources();
                let update = template.update_volume_sources();
                let positional = create
                    .iter()
                    .zip(update)
                    .filter(|(left, right)| {
                        left.rigid_body_index != right.part_ordinal || left.volume != right.volume
                    })
                    .count();
                positional + create.len().abs_diff(update.len())
            })
            .sum()
    }

    /// Fingerprints the resolved Assembly/Definition Part source chain and
    /// retained header words in dictionary and Part order using raw floats.
    ///
    /// # Panics
    ///
    /// Panics only if a previously validated RW4 resource index cannot be
    /// represented by the native 32-bit signature word.
    #[must_use]
    #[allow(clippy::too_many_lines)] // Keep the semantic signature word order auditable.
    pub fn serialised_assembly_source_summary(&self) -> DmoSerialisedAssemblySourceSummary {
        let mut signature = FNV_OFFSET;
        hash_bytes(
            &mut signature,
            b"SKATE2/DMO/SERIALISED_ASSEMBLY_SOURCE\0V2\0",
        );
        hash_u64(
            &mut signature,
            self.plan.static_plan().asset().decoded.asset_id,
        );
        let mut inertia_source_signature = FNV_OFFSET;
        hash_bytes(
            &mut inertia_source_signature,
            b"SKATE2/DMO/SERIALISED_ASSEMBLY_INERTIA_SOURCE\0V1\0",
        );
        hash_u64(
            &mut inertia_source_signature,
            self.plan.static_plan().asset().decoded.asset_id,
        );
        for template in &self.serialised_assemblies {
            debug_assert!(Arc::ptr_eq(template.owner(), &self.plan));
            hash_u32(&mut signature, template.assembly_object_id().0);
            hash_u32(&mut signature, template.definition_object_id().0);
            hash_u32(
                &mut inertia_source_signature,
                template.assembly_object_id().0,
            );
            hash_u32(
                &mut inertia_source_signature,
                template.definition_object_id().0,
            );
            let assembly = template.assembly();
            for word in [
                assembly.culling_table.map_or(0, |id| id.0),
                assembly.definition.0,
                assembly.part_list_offset,
                assembly.joint_list_offset,
                assembly.drive_list_offset,
                assembly.world_index,
                assembly.group_id,
                assembly.part_count,
                assembly.joint_count,
                assembly.drive_count,
                assembly.user_data,
                u32::from(assembly.flags),
                u32::from(assembly.user_flags),
            ] {
                hash_u32(&mut signature, word);
            }
            let definition = template.definition();
            for word in [
                definition.parts_offset,
                definition.part_count,
                definition.joint_count,
                definition.joint_limits_offset,
                definition.joint_frames_offset,
                definition.joint_parts_offset,
                definition.drive_count,
                definition.drive_dynamics_offset,
                definition.drive_frames_offset,
                definition.drive_parts_offset,
                definition.culling_table.map_or(0, |id| id.0),
                definition.hashed_part_names_offset,
            ] {
                hash_u32(&mut signature, word);
            }
            for (part_ordinal, part) in template.parts().iter().enumerate() {
                debug_assert!(Arc::ptr_eq(part.inertia_source.owner(), &self.plan));
                hash_u32(
                    &mut inertia_source_signature,
                    u32::try_from(part.inertia_source.resource_index())
                        .expect("validated arena resource index must fit u32"),
                );
                hash_u32(
                    &mut inertia_source_signature,
                    part.inertia_source.resource_offset(),
                );
                hash_bytes(&mut inertia_source_signature, part.inertia_source.bytes());
                hash_u32(&mut signature, part.volume_object_id.0);
                for value in part.inertia.inverse_tensor {
                    hash_u32(&mut signature, value.to_bits());
                }
                for value in [
                    part.inertia.inverse_mass,
                    part.inertia.spherical,
                    part.inertia.max_velocity,
                    part.inertia.max_omega,
                    part.inertia.linear_drag,
                    part.inertia.angular_drag,
                ] {
                    hash_u32(&mut signature, value.to_bits());
                }
                for value in part.inverse_body_transform {
                    hash_u32(&mut signature, value.to_bits());
                }
                for value in [
                    part.material.static_friction,
                    part.material.dynamic_friction,
                    part.material.restitution,
                ] {
                    hash_u32(&mut signature, value.to_bits());
                }
                hash_u32(&mut signature, part.has_inverse_body_transform);
                hash_u32(&mut signature, part.is_static_body);
                let volume = template.root_volume(part_ordinal);
                for value in volume.transform {
                    hash_u32(&mut signature, value.to_bits());
                }
                hash_u32(&mut signature, volume.vtable_offset);
                for word in volume.shape_words {
                    hash_u32(&mut signature, word);
                }
                hash_u32(&mut signature, volume.radius.to_bits());
                hash_u32(&mut signature, volume.group_id);
                hash_u32(&mut signature, volume.surface_id);
                hash_u32(&mut signature, volume.flags);
            }
            for source in template.volume_sources() {
                debug_assert_eq!(usize::from(source.rigid_body_index), source.part_ordinal);
                hash_u32(&mut signature, u32::from(source.rigid_body_index));
                hash_u32(&mut signature, source.volume_type as u32);
                for value in source.volume.transform {
                    hash_u32(&mut signature, value.to_bits());
                }
                hash_u32(&mut signature, source.volume.vtable_offset);
                for word in source.volume.shape_words {
                    hash_u32(&mut signature, word);
                }
                hash_u32(&mut signature, source.volume.radius.to_bits());
                hash_u32(&mut signature, source.volume.group_id);
                hash_u32(&mut signature, source.volume.surface_id);
                hash_u32(&mut signature, source.volume.flags);
                hash_u32(&mut signature, source.padding.to_bits());
                hash_u32(&mut signature, source.surface_id);
            }
        }
        let mut update_signature = FNV_OFFSET;
        hash_bytes(
            &mut update_signature,
            b"SKATE2/DMO/SERIALISED_ASSEMBLY_UPDATE_SELECTION\0V1\0",
        );
        hash_u64(
            &mut update_signature,
            self.plan.static_plan().asset().decoded.asset_id,
        );
        for template in &self.serialised_assemblies {
            hash_u32(&mut update_signature, template.assembly_object_id().0);
            hash_u32(&mut update_signature, template.definition_object_id().0);
            for source in template.update_volume_sources() {
                hash_u32(&mut update_signature, u32::from(source.part_ordinal));
                for value in source.volume.transform {
                    hash_u32(&mut update_signature, value.to_bits());
                }
                hash_u32(&mut update_signature, source.volume.vtable_offset);
                for word in source.volume.shape_words {
                    hash_u32(&mut update_signature, word);
                }
                hash_u32(&mut update_signature, source.volume.radius.to_bits());
                hash_u32(&mut update_signature, source.volume.group_id);
                hash_u32(&mut update_signature, source.volume.surface_id);
                hash_u32(&mut update_signature, source.volume.flags);
            }
        }
        DmoSerialisedAssemblySourceSummary {
            assembly_count: self.serialised_assembly_count(),
            part_count: self.serialised_assembly_part_count(),
            joint_count: self.serialised_assembly_joint_count(),
            drive_count: self.serialised_assembly_drive_count(),
            joint_count_mismatches: self.serialised_assembly_joint_count_mismatches(),
            drive_count_mismatches: self.serialised_assembly_drive_count_mismatches(),
            volume_count: self.serialised_assembly_volume_count(),
            signature,
            inertia_source_signature,
            update_volume_count: self.serialised_assembly_update_volume_count(),
            create_update_selection_mismatches: self
                .serialised_assembly_create_update_selection_mismatches(),
            update_signature,
        }
    }
}

impl ResidentAssetIdentity for DmoBankProcessedArenaAsset {
    fn resident_asset_id(&self) -> CollectionId {
        CollectionId(self.plan.static_plan().asset().decoded.asset_id)
    }
}

/// Per-collection builder created only after DMO-bank fingerprint checks.
///
/// Reference producer boundary: `cProcessArenaAsset::ProcessArena` at
/// `0x82d5fee0`.
#[derive(Debug)]
pub struct DmoBankProcessedArenaAssetBuilder {
    decoder: DecodedArenaAssetBuilder,
    expected_toc: Arc<skate2_formats::stream::AssetToc>,
    next_asset: usize,
}

impl DmoBankProcessedArenaAssetBuilder {
    fn validate_next_metadata(
        &self,
        asset_id: u64,
        asset_info: &AssetRegistrationSnapshot,
    ) -> Result<usize, DmoBankProcessedArenaBuildError> {
        let index = self.next_asset;
        let expected = self
            .expected_toc
            .assets
            .get(index)
            .ok_or(DmoBankProcessedArenaBuildError::UnexpectedAsset { index })?;
        if asset_id != expected.id {
            return Err(DmoBankProcessedArenaBuildError::AssetOrderMismatch {
                index,
                expected: expected.id,
                actual: asset_id,
            });
        }
        if asset_info.entry != *expected {
            return Err(DmoBankProcessedArenaBuildError::AssetMetadataMismatch {
                index,
                id: asset_id,
            });
        }
        Ok(index)
    }
}

impl CollectionAssetBuilder<AssetRegistrationSnapshot> for DmoBankProcessedArenaAssetBuilder {
    type Asset = DmoBankProcessedArenaAsset;
    type Error = DmoBankProcessedArenaBuildError;

    fn build(
        &mut self,
        input: CollectionAssetBuildInput<'_, AssetRegistrationSnapshot>,
    ) -> Result<Option<Self::Asset>, Self::Error> {
        let index = self.validate_next_metadata(input.asset_id, input.asset_info)?;

        let decoded = self
            .decoder
            .build(input)
            .map_err(DmoBankProcessedArenaBuildError::Decode)?
            .ok_or(DmoBankProcessedArenaBuildError::UnexpectedNullAsset { index })?;
        let collision = SimulationCollisionAsset::from_decoded(decoded)
            .map_err(DmoBankProcessedArenaBuildError::Projection)?;
        let plan = Arc::new(
            SimulationNormalArenaPlan::from_collision_asset(collision)
                .map_err(DmoBankProcessedArenaBuildError::Plan)?,
        );
        let serialised_assemblies =
            SerialisedAssemblyTemplate::all_in_plan(&plan).map_err(|source| {
                DmoBankProcessedArenaBuildError::SerialisedAssembly {
                    reason: source.to_string(),
                }
            })?;
        let asset = DmoBankProcessedArenaAsset {
            plan,
            serialised_assemblies,
        };
        self.next_asset = index + 1;
        Ok(Some(asset))
    }
}

/// Cloneable session factory restricted to the main DMO bank fingerprint.
///
/// Reference load chain: `cDMOBankManager::LoadMainDMOBank` at `0x82e0e628`
/// through `cHullLoader::LoadHull` at `0x82eeedc8`.
#[derive(Clone, Debug)]
pub struct DmoBankProcessedArenaAssetBuilderSessions {
    catalog: WorldStreamCatalog,
}

impl DmoBankProcessedArenaAssetBuilderSessions {
    #[must_use]
    pub const fn new(catalog: WorldStreamCatalog) -> Self {
        Self { catalog }
    }

    #[must_use]
    pub const fn catalog(&self) -> &WorldStreamCatalog {
        &self.catalog
    }
}

impl CollectionAssetBuilderSessionFactory for DmoBankProcessedArenaAssetBuilderSessions {
    type Asset = DmoBankProcessedArenaAsset;
    type Builder = DmoBankProcessedArenaAssetBuilder;
    type Error = DmoBankProcessedArenaSessionError;

    fn begin_session(
        &mut self,
        collection: &CollectionRegistrationSnapshot,
    ) -> Result<Self::Builder, Self::Error> {
        let registered = self
            .catalog
            .collection(collection.id)
            .map_err(DmoBankProcessedArenaSessionError::Catalog)?
            .ok_or(DmoBankProcessedArenaSessionError::UnknownCollection(
                collection.id,
            ))?;
        if &registered != collection {
            return Err(
                DmoBankProcessedArenaSessionError::CollectionSnapshotMismatch { id: collection.id },
            );
        }
        let file = self
            .catalog
            .file(&registered.owner)
            .map_err(DmoBankProcessedArenaSessionError::Catalog)?
            .ok_or(DmoBankProcessedArenaSessionError::UnknownFileOwner)?;
        if file.kind != StreamKind::Presentation {
            return Err(DmoBankProcessedArenaSessionError::WrongStreamKind(
                file.kind,
            ));
        }
        if file.paths.manifest_stem != DMO_BANK_MANIFEST_STEM {
            return Err(DmoBankProcessedArenaSessionError::WrongManifestPath);
        }
        if file.phase != FileRegistrationPhase::Registered
            || !file.collections_active
            || file.collection_map.is_none()
        {
            return Err(DmoBankProcessedArenaSessionError::InactiveCollectionMap);
        }
        if !file.assets_active || file.asset_toc.is_none() {
            return Err(DmoBankProcessedArenaSessionError::InactiveAssetToc);
        }
        let map = file
            .collection_map
            .as_deref()
            .expect("active collection map was checked");
        let toc = file
            .asset_toc
            .as_ref()
            .expect("active asset TOC was checked");

        validate_manifest(collection, &file.collection_ids, &file.asset_ids, map, toc)?;
        for local in &toc.assets {
            let global = self
                .catalog
                .asset(local.id)
                .map_err(DmoBankProcessedArenaSessionError::Catalog)?
                .ok_or(DmoBankProcessedArenaSessionError::UnknownGlobalAsset(
                    local.id,
                ))?;
            if global.reference_count <= 0 || global.entry != *local {
                return Err(
                    DmoBankProcessedArenaSessionError::GlobalAssetMetadataMismatch(local.id),
                );
            }
        }

        Ok(DmoBankProcessedArenaAssetBuilder {
            decoder: DecodedArenaAssetBuilder,
            expected_toc: Arc::clone(toc),
            next_asset: 0,
        })
    }
}

fn validate_manifest(
    collection: &CollectionRegistrationSnapshot,
    collection_ids: &[u64],
    asset_ids: &[u64],
    map: &skate2_formats::stream::CollectionMap,
    toc: &skate2_formats::stream::AssetToc,
) -> Result<(), DmoBankProcessedArenaSessionError> {
    if map.version != 3
        || map.stamp != DMO_BANK_STAMP
        || map.stream_format != 2
        || map.name_table_offset != 0xdead_c0de
        || map.collections.len() != 1
        || collection_ids != [DMO_BANK_COLLECTION_ID]
    {
        return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
            "collection map",
        ));
    }
    let entry = &map.collections[0];
    if collection.id != DMO_BANK_COLLECTION_ID
        || collection.source_index != 0
        || collection.path.as_ref() != DMO_BANK_COLLECTION_PATH
        || collection.offset != u64::from(DMO_BANK_PAYLOAD_OFFSET)
        || collection.size != DMO_BANK_PAYLOAD_SIZE
        || collection.asset_count != DMO_BANK_ASSET_COUNT
        || collection.stream_format != 2
        || collection.parent_id != DMO_BANK_PARENT_ID
        || entry.id != collection.id
        || entry.parent_id != collection.parent_id
        || u64::from(entry.payload_offset) != collection.offset
        || entry.payload_size != collection.size
        || entry.asset_count != collection.asset_count
        || entry.filename != DMO_BANK_COLLECTION_NAME
        || entry.name_offset != u32::MAX
        || entry.alignment_pad != 0
    {
        return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
            "collection",
        ));
    }
    if toc.version != 3
        || toc.stamp != DMO_BANK_STAMP
        || toc.name_table_offset != 0
        || toc.assets.len() != DMO_BANK_ASSET_COUNT_USIZE
        || asset_ids.len() != toc.assets.len()
        || asset_ids
            .iter()
            .zip(&toc.assets)
            .any(|(id, asset)| *id != asset.id)
        || toc.assets.first().map(|asset| asset.id) != Some(DMO_BANK_FIRST_ASSET_ID)
        || toc.assets.last().map(|asset| asset.id) != Some(DMO_BANK_LAST_ASSET_ID)
    {
        return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
            "asset TOC",
        ));
    }
    validate_asset_corpus(toc)
}

fn validate_asset_corpus(
    toc: &skate2_formats::stream::AssetToc,
) -> Result<(), DmoBankProcessedArenaSessionError> {
    let mut id_signature = FNV_OFFSET;
    let mut descriptor_signature = FNV_OFFSET;
    let mut serialized_bytes = 0_u64;
    let mut maximum_serialized = 0_u32;
    let mut resource_bytes = [0_u64; 5];
    for (index, asset) in toc.assets.iter().enumerate() {
        if toc.assets[..index]
            .iter()
            .any(|previous| previous.id == asset.id)
            || asset.asset_type != WORLD_STREAM_ARENA_ASSET_TYPE
        {
            return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
                "asset entry",
            ));
        }
        for byte in asset.id.to_be_bytes() {
            id_signature ^= u64::from(byte);
            id_signature = id_signature.wrapping_mul(FNV_PRIME);
        }
        hash_u64(&mut descriptor_signature, asset.id);
        hash_u32(&mut descriptor_signature, asset.asset_type);
        hash_u32(&mut descriptor_signature, asset.serialized_size);
        for resource in asset.resources {
            hash_u32(&mut descriptor_signature, resource.size);
            hash_u32(&mut descriptor_signature, resource.alignment);
        }
        hash_u32(&mut descriptor_signature, asset.name_offset);
        hash_u32(&mut descriptor_signature, asset.unused);
        let resource_sum = asset
            .resources
            .iter()
            .try_fold(0_u32, |sum, resource| sum.checked_add(resource.size));
        if resource_sum != Some(asset.serialized_size) {
            return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
                "asset resource sizes",
            ));
        }
        serialized_bytes = serialized_bytes
            .checked_add(u64::from(asset.serialized_size))
            .ok_or(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
                "serialized-byte total",
            ))?;
        maximum_serialized = maximum_serialized.max(asset.serialized_size);
        for (total, resource) in resource_bytes.iter_mut().zip(asset.resources) {
            *total = total.checked_add(u64::from(resource.size)).ok_or(
                DmoBankProcessedArenaSessionError::ManifestIdentityMismatch("resource-byte total"),
            )?;
        }
    }
    if id_signature != DMO_BANK_ASSET_ID_SIGNATURE
        || descriptor_signature != DMO_BANK_ASSET_DESCRIPTOR_SIGNATURE
        || serialized_bytes != DMO_BANK_SERIALIZED_BYTES
        || maximum_serialized != DMO_BANK_MAX_SERIALIZED_BYTES
        || resource_bytes != DMO_BANK_RESOURCE_BYTES
    {
        return Err(DmoBankProcessedArenaSessionError::ManifestIdentityMismatch(
            "asset corpus",
        ));
    }
    Ok(())
}

fn hash_u32(signature: &mut u64, value: u32) {
    for byte in value.to_be_bytes() {
        *signature ^= u64::from(byte);
        *signature = signature.wrapping_mul(FNV_PRIME);
    }
}

fn hash_bytes(signature: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *signature ^= u64::from(*byte);
        *signature = signature.wrapping_mul(FNV_PRIME);
    }
}

fn hash_u64(signature: &mut u64, value: u64) {
    for byte in value.to_be_bytes() {
        *signature ^= u64::from(byte);
        *signature = signature.wrapping_mul(FNV_PRIME);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DmoBankProcessedArenaSessionError {
    Catalog(CatalogError),
    UnknownCollection(u64),
    CollectionSnapshotMismatch { id: u64 },
    UnknownFileOwner,
    WrongStreamKind(StreamKind),
    WrongManifestPath,
    InactiveCollectionMap,
    InactiveAssetToc,
    ManifestIdentityMismatch(&'static str),
    UnknownGlobalAsset(u64),
    GlobalAssetMetadataMismatch(u64),
}

impl fmt::Display for DmoBankProcessedArenaSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(source) => write!(formatter, "stream catalog lookup failed: {source}"),
            Self::UnknownCollection(id) => {
                write!(
                    formatter,
                    "DMO collection {id:#018x} is absent from the catalog"
                )
            }
            Self::CollectionSnapshotMismatch { id } => write!(
                formatter,
                "DMO collection {id:#018x} snapshot differs from the catalog"
            ),
            Self::UnknownFileOwner => {
                formatter.write_str("DMO collection owner is absent from the catalog")
            }
            Self::WrongStreamKind(kind) => {
                write!(
                    formatter,
                    "DMO bank requires Presentation, observed {kind:?}"
                )
            }
            Self::WrongManifestPath => {
                formatter.write_str("Presentation owner is not the main DMO manifest")
            }
            Self::InactiveCollectionMap => formatter.write_str("DMO collection map is not active"),
            Self::InactiveAssetToc => formatter.write_str("DMO asset TOC is not active"),
            Self::ManifestIdentityMismatch(field) => {
                write!(
                    formatter,
                    "DMO bank {field} differs from the reference corpus"
                )
            }
            Self::UnknownGlobalAsset(id) => {
                write!(
                    formatter,
                    "DMO asset {id:#018x} is absent from the global catalog"
                )
            }
            Self::GlobalAssetMetadataMismatch(id) => write!(
                formatter,
                "DMO asset {id:#018x} global metadata differs from its local XST entry"
            ),
        }
    }
}

impl Error for DmoBankProcessedArenaSessionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Catalog(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DmoBankProcessedArenaBuildError {
    UnexpectedAsset {
        index: usize,
    },
    AssetOrderMismatch {
        index: usize,
        expected: u64,
        actual: u64,
    },
    AssetMetadataMismatch {
        index: usize,
        id: u64,
    },
    UnexpectedNullAsset {
        index: usize,
    },
    Decode(DecodedArenaAssetError),
    Projection(SimulationCollisionAssetError),
    Plan(SimulationNormalArenaPlanError),
    SerialisedAssembly {
        reason: String,
    },
}

impl fmt::Display for DmoBankProcessedArenaBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedAsset { index } => {
                write!(formatter, "DMO stream produced asset after index {index}")
            }
            Self::AssetOrderMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "DMO asset {index} is {actual:#018x}, expected {expected:#018x}"
            ),
            Self::AssetMetadataMismatch { index, id } => write!(
                formatter,
                "DMO asset {index}/{id:#018x} metadata differs from its retained XST entry"
            ),
            Self::UnexpectedNullAsset { index } => {
                write!(
                    formatter,
                    "DMO asset builder returned null at index {index}"
                )
            }
            Self::Decode(source) => write!(formatter, "DMO arena decoding failed: {source}"),
            Self::Projection(source) => {
                write!(formatter, "DMO collision projection failed: {source}")
            }
            Self::Plan(source) => write!(formatter, "DMO normal plan failed: {source}"),
            Self::SerialisedAssembly { reason } => {
                write!(formatter, "DMO SerialisedAssembly source failed: {reason}")
            }
        }
    }
}

impl Error for DmoBankProcessedArenaBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            Self::Projection(source) => Some(source),
            Self::Plan(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::CollectionStreamFormat, game_stream::StreamFile,
        mounted_collection_producer::MountedCollectionBlockView,
        mounted_stream_backend::StreamRegistrationSink,
        resident_collection_load_driver::CollectionAssetBuildMode,
    };
    use skate2_formats::stream::{
        AssetMapEntry, AssetToc, BaseResourceDescriptor, CollectionMap, CollectionMapEntry,
    };

    type TestFile = StreamFile<(), CatalogError>;

    fn entry(id: u64) -> AssetMapEntry {
        AssetMapEntry {
            id,
            asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
            serialized_size: 0,
            resources: [BaseResourceDescriptor {
                size: 0,
                alignment: 0,
            }; 5],
            name_offset: 0,
            unused: 0,
        }
    }

    fn arena_resource() -> Vec<u8> {
        let mut bytes = vec![0_u8; 0x200];
        bytes[..12].copy_from_slice(b"\x89RW4xb2\0\r\n\x1a\n");
        bytes[12] = 1;
        bytes[13] = 32;
        bytes[14] = 4;
        bytes[16..20].copy_from_slice(b"454\0");
        bytes[20..24].copy_from_slice(b"000\0");
        bytes[28..32].copy_from_slice(&0x8646_a1f7_u32.to_be_bytes());
        bytes[32..36].copy_from_slice(&1_u32.to_be_bytes());
        bytes[36..40].copy_from_slice(&1_u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&16_u32.to_be_bytes());
        bytes[48..52].copy_from_slice(&0x1e8_u32.to_be_bytes());
        bytes[68..72].copy_from_slice(&0x200_u32.to_be_bytes());
        bytes[72..76].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1e8..0x1ec].copy_from_slice(&0x100_u32.to_be_bytes());
        bytes[0x1f0..0x1f4].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1f4..0x1f8].copy_from_slice(&16_u32.to_be_bytes());
        bytes[0x1f8..0x1fc].copy_from_slice(&0x10_u32.to_be_bytes());
        bytes[0x1fc..0x200].copy_from_slice(&0x00eb_0008_u32.to_be_bytes());
        bytes
    }

    #[test]
    fn constraint_census_keeps_joint_drive_totals_and_mismatches_distinct() {
        let counts = count_serialised_assembly_constraints([(3, 2, 5, 5), (7, 7, 11, 13)]);

        assert_eq!(
            counts,
            DmoSerialisedAssemblyConstraintCounts {
                joints: 9,
                drives: 18,
                joint_mismatches: 1,
                drive_mismatches: 1,
            }
        );
    }

    fn envelope(resource: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0_u8; 80];
        let size = u32::try_from(resource.len()).unwrap();
        bytes[..4].copy_from_slice(&size.to_be_bytes());
        bytes[4..8].copy_from_slice(&size.to_be_bytes());
        bytes.extend_from_slice(resource);
        bytes
    }

    fn registered_collection(
        base: &str,
        basename: &str,
        kind: StreamKind,
    ) -> (WorldStreamCatalog, CollectionRegistrationSnapshot) {
        let file = TestFile::load(base, basename, kind).unwrap();
        let map = CollectionMap {
            version: 3,
            stamp: DMO_BANK_STAMP,
            stream_format: 2,
            name_table_offset: 0xdead_c0de,
            collections: vec![CollectionMapEntry {
                id: DMO_BANK_COLLECTION_ID,
                parent_id: DMO_BANK_PARENT_ID,
                payload_offset: DMO_BANK_PAYLOAD_OFFSET,
                payload_size: DMO_BANK_PAYLOAD_SIZE,
                asset_count: DMO_BANK_ASSET_COUNT,
                filename: DMO_BANK_COLLECTION_NAME.to_owned(),
                name_offset: u32::MAX,
                alignment_pad: 0,
            }],
        };
        let toc = AssetToc {
            version: 3,
            stamp: DMO_BANK_STAMP,
            name_table_offset: 0,
            assets: vec![entry(1)],
        };
        let mut catalog = WorldStreamCatalog::new();
        catalog
            .collection_map_loaded(file.registration_context(), &map)
            .unwrap();
        catalog
            .asset_toc_loaded(file.registration_context(), &toc)
            .unwrap();
        let collection = catalog.collection(DMO_BANK_COLLECTION_ID).unwrap().unwrap();
        (catalog, collection)
    }

    #[test]
    fn session_rejects_snapshot_tampering_before_owner_policy() {
        let (catalog, collection) =
            registered_collection("data\\content\\world\\dmo", "DMO", StreamKind::Simulation);
        let mut tampered = collection.clone();
        tampered.offset += 1;
        let mut sessions = DmoBankProcessedArenaAssetBuilderSessions::new(catalog);
        assert_eq!(
            sessions.begin_session(&tampered).unwrap_err(),
            DmoBankProcessedArenaSessionError::CollectionSnapshotMismatch {
                id: DMO_BANK_COLLECTION_ID
            }
        );
        assert_eq!(
            sessions.begin_session(&collection).unwrap_err(),
            DmoBankProcessedArenaSessionError::WrongStreamKind(StreamKind::Simulation)
        );
    }

    #[test]
    fn session_rejects_arbitrary_presentation_before_corpus_shape() {
        let (catalog, collection) =
            registered_collection("world", "ordinary", StreamKind::Presentation);
        let mut sessions = DmoBankProcessedArenaAssetBuilderSessions::new(catalog);
        assert_eq!(
            sessions.begin_session(&collection).unwrap_err(),
            DmoBankProcessedArenaSessionError::WrongManifestPath
        );
    }

    #[test]
    fn builder_checks_local_order_and_metadata_before_decoding() {
        let expected = entry(7);
        let builder = DmoBankProcessedArenaAssetBuilder {
            decoder: DecodedArenaAssetBuilder,
            expected_toc: Arc::new(AssetToc {
                version: 3,
                stamp: DMO_BANK_STAMP,
                name_table_offset: 0,
                assets: vec![expected.clone()],
            }),
            next_asset: 0,
        };
        let actual = AssetRegistrationSnapshot {
            first_owner: TestFile::load("world", "owner", StreamKind::Presentation)
                .unwrap()
                .registration_id()
                .clone(),
            entry: expected.clone(),
            reference_count: 1,
        };
        assert_eq!(
            builder.validate_next_metadata(8, &actual).unwrap_err(),
            DmoBankProcessedArenaBuildError::AssetOrderMismatch {
                index: 0,
                expected: 7,
                actual: 8,
            }
        );
        let changed = AssetRegistrationSnapshot {
            entry: AssetMapEntry {
                unused: 1,
                ..expected
            },
            ..actual.clone()
        };
        assert_eq!(
            builder.validate_next_metadata(7, &changed).unwrap_err(),
            DmoBankProcessedArenaBuildError::AssetMetadataMismatch { index: 0, id: 7 }
        );
        assert_eq!(builder.validate_next_metadata(7, &actual), Ok(0));
        assert_eq!(builder.next_asset, 0);
    }

    #[test]
    fn builder_advances_only_after_the_complete_owned_plan_exists() {
        let resource = arena_resource();
        let payload = envelope(&resource);
        let mut expected = entry(7);
        expected.serialized_size = u32::try_from(resource.len()).unwrap();
        expected.resources[0].size = expected.serialized_size;
        let actual = AssetRegistrationSnapshot {
            first_owner: TestFile::load("world", "owner", StreamKind::Presentation)
                .unwrap()
                .registration_id()
                .clone(),
            entry: expected.clone(),
            reference_count: 1,
        };
        let mut builder = DmoBankProcessedArenaAssetBuilder {
            decoder: DecodedArenaAssetBuilder,
            expected_toc: Arc::new(AssetToc {
                version: 3,
                stamp: DMO_BANK_STAMP,
                name_table_offset: 0,
                assets: vec![expected],
            }),
            next_asset: 0,
        };
        let block = MountedCollectionBlockView::from_test_bytes(payload);
        let stored_size = u32::try_from(block.bytes().len()).unwrap();

        let failure = builder
            .build(CollectionAssetBuildInput {
                asset_id: 7,
                asset_info: &actual,
                block: &block,
                payload: block.bytes(),
                stored_size,
                raw_format: CollectionStreamFormat(1),
                mode: CollectionAssetBuildMode::Compressed,
            })
            .unwrap_err();
        assert!(matches!(
            failure,
            DmoBankProcessedArenaBuildError::Decode(_)
        ));
        assert_eq!(builder.next_asset, 0);

        let asset = builder
            .build(CollectionAssetBuildInput {
                asset_id: 7,
                asset_info: &actual,
                block: &block,
                payload: block.bytes(),
                stored_size,
                raw_format: CollectionStreamFormat(2),
                mode: CollectionAssetBuildMode::CompressedArena,
            })
            .unwrap()
            .unwrap();
        assert_eq!(
            asset.plan().static_plan().asset().decoded.asset_id,
            actual.entry.id
        );
        let cloned_plan_owner = asset.plan_owner();
        assert!(Arc::ptr_eq(&asset.plan, &cloned_plan_owner));
        assert_eq!(builder.next_asset, 1);

        assert_eq!(
            builder
                .build(CollectionAssetBuildInput {
                    asset_id: 7,
                    asset_info: &actual,
                    block: &block,
                    payload: block.bytes(),
                    stored_size,
                    raw_format: CollectionStreamFormat(2),
                    mode: CollectionAssetBuildMode::CompressedArena,
                })
                .unwrap_err(),
            DmoBankProcessedArenaBuildError::UnexpectedAsset { index: 1 }
        );
        assert_eq!(builder.next_asset, 1);
    }
}
