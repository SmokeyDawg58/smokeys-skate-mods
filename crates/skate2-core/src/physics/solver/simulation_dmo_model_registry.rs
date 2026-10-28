//! Manager-scoped ownership corresponding to the native manager-global DMO
//! model table.
//!
//! `Sk8::Dmo::Registration::RegDmoModel` (`0x825eab98`) keys the table by the
//! exact `pegasus::tInstance` record's full local GUID at `+0x68`, keeps the
//! first pointer for a duplicate key, and has capacity 256.
//! `Sk8::Dmo::Registration::DRegDmoModel` (`0x825ea628`) erases only when the
//! full key and stored record pointer both match. The passed primary `AssetID` is
//! unused by both native functions and is therefore absent from this API.
//!
//! Native registration loads `+0x68` once before lookup and again before
//! insertion. This immutable safe-host source samples it once during
//! effect-free preparation; it does not claim parity for mutation between
//! those two native loads. Operation kind, retained-source presence, record
//! kind/size, and operation-versus-record local-GUID consistency are safe-host
//! validation around the recovered raw record access; the retained plan has
//! already bounded that record span. This staged source also accepts only the
//! processed main-DMO-bank owner. Native `RegArena` can forward any qualifying
//! kind-11 row from a normal arena, so arbitrary normal-arena admission remains
//! an explicit later integration boundary. This component has no `clear`: the
//! recovered manager `Clear` destroys locators only, while the parent combined
//! registration manager owns the recovered locator-before-model release order.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged manager-scoped model owner awaits live DMO registration wiring"
    )
)]

use crate::{
    dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
    simulation_normal_arena_plan::{
        SimulationNormalOperation, SimulationRecordHandle, SimulationRecordSourceKind,
    },
};
use std::{
    error::Error,
    fmt,
    sync::{Arc, Weak},
};

const DMO_MODEL_REGISTRY_CAPACITY: usize = 256;
const DMO_MODEL_RECORD_SIZE: usize = 0xa0;
const DMO_MODEL_LOCAL_GUID_OFFSET: usize = 0x68;

/// Effect-free refusal while binding one processed-asset operation to its
/// exact retained `tInstance` record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoModelRegistrationSourceError {
    OperationOutOfRange {
        operation_index: usize,
        operation_count: usize,
    },
    WrongOperationKind {
        operation_index: usize,
    },
    MissingRecordSource {
        operation_index: usize,
    },
    WrongRecordKind {
        operation_index: usize,
    },
    WrongRecordSize {
        operation_index: usize,
        actual: usize,
    },
    LocalGuidMismatch {
        operation_index: usize,
    },
}

impl fmt::Display for SimulationDmoModelRegistrationSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperationOutOfRange {
                operation_index,
                operation_count,
            } => write!(
                formatter,
                "Simulation DMO model operation {operation_index} is outside {operation_count} operations"
            ),
            Self::WrongOperationKind { operation_index } => write!(
                formatter,
                "Simulation operation {operation_index} is not a DMO model"
            ),
            Self::MissingRecordSource { operation_index } => write!(
                formatter,
                "Simulation DMO model operation {operation_index} has no retained record"
            ),
            Self::WrongRecordKind { operation_index } => write!(
                formatter,
                "Simulation DMO model operation {operation_index} has the wrong retained record kind"
            ),
            Self::WrongRecordSize {
                operation_index,
                actual,
            } => write!(
                formatter,
                "Simulation DMO model operation {operation_index} has a {actual}-byte record"
            ),
            Self::LocalGuidMismatch { operation_index } => write!(
                formatter,
                "Simulation DMO model operation {operation_index} disagrees with its retained local GUID"
            ),
        }
    }
}

impl Error for SimulationDmoModelRegistrationSourceError {}

/// Opaque, non-cloneable processed-asset occurrence for one exact model
/// registration call. It exposes neither its key, record, nor asset owner.
pub(crate) struct SimulationDmoModelRegistrationSource {
    asset: Arc<DmoBankProcessedArenaAsset>,
    source: SimulationRecordHandle,
    model_key: u64,
}

impl SimulationDmoModelRegistrationSource {
    /// Binds one exact processed operation before manager effects.
    pub(crate) fn try_prepare(
        asset: Arc<DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) -> Result<Self, SimulationDmoModelRegistrationSourceError> {
        let operation = asset.plan().operations().get(operation_index).ok_or(
            SimulationDmoModelRegistrationSourceError::OperationOutOfRange {
                operation_index,
                operation_count: asset.plan().operations().len(),
            },
        )?;
        let SimulationNormalOperation::DmoInstance { local_guid, .. } = *operation else {
            return Err(
                SimulationDmoModelRegistrationSourceError::WrongOperationKind { operation_index },
            );
        };
        let source = asset.plan().record_handle(operation_index).ok_or(
            SimulationDmoModelRegistrationSourceError::MissingRecordSource { operation_index },
        )?;
        if source.kind() != SimulationRecordSourceKind::DmoInstance {
            return Err(SimulationDmoModelRegistrationSourceError::WrongRecordKind {
                operation_index,
            });
        }
        if source.bytes().len() != DMO_MODEL_RECORD_SIZE {
            return Err(SimulationDmoModelRegistrationSourceError::WrongRecordSize {
                operation_index,
                actual: source.bytes().len(),
            });
        }
        let model_key = read_local_guid(&source);
        if model_key != local_guid {
            return Err(
                SimulationDmoModelRegistrationSourceError::LocalGuidMismatch { operation_index },
            );
        }
        Ok(Self {
            asset,
            source,
            model_key,
        })
    }
}

fn read_local_guid(source: &SimulationRecordHandle) -> u64 {
    u64::from_be_bytes(
        source.bytes()[DMO_MODEL_LOCAL_GUID_OFFSET..DMO_MODEL_LOCAL_GUID_OFFSET + 8]
            .try_into()
            .expect("validated DMO model record span"),
    )
}

struct RegisteredDmoModel {
    model_key: u64,
    source: SimulationRecordHandle,
    asset: Arc<DmoBankProcessedArenaAsset>,
}

/// Borrowed, non-forgeable access to one registry's current winner lookup.
/// It exposes neither the registry nor a raw-key lookup result.
pub(crate) struct SimulationDmoModelRegistryResolver<'a> {
    registry: &'a SimulationDmoModelRegistry,
}

/// Borrow of one current winning row. Candidate catalogs can test exact
/// identity through its narrow predicates without extracting the key, record,
/// or processed-asset owner.
pub(crate) struct SimulationDmoModelCurrentWinner<'a> {
    row: &'a RegisteredDmoModel,
}

impl SimulationDmoModelCurrentWinner<'_> {
    pub(super) fn matches_key(&self, candidate: u64) -> bool {
        self.row.model_key == candidate
    }

    pub(super) fn matches_catalog_route(
        &self,
        candidate_asset: &Weak<DmoBankProcessedArenaAsset>,
        candidate_operation_index: usize,
    ) -> bool {
        if !candidate_asset.ptr_eq(&Arc::downgrade(&self.row.asset)) {
            return false;
        }
        self.row
            .asset
            .plan()
            .record_handle(candidate_operation_index)
            .is_some_and(|candidate| self.row.source.is_same_backing_record(&candidate))
    }
}

/// Keyless refusal when a locator's private model key has no manager winner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoModelRegistryResolveError {
    MissingWinner,
}

impl fmt::Display for SimulationDmoModelRegistryResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWinner => formatter.write_str("DMO model has no manager winner"),
        }
    }
}

impl Error for SimulationDmoModelRegistryResolveError {}

impl<'a> SimulationDmoModelRegistryResolver<'a> {
    /// Borrows the current winner selected by a locator's private `+0x70` key.
    /// The resolver itself can only be minted by the parent bridge.
    pub(crate) fn try_current(
        self,
        model_key: u64,
    ) -> Result<SimulationDmoModelCurrentWinner<'a>, SimulationDmoModelRegistryResolveError> {
        let row = self
            .registry
            .rows
            .iter()
            .find(|row| row.model_key == model_key)
            .ok_or(SimulationDmoModelRegistryResolveError::MissingWinner)?;
        Ok(SimulationDmoModelCurrentWinner { row })
    }
}

/// Construction refusal for the fixed-capacity model registry storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoModelRegistryInitError {
    StorageAllocation { requested: usize },
}

impl fmt::Display for SimulationDmoModelRegistryInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageAllocation { requested } => {
                write!(formatter, "could not reserve {requested} DMO model rows")
            }
        }
    }
}

impl Error for SimulationDmoModelRegistryInitError {}

/// Safe-host translation of the native `FailAssert`/`DebugBreak` path when a
/// new, nonduplicate model registration exceeds the recovered capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoModelRegisterError {
    CapacityExceeded { capacity: usize },
}

impl fmt::Display for SimulationDmoModelRegisterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { capacity } => {
                write!(
                    formatter,
                    "DMO model registry capacity {capacity} is exhausted"
                )
            }
        }
    }
}

impl Error for SimulationDmoModelRegisterError {}

/// Fixed-capacity manager-scoped DMO model registration owner.
///
/// It deliberately exposes no iterator, count, key, source, or native hash
/// traversal representation. The retained processed asset is a safe-host
/// lifetime owner for the exact record pointer represented by each row.
pub(super) struct SimulationDmoModelRegistry {
    rows: Vec<RegisteredDmoModel>,
}

impl SimulationDmoModelRegistry {
    /// Preallocates all 256 possible manager rows before publication.
    pub(super) fn try_new() -> Result<Self, SimulationDmoModelRegistryInitError> {
        let mut rows = Vec::new();
        rows.try_reserve_exact(DMO_MODEL_REGISTRY_CAPACITY)
            .map_err(|_| SimulationDmoModelRegistryInitError::StorageAllocation {
                requested: DMO_MODEL_REGISTRY_CAPACITY,
            })?;
        Ok(Self { rows })
    }

    /// Mints one borrowed lookup capability for the sealed locator bridge.
    pub(super) fn resolver(&self) -> SimulationDmoModelRegistryResolver<'_> {
        SimulationDmoModelRegistryResolver { registry: self }
    }

    /// Keeps the first exact model source for a full local-GUID key. Duplicate
    /// lookup precedes the capacity refusal.
    pub(super) fn register(
        &mut self,
        source: SimulationDmoModelRegistrationSource,
    ) -> Result<(), SimulationDmoModelRegisterError> {
        if self
            .rows
            .iter()
            .any(|row| row.model_key == source.model_key)
        {
            return Ok(());
        }
        if self.rows.len() == DMO_MODEL_REGISTRY_CAPACITY {
            return Err(SimulationDmoModelRegisterError::CapacityExceeded {
                capacity: DMO_MODEL_REGISTRY_CAPACITY,
            });
        }
        let SimulationDmoModelRegistrationSource {
            asset,
            source,
            model_key,
        } = source;
        self.rows.push(RegisteredDmoModel {
            model_key,
            source,
            asset,
        });
        Ok(())
    }

    /// Erases only when the source's full local key and pointer-equivalent
    /// retained record both match the winning row. Missing keys and pointer
    /// mismatches are intentional no-ops.
    pub(super) fn deregister(&mut self, source: SimulationDmoModelRegistrationSource) {
        let SimulationDmoModelRegistrationSource {
            asset,
            source,
            model_key,
        } = source;
        if let Some(index) = self.rows.iter().position(|row| {
            row.model_key == model_key && row.source.is_same_backing_record(&source)
        }) {
            self.rows.remove(index);
        }
        drop(asset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_normal_arena_plan::{SimulationNormalArenaPlan, SimulationRegistryAssetKind},
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::ArenaResources,
    };

    struct SourceSpec {
        operation: SimulationNormalOperation,
        source_kind: Option<SimulationRecordSourceKind>,
        record: Vec<u8>,
    }

    fn model_record(primary_guid: u64, local_guid: u64) -> Vec<u8> {
        let mut record = vec![0; DMO_MODEL_RECORD_SIZE];
        record[0x60..0x68].copy_from_slice(&primary_guid.to_be_bytes());
        record[0x68..0x70].copy_from_slice(&local_guid.to_be_bytes());
        record
    }

    fn dmo_spec(primary_guid: u64, local_guid: u64) -> SourceSpec {
        SourceSpec {
            operation: SimulationNormalOperation::DmoInstance {
                asset_id: primary_guid,
                local_guid,
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_record_offset: 0,
                raw_object_id: 0,
            },
            source_kind: Some(SimulationRecordSourceKind::DmoInstance),
            record: model_record(primary_guid, local_guid),
        }
    }

    fn model_asset(asset_id: u64, pairs: &[(u64, u64)]) -> Arc<DmoBankProcessedArenaAsset> {
        model_asset_from_specs(
            asset_id,
            pairs
                .iter()
                .map(|&(primary, local)| dmo_spec(primary, local))
                .collect(),
        )
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one coherent owner fixture preserves every record locator and arena allocation"
    )]
    fn model_asset_from_specs(
        asset_id: u64,
        specs: Vec<SourceSpec>,
    ) -> Arc<DmoBankProcessedArenaAsset> {
        let mut resource_zero = Vec::new();
        let mut operations = Vec::new();
        let mut sources = Vec::new();
        for spec in specs {
            operations.push(spec.operation);
            let start = resource_zero.len();
            resource_zero.extend_from_slice(&spec.record);
            sources.push(
                spec.source_kind
                    .map(|kind| (kind, 0, start, spec.record.len())),
            );
        }
        let descriptor = ResourceDescriptor {
            size: u32::try_from(resource_zero.len()).unwrap(),
            alignment: 0x10,
        };
        let empty = ResourceDescriptor {
            size: 0,
            alignment: 0x10,
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
            id: 0x1234_5678,
            entry_count: 0,
            used_entry_count: 0,
            alignment: 0x10,
            virtual_size: descriptor.size,
            dictionary_offset: 0,
            sections_offset: 0,
            base_offset: 0,
            unfix_context_offset: 0,
            fix_context_offset: 0,
            resource_descriptor: [descriptor, empty, empty, empty, empty],
            resources_used: [descriptor, empty, empty, empty, empty],
            resource_offsets: [u32::MAX; 5],
            arena_group_offset: 0,
            dictionary: Vec::new(),
        };
        let collision = SimulationCollisionAsset {
            decoded: DecodedArenaAsset::from_parts_for_test(
                asset_id,
                WORLD_STREAM_ARENA_ASSET_TYPE,
                ArenaResources {
                    resources: [
                        resource_zero,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    ],
                },
                arena,
            ),
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
        Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
            plan,
            Vec::new(),
        ))
    }

    fn source(
        asset: &Arc<DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) -> SimulationDmoModelRegistrationSource {
        SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(asset), operation_index)
            .unwrap()
    }

    fn source_error(
        asset: Arc<DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) -> SimulationDmoModelRegistrationSourceError {
        let Err(error) = SimulationDmoModelRegistrationSource::try_prepare(asset, operation_index)
        else {
            panic!("expected DMO model source preparation to fail");
        };
        error
    }

    #[test]
    fn source_preparation_checks_operation_kind_record_shape_and_local_lane() {
        let valid = model_asset(1, &[(0xaaaa, 0xbbbb)]);
        assert!(SimulationDmoModelRegistrationSource::try_prepare(Arc::clone(&valid), 0).is_ok());
        assert_eq!(
            source_error(valid, 1),
            SimulationDmoModelRegistrationSourceError::OperationOutOfRange {
                operation_index: 1,
                operation_count: 1,
            }
        );

        let wrong_operation = model_asset_from_specs(
            2,
            vec![SourceSpec {
                operation: SimulationNormalOperation::RegistryAsset {
                    kind: SimulationRegistryAssetKind::WorldPainter,
                    asset_id: 1,
                    source_toc_index: 0,
                    source_dictionary_index: 0,
                    raw_object_type: 0,
                    raw_object_id: 0,
                },
                source_kind: None,
                record: Vec::new(),
            }],
        );
        assert_eq!(
            source_error(wrong_operation, 0),
            SimulationDmoModelRegistrationSourceError::WrongOperationKind { operation_index: 0 }
        );

        let mut missing = dmo_spec(1, 2);
        missing.source_kind = None;
        let missing = model_asset_from_specs(3, vec![missing]);
        assert_eq!(
            source_error(missing, 0),
            SimulationDmoModelRegistrationSourceError::MissingRecordSource { operation_index: 0 }
        );

        let mut wrong_kind = dmo_spec(1, 2);
        wrong_kind.source_kind = Some(SimulationRecordSourceKind::TriggerInstance);
        wrong_kind.record.pop();
        let wrong_kind = model_asset_from_specs(4, vec![wrong_kind]);
        assert_eq!(
            source_error(wrong_kind, 0),
            SimulationDmoModelRegistrationSourceError::WrongRecordKind { operation_index: 0 }
        );

        let mut short = dmo_spec(1, 2);
        short.record.pop();
        short.record[0x68..0x70].copy_from_slice(&3_u64.to_be_bytes());
        let short = model_asset_from_specs(5, vec![short]);
        assert_eq!(
            source_error(short, 0),
            SimulationDmoModelRegistrationSourceError::WrongRecordSize {
                operation_index: 0,
                actual: DMO_MODEL_RECORD_SIZE - 1,
            }
        );

        let mut long = dmo_spec(1, 2);
        long.record.push(0);
        let long = model_asset_from_specs(6, vec![long]);
        assert_eq!(
            source_error(long, 0),
            SimulationDmoModelRegistrationSourceError::WrongRecordSize {
                operation_index: 0,
                actual: DMO_MODEL_RECORD_SIZE + 1,
            }
        );

        let mut mismatch = dmo_spec(1, 2);
        mismatch.record[0x68..0x70].copy_from_slice(&3_u64.to_be_bytes());
        let mismatch = model_asset_from_specs(7, vec![mismatch]);
        assert_eq!(
            source_error(mismatch, 0),
            SimulationDmoModelRegistrationSourceError::LocalGuidMismatch { operation_index: 0 }
        );
    }

    #[test]
    fn capacity_is_manager_scoped_across_assets_and_duplicate_precedes_full() {
        let first_pairs = (0_u64..128)
            .map(|key| (0x1000_0000_0000_0000 | key, key))
            .collect::<Vec<_>>();
        let second_pairs = (128_u64..257)
            .map(|key| (0x2000_0000_0000_0000 | key, key))
            .collect::<Vec<_>>();
        let first = model_asset(10, &first_pairs);
        let second = model_asset(11, &second_pairs);
        let duplicate = model_asset(12, &[(0xfeed, 17)]);
        let duplicate_weak = Arc::downgrade(&duplicate);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        assert!(registry.rows.capacity() >= DMO_MODEL_REGISTRY_CAPACITY);
        for ordinal in 0..128 {
            registry.register(source(&first, ordinal)).unwrap();
        }
        for ordinal in 0..128 {
            registry.register(source(&second, ordinal)).unwrap();
        }
        assert_eq!(registry.rows.len(), DMO_MODEL_REGISTRY_CAPACITY);
        registry.register(source(&duplicate, 0)).unwrap();
        drop(duplicate);
        assert!(duplicate_weak.upgrade().is_none());
        let winner = registry
            .rows
            .iter()
            .find(|row| row.model_key == 17)
            .unwrap();
        assert!(
            winner
                .source
                .is_same_backing_record(&first.plan().record_handle(17).unwrap())
        );

        assert_eq!(
            registry.register(source(&second, 128)),
            Err(SimulationDmoModelRegisterError::CapacityExceeded { capacity: 256 })
        );
        assert_eq!(registry.rows.len(), 256);
        assert!(!registry.rows.iter().any(|row| row.model_key == 256));

        registry.deregister(source(&first, 0));
        registry.register(source(&second, 128)).unwrap();
        assert_eq!(registry.rows.len(), 256);
        assert!(!registry.rows.iter().any(|row| row.model_key == 0));
        assert!(registry.rows.iter().any(|row| row.model_key == 256));
    }

    #[test]
    fn full_local_key_not_primary_or_low_word_controls_membership() {
        const FIRST: u64 = 0x1111_0000_dead_beef;
        const SECOND: u64 = 0x2222_0000_dead_beef;
        let asset = model_asset(20, &[(0xaaaa, FIRST), (0xbbbb, SECOND), (FIRST, 0)]);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&asset, 0)).unwrap();
        registry.register(source(&asset, 1)).unwrap();
        registry.register(source(&asset, 2)).unwrap();
        assert!(registry.rows.iter().any(|row| row.model_key == FIRST));
        assert!(registry.rows.iter().any(|row| row.model_key == SECOND));
        assert!(registry.rows.iter().any(|row| row.model_key == 0));
        assert!(!registry.rows.iter().any(|row| row.model_key == 0xaaaa));
        assert!(!registry.rows.iter().any(|row| row.model_key == 0xbbbb));

        registry.deregister(source(&asset, 1));
        assert!(registry.rows.iter().any(|row| row.model_key == FIRST));
        assert!(!registry.rows.iter().any(|row| row.model_key == SECOND));
        assert!(registry.rows.iter().any(|row| row.model_key == 0));
    }

    #[test]
    fn duplicate_loser_pointer_cannot_deregister_byte_equal_winner() {
        let winner = model_asset(30, &[(1, 7)]);
        let loser = model_asset(30, &[(1, 7)]);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&winner, 0)).unwrap();
        registry.register(source(&loser, 0)).unwrap();
        registry.deregister(source(&loser, 0));
        assert_eq!(registry.rows.len(), 1);
        let remaining = registry.rows.iter().find(|row| row.model_key == 7).unwrap();
        assert!(
            remaining
                .source
                .is_same_backing_record(&winner.plan().record_handle(0).unwrap())
        );
        registry.deregister(source(&winner, 0));
        assert!(registry.rows.is_empty());
        registry.deregister(source(&loser, 0));
        assert!(registry.rows.is_empty());
    }

    #[test]
    fn different_record_start_in_one_owner_is_not_the_same_pointer() {
        let asset = model_asset(35, &[(1, 8), (1, 8)]);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&asset, 0)).unwrap();
        registry.register(source(&asset, 1)).unwrap();
        registry.deregister(source(&asset, 1));
        assert_eq!(registry.rows.len(), 1);
        let remaining = registry.rows.iter().find(|row| row.model_key == 8).unwrap();
        assert!(
            remaining
                .source
                .is_same_backing_record(&asset.plan().record_handle(0).unwrap())
        );
        assert!(
            !remaining
                .source
                .is_same_backing_record(&asset.plan().record_handle(1).unwrap())
        );
        registry.deregister(source(&asset, 0));
        assert!(registry.rows.is_empty());
    }

    #[test]
    fn distinct_processed_assets_sharing_one_record_match_pointer_equivalent_identity() {
        let first = model_asset(40, &[(1, 9)]);
        let shared_plan = first.plan_owner();
        let second = Arc::new(DmoBankProcessedArenaAsset::from_parts_for_test(
            shared_plan,
            Vec::new(),
        ));
        assert!(!Arc::ptr_eq(&first, &second));
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&first, 0)).unwrap();
        registry.register(source(&second, 0)).unwrap();
        assert_eq!(registry.rows.len(), 1);
        registry.deregister(source(&second, 0));
        assert!(registry.rows.is_empty());
    }

    #[test]
    fn removal_and_reinsertion_do_not_give_old_backing_aba_authority() {
        let first = model_asset(50, &[(1, 11)]);
        let second = model_asset(50, &[(1, 11)]);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&first, 0)).unwrap();
        registry.deregister(source(&first, 0));
        registry.register(source(&second, 0)).unwrap();
        registry.deregister(source(&first, 0));
        assert_eq!(registry.rows.len(), 1);
        let remaining = registry
            .rows
            .iter()
            .find(|row| row.model_key == 11)
            .unwrap();
        assert!(
            remaining
                .source
                .is_same_backing_record(&second.plan().record_handle(0).unwrap())
        );
        registry.deregister(source(&second, 0));
        assert!(registry.rows.is_empty());
    }

    #[test]
    fn winning_row_retains_complete_processed_asset_until_registry_drop() {
        let asset = model_asset(60, &[(1, 13)]);
        let weak_asset = Arc::downgrade(&asset);
        let mut registry = SimulationDmoModelRegistry::try_new().unwrap();
        registry.register(source(&asset, 0)).unwrap();
        drop(asset);
        assert!(weak_asset.upgrade().is_some());
        let retained = registry
            .rows
            .iter()
            .find(|row| row.model_key == 13)
            .unwrap();
        assert!(Arc::ptr_eq(&retained.asset, &weak_asset.upgrade().unwrap()));
        drop(registry);
        assert!(weak_asset.upgrade().is_none());
    }
}
