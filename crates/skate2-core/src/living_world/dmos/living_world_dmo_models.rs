//! `Sk8::LivingWorld::ModelMan` DMO models over the main DMO bank.
//!
//! `Simulation::RegArena` 0x828a01e0 sends every kind-11 instance of an arena
//! registered with userdata bit 3 to `ModelMan::AddDMOModel` 0x82ce9c60, which
//! keys `LWDynamicObjectModel::Create` 0x82d39f90 by
//! `GetSimModelIDFromInstanceID` 0x82d2ee20. Create runs `FindCharacteristic`
//! 0x82d29f58, `CreateFromMesh` 0x82d337b0 (pipe HiLOD: `ApplyAttribulator
//! Offsets` 0x82d31e28 edits the bank Definition's Part 0 in place) and
//! `InitializeHotpoints` 0x82d38ec0. The last builds only pedestrian waypoint
//! groups and the hand hotpoint. Both are retained on the same model; live
//! Props consume the groups through the existing WaypointGroup.SetData owner.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    dmo_bank_processed_asset::{DmoBankProcessedArenaAsset, DmoSerialisedAssemblySelectError},
    grind_data_runtime::{GrindDataSourceError, GrindSplineSource},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    serialised_assembly_template::{
        SerialisedAssemblyCullingTableSource, SerialisedAssemblyLivingWorldPart,
        SerialisedAssemblyTemplate,
    },
    simulation_normal_arena_plan::{SimulationNormalOperation, SimulationRecordSourceKind},
};
use skate2_formats::{
    collision::{AssemblyPartInertia, AssemblyPartMaterial, BitTable},
    pegasus::{C_MODEL_DATA_TYPE_ID, CollisionModelData, EmbeddedData, GrabData, ObjectId},
    rw4::ArenaError,
};
use std::{collections::BTreeMap, fmt, sync::Arc};

/// `assman::kNullAssetID`: dynamic initializers 0x82f5e640/0x82f8cf50 store
/// `RwHash64String("Null Asset ID", RWHASH64_INIT)`.
pub(crate) const NULL_ASSET_ID: u64 = 0x0167_8c48_6213_9059;
/// `assman::kInvalidAssetID` 0x82f760f8: `RwHash64String("InvalidStringHash")`.
pub(crate) const INVALID_ASSET_ID: u64 = 0xe171_e1ab_fedb_69ed;
/// `livingworld_dynamicobject_characteristics`.
pub(crate) const CHARACTERISTICS_CLASS: u64 = 0xccb7_c068_68eb_1c87;
/// `livingworld_dynamicobject_priority`.
pub(crate) const PRIORITY_CLASS: u64 = 0x7d8f_c9a0_d6e8_0945;
/// Attrib `default` collection key.
pub(crate) const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
/// `pegasus::tAttribPair::kInvalidKey`.
const INVALID_KEY: u64 = u64::MAX;
/// Characteristics node read by `GeneratePriorityIndexForLocator` 0x82d34be8
/// and `FillPriorityIndex`; type `ClassRefSpec_livingworld_dynamicobject_priority`.
pub(crate) const PRIORITY_KEY: u64 = 0x1a3f_b63d_fdae_0c4f;
const PRIORITY_REF_SPEC_TYPE: u64 = 0xc692_2e30_90d2_2e50;
/// Characteristics nodes read by `ApplyAttribulatorOffsets`.
const MAX_VELOCITY_KEY: u64 = 0x4890_392c_9182_9954;
const MAX_OMEGA_KEY: u64 = 0xbaa0_1e2b_a123_7455;
const INERTIA_EXTENT_SCALE_KEY: u64 = 0xf4d1_c84c_36a8_54ac;
/// `__real_3eaaaaab`, SetInertiaExtents 0x82d3023c.
const ONE_THIRD: f32 = f32::from_bits(0x3eaa_aaab);
/// `__real_447a0000`, ApplyAttribulatorOffsets 0x82d32114.
const DEGENERATE_EXTENT: f32 = 1000.0;
/// `LWModel::kLWModelCullingTable`: `CreateCollisionCullingTable` 0x82e63e40
/// with four parts sets every row/column bit of one 4x4 table.
const LW_MODEL_CULLING_PARTS: u32 = 4;

const INSTANCE_LOCAL_GUID_OFFSET: usize = 0x68;
const INSTANCE_CLASS_OFFSET: usize = 0x70;
const INSTANCE_COLLECTION_OFFSET: usize = 0x78;
const INSTANCE_R_MODEL_OFFSET: usize = 0x80;
const INSTANCE_C_MODEL_OFFSET: usize = 0x84;
/// `tInstance::m_pEmbedded`, read by `DynamicObject::InitSplines` 0x82d2bf54.
const INSTANCE_EMBEDDED_OFFSET: usize = 0x88;
const INSTANCE_RECORD_MINIMUM: usize = 0x8c;
/// Arena object-ID section of subreferences (`Arena::IdToObject`).

/// Characteristics layout offsets (`livingworld_dynamicobject_characteristics`).
const LAYOUT_OFFSET_VECTOR: usize = 0x00;
const LAYOUT_CENTRE_OF_MASS: usize = 0x10;
const LAYOUT_RESTITUTION: usize = 0x5c;
const LAYOUT_MASS: usize = 0x6c;
const LAYOUT_LINEAR_DRAG: usize = 0x70;
const LAYOUT_STATIC_FRICTION: usize = 0x74;
const LAYOUT_DYNAMIC_FRICTION: usize = 0x78;
const LAYOUT_ANGULAR_DRAG: usize = 0x80;

#[derive(Debug)]
pub(crate) enum DmoModelError {
    EmbeddedSplines(super::embedded_splines::EmbeddedSplineError),
    ModelCapacity,
    Attribute(AttributeRuntimeError),
    /// `Create` line 49: the kind-11 record is absent from the plan.
    MissingInstance {
        operation_index: usize,
    },
    ShortInstance {
        operation_index: usize,
        len: usize,
    },
    /// A nonlocal or missing RModel/CModel object behind a nonzero word.
    UnresolvedObject {
        operation_index: usize,
        word: u32,
    },
    ShortObject {
        operation_index: usize,
        word: u32,
    },
    CollisionModel(skate2_formats::pegasus::PegasusError),
    /// `LWModel::Create` 0x82e51010 (no pipe mesh) is not ported.
    UnportedFallback {
        model_id: u64,
    },
    /// `numMeshes != -1` procedural parts of CreateFromMesh are not ported.
    UnportedProceduralParts {
        model_id: u64,
        meshes: u32,
    },
    /// `[[CModel+8]+4] < 0x3e8`: the sphere(1.0) Definition is not ported.
    UnportedSphereDefinition {
        model_id: u64,
    },
    Assembly(DmoSerialisedAssemblySelectError),
    /// AdjustCentreOfMass 0x82d2a0b0 asserts a one-Part HiLOD Definition.
    PartCount {
        model_id: u64,
        parts: usize,
    },
    /// `FindCharacteristic` line 425: neither collection exists.
    MissingDefaultCharacteristic {
        class: u64,
    },
    Numeric(ReckoningHostNumericError),
    Allocation,
    EmbeddedData(skate2_formats::pegasus::PegasusError),
    GrabData(skate2_formats::pegasus::PegasusError),
    Sections(ArenaError),
    SplineSource(GrindDataSourceError),
    /// An embedded spline reference that names no local object or no inline
    /// `tSplineHeader`.
    UnresolvedSpline {
        operation_index: usize,
        object: u32,
    },
}

impl fmt::Display for DmoModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "DMO model attribute: {error}"),
            Self::Assembly(error) => write!(f, "DMO model Assembly: {error}"),
            Self::CollisionModel(error) => write!(f, "DMO model CModel: {error}"),
            other => write!(f, "DMO model: {other:?}"),
        }
    }
}

impl std::error::Error for DmoModelError {}

impl From<AttributeRuntimeError> for DmoModelError {
    fn from(error: AttributeRuntimeError) -> Self {
        Self::Attribute(error)
    }
}

/// `Sk8::LivingWorld::LWDynamicObjectModel` (224 bytes).
pub(crate) struct LwDynamicObjectModel {
    /// InitializeHotpoints82d38ec0, last authored type0 wins; ctor is null.
    pub(crate) hand_hotpoint_transform: Option<[f32;16]>,
    pub(crate) waypoint_groups: Vec<crate::living_world_grind_services::waypoints::WaypointGroupSource>,
    /// `mAssetID`: the instance's primary asset ID.
    pub(crate) asset_id: u64,
    /// `ModelMan::mModelMap` key.
    pub(crate) model_id: u64,
    /// `mCharacteristics` (+0xcc) and the collection `FindCharacteristic` chose.
    pub(crate) characteristics: AttributeRuntimeInstance,
    pub(crate) characteristic_collection: u64,
    /// Characteristics `mass` (+0x6c), read again by `InitCollisionBodies`.
    pub(crate) mass: f32,
    /// `SetDimensions` (+0x20), `SetBBoxMin` (+0x30), `SetBBoxMax` (+0x40).
    pub(crate) dimensions: [f32; 4],
    pub(crate) bbox_min: [f32; 4],
    pub(crate) bbox_max: [f32; 4],
    /// `mpWorldCollisionModelHiLOD` after the in-place Part-0 edits, with the
    /// LivingWorld table stored as its `m_cullingTable` (0x82d33c9c).
    pub(crate) world_collision: Arc<SerialisedAssemblyTemplate>,
    pub(crate) culling_table: SerialisedAssemblyCullingTableSource,
    /// The processed bank arena that owns the instance, meshes and RModel.
    pub(crate) asset: Arc<DmoBankProcessedArenaAsset>,
    /// `tInstance +0x80` RModel object in that arena.
    pub(crate) render_model: ObjectId,
    /// Serialized source Assembly selected through `[CModel+8]`.
    pub(crate) assembly_object_id: ObjectId,
    /// `tEmbeddedData` group 1 in reference order: the `tSplineHeader*`
    /// values `InitSplines` hands `AddGrindSpline` for every spawned object.
    pub(crate) grind_splines: Vec<DmoGrindSpline>,
    /// `tEmbeddedData` group 2 in reference order (`AddGrabSpline`'s `tGrab*`):
    /// Retained decoded source, dictionary entry and byte offset.
    pub(crate) grab_splines: Vec<DmoGrabSpline>,
}

/// A section-2 reference into a retained tGrabData object. The containing
/// LwDynamicObjectModel also retains the original processed bank arena.
#[derive(Clone, Debug)]
pub(crate) struct DmoGrabSpline {
    pub(crate) entry_index: u32,
    pub(crate) header_offset: u32,
    pub(crate) data: Arc<GrabData>,
}
impl DmoGrabSpline {
    pub(crate) fn grab(&self) -> &skate2_formats::pegasus::Grab {
        self.data
            .grab_at_object_offset(self.header_offset)
            .expect("validated grab reference")
    }
}

/// One embedded grind spline: its `tSplineData` source and the inline header.
#[derive(Clone, Debug)]
pub(crate) struct DmoGrindSpline {
    pub(crate) source: GrindSplineSource,
    /// Object offset of the `tSplineHeader` inside the `tSplineData`.
    pub(crate) header_offset: u32,
}

impl fmt::Debug for LwDynamicObjectModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LwDynamicObjectModel")
            .field("asset_id", &format_args!("{:016x}", self.asset_id))
            .field("model_id", &format_args!("{:016x}", self.model_id))
            .field("mass", &self.mass)
            .finish_non_exhaustive()
    }
}

impl LwDynamicObjectModel {
    /// `LWModel::GetBBoxTransform` 0x82e4cd60: rows 0..2 copied; the bbox
    /// centre is transformed as `fma(z, c.z, fma(y, c.y, fma(x, c.x, w)))`.
    pub(crate) fn bbox_transform(&self, transform: &[f32; 16]) -> [f32; 16] {
        let mut result = *transform;
        let centre: [f32; 4] =
            std::array::from_fn(|lane| (self.bbox_min[lane] + self.bbox_max[lane]) * 0.5);
        for lane in 0..4 {
            let x = transform[lane].mul_add(centre[0], transform[12 + lane]);
            let y = transform[4 + lane].mul_add(centre[1], x);
            result[12 + lane] = transform[8 + lane].mul_add(centre[2], y);
        }
        result
    }

    /// `livingworld_dynamicobject_priority` collection key from the model's
    /// characteristics, or the zero `DefaultDataArea(0x10)` key.
    pub(crate) fn try_priority_collection(&self) -> Result<u64, AttributeRuntimeError> {
        Ok(self
            .characteristics
            .try_class_ref_spec_collection(PRIORITY_KEY, PRIORITY_REF_SPEC_TYPE)?
            .unwrap_or(0))
    }

    /// Generated MaxAngularAccelerationWhileMoving82d29348. Raw insrdi at
    /// 82d29368 supplies the upper hash word missing in Hex-Rays output.
    pub(crate) fn try_max_angular_acceleration_while_moving(
        &self,
    ) -> Result<f32, AttributeRuntimeError> {
        Ok(f32::from_be_bytes(node_bytes::<4>(
            &self.characteristics,
            0x116f_ceb3_0856_1152,
        )?))
    }
}

/// One kind-11 source handed to `AddDMOModel` by the bank's RegArena pass.
pub(crate) struct DmoModelSource<'a> {
    pub(crate) asset: &'a Arc<DmoBankProcessedArenaAsset>,
    pub(crate) operation_index: usize,
}

/// Cvars sampled by the model route.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DmoModelCvars {
    /// `kEnableDMOModelLoading`, AddDMOModel 0x82ce9c80.
    pub(crate) enable_model_loading: bool,
    /// `kEnableDMOBankIDLookup`, GetSimModelIDFromInstanceID 0x82d2eea0.
    pub(crate) enable_bank_id_lookup: bool,
}

enum LwModel {
    DynamicObject(Arc<LwDynamicObjectModel>),
    Character(Arc<super::character_models::CharacterModel>),
}

/// ONE ModelMan map for all categories, preserving cross-category identity
/// and the shared 256-node limit. The historical type name is kept for callers.
pub(crate) struct DmoModelMan {
    models: BTreeMap<u64, LwModel>,
    dmo_model_count: u32,
    recipe_count: u32,
    models_updated: bool,
    initial_model_update: bool,
    /// The Inventory's shared `kLWModelCullingTable` resource.
    culling_table: Option<Arc<BitTable>>,
}

impl DmoModelMan {
    pub(crate) const fn new() -> Self {
        Self {
            models: BTreeMap::new(),
            dmo_model_count: 0,
            recipe_count: 0,
            models_updated: false,
            initial_model_update: true,
            culling_table: None,
        }
    }

    /// GetModel82ce7970 presence check, before a caller selects its category.
    pub(crate) fn contains_model(&self, model_id: u64) -> bool {
        self.models.contains_key(&model_id)
    }

    /// `ModelMan::GetModel` 0x82ce7970.
    pub(crate) fn model(&self, model_id: u64) -> Option<&Arc<LwDynamicObjectModel>> {
        match self.models.get(&model_id) {
            Some(LwModel::DynamicObject(model)) => Some(model),
            _ => None,
        }
    }

    pub(crate) fn character_model(
        &self,
        id: u64,
    ) -> Option<&Arc<super::character_models::CharacterModel>> {
        match self.models.get(&id) {
            Some(LwModel::Character(model)) => Some(model),
            _ => None,
        }
    }
    /// ModelMan.Update82ce6c30; this is not a count of load completions.
    pub(crate) fn update(&mut self) {
        self.models_updated = self.initial_model_update;
        self.initial_model_update = false;
    }
    pub(crate) fn models_updated(&self) -> bool {
        self.models_updated
    }
    pub(crate) fn recipe_count(&self) -> u32 {
        self.recipe_count
    }

    /// AddRecipe82ce9b00. Caller must deliver the real completed schema
    /// registration; category comes from that registered CharacterEntity.
    pub(crate) fn add_recipe(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        asset_id: u64,
        model_key: u64,
        scene: &crate::player::mounted_recipe::finalize::FinalizedRecipeScene,
        geometry: &BTreeMap<u64, Arc<crate::world::CollisionArena>>,
    ) -> Result<(), super::character_models::CharacterModelError> {
        use super::character_models::{CharacterModel, CharacterModelError as E};
        if self.models.contains_key(&asset_id) {
            return Ok(());
        }
        let category = scene.scene().map_err(E::Finalize)?.category;
        if category > 2 {
            return Ok(());
        }
        if self.models.len() == 256 {
            return Err(E::Capacity);
        }
        let table = self.culling_table_resource().map_err(E::Culling)?;
        let model = if category < 2 {
            CharacterModel::create_pedestrian(vault, model_key, table)?
        } else {
            CharacterModel::create_vehicle(vault, model_key, asset_id, scene, geometry, table)?
        };
        self.models
            .insert(asset_id, LwModel::Character(Arc::new(model)));
        self.recipe_count = self.recipe_count.wrapping_add(1);
        Ok(())
    }

    /// RemoveRecipe82ce8cd0 notifies the retained entity managers BEFORE
    /// releasing the model. Existing live references must be handled by them.
    pub(crate) fn remove_recipe<E>(
        &mut self,
        id: u64,
        notify: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.models.contains_key(&id) {
            notify(id)?;
            self.models.remove(&id);
            self.recipe_count = self.recipe_count.wrapping_sub(1);
        }
        Ok(())
    }

    pub(crate) const fn dmo_model_count(&self) -> u32 {
        self.dmo_model_count
    }

    /// `ModelMan::AddDMOModel` 0x82ce9c60 for one kind-11 operation.
    pub(crate) fn try_add_dmo_model(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        source: &DmoModelSource<'_>,
        cvars: DmoModelCvars,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), DmoModelError> {
        if !cvars.enable_model_loading {
            return Ok(());
        }
        let SimulationNormalOperation::DmoInstance { asset_id, .. } = source
            .asset
            .plan()
            .operations()
            .get(source.operation_index)
            .copied()
            .ok_or(DmoModelError::MissingInstance {
                operation_index: source.operation_index,
            })?
        else {
            return Err(DmoModelError::MissingInstance {
                operation_index: source.operation_index,
            });
        };
        let record = instance_record(source)?;
        let model_id = if cvars.enable_bank_id_lookup {
            read_u64(&record, INSTANCE_LOCAL_GUID_OFFSET)
        } else {
            asset_id
        };
        if self.models.contains_key(&model_id) {
            return Ok(());
        }
        let table = self.culling_table_resource()?;
        // Create returned false: the new model is destroyed, nothing is kept.
        let Some(model) =
            try_create_model(vault, source, &record, asset_id, model_id, table, numeric)?
        else {
            return Ok(());
        };
        if self.models.len() == 256 {
            return Err(DmoModelError::ModelCapacity);
        }
        self.models
            .insert(model_id, LwModel::DynamicObject(Arc::new(model)));
        self.dmo_model_count = self.dmo_model_count.wrapping_add(1);
        Ok(())
    }

    /// `Inventory::FindResource`/`AddResource` of `kLWModelCullingTable`:
    /// the first CreateCollisionCullingTable builds it, later ones share it.
    fn culling_table_resource(&mut self) -> Result<Arc<BitTable>, DmoModelError> {
        if let Some(table) = &self.culling_table {
            return Ok(Arc::clone(table));
        }
        let parts = LW_MODEL_CULLING_PARTS;
        let words = (parts * parts).wrapping_add(31) >> 5;
        let mut table = BitTable {
            rows: parts,
            columns: parts,
            array_size: words,
            words: Vec::new(),
        };
        table
            .words
            .try_reserve_exact(words as usize)
            .map_err(|_| DmoModelError::Allocation)?;
        table.words.resize(words as usize, 0);
        for row in 0..parts {
            for column in 0..parts {
                let bit = table.columns * row + column;
                table.words[(bit >> 5) as usize] |= 1 << (bit & 0x1f);
            }
        }
        let table = Arc::new(table);
        self.culling_table = Some(Arc::clone(&table));
        Ok(table)
    }
}

/// The kind-11 record bytes (`GetRegAsset` of the instance ID).
fn instance_record(source: &DmoModelSource<'_>) -> Result<Vec<u8>, DmoModelError> {
    let plan = source.asset.plan();
    let record = plan
        .record_source(source.operation_index)
        .filter(|record| record.kind() == SimulationRecordSourceKind::DmoInstance)
        .ok_or(DmoModelError::MissingInstance {
            operation_index: source.operation_index,
        })?;
    let bytes = record.bytes();
    if bytes.len() < INSTANCE_RECORD_MINIMUM {
        return Err(DmoModelError::ShortInstance {
            operation_index: source.operation_index,
            len: bytes.len(),
        });
    }
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| DmoModelError::Allocation)?;
    owned.extend_from_slice(bytes);
    Ok(owned)
}

/// Bytes of one local arena object named by a pointer word, `None` for null.
fn object_bytes<'a>(
    asset: &'a DmoBankProcessedArenaAsset,
    operation_index: usize,
    word: u32,
) -> Result<Option<&'a [u8]>, DmoModelError> {
    if word == 0 {
        return Ok(None);
    }
    let decoded = &asset.plan().static_plan().asset().decoded;
    let entry =
        decoded
            .arena
            .local_entry(ObjectId(word))
            .ok_or(DmoModelError::UnresolvedObject {
                operation_index,
                word,
            })?;
    decoded
        .arena
        .object_bytes(&decoded.resources.resources, entry)
        .map(Some)
        .ok_or(DmoModelError::UnresolvedObject {
            operation_index,
            word,
        })
}

/// `LWDynamicObjectModel::Create` 0x82d39f90. `Ok(None)` is Create's false.
fn try_create_model(
    vault: &Arc<AttributeRuntimeVault>,
    source: &DmoModelSource<'_>,
    record: &[u8],
    asset_id: u64,
    model_id: u64,
    culling_table: Arc<BitTable>,
    numeric: &mut HostReckoningNumeric,
) -> Result<Option<LwDynamicObjectModel>, DmoModelError> {
    let collection = find_characteristic(vault, record)?;
    let characteristics = vault.instance(CHARACTERISTICS_CLASS, collection);
    let operation_index = source.operation_index;
    let asset = source.asset;
    // CreateFromMesh 0x82d337b0.
    let r_model_word = read_u32(record, INSTANCE_R_MODEL_OFFSET);
    let c_model_word = read_u32(record, INSTANCE_C_MODEL_OFFSET);
    let r_model = object_bytes(asset, operation_index, r_model_word)?;
    let c_model = object_bytes(asset, operation_index, c_model_word)?;
    let (Some(r_model), Some(c_model)) = (r_model, c_model) else {
        return Err(DmoModelError::UnportedFallback { model_id });
    };
    let meshes =
        CollisionModelData::raw_mesh_count(c_model).map_err(DmoModelError::CollisionModel)?;
    if meshes == 0 {
        return Err(DmoModelError::UnportedFallback { model_id });
    }
    // A mesh-table CModel builds its own one-Part Definition with `PartDefinition` defaults
    // and no ApplyAttribulatorOffsets. Only the Skate 3 prop loader supplies that Definition
    // (`ui::object_dropper::add_procedural_assembly`), as the arena's only Assembly.
    let procedural = meshes != u32::MAX;
    if procedural && (meshes != 1 || asset.only_serialised_assembly().is_none()) {
        return Err(DmoModelError::UnportedProceduralParts { model_id, meshes });
    }
    if r_model.len() < 0x20 {
        return Err(DmoModelError::ShortObject {
            operation_index,
            word: r_model_word,
        });
    }
    let entry_type = asset
        .plan()
        .static_plan()
        .asset()
        .decoded
        .arena
        .local_entry(ObjectId(c_model_word))
        .map(|entry| entry.type_id);
    if entry_type != Some(C_MODEL_DATA_TYPE_ID) || c_model.len() < 0x0c {
        return Err(DmoModelError::ShortObject {
            operation_index,
            word: c_model_word,
        });
    }
    let bbox_min = read_vec4(r_model, 0x00);
    let bbox_max = read_vec4(r_model, 0x10);
    // lvx/vsubfp: all four lanes, SetDimensions retains W too.
    let dimensions = std::array::from_fn(|lane| bbox_max[lane] - bbox_min[lane]);
    let template = match asset.only_serialised_assembly() {
        Some(template) if procedural => template,
        _ => {
            asset
                .try_serialised_assembly_by_object_id(ObjectId(read_u32(c_model, 0x08)))
                .map_err(|error| match error {
                    DmoSerialisedAssemblySelectError::Missing { .. } => {
                        DmoModelError::UnportedSphereDefinition { model_id }
                    }
                    other => DmoModelError::Assembly(other),
                })?
                .1
        }
    };
    let assembly_object_id = template.assembly_object_id();
    if template.parts().len() != 1 {
        return Err(DmoModelError::PartCount {
            model_id,
            parts: template.parts().len(),
        });
    }
    let layout = CharacteristicsLayout::try_read(&characteristics)?;
    let part = if procedural {
        let part = &template.parts()[0];
        SerialisedAssemblyLivingWorldPart {
            inertia: part.inertia,
            inverse_body_transform: part.inverse_body_transform,
            has_inverse_body_transform: part.has_inverse_body_transform,
            material: part.material,
        }
    } else {
        apply_attribulator_offsets(
            &template.parts()[0],
            &characteristics,
            &layout,
            bbox_min,
            bbox_max,
            numeric,
        )?
    };
    let owner = Arc::clone(template.owner());
    let culling_table =
        SerialisedAssemblyCullingTableSource::living_world_model(owner, culling_table);
    let world_collision = Arc::new(template.living_world_model_copy(part));
    let (grind_splines, grab_splines) =
        try_model_splines(asset, operation_index, record, asset_id)?;
    let (hand_hotpoint_transform,waypoint_groups)=try_model_hotpoints(asset,operation_index,record)?;
    Ok(Some(LwDynamicObjectModel {
        hand_hotpoint_transform,waypoint_groups,
        asset_id,
        model_id,
        characteristics,
        characteristic_collection: collection,
        mass: layout.mass,
        dimensions,
        bbox_min,
        bbox_max,
        world_collision,
        culling_table,
        asset: Arc::clone(asset),
        render_model: ObjectId(r_model_word),
        assembly_object_id,
        grind_splines,
        grab_splines,
    }))
}

/// `Arena::IdToObject` for a local object or a subreference: the dictionary
/// entry index and the byte offset inside that object.
use super::embedded_splines::ModelSplines;

/// Project the native retained instance's hotpoint array without another arena
/// or decoder. tHotPoint PDB: affine+0, bbox+40, type+60.
fn try_model_hotpoints(asset:&DmoBankProcessedArenaAsset,operation_index:usize,record:&[u8])
    ->Result<(Option<[f32;16]>,Vec<crate::living_world_grind_services::waypoints::WaypointGroupSource>),DmoModelError>{
    let Some(bytes)=object_bytes(asset,operation_index,read_u32(record,INSTANCE_EMBEDDED_OFFSET))?
        else{return Ok((None,Vec::new()));};
    let embedded=EmbeddedData::parse(bytes).map_err(DmoModelError::EmbeddedData)?;
    let decoded=&asset.plan().static_plan().asset().decoded;
    let sections=decoded.arena.sections(&decoded.resources.resources).map_err(DmoModelError::Sections)?;
    let mut result=None;
    let mut groups=Vec::new();
    for reference in embedded.hot_points(){
        let fail=||DmoModelError::UnresolvedObject{operation_index,word:reference.object.0};
        let(index,offset)=super::embedded_splines::resolve_reference(&sections,reference.object).ok_or_else(fail)?;
        let entry=decoded.arena.dictionary.get(index as usize).ok_or_else(fail)?;
        let bytes=decoded.arena.object_bytes(&decoded.resources.resources,entry).ok_or_else(fail)?;
        let start=offset as usize;
        let end=start.checked_add(0x64).ok_or_else(fail)?;
        let bytes=bytes.get(start..end).ok_or_else(fail)?;
        let transform=std::array::from_fn(|i|read_f32(bytes,i*4));
        let kind=read_u32(bytes,0x60);
        if kind==0 {result=Some(transform);}else{
            let name=match kind{1=>"waypoint_usetrashbin",2=>"waypoint_newspaperbox",_=>"waypoint_sit"};
            let key=skate2_formats::attrib::string_hash64(name);
            if key!=0 {groups.push(crate::living_world_grind_services::waypoints::WaypointGroupSource::from_hotpoint(key,transform));}
        }
    }
    Ok((result,groups))
}

/// `DynamicObject::InitSplines` 0x82d2bf18 inputs: `tInstance::m_pEmbedded`
/// group 2 (grab) and group 1 (grind) references, in array order. A null
/// embedded pointer adds none.
fn try_model_splines(
    asset: &Arc<DmoBankProcessedArenaAsset>,
    operation_index: usize,
    record: &[u8],
    asset_id: u64,
) -> Result<ModelSplines, DmoModelError> {
    let word = read_u32(record, INSTANCE_EMBEDDED_OFFSET);
    let Some(bytes) = object_bytes(asset, operation_index, word)? else {
        return Ok((Vec::new(), Vec::new()));
    };
    let embedded = EmbeddedData::parse(bytes).map_err(DmoModelError::EmbeddedData)?;
    let decoded = &asset.plan().static_plan().asset().decoded;
    super::embedded_splines::decode(
        &decoded.arena,
        &decoded.resources.resources,
        &embedded,
        &mut |entry_index, bytes| {
            GrindSplineSource::try_from_dmo_spline_data(
                asset.plan_owner(),
                operation_index,
                entry_index,
                asset_id,
                bytes,
            )
        },
    )
    .map_err(DmoModelError::EmbeddedSplines)
}

/// `FindCharacteristic` 0x82d29f58: the returned key is always bound to the
/// characteristics class; the instance's class only decides the lookup.
fn find_characteristic(vault: &AttributeRuntimeVault, record: &[u8]) -> Result<u64, DmoModelError> {
    let mut class = read_u64(record, INSTANCE_CLASS_OFFSET);
    let mut collection = read_u64(record, INSTANCE_COLLECTION_OFFSET);
    if class == INVALID_KEY || class == DEFAULT_COLLECTION {
        class = CHARACTERISTICS_CLASS;
    }
    if collection == INVALID_KEY {
        collection = DEFAULT_COLLECTION;
    }
    if !vault.has_class(class) {
        class = CHARACTERISTICS_CLASS;
    }
    if vault.has_collection(class, collection) {
        return Ok(collection);
    }
    if !vault.has_collection(class, DEFAULT_COLLECTION) {
        return Err(DmoModelError::MissingDefaultCharacteristic { class });
    }
    Ok(DEFAULT_COLLECTION)
}

/// Characteristics layout fields read as plain data. A null layout is the
/// generated class's zero `DefaultDataArea`.
struct CharacteristicsLayout {
    offset: [f32; 4],
    centre_of_mass: [f32; 4],
    restitution: f32,
    mass: f32,
    linear_drag: f32,
    static_friction: f32,
    dynamic_friction: f32,
    angular_drag: f32,
}

impl CharacteristicsLayout {
    fn try_read(instance: &AttributeRuntimeInstance) -> Result<Self, AttributeRuntimeError> {
        let scalar = |offset| -> Result<f32, AttributeRuntimeError> {
            Ok(instance
                .try_cached_layout_field(offset, 4)?
                .map_or(0.0, |data| read_f32(data.bytes(), 0)))
        };
        let vector = |offset| -> Result<[f32; 4], AttributeRuntimeError> {
            Ok(instance
                .try_cached_layout_field(offset, 16)?
                .map_or([0.0; 4], |data| read_vec4(data.bytes(), 0)))
        };
        Ok(Self {
            offset: vector(LAYOUT_OFFSET_VECTOR)?,
            centre_of_mass: vector(LAYOUT_CENTRE_OF_MASS)?,
            restitution: scalar(LAYOUT_RESTITUTION)?,
            mass: scalar(LAYOUT_MASS)?,
            linear_drag: scalar(LAYOUT_LINEAR_DRAG)?,
            static_friction: scalar(LAYOUT_STATIC_FRICTION)?,
            dynamic_friction: scalar(LAYOUT_DYNAMIC_FRICTION)?,
            angular_drag: scalar(LAYOUT_ANGULAR_DRAG)?,
        })
    }
}

/// Characteristics node value, or the zero `DefaultDataArea(size)`.
fn node_bytes<const N: usize>(
    instance: &AttributeRuntimeInstance,
    key: u64,
) -> Result<[u8; N], AttributeRuntimeError> {
    let Some(data) = instance.try_get_data(key, 0)? else {
        return Ok([0; N]);
    };
    let bytes = data.bytes();
    if bytes.len() < N {
        return Err(AttributeRuntimeError::PayloadSize {
            key,
            expected: N,
            actual: bytes.len(),
        });
    }
    Ok(bytes[..N].try_into().expect("checked length"))
}

/// `ApplyAttribulatorOffsets` 0x82d31e28 on the HiLOD Definition's Part 0.
/// No field is read back between its stores, so only final values matter.
fn apply_attribulator_offsets(
    part: &crate::serialised_assembly_template::SerialisedAssemblyTemplatePart,
    characteristics: &AttributeRuntimeInstance,
    layout: &CharacteristicsLayout,
    bbox_min: [f32; 4],
    bbox_max: [f32; 4],
    numeric: &mut HostReckoningNumeric,
) -> Result<SerialisedAssemblyLivingWorldPart, DmoModelError> {
    let mut inertia: AssemblyPartInertia = part.inertia;
    // 0x82d31e8c..1f1c: W is forced to zero before the +Inf compare chain.
    let tensor = inertia.inverse_tensor;
    if [tensor[0], tensor[1], tensor[2], 0.0].contains(&f32::INFINITY) {
        inertia.inverse_tensor = [1.0; 4];
        inertia.spherical = spherical(inertia.inverse_tensor);
    }
    let (inverse_body_transform, has_inverse_body_transform) =
        adjust_centre_of_mass(part, layout.centre_of_mass);
    inertia.inverse_mass = 1.0 / layout.mass;
    inertia.linear_drag = layout.linear_drag;
    inertia.angular_drag = layout.angular_drag;
    inertia.max_velocity = f32::from_be_bytes(node_bytes::<4>(characteristics, MAX_VELOCITY_KEY)?);
    inertia.max_omega = f32::from_be_bytes(node_bytes::<4>(characteristics, MAX_OMEGA_KEY)?);
    // 0x82d32068..20dc: half = (max - min) * 0.5 (vcsxwfp 1, scale 1);
    // vmaddfp128 extents = half * scale + layout offset.
    let scale = node_bytes::<16>(characteristics, INERTIA_EXTENT_SCALE_KEY)?;
    let scale = read_vec4(&scale, 0);
    let extents: [f32; 4] = std::array::from_fn(|lane| {
        ((bbox_max[lane] - bbox_min[lane]) * 0.5).mul_add(scale[lane], layout.offset[lane])
    });
    let extents = if extents[0] > 0.0 && extents[1] > 0.0 && extents[2] > 0.0 {
        extents
    } else {
        [DEGENERATE_EXTENT, DEGENERATE_EXTENT, DEGENERATE_EXTENT, 0.0]
    };
    set_inertia_extents(&mut inertia, extents, numeric).map_err(DmoModelError::Numeric)?;
    Ok(SerialisedAssemblyLivingWorldPart {
        inertia,
        inverse_body_transform,
        has_inverse_body_transform,
        material: AssemblyPartMaterial {
            static_friction: layout.static_friction,
            dynamic_friction: layout.dynamic_friction,
            restitution: layout.restitution,
        },
    })
}

/// `x < y ? x : y` then `f0 < z ? 1/f0 : 1/z` (fcmpu; unordered takes the
/// second operand), shared by SetInertia 0x82d2d8d8 and 0x82d31f44.
fn spherical(tensor: [f32; 4]) -> f32 {
    let first = if tensor[0] < tensor[1] {
        tensor[0]
    } else {
        tensor[1]
    };
    if first < tensor[2] {
        1.0 / first
    } else {
        1.0 / tensor[2]
    }
}

/// `AdjustCentreOfMass` 0x82d2a090: `com = -layout[0x10]` (sign xor on all
/// four lanes). No inverse: rows `I, J, K, com`. Otherwise `A * M` with
/// `A = [I; J; K; com]`, each row a left-to-right fused chain.
fn adjust_centre_of_mass(
    part: &crate::serialised_assembly_template::SerialisedAssemblyTemplatePart,
    centre_of_mass: [f32; 4],
) -> ([f32; 16], u32) {
    let com = centre_of_mass.map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000));
    const I: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
    const J: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
    const K: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
    if part.has_inverse_body_transform == 0 {
        let mut matrix = [0.0; 16];
        matrix[0..4].copy_from_slice(&I);
        matrix[4..8].copy_from_slice(&J);
        matrix[8..12].copy_from_slice(&K);
        matrix[12..16].copy_from_slice(&com);
        return (matrix, 1);
    }
    let source = part.inverse_body_transform;
    let row = |index: usize| -> [f32; 4] { std::array::from_fn(|lane| source[index * 4 + lane]) };
    let (row0, row1, row2, row3) = (row(0), row(1), row(2), row(3));
    let mut matrix = [0.0; 16];
    for lane in 0..4 {
        // v12 = I.x*row0; v30 = I.y*row1 + v12; v26 = I.z*row2 + v30.
        let first = I[1].mul_add(row1[lane], I[0] * row0[lane]);
        matrix[lane] = I[2].mul_add(row2[lane], first);
        // v9 = J.x*row0; v29 = J.y*row1 + v9; v25 = J.z*row2 + v29.
        let second = J[0] * row0[lane];
        let second = J[1].mul_add(row1[lane], second);
        matrix[4 + lane] = J[2].mul_add(row2[lane], second);
        // v8 = K.x*row0; v28 = K.y*row1 + v8; v24 = K.z*row2 + v28.
        let third = K[0] * row0[lane];
        let third = K[1].mul_add(row1[lane], third);
        matrix[8 + lane] = K[2].mul_add(row2[lane], third);
        // v6 = com.x*row0 + row3; v27 = com.y*row1 + v6; v23 = com.z*row2 + v27.
        let fourth = com[0].mul_add(row0[lane], row3[lane]);
        let fourth = com[1].mul_add(row1[lane], fourth);
        matrix[12 + lane] = com[2].mul_add(row2[lane], fourth);
    }
    (matrix, 1)
}

/// `Inertia::SetInertiaExtents` 0x82d30210 then `SetInertia` 0x82d2d8a8.
fn set_inertia_extents(
    inertia: &mut AssemblyPartInertia,
    extents: [f32; 4],
    numeric: &mut HostReckoningNumeric,
) -> Result<(), ReckoningHostNumericError> {
    let squared = extents.map(|lane| lane * lane);
    let third_mass = ONE_THIRD / inertia.inverse_mass;
    // vperm<0,5,0,0>(ey2+ez2, ex2+ez2) then vrlimi z <- ex2+ey2.
    let yz = squared[1] + squared[2];
    let xz = squared[0] + squared[2];
    let xy = squared[0] + squared[1];
    let moment = [yz, xz, xy, yz].map(|lane| lane * third_mass);
    let mut tensor = [0.0; 4];
    for (out, value) in tensor.iter_mut().zip(moment) {
        let estimate = numeric.reciprocal_estimate(value)?;
        // vnmsubfp 1 - e*x, vmaddfp e*t + e, twice.
        let first = estimate.mul_add((-estimate).mul_add(value, 1.0), estimate);
        *out = first.mul_add((-first).mul_add(value, 1.0), first);
    }
    inertia.inverse_tensor = tensor;
    inertia.spherical = spherical(tensor);
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked record"),
    )
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("checked record"),
    )
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_bits(read_u32(bytes, offset))
}

fn read_vec4(bytes: &[u8], offset: usize) -> [f32; 4] {
    std::array::from_fn(|lane| read_f32(bytes, offset + lane * 4))
}
