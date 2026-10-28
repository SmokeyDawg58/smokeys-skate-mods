use super::*;
use crate::{
    attribute_runtime::tests::{collection, graph, set_dependency_bytes},
    reckoning::ReckoningDebug,
    reckoning_body_spin::{BodySpinCvar, BodySpinCvars},
    reckoning_numeric::HostReckoningNumeric,
};
struct Cvars;
impl BodySpinCvars for Cvars {
    type Pointer = BodySpinCvar;
    type Error = ();
    fn resolve(&mut self, _: BodySpinCvar) -> Result<BodySpinCvar, ()> {
        panic!("source refusal precedes cvars")
    }
    fn float(&mut self, _: &BodySpinCvar) -> Result<f32, ()> {
        panic!("source refusal precedes values")
    }
}
impl ReckoningCvars for Cvars {
    fn debug_enabled(&mut self, _: ReckoningDebug) -> Result<bool, ()> {
        panic!("source refusal precedes debug")
    }
}
fn vault() -> Arc<AttributeRuntimeVault> {
    Arc::new(graph(crate::reckoning_attributes::RECKONING_CLASS, vec![]))
}
#[test]
fn storage_constructor_and_processed_occurrence_have_distinct_readiness() {
    let mut owner = PhysicalPlayerReckoningOwner::new();
    let processed = Arc::clone(owner.processed());
    assert!(owner.reckoning().is_none());
    owner.try_construct(&vault()).unwrap();
    assert!(Arc::ptr_eq(owner.processed(), &processed));
    assert!(owner.reckoning().is_some());
    assert!(owner.completed.is_none());
    assert!(matches!(
        owner.try_construct(&vault()),
        Err(PhysicalPlayerReckoningConstructError::AlreadyAttempted)
    ));
}
#[test]
fn failed_actual_layout_constructor_consumes_attempt_without_claiming_pid_prefix() {
    let class = crate::reckoning_attributes::RECKONING_CLASS;
    let mut data = graph(class, vec![]);
    collection(&mut data, class, 0xd7ed_bd36_2d7d_2152, 0, Some(64), vec![]);
    set_dependency_bytes(&mut data, 1, vec![0; 68]);
    let mut owner = PhysicalPlayerReckoningOwner::new();
    assert!(matches!(
        owner.try_construct(&Arc::new(data)),
        Err(PhysicalPlayerReckoningConstructError::Attribute(_))
    ));
    assert!(owner.reckoning().is_none());
    assert!(matches!(
        owner.try_construct(&vault()),
        Err(PhysicalPlayerReckoningConstructError::AlreadyAttempted)
    ));
}
#[test]
fn foreign_source_and_source_failure_invalidate_existing_phase_before_callbacks() {
    let mut owner = PhysicalPlayerReckoningOwner::new();
    owner.try_construct(&vault()).unwrap();
    let processed = Arc::clone(owner.processed());
    let foreign = Arc::new(Mutex::new(SkeletonProcessedContact::new()));
    // Seed only host completion bookkeeping to isolate invalidation. This is not
    // evidence of a successful numerical update; actual corpus covers that path.
    let old = PhysicalPlayerReckoningPhase(Arc::new(()));
    owner.completed = Some(old.clone());
    assert!(owner.try_completed(&old).is_ok());
    let result = owner.try_update_grounded_fixed_heading(
        &foreign,
        |_| -> Result<_, ()> { panic!("foreign source not read") },
        &mut Cvars,
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(PhysicalPlayerReckoningUpdateError::ForeignProcessed)
    ));
    assert!(owner.try_completed(&old).is_err());
    owner.completed = Some(old.clone());
    let result = owner.try_update_grounded_fixed_heading(
        &processed,
        |_| Err::<PhysicalPlayerReckoningGroundSample, _>("source"),
        &mut Cvars,
        &mut HostReckoningNumeric,
    );
    assert!(matches!(
        result,
        Err(PhysicalPlayerReckoningUpdateError::Source("source"))
    ));
    assert!(owner.try_completed(&old).is_err());
}
#[test]
fn foreign_phase_and_reset_cannot_grant_current_completion() {
    let mut owner = PhysicalPlayerReckoningOwner::new();
    owner.try_construct(&vault()).unwrap();
    let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
    owner.completed = Some(phase.clone());
    assert!(
        owner
            .try_completed(&PhysicalPlayerReckoningPhase(Arc::new(())))
            .is_err()
    );
    let before = *owner.reckoning().unwrap().transform();
    owner.try_reset().unwrap();
    assert!(owner.try_completed(&phase).is_err());
    assert_eq!(
        owner
            .reckoning()
            .unwrap()
            .transform()
            .known_lanes()
            .expect("constructor known lanes")
            .map(f32::to_bits),
        before
            .known_lanes()
            .expect("constructor known lanes")
            .map(f32::to_bits)
    );
}

#[test]
fn reset_body_spin_requires_construction_and_invalidates_existing_completion() {
    let mut owner = PhysicalPlayerReckoningOwner::new();
    assert!(owner.try_reset_body_spin().is_err());
    owner.try_construct(&vault()).unwrap();
    // A selected reset needs no preceding grounded update.
    owner.try_reset_body_spin().unwrap();
    let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
    owner.completed = Some(phase.clone());
    let before = *owner.reckoning().unwrap().transform();
    let processed = Arc::clone(owner.processed());
    owner.try_reset_body_spin().unwrap();
    assert!(owner.try_completed(&phase).is_err());
    assert!(Arc::ptr_eq(&processed, owner.processed()));
    assert_eq!(owner.reckoning().unwrap().body_spin(), (0.0, 0.0));
    assert_eq!(
        owner
            .reckoning()
            .unwrap()
            .transform()
            .known_lanes()
            .expect("constructor known lanes")
            .map(f32::to_bits),
        before
            .known_lanes()
            .expect("constructor known lanes")
            .map(f32::to_bits)
    );
}
