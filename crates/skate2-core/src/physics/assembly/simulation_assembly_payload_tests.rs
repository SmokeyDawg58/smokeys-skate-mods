//! Synthetic mixed-storage tests of the existing backend and Assembly transaction.
use super::tests::{
    Backend, IDENTITY, Template, assert_detached, collision_template, dynamics, frames, template,
    transforms,
};
use super::*;
use crate::{
    dynamic_assembly_session::{
        DynamicAssemblyDriveDefinition, DynamicAssemblyJointDefinition,
        DynamicAssemblyRuntimeTemplate, DynamicAssemblySession,
    },
    simulation_joint_pool::SimulationMutableJointLimits,
    simulation_rigid_body_pool::SimulationMutableInertia,
};

fn mixed_template(
    inertia: &SimulationMutableInertia,
    limits: &SimulationMutableJointLimits<u32>,
    with_drive: bool,
) -> Arc<Template> {
    Arc::new(
        DynamicAssemblyRuntimeTemplate::try_new(
            collision_template(2),
            vec![
                DynamicAssemblyPartPayload {
                    inertia: Arc::new(SimulationInertia::native_default()).into(),
                    material: (),
                },
                DynamicAssemblyPartPayload {
                    inertia: SimulationAssemblyInertia::mutable(inertia.clone()),
                    material: (),
                },
            ],
            vec![DynamicAssemblyJointDefinition {
                parent_part: 0,
                child_part: 1,
                frames: Arc::new(11),
                limits: SimulationAssemblyJointLimits::mutable(limits.clone()),
            }],
            if with_drive {
                vec![DynamicAssemblyDriveDefinition {
                    parent_part: 0,
                    child_part: 1,
                    frames: frames(1.0),
                    dynamics: dynamics(1.0),
                }]
            } else {
                Vec::new()
            },
        )
        .unwrap(),
    )
}

#[test]
fn one_backend_retains_exact_immutable_and_live_mutable_payloads() {
    let mut backend = Backend::try_new(2, 2, 0).unwrap();
    let original = Arc::new(SimulationInertia::native_default());
    let immutable = SimulationAssemblyInertia::from(Arc::clone(&original));
    let owner = SimulationMutableInertia::new(*original);
    let mutable = SimulationAssemblyInertia::mutable(owner.clone());
    assert!(Arc::ptr_eq(immutable.as_immutable().unwrap(), &original));
    assert!(mutable.as_immutable().is_none());
    let parent = backend.add_rigid_body(&IDENTITY, &immutable, 4).unwrap();
    let child = backend.add_rigid_body(&IDENTITY, &mutable, 4).unwrap();
    assert!(Arc::ptr_eq(
        backend.rigid_bodies.try_inertia(parent).unwrap().unwrap(),
        &original
    ));

    let frames = Arc::new(13);
    let original_limits = Arc::new(17);
    let immutable_limits = SimulationAssemblyJointLimits::from(Arc::clone(&original_limits));
    let limits_owner = SimulationMutableJointLimits::new(19);
    let mutable_limits = SimulationAssemblyJointLimits::mutable(limits_owner.clone());
    let first = backend
        .add_joint(&parent, &child, &frames, &immutable_limits)
        .unwrap();
    let second = backend
        .add_joint(&parent, &child, &frames, &mutable_limits)
        .unwrap();
    let binding = backend.joints.try_binding(first).unwrap();
    assert!(Arc::ptr_eq(binding.frames, &frames));
    assert!(Arc::ptr_eq(binding.limits, &original_limits));
    assert!(mutable_limits.as_immutable().is_none());

    let changed = SimulationInertia {
        inverse_mass: 0.125,
        linear_drag: 0.5,
        ..*original
    };
    backend
        .rigid_bodies
        .try_write_mutable_inertia(child, &owner, changed)
        .unwrap();
    backend
        .joints
        .try_write_mutable_limits(second, &limits_owner, 23)
        .unwrap();
    assert_eq!(mutable.try_read().unwrap(), changed);
    assert_eq!(immutable.try_read().unwrap(), *original);
    assert_eq!(mutable_limits.try_read().unwrap(), 23);
    assert_eq!(backend.joints.try_limits(second).unwrap(), 23);
    assert_eq!(immutable_limits.try_read().unwrap(), 17);
    assert_eq!(
        backend.rigid_body_snapshot().slots[1].inertia,
        Some(changed)
    );
    backend.remove_rigid_body(&parent);
    backend.remove_rigid_body(&child);
    backend.remove_joint(&first);
    backend.remove_joint(&second);
    assert_detached(&backend);
}

#[test]
fn immutable_and_mixed_sessions_share_one_authority_and_teardown() {
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let limits = SimulationMutableJointLimits::new(29);
    let mut immutable =
        DynamicAssemblySession::<Backend>::try_new(template(2, &[(0, 1)], &[]), transforms(2))
            .unwrap();
    let mut mixed = DynamicAssemblySession::<Backend>::try_new(
        mixed_template(&owner, &limits, false),
        transforms(2),
    )
    .unwrap();
    let mut backend = Backend::try_new(4, 2, 0).unwrap();
    immutable.attach(&mut backend, 4).unwrap();
    mixed.attach(&mut backend, 4).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 4);
    assert_eq!(backend.joint_snapshot().counts.active, 2);
    mixed.detach(&mut backend).unwrap();
    assert_eq!(backend.rigid_body_snapshot().counts.active, 2);
    assert_eq!(backend.joint_snapshot().counts.active, 1);
    immutable.detach(&mut backend).unwrap();
    assert_detached(&backend);
}

#[test]
fn drive_rollback_keeps_mutable_owners_bound_to_original_backend() {
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let limits = SimulationMutableJointLimits::new(31);
    let mut failing = DynamicAssemblySession::<Backend>::try_new(
        mixed_template(&owner, &limits, true),
        transforms(2),
    )
    .unwrap();
    let mut backend = Backend::try_new(2, 1, 0).unwrap();
    assert!(failing.attach(&mut backend, 4).is_err());
    assert!(!failing.is_attached());
    assert_detached(&backend);
    assert!(backend.last_add_failure().is_none());

    // Reuse the exact owners after rollback in the same pools, without the
    // intentionally unavailable Drive. Host binding is not undone by removal.
    let mut retry = DynamicAssemblySession::<Backend>::try_new(
        mixed_template(&owner, &limits, false),
        transforms(2),
    )
    .unwrap();
    retry.attach(&mut backend, 4).unwrap();
    retry.detach(&mut backend).unwrap();
    assert_detached(&backend);
    let mut foreign = Backend::try_new(2, 1, 0).unwrap();
    assert!(retry.attach(&mut foreign, 4).is_err());
    assert_detached(&foreign);
    assert_eq!(
        foreign.last_add_failure(),
        Some(&SimulationAssemblyBackendAddFailure::RigidBody(
            SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch
        ))
    );
}

#[test]
fn body_capacity_and_static_precede_foreign_mutable_owner_inspection() {
    let owner = SimulationMutableInertia::new(SimulationInertia::native_default());
    let payload = SimulationAssemblyInertia::mutable(owner);
    let mut first = Backend::try_new(1, 0, 0).unwrap();
    let body = first.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    first.remove_rigid_body(&body);
    let mut empty = Backend::try_new(0, 0, 0).unwrap();
    assert!(empty.add_rigid_body(&IDENTITY, &payload, 4).is_none());
    assert!(empty.last_add_failure().is_none());
    let mut other = Backend::try_new(1, 0, 0).unwrap();
    let static_body = other.add_rigid_body(&IDENTITY, &payload, 1).unwrap();
    assert!(
        other
            .rigid_bodies
            .try_inertia(static_body)
            .unwrap()
            .is_none()
    );
    other.remove_rigid_body(&static_body);
    assert!(other.add_rigid_body(&IDENTITY, &payload, 4).is_none());
    assert_eq!(
        other.last_add_failure(),
        Some(&SimulationAssemblyBackendAddFailure::RigidBody(
            SimulationRigidBodyPoolError::InertiaOwnerPoolMismatch
        ))
    );
    assert_detached(&other);
}

#[test]
fn joint_capacity_and_endpoint_checks_precede_foreign_limits_owner() {
    let payload = SimulationAssemblyInertia::from(Arc::new(SimulationInertia::native_default()));
    let frames = Arc::new(1);
    let limits = SimulationAssemblyJointLimits::mutable(SimulationMutableJointLimits::new(2));
    let mut first = Backend::try_new(2, 1, 0).unwrap();
    let a = first.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    let b = first.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    let joint = first.add_joint(&a, &b, &frames, &limits).unwrap();
    first.remove_rigid_body(&a);
    first.remove_rigid_body(&b);
    first.remove_joint(&joint);
    let mut empty = Backend::try_new(0, 0, 0).unwrap();
    assert!(empty.add_joint(&a, &b, &frames, &limits).is_none());
    assert!(empty.last_add_failure().is_none());
    let mut other = Backend::try_new(2, 1, 0).unwrap();
    assert!(other.add_joint(&a, &b, &frames, &limits).is_none());
    assert!(matches!(
        other.last_add_failure(),
        Some(SimulationAssemblyBackendAddFailure::JointBody {
            endpoint: SimulationAssemblyJointEndpoint::Parent,
            ..
        })
    ));
    let c = other.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    let d = other.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    assert!(other.add_joint(&c, &d, &frames, &limits).is_none());
    assert_eq!(
        other.last_add_failure(),
        Some(&SimulationAssemblyBackendAddFailure::Joint(
            SimulationJointPoolError::LimitsOwnerPoolMismatch
        ))
    );
    other.remove_rigid_body(&c);
    other.remove_rigid_body(&d);
    assert_detached(&other);
}
