//! Synchronous manager effects for one exact prepared main-DMO-bank model
//! callback pass.
//!
//! Registration and deregistration are distinct phase types. Each successful
//! callback immediately applies the manager operation; a first source or
//! manager refusal latches the pass and preserves its already-applied prefix.
//! There is no rollback or drop cleanup. This module is intended only for a
//! future registry stage; it neither invokes nor claims the surrounding
//! collection/`LivingWorld` route, userdata gating, locator wiring, rollback,
//! or global fault behavior.
//! Detached controllers are safe-host ownership tools for a later coordinator;
//! no corresponding native controller object or lifetime is claimed.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged synchronous owner awaits the live DMO registration adapter"
    )
)]

use super::{
    SimulationDmoBankModelRegistrationBatch, SimulationDmoBankModelRegistrationPassError,
    SimulationDmoModelRegisterError, SimulationDmoRegistrationManager,
    SimulationDmoRegistrationManagerInitError,
    model_registration_batch::SimulationDmoBankModelRegistrationPass,
};
use crate::simulation_normal_arena_plan::SimulationRecordHandle;
use std::{error::Error, fmt, sync::Arc};

/// Typed construction refusal for the synchronous DMO registration owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoRegistrationRuntimeInitError {
    Manager(SimulationDmoRegistrationManagerInitError),
}

impl fmt::Display for SimulationDmoRegistrationRuntimeInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manager(source) => {
                write!(
                    formatter,
                    "could not initialize DMO registration runtime: {source}"
                )
            }
        }
    }
}

impl Error for SimulationDmoRegistrationRuntimeInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manager(source) => Some(source),
        }
    }
}

/// Typed refusal from one synchronous model-registration pass.
#[derive(Debug)]
pub(crate) enum SimulationDmoBankModelRegisterPassError {
    RuntimeMismatch,
    AlreadyFaulted,
    Source(SimulationDmoBankModelRegistrationPassError),
    Manager(SimulationDmoModelRegisterError),
}

impl fmt::Display for SimulationDmoBankModelRegisterPassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeMismatch => formatter
                .write_str("DMO model registration controller belongs to another runtime domain"),
            Self::AlreadyFaulted => formatter.write_str("DMO model registration pass faulted"),
            Self::Source(source) => {
                write!(formatter, "DMO model registration source refused: {source}")
            }
            Self::Manager(source) => {
                write!(formatter, "DMO model registration effect refused: {source}")
            }
        }
    }
}

impl Error for SimulationDmoBankModelRegisterPassError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Manager(source) => Some(source),
            Self::RuntimeMismatch | Self::AlreadyFaulted => None,
        }
    }
}

/// Typed refusal from one synchronous model-deregistration pass.
#[derive(Debug)]
pub(crate) enum SimulationDmoBankModelDeregisterPassError {
    RuntimeMismatch,
    AlreadyFaulted,
    Source(SimulationDmoBankModelRegistrationPassError),
}

impl fmt::Display for SimulationDmoBankModelDeregisterPassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeMismatch => formatter
                .write_str("DMO model deregistration controller belongs to another runtime domain"),
            Self::AlreadyFaulted => formatter.write_str("DMO model deregistration pass faulted"),
            Self::Source(source) => {
                write!(
                    formatter,
                    "DMO model deregistration source refused: {source}"
                )
            }
        }
    }
}

impl Error for SimulationDmoBankModelDeregisterPassError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::RuntimeMismatch | Self::AlreadyFaulted => None,
        }
    }
}

/// Private owner of the manager tables used by synchronous prepared passes.
///
/// No manager, registry, source, row, key, or lookup capability escapes.
pub(crate) struct SimulationDmoRegistrationRuntime {
    manager: SimulationDmoRegistrationManager,
    domain: Arc<()>,
}

impl SimulationDmoRegistrationRuntime {
    /// Constructs the combined manager through its typed safe-host initializer.
    pub(crate) fn try_new() -> Result<Self, SimulationDmoRegistrationRuntimeInitError> {
        let manager = SimulationDmoRegistrationManager::try_new()
            .map_err(SimulationDmoRegistrationRuntimeInitError::Manager)?;
        Ok(Self {
            manager,
            domain: Arc::new(()),
        })
    }

    /// Prepares a dormant registration controller bound to this exact runtime
    /// domain. Multiple dormant controllers are allowed; actual manager effects
    /// still serialize through `&mut self`.
    pub(crate) fn begin_model_register_controller(
        &self,
        batch: &SimulationDmoBankModelRegistrationBatch,
    ) -> SimulationDmoBankModelRegisterController {
        SimulationDmoBankModelRegisterController {
            domain: Arc::clone(&self.domain),
            source: batch.begin_pass(),
            faulted: false,
        }
    }

    /// Prepares a dormant deregistration controller bound to this exact
    /// runtime domain.
    pub(crate) fn begin_model_deregister_controller(
        &self,
        batch: &SimulationDmoBankModelRegistrationBatch,
    ) -> SimulationDmoBankModelDeregisterController {
        SimulationDmoBankModelDeregisterController {
            domain: Arc::clone(&self.domain),
            source: batch.begin_pass(),
            faulted: false,
        }
    }

    /// Applies one exact registration callback through a matching controller.
    pub(crate) fn try_model_register_next(
        &mut self,
        controller: &mut SimulationDmoBankModelRegisterController,
        actual_source: &SimulationRecordHandle,
        actual_primary_asset_id: u64,
    ) -> Result<(), SimulationDmoBankModelRegisterPassError> {
        if !Arc::ptr_eq(&self.domain, &controller.domain) {
            return Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch);
        }
        if controller.faulted {
            return Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted);
        }
        let source = match controller
            .source
            .try_issue(actual_source, actual_primary_asset_id)
        {
            Ok(source) => source,
            Err(source) => {
                controller.faulted = true;
                return Err(SimulationDmoBankModelRegisterPassError::Source(source));
            }
        };
        if let Err(source) = self.manager.register_model(source) {
            controller.faulted = true;
            return Err(SimulationDmoBankModelRegisterPassError::Manager(source));
        }
        Ok(())
    }

    /// Applies one exact deregistration callback through a matching
    /// controller.
    pub(crate) fn try_model_deregister_next(
        &mut self,
        controller: &mut SimulationDmoBankModelDeregisterController,
        actual_source: &SimulationRecordHandle,
        actual_primary_asset_id: u64,
    ) -> Result<(), SimulationDmoBankModelDeregisterPassError> {
        if !Arc::ptr_eq(&self.domain, &controller.domain) {
            return Err(SimulationDmoBankModelDeregisterPassError::RuntimeMismatch);
        }
        if controller.faulted {
            return Err(SimulationDmoBankModelDeregisterPassError::AlreadyFaulted);
        }
        let source = match controller
            .source
            .try_issue(actual_source, actual_primary_asset_id)
        {
            Ok(source) => source,
            Err(source) => {
                controller.faulted = true;
                return Err(SimulationDmoBankModelDeregisterPassError::Source(source));
            }
        };
        self.manager.deregister_model(source);
        Ok(())
    }

    /// Begins a synchronous forward model-registration pass.
    pub(crate) fn begin_model_register_pass<'runtime>(
        &'runtime mut self,
        batch: &SimulationDmoBankModelRegistrationBatch,
    ) -> SimulationDmoBankModelRegisterPass<'runtime> {
        let controller = self.begin_model_register_controller(batch);
        SimulationDmoBankModelRegisterPass {
            runtime: self,
            controller,
        }
    }

    /// Begins a synchronous forward model-deregistration pass.
    pub(crate) fn begin_model_deregister_pass<'runtime>(
        &'runtime mut self,
        batch: &SimulationDmoBankModelRegistrationBatch,
    ) -> SimulationDmoBankModelDeregisterPass<'runtime> {
        let controller = self.begin_model_deregister_controller(batch);
        SimulationDmoBankModelDeregisterPass {
            runtime: self,
            controller,
        }
    }

    #[cfg(test)]
    pub(crate) fn has_model_for_test(&self, model_id: u64) -> bool {
        self.manager.has_model_for_test(model_id)
    }
}

/// Detached, runtime-domain-bound registration state for one exact source
/// cursor. It carries no manager authority and is deliberately not cloneable,
/// copyable, or debuggable.
#[must_use = "a detached DMO model registration controller must be finished"]
pub(crate) struct SimulationDmoBankModelRegisterController {
    domain: Arc<()>,
    source: SimulationDmoBankModelRegistrationPass,
    faulted: bool,
}

impl SimulationDmoBankModelRegisterController {
    /// Checks only this controller's fault and callback completeness state.
    /// No runtime or resident-liveness attestation occurs here.
    pub(crate) fn finish(self) -> Result<(), SimulationDmoBankModelRegisterPassError> {
        if self.faulted {
            return Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted);
        }
        self.source
            .finish()
            .map_err(SimulationDmoBankModelRegisterPassError::Source)
    }
}

/// Detached, runtime-domain-bound deregistration state for one exact source
/// cursor. It carries no manager authority and is deliberately not cloneable,
/// copyable, or debuggable.
#[must_use = "a detached DMO model deregistration controller must be finished"]
pub(crate) struct SimulationDmoBankModelDeregisterController {
    domain: Arc<()>,
    source: SimulationDmoBankModelRegistrationPass,
    faulted: bool,
}

impl SimulationDmoBankModelDeregisterController {
    /// Checks only this controller's fault and callback completeness state.
    /// No runtime or resident-liveness attestation occurs here.
    pub(crate) fn finish(self) -> Result<(), SimulationDmoBankModelDeregisterPassError> {
        if self.faulted {
            return Err(SimulationDmoBankModelDeregisterPassError::AlreadyFaulted);
        }
        self.source
            .finish()
            .map_err(SimulationDmoBankModelDeregisterPassError::Source)
    }
}

/// One synchronous registration phase over an exact prepared source pass.
#[must_use = "a DMO model registration pass must be finished"]
pub(crate) struct SimulationDmoBankModelRegisterPass<'runtime> {
    runtime: &'runtime mut SimulationDmoRegistrationRuntime,
    controller: SimulationDmoBankModelRegisterController,
}

impl SimulationDmoBankModelRegisterPass<'_> {
    /// Issues and immediately registers one exact callback source.
    ///
    /// Source refusal occurs before manager effects. Manager-capacity refusal
    /// occurs after the issuer cursor advances but before that row is inserted.
    /// Either refusal latches the pass; its successful prefix remains applied.
    pub(crate) fn try_next(
        &mut self,
        actual_source: &SimulationRecordHandle,
        actual_primary_asset_id: u64,
    ) -> Result<(), SimulationDmoBankModelRegisterPassError> {
        self.runtime.try_model_register_next(
            &mut self.controller,
            actual_source,
            actual_primary_asset_id,
        )
    }

    /// Finishes only a nonfaulted, complete callback pass.
    ///
    /// This is a completeness check, not resident-liveness attestation. Drop
    /// or incomplete/faulted finish never rolls back the applied prefix.
    pub(crate) fn finish(self) -> Result<(), SimulationDmoBankModelRegisterPassError> {
        self.controller.finish()
    }
}

/// One synchronous deregistration phase over an exact prepared source pass.
#[must_use = "a DMO model deregistration pass must be finished"]
pub(crate) struct SimulationDmoBankModelDeregisterPass<'runtime> {
    runtime: &'runtime mut SimulationDmoRegistrationRuntime,
    controller: SimulationDmoBankModelDeregisterController,
}

impl SimulationDmoBankModelDeregisterPass<'_> {
    /// Issues and immediately deregisters one exact callback source.
    ///
    /// Source refusal latches the pass. Successful prior removals remain
    /// applied, and manager pointer/key misses remain the native-shaped no-op.
    pub(crate) fn try_next(
        &mut self,
        actual_source: &SimulationRecordHandle,
        actual_primary_asset_id: u64,
    ) -> Result<(), SimulationDmoBankModelDeregisterPassError> {
        self.runtime.try_model_deregister_next(
            &mut self.controller,
            actual_source,
            actual_primary_asset_id,
        )
    }

    /// Finishes only a nonfaulted, complete callback pass.
    ///
    /// This is a completeness check, not resident-liveness attestation. Drop
    /// or incomplete/faulted finish never rolls back the applied prefix.
    pub(crate) fn finish(self) -> Result<(), SimulationDmoBankModelDeregisterPassError> {
        self.controller.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collection_load_pump::LoadedAssetCollection,
        collection_resident_streamer::ResidentCollectionStorage,
        decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
        dmo_bank_processed_asset::DmoBankProcessedArenaAsset,
        owned_resident_collection::{
            OwnedResidentCollection, OwnedResidentCollectionError, OwnedResidentCollectionStorage,
        },
        simulation_collision_asset::SimulationCollisionAsset,
        simulation_dmo_assembly_part_session::SimulationDmoModelRegistrationSource,
        simulation_normal_arena_plan::{
            SimulationNormalArenaPlan, SimulationNormalOperation, SimulationRecordSourceKind,
            SimulationRegistryAssetKind,
        },
        simulation_static_arena_plan::SimulationStaticArenaPlan,
        world::CollisionArena,
    };
    use skate2_formats::{
        rw4::{Arena, FileHeader, ResourceDescriptor},
        stream::{ArenaResources, BaseResourceDescriptor},
    };
    use std::sync::Arc;

    struct OperationSpec {
        operation: SimulationNormalOperation,
        record: Option<Vec<u8>>,
    }

    type TestStorage = OwnedResidentCollectionStorage<
        DmoBankProcessedArenaAsset,
        fn(OwnedResidentCollectionError) -> OwnedResidentCollectionError,
        OwnedResidentCollectionError,
    >;

    fn identity_error(error: OwnedResidentCollectionError) -> OwnedResidentCollectionError {
        error
    }

    fn dmo(primary: u64, local: u64) -> OperationSpec {
        let mut record = vec![0; 0xa0];
        record[0x60..0x68].copy_from_slice(&primary.to_be_bytes());
        record[0x68..0x70].copy_from_slice(&local.to_be_bytes());
        OperationSpec {
            operation: SimulationNormalOperation::DmoInstance {
                asset_id: primary,
                local_guid: local,
                source_toc_index: 0,
                source_dictionary_index: 0,
                source_inline_index: 0,
                source_record_offset: 0,
                raw_object_id: 0,
            },
            record: Some(record),
        }
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
            record: None,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one coherent fixture preserves every exact record and owner allocation"
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
            sources.push(spec.record.map(|record| {
                let start = resource_zero.len();
                let len = record.len();
                resource_zero.extend_from_slice(&record);
                (SimulationRecordSourceKind::DmoInstance, 0, start, len)
            }));
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

    fn issue_register(
        pass: &mut SimulationDmoBankModelRegisterPass<'_>,
        plan: &SimulationNormalArenaPlan,
        operation_index: usize,
        primary: u64,
    ) {
        pass.try_next(&plan.record_handle(operation_index).unwrap(), primary)
            .unwrap();
    }

    fn issue_deregister(
        pass: &mut SimulationDmoBankModelDeregisterPass<'_>,
        plan: &SimulationNormalArenaPlan,
        operation_index: usize,
        primary: u64,
    ) {
        pass.try_next(&plan.record_handle(operation_index).unwrap(), primary)
            .unwrap();
    }

    #[test]
    fn mixed_two_asset_forward_register_deregister_and_register_again() {
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
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();

        let mut register = runtime.begin_model_register_pass(&batch);
        issue_register(&mut register, &plan_a, 1, A0);
        issue_register(&mut register, &plan_a, 3, A1);
        issue_register(&mut register, &plan_b, 0, B0);
        register.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(0xa001));
        assert!(runtime.manager.has_model_for_test(0xa002));
        assert!(runtime.manager.has_model_for_test(0xb001));

        let mut deregister = runtime.begin_model_deregister_pass(&batch);
        issue_deregister(&mut deregister, &plan_a, 1, A0);
        issue_deregister(&mut deregister, &plan_a, 3, A1);
        issue_deregister(&mut deregister, &plan_b, 0, B0);
        deregister.finish().unwrap();
        assert!(!runtime.manager.has_model_for_test(0xa001));
        assert!(!runtime.manager.has_model_for_test(0xa002));
        assert!(!runtime.manager.has_model_for_test(0xb001));

        let mut again = runtime.begin_model_register_pass(&batch);
        issue_register(&mut again, &plan_a, 1, A0);
        issue_register(&mut again, &plan_a, 3, A1);
        issue_register(&mut again, &plan_b, 0, B0);
        again.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(0xa001));
        assert!(runtime.manager.has_model_for_test(0xa002));
        assert!(runtime.manager.has_model_for_test(0xb001));
    }

    #[test]
    fn duplicate_keys_keep_the_first_exact_winner() {
        const KEY: u64 = 0x5555_6666_7777_8888;
        let (asset_a, plan_a) = asset(0xa0, vec![dmo(1, KEY)]);
        let (asset_b, plan_b) = asset(0xb0, vec![dmo(2, KEY)]);
        let (_storage, collection) = collection(vec![asset_a, asset_b]);
        let retained = collection.weak_assets_in_order().unwrap();
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut pass = runtime.begin_model_register_pass(&batch);
        issue_register(&mut pass, &plan_a, 0, 1);
        issue_register(&mut pass, &plan_b, 0, 2);
        pass.finish().unwrap();

        assert!(
            runtime
                .manager
                .model_matches_asset_operation_for_test(KEY, retained[0].weak(), 0)
        );
        assert!(!runtime.manager.model_matches_asset_operation_for_test(
            KEY,
            retained[1].weak(),
            0
        ));
    }

    #[test]
    fn manager_deregister_missing_and_pointer_mismatch_remain_noops() {
        const KEY: u64 = 0x5555_6666_7777_8888;
        let (winner, winner_plan) = asset(0xa0, vec![dmo(1, KEY)]);
        let (_storage, resident_collection) = collection(vec![winner]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&resident_collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut register = runtime.begin_model_register_pass(&batch);
        issue_register(&mut register, &winner_plan, 0, 1);
        register.finish().unwrap();

        let (foreign, _foreign_plan) = asset(0xb0, vec![dmo(1, KEY)]);
        runtime.manager.deregister_model(
            SimulationDmoModelRegistrationSource::try_prepare(Arc::new(foreign), 0).unwrap(),
        );
        assert!(runtime.manager.has_model_for_test(KEY));

        let (missing, _missing_plan) = asset(0xc0, vec![dmo(2, KEY + 1)]);
        runtime.manager.deregister_model(
            SimulationDmoModelRegistrationSource::try_prepare(Arc::new(missing), 0).unwrap(),
        );
        assert!(runtime.manager.has_model_for_test(KEY));
    }

    #[test]
    fn registration_source_and_full_id_faults_latch_with_prefix() {
        const P0: u64 = 0x1111_0000_dead_beef;
        const P1: u64 = 0x2222_0000_dead_beef;
        const WRONG_HIGH: u64 = 0x3333_0000_dead_beef;
        let (asset, plan) = asset(0xa0, vec![dmo(P0, 10), dmo(P1, 11)]);
        let (_storage, resident_collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&resident_collection).unwrap();
        let first = plan.record_handle(0).unwrap();
        let second = plan.record_handle(1).unwrap();

        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut pass = runtime.begin_model_register_pass(&batch);
        pass.try_next(&first, P0).unwrap();
        assert!(matches!(
            pass.try_next(&first, WRONG_HIGH),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 1 }
            ))
        ));
        assert!(matches!(
            pass.try_next(&second, P1),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(matches!(
            pass.finish(),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(runtime.manager.has_model_for_test(10));
        assert!(!runtime.manager.has_model_for_test(11));

        let mut fresh = runtime.begin_model_register_pass(&batch);
        issue_register(&mut fresh, &plan, 0, P0);
        issue_register(&mut fresh, &plan, 1, P1);
        fresh.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(11));

        let mut other = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut full_id = other.begin_model_register_pass(&batch);
        assert!(matches!(
            full_id.try_next(&first, WRONG_HIGH),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::PrimaryAssetIdMismatch { ordinal: 0 }
            ))
        ));
        assert!(!other.manager.has_model_for_test(10));
    }

    #[test]
    fn deregistration_fault_latches_after_preserving_removed_prefix() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, resident_collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&resident_collection).unwrap();
        let first = plan.record_handle(0).unwrap();
        let second = plan.record_handle(1).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut register = runtime.begin_model_register_pass(&batch);
        issue_register(&mut register, &plan, 0, 1);
        issue_register(&mut register, &plan, 1, 2);
        register.finish().unwrap();

        let mut deregister = runtime.begin_model_deregister_pass(&batch);
        deregister.try_next(&first, 1).unwrap();
        assert!(matches!(
            deregister.try_next(&first, 2),
            Err(SimulationDmoBankModelDeregisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::SourceMismatch { ordinal: 1 }
            ))
        ));
        assert!(matches!(
            deregister.try_next(&second, 2),
            Err(SimulationDmoBankModelDeregisterPassError::AlreadyFaulted)
        ));
        assert!(matches!(
            deregister.finish(),
            Err(SimulationDmoBankModelDeregisterPassError::AlreadyFaulted)
        ));
        assert!(!runtime.manager.has_model_for_test(10));
        assert!(runtime.manager.has_model_for_test(11));
    }

    #[test]
    fn second_row_resident_expiry_fault_keeps_first_registration() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut pass = runtime.begin_model_register_pass(&batch);
        issue_register(&mut pass, &plan, 0, 1);
        collection.tombstone_without_releasing_assets_for_test();
        assert!(matches!(
            pass.try_next(&plan.record_handle(1).unwrap(), 2),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::ResidentExpired
            ))
        ));
        assert!(matches!(
            pass.finish(),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(runtime.manager.has_model_for_test(10));
        assert!(!runtime.manager.has_model_for_test(11));
    }

    #[test]
    fn capacity_allows_duplicate_then_faults_on_new_key_after_issue() {
        let mut specs = (0_u64..256)
            .map(|key| dmo(0x1000 + key, key))
            .collect::<Vec<_>>();
        specs.push(dmo(0x2000, 0));
        specs.push(dmo(0x2001, 256));
        let (asset, plan) = asset(0xa0, specs);
        let (_storage, collection) = collection(vec![asset]);
        let retained = collection.weak_assets_in_order().unwrap();
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut pass = runtime.begin_model_register_pass(&batch);
        for operation_index in 0..256 {
            issue_register(
                &mut pass,
                &plan,
                operation_index,
                0x1000 + u64::try_from(operation_index).unwrap(),
            );
        }
        issue_register(&mut pass, &plan, 256, 0x2000);
        assert!(matches!(
            pass.try_next(&plan.record_handle(257).unwrap(), 0x2001),
            Err(SimulationDmoBankModelRegisterPassError::Manager(
                SimulationDmoModelRegisterError::CapacityExceeded { capacity: 256 }
            ))
        ));
        assert!(matches!(
            pass.try_next(&plan.record_handle(257).unwrap(), 0x2001),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(matches!(
            pass.finish(),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(
            runtime
                .manager
                .model_matches_asset_operation_for_test(0, retained[0].weak(), 0)
        );
        assert!(!runtime.manager.model_matches_asset_operation_for_test(
            0,
            retained[0].weak(),
            256
        ));
        assert!(!runtime.manager.has_model_for_test(256));
    }

    #[test]
    fn incomplete_drop_and_empty_passes_never_roll_back() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, resident_collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&resident_collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut incomplete = runtime.begin_model_register_pass(&batch);
        issue_register(&mut incomplete, &plan, 0, 1);
        assert!(matches!(
            incomplete.finish(),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::Incomplete {
                    issued: 1,
                    expected: 2
                }
            ))
        ));
        assert!(runtime.manager.has_model_for_test(10));
        assert!(!runtime.manager.has_model_for_test(11));

        let mut dropped = runtime.begin_model_deregister_pass(&batch);
        issue_deregister(&mut dropped, &plan, 0, 1);
        drop(dropped);
        assert!(!runtime.manager.has_model_for_test(10));

        let (_empty_storage, empty_collection) = collection(Vec::new());
        let empty = SimulationDmoBankModelRegistrationBatch::try_new(&empty_collection).unwrap();
        runtime.begin_model_register_pass(&empty).finish().unwrap();
        runtime
            .begin_model_deregister_pass(&empty)
            .finish()
            .unwrap();
    }

    #[test]
    fn runtime_pass_owns_its_exact_cursor_after_batch_drop() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut pass = runtime.begin_model_register_pass(&batch);
        drop(batch);

        pass.try_next(&plan.record_handle(0).unwrap(), 1).unwrap();
        pass.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(10));
    }

    #[test]
    fn detached_controller_remains_exact_after_batch_drop() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut controller = runtime.begin_model_register_controller(&batch);
        drop(batch);

        runtime
            .try_model_register_next(&mut controller, &plan.record_handle(0).unwrap(), 1)
            .unwrap();
        controller.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(10));
    }

    #[test]
    fn runtime_mismatch_precedes_source_and_does_not_latch_or_apply() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut owner = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut foreign = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut controller = owner.begin_model_register_controller(&batch);
        let record = plan.record_handle(0).unwrap();

        assert!(matches!(
            foreign.try_model_register_next(&mut controller, &record, u64::MAX),
            Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch)
        ));
        assert!(!foreign.manager.has_model_for_test(10));
        owner
            .try_model_register_next(&mut controller, &record, 1)
            .unwrap();
        controller.finish().unwrap();
        assert!(owner.manager.has_model_for_test(10));
    }

    #[test]
    fn controller_domain_survives_creator_drop_without_aba() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let owner = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut controller = owner.begin_model_register_controller(&batch);
        drop(owner);
        let mut replacement = SimulationDmoRegistrationRuntime::try_new().unwrap();

        assert!(matches!(
            replacement.try_model_register_next(
                &mut controller,
                &plan.record_handle(0).unwrap(),
                1,
            ),
            Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch)
        ));
        assert!(!replacement.manager.has_model_for_test(10));
        assert!(matches!(
            controller.finish(),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::Incomplete {
                    issued: 0,
                    expected: 1
                }
            ))
        ));
    }

    #[test]
    fn correct_runtime_source_fault_latches_but_foreign_mismatch_stays_first() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut owner = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut foreign = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut controller = owner.begin_model_register_controller(&batch);
        let record = plan.record_handle(0).unwrap();

        assert!(matches!(
            owner.try_model_register_next(&mut controller, &record, 2),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::PrimaryAssetIdMismatch { ordinal: 0 }
            ))
        ));
        assert!(matches!(
            foreign.try_model_register_next(&mut controller, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch)
        ));
        assert!(matches!(
            owner.try_model_register_next(&mut controller, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(matches!(
            controller.finish(),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(!owner.manager.has_model_for_test(10));
        assert!(!foreign.manager.has_model_for_test(10));
    }

    #[test]
    fn dormant_controllers_are_independent_and_partial_drop_is_inert() {
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10), dmo(2, 11)]);
        let (_storage, collection) = collection(vec![asset]);
        let batch = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let mut runtime = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut partial = runtime.begin_model_register_controller(&batch);
        let mut complete = runtime.begin_model_register_controller(&batch);

        runtime
            .try_model_register_next(&mut partial, &plan.record_handle(0).unwrap(), 1)
            .unwrap();
        drop(partial);
        assert!(runtime.manager.has_model_for_test(10));
        runtime
            .try_model_register_next(&mut complete, &plan.record_handle(0).unwrap(), 1)
            .unwrap();
        runtime
            .try_model_register_next(&mut complete, &plan.record_handle(1).unwrap(), 2)
            .unwrap();
        complete.finish().unwrap();
        assert!(runtime.manager.has_model_for_test(10));
        assert!(runtime.manager.has_model_for_test(11));
    }

    #[test]
    fn empty_incomplete_and_fault_precedence_are_controller_local() {
        let (_empty_storage, empty_collection) = collection(Vec::new());
        let empty = SimulationDmoBankModelRegistrationBatch::try_new(&empty_collection).unwrap();
        let (asset, plan) = asset(0xa0, vec![dmo(1, 10)]);
        let (_storage, collection) = collection(vec![asset]);
        let nonempty = SimulationDmoBankModelRegistrationBatch::try_new(&collection).unwrap();
        let record = plan.record_handle(0).unwrap();
        let mut owner = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut foreign = SimulationDmoRegistrationRuntime::try_new().unwrap();
        let mut empty_register = owner.begin_model_register_controller(&empty);

        assert!(matches!(
            foreign.try_model_register_next(&mut empty_register, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch)
        ));
        assert!(matches!(
            owner.try_model_register_next(&mut empty_register, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::Extra {
                    issued: 0,
                    expected: 0
                }
            ))
        ));
        assert!(matches!(
            foreign.try_model_register_next(&mut empty_register, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::RuntimeMismatch)
        ));
        assert!(matches!(
            owner.try_model_register_next(&mut empty_register, &record, 1),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        assert!(matches!(
            empty_register.finish(),
            Err(SimulationDmoBankModelRegisterPassError::AlreadyFaulted)
        ));
        owner
            .begin_model_register_controller(&empty)
            .finish()
            .unwrap();
        assert!(matches!(
            owner.begin_model_register_controller(&nonempty).finish(),
            Err(SimulationDmoBankModelRegisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::Incomplete {
                    issued: 0,
                    expected: 1
                }
            ))
        ));

        let mut empty_deregister = owner.begin_model_deregister_controller(&empty);
        assert!(matches!(
            foreign.try_model_deregister_next(&mut empty_deregister, &record, 1),
            Err(SimulationDmoBankModelDeregisterPassError::RuntimeMismatch)
        ));
        assert!(matches!(
            owner.try_model_deregister_next(&mut empty_deregister, &record, 1),
            Err(SimulationDmoBankModelDeregisterPassError::Source(
                SimulationDmoBankModelRegistrationPassError::Extra {
                    issued: 0,
                    expected: 0
                }
            ))
        ));
        assert!(matches!(
            empty_deregister.finish(),
            Err(SimulationDmoBankModelDeregisterPassError::AlreadyFaulted)
        ));
    }
}
