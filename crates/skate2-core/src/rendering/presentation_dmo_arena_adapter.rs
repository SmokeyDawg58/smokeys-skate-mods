//! Presentation-side routing for the processed main-DMO-bank asset slice.
//!
//! This is deliberately a narrow subsequence of `Presentation::RegArena`
//! (`0x827f1430`) and `Presentation::DRegArena` (`0x827f1680`). It preserves
//! the collection receiver's first-use/last-use arena coalescing and the
//! authored kind-2-material-before-kind-11-DMO order in the reference bank,
//! including the Living World-before-DMO pair order. It does not claim the
//! other Presentation asset kinds, deferred message emission/receiver timing,
//! proxy collections, or the manager internals behind the dispatch seam. Its
//! owner-bound deferred selector is a producer-input boundary only.

use crate::{
    arena_registration_ref_counter::{
        ArenaRegistrationAddTransition, ArenaRegistrationRefCounter,
        ArenaRegistrationRemoveTransition,
    },
    collection_resident_streamer::CollectionId,
    dmo_bank_processed_asset::{
        DMO_BANK_ASSET_ID_SIGNATURE, DMO_BANK_COLLECTION_ID, DmoBankProcessedArenaAsset,
    },
    owned_resident_collection::OwnedResidentAssetsVisitError,
    simulation_deferred_batch_policy::{
        DeferredCollectionBatchControls, PresentationDeferredBatchSelection,
        PresentationDeferredBatchSelectionError, select_presentation_deferred_batch,
    },
    simulation_normal_arena_plan::{
        SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordHandle,
        SimulationRecordSourceKind,
    },
    simulation_resident_registry::{
        SimulationResidentArenaLookup, SimulationResidentArenaLookupError,
        SimulationResidentLookup, SimulationResidentLookupError, SimulationResidentVisitError,
    },
    simulation_static_arena_plan::SimulationStaticArenaPlan,
};
use skate2_formats::{
    material::{MaterialError, R_MATERIAL_DATA_TYPE_ID, RenderMaterialData},
    pegasus::{
        INSTANCE_DATA_TYPE_ID, ObjectId, PegasusError, TABLE_OF_CONTENTS_TYPE_ID, TableOfContents,
    },
    rw4::DictionaryEntry,
};
use std::{
    collections::{BTreeMap, btree_map::Entry},
    convert::Infallible,
    error::Error,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
};

const DMO_USERDATA_MASK: u32 = 8;
const DMO_RECORD_LEN: usize = 160;
const DMO_PRIMARY_GUID_OFFSET: usize = 96;
const DMO_LOCAL_GUID_OFFSET: usize = 104;
const MAIN_DMO_BANK_ARENA_COUNT: usize = 125;
const MAIN_DMO_BANK_ACTION_COUNT: usize = 94;
const MAIN_DMO_BANK_MATERIAL_COUNT: usize = 1;
const MAIN_DMO_BANK_DMO_COUNT: usize = 93;
const MAIN_DMO_BANK_FINAL_ARENA_ID: u64 = 0xd309_6f13_cf1c_0aab;
const MAIN_DMO_BANK_MATERIAL_GUID: u64 = 0x8f51_812c_ef1b_09a4;
const MAIN_DMO_BANK_MATERIAL_TOC_INDEX: u32 = 68;
const MAIN_DMO_BANK_DMO_TOC_INDEX: u32 = 162;
const MAIN_DMO_BANK_FIRST_ARENA_ID: u64 = 0x91d7_2407_5ae4_4972;
const MAIN_DMO_BANK_ACTION_SIGNATURE: u64 = 0xa73f_2da8_1dae_97b8;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Exact void-returning manager boundaries reached by supported kinds 2 and 11.
///
/// The adapter owns call ordering. The Living World call receives the typed
/// asset's primary GUID. The DMO registration call receives the retained
/// `Instance_DMO` record whose local GUID is its internal registry key.
/// Implementations may clone the source handle if manager state retains the
/// native-pointer-equivalent record beyond the call.
pub trait PresentationDmoArenaDispatch {
    /// `TextureMan::AddMaterial` (`0x8282c838`) receives only this whole
    /// `tRMaterialData` source pointer; its TOC GUID is not a call argument.
    fn add_material(&mut self, source: &PresentationMaterialDataHandle);
    fn add_living_world_dmo_model(&mut self, asset_id: u64);
    fn register_presentation_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64);
    /// `TextureMan::RemMaterial` (`0x82825cf8`) receives the same source shape.
    fn remove_material(&mut self, source: &PresentationMaterialDataHandle);
    fn remove_living_world_dmo_model(&mut self, asset_id: u64);
    fn deregister_presentation_dmo_model(&mut self, source: &SimulationRecordHandle, asset_id: u64);
}

#[derive(Clone, Debug)]
struct PreparedDmoOperation {
    source: SimulationRecordHandle,
    asset_id: u64,
    local_guid: u64,
    source_toc_index: u32,
    source_dictionary_index: u32,
    source_inline_index: u32,
    source_record_offset: u32,
    raw_object_id: u32,
}

#[derive(Clone, Copy, Debug)]
struct PresentationMaterialDataLocator {
    resource_index: usize,
    object_start: usize,
    object_len: usize,
}

/// Owner-bound identity for the complete serialized `tRMaterialData` object.
///
/// This is immutable provenance, not an ABI-ready mutable Xbox structure.
/// Native `AddMaterial` mutates channel stream fields and `RemMaterial` clears
/// them; that concrete `TextureMan` materialization remains behind the dispatch
/// seam. Pointer-equivalent identity is owner plus resource and object start,
/// never value equality.
#[derive(Clone)]
pub struct PresentationMaterialDataHandle {
    owner: Arc<SimulationStaticArenaPlan>,
    locator: PresentationMaterialDataLocator,
    diagnostic_guid: u64,
    source_toc_index: u32,
    source_dictionary_index: u32,
    raw_object_id: u32,
}

impl fmt::Debug for PresentationMaterialDataHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PresentationMaterialDataHandle")
            .field("diagnostic_guid", &self.diagnostic_guid)
            .field("source_toc_index", &self.source_toc_index)
            .field("source_dictionary_index", &self.source_dictionary_index)
            .field("raw_object_id", &self.raw_object_id)
            .field("resource_index", &self.locator.resource_index)
            .field("object_start", &self.locator.object_start)
            .field("object_len", &self.locator.object_len)
            .finish_non_exhaustive()
    }
}

impl PresentationMaterialDataHandle {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.owner.asset().decoded.resources.resources[self.locator.resource_index]
            [self.locator.object_start..self.locator.object_start + self.locator.object_len]
    }

    /// Diagnostic processed-asset identity retained from the TOC. `TextureMan`
    /// does not receive this value in the kind-2 manager call.
    #[must_use]
    pub const fn diagnostic_guid(&self) -> u64 {
        self.diagnostic_guid
    }

    #[must_use]
    pub const fn source_toc_index(&self) -> u32 {
        self.source_toc_index
    }

    #[must_use]
    pub const fn source_dictionary_index(&self) -> u32 {
        self.source_dictionary_index
    }

    #[must_use]
    pub const fn raw_object_id(&self) -> u32 {
        self.raw_object_id
    }

    #[must_use]
    pub fn is_same_backing_object(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.locator.resource_index == other.locator.resource_index
            && self.locator.object_start == other.locator.object_start
    }
}

#[derive(Clone, Debug)]
enum PreparedPresentationOperation {
    Material(PresentationMaterialDataHandle),
    Dmo(PreparedDmoOperation),
}

#[derive(Debug)]
struct PreparedPresentationDmoArenaInner {
    plan_owner: Arc<SimulationNormalArenaPlan>,
    arena_id: CollectionId,
    cost: u32,
    operations: Vec<PreparedPresentationOperation>,
}

/// One prevalidated, owner-bound main-DMO-bank Presentation subsequence.
pub struct PreparedPresentationDmoArena {
    inner: Arc<PreparedPresentationDmoArenaInner>,
}

impl fmt::Debug for PreparedPresentationDmoArena {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedPresentationDmoArena")
            .field("arena_id", &self.inner.arena_id)
            .field("cost", &self.inner.cost)
            .field("operation_count", &self.inner.operations.len())
            .field(
                "plan_operation_count",
                &self.inner.plan_owner.operations().len(),
            )
            .finish()
    }
}

impl PreparedPresentationDmoArena {
    /// Validates the supported material and every kind-11 source before
    /// runtime manager effects, preserving their authored TOC order.
    /// The public input is the opaque asset produced by the dedicated checked
    /// DMO-bank builder session; arbitrary normal Simulation plans cannot
    /// acquire this bank-scoped dispatch authority.
    ///
    /// # Errors
    ///
    /// Returns a typed TOC/material/source/allocation error. No dispatcher is
    /// reachable during preparation.
    pub fn try_new(
        asset: &DmoBankProcessedArenaAsset,
    ) -> Result<Self, PresentationDmoArenaPrepareError> {
        Self::from_plan(asset.plan_owner())
    }

    pub(crate) fn from_plan(
        plan: Arc<SimulationNormalArenaPlan>,
    ) -> Result<Self, PresentationDmoArenaPrepareError> {
        let arena_id = CollectionId(plan.static_plan().asset().decoded.asset_id);
        let cost = plan.presentation_arena_cost();
        let operations = prepare_operations(&plan)?;

        Ok(Self {
            inner: Arc::new(PreparedPresentationDmoArenaInner {
                plan_owner: plan,
                arena_id,
                cost,
                operations,
            }),
        })
    }

    #[must_use]
    pub fn arena_id(&self) -> CollectionId {
        self.inner.arena_id
    }

    /// Processed `cArena::mCost`, retained beside the exact Arena owner for
    /// deferred Presentation prefix selection.
    #[must_use]
    pub fn cost(&self) -> u32 {
        self.inner.cost
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operations.len()
    }

    #[must_use]
    pub fn material_operation_count(&self) -> usize {
        self.inner
            .operations
            .iter()
            .filter(|operation| matches!(operation, PreparedPresentationOperation::Material(_)))
            .count()
    }

    #[must_use]
    pub fn dmo_operation_count(&self) -> usize {
        self.inner
            .operations
            .iter()
            .filter(|operation| matches!(operation, PreparedPresentationOperation::Dmo(_)))
            .count()
    }
}

fn prepare_operations(
    plan: &Arc<SimulationNormalArenaPlan>,
) -> Result<Vec<PreparedPresentationOperation>, PresentationDmoArenaPrepareError> {
    let dmo_count = plan
        .operations()
        .iter()
        .filter(|operation| matches!(operation, SimulationNormalOperation::DmoInstance { .. }))
        .count();
    let decoded = &plan.static_plan().asset().decoded;
    let Some((toc_dictionary_index, toc_entry)) = decoded
        .arena
        .dictionary
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)
    else {
        if dmo_count != 0 {
            return Err(PresentationDmoArenaPrepareError::MissingTocWithDmoOperations);
        }
        return Ok(Vec::new());
    };
    let toc_bytes = object_bytes(plan, toc_dictionary_index, toc_entry)?;
    let toc =
        TableOfContents::parse(toc_bytes).map_err(PresentationDmoArenaPrepareError::InvalidToc)?;
    let material_count = toc
        .entries
        .iter()
        .filter(|entry| entry.object_type == R_MATERIAL_DATA_TYPE_ID)
        .count();
    let requested = material_count.saturating_add(dmo_count);
    let mut operations = Vec::new();
    operations
        .try_reserve_exact(requested)
        .map_err(|_| PresentationDmoArenaPrepareError::OperationStorageAllocation { requested })?;

    let mut dmo_iter = plan
        .operations()
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            SimulationNormalOperation::DmoInstance {
                source_toc_index, ..
            } => Some((index, *source_toc_index)),
            _ => None,
        })
        .peekable();

    for (toc_index, toc_entry) in toc.entries.iter().enumerate() {
        let source_toc_index = u32::try_from(toc_index)
            .map_err(|_| PresentationDmoArenaPrepareError::TocIndexOverflow(toc_index))?;
        if toc_entry.object_type == R_MATERIAL_DATA_TYPE_ID {
            operations.push(PreparedPresentationOperation::Material(
                prepare_material_handle(plan, source_toc_index, toc_entry.object, toc_entry.guid)?,
            ));
        }
        while dmo_iter
            .peek()
            .is_some_and(|(_, operation_toc_index)| *operation_toc_index == source_toc_index)
        {
            let (normal_operation_index, _) = dmo_iter
                .next()
                .ok_or(PresentationDmoArenaPrepareError::DmoOrderMismatch)?;
            operations.push(PreparedPresentationOperation::Dmo(prepare_dmo_operation(
                plan,
                normal_operation_index,
            )?));
        }
        if dmo_iter
            .peek()
            .is_some_and(|(_, operation_toc_index)| *operation_toc_index < source_toc_index)
        {
            return Err(PresentationDmoArenaPrepareError::DmoOrderMismatch);
        }
    }
    if dmo_iter.next().is_some() {
        return Err(PresentationDmoArenaPrepareError::DmoOrderMismatch);
    }
    Ok(operations)
}

fn prepare_dmo_operation(
    plan: &Arc<SimulationNormalArenaPlan>,
    normal_operation_index: usize,
) -> Result<PreparedDmoOperation, PresentationDmoArenaPrepareError> {
    let SimulationNormalOperation::DmoInstance {
        asset_id,
        local_guid,
        source_toc_index,
        source_dictionary_index,
        source_inline_index,
        source_record_offset,
        raw_object_id,
        ..
    } = plan.operations()[normal_operation_index]
    else {
        return Err(PresentationDmoArenaPrepareError::DmoOrderMismatch);
    };
    let source = plan.record_handle(normal_operation_index).ok_or(
        PresentationDmoArenaPrepareError::MissingRecordSource {
            normal_operation_index,
        },
    )?;
    if source.kind() != SimulationRecordSourceKind::DmoInstance {
        return Err(PresentationDmoArenaPrepareError::WrongRecordKind {
            normal_operation_index,
            actual: source.kind(),
        });
    }
    if source.bytes().len() != DMO_RECORD_LEN {
        return Err(PresentationDmoArenaPrepareError::WrongRecordLength {
            normal_operation_index,
            actual: source.bytes().len(),
        });
    }
    if read_u64(source.bytes(), DMO_PRIMARY_GUID_OFFSET) != Some(asset_id)
        || read_u64(source.bytes(), DMO_LOCAL_GUID_OFFSET) != Some(local_guid)
    {
        return Err(PresentationDmoArenaPrepareError::RecordIdentityMismatch {
            normal_operation_index,
        });
    }
    Ok(PreparedDmoOperation {
        source,
        asset_id,
        local_guid,
        source_toc_index,
        source_dictionary_index,
        source_inline_index,
        source_record_offset,
        raw_object_id,
    })
}

fn prepare_material_handle(
    plan: &Arc<SimulationNormalArenaPlan>,
    source_toc_index: u32,
    object: ObjectId,
    diagnostic_guid: u64,
) -> Result<PresentationMaterialDataHandle, PresentationDmoArenaPrepareError> {
    let section = object.section_index();
    if section != 0 {
        return Err(PresentationDmoArenaPrepareError::NonLocalMaterialObject {
            source_toc_index,
            raw_object_id: object.0,
            section,
        });
    }
    let source_dictionary_index = object.entry_index();
    let decoded = &plan.static_plan().asset().decoded;
    let entry = decoded
        .arena
        .dictionary
        .get(source_dictionary_index as usize)
        .ok_or(PresentationDmoArenaPrepareError::MissingMaterialObject {
            source_toc_index,
            raw_object_id: object.0,
            source_dictionary_index,
        })?;
    if entry.type_id != R_MATERIAL_DATA_TYPE_ID {
        return Err(PresentationDmoArenaPrepareError::WrongMaterialObjectType {
            source_toc_index,
            source_dictionary_index,
            actual: entry.type_id,
        });
    }
    let bytes = object_bytes(plan, source_dictionary_index as usize, entry)?;
    RenderMaterialData::parse(bytes).map_err(PresentationDmoArenaPrepareError::InvalidMaterial)?;
    Ok(PresentationMaterialDataHandle {
        owner: plan.static_plan_owner(),
        locator: PresentationMaterialDataLocator {
            resource_index: entry.resource_index,
            object_start: entry.object_range.start,
            object_len: entry.object_range.len(),
        },
        diagnostic_guid,
        source_toc_index,
        source_dictionary_index,
        raw_object_id: object.0,
    })
}

fn object_bytes<'a>(
    plan: &'a SimulationNormalArenaPlan,
    dictionary_index: usize,
    entry: &DictionaryEntry,
) -> Result<&'a [u8], PresentationDmoArenaPrepareError> {
    let decoded = &plan.static_plan().asset().decoded;
    decoded
        .arena
        .object_bytes(&decoded.resources.resources, entry)
        .ok_or(
            PresentationDmoArenaPrepareError::ObjectRangeOutsideResource {
                dictionary_index: u32::try_from(dictionary_index).unwrap_or(u32::MAX),
                type_id: entry.type_id,
            },
        )
}

#[derive(Debug)]
struct PreparedPresentationDmoCollectionInner {
    arenas: Vec<Arc<PreparedPresentationDmoArenaInner>>,
    arena_count: u32,
    operation_count: usize,
    material_operation_count: usize,
    dmo_operation_count: usize,
}

/// Caller-ordered set of processed arenas for one collection message.
pub struct PreparedPresentationDmoCollection {
    inner: Arc<PreparedPresentationDmoCollectionInner>,
}

impl Clone for PreparedPresentationDmoCollection {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl fmt::Debug for PreparedPresentationDmoCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedPresentationDmoCollection")
            .field("arena_count", &self.inner.arenas.len())
            .field("operation_count", &self.inner.operation_count)
            .field(
                "material_operation_count",
                &self.inner.material_operation_count,
            )
            .field("dmo_operation_count", &self.inner.dmo_operation_count)
            .finish()
    }
}

impl PreparedPresentationDmoCollection {
    /// Retains prepared arenas in caller-supplied collection order after
    /// certifying the exact main-DMO-bank arena/action census.
    ///
    /// # Errors
    ///
    /// Returns before producing a partial collection if host storage cannot be
    /// reserved or the aggregate operation count overflows.
    pub fn try_from_prepared(
        arenas: &[PreparedPresentationDmoArena],
    ) -> Result<Self, PresentationDmoCollectionPrepareError> {
        let requested = arenas.len();
        let arena_count = u32::try_from(requested).map_err(|_| {
            PresentationDmoCollectionPrepareError::ArenaCountOverflow { actual: requested }
        })?;
        let mut retained = Vec::new();
        retained.try_reserve_exact(requested).map_err(|_| {
            PresentationDmoCollectionPrepareError::ArenaStorageAllocation { requested }
        })?;
        let mut operation_count = 0_usize;
        let mut material_operation_count = 0_usize;
        let mut dmo_operation_count = 0_usize;
        let mut arena_id_signature = FNV_OFFSET;
        for arena in arenas {
            operation_count = operation_count
                .checked_add(arena.inner.operations.len())
                .ok_or(PresentationDmoCollectionPrepareError::OperationCountOverflow)?;
            material_operation_count = material_operation_count
                .checked_add(arena.material_operation_count())
                .ok_or(PresentationDmoCollectionPrepareError::OperationCountOverflow)?;
            dmo_operation_count = dmo_operation_count
                .checked_add(arena.dmo_operation_count())
                .ok_or(PresentationDmoCollectionPrepareError::OperationCountOverflow)?;
            for byte in arena.arena_id().0.to_be_bytes() {
                arena_id_signature ^= u64::from(byte);
                arena_id_signature = arena_id_signature.wrapping_mul(FNV_PRIME);
            }
            retained.push(Arc::clone(&arena.inner));
        }
        validate_main_dmo_bank_collection(
            &retained,
            operation_count,
            material_operation_count,
            dmo_operation_count,
            arena_id_signature,
        )?;
        Ok(Self {
            inner: Arc::new(PreparedPresentationDmoCollectionInner {
                arenas: retained,
                arena_count,
                operation_count,
                material_operation_count,
                dmo_operation_count,
            }),
        })
    }

    #[must_use]
    pub fn arena_count(&self) -> usize {
        self.inner.arenas.len()
    }

    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.inner.operation_count
    }

    #[must_use]
    pub fn material_operation_count(&self) -> usize {
        self.inner.material_operation_count
    }

    #[must_use]
    pub fn dmo_operation_count(&self) -> usize {
        self.inner.dmo_operation_count
    }

    /// Applies the recovered Presentation prefix scan to this exact retained
    /// collection. Each following cost lookup stays lazy and owner-bound.
    ///
    /// # Errors
    ///
    /// Returns the selector's progress/control refusal, or identifies an
    /// ordinal that was not present in the retained collection. The latter is
    /// an invariant boundary; it is not native null-pointer recovery.
    pub fn select_deferred_batch(
        &self,
        controls: DeferredCollectionBatchControls,
        active_index: u32,
    ) -> Result<
        PresentationDeferredBatchSelection,
        PresentationDeferredBatchSelectionError<PresentationDmoArenaCostUnavailable>,
    > {
        select_presentation_deferred_batch(
            controls,
            0,
            active_index,
            self.inner.arena_count,
            |ordinal| {
                usize::try_from(ordinal)
                    .ok()
                    .and_then(|ordinal| self.arena_cost(ordinal))
                    .ok_or(PresentationDmoArenaCostUnavailable { ordinal })
            },
        )
    }

    /// Returns the retained processed Arena cost at the authored collection
    /// ordinal. Native code reaches this value through the live collection
    /// asset, Arena asset, and `cArena::mCost`; the prepared owner keeps the
    /// equivalent lifetime without exposing host addresses.
    #[must_use]
    pub(crate) fn arena_cost(&self, ordinal: usize) -> Option<u32> {
        self.inner.arenas.get(ordinal).map(|arena| arena.cost)
    }

    #[must_use]
    pub(crate) fn arena_id(&self, ordinal: usize) -> Option<CollectionId> {
        self.inner.arenas.get(ordinal).map(|arena| arena.arena_id)
    }

    #[cfg(test)]
    pub(crate) fn from_parts_for_deferred_producer_test(
        arenas: &[PreparedPresentationDmoArena],
    ) -> Self {
        prepared_collection_for_test(arenas)
    }
}

/// Safe-host refusal when a deferred selector asks outside its retained Arena
/// owner. Native code asserts/debug-breaks on a null asset-data pointer and
/// then dereferences it instead of returning this error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationDmoArenaCostUnavailable {
    ordinal: u32,
}

impl PresentationDmoArenaCostUnavailable {
    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

impl fmt::Display for PresentationDmoArenaCostUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "retained Presentation Arena ordinal {} is unavailable",
            self.ordinal
        )
    }
}

impl Error for PresentationDmoArenaCostUnavailable {}

fn validate_main_dmo_bank_collection(
    arenas: &[Arc<PreparedPresentationDmoArenaInner>],
    operation_count: usize,
    material_operation_count: usize,
    dmo_operation_count: usize,
    arena_id_signature: u64,
) -> Result<(), PresentationDmoCollectionPrepareError> {
    let exact_counts = arenas.len() == MAIN_DMO_BANK_ARENA_COUNT
        && operation_count == MAIN_DMO_BANK_ACTION_COUNT
        && material_operation_count == MAIN_DMO_BANK_MATERIAL_COUNT
        && dmo_operation_count == MAIN_DMO_BANK_DMO_COUNT;
    let exact_ids = arenas.first().map(|arena| arena.arena_id.0)
        == Some(MAIN_DMO_BANK_FIRST_ARENA_ID)
        && arenas.last().map(|arena| arena.arena_id.0) == Some(MAIN_DMO_BANK_FINAL_ARENA_ID)
        && arena_id_signature == DMO_BANK_ASSET_ID_SIGNATURE
        && arenas.iter().enumerate().all(|(index, arena)| {
            arenas[..index]
                .iter()
                .all(|prior| prior.arena_id != arena.arena_id)
        });
    let leading_arenas_empty = arenas
        .get(..MAIN_DMO_BANK_ARENA_COUNT.saturating_sub(1))
        .is_some_and(|leading| leading.iter().all(|arena| arena.operations.is_empty()));
    let exact_final_actions = arenas.last().is_some_and(|arena| {
        let Some((first, rest)) = arena.operations.split_first() else {
            return false;
        };
        let PreparedPresentationOperation::Material(material) = first else {
            return false;
        };
        material.diagnostic_guid == MAIN_DMO_BANK_MATERIAL_GUID
            && material.source_toc_index == MAIN_DMO_BANK_MATERIAL_TOC_INDEX
            && rest.len() == MAIN_DMO_BANK_DMO_COUNT
            && rest.iter().all(|operation| {
                matches!(
                    operation,
                    PreparedPresentationOperation::Dmo(dmo)
                        if dmo.source_toc_index == MAIN_DMO_BANK_DMO_TOC_INDEX
                )
            })
    });
    let action_signature = main_dmo_bank_action_signature(arenas);
    if exact_counts
        && exact_ids
        && leading_arenas_empty
        && exact_final_actions
        && action_signature == Some(MAIN_DMO_BANK_ACTION_SIGNATURE)
    {
        Ok(())
    } else {
        Err(
            PresentationDmoCollectionPrepareError::MainDmoBankCensusMismatch {
                arena_count: arenas.len(),
                operation_count,
                material_operation_count,
                dmo_operation_count,
                arena_id_signature,
                action_signature,
            },
        )
    }
}

fn main_dmo_bank_action_signature(
    arenas: &[Arc<PreparedPresentationDmoArenaInner>],
) -> Option<u64> {
    let mut hash = FNV_OFFSET;
    hash_bytes(&mut hash, b"PRES_DMO_ACTION_STREAM_V1\0");
    for arena in arenas {
        let decoded = &arena.plan_owner.static_plan().asset().decoded;
        let toc_entry = decoded
            .arena
            .dictionary
            .iter()
            .find(|entry| entry.type_id == TABLE_OF_CONTENTS_TYPE_ID)?;
        for operation in &arena.operations {
            let (
                kind,
                source_toc_index,
                source_dictionary_index,
                source_inline_index,
                raw_object_type,
                raw_object_id,
                source_resource_index,
                source_start,
                source_end,
                first_payload,
                second_payload,
            ) = match operation {
                PreparedPresentationOperation::Material(source) => (
                    2_u32,
                    source.source_toc_index,
                    source.source_dictionary_index,
                    u32::MAX,
                    R_MATERIAL_DATA_TYPE_ID,
                    source.raw_object_id,
                    u32::try_from(source.locator.resource_index).ok()?,
                    u32::try_from(source.locator.object_start).ok()?,
                    u32::try_from(
                        source
                            .locator
                            .object_start
                            .checked_add(source.locator.object_len)?,
                    )
                    .ok()?,
                    source.diagnostic_guid,
                    None,
                ),
                PreparedPresentationOperation::Dmo(source) => {
                    let dictionary = decoded
                        .arena
                        .dictionary
                        .get(source.source_dictionary_index as usize)?;
                    let source_start = dictionary
                        .object_range
                        .start
                        .checked_add(source.source_record_offset as usize)?;
                    (
                        11,
                        source.source_toc_index,
                        source.source_dictionary_index,
                        source.source_inline_index,
                        INSTANCE_DATA_TYPE_ID,
                        source.raw_object_id,
                        u32::try_from(dictionary.resource_index).ok()?,
                        u32::try_from(source_start).ok()?,
                        u32::try_from(source_start.checked_add(DMO_RECORD_LEN)?).ok()?,
                        source.asset_id,
                        Some(source.local_guid),
                    )
                }
            };
            let toc_record_start = toc_entry
                .object_range
                .start
                .checked_add(20)?
                .checked_add((source_toc_index as usize).checked_mul(24)?)?;
            let toc_record_end = toc_record_start.checked_add(24)?;
            hash_bytes(&mut hash, b"A");
            hash_u32(&mut hash, kind);
            hash_u64(&mut hash, arena.arena_id.0);
            hash_u32(&mut hash, decoded.arena.id);
            hash_u32(&mut hash, source_toc_index);
            hash_u32(&mut hash, u32::try_from(toc_record_start).ok()?);
            hash_u32(&mut hash, u32::try_from(toc_record_end).ok()?);
            hash_u32(&mut hash, source_dictionary_index);
            hash_u32(&mut hash, source_inline_index);
            hash_u32(&mut hash, raw_object_type);
            hash_u32(&mut hash, raw_object_id);
            hash_u32(&mut hash, source_resource_index);
            hash_u32(&mut hash, source_start);
            hash_u32(&mut hash, source_end);
            hash_u64(&mut hash, first_payload);
            if let Some(second_payload) = second_payload {
                hash_u64(&mut hash, second_payload);
            }
        }
    }
    Some(hash)
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn hash_u32(hash: &mut u64, value: u32) {
    hash_bytes(hash, &value.to_be_bytes());
}

fn hash_u64(hash: &mut u64, value: u64) {
    hash_bytes(hash, &value.to_be_bytes());
}

#[cfg(test)]
fn prepared_collection_for_test(
    arenas: &[PreparedPresentationDmoArena],
) -> PreparedPresentationDmoCollection {
    let retained: Vec<_> = arenas
        .iter()
        .map(|arena| Arc::clone(&arena.inner))
        .collect();
    let operation_count = retained.iter().map(|arena| arena.operations.len()).sum();
    let material_operation_count = retained
        .iter()
        .flat_map(|arena| &arena.operations)
        .filter(|operation| matches!(operation, PreparedPresentationOperation::Material(_)))
        .count();
    let dmo_operation_count = operation_count - material_operation_count;
    PreparedPresentationDmoCollection {
        inner: Arc::new(PreparedPresentationDmoCollectionInner {
            arena_count: u32::try_from(retained.len()).unwrap(),
            arenas: retained,
            operation_count,
            material_operation_count,
            dmo_operation_count,
        }),
    }
}

#[derive(Debug)]
struct LiveCollectionRegistration {
    id: u64,
    prepared: Arc<PreparedPresentationDmoCollectionInner>,
    phase: LiveCollectionRegistrationPhase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveCollectionRegistrationPhase {
    Deferred { next_arena_index: usize },
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PresentationDmoResidentCollectionValidationError {
    ExtraArena,
    ArenaIdMismatch,
    ArenaOwnerMismatch,
}

impl fmt::Display for PresentationDmoResidentCollectionValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for PresentationDmoResidentCollectionValidationError {}

#[derive(Debug)]
struct FirstUseOwner {
    arena: Arc<PreparedPresentationDmoArenaInner>,
    registration_userdata_raw: u32,
}

/// Applies the Presentation receiver's arena coalescing around the certified
/// main-DMO-bank kind-2/kind-11 action stream.
///
/// `dispatch` is declared first so it is destroyed before every retained
/// source owner. A first-use owner is retained independently of its collection
/// occurrence: another collection may keep the arena count above zero after
/// the first collection is deregistered. `Drop` performs no implicit manager
/// calls.
pub struct PresentationDmoArenaRegistrationAdapter<D> {
    dispatch: D,
    ref_counter: ArenaRegistrationRefCounter,
    live: Vec<LiveCollectionRegistration>,
    first_use_owners: BTreeMap<u64, FirstUseOwner>,
    orphaned_owners: Vec<Arc<PreparedPresentationDmoArenaInner>>,
    domain: Arc<()>,
    next_registration_id: u64,
    fault: Option<PresentationDmoAdapterFault>,
}

impl<D: fmt::Debug> fmt::Debug for PresentationDmoArenaRegistrationAdapter<D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PresentationDmoArenaRegistrationAdapter")
            .field("dispatch", &self.dispatch)
            .field("live_collection_count", &self.live.len())
            .field("first_use_owner_count", &self.first_use_owners.len())
            .field("orphaned_owner_count", &self.orphaned_owners.len())
            .field("next_registration_id", &self.next_registration_id)
            .field("fault", &self.fault)
            .finish_non_exhaustive()
    }
}

impl<D: PresentationDmoArenaDispatch> PresentationDmoArenaRegistrationAdapter<D> {
    #[must_use]
    pub fn new(dispatch: D) -> Self {
        Self {
            dispatch,
            ref_counter: ArenaRegistrationRefCounter::default(),
            live: Vec::new(),
            first_use_owners: BTreeMap::new(),
            orphaned_owners: Vec::new(),
            domain: Arc::new(()),
            next_registration_id: 0,
            fault: None,
        }
    }

    /// Mirrors the collection receiver's forward per-arena `AddAsset` gate.
    ///
    /// This method is the activation-true path. The receiver's activation-zero
    /// early return and deferred activation-false completion message belong to
    /// the still-external message layer.
    ///
    /// The complete collection owner and non-cloneable result authority are
    /// installed before the first manager call. Existing arena IDs only gain
    /// a wrapping reference count; they do not dispatch again.
    ///
    /// # Errors
    ///
    /// Live-occurrence allocation or registration-ID exhaustion is rejected
    /// before any refcounter or manager effect. First-use map node allocation
    /// follows the process allocator's abort policy and is not a recoverable
    /// native failure. A private-ledger contradiction or dispatcher unwind
    /// permanently faults the adapter while retaining the applied prefix.
    ///
    pub fn register_collection(
        &mut self,
        prepared: &PreparedPresentationDmoCollection,
        userdata_raw: u32,
    ) -> Result<PresentationDmoCollectionRegistration, PresentationDmoRegistrationError> {
        if let Some(fault) = self.fault {
            return Err(PresentationDmoRegistrationError::AdapterFaulted(fault));
        }
        self.live.try_reserve(1).map_err(|_| {
            PresentationDmoRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self
            .next_registration_id
            .checked_add(1)
            .ok_or(PresentationDmoRegistrationError::RegistrationIdExhausted)?;
        let registration = PresentationDmoCollectionRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        };
        self.live.push(LiveCollectionRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            phase: LiveCollectionRegistrationPhase::Complete,
        });

        for (arena_index, arena) in prepared.inner.arenas.iter().enumerate() {
            if self.ref_counter.add_asset(arena.arena_id)
                == ArenaRegistrationAddTransition::FirstUse
            {
                match self.first_use_owners.entry(arena.arena_id.0) {
                    Entry::Vacant(entry) => {
                        entry.insert(FirstUseOwner {
                            arena: Arc::clone(arena),
                            registration_userdata_raw: userdata_raw,
                        });
                    }
                    Entry::Occupied(_) => {
                        let fault = PresentationDmoAdapterFault {
                            phase: PresentationDmoAdapterFaultPhase::Registration,
                            arena_index,
                            operation_index: None,
                            stage: PresentationDmoAdapterFaultStage::FirstUseOwnerInvariant,
                        };
                        self.fault = Some(fault);
                        return Err(PresentationDmoRegistrationError::InvariantFault(fault));
                    }
                }
                if let Err(fault) =
                    dispatch_registration(&mut self.dispatch, arena, arena_index, userdata_raw)
                {
                    self.fault = Some(fault);
                    return Err(PresentationDmoRegistrationError::DispatchPanicked(fault));
                }
            }
        }
        Ok(registration)
    }

    /// Installs one deferred collection occurrence before any Arena message is
    /// received.
    ///
    /// This is safe-host ownership bookkeeping for the producer/receiver gap;
    /// it performs no native refcounter, registry, or Presentation-manager
    /// effect. Arena effects begin only in [`Self::receive_deferred_arena`].
    ///
    /// # Errors
    ///
    /// Refuses a faulted adapter, live-occurrence allocation failure, or
    /// registration-ID exhaustion before installing an occurrence.
    pub fn begin_deferred_collection(
        &mut self,
        prepared: &PreparedPresentationDmoCollection,
    ) -> Result<PresentationDmoDeferredCollectionRegistration, PresentationDmoRegistrationError>
    {
        if let Some(fault) = self.fault {
            return Err(PresentationDmoRegistrationError::AdapterFaulted(fault));
        }
        self.live.try_reserve(1).map_err(|_| {
            PresentationDmoRegistrationError::LiveStorageAllocation {
                requested: self.live.len().saturating_add(1),
            }
        })?;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self
            .next_registration_id
            .checked_add(1)
            .ok_or(PresentationDmoRegistrationError::RegistrationIdExhausted)?;
        self.live.push(LiveCollectionRegistration {
            id: registration_id,
            prepared: Arc::clone(&prepared.inner),
            phase: LiveCollectionRegistrationPhase::Deferred {
                next_arena_index: 0,
            },
        });
        Ok(PresentationDmoDeferredCollectionRegistration {
            domain: Arc::downgrade(&self.domain),
            id: registration_id,
        })
    }

    /// Receives one deferred full-ID Arena message in collection order.
    ///
    /// The recovered receiver order is enforced inside this boundary:
    /// `AddAsset` first, no registry access on retained use, and a fresh
    /// full-ID Arena lookup before first-use dispatch. The lookup winner must
    /// be the exact processed owner retained at the message ordinal; equal IDs
    /// or bytes from a replaced owner are not accepted as pointer identity.
    ///
    /// A missing or invalid lookup occurs after the native refcount mutation.
    /// The safe host therefore faults with the applied prefix and never
    /// retries the same message, which would incorrectly perform `AddAsset`
    /// twice.
    ///
    /// # Errors
    ///
    /// Effect-free capability/order refusals leave the deferred occurrence
    /// usable. Any post-`AddAsset` lookup, ownership, ledger, or dispatcher
    /// failure permanently faults the adapter and retains the exact prefix.
    pub fn receive_deferred_arena(
        &mut self,
        registration: &PresentationDmoDeferredCollectionRegistration,
        ordinal: u32,
        arena_id: CollectionId,
        userdata_raw: u32,
        arena_lookup: &SimulationResidentArenaLookup<DmoBankProcessedArenaAsset>,
    ) -> Result<PresentationDmoDeferredArenaTransition, PresentationDmoDeferredArenaReceiveError>
    {
        let (live_index, next_arena_index, arena) =
            self.deferred_arena_for_message(registration, ordinal, arena_id)?;

        let transition = self.ref_counter.add_asset(arena_id);
        if transition == ArenaRegistrationAddTransition::Retained {
            self.live[live_index].phase = LiveCollectionRegistrationPhase::Deferred {
                next_arena_index: next_arena_index + 1,
            };
            return Ok(PresentationDmoDeferredArenaTransition::RetainedUse);
        }

        let owner_matches = match arena_lookup
            .try_with_asset::<_, Infallible>(arena.arena_id, |resolved| {
                Ok(std::ptr::eq(resolved.plan(), arena.plan_owner.as_ref()))
            }) {
            Ok(Some(owner_matches)) => owner_matches,
            Ok(None) => {
                return Err(self.fault_deferred_registration(
                    next_arena_index,
                    PresentationDmoAdapterFaultStage::ArenaLookupMissing,
                ));
            }
            Err(error) => {
                let stage = match error {
                    SimulationResidentArenaLookupError::RegistryDetached => {
                        PresentationDmoAdapterFaultStage::ArenaLookupRegistryDetached
                    }
                    SimulationResidentArenaLookupError::Poisoned => {
                        PresentationDmoAdapterFaultStage::ArenaLookupRegistryPoisoned
                    }
                    SimulationResidentArenaLookupError::PublishedKindMismatch { .. } => {
                        PresentationDmoAdapterFaultStage::ArenaLookupPublishedKindMismatch
                    }
                    SimulationResidentArenaLookupError::PublishedAssetExpired { .. } => {
                        PresentationDmoAdapterFaultStage::ArenaLookupPublishedAssetExpired
                    }
                    SimulationResidentArenaLookupError::Visitor(never) => match never {},
                };
                return Err(self.fault_deferred_registration(next_arena_index, stage));
            }
        };
        if !owner_matches {
            return Err(self.fault_deferred_registration(
                next_arena_index,
                PresentationDmoAdapterFaultStage::ArenaLookupOwnerMismatch,
            ));
        }
        match self.first_use_owners.entry(arena.arena_id.0) {
            Entry::Vacant(entry) => {
                entry.insert(FirstUseOwner {
                    arena: Arc::clone(&arena),
                    registration_userdata_raw: userdata_raw,
                });
            }
            Entry::Occupied(_) => {
                return Err(self.fault_deferred_registration(
                    next_arena_index,
                    PresentationDmoAdapterFaultStage::FirstUseOwnerInvariant,
                ));
            }
        }
        if let Err(fault) =
            dispatch_registration(&mut self.dispatch, &arena, next_arena_index, userdata_raw)
        {
            self.fault = Some(fault);
            return Err(PresentationDmoDeferredArenaReceiveError::DispatchPanicked(
                fault,
            ));
        }
        self.live[live_index].phase = LiveCollectionRegistrationPhase::Deferred {
            next_arena_index: next_arena_index + 1,
        };
        Ok(PresentationDmoDeferredArenaTransition::FirstUseDispatched)
    }

    fn deferred_arena_for_message(
        &self,
        registration: &PresentationDmoDeferredCollectionRegistration,
        ordinal: u32,
        arena_id: CollectionId,
    ) -> Result<
        (usize, usize, Arc<PreparedPresentationDmoArenaInner>),
        PresentationDmoDeferredArenaReceiveError,
    > {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(PresentationDmoDeferredArenaReceiveError::ForeignDomain);
        }
        if let Some(fault) = self.fault {
            return Err(PresentationDmoDeferredArenaReceiveError::AdapterFaulted(
                fault,
            ));
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(PresentationDmoDeferredArenaReceiveError::UnknownRegistration);
        };
        let next_arena_index = match self.live[live_index].phase {
            LiveCollectionRegistrationPhase::Deferred { next_arena_index } => next_arena_index,
            LiveCollectionRegistrationPhase::Complete => {
                return Err(PresentationDmoDeferredArenaReceiveError::RegistrationComplete);
            }
        };
        let actual_index = usize::try_from(ordinal).map_err(|_| {
            PresentationDmoDeferredArenaReceiveError::OrdinalDoesNotFitHost { ordinal }
        })?;
        if actual_index != next_arena_index {
            return Err(
                PresentationDmoDeferredArenaReceiveError::UnexpectedOrdinal {
                    expected: u32::try_from(next_arena_index).unwrap_or(u32::MAX),
                    actual: ordinal,
                },
            );
        }
        let Some(arena) = self.live[live_index]
            .prepared
            .arenas
            .get(next_arena_index)
            .cloned()
        else {
            return Err(
                PresentationDmoDeferredArenaReceiveError::CollectionComplete {
                    arena_count: self.live[live_index].prepared.arena_count,
                },
            );
        };
        if arena_id != arena.arena_id {
            return Err(
                PresentationDmoDeferredArenaReceiveError::UnexpectedArenaId {
                    expected: arena.arena_id,
                    actual: arena_id,
                },
            );
        }
        Ok((live_index, next_arena_index, arena))
    }

    fn fault_deferred_registration(
        &mut self,
        arena_index: usize,
        stage: PresentationDmoAdapterFaultStage,
    ) -> PresentationDmoDeferredArenaReceiveError {
        let fault = PresentationDmoAdapterFault {
            phase: PresentationDmoAdapterFaultPhase::Registration,
            arena_index,
            operation_index: None,
            stage,
        };
        self.fault = Some(fault);
        PresentationDmoDeferredArenaReceiveError::PostTransitionFault(fault)
    }

    /// Accepts the activation-zero terminal Collection marker after every
    /// deferred Arena message and converts its occurrence into ordinary live
    /// deregistration authority.
    ///
    /// The recovered activation-zero receiver returns before singleton lookup,
    /// Collection lookup, iteration, or refcounter mutation. Accordingly this
    /// method changes only safe-host capability state.
    ///
    /// # Errors
    ///
    /// Returns an owning, effect-free refusal for a foreign, faulted, unknown,
    /// already-finished, or incomplete deferred occurrence.
    pub fn finish_deferred_collection(
        &mut self,
        registration: PresentationDmoDeferredCollectionRegistration,
    ) -> Result<PresentationDmoCollectionRegistration, PresentationDmoDeferredFinishError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(PresentationDmoDeferredFinishError::new(
                PresentationDmoDeferredFinishErrorKind::ForeignDomain,
                registration,
                None,
            ));
        }
        if let Some(fault) = self.fault {
            return Err(PresentationDmoDeferredFinishError::new(
                PresentationDmoDeferredFinishErrorKind::AdapterFaulted,
                registration,
                Some(fault),
            ));
        }
        let Some(live) = self.live.iter_mut().find(|live| live.id == registration.id) else {
            return Err(PresentationDmoDeferredFinishError::new(
                PresentationDmoDeferredFinishErrorKind::UnknownRegistration,
                registration,
                None,
            ));
        };
        let LiveCollectionRegistrationPhase::Deferred { next_arena_index } = live.phase else {
            return Err(PresentationDmoDeferredFinishError::new(
                PresentationDmoDeferredFinishErrorKind::RegistrationComplete,
                registration,
                None,
            ));
        };
        if next_arena_index != live.prepared.arenas.len() {
            return Err(PresentationDmoDeferredFinishError::new(
                PresentationDmoDeferredFinishErrorKind::ArenasRemain {
                    next_ordinal: u32::try_from(next_arena_index).unwrap_or(u32::MAX),
                    arena_count: live.prepared.arena_count,
                },
                registration,
                None,
            ));
        }
        live.phase = LiveCollectionRegistrationPhase::Complete;
        Ok(PresentationDmoCollectionRegistration {
            domain: registration.domain,
            id: registration.id,
        })
    }

    /// Receives the main-bank `DRegCollection` front through one fresh
    /// full-ID Collection lookup before the existing forward `RemAsset` walk.
    ///
    /// Native `ReceiveDRegCollectionMsg` resolves the Collection once, then
    /// iterates its Arena pointer vector and uses those pointers directly;
    /// it does not perform an individual global Arena lookup after a last-use
    /// transition. This safe-host boundary verifies that the current resident
    /// occurrence contains the exact prepared Arena owners in authored order,
    /// then delegates the recovered per-Arena removal/dispatch sequence.
    ///
    /// The native Presentation singleton assertion precedes Collection lookup.
    /// A constructed adapter/dispatcher represents the supported non-null
    /// singleton path; the null path remains outside this boundary.
    ///
    /// # Errors
    ///
    /// Capability, lookup, liveness, and owner-sequence refusals occur before
    /// any `RemAsset` and return the registration unchanged. Once the removal
    /// walk begins, [`Self::deregister_collection`] retains its existing
    /// prefix-preserving failure semantics.
    pub fn deregister_collection_after_resident_lookup(
        &mut self,
        collection_id: CollectionId,
        registration: PresentationDmoCollectionRegistration,
        userdata_raw: u32,
        lookup: &SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    ) -> Result<(), PresentationDmoDeregistrationError> {
        let prepared =
            match self.preflight_deregistration_lookup_authority(collection_id, &registration) {
                Ok(prepared) => prepared,
                Err(kind) => {
                    let fault = if kind == PresentationDmoDeregistrationErrorKind::AdapterFaulted {
                        self.fault
                    } else {
                        None
                    };
                    return Err(PresentationDmoDeregistrationError {
                        kind,
                        registration: Some(registration),
                        fault,
                    });
                }
            };
        let occurrence = match lookup.lookup(collection_id) {
            Ok(Some(occurrence)) => occurrence,
            Ok(None) => {
                return Err(PresentationDmoDeregistrationError {
                    kind: PresentationDmoDeregistrationErrorKind::CollectionLookupMissing,
                    registration: Some(registration),
                    fault: None,
                });
            }
            Err(error) => {
                let kind = match error {
                    SimulationResidentLookupError::RegistryDetached => {
                        PresentationDmoDeregistrationErrorKind::CollectionLookupRegistryDetached
                    }
                    SimulationResidentLookupError::Poisoned => {
                        PresentationDmoDeregistrationErrorKind::CollectionLookupRegistryPoisoned
                    }
                    SimulationResidentLookupError::PublishedKindMismatch { .. } => {
                        PresentationDmoDeregistrationErrorKind::CollectionLookupKindMismatch
                    }
                };
                return Err(PresentationDmoDeregistrationError {
                    kind,
                    registration: Some(registration),
                    fault: None,
                });
            }
        };
        let mut arena_index = 0_usize;
        let validation = occurrence.try_for_each(|asset| {
            let expected = prepared
                .arenas
                .get(arena_index)
                .ok_or(PresentationDmoResidentCollectionValidationError::ExtraArena)?;
            let actual_id = CollectionId(asset.plan().static_plan().asset().decoded.asset_id);
            if actual_id != expected.arena_id {
                return Err(PresentationDmoResidentCollectionValidationError::ArenaIdMismatch);
            }
            if !std::ptr::eq(asset.plan(), expected.plan_owner.as_ref()) {
                return Err(PresentationDmoResidentCollectionValidationError::ArenaOwnerMismatch);
            }
            arena_index += 1;
            Ok(())
        });
        if let Err(error) = validation {
            let kind = map_deregistration_collection_visit_error(error);
            return Err(PresentationDmoDeregistrationError {
                kind,
                registration: Some(registration),
                fault: None,
            });
        }
        if arena_index != prepared.arenas.len() {
            return Err(PresentationDmoDeregistrationError {
                kind: PresentationDmoDeregistrationErrorKind::CollectionArenaCountMismatch,
                registration: Some(registration),
                fault: None,
            });
        }
        self.deregister_collection(registration, userdata_raw)
    }

    fn preflight_deregistration_lookup_authority(
        &self,
        collection_id: CollectionId,
        registration: &PresentationDmoCollectionRegistration,
    ) -> Result<Arc<PreparedPresentationDmoCollectionInner>, PresentationDmoDeregistrationErrorKind>
    {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(PresentationDmoDeregistrationErrorKind::ForeignDomain);
        }
        if self.fault.is_some() {
            return Err(PresentationDmoDeregistrationErrorKind::AdapterFaulted);
        }
        if collection_id != CollectionId(DMO_BANK_COLLECTION_ID) {
            return Err(PresentationDmoDeregistrationErrorKind::UnexpectedCollectionId);
        }
        let live = self
            .live
            .iter()
            .find(|live| live.id == registration.id)
            .ok_or(PresentationDmoDeregistrationErrorKind::UnknownRegistration)?;
        if live.phase != LiveCollectionRegistrationPhase::Complete {
            return Err(PresentationDmoDeregistrationErrorKind::IncompleteRegistration);
        }
        Ok(Arc::clone(&live.prepared))
    }

    /// Mirrors the collection receiver's forward per-arena `RemAsset` gate.
    ///
    /// Deregistration samples its fresh userdata independently. Only an arena
    /// whose old count was one dispatches. If first-use registration enabled
    /// DMO publication but last-use deregistration disables it, or supplies a
    /// different backing record which the native pointer-identity check cannot
    /// erase, the original source owner is retained until adapter destruction
    /// rather than creating a host dangling pointer.
    ///
    /// # Errors
    ///
    /// Foreign, already-consumed, and pre-fault authorities are returned
    /// unchanged and have no effects. Required asymmetric-owner storage is
    /// reserved before the first refcounter or manager effect. A private-ledger
    /// contradiction or dispatcher unwind permanently faults the adapter,
    /// consumes the in-flight authority, retains the partial prefix, and does
    /// not invent rollback.
    pub fn deregister_collection(
        &mut self,
        registration: PresentationDmoCollectionRegistration,
        userdata_raw: u32,
    ) -> Result<(), PresentationDmoDeregistrationError> {
        if !Weak::ptr_eq(&registration.domain, &Arc::downgrade(&self.domain)) {
            return Err(PresentationDmoDeregistrationError {
                kind: PresentationDmoDeregistrationErrorKind::ForeignDomain,
                registration: Some(registration),
                fault: None,
            });
        }
        if self.fault.is_some() {
            return Err(PresentationDmoDeregistrationError {
                kind: PresentationDmoDeregistrationErrorKind::AdapterFaulted,
                registration: Some(registration),
                fault: self.fault,
            });
        }
        let Some(live_index) = self.live.iter().position(|live| live.id == registration.id) else {
            return Err(PresentationDmoDeregistrationError {
                kind: PresentationDmoDeregistrationErrorKind::UnknownRegistration,
                registration: Some(registration),
                fault: None,
            });
        };
        let prepared = Arc::clone(&self.live[live_index].prepared);
        let required_orphans = match self.preflight_deregistration(&prepared, userdata_raw) {
            Ok(required) => required,
            Err(fault) => {
                self.fault = Some(fault);
                return Err(PresentationDmoDeregistrationError::consumed_fault(fault));
            }
        };
        self.orphaned_owners
            .try_reserve(required_orphans)
            .map_err(|_| PresentationDmoDeregistrationError {
                kind: PresentationDmoDeregistrationErrorKind::OrphanStorageAllocation,
                registration: Some(registration),
                fault: None,
            })?;

        if let Err(fault) = self.apply_deregistration(&prepared, userdata_raw) {
            self.fault = Some(fault);
            return Err(PresentationDmoDeregistrationError::consumed_fault(fault));
        }
        self.live.swap_remove(live_index);
        Ok(())
    }

    fn preflight_deregistration(
        &self,
        prepared: &PreparedPresentationDmoCollectionInner,
        userdata_raw: u32,
    ) -> Result<usize, PresentationDmoAdapterFault> {
        let mut required_orphans = 0_usize;
        for (arena_index, arena) in prepared.arenas.iter().enumerate() {
            let Some(is_last_use) = removal_is_last_use(&self.ref_counter, prepared, arena_index)
            else {
                return Err(deregistration_invariant_fault(
                    arena_index,
                    PresentationDmoAdapterFaultStage::RefCounterInvariant,
                ));
            };
            if !is_last_use {
                continue;
            }
            let owner = self
                .first_use_owners
                .get(&arena.arena_id.0)
                .ok_or_else(|| {
                    deregistration_invariant_fault(
                        arena_index,
                        PresentationDmoAdapterFaultStage::FirstUseOwnerInvariant,
                    )
                })?;
            if requires_orphan(owner, arena, userdata_raw) {
                required_orphans = required_orphans.saturating_add(1);
            }
        }
        Ok(required_orphans)
    }

    fn apply_deregistration(
        &mut self,
        prepared: &PreparedPresentationDmoCollectionInner,
        userdata_raw: u32,
    ) -> Result<(), PresentationDmoAdapterFault> {
        for (arena_index, arena) in prepared.arenas.iter().enumerate() {
            let transition = self.ref_counter.remove_asset(arena.arena_id).map_err(|_| {
                deregistration_invariant_fault(
                    arena_index,
                    PresentationDmoAdapterFaultStage::RefCounterInvariant,
                )
            })?;
            if transition != ArenaRegistrationRemoveTransition::LastUse {
                continue;
            }
            dispatch_deregistration(&mut self.dispatch, arena, arena_index, userdata_raw)?;
            let owner = self
                .first_use_owners
                .remove(&arena.arena_id.0)
                .ok_or_else(|| {
                    deregistration_invariant_fault(
                        arena_index,
                        PresentationDmoAdapterFaultStage::FirstUseOwnerInvariant,
                    )
                })?;
            if requires_orphan(&owner, arena, userdata_raw) {
                self.orphaned_owners.push(owner.arena);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn arena_ref_count(&self, id: CollectionId) -> Option<u32> {
        self.ref_counter.count(id)
    }

    #[must_use]
    pub fn live_collection_count(&self) -> usize {
        self.live.len()
    }

    #[must_use]
    pub fn first_use_owner_count(&self) -> usize {
        self.first_use_owners.len()
    }

    #[must_use]
    pub fn orphaned_owner_count(&self) -> usize {
        self.orphaned_owners.len()
    }

    #[must_use]
    pub const fn fault(&self) -> Option<PresentationDmoAdapterFault> {
        self.fault
    }

    #[must_use]
    pub const fn dispatch(&self) -> &D {
        &self.dispatch
    }
}

fn dispatch_registration<D: PresentationDmoArenaDispatch>(
    dispatch: &mut D,
    arena: &PreparedPresentationDmoArenaInner,
    arena_index: usize,
    userdata_raw: u32,
) -> Result<(), PresentationDmoAdapterFault> {
    for (operation_index, operation) in arena.operations.iter().enumerate() {
        match operation {
            PreparedPresentationOperation::Material(source) => {
                if catch_unwind(AssertUnwindSafe(|| dispatch.add_material(source))).is_err() {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Registration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::Material,
                    });
                }
            }
            PreparedPresentationOperation::Dmo(operation) if dmo_enabled(userdata_raw) => {
                if catch_unwind(AssertUnwindSafe(|| {
                    dispatch.add_living_world_dmo_model(operation.asset_id);
                }))
                .is_err()
                {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Registration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::LivingWorld,
                    });
                }
                if catch_unwind(AssertUnwindSafe(|| {
                    dispatch.register_presentation_dmo_model(&operation.source, operation.asset_id);
                }))
                .is_err()
                {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Registration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::Dmo,
                    });
                }
            }
            PreparedPresentationOperation::Dmo(_) => {}
        }
    }
    Ok(())
}

fn same_presentation_sources(
    first: &PreparedPresentationDmoArenaInner,
    last: &PreparedPresentationDmoArenaInner,
) -> bool {
    first.operations.len() == last.operations.len()
        && first
            .operations
            .iter()
            .zip(&last.operations)
            .all(|(first, last)| match (first, last) {
                (
                    PreparedPresentationOperation::Material(first),
                    PreparedPresentationOperation::Material(last),
                ) => {
                    first.diagnostic_guid == last.diagnostic_guid
                        && first.is_same_backing_object(last)
                }
                (
                    PreparedPresentationOperation::Dmo(first),
                    PreparedPresentationOperation::Dmo(last),
                ) => {
                    first.asset_id == last.asset_id
                        && first.local_guid == last.local_guid
                        && first.source.is_same_backing_record(&last.source)
                }
                _ => false,
            })
}

fn requires_orphan(
    first: &FirstUseOwner,
    last: &PreparedPresentationDmoArenaInner,
    deregistration_userdata_raw: u32,
) -> bool {
    if first.arena.operations.is_empty() {
        return false;
    }
    if !same_presentation_sources(&first.arena, last) {
        return true;
    }
    dmo_enabled(first.registration_userdata_raw)
        && !dmo_enabled(deregistration_userdata_raw)
        && first
            .arena
            .operations
            .iter()
            .any(|operation| matches!(operation, PreparedPresentationOperation::Dmo(_)))
}

fn dispatch_deregistration<D: PresentationDmoArenaDispatch>(
    dispatch: &mut D,
    arena: &PreparedPresentationDmoArenaInner,
    arena_index: usize,
    userdata_raw: u32,
) -> Result<(), PresentationDmoAdapterFault> {
    for (operation_index, operation) in arena.operations.iter().enumerate() {
        match operation {
            PreparedPresentationOperation::Material(source) => {
                if catch_unwind(AssertUnwindSafe(|| dispatch.remove_material(source))).is_err() {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Deregistration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::Material,
                    });
                }
            }
            PreparedPresentationOperation::Dmo(operation) if dmo_enabled(userdata_raw) => {
                if catch_unwind(AssertUnwindSafe(|| {
                    dispatch.remove_living_world_dmo_model(operation.asset_id);
                }))
                .is_err()
                {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Deregistration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::LivingWorld,
                    });
                }
                if catch_unwind(AssertUnwindSafe(|| {
                    dispatch
                        .deregister_presentation_dmo_model(&operation.source, operation.asset_id);
                }))
                .is_err()
                {
                    return Err(PresentationDmoAdapterFault {
                        phase: PresentationDmoAdapterFaultPhase::Deregistration,
                        arena_index,
                        operation_index: Some(operation_index),
                        stage: PresentationDmoAdapterFaultStage::Dmo,
                    });
                }
            }
            PreparedPresentationOperation::Dmo(_) => {}
        }
    }
    Ok(())
}

fn removal_is_last_use(
    ref_counter: &ArenaRegistrationRefCounter,
    prepared: &PreparedPresentationDmoCollectionInner,
    arena_index: usize,
) -> Option<bool> {
    let arena = prepared.arenas.get(arena_index)?;
    let mut count = ref_counter.count(arena.arena_id)?;
    for prior in &prepared.arenas[..arena_index] {
        if prior.arena_id == arena.arena_id {
            if count == 1 {
                return None;
            }
            count = count.wrapping_sub(1);
        }
    }
    Some(count == 1)
}

const fn deregistration_invariant_fault(
    arena_index: usize,
    stage: PresentationDmoAdapterFaultStage,
) -> PresentationDmoAdapterFault {
    PresentationDmoAdapterFault {
        phase: PresentationDmoAdapterFaultPhase::Deregistration,
        arena_index,
        operation_index: None,
        stage,
    }
}

fn map_deregistration_collection_visit_error(
    error: SimulationResidentVisitError<PresentationDmoResidentCollectionValidationError>,
) -> PresentationDmoDeregistrationErrorKind {
    match error {
        SimulationResidentVisitError::RegistryDetached => {
            PresentationDmoDeregistrationErrorKind::CollectionVisitRegistryDetached
        }
        SimulationResidentVisitError::Poisoned => {
            PresentationDmoDeregistrationErrorKind::CollectionVisitRegistryPoisoned
        }
        SimulationResidentVisitError::UnknownOccurrence { .. } => {
            PresentationDmoDeregistrationErrorKind::CollectionOccurrenceStale
        }
        SimulationResidentVisitError::Assets(source) => match source {
            OwnedResidentAssetsVisitError::Detached => {
                PresentationDmoDeregistrationErrorKind::CollectionAssetsDetached
            }
            OwnedResidentAssetsVisitError::NotAlive => {
                PresentationDmoDeregistrationErrorKind::CollectionAssetsNotAlive
            }
            OwnedResidentAssetsVisitError::Poisoned => {
                PresentationDmoDeregistrationErrorKind::CollectionAssetsPoisoned
            }
            OwnedResidentAssetsVisitError::SnapshotReserveFailed { .. } => {
                PresentationDmoDeregistrationErrorKind::CollectionAssetsSnapshotAllocation
            }
            OwnedResidentAssetsVisitError::Visitor(
                PresentationDmoResidentCollectionValidationError::ExtraArena,
            ) => PresentationDmoDeregistrationErrorKind::CollectionArenaCountMismatch,
            OwnedResidentAssetsVisitError::Visitor(
                PresentationDmoResidentCollectionValidationError::ArenaIdMismatch,
            ) => PresentationDmoDeregistrationErrorKind::CollectionArenaIdMismatch,
            OwnedResidentAssetsVisitError::Visitor(
                PresentationDmoResidentCollectionValidationError::ArenaOwnerMismatch,
            ) => PresentationDmoDeregistrationErrorKind::CollectionArenaOwnerMismatch,
        },
    }
}

const fn dmo_enabled(userdata_raw: u32) -> bool {
    userdata_raw & DMO_USERDATA_MASK == DMO_USERDATA_MASK
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_be_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// Non-cloneable authority for one live Presentation collection occurrence.
pub struct PresentationDmoCollectionRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for PresentationDmoCollectionRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PresentationDmoCollectionRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Non-cloneable authority for one collection whose deferred Arena messages
/// have not all reached the Presentation receiver.
pub struct PresentationDmoDeferredCollectionRegistration {
    domain: Weak<()>,
    id: u64,
}

impl fmt::Debug for PresentationDmoDeferredCollectionRegistration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PresentationDmoDeferredCollectionRegistration")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

/// Observable branch taken by one successfully received deferred Arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoDeferredArenaTransition {
    /// `AddAsset` found a nonzero old count; lookup and dispatch were skipped.
    RetainedUse,
    /// `AddAsset` found zero, fresh lookup matched, and dispatch completed.
    FirstUseDispatched,
}

/// Refusal while receiving one deferred Arena marker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoDeferredArenaReceiveError {
    ForeignDomain,
    AdapterFaulted(PresentationDmoAdapterFault),
    UnknownRegistration,
    RegistrationComplete,
    OrdinalDoesNotFitHost {
        ordinal: u32,
    },
    UnexpectedOrdinal {
        expected: u32,
        actual: u32,
    },
    UnexpectedArenaId {
        expected: CollectionId,
        actual: CollectionId,
    },
    CollectionComplete {
        arena_count: u32,
    },
    PostTransitionFault(PresentationDmoAdapterFault),
    DispatchPanicked(PresentationDmoAdapterFault),
}

impl fmt::Display for PresentationDmoDeferredArenaReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForeignDomain => formatter.write_str(
                "deferred collection belongs to another main-DMO-bank Presentation adapter",
            ),
            Self::AdapterFaulted(fault) => write!(
                formatter,
                "main-DMO-bank Presentation adapter is faulted: {fault:?}"
            ),
            Self::UnknownRegistration => formatter.write_str(
                "deferred collection is not live in this main-DMO-bank Presentation adapter",
            ),
            Self::RegistrationComplete => {
                formatter.write_str("Presentation collection registration is already complete")
            }
            Self::OrdinalDoesNotFitHost { ordinal } => write!(
                formatter,
                "deferred Presentation Arena ordinal {ordinal} does not fit the host index"
            ),
            Self::UnexpectedOrdinal { expected, actual } => write!(
                formatter,
                "deferred Presentation Arena ordinal {actual} arrived while {expected} was expected"
            ),
            Self::UnexpectedArenaId { expected, actual } => write!(
                formatter,
                "deferred Presentation Arena ID {:#018x} arrived while {:#018x} was expected",
                actual.0, expected.0
            ),
            Self::CollectionComplete { arena_count } => write!(
                formatter,
                "deferred Presentation collection already received all {arena_count} Arenas"
            ),
            Self::PostTransitionFault(fault) => write!(
                formatter,
                "deferred Presentation Arena failed after AddAsset: {fault:?}"
            ),
            Self::DispatchPanicked(fault) => write!(
                formatter,
                "deferred Presentation Arena dispatcher panicked: {fault:?}"
            ),
        }
    }
}

impl Error for PresentationDmoDeferredArenaReceiveError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoDeferredFinishErrorKind {
    ForeignDomain,
    AdapterFaulted,
    UnknownRegistration,
    RegistrationComplete,
    ArenasRemain { next_ordinal: u32, arena_count: u32 },
}

/// Owning effect-free refusal to accept an activation-zero terminal marker.
#[derive(Debug)]
pub struct PresentationDmoDeferredFinishError {
    kind: PresentationDmoDeferredFinishErrorKind,
    registration: PresentationDmoDeferredCollectionRegistration,
    fault: Option<PresentationDmoAdapterFault>,
}

impl PresentationDmoDeferredFinishError {
    const fn new(
        kind: PresentationDmoDeferredFinishErrorKind,
        registration: PresentationDmoDeferredCollectionRegistration,
        fault: Option<PresentationDmoAdapterFault>,
    ) -> Self {
        Self {
            kind,
            registration,
            fault,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> PresentationDmoDeferredFinishErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn fault(&self) -> Option<PresentationDmoAdapterFault> {
        self.fault
    }

    #[must_use]
    pub fn into_registration(self) -> PresentationDmoDeferredCollectionRegistration {
        self.registration
    }
}

impl fmt::Display for PresentationDmoDeferredFinishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PresentationDmoDeferredFinishErrorKind::ForeignDomain => formatter.write_str(
                "deferred collection belongs to another main-DMO-bank Presentation adapter",
            ),
            PresentationDmoDeferredFinishErrorKind::AdapterFaulted => {
                formatter.write_str("main-DMO-bank Presentation adapter is faulted")
            }
            PresentationDmoDeferredFinishErrorKind::UnknownRegistration => formatter.write_str(
                "deferred collection is not live in this main-DMO-bank Presentation adapter",
            ),
            PresentationDmoDeferredFinishErrorKind::RegistrationComplete => {
                formatter.write_str("Presentation collection registration is already complete")
            }
            PresentationDmoDeferredFinishErrorKind::ArenasRemain {
                next_ordinal,
                arena_count,
            } => write!(
                formatter,
                "deferred Presentation collection expects Arena {next_ordinal} of {arena_count} before its terminal marker"
            ),
        }
    }
}

impl Error for PresentationDmoDeferredFinishError {}

/// Host fail-stop location after a private invariant failure or caught unwind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationDmoAdapterFault {
    phase: PresentationDmoAdapterFaultPhase,
    arena_index: usize,
    operation_index: Option<usize>,
    stage: PresentationDmoAdapterFaultStage,
}

impl PresentationDmoAdapterFault {
    #[must_use]
    pub const fn phase(self) -> PresentationDmoAdapterFaultPhase {
        self.phase
    }

    #[must_use]
    pub const fn arena_index(self) -> usize {
        self.arena_index
    }

    #[must_use]
    pub const fn operation_index(self) -> Option<usize> {
        self.operation_index
    }

    #[must_use]
    pub const fn stage(self) -> PresentationDmoAdapterFaultStage {
        self.stage
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoAdapterFaultPhase {
    Registration,
    Deregistration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoAdapterFaultStage {
    FirstUseOwnerInvariant,
    RefCounterInvariant,
    ArenaLookupRegistryDetached,
    ArenaLookupRegistryPoisoned,
    ArenaLookupPublishedKindMismatch,
    ArenaLookupPublishedAssetExpired,
    ArenaLookupMissing,
    ArenaLookupOwnerMismatch,
    Material,
    LivingWorld,
    Dmo,
}

#[derive(Debug)]
pub enum PresentationDmoArenaPrepareError {
    MissingTocWithDmoOperations,
    InvalidToc(PegasusError),
    TocIndexOverflow(usize),
    OperationStorageAllocation {
        requested: usize,
    },
    NonLocalMaterialObject {
        source_toc_index: u32,
        raw_object_id: u32,
        section: u16,
    },
    MissingMaterialObject {
        source_toc_index: u32,
        raw_object_id: u32,
        source_dictionary_index: u32,
    },
    WrongMaterialObjectType {
        source_toc_index: u32,
        source_dictionary_index: u32,
        actual: u32,
    },
    ObjectRangeOutsideResource {
        dictionary_index: u32,
        type_id: u32,
    },
    InvalidMaterial(MaterialError),
    DmoOrderMismatch,
    MissingRecordSource {
        normal_operation_index: usize,
    },
    WrongRecordKind {
        normal_operation_index: usize,
        actual: SimulationRecordSourceKind,
    },
    WrongRecordLength {
        normal_operation_index: usize,
        actual: usize,
    },
    RecordIdentityMismatch {
        normal_operation_index: usize,
    },
}

impl fmt::Display for PresentationDmoArenaPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTocWithDmoOperations => formatter.write_str(
                "normal Simulation plan contains DMO operations but its retained arena has no TOC",
            ),
            Self::InvalidToc(source) => write!(
                formatter,
                "main-DMO-bank Presentation TOC is invalid: {source}"
            ),
            Self::TocIndexOverflow(index) => {
                write!(formatter, "Presentation TOC entry index {index} does not fit in u32")
            }
            Self::OperationStorageAllocation { requested } => write!(
                formatter,
                "could not allocate {requested} main-DMO-bank Presentation operations"
            ),
            Self::NonLocalMaterialObject {
                source_toc_index,
                raw_object_id,
                section,
            } => write!(
                formatter,
                "Presentation material TOC entry {source_toc_index} uses nonlocal object {raw_object_id:08x} in section {section}"
            ),
            Self::MissingMaterialObject {
                source_toc_index,
                raw_object_id,
                source_dictionary_index,
            } => write!(
                formatter,
                "Presentation material TOC entry {source_toc_index} object {raw_object_id:08x} targets missing dictionary entry {source_dictionary_index}"
            ),
            Self::WrongMaterialObjectType {
                source_toc_index,
                source_dictionary_index,
                actual,
            } => write!(
                formatter,
                "Presentation material TOC entry {source_toc_index} targets dictionary entry {source_dictionary_index} of type {actual:08x}, expected {R_MATERIAL_DATA_TYPE_ID:08x}"
            ),
            Self::ObjectRangeOutsideResource {
                dictionary_index,
                type_id,
            } => write!(
                formatter,
                "Presentation dictionary entry {dictionary_index} type {type_id:08x} is outside its retained resource"
            ),
            Self::InvalidMaterial(source) => {
                write!(formatter, "Presentation material data is invalid: {source}")
            }
            Self::DmoOrderMismatch => formatter.write_str(
                "normal Simulation DMO operations do not monotonically match the retained first TOC",
            ),
            Self::MissingRecordSource {
                normal_operation_index,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has no retained DMO source"
            ),
            Self::WrongRecordKind {
                normal_operation_index,
                actual,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has source kind {actual:?}, expected DmoInstance"
            ),
            Self::WrongRecordLength {
                normal_operation_index,
                actual,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} has {actual} DMO bytes, expected {DMO_RECORD_LEN}"
            ),
            Self::RecordIdentityMismatch {
                normal_operation_index,
            } => write!(
                formatter,
                "normal Simulation operation {normal_operation_index} disagrees with its retained DMO IDs"
            ),
        }
    }
}

impl Error for PresentationDmoArenaPrepareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidToc(source) => Some(source),
            Self::InvalidMaterial(source) => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoCollectionPrepareError {
    ArenaCountOverflow {
        actual: usize,
    },
    ArenaStorageAllocation {
        requested: usize,
    },
    OperationCountOverflow,
    MainDmoBankCensusMismatch {
        arena_count: usize,
        operation_count: usize,
        material_operation_count: usize,
        dmo_operation_count: usize,
        arena_id_signature: u64,
        action_signature: Option<u64>,
    },
}

impl fmt::Display for PresentationDmoCollectionPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArenaCountOverflow { actual } => write!(
                formatter,
                "Presentation collection Arena count {actual} does not fit in u32"
            ),
            Self::ArenaStorageAllocation { requested } => write!(
                formatter,
                "could not retain {requested} prepared main-DMO-bank Presentation arenas"
            ),
            Self::OperationCountOverflow => {
                formatter.write_str("main-DMO-bank Presentation operation count overflowed")
            }
            Self::MainDmoBankCensusMismatch {
                arena_count,
                operation_count,
                material_operation_count,
                dmo_operation_count,
                arena_id_signature,
                action_signature,
            } => write!(
                formatter,
                "main-DMO-bank Presentation census differs: {arena_count} arenas, {operation_count} actions ({material_operation_count} material, {dmo_operation_count} DMO), arena-ID signature {arena_id_signature:016x}, action signature {action_signature:?}"
            ),
        }
    }
}

impl Error for PresentationDmoCollectionPrepareError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoRegistrationError {
    AdapterFaulted(PresentationDmoAdapterFault),
    LiveStorageAllocation { requested: usize },
    RegistrationIdExhausted,
    InvariantFault(PresentationDmoAdapterFault),
    DispatchPanicked(PresentationDmoAdapterFault),
}

impl fmt::Display for PresentationDmoRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdapterFaulted(fault) => {
                write!(
                    formatter,
                    "main-DMO-bank Presentation adapter is faulted: {fault:?}"
                )
            }
            Self::LiveStorageAllocation { requested } => write!(
                formatter,
                "could not allocate main-DMO-bank Presentation collection registration {requested}"
            ),
            Self::RegistrationIdExhausted => {
                formatter.write_str("main-DMO-bank Presentation registration IDs are exhausted")
            }
            Self::InvariantFault(fault) => write!(
                formatter,
                "main-DMO-bank Presentation registration hit a private invariant fault: {fault:?}"
            ),
            Self::DispatchPanicked(fault) => write!(
                formatter,
                "main-DMO-bank Presentation registration dispatcher panicked: {fault:?}"
            ),
        }
    }
}

impl Error for PresentationDmoRegistrationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PresentationDmoDeregistrationErrorKind {
    ForeignDomain,
    AdapterFaulted,
    UnknownRegistration,
    IncompleteRegistration,
    UnexpectedCollectionId,
    CollectionLookupRegistryDetached,
    CollectionLookupRegistryPoisoned,
    CollectionLookupKindMismatch,
    CollectionLookupMissing,
    CollectionVisitRegistryDetached,
    CollectionVisitRegistryPoisoned,
    CollectionOccurrenceStale,
    CollectionAssetsDetached,
    CollectionAssetsNotAlive,
    CollectionAssetsPoisoned,
    CollectionAssetsSnapshotAllocation,
    CollectionArenaCountMismatch,
    CollectionArenaIdMismatch,
    CollectionArenaOwnerMismatch,
    OrphanStorageAllocation,
    InvariantFault,
    DispatchPanicked,
}

#[derive(Debug)]
pub struct PresentationDmoDeregistrationError {
    kind: PresentationDmoDeregistrationErrorKind,
    registration: Option<PresentationDmoCollectionRegistration>,
    fault: Option<PresentationDmoAdapterFault>,
}

impl PresentationDmoDeregistrationError {
    fn consumed_fault(fault: PresentationDmoAdapterFault) -> Self {
        let kind = match fault.stage {
            PresentationDmoAdapterFaultStage::Material
            | PresentationDmoAdapterFaultStage::LivingWorld
            | PresentationDmoAdapterFaultStage::Dmo => {
                PresentationDmoDeregistrationErrorKind::DispatchPanicked
            }
            PresentationDmoAdapterFaultStage::FirstUseOwnerInvariant
            | PresentationDmoAdapterFaultStage::RefCounterInvariant
            | PresentationDmoAdapterFaultStage::ArenaLookupRegistryDetached
            | PresentationDmoAdapterFaultStage::ArenaLookupRegistryPoisoned
            | PresentationDmoAdapterFaultStage::ArenaLookupPublishedKindMismatch
            | PresentationDmoAdapterFaultStage::ArenaLookupPublishedAssetExpired
            | PresentationDmoAdapterFaultStage::ArenaLookupMissing
            | PresentationDmoAdapterFaultStage::ArenaLookupOwnerMismatch => {
                PresentationDmoDeregistrationErrorKind::InvariantFault
            }
        };
        Self {
            kind,
            registration: None,
            fault: Some(fault),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> PresentationDmoDeregistrationErrorKind {
        self.kind
    }

    #[must_use]
    pub fn into_registration(self) -> Option<PresentationDmoCollectionRegistration> {
        self.registration
    }

    #[must_use]
    pub const fn fault(&self) -> Option<PresentationDmoAdapterFault> {
        self.fault
    }
}

impl fmt::Display for PresentationDmoDeregistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            PresentationDmoDeregistrationErrorKind::ForeignDomain => {
                "collection registration belongs to another main-DMO-bank Presentation adapter"
            }
            PresentationDmoDeregistrationErrorKind::AdapterFaulted => {
                "main-DMO-bank Presentation adapter is faulted"
            }
            PresentationDmoDeregistrationErrorKind::UnknownRegistration => {
                "collection registration is not live in this main-DMO-bank Presentation adapter"
            }
            PresentationDmoDeregistrationErrorKind::IncompleteRegistration => {
                "deferred Presentation collection is not complete"
            }
            PresentationDmoDeregistrationErrorKind::UnexpectedCollectionId => {
                "Presentation DRegCollection message does not name the main DMO bank"
            }
            PresentationDmoDeregistrationErrorKind::CollectionLookupRegistryDetached => {
                "Presentation Collection lookup registry is detached"
            }
            PresentationDmoDeregistrationErrorKind::CollectionLookupRegistryPoisoned => {
                "Presentation Collection lookup registry is poisoned"
            }
            PresentationDmoDeregistrationErrorKind::CollectionLookupKindMismatch => {
                "Presentation Collection ID resolves to a non-Collection asset"
            }
            PresentationDmoDeregistrationErrorKind::CollectionLookupMissing => {
                "Presentation Collection is absent from the resident registry"
            }
            PresentationDmoDeregistrationErrorKind::CollectionVisitRegistryDetached => {
                "Presentation Collection occurrence registry detached during validation"
            }
            PresentationDmoDeregistrationErrorKind::CollectionVisitRegistryPoisoned => {
                "Presentation Collection occurrence registry poisoned during validation"
            }
            PresentationDmoDeregistrationErrorKind::CollectionOccurrenceStale => {
                "Presentation Collection occurrence became stale before validation"
            }
            PresentationDmoDeregistrationErrorKind::CollectionAssetsDetached => {
                "Presentation Collection resident assets are detached"
            }
            PresentationDmoDeregistrationErrorKind::CollectionAssetsNotAlive => {
                "Presentation Collection resident assets are not alive"
            }
            PresentationDmoDeregistrationErrorKind::CollectionAssetsPoisoned => {
                "Presentation Collection resident assets are poisoned"
            }
            PresentationDmoDeregistrationErrorKind::CollectionAssetsSnapshotAllocation => {
                "Presentation Collection resident asset snapshot allocation failed"
            }
            PresentationDmoDeregistrationErrorKind::CollectionArenaCountMismatch => {
                "Presentation Collection Arena count differs from its registered owner"
            }
            PresentationDmoDeregistrationErrorKind::CollectionArenaIdMismatch => {
                "Presentation Collection Arena ID sequence differs from its registered owner"
            }
            PresentationDmoDeregistrationErrorKind::CollectionArenaOwnerMismatch => {
                "Presentation Collection Arena owner sequence differs from its registered owner"
            }
            PresentationDmoDeregistrationErrorKind::OrphanStorageAllocation => {
                "could not retain main-DMO-bank Presentation owners after asymmetric deregistration"
            }
            PresentationDmoDeregistrationErrorKind::InvariantFault => {
                "main-DMO-bank Presentation deregistration hit a private invariant fault"
            }
            PresentationDmoDeregistrationErrorKind::DispatchPanicked => {
                "main-DMO-bank Presentation deregistration dispatcher panicked"
            }
        })
    }
}

impl Error for PresentationDmoDeregistrationError {}

/// Bank-scoped public name for the supported kind-2/kind-11 dispatch seam.
pub use PresentationDmoArenaDispatch as MainDmoBankPresentationDispatch;
/// Bank-scoped public name for one prepared arena.
pub type PreparedMainDmoBankPresentationArena = PreparedPresentationDmoArena;
/// Bank-scoped public name for the exact 125-arena prepared collection.
pub type PreparedMainDmoBankPresentationCollection = PreparedPresentationDmoCollection;
/// Bank-scoped public name for the shared first/last-use runtime adapter.
pub type MainDmoBankPresentationRegistrationAdapter<D> = PresentationDmoArenaRegistrationAdapter<D>;
/// Bank-scoped public name for one live collection occurrence authority.
pub type MainDmoBankPresentationRegistration = PresentationDmoCollectionRegistration;
/// Bank-scoped public name for arena preparation failure.
pub type MainDmoBankPresentationArenaPrepareError = PresentationDmoArenaPrepareError;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::{ResidentCollectionRegistry, ResidentCollectionStorage},
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionStorage},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_resident_registry::{
            SimulationResidentRegistry, SimulationResidentRegistryError,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        pegasus::{
            INSTANCE_DATA_TYPE_ID, INSTANCE_DMO_ASSET_TYPE_ID, ObjectId, TABLE_OF_CONTENTS_TYPE_ID,
        },
        rw4::{Arena, DictionaryEntry, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };
    use std::{
        ops::Range,
        sync::atomic::{AtomicBool, Ordering},
    };

    #[derive(Debug, Eq, PartialEq)]
    enum Event {
        MaterialAdd(u64),
        LivingWorldAdd(u64),
        DmoRegister { primary: u64, local: u64 },
        MaterialRemove(u64),
        LivingWorldRemove(u64),
        DmoDeregister { primary: u64, local: u64 },
    }

    #[derive(Default)]
    struct Recorder {
        events: Vec<Event>,
    }

    #[derive(Clone, Copy)]
    enum PanicStage {
        RegisterMaterial,
        RegisterDmo,
        DeregisterMaterial,
        DeregisterDmo,
    }

    struct PanickingRecorder {
        events: Vec<Event>,
        panic_stage: PanicStage,
    }

    struct DropObservationDispatch {
        plan: Weak<SimulationNormalArenaPlan>,
        source_was_alive: Arc<AtomicBool>,
    }

    impl Drop for DropObservationDispatch {
        fn drop(&mut self) {
            self.source_was_alive
                .store(self.plan.upgrade().is_some(), Ordering::Release);
        }
    }

    impl PresentationDmoArenaDispatch for Recorder {
        fn add_material(&mut self, source: &PresentationMaterialDataHandle) {
            self.events
                .push(Event::MaterialAdd(source.diagnostic_guid()));
        }

        fn add_living_world_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::LivingWorldAdd(asset_id));
        }

        fn register_presentation_dmo_model(
            &mut self,
            source: &SimulationRecordHandle,
            asset_id: u64,
        ) {
            assert_eq!(record_u64(source, DMO_PRIMARY_GUID_OFFSET), asset_id);
            self.events.push(Event::DmoRegister {
                primary: record_u64(source, DMO_PRIMARY_GUID_OFFSET),
                local: record_u64(source, DMO_LOCAL_GUID_OFFSET),
            });
        }

        fn remove_living_world_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::LivingWorldRemove(asset_id));
        }

        fn remove_material(&mut self, source: &PresentationMaterialDataHandle) {
            self.events
                .push(Event::MaterialRemove(source.diagnostic_guid()));
        }

        fn deregister_presentation_dmo_model(
            &mut self,
            source: &SimulationRecordHandle,
            asset_id: u64,
        ) {
            assert_eq!(record_u64(source, DMO_PRIMARY_GUID_OFFSET), asset_id);
            self.events.push(Event::DmoDeregister {
                primary: record_u64(source, DMO_PRIMARY_GUID_OFFSET),
                local: record_u64(source, DMO_LOCAL_GUID_OFFSET),
            });
        }
    }

    impl PresentationDmoArenaDispatch for PanickingRecorder {
        fn add_material(&mut self, source: &PresentationMaterialDataHandle) {
            if matches!(self.panic_stage, PanicStage::RegisterMaterial) {
                panic!("injected material registration panic");
            }
            self.events
                .push(Event::MaterialAdd(source.diagnostic_guid()));
        }

        fn add_living_world_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::LivingWorldAdd(asset_id));
        }

        fn register_presentation_dmo_model(
            &mut self,
            source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
            if matches!(self.panic_stage, PanicStage::RegisterDmo) {
                panic!("injected DMO registration panic");
            }
            self.events.push(Event::DmoRegister {
                primary: record_u64(source, DMO_PRIMARY_GUID_OFFSET),
                local: record_u64(source, DMO_LOCAL_GUID_OFFSET),
            });
        }

        fn remove_living_world_dmo_model(&mut self, asset_id: u64) {
            self.events.push(Event::LivingWorldRemove(asset_id));
        }

        fn remove_material(&mut self, source: &PresentationMaterialDataHandle) {
            if matches!(self.panic_stage, PanicStage::DeregisterMaterial) {
                panic!("injected material deregistration panic");
            }
            self.events
                .push(Event::MaterialRemove(source.diagnostic_guid()));
        }

        fn deregister_presentation_dmo_model(
            &mut self,
            source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
            if matches!(self.panic_stage, PanicStage::DeregisterDmo) {
                panic!("injected DMO deregistration panic");
            }
            self.events.push(Event::DmoDeregister {
                primary: record_u64(source, DMO_PRIMARY_GUID_OFFSET),
                local: record_u64(source, DMO_LOCAL_GUID_OFFSET),
            });
        }
    }

    impl PresentationDmoArenaDispatch for DropObservationDispatch {
        fn add_material(&mut self, _source: &PresentationMaterialDataHandle) {}
        fn add_living_world_dmo_model(&mut self, _asset_id: u64) {}
        fn register_presentation_dmo_model(
            &mut self,
            _source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
        }
        fn remove_living_world_dmo_model(&mut self, _asset_id: u64) {}
        fn remove_material(&mut self, _source: &PresentationMaterialDataHandle) {}
        fn deregister_presentation_dmo_model(
            &mut self,
            _source: &SimulationRecordHandle,
            _asset_id: u64,
        ) {
        }
    }

    fn record_u64(source: &SimulationRecordHandle, offset: usize) -> u64 {
        u64::from_be_bytes(source.bytes()[offset..offset + 8].try_into().unwrap())
    }

    struct ArenaFixture {
        bytes: Vec<u8>,
        dictionary: Vec<DictionaryEntry>,
    }

    impl ArenaFixture {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                dictionary: Vec::new(),
            }
        }

        fn push(&mut self, type_id: u32, object: &[u8]) -> ObjectId {
            let index = self.dictionary.len();
            let start = self.bytes.len();
            self.bytes.extend_from_slice(object);
            let end = self.bytes.len();
            self.dictionary.push(DictionaryEntry {
                resource_index: 0,
                object_offset: u32::try_from(start).unwrap(),
                relocation_offset: 0,
                size: u32::try_from(object.len()).unwrap(),
                alignment: 1,
                type_index: 0,
                type_id,
                object_range: Range { start, end },
            });
            ObjectId(u32::try_from(index).unwrap())
        }

        fn finish(self, asset_id: u64) -> Arc<SimulationNormalArenaPlan> {
            let descriptor = ResourceDescriptor {
                size: 0,
                alignment: 1,
            };
            let entry_count = u32::try_from(self.dictionary.len()).unwrap();
            let virtual_size = u32::try_from(self.bytes.len()).unwrap();
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
                id: u32::try_from(asset_id).unwrap_or(0),
                entry_count,
                used_entry_count: entry_count,
                alignment: 1,
                virtual_size,
                dictionary_offset: 0,
                sections_offset: 0,
                base_offset: 0,
                unfix_context_offset: 0,
                fix_context_offset: 0,
                resource_descriptor: [descriptor; 5],
                resources_used: [descriptor; 5],
                resource_offsets: [0; 5],
                arena_group_offset: 0,
                dictionary: self.dictionary,
            };
            let asset = SimulationCollisionAsset {
                decoded: DecodedArenaAsset::from_parts_for_test(
                    asset_id,
                    WORLD_STREAM_ARENA_ASSET_TYPE,
                    ArenaResources {
                        resources: [self.bytes, Vec::new(), Vec::new(), Vec::new(), Vec::new()],
                    },
                    arena,
                ),
                collision: CollisionArena {
                    id: u32::try_from(asset_id).unwrap_or(0),
                    grind_assets: Vec::new(),
                    aggregate_roots: Vec::new(),
                    primitive_sets: Vec::new(),
                    simple_primitive_sets: Vec::new(),
                    meshes: Vec::new(),
                    collision_models: Vec::new(),
                    instances: Vec::new(),
                },
            };
            let static_plan = Arc::new(SimulationStaticArenaPlan::from_parts_for_test(
                asset,
                Vec::new(),
            ));
            Arc::new(SimulationNormalArenaPlan::from_static_plan(static_plan).unwrap())
        }
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn instance_data(records: &[(u64, u64)]) -> Vec<u8> {
        let mut bytes = vec![0; 20 + records.len() * DMO_RECORD_LEN];
        put_u32(&mut bytes, 0, INSTANCE_DMO_ASSET_TYPE_ID);
        put_u32(&mut bytes, 4, u32::try_from(records.len()).unwrap());
        put_u32(&mut bytes, 12, 20);
        let end = u32::try_from(bytes.len()).unwrap();
        put_u32(&mut bytes, 16, end);
        for (index, &(primary, local)) in records.iter().enumerate() {
            let start = 20 + index * DMO_RECORD_LEN;
            put_u64(&mut bytes, start + DMO_PRIMARY_GUID_OFFSET, primary);
            put_u64(&mut bytes, start + DMO_LOCAL_GUID_OFFSET, local);
        }
        bytes
    }

    fn toc_entries(entries: &[(u64, u32, ObjectId)]) -> Vec<u8> {
        let entries_size = entries.len() * 24;
        let name_offset = u32::try_from(20 + entries_size).unwrap();
        let mut bytes = vec![0; 21 + entries_size];
        put_u32(&mut bytes, 0, u32::try_from(entries.len()).unwrap());
        put_u32(&mut bytes, 4, 20);
        put_u32(&mut bytes, 8, name_offset);
        put_u32(&mut bytes, 16, name_offset);
        for (index, &(guid, object_type, object)) in entries.iter().enumerate() {
            let start = 20 + index * 24;
            put_u32(&mut bytes, start, name_offset);
            put_u64(&mut bytes, start + 8, guid);
            put_u32(&mut bytes, start + 16, object_type);
            put_u32(&mut bytes, start + 20, object.0);
        }
        bytes
    }

    fn toc(guid: u64, object: ObjectId) -> Vec<u8> {
        toc_entries(&[(guid, INSTANCE_DATA_TYPE_ID, object)])
    }

    fn empty_material_data() -> Vec<u8> {
        let mut bytes = vec![0; 20];
        put_u32(&mut bytes, 8, 20);
        put_u32(&mut bytes, 12, 20);
        put_u32(&mut bytes, 16, 20);
        bytes
    }

    fn plan(asset_id: u64, records: &[(u64, u64)]) -> Arc<SimulationNormalArenaPlan> {
        let mut fixture = ArenaFixture::new();
        let instances = fixture.push(INSTANCE_DATA_TYPE_ID, &instance_data(records));
        fixture.push(TABLE_OF_CONTENTS_TYPE_ID, &toc(1, instances));
        fixture.finish(asset_id)
    }

    fn material_dmo_plan(
        asset_id: u64,
        material_guid: u64,
        records: &[(u64, u64)],
    ) -> Arc<SimulationNormalArenaPlan> {
        let mut fixture = ArenaFixture::new();
        let material = fixture.push(R_MATERIAL_DATA_TYPE_ID, &empty_material_data());
        let instances = fixture.push(INSTANCE_DATA_TYPE_ID, &instance_data(records));
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc_entries(&[
                (material_guid, R_MATERIAL_DATA_TYPE_ID, material),
                (1, INSTANCE_DATA_TYPE_ID, instances),
            ]),
        );
        fixture.finish(asset_id)
    }

    fn prepared_collection(
        plans: &[Arc<SimulationNormalArenaPlan>],
    ) -> (
        Vec<PreparedPresentationDmoArena>,
        PreparedPresentationDmoCollection,
    ) {
        let arenas: Vec<_> = plans
            .iter()
            .map(|plan| PreparedPresentationDmoArena::from_plan(Arc::clone(plan)).unwrap())
            .collect();
        let collection = prepared_collection_for_test(&arenas);
        (arenas, collection)
    }

    type DmoStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;
    type DmoRegistry = SimulationResidentRegistry<
        DmoBankProcessedArenaAsset,
        fn(SimulationResidentRegistryError) -> SimulationResidentRegistryError,
        SimulationResidentRegistryError,
    >;

    fn dmo_storage() -> DmoStorage {
        OwnedResidentCollectionStorage::new(std::convert::identity)
    }

    fn dmo_registry() -> (
        DmoRegistry,
        crate::simulation_resident_registry::SimulationResidentLookup<DmoBankProcessedArenaAsset>,
    ) {
        SimulationResidentRegistry::new(std::convert::identity)
    }

    #[test]
    fn kind_11_pairs_are_forward_and_living_world_precedes_dmo() {
        let source = plan(0x100, &[(0x11, 0x101), (0x22, 0x202)]);
        assert_eq!(source.presentation_arena_cost(), 0);
        let (arenas, collection) = prepared_collection(&[source]);
        assert_eq!(arenas[0].cost(), 0);
        assert_eq!(collection.arena_cost(0), Some(0));
        assert_eq!(collection.arena_cost(1), None);
        assert_eq!(collection.arena_count(), 1);
        assert_eq!(collection.operation_count(), 2);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());

        let registration = adapter
            .register_collection(&collection, 0x8000_0008)
            .unwrap();
        adapter.deregister_collection(registration, 8).unwrap();

        assert_eq!(
            adapter.dispatch().events,
            [
                Event::LivingWorldAdd(0x11),
                Event::DmoRegister {
                    primary: 0x11,
                    local: 0x101,
                },
                Event::LivingWorldAdd(0x22),
                Event::DmoRegister {
                    primary: 0x22,
                    local: 0x202,
                },
                Event::LivingWorldRemove(0x11),
                Event::DmoDeregister {
                    primary: 0x11,
                    local: 0x101,
                },
                Event::LivingWorldRemove(0x22),
                Event::DmoDeregister {
                    primary: 0x22,
                    local: 0x202,
                },
            ]
        );
        assert_eq!(adapter.live_collection_count(), 0);
        assert_eq!(adapter.first_use_owner_count(), 0);
    }

    #[test]
    fn deferred_receiver_adds_then_looks_up_then_dispatches_and_finishes() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[Arc::clone(&source)]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(source, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let arena_lookup = lookup.arena_lookup();
        let lookup_attempts = arena_lookup.attached_lookup_attempt_observer_for_test();
        let wrapper = registry
            .create_wrapper(CollectionId(0x900), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(0x900), &token).unwrap();

        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();
        assert_eq!(adapter.live_collection_count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        assert_eq!(
            adapter
                .receive_deferred_arena(&deferred, 0, CollectionId(0x100), 8, &arena_lookup)
                .unwrap(),
            PresentationDmoDeferredArenaTransition::FirstUseDispatched
        );
        assert_eq!(lookup_attempts.count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(
            adapter.dispatch().events,
            [
                Event::LivingWorldAdd(0x11),
                Event::DmoRegister {
                    primary: 0x11,
                    local: 0x101,
                },
            ]
        );

        let registration = adapter.finish_deferred_collection(deferred).unwrap();
        adapter.deregister_collection(registration, 8).unwrap();
        assert_eq!(lookup_attempts.count(), 1);
        assert_eq!(adapter.live_collection_count(), 0);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn deferred_retained_use_skips_even_a_detached_lookup() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[Arc::clone(&source)]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(source, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let arena_lookup = lookup.arena_lookup();
        let lookup_attempts = arena_lookup.attached_lookup_attempt_observer_for_test();
        let wrapper = registry
            .create_wrapper(CollectionId(0x900), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(0x900), &token).unwrap();

        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.begin_deferred_collection(&prepared).unwrap();
        adapter
            .receive_deferred_arena(&first, 0, CollectionId(0x100), 8, &arena_lookup)
            .unwrap();
        let first = adapter.finish_deferred_collection(first).unwrap();
        assert_eq!(lookup_attempts.count(), 1);

        let second = adapter.begin_deferred_collection(&prepared).unwrap();
        drop((registry, wrapper));
        assert_eq!(
            adapter
                .receive_deferred_arena(&second, 0, CollectionId(0x100), 8, &arena_lookup)
                .unwrap(),
            PresentationDmoDeferredArenaTransition::RetainedUse
        );
        assert_eq!(lookup_attempts.count(), 1);
        let second = adapter.finish_deferred_collection(second).unwrap();
        adapter.deregister_collection(first, 8).unwrap();
        adapter.deregister_collection(second, 8).unwrap();
        assert_eq!(adapter.dispatch().events.len(), 4);
        storage.destroy(resident);
    }

    #[test]
    fn deferred_missing_lookup_faults_after_the_single_add() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[source]);
        let (_registry, lookup) = dmo_registry();
        let arena_lookup = lookup.arena_lookup();
        let lookup_attempts = arena_lookup.attached_lookup_attempt_observer_for_test();
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();

        assert_eq!(
            adapter.receive_deferred_arena(&deferred, 0, CollectionId(0x200), 8, &arena_lookup,),
            Err(
                PresentationDmoDeferredArenaReceiveError::UnexpectedArenaId {
                    expected: CollectionId(0x100),
                    actual: CollectionId(0x200),
                }
            )
        );
        assert_eq!(lookup_attempts.count(), 0);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        let error = adapter
            .receive_deferred_arena(&deferred, 0, CollectionId(0x100), 8, &arena_lookup)
            .unwrap_err();
        let PresentationDmoDeferredArenaReceiveError::PostTransitionFault(fault) = error else {
            panic!("unexpected deferred receiver error: {error:?}");
        };
        assert_eq!(
            fault.stage(),
            PresentationDmoAdapterFaultStage::ArenaLookupMissing
        );
        assert_eq!(lookup_attempts.count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert!(adapter.dispatch().events.is_empty());
        assert_eq!(
            adapter.receive_deferred_arena(&deferred, 0, CollectionId(0x100), 8, &arena_lookup,),
            Err(PresentationDmoDeferredArenaReceiveError::AdapterFaulted(
                fault
            ))
        );
        assert_eq!(lookup_attempts.count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
    }

    #[test]
    fn deferred_first_use_rejects_a_replaced_equal_id_owner_after_add() {
        let expected = plan(0x100, &[(0x11, 0x101)]);
        let replacement = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[expected]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(replacement, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let arena_lookup = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(0x900), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(0x900), &token).unwrap();
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();

        let error = adapter
            .receive_deferred_arena(&deferred, 0, CollectionId(0x100), 8, &arena_lookup)
            .unwrap_err();
        let PresentationDmoDeferredArenaReceiveError::PostTransitionFault(fault) = error else {
            panic!("unexpected replaced-owner error: {error:?}");
        };
        assert_eq!(
            fault.stage(),
            PresentationDmoAdapterFaultStage::ArenaLookupOwnerMismatch
        );
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.first_use_owner_count(), 0);
        assert!(adapter.dispatch().events.is_empty());
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn collection_deregistration_receiver_looks_up_once_before_any_remove() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[Arc::clone(&source)]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(source, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let lookup_attempts = lookup.attached_lookup_attempt_observer_for_test();
        let wrapper = registry
            .create_wrapper(CollectionId(DMO_BANK_COLLECTION_ID), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_collection(&prepared, 8).unwrap();

        let refusal = adapter
            .deregister_collection_after_resident_lookup(
                CollectionId(0x900),
                registration,
                8,
                &lookup,
            )
            .unwrap_err();
        assert_eq!(
            refusal.kind(),
            PresentationDmoDeregistrationErrorKind::UnexpectedCollectionId
        );
        assert_eq!(lookup_attempts.count(), 0);
        let registration = refusal.into_registration().unwrap();
        let refusal = adapter
            .deregister_collection_after_resident_lookup(
                CollectionId(DMO_BANK_COLLECTION_ID),
                registration,
                8,
                &lookup,
            )
            .unwrap_err();
        assert_eq!(
            refusal.kind(),
            PresentationDmoDeregistrationErrorKind::CollectionLookupMissing
        );
        assert_eq!(lookup_attempts.count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.dispatch().events.len(), 2);

        registry
            .publish(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        adapter
            .deregister_collection_after_resident_lookup(
                CollectionId(DMO_BANK_COLLECTION_ID),
                refusal.into_registration().unwrap(),
                8,
                &lookup,
            )
            .unwrap();
        assert_eq!(lookup_attempts.count(), 2);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        assert_eq!(adapter.dispatch().events.len(), 4);
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn collection_deregistration_rejects_replaced_owner_before_remove() {
        let expected = plan(0x100, &[(0x11, 0x101)]);
        let replacement = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, prepared) = prepared_collection(&[expected]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(replacement, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let wrapper = registry
            .create_wrapper(CollectionId(DMO_BANK_COLLECTION_ID), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry
            .publish(CollectionId(DMO_BANK_COLLECTION_ID), &token)
            .unwrap();
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_collection(&prepared, 8).unwrap();

        let refusal = adapter
            .deregister_collection_after_resident_lookup(
                CollectionId(DMO_BANK_COLLECTION_ID),
                registration,
                8,
                &lookup,
            )
            .unwrap_err();
        assert_eq!(
            refusal.kind(),
            PresentationDmoDeregistrationErrorKind::CollectionArenaOwnerMismatch
        );
        assert!(refusal.into_registration().is_some());
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.first_use_owner_count(), 1);
        assert_eq!(adapter.dispatch().events.len(), 2);
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn premature_deferred_finish_is_effect_free_and_retains_authority() {
        let source = plan(0x100, &[]);
        let (_arenas, prepared) = prepared_collection(&[Arc::clone(&source)]);
        let mut storage = dmo_storage();
        let resident = storage.create_empty(1).unwrap();
        storage.writer(&resident).unwrap().add_asset(
            DmoBankProcessedArenaAsset::from_parts_for_test(source, Vec::new()),
        );
        let (mut registry, lookup) = dmo_registry();
        let arena_lookup = lookup.arena_lookup();
        let wrapper = registry
            .create_wrapper(CollectionId(0x900), &resident)
            .unwrap();
        let token = DmoRegistry::stable_token(&wrapper);
        registry.publish(CollectionId(0x900), &token).unwrap();
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let deferred = adapter.begin_deferred_collection(&prepared).unwrap();

        let refusal = adapter.finish_deferred_collection(deferred).unwrap_err();
        assert_eq!(
            refusal.kind(),
            PresentationDmoDeferredFinishErrorKind::ArenasRemain {
                next_ordinal: 0,
                arena_count: 1,
            }
        );
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        let deferred = refusal.into_registration();
        adapter
            .receive_deferred_arena(&deferred, 0, CollectionId(0x100), 8, &arena_lookup)
            .unwrap();
        let registration = adapter.finish_deferred_collection(deferred).unwrap();
        adapter.deregister_collection(registration, 8).unwrap();
        drop(wrapper);
        storage.destroy(resident);
    }

    #[test]
    fn material_is_unconditional_and_precedes_dmo_in_both_forward_walks() {
        let source = material_dmo_plan(0x100, 0xfeed, &[(0x11, 0x101), (0x22, 0x202)]);
        let arenas = [PreparedPresentationDmoArena::from_plan(source).unwrap()];
        assert_eq!(arenas[0].material_operation_count(), 1);
        assert_eq!(arenas[0].dmo_operation_count(), 2);
        let collection = prepared_collection_for_test(&arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());

        let registration = adapter.register_collection(&collection, 0).unwrap();
        adapter.deregister_collection(registration, 0).unwrap();

        assert_eq!(
            adapter.dispatch().events,
            [Event::MaterialAdd(0xfeed), Event::MaterialRemove(0xfeed)]
        );

        let source = material_dmo_plan(0x200, 0xbeef, &[(0x33, 0x303)]);
        let arenas = [PreparedPresentationDmoArena::from_plan(source).unwrap()];
        let collection = prepared_collection_for_test(&arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_collection(&collection, 8).unwrap();
        adapter.deregister_collection(registration, 8).unwrap();
        assert_eq!(
            adapter.dispatch().events,
            [
                Event::MaterialAdd(0xbeef),
                Event::LivingWorldAdd(0x33),
                Event::DmoRegister {
                    primary: 0x33,
                    local: 0x303,
                },
                Event::MaterialRemove(0xbeef),
                Event::LivingWorldRemove(0x33),
                Event::DmoDeregister {
                    primary: 0x33,
                    local: 0x303,
                },
            ]
        );
    }

    #[test]
    fn retained_costs_drive_native_default_deferred_prefixes() {
        let plans: Vec<_> = (0_u64..13)
            .map(|ordinal| plan(0x100 + ordinal, &[]))
            .collect();
        let (_arenas, collection) = prepared_collection(&plans);

        let first = collection
            .select_deferred_batch(DeferredCollectionBatchControls::NATIVE_DEFAULT, 0)
            .unwrap();
        assert_eq!(first.requested_arena_count, 10);
        assert_eq!(first.selected_arena_count, 10);
        assert_eq!(first.next_index, 10);
        assert!(!first.completed_this_update);

        let second = collection
            .select_deferred_batch(
                DeferredCollectionBatchControls::NATIVE_DEFAULT,
                first.next_index,
            )
            .unwrap();
        assert_eq!(second.requested_arena_count, 3);
        assert_eq!(second.selected_arena_count, 3);
        assert_eq!(second.next_index, 13);
        assert!(second.completed_this_update);
    }

    #[test]
    fn public_collection_constructor_rejects_non_bank_census_before_effects() {
        let source = material_dmo_plan(0x100, 0xfeed, &[(0x11, 0x101)]);
        let arenas = [PreparedPresentationDmoArena::from_plan(source).unwrap()];
        assert!(matches!(
            PreparedPresentationDmoCollection::try_from_prepared(&arenas),
            Err(
                PresentationDmoCollectionPrepareError::MainDmoBankCensusMismatch {
                    arena_count: 1,
                    operation_count: 2,
                    material_operation_count: 1,
                    dmo_operation_count: 1,
                    ..
                }
            )
        ));
    }

    #[test]
    fn equal_material_bytes_from_distinct_owners_retain_the_first_owner() {
        let first_source = material_dmo_plan(0x100, 0xfeed, &[]);
        let second_source = material_dmo_plan(0x100, 0xfeed, &[]);
        let first_weak = Arc::downgrade(&first_source);
        let first_arenas =
            [PreparedPresentationDmoArena::from_plan(Arc::clone(&first_source)).unwrap()];
        let second_arenas = [PreparedPresentationDmoArena::from_plan(second_source).unwrap()];
        let first_collection = prepared_collection_for_test(&first_arenas);
        let second_collection = prepared_collection_for_test(&second_arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.register_collection(&first_collection, 0).unwrap();
        let second = adapter.register_collection(&second_collection, 0).unwrap();
        adapter.deregister_collection(first, 0).unwrap();
        drop(first_source);
        drop(first_arenas);
        drop(first_collection);
        adapter.deregister_collection(second, 0).unwrap();

        assert_eq!(adapter.orphaned_owner_count(), 1);
        assert!(first_weak.upgrade().is_some());
        assert_eq!(
            adapter.dispatch().events,
            [Event::MaterialAdd(0xfeed), Event::MaterialRemove(0xfeed)]
        );
    }

    #[test]
    fn separate_normal_plans_over_one_static_owner_share_material_identity() {
        let first_source = material_dmo_plan(0x100, 0xfeed, &[]);
        let second_source = Arc::new(
            SimulationNormalArenaPlan::from_static_plan(first_source.static_plan_owner()).unwrap(),
        );
        let first_arenas = [PreparedPresentationDmoArena::from_plan(first_source).unwrap()];
        let second_arenas = [PreparedPresentationDmoArena::from_plan(second_source).unwrap()];
        let first_collection = prepared_collection_for_test(&first_arenas);
        let second_collection = prepared_collection_for_test(&second_arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());

        let first = adapter.register_collection(&first_collection, 0).unwrap();
        let second = adapter.register_collection(&second_collection, 0).unwrap();
        adapter.deregister_collection(first, 0).unwrap();
        adapter.deregister_collection(second, 0).unwrap();

        assert_eq!(adapter.orphaned_owner_count(), 0);
        assert_eq!(
            adapter.dispatch().events,
            [Event::MaterialAdd(0xfeed), Event::MaterialRemove(0xfeed)]
        );
    }

    #[test]
    fn malformed_material_is_rejected_during_preparation() {
        let mut fixture = ArenaFixture::new();
        let material = fixture.push(R_MATERIAL_DATA_TYPE_ID, &[0; 8]);
        fixture.push(
            TABLE_OF_CONTENTS_TYPE_ID,
            &toc_entries(&[(0xfeed, R_MATERIAL_DATA_TYPE_ID, material)]),
        );
        let error = PreparedPresentationDmoArena::from_plan(fixture.finish(0x100)).unwrap_err();
        assert!(matches!(
            error,
            PresentationDmoArenaPrepareError::InvalidMaterial(MaterialError::Truncated { .. })
        ));
    }

    #[test]
    fn repeated_arena_id_dispatches_only_first_add_and_last_remove() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, collection) = prepared_collection(&[source]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());

        let first = adapter.register_collection(&collection, 8).unwrap();
        let second = adapter.register_collection(&collection, 8).unwrap();
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(2));
        assert_eq!(adapter.dispatch().events.len(), 2);
        adapter.deregister_collection(first, 8).unwrap();
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.dispatch().events.len(), 2);
        adapter.deregister_collection(second, 8).unwrap();
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        assert_eq!(adapter.dispatch().events.len(), 4);
    }

    #[test]
    fn first_use_owner_survives_earlier_collection_deregistration() {
        let first_plan = plan(0x100, &[(0x11, 0x101)]);
        let second_plan = plan(0x100, &[(0x22, 0x202)]);
        let first_weak = Arc::downgrade(&first_plan);
        let (first_arenas, first_collection) = prepared_collection(&[Arc::clone(&first_plan)]);
        let (_second_arenas, second_collection) = prepared_collection(&[Arc::clone(&second_plan)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());

        let first = adapter.register_collection(&first_collection, 8).unwrap();
        let second = adapter.register_collection(&second_collection, 8).unwrap();
        drop(first_plan);
        drop(first_arenas);
        drop(first_collection);
        adapter.deregister_collection(first, 8).unwrap();
        assert!(first_weak.upgrade().is_some());
        assert_eq!(adapter.first_use_owner_count(), 1);

        adapter.deregister_collection(second, 8).unwrap();
        assert!(first_weak.upgrade().is_some());
        assert_eq!(adapter.orphaned_owner_count(), 1);
        assert_eq!(
            adapter.dispatch().events,
            [
                Event::LivingWorldAdd(0x11),
                Event::DmoRegister {
                    primary: 0x11,
                    local: 0x101,
                },
                Event::LivingWorldRemove(0x22),
                Event::DmoDeregister {
                    primary: 0x22,
                    local: 0x202,
                },
            ]
        );
    }

    #[test]
    fn userdata_is_sampled_independently_and_asymmetry_retains_owner() {
        for (reg_userdata, dreg_userdata, expected_events, expected_orphans) in [
            (0, 0, 0, 0),
            (0, 8, 2, 0),
            (8, 0, 2, 1),
            (u32::MAX, 8, 4, 0),
        ] {
            let source = plan(0x100, &[(0x11, 0x101)]);
            let weak = Arc::downgrade(&source);
            let (arenas, collection) = prepared_collection(&[Arc::clone(&source)]);
            let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
            let registration = adapter
                .register_collection(&collection, reg_userdata)
                .unwrap();
            drop(source);
            drop(arenas);
            drop(collection);
            adapter
                .deregister_collection(registration, dreg_userdata)
                .unwrap();
            assert_eq!(adapter.dispatch().events.len(), expected_events);
            assert_eq!(adapter.orphaned_owner_count(), expected_orphans);
            assert_eq!(weak.upgrade().is_some(), expected_orphans == 1);
        }
    }

    #[test]
    fn zero_operation_arena_counts_without_creating_an_orphan() {
        let source = plan(0x100, &[]);
        let weak = Arc::downgrade(&source);
        let (arenas, collection) = prepared_collection(&[Arc::clone(&source)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let registration = adapter.register_collection(&collection, 8).unwrap();
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        drop(source);
        drop(arenas);
        drop(collection);

        adapter.deregister_collection(registration, 0).unwrap();
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        assert_eq!(adapter.orphaned_owner_count(), 0);
        assert!(adapter.dispatch().events.is_empty());
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn separately_prepared_same_backing_source_matches_pointer_identity() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (first_arenas, first_collection) = prepared_collection(&[Arc::clone(&source)]);
        let (second_arenas, second_collection) = prepared_collection(&[Arc::clone(&source)]);
        let weak = Arc::downgrade(&source);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.register_collection(&first_collection, 8).unwrap();
        let second = adapter.register_collection(&second_collection, 8).unwrap();
        adapter.deregister_collection(first, 8).unwrap();
        drop(source);
        drop(first_arenas);
        drop(first_collection);
        drop(second_arenas);
        drop(second_collection);

        adapter.deregister_collection(second, 8).unwrap();
        assert_eq!(adapter.orphaned_owner_count(), 0);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn equal_ids_and_bytes_from_distinct_owners_do_not_fake_pointer_identity() {
        let first_source = plan(0x100, &[(0x11, 0x101)]);
        let second_source = plan(0x100, &[(0x11, 0x101)]);
        let first_weak = Arc::downgrade(&first_source);
        let (first_arenas, first_collection) = prepared_collection(&[Arc::clone(&first_source)]);
        let (_second_arenas, second_collection) =
            prepared_collection(&[Arc::clone(&second_source)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let first = adapter.register_collection(&first_collection, 8).unwrap();
        let second = adapter.register_collection(&second_collection, 8).unwrap();
        drop(first_source);
        drop(first_arenas);
        drop(first_collection);
        adapter.deregister_collection(first, 8).unwrap();
        adapter.deregister_collection(second, 8).unwrap();

        assert_eq!(adapter.orphaned_owner_count(), 1);
        assert!(first_weak.upgrade().is_some());
        drop(adapter);
        assert!(first_weak.upgrade().is_none());
    }

    #[test]
    fn foreign_authority_is_recoverable_without_effects() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let (_arenas, collection) = prepared_collection(&[source]);
        let mut owner = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let mut foreign = PresentationDmoArenaRegistrationAdapter::new(Recorder::default());
        let registration = owner.register_collection(&collection, 8).unwrap();

        let rejection = foreign.deregister_collection(registration, 8).unwrap_err();
        assert_eq!(
            rejection.kind(),
            PresentationDmoDeregistrationErrorKind::ForeignDomain
        );
        assert!(foreign.dispatch().events.is_empty());
        owner
            .deregister_collection(rejection.into_registration().unwrap(), 8)
            .unwrap();
    }

    #[test]
    fn registration_panic_faults_domain_and_retains_prefix_owner() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let weak = Arc::downgrade(&source);
        let (arenas, collection) = prepared_collection(&[Arc::clone(&source)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(PanickingRecorder {
            events: Vec::new(),
            panic_stage: PanicStage::RegisterDmo,
        });

        let error = adapter.register_collection(&collection, 8).unwrap_err();
        let PresentationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected registration error: {error:?}");
        };
        assert_eq!(
            fault.phase(),
            PresentationDmoAdapterFaultPhase::Registration
        );
        assert_eq!(fault.arena_index(), 0);
        assert_eq!(fault.operation_index(), Some(0));
        assert_eq!(fault.stage(), PresentationDmoAdapterFaultStage::Dmo);
        assert_eq!(adapter.fault(), Some(fault));
        assert_eq!(adapter.live_collection_count(), 1);
        assert_eq!(adapter.first_use_owner_count(), 1);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), Some(1));
        assert_eq!(adapter.dispatch().events, [Event::LivingWorldAdd(0x11)]);
        drop(source);
        drop(arenas);
        drop(collection);
        assert!(weak.upgrade().is_some());

        let retry_source = plan(0x100, &[(0x22, 0x202)]);
        let (_retry_arenas, retry_collection) = prepared_collection(&[retry_source]);
        assert!(matches!(
            adapter.register_collection(&retry_collection, 8),
            Err(PresentationDmoRegistrationError::AdapterFaulted(value)) if value == fault
        ));
        assert_eq!(adapter.first_use_owner_count(), 1);
        drop(adapter);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn material_registration_panic_faults_before_the_dmo_suffix() {
        let source = material_dmo_plan(0x100, 0xfeed, &[(0x11, 0x101)]);
        let weak = Arc::downgrade(&source);
        let arenas = [PreparedPresentationDmoArena::from_plan(Arc::clone(&source)).unwrap()];
        let collection = prepared_collection_for_test(&arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(PanickingRecorder {
            events: Vec::new(),
            panic_stage: PanicStage::RegisterMaterial,
        });

        let error = adapter.register_collection(&collection, 8).unwrap_err();
        let PresentationDmoRegistrationError::DispatchPanicked(fault) = error else {
            panic!("unexpected registration error: {error:?}");
        };
        assert_eq!(
            fault.phase(),
            PresentationDmoAdapterFaultPhase::Registration
        );
        assert_eq!(fault.operation_index(), Some(0));
        assert_eq!(fault.stage(), PresentationDmoAdapterFaultStage::Material);
        assert!(adapter.dispatch().events.is_empty());
        drop(source);
        drop(arenas);
        drop(collection);
        assert!(weak.upgrade().is_some());
    }

    #[test]
    fn deregistration_panic_consumes_token_and_blocks_source_replacement() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let weak = Arc::downgrade(&source);
        let (arenas, collection) = prepared_collection(&[Arc::clone(&source)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(PanickingRecorder {
            events: Vec::new(),
            panic_stage: PanicStage::DeregisterDmo,
        });
        let registration = adapter.register_collection(&collection, 8).unwrap();
        drop(source);
        drop(arenas);
        drop(collection);

        let error = adapter.deregister_collection(registration, 8).unwrap_err();
        assert_eq!(
            error.kind(),
            PresentationDmoDeregistrationErrorKind::DispatchPanicked
        );
        assert!(error.into_registration().is_none());
        let fault = adapter.fault().unwrap();
        assert_eq!(
            fault.phase(),
            PresentationDmoAdapterFaultPhase::Deregistration
        );
        assert_eq!(fault.arena_index(), 0);
        assert_eq!(fault.operation_index(), Some(0));
        assert_eq!(fault.stage(), PresentationDmoAdapterFaultStage::Dmo);
        assert_eq!(adapter.arena_ref_count(CollectionId(0x100)), None);
        assert_eq!(adapter.live_collection_count(), 1);
        assert_eq!(adapter.first_use_owner_count(), 1);
        assert!(weak.upgrade().is_some());
        assert_eq!(
            adapter.dispatch().events,
            [
                Event::LivingWorldAdd(0x11),
                Event::DmoRegister {
                    primary: 0x11,
                    local: 0x101,
                },
                Event::LivingWorldRemove(0x11),
            ]
        );

        let retry_source = plan(0x100, &[(0x22, 0x202)]);
        let (_retry_arenas, retry_collection) = prepared_collection(&[retry_source]);
        assert!(matches!(
            adapter.register_collection(&retry_collection, 8),
            Err(PresentationDmoRegistrationError::AdapterFaulted(value)) if value == fault
        ));
        assert_eq!(adapter.first_use_owner_count(), 1);
        drop(adapter);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn material_deregistration_panic_consumes_token_before_the_dmo_suffix() {
        let source = material_dmo_plan(0x100, 0xfeed, &[(0x11, 0x101)]);
        let weak = Arc::downgrade(&source);
        let arenas = [PreparedPresentationDmoArena::from_plan(Arc::clone(&source)).unwrap()];
        let collection = prepared_collection_for_test(&arenas);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(PanickingRecorder {
            events: Vec::new(),
            panic_stage: PanicStage::DeregisterMaterial,
        });
        let registration = adapter.register_collection(&collection, 8).unwrap();

        let error = adapter.deregister_collection(registration, 8).unwrap_err();
        assert_eq!(
            error.kind(),
            PresentationDmoDeregistrationErrorKind::DispatchPanicked
        );
        assert!(error.into_registration().is_none());
        let fault = adapter.fault().unwrap();
        assert_eq!(
            fault.phase(),
            PresentationDmoAdapterFaultPhase::Deregistration
        );
        assert_eq!(fault.operation_index(), Some(0));
        assert_eq!(fault.stage(), PresentationDmoAdapterFaultStage::Material);
        assert_eq!(
            adapter.dispatch().events,
            [
                Event::MaterialAdd(0xfeed),
                Event::LivingWorldAdd(0x11),
                Event::DmoRegister {
                    primary: 0x11,
                    local: 0x101,
                },
            ]
        );
        drop(source);
        drop(arenas);
        drop(collection);
        assert!(weak.upgrade().is_some());
    }

    #[test]
    fn dispatcher_drops_before_abandoned_live_source_owners() {
        let source = plan(0x100, &[(0x11, 0x101)]);
        let weak = Arc::downgrade(&source);
        let source_was_alive = Arc::new(AtomicBool::new(false));
        let (arenas, collection) = prepared_collection(&[Arc::clone(&source)]);
        let mut adapter = PresentationDmoArenaRegistrationAdapter::new(DropObservationDispatch {
            plan: Weak::clone(&weak),
            source_was_alive: Arc::clone(&source_was_alive),
        });
        let registration = adapter.register_collection(&collection, 8).unwrap();
        drop(registration);
        drop(source);
        drop(arenas);
        drop(collection);

        drop(adapter);
        assert!(source_was_alive.load(Ordering::Acquire));
        assert!(weak.upgrade().is_none());
    }
}
