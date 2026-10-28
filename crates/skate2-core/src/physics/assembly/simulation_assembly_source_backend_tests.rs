use super::super::tests::{serialised_arena_owner, serialised_binding, serialised_runtime_inertia};
use super::*;
use crate::dynamic_assembly_session::DynamicAssemblySimulationBackend;
use crate::simulation_rigid_body_pool::SimulationInertia;

pub(crate) type Backend = SimulationAssemblyBackend<AssemblyPartMaterial, u32, u32>;
pub(crate) const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

pub(crate) fn configured(capacity: usize, owner: &Arc<SimulationNormalArenaPlan>) -> Backend {
    let mut backend = Backend::try_new(capacity, 0, 0).unwrap();
    backend
        .try_bind_source_arenas(
            image(capacity),
            vec![(
                Arc::clone(owner),
                serialised_binding(Arc::clone(owner), 0x2000),
            )],
        )
        .unwrap();
    backend
}

pub(crate) fn source_template(
    offsets: &[u32],
    inverse: Option<[f32; 16]>,
) -> SerialisedAssemblyTemplate {
    let owner = serialised_arena_owner();
    let parts = offsets
        .iter()
        .enumerate()
        .map(|(index, &offset)| {
            let mut part = super::super::tests::serialised_template_part(
                Arc::clone(&owner),
                index,
                offset,
                1.0,
                u32::from(inverse.is_some()),
                0,
            );
            part.inverse_body_transform = inverse.unwrap_or(IDENTITY);
            part
        })
        .collect();
    super::super::tests::serialised_template_for_binding(owner, [0, 0, 0, 0], parts)
}

pub(crate) fn move_body(backend: &mut Backend, body: SimulationRigidBodyHandle, pose: [f32; 16]) {
    backend
        .rigid_bodies
        .try_set_part_body_transform_host(body, pose)
        .unwrap();
}

fn image(capacity: usize) -> SimulationXenonAddressImage {
    SimulationXenonAddressImage::try_new(0x1000_0080, u32::try_from(capacity).unwrap(), 0, 0)
        .unwrap()
}

fn inertia(
    backend: &Backend,
    owner: &Arc<SimulationNormalArenaPlan>,
    offset: u32,
) -> SimulationAssemblyInertia {
    SimulationAssemblyInertia::source_routed(
        backend
            .source_routes
            .as_ref()
            .unwrap()
            .bind(&serialised_runtime_inertia(Arc::clone(owner), offset))
            .unwrap(),
    )
}

#[test]
fn ordinary_and_two_source_arenas_share_the_existing_pool_and_free_list() {
    let a = serialised_arena_owner();
    let b = serialised_arena_owner();
    let mut backend = Backend::try_new(3, 0, 0).unwrap();
    let authority = backend.authority();
    let ordinary = backend
        .add_rigid_body(
            &IDENTITY,
            &Arc::new(SimulationInertia::native_default()).into(),
            4,
        )
        .unwrap();
    backend
        .try_bind_source_arenas(
            image(3),
            vec![
                (Arc::clone(&a), serialised_binding(Arc::clone(&a), 0x2000)),
                (Arc::clone(&b), serialised_binding(Arc::clone(&b), 0x3000)),
            ],
        )
        .unwrap();
    let ia = inertia(&backend, &a, 0x10);
    let ib = inertia(&backend, &b, 0x10);
    let ba = backend.add_rigid_body(&IDENTITY, &ia, 4).unwrap();
    let bb = backend.add_rigid_body(&IDENTITY, &ib, 4).unwrap();
    assert_eq!(backend.authority(), authority);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 0);
    assert_eq!(
        backend
            .try_source_context(&a)
            .unwrap()
            .body_pool
            .authority(),
        authority
    );
    assert_eq!(
        backend.try_source_body_pose(ba, &a).unwrap()[7].to_bits(),
        0x2010
    );
    assert_eq!(
        backend.try_source_body_pose(bb, &b).unwrap()[7].to_bits(),
        0x3010
    );
    assert_eq!(
        backend.try_source_body_pose(ordinary, &a),
        Err(SimulationAssemblySourceError::UnroutedBody)
    );
    assert_eq!(
        backend.rigid_body_pose(&ordinary).map(f32::to_bits),
        IDENTITY.map(f32::to_bits)
    );
    for body in [ba, ordinary, bb] {
        backend.remove_rigid_body(&body);
    }
    assert_eq!(backend.rigid_body_snapshot().counts.free, 3);
}

#[test]
fn configuration_is_atomic_and_never_replaces_a_live_image() {
    let owner = serialised_arena_owner();
    let foreign = serialised_arena_owner();
    let mut backend = Backend::try_new(1, 0, 0).unwrap();
    let before = backend.rigid_body_snapshot();
    assert!(
        backend
            .try_bind_source_arenas(
                image(1),
                vec![(Arc::clone(&owner), serialised_binding(foreign, 0x2000))]
            )
            .is_err()
    );
    assert!(backend.source_routes.is_none());
    assert_eq!(backend.rigid_body_snapshot(), before);
    backend
        .try_bind_source_arenas(
            image(1),
            vec![(
                Arc::clone(&owner),
                serialised_binding(Arc::clone(&owner), 0x2000),
            )],
        )
        .unwrap();
    let epoch = backend
        .try_source_context(&owner)
        .unwrap()
        .image_domain
        .epoch();
    assert_eq!(
        backend.try_bind_source_arenas(image(1), Vec::new()),
        Err(SimulationAssemblySourceInitError::AlreadyConfigured)
    );
    assert_eq!(
        backend
            .try_source_context(&owner)
            .unwrap()
            .image_domain
            .epoch(),
        epoch
    );
}

#[test]
fn duplicate_owner_refuses_before_publication() {
    let owner = serialised_arena_owner();
    let mut backend = Backend::try_new(1, 0, 0).unwrap();
    assert!(matches!(
        backend.try_bind_source_arenas(
            image(1),
            vec![
                (
                    Arc::clone(&owner),
                    serialised_binding(Arc::clone(&owner), 0x2000)
                ),
                (Arc::clone(&owner), serialised_binding(owner, 0x3000)),
            ]
        ),
        Err(SimulationAssemblySourceInitError::Source(
            SerialisedAssemblySimulationBackendInitError::DuplicateArenaOwner {
                first: 0,
                second: 1
            }
        ))
    ));
    assert!(backend.source_routes.is_none());
}

#[test]
fn foreign_route_refuses_without_touching_either_pool() {
    let owner = serialised_arena_owner();
    let a = configured(1, &owner);
    let mut b = configured(1, &owner);
    let routed = inertia(&a, &owner, 0x10);
    let before = b.rigid_body_snapshot();
    assert!(b.add_rigid_body(&IDENTITY, &routed, 4).is_none());
    assert_eq!(b.rigid_body_snapshot(), before);
    let Some(SimulationAssemblyBackendAddFailure::SourceRigidBody(error)) = b.last_add_failure()
    else {
        panic!("source diagnostic");
    };
    assert_eq!(error.cause(), &SimulationAssemblySourceError::ForeignRoute);
    assert_eq!(a.rigid_body_snapshot().counts.free, 1);
}

#[test]
fn static_add_retains_route_but_skips_invalid_definition_address() {
    let owner = serialised_arena_owner();
    let mut backend = configured(1, &owner);
    let invalid = inertia(&backend, &owner, 0x11);
    let before = backend.rigid_body_snapshot();
    assert!(backend.add_rigid_body(&IDENTITY, &invalid, 4).is_none());
    assert!(backend.last_add_failure().is_some());
    assert_eq!(backend.rigid_body_snapshot(), before);
    let body = backend.add_rigid_body(&IDENTITY, &invalid, 1).unwrap();
    assert_eq!(
        backend.try_source_body_pose(body, &owner).unwrap()[7].to_bits(),
        0
    );
    assert!(backend.try_source_body_observation(body, &owner).is_ok());
    assert_eq!(backend.last_add_failure(), None);
    backend.remove_rigid_body(&body);
    assert_eq!(backend.rigid_body_snapshot().counts.free, 1);
}

#[test]
fn exhaustion_precedes_definition_address_and_clears_prior_add_error() {
    let owner = serialised_arena_owner();
    let mut backend = configured(1, &owner);
    let invalid = inertia(&backend, &owner, 0x11);
    let good = inertia(&backend, &owner, 0x10);
    assert!(backend.add_rigid_body(&IDENTITY, &invalid, 4).is_none());
    let body = backend.add_rigid_body(&IDENTITY, &good, 4).unwrap();
    let before = backend.rigid_body_snapshot();
    assert!(backend.add_rigid_body(&IDENTITY, &invalid, 4).is_none());
    assert_eq!(backend.last_add_failure(), None);
    assert_eq!(backend.rigid_body_snapshot(), before);
    backend.remove_rigid_body(&body);
}

#[test]
fn retired_occurrence_cannot_read_or_remove_reused_source_slot() {
    let owner = serialised_arena_owner();
    let next_owner = serialised_arena_owner();
    let mut backend = Backend::try_new(1, 0, 0).unwrap();
    backend
        .try_bind_source_arenas(
            image(1),
            vec![
                (
                    Arc::clone(&owner),
                    serialised_binding(Arc::clone(&owner), 0x2000),
                ),
                (
                    Arc::clone(&next_owner),
                    serialised_binding(Arc::clone(&next_owner), 0x3000),
                ),
            ],
        )
        .unwrap();
    let first = inertia(&backend, &owner, 0x10);
    let second = inertia(&backend, &next_owner, 0x40);
    let old = backend.add_rigid_body(&IDENTITY, &first, 4).unwrap();
    backend.remove_rigid_body(&old);
    let new = backend.add_rigid_body(&IDENTITY, &second, 4).unwrap();
    assert_eq!(old.id(), new.id());
    assert_ne!(old.observation(), new.observation());
    assert!(backend.try_source_body_pose(old, &owner).is_err());
    assert!(backend.remove_source_body(old).is_err());
    assert_eq!(
        backend.try_source_body_pose(new, &next_owner).unwrap()[7].to_bits(),
        0x3040
    );
    assert_eq!(
        backend.try_source_body_pose(new, &owner),
        Err(SimulationAssemblySourceError::ForeignArena)
    );
    backend.remove_rigid_body(&new);
}

#[test]
fn source_owner_identity_is_not_byte_equality() {
    let owner = serialised_arena_owner();
    let foreign = serialised_arena_owner();
    let mut backend = configured(1, &owner);
    let payload = inertia(&backend, &owner, 0x10);
    let body = backend.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    assert_eq!(
        backend.try_source_body_pose(body, &foreign),
        Err(SimulationAssemblySourceError::ForeignArena)
    );
    assert!(matches!(
        backend.try_source_context(&foreign),
        Err(SimulationAssemblySourceError::ArenaNotRegistered)
    ));
    backend.remove_rigid_body(&body);
}

#[test]
fn direct_pool_bypass_cannot_overwrite_an_existing_source_directory_record() {
    let owner = serialised_arena_owner();
    let mut backend = configured(1, &owner);
    let payload = inertia(&backend, &owner, 0x10);
    let body = backend.add_rigid_body(&IDENTITY, &payload, 4).unwrap();
    // Deliberately violate private backend invariants to exercise preflight.
    backend.rigid_bodies.try_remove_rigid_body(body).unwrap();
    let before = backend.rigid_body_snapshot();
    assert!(backend.add_rigid_body(&IDENTITY, &payload, 4).is_none());
    assert_eq!(backend.rigid_body_snapshot(), before);
    let Some(SimulationAssemblyBackendAddFailure::SourceRigidBody(error)) =
        backend.last_add_failure()
    else {
        panic!("directory diagnostic");
    };
    assert_eq!(
        error.cause(),
        &SimulationAssemblySourceError::OccupiedDirectory {
            slot: body.id().index()
        }
    );
}

#[test]
fn source_semantic_payload_retains_pair_without_claiming_unknown_tensor_w() {
    let owner = serialised_arena_owner();
    let backend = configured(1, &owner);
    let payload = inertia(&backend, &owner, 0x10);
    assert_eq!(
        payload.try_read().unwrap().inverse_tensor.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0].map(f32::to_bits)
    );
    assert!(payload.as_immutable().is_none());
    assert_eq!(payload.try_read_packed().unwrap(), None);
    assert!(Arc::ptr_eq(
        payload.as_source_routed().unwrap().source_owner(),
        &owner
    ));
}
