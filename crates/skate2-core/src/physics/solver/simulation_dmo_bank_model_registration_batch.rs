//! Effect-free, exact-owner model-registration source issuance for one
//! resident processed DMO bank.
//!
//! The batch freezes resident asset order and each asset's authored DMO
//! operation subsequence. It does not mutate the manager, install an adapter,
//! infer registration timing, or deduplicate model keys. Each issue samples
//! resident liveness only at that point; the returned source can outlive the
//! check. Reusing the immutable batch for fresh passes is safe-host prepared-
//! source policy, not recovered native occurrence authority. Delayed manager
//! use of an issued owning source is not live-dispatch authority.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged source batch awaits the outer DMO collection coordinator"
    )
)]

use super::{SimulationDmoModelRegistrationSource, SimulationDmoModelRegistrationSourceError};
use crate::{
    dmo_bank_processed_asset::{
        DMO_BANK_ASSET_COUNT, DMO_BANK_ASSET_ID_SIGNATURE, DMO_BANK_COLLECTION_ID,
        DmoBankProcessedArenaAsset,
    },
    owned_resident_collection::{OwnedResidentCollectionError, OwnedResidentCollectionOperation},
    simulation_collection_adapter::{
        PreparedSimulationCollection, SimulationCollectionPrepareError,
    },
    simulation_normal_arena_plan::{SimulationNormalOperation, SimulationRecordHandle},
    simulation_resident_registry::{
        SimulationMainDmoBankResidentLiveness, SimulationMainDmoBankResidentLivenessError,
        SimulationMainDmoBankResidentRevalidationError, SimulationMainDmoBankResidentSnapshot,
        SimulationResidentOccurrence,
    },
};
use std::{error::Error, fmt, sync::Arc};

#[cfg(test)]
use crate::owned_resident_collection::{OwnedResidentCollection, OwnedResidentCollectionView};

// Pinned reference-corpus census, not a recovered native table capacity.
const DMO_BANK_DMO_INSTANCE_COUNT: usize = 93;
const DMO_BANK_FIRST_ASSET_ID: u64 = 0x91d7_2407_5ae4_4972;
const DMO_BANK_LAST_ASSET_ID: u64 = 0xd309_6f13_cf1c_0aab;

struct RetainedDmoBankAsset {
    asset: Arc<DmoBankProcessedArenaAsset>,
}

struct ExpectedDmoModelRegistration {
    asset_index: usize,
    operation_index: usize,
    primary_asset_id: u64,
    source: SimulationRecordHandle,
}

/// Effect-free refusal while freezing one resident bank's DMO model callback
/// subsequence.
#[derive(Debug)]
pub(crate) enum SimulationDmoBankModelRegistrationBatchError {
    ResidentSnapshot(OwnedResidentCollectionError),
    AssetStorageAllocation {
        requested: usize,
    },
    AssetExpiredDuringBuild {
        asset_index: usize,
    },
    SourceCountOverflow,
    SourceStorageAllocation {
        requested: usize,
    },
    SourcePreparation {
        asset_index: usize,
        operation_index: usize,
        source: SimulationDmoModelRegistrationSourceError,
    },
}

impl fmt::Display for SimulationDmoBankModelRegistrationBatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentSnapshot(source) => {
                write!(formatter, "could not snapshot resident DMO bank: {source}")
            }
            Self::AssetStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not retain {requested} resident DMO assets"
                )
            }
            Self::AssetExpiredDuringBuild { asset_index } => write!(
                formatter,
                "resident DMO asset {asset_index} expired while building model sources"
            ),
            Self::SourceCountOverflow => {
                formatter.write_str("resident DMO model source count overflowed")
            }
            Self::SourceStorageAllocation { requested } => {
                write!(
                    formatter,
                    "could not retain {requested} DMO model source rows"
                )
            }
            Self::SourcePreparation {
                asset_index,
                operation_index,
                source,
            } => write!(
                formatter,
                "could not prepare DMO model source at asset {asset_index}, operation {operation_index}: {source}"
            ),
        }
    }
}

impl Error for SimulationDmoBankModelRegistrationBatchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentSnapshot(source) => Some(source),
            Self::SourcePreparation { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Typed safe-host refusal while deriving the inseparable collection view
/// from one model batch's already-retained exact arena owners.
#[derive(Debug)]
pub(super) enum SimulationDmoBankPreparedCollectionError {
    CorpusAssetCountMismatch {
        actual: usize,
    },
    CorpusDmoInstanceCountMismatch {
        actual: usize,
    },
    CorpusAssetIdDuplicate {
        first_ordinal: usize,
        second_ordinal: usize,
    },
    CorpusAssetIdSequenceMismatch,
    PlanOwnerStorageAllocation {
        requested: usize,
    },
    Collection(SimulationCollectionPrepareError),
}

impl fmt::Display for SimulationDmoBankPreparedCollectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorpusAssetCountMismatch { actual } => write!(
                formatter,
                "DMO bank binding retained {actual} assets instead of the pinned corpus count"
            ),
            Self::CorpusDmoInstanceCountMismatch { actual } => write!(
                formatter,
                "DMO bank binding retained {actual} kind-11 rows instead of the pinned corpus count"
            ),
            Self::CorpusAssetIdDuplicate {
                first_ordinal,
                second_ordinal,
            } => write!(
                formatter,
                "DMO bank binding repeats one asset ID at ordinals {first_ordinal} and {second_ordinal}"
            ),
            Self::CorpusAssetIdSequenceMismatch => formatter
                .write_str("DMO bank binding asset-ID sequence does not match the pinned corpus"),
            Self::PlanOwnerStorageAllocation { requested } => write!(
                formatter,
                "could not retain {requested} DMO bank collection plan owners"
            ),
            Self::Collection(source) => {
                write!(formatter, "could not prepare DMO bank collection: {source}")
            }
        }
    }
}

impl Error for SimulationDmoBankPreparedCollectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Collection(source) => Some(source),
            Self::CorpusAssetCountMismatch { .. }
            | Self::CorpusDmoInstanceCountMismatch { .. }
            | Self::CorpusAssetIdDuplicate { .. }
            | Self::CorpusAssetIdSequenceMismatch
            | Self::PlanOwnerStorageAllocation { .. } => None,
        }
    }
}

/// Typed refusal while replaying one exact DMO model callback subsequence.
#[derive(Debug)]
pub(crate) enum SimulationDmoBankModelRegistrationPassError {
    ResidentExpired,
    ResidentAccess(OwnedResidentCollectionError),
    PublishedResidentRegistryPoisoned,
    Extra {
        issued: usize,
        expected: usize,
    },
    SourceMismatch {
        ordinal: usize,
    },
    PrimaryAssetIdMismatch {
        ordinal: usize,
    },
    SourcePreparation {
        ordinal: usize,
        source: SimulationDmoModelRegistrationSourceError,
    },
    Incomplete {
        issued: usize,
        expected: usize,
    },
}

impl fmt::Display for SimulationDmoBankModelRegistrationPassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidentExpired => formatter.write_str("resident DMO model source bank expired"),
            Self::ResidentAccess(source) => {
                write!(
                    formatter,
                    "resident DMO model source access failed: {source}"
                )
            }
            Self::PublishedResidentRegistryPoisoned => {
                formatter.write_str("published resident DMO source registry is poisoned")
            }
            Self::Extra { issued, expected } => write!(
                formatter,
                "DMO model callback {issued} exceeds {expected} expected callbacks"
            ),
            Self::SourceMismatch { ordinal } => write!(
                formatter,
                "DMO model callback {ordinal} has another exact record source"
            ),
            Self::PrimaryAssetIdMismatch { ordinal } => write!(
                formatter,
                "DMO model callback {ordinal} has another full primary asset ID"
            ),
            Self::SourcePreparation { ordinal, source } => write!(
                formatter,
                "could not issue DMO model callback {ordinal}: {source}"
            ),
            Self::Incomplete { issued, expected } => write!(
                formatter,
                "DMO model callback pass issued {issued} of {expected} callbacks"
            ),
        }
    }
}

impl Error for SimulationDmoBankModelRegistrationPassError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ResidentAccess(source) => Some(source),
            Self::SourcePreparation { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Immutable, non-cloneable issuer for one prepared resident DMO-bank snapshot.
///
/// Retaining assets keeps exact record storage valid but does not make the
/// collection resident. Every issue performs a new point-in-time liveness
/// sample.
pub(crate) struct SimulationDmoBankModelRegistrationBatch {
    inner: Arc<SimulationDmoBankModelRegistrationBatchInner>,
}

struct SimulationDmoBankModelRegistrationBatchInner {
    resident: SimulationDmoBankModelRegistrationResident,
    assets: Vec<RetainedDmoBankAsset>,
    expected: Vec<ExpectedDmoModelRegistration>,
}

enum SimulationDmoBankModelRegistrationResident {
    Published(SimulationMainDmoBankResidentLiveness),
    #[cfg(test)]
    Synthetic(OwnedResidentCollectionView<DmoBankProcessedArenaAsset>),
}

/// Unforgeable handoff accepted by the exact published-occurrence snapshot.
///
/// Its constructor remains private to this module. The registry can name the
/// type and complete the handoff, but no other crate sibling can manufacture
/// one to separate snapshot liveness from its exact retained assets.
pub(crate) struct SimulationDmoBankSnapshotConsumer(());

impl SimulationDmoBankSnapshotConsumer {
    /// Completes the registry-owned handoff. On any fallible preparation the
    /// exact liveness and asset owners are returned so the registry can
    /// reconstruct the opaque retry token without exposing either half.
    pub(crate) fn finish(
        self,
        liveness: SimulationMainDmoBankResidentLiveness,
        assets: Vec<Arc<DmoBankProcessedArenaAsset>>,
    ) -> Result<
        SimulationDmoBankModelRegistrationBatch,
        (
            SimulationDmoBankModelRegistrationBatchError,
            SimulationMainDmoBankResidentLiveness,
            Vec<Arc<DmoBankProcessedArenaAsset>>,
        ),
    > {
        let Self(()) = self;
        let mut retained = Vec::new();
        if retained.try_reserve_exact(assets.len()).is_err() {
            return Err((
                SimulationDmoBankModelRegistrationBatchError::AssetStorageAllocation {
                    requested: assets.len(),
                },
                liveness,
                assets,
            ));
        }
        retained.extend(
            assets
                .iter()
                .cloned()
                .map(|asset| RetainedDmoBankAsset { asset }),
        );
        let expected =
            match SimulationDmoBankModelRegistrationBatch::try_prepare_expected(&retained) {
                Ok(expected) => expected,
                Err(error) => return Err((error, liveness, assets)),
            };
        drop(assets);
        Ok(SimulationDmoBankModelRegistrationBatch {
            inner: Arc::new(SimulationDmoBankModelRegistrationBatchInner {
                resident: SimulationDmoBankModelRegistrationResident::Published(liveness),
                assets: retained,
                expected,
            }),
        })
    }
}

impl fmt::Debug for SimulationDmoBankModelRegistrationBatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resident = match &self.inner.resident {
            SimulationDmoBankModelRegistrationResident::Published(liveness) => {
                liveness.try_require_alive().is_ok()
            }
            #[cfg(test)]
            SimulationDmoBankModelRegistrationResident::Synthetic(collection) => {
                collection.is_alive()
            }
        };
        formatter
            .debug_struct("SimulationDmoBankModelRegistrationBatch")
            .field("asset_count", &self.inner.assets.len())
            .field("source_count", &self.inner.expected.len())
            .field("resident", &resident)
            .finish_non_exhaustive()
    }
}

impl SimulationDmoBankModelRegistrationBatch {
    /// Consumes one exact published-occurrence snapshot through the sealed
    /// registry handoff. No occurrence, liveness, or retained-asset half is
    /// returned to this caller.
    pub(super) fn try_from_published_snapshot(
        snapshot: SimulationMainDmoBankResidentSnapshot,
    ) -> Result<
        Self,
        crate::simulation_resident_registry::SimulationMainDmoBankResidentSnapshotConsumeFailure,
    > {
        snapshot.consume(SimulationDmoBankSnapshotConsumer(()))
    }

    /// Freezes exact resident assets and prevalidates every authored DMO source
    /// before publishing the batch. Duplicate model keys remain distinct rows.
    #[cfg(test)]
    pub(crate) fn try_new(
        collection: &OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) -> Result<Self, SimulationDmoBankModelRegistrationBatchError> {
        let weak_assets = collection
            .weak_assets_in_order()
            .map_err(SimulationDmoBankModelRegistrationBatchError::ResidentSnapshot)?;
        let mut assets = Vec::new();
        assets.try_reserve_exact(weak_assets.len()).map_err(|_| {
            SimulationDmoBankModelRegistrationBatchError::AssetStorageAllocation {
                requested: weak_assets.len(),
            }
        })?;
        for (asset_index, weak_asset) in weak_assets.iter().enumerate() {
            let asset = weak_asset.weak().upgrade().ok_or(
                SimulationDmoBankModelRegistrationBatchError::AssetExpiredDuringBuild {
                    asset_index,
                },
            )?;
            assets.push(RetainedDmoBankAsset { asset });
        }

        let expected = Self::try_prepare_expected(&assets)?;

        Ok(Self {
            inner: Arc::new(SimulationDmoBankModelRegistrationBatchInner {
                resident: SimulationDmoBankModelRegistrationResident::Synthetic(
                    collection.weak_view(),
                ),
                assets,
                expected,
            }),
        })
    }

    fn try_prepare_expected(
        assets: &[RetainedDmoBankAsset],
    ) -> Result<Vec<ExpectedDmoModelRegistration>, SimulationDmoBankModelRegistrationBatchError>
    {
        let source_count = assets
            .iter()
            .try_fold(0usize, |count, retained| {
                let additions = retained
                    .asset
                    .plan()
                    .operations()
                    .iter()
                    .filter(|operation| {
                        matches!(operation, SimulationNormalOperation::DmoInstance { .. })
                    })
                    .count();
                count.checked_add(additions)
            })
            .ok_or(SimulationDmoBankModelRegistrationBatchError::SourceCountOverflow)?;
        let mut expected = Vec::new();
        expected.try_reserve_exact(source_count).map_err(|_| {
            SimulationDmoBankModelRegistrationBatchError::SourceStorageAllocation {
                requested: source_count,
            }
        })?;
        for (asset_index, retained) in assets.iter().enumerate() {
            for (operation_index, operation) in
                retained.asset.plan().operations().iter().enumerate()
            {
                let SimulationNormalOperation::DmoInstance { asset_id, .. } = *operation else {
                    continue;
                };
                let prepared = SimulationDmoModelRegistrationSource::try_prepare(
                    Arc::clone(&retained.asset),
                    operation_index,
                )
                .map_err(|source| {
                    SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                        asset_index,
                        operation_index,
                        source,
                    }
                })?;
                drop(prepared);
                let source = retained.asset.plan().record_handle(operation_index).ok_or(
                    SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                        asset_index,
                        operation_index,
                        source: SimulationDmoModelRegistrationSourceError::MissingRecordSource {
                            operation_index,
                        },
                    },
                )?;
                expected.push(ExpectedDmoModelRegistration {
                    asset_index,
                    operation_index,
                    primary_asset_id: asset_id,
                    source,
                });
            }
        }

        Ok(expected)
    }

    /// Begins a fresh sealed pass over the complete expected DMO subsequence.
    /// Unlimited reuse is safe-host prepared-source policy, not native
    /// registration-pass or occurrence authority.
    pub(super) fn begin_pass(&self) -> SimulationDmoBankModelRegistrationPass {
        SimulationDmoBankModelRegistrationPass {
            inner: Arc::clone(&self.inner),
            next: 0,
        }
    }

    /// Verifies that a fresh lookup still names the exact published occurrence
    /// and wrapper-time owners from which this batch was prepared.
    pub(super) fn try_revalidate_published_occurrence(
        &self,
        occurrence: &SimulationResidentOccurrence<DmoBankProcessedArenaAsset>,
    ) -> Result<(), SimulationMainDmoBankResidentRevalidationError> {
        #[cfg(not(test))]
        let SimulationDmoBankModelRegistrationResident::Published(liveness) = &self.inner.resident;
        #[cfg(test)]
        let liveness = match &self.inner.resident {
            SimulationDmoBankModelRegistrationResident::Published(liveness) => liveness,
            SimulationDmoBankModelRegistrationResident::Synthetic(_) => {
                return Err(SimulationMainDmoBankResidentRevalidationError::DifferentOccurrence);
            }
        };
        let snapshot = liveness.try_snapshot_same_occurrence(occurrence)?;
        if snapshot.has_same_assets(self.inner.assets.iter().map(|retained| &retained.asset)) {
            Ok(())
        } else {
            Err(SimulationMainDmoBankResidentRevalidationError::RetainedAssetSequenceMismatch)
        }
    }

    /// Tests one current Arena winner against the exact outer asset owner
    /// retained at collection Setup. This exposes neither owner nor source
    /// authority to the caller.
    pub(super) fn retained_arena_owner_matches(
        &self,
        ordinal: usize,
        current: &DmoBankProcessedArenaAsset,
    ) -> bool {
        self.inner.assets.get(ordinal).is_some_and(|retained| {
            std::ptr::eq(Arc::as_ptr(&retained.asset), std::ptr::from_ref(current))
        })
    }

    /// Derives the corpus-policy fixed-ID prepared collection from this
    /// batch's retained exact assets, without taking another resident
    /// snapshot. The resident container itself supplies no collection ID.
    pub(super) fn try_prepare_collection(
        &self,
    ) -> Result<PreparedSimulationCollection, SimulationDmoBankPreparedCollectionError> {
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

        let requested = self.inner.assets.len();
        if requested != DMO_BANK_ASSET_COUNT as usize {
            return Err(
                SimulationDmoBankPreparedCollectionError::CorpusAssetCountMismatch {
                    actual: requested,
                },
            );
        }
        if self.inner.expected.len() != DMO_BANK_DMO_INSTANCE_COUNT {
            return Err(
                SimulationDmoBankPreparedCollectionError::CorpusDmoInstanceCountMismatch {
                    actual: self.inner.expected.len(),
                },
            );
        }
        let first_asset_id = self
            .inner
            .assets
            .first()
            .map(|retained| retained.asset.plan().static_plan().asset().decoded.asset_id);
        let last_asset_id = self
            .inner
            .assets
            .last()
            .map(|retained| retained.asset.plan().static_plan().asset().decoded.asset_id);
        if first_asset_id != Some(DMO_BANK_FIRST_ASSET_ID)
            || last_asset_id != Some(DMO_BANK_LAST_ASSET_ID)
        {
            return Err(SimulationDmoBankPreparedCollectionError::CorpusAssetIdSequenceMismatch);
        }
        let mut signature = FNV_OFFSET;
        for (ordinal, retained) in self.inner.assets.iter().enumerate() {
            let asset_id = retained.asset.plan().static_plan().asset().decoded.asset_id;
            if let Some(first_ordinal) = self.inner.assets[..ordinal].iter().position(|previous| {
                previous.asset.plan().static_plan().asset().decoded.asset_id == asset_id
            }) {
                return Err(
                    SimulationDmoBankPreparedCollectionError::CorpusAssetIdDuplicate {
                        first_ordinal,
                        second_ordinal: ordinal,
                    },
                );
            }
            for byte in asset_id.to_be_bytes() {
                signature ^= u64::from(byte);
                signature = signature.wrapping_mul(FNV_PRIME);
            }
        }
        if signature != DMO_BANK_ASSET_ID_SIGNATURE {
            return Err(SimulationDmoBankPreparedCollectionError::CorpusAssetIdSequenceMismatch);
        }
        let mut plans = Vec::new();
        plans.try_reserve_exact(requested).map_err(|_| {
            SimulationDmoBankPreparedCollectionError::PlanOwnerStorageAllocation { requested }
        })?;
        plans.extend(
            self.inner
                .assets
                .iter()
                .map(|retained| retained.asset.plan_owner()),
        );
        PreparedSimulationCollection::try_new(DMO_BANK_COLLECTION_ID, plans)
            .map_err(SimulationDmoBankPreparedCollectionError::Collection)
    }
}

/// One non-cloneable safe-host cursor through a sealed model-registration snapshot.
///
/// Retaining the exact snapshot after the outer batch value is dropped is Rust
/// lifetime policy. Native `RegArena`/`DRegArena` instead initialize local
/// iterators from each arena's begin pointer on every call; they expose no
/// cursor object, cloning/forking rule, or pass-outliving contract. This cursor
/// is deliberately not debuggable and carries no manager authority.
#[must_use = "a DMO model registration pass must be finished"]
pub(super) struct SimulationDmoBankModelRegistrationPass {
    inner: Arc<SimulationDmoBankModelRegistrationBatchInner>,
    next: usize,
}

impl SimulationDmoBankModelRegistrationPass {
    fn require_resident(&self) -> Result<(), SimulationDmoBankModelRegistrationPassError> {
        match &self.inner.resident {
            SimulationDmoBankModelRegistrationResident::Published(liveness) => liveness
                .try_require_alive()
                .map_err(|source| match source {
                    SimulationMainDmoBankResidentLivenessError::RegistryDetached
                    | SimulationMainDmoBankResidentLivenessError::UnknownOccurrence => {
                        SimulationDmoBankModelRegistrationPassError::ResidentExpired
                    }
                    SimulationMainDmoBankResidentLivenessError::RegistryPoisoned => {
                        SimulationDmoBankModelRegistrationPassError::PublishedResidentRegistryPoisoned
                    }
                    SimulationMainDmoBankResidentLivenessError::Resident(source) => match source {
                        OwnedResidentCollectionError::NotAlive {
                            operation: OwnedResidentCollectionOperation::Snapshot,
                        } => SimulationDmoBankModelRegistrationPassError::ResidentExpired,
                        source => {
                            SimulationDmoBankModelRegistrationPassError::ResidentAccess(source)
                        }
                    },
                }),
            #[cfg(test)]
            SimulationDmoBankModelRegistrationResident::Synthetic(collection) => collection
                .try_require_alive()
                .map_err(|source| match source {
                    OwnedResidentCollectionError::NotAlive {
                        operation: OwnedResidentCollectionOperation::Snapshot,
                    } => SimulationDmoBankModelRegistrationPassError::ResidentExpired,
                    source => SimulationDmoBankModelRegistrationPassError::ResidentAccess(source),
                }),
        }
    }

    /// Validates one actual callback without advancing on any refusal, then
    /// mints a fresh consumed source bound to the batch's exact asset owner.
    /// Liveness is sampled only during this call; an issued source may outlive
    /// that point-in-time sample, its pass, the batch, and resident teardown.
    /// Such delayed ownership does not prove a live native dispatch.
    pub(super) fn try_issue(
        &mut self,
        actual_source: &SimulationRecordHandle,
        actual_primary_asset_id: u64,
    ) -> Result<SimulationDmoModelRegistrationSource, SimulationDmoBankModelRegistrationPassError>
    {
        self.require_resident()?;
        let expected = self.inner.expected.get(self.next).ok_or(
            SimulationDmoBankModelRegistrationPassError::Extra {
                issued: self.next,
                expected: self.inner.expected.len(),
            },
        )?;
        if !expected.source.is_same_backing_record(actual_source) {
            return Err(
                SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: self.next },
            );
        }
        if expected.primary_asset_id != actual_primary_asset_id {
            return Err(
                SimulationDmoBankModelRegistrationPassError::PrimaryAssetIdMismatch {
                    ordinal: self.next,
                },
            );
        }
        let source = SimulationDmoModelRegistrationSource::try_prepare(
            Arc::clone(&self.inner.assets[expected.asset_index].asset),
            expected.operation_index,
        )
        .map_err(|source| {
            SimulationDmoBankModelRegistrationPassError::SourcePreparation {
                ordinal: self.next,
                source,
            }
        })?;
        self.next += 1;
        Ok(source)
    }

    /// Requires every expected DMO callback to have issued exactly once.
    /// This is a completeness-only check, not a resident-liveness attestation;
    /// success after resident destruction is therefore intentional.
    pub(super) fn finish(self) -> Result<(), SimulationDmoBankModelRegistrationPassError> {
        if self.next != self.inner.expected.len() {
            return Err(SimulationDmoBankModelRegistrationPassError::Incomplete {
                issued: self.next,
                expected: self.inner.expected.len(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::{CollectionId, ResidentCollectionStorage},
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        owned_resident_collection::OwnedResidentCollectionStorage,
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_dmo_assembly_part_session::{
            SimulationDmoModelRegistry, SimulationDmoModelRegistryResolveError,
        },
        simulation_normal_arena_plan::{
            SimulationNormalArenaPlan, SimulationRecordSourceKind, SimulationRegistryAssetKind,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };

    struct OperationSpec {
        operation: SimulationNormalOperation,
        source: Option<(SimulationRecordSourceKind, Vec<u8>)>,
    }

    type TestStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;

    fn identity_error(error: OwnedResidentCollectionError) -> OwnedResidentCollectionError {
        error
    }

    fn dmo_record(primary_asset_id: u64, local_guid: u64) -> Vec<u8> {
        let mut record = vec![0; 0xa0];
        record[0x60..0x68].copy_from_slice(&primary_asset_id.to_be_bytes());
        record[0x68..0x70].copy_from_slice(&local_guid.to_be_bytes());
        record
    }

    fn dmo(primary_asset_id: u64, local_guid: u64) -> OperationSpec {
        OperationSpec {
            operation: SimulationNormalOperation::DmoInstance {
                asset_id: primary_asset_id,
                local_guid,
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_record_offset: 0,
                raw_object_id: 0,
            },
            source: Some((
                SimulationRecordSourceKind::DmoInstance,
                dmo_record(primary_asset_id, local_guid),
            )),
        }
    }

    fn malformed_dmo(
        primary_asset_id: u64,
        operation_local_guid: u64,
        record_local_guid: u64,
    ) -> OperationSpec {
        let mut spec = dmo(primary_asset_id, operation_local_guid);
        spec.source = Some((
            SimulationRecordSourceKind::DmoInstance,
            dmo_record(primary_asset_id, record_local_guid),
        ));
        spec
    }

    fn non_dmo(asset_id: u64) -> OperationSpec {
        OperationSpec {
            operation: SimulationNormalOperation::RegistryAsset {
                kind: SimulationRegistryAssetKind::WorldPainter,
                asset_id,
                source_toc_index: 0,
                source_dictionary_index: 0,
                raw_object_type: 0,
                raw_object_id: 0,
            },
            source: None,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one exact owner fixture keeps each operation, record span, and arena allocation paired"
    )]
    fn asset(
        asset_id: u64,
        specs: Vec<OperationSpec>,
    ) -> (DmoBankProcessedArenaAsset, Arc<SimulationNormalArenaPlan>) {
        let mut resource_zero = Vec::new();
        let mut operations = Vec::new();
        let mut sources = Vec::new();
        for spec in specs {
            operations.push(spec.operation);
            let source = spec.source.map(|(kind, record)| {
                let start = resource_zero.len();
                let len = record.len();
                resource_zero.extend_from_slice(&record);
                (kind, 0, start, len)
            });
            sources.push(source);
        }
        let resources = [
            resource_zero,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        let resource_descriptor = std::array::from_fn(|index| ResourceDescriptor {
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
            id: 0x1234_5678,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 0x10,
            virtual_size: resource_descriptor[0].size,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor,
            resources_used: resource_descriptor,
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let asset_resource_descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
            size: u32::try_from(resources[index].len()).unwrap(),
            alignment: 0x10,
        });
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset {
                asset_id,
                asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
                asset_resource_descriptors,
                resources: ArenaResources { resources },
                arena,
            },
            collision: CollisionArena {
                id: 0x1234_5678,
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
            collision,
            Vec::new(),
        ));
        let plan = Arc::new(
            SimulationNormalArenaPlan::from_parts_with_record_sources_for_test(
                static_plan,
                operations,
                sources,
            ),
        );
        (
            DmoBankProcessedArenaAsset::from_parts_for_test(Arc::clone(&plan), Vec::new()),
            plan,
        )
    }

    fn collection(
        assets: Vec<DmoBankProcessedArenaAsset>,
    ) -> (
        TestStorage,
        OwnedResidentCollection<DmoBankProcessedArenaAsset>,
    ) {
        let mut storage = TestStorage::new(identity_error);
        let collection = storage
            .create_empty(i32::try_from(assets.len()).unwrap())
            .unwrap();
        let mut writer = storage.writer(&collection).unwrap();
        for asset in assets {
            writer.add_asset(asset);
        }
        drop(writer);
        (storage, collection)
    }

    #[test]
    fn batch_keeps_two_asset_authored_dmo_subsequence_and_safe_debug() {
        const A0: u64 = 0x1111_0000_0000_0001;
        const A1: u64 = 0x1111_0000_0000_0002;
        const B0: u64 = 0x2222_0000_0000_0001;
        let (asset_a, plan_a) = asset(
            0xa0,
            vec![non_dmo(9), dmo(A0, 0xa001), non_dmo(10), dmo(A1, 0xa002)],
        );
        let (asset_b, plan_b) = asset(0xb0, vec![dmo(B0, 0xb001), non_dmo(11)]);
        let (_storage, collection) = collection(vec![asset_a, asset_b]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        assert_eq!(
            format!("{batch:?}"),
            "SimulationDmoBankModelRegistrationBatch { asset_count: 2, source_count: 3, resident: true, .. }"
        );

        let mut pass = batch.begin_pass();
        drop(
            pass.try_issue(&plan_a.record_handle(1).unwrap(), A0)
                .unwrap(),
        );
        drop(
            pass.try_issue(&plan_a.record_handle(3).unwrap(), A1)
                .unwrap(),
        );
        drop(
            pass.try_issue(&plan_b.record_handle(0).unwrap(), B0)
                .unwrap(),
        );
        pass.finish().unwrap();
    }

    #[test]
    fn cursor_rejects_full_width_and_exact_record_mismatches_without_advancing() {
        const PRIMARY: u64 = 0x1111_0000_dead_beef;
        const WRONG_HIGH: u64 = 0x2222_0000_dead_beef;
        const LOCAL: u64 = 0x3333_4444_5555_6666;
        let (resident_asset, plan) = asset(0xa0, vec![dmo(PRIMARY, LOCAL), dmo(PRIMARY, LOCAL)]);
        let (foreign_asset, foreign_plan) =
            asset(0xa0, vec![dmo(PRIMARY, LOCAL), dmo(PRIMARY, LOCAL)]);
        drop(foreign_asset);
        let (_storage, collection) = collection(vec![resident_asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let first = plan.record_handle(0).unwrap();
        let second = plan.record_handle(1).unwrap();
        let foreign = foreign_plan.record_handle(0).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.bytes(), foreign.bytes());

        let mut pass = batch.begin_pass();
        assert!(matches!(
            pass.try_issue(&foreign, PRIMARY),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        assert!(matches!(
            pass.try_issue(&foreign, WRONG_HIGH),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        assert!(matches!(
            pass.try_issue(&second, PRIMARY),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        assert!(matches!(
            pass.try_issue(&first, WRONG_HIGH),
            Err(SimulationDmoBankModelRegistrationPassError::PrimaryAssetIdMismatch { ordinal: 0 })
        ));
        drop(pass.try_issue(&first, PRIMARY).unwrap());
        assert!(matches!(
            pass.finish(),
            Err(SimulationDmoBankModelRegistrationPassError::Incomplete {
                issued: 1,
                expected: 2
            })
        ));

        let mut complete = batch.begin_pass();
        drop(complete.try_issue(&first, PRIMARY).unwrap());
        drop(complete.try_issue(&second, PRIMARY).unwrap());
        complete.finish().unwrap();
    }

    #[test]
    fn empty_batch_finishes_and_rejects_an_extra_callback() {
        let (_storage, collection) = collection(Vec::new());
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        assert_eq!(
            format!("{batch:?}"),
            "SimulationDmoBankModelRegistrationBatch { asset_count: 0, source_count: 0, resident: true, .. }"
        );
        batch.begin_pass().finish().unwrap();

        let (foreign_asset, foreign_plan) = asset(0xb0, vec![dmo(1, 2)]);
        drop(foreign_asset);
        let mut extra = batch.begin_pass();
        assert!(matches!(
            extra.try_issue(&foreign_plan.record_handle(0).unwrap(), 1),
            Err(SimulationDmoBankModelRegistrationPassError::Extra {
                issued: 0,
                expected: 0
            })
        ));

        collection.tombstone_without_releasing_assets_for_test();
        let mut expired = batch.begin_pass();
        assert!(matches!(
            expired.try_issue(&foreign_plan.record_handle(0).unwrap(), 1),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
        batch.begin_pass().finish().unwrap();
    }

    #[test]
    fn later_source_preparation_failure_refuses_the_whole_batch() {
        let (first, _first_plan) = asset(0xa0, vec![dmo(1, 2)]);
        let (second, _second_plan) = asset(0xb0, vec![non_dmo(9), malformed_dmo(3, 4, 5)]);
        let (_storage, collection) = collection(vec![first, second]);
        assert!(matches!(
            SimulationDmoBankModelRegistrationBatch::try_new(&collection),
            Err(
                SimulationDmoBankModelRegistrationBatchError::SourcePreparation {
                    asset_index: 1,
                    operation_index: 1,
                    source: SimulationDmoModelRegistrationSourceError::LocalGuidMismatch {
                        operation_index: 1
                    }
                }
            )
        ));

        collection.tombstone_without_releasing_assets_for_test();
        assert!(matches!(
            SimulationDmoBankModelRegistrationBatch::try_new(&collection),
            Err(
                SimulationDmoBankModelRegistrationBatchError::ResidentSnapshot(
                    OwnedResidentCollectionError::NotAlive {
                        operation: OwnedResidentCollectionOperation::Snapshot
                    }
                )
            )
        ));
    }

    #[test]
    fn fresh_passes_mint_exact_register_and_deregister_sources() {
        const PRIMARY: u64 = 0xaaaa_bbbb_cccc_dddd;
        const LOCAL: u64 = 0x1111_2222_3333_4444;
        let (asset, plan) = asset(0xa0, vec![dmo(PRIMARY, LOCAL)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let record = plan.record_handle(0).unwrap();
        let mut models = SimulationDmoModelRegistry::try_new().unwrap();

        let mut register = batch.begin_pass();
        models
            .register(register.try_issue(&record, PRIMARY).unwrap())
            .unwrap();
        register.finish().unwrap();
        assert!(models.resolver().try_current(LOCAL).is_ok());

        let mut deregister = batch.begin_pass();
        models.deregister(deregister.try_issue(&record, PRIMARY).unwrap());
        deregister.finish().unwrap();
        assert!(matches!(
            models.resolver().try_current(LOCAL),
            Err(SimulationDmoModelRegistryResolveError::MissingWinner)
        ));

        let mut register_again = batch.begin_pass();
        models
            .register(register_again.try_issue(&record, PRIMARY).unwrap())
            .unwrap();
        register_again.finish().unwrap();
        assert!(models.resolver().try_current(LOCAL).is_ok());
    }

    #[test]
    fn destroyed_resident_collection_blocks_issue_but_batch_keeps_safe_counts() {
        const PRIMARY: u64 = 0x0102_0304_0506_0708;
        let (asset, plan) = asset(0xa0, vec![dmo(PRIMARY, 9)]);
        let (mut storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let record = plan.record_handle(0).unwrap();
        storage.destroy(collection);
        assert_eq!(
            format!("{batch:?}"),
            "SimulationDmoBankModelRegistrationBatch { asset_count: 1, source_count: 1, resident: false, .. }"
        );
        let mut pass = batch.begin_pass();
        assert!(matches!(
            pass.try_issue(&record, PRIMARY),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
        assert!(matches!(
            pass.finish(),
            Err(SimulationDmoBankModelRegistrationPassError::Incomplete {
                issued: 0,
                expected: 1
            })
        ));
    }

    #[test]
    fn batch_retains_assets_after_collection_teardown_until_batch_drop() {
        let (asset_a, plan_a) = asset(0xa0, vec![dmo(1, 2)]);
        let (asset_b, plan_b) = asset(0xb0, vec![dmo(3, 4)]);
        let weak_a = Arc::downgrade(&plan_a);
        let weak_b = Arc::downgrade(&plan_b);
        let (mut storage, collection) = collection(vec![asset_a, asset_b]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        drop(plan_a);
        drop(plan_b);
        storage.destroy(collection);
        assert!(weak_a.upgrade().is_some());
        assert!(weak_b.upgrade().is_some());
        drop(batch);
        assert!(weak_a.upgrade().is_none());
        assert!(weak_b.upgrade().is_none());
    }

    #[test]
    fn issued_source_supports_delayed_exact_manager_use_after_every_issuer_owner_drops() {
        const PRIMARY: u64 = 0xaaaa_0000_dead_beef;
        const LOCAL: u64 = 0xbbbb_0000_cafe_babe;
        let (asset, plan) = asset(0xa0, vec![dmo(PRIMARY, LOCAL)]);
        let (mut storage, collection) = collection(vec![asset]);
        let weak_asset = collection.weak_assets_in_order().unwrap()[0].weak().clone();
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut pass = batch.begin_pass();
        let issued = pass
            .try_issue(&plan.record_handle(0).unwrap(), PRIMARY)
            .unwrap();
        pass.finish().unwrap();

        storage.destroy(collection);
        drop(batch);
        drop(plan);
        assert!(weak_asset.upgrade().is_some());

        let mut models = SimulationDmoModelRegistry::try_new().unwrap();
        models.register(issued).unwrap();
        assert!(models.resolver().try_current(LOCAL).is_ok());
        let exact_asset = weak_asset.upgrade().unwrap();
        models
            .deregister(SimulationDmoModelRegistrationSource::try_prepare(exact_asset, 0).unwrap());
        assert!(matches!(
            models.resolver().try_current(LOCAL),
            Err(SimulationDmoModelRegistryResolveError::MissingWinner)
        ));
        assert!(weak_asset.upgrade().is_none());
    }

    #[test]
    fn collection_ids_do_not_replace_authored_asset_order() {
        let (first, first_plan) = asset(0xff, vec![dmo(1, 2)]);
        let (second, second_plan) = asset(0x01, vec![dmo(3, 4)]);
        let (_storage, collection) = collection(vec![first, second]);
        let snapshot = collection.weak_assets_in_order().unwrap();
        assert_eq!(snapshot[0].id(), CollectionId(0xff));
        assert_eq!(snapshot[1].id(), CollectionId(0x01));
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut pass = batch.begin_pass();
        drop(
            pass.try_issue(&first_plan.record_handle(0).unwrap(), 1)
                .unwrap(),
        );
        drop(
            pass.try_issue(&second_plan.record_handle(0).unwrap(), 3)
                .unwrap(),
        );
        pass.finish().unwrap();
    }

    #[test]
    fn owned_cursor_remains_exact_after_outer_batch_drop() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut cursor = batch.begin_pass();
        drop(batch);

        assert!(matches!(
            cursor.try_issue(&plan.record_handle(1).unwrap(), 2),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        drop(
            cursor
                .try_issue(&plan.record_handle(0).unwrap(), 1)
                .unwrap(),
        );
        drop(
            cursor
                .try_issue(&plan.record_handle(1).unwrap(), 2)
                .unwrap(),
        );
        cursor.finish().unwrap();
    }

    #[test]
    fn fresh_owned_cursors_start_at_zero_and_resample_residence() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut first = batch.begin_pass();
        let mut fresh = batch.begin_pass();
        drop(first.try_issue(&plan.record_handle(0).unwrap(), 1).unwrap());
        assert!(matches!(
            fresh.try_issue(&plan.record_handle(1).unwrap(), 2),
            Err(SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 0 })
        ));
        drop(fresh.try_issue(&plan.record_handle(0).unwrap(), 1).unwrap());

        collection.tombstone_without_releasing_assets_for_test();
        assert!(matches!(
            first.try_issue(&plan.record_handle(1).unwrap(), 2),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
        assert!(matches!(
            fresh.try_issue(&plan.record_handle(1).unwrap(), 2),
            Err(SimulationDmoBankModelRegistrationPassError::ResidentExpired)
        ));
    }
}
