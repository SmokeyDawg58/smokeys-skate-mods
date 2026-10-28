//! Empty real manager/body domains; no fabricated board, geometry, or collision
//! publication. Mounted corpus checks own actual board admission coverage.
use super::*;

fn domain() -> (SkateboardSimulationBackend, SimulationManagerContactDomain) {
    // This one real body slot is consumed by the actual manager Static body.
    let mut backend = SkateboardSimulationBackend::try_new(1, 0, 0).unwrap();
    let manager = backend.try_manager_for_test(0).unwrap();
    (backend, manager)
}

#[test]
fn source_capacity_uses_only_island_flag_bit_zero_without_adding_members() {
    let (backend, mut manager) = domain();
    let before = backend.rigid_body_snapshot();
    for (flags, expected) in [
        (0, 2),
        (2, 2),
        (0x8000_0000, 2),
        (1, 16),
        (3, 16),
        (0x8000_0001, 16),
    ] {
        let pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, flags, false).unwrap();
        assert_eq!(pool.capacity(), expected);
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.handles().len(), 0);
        assert_eq!(pool.publications().len(), 0);
        assert!(pool.active_ordinals().is_none());
        assert!(pool.active_frame().is_none());
    }
    assert_eq!(backend.rigid_body_snapshot(), before);
    // Test-only observation of the real shared allocator: pool construction
    // alone must not burn any Assembly index. This is not runtime admission.
    assert_eq!(
        manager
            .try_allocate_assembly_world_index(backend.authority())
            .unwrap(),
        0
    );
}

#[test]
fn current_parallel_true_is_refused_without_body_effects_or_index_consumption() {
    let (backend, mut manager) = domain();
    let before = backend.rigid_body_snapshot();
    for flags in [0, 3] {
        assert!(matches!(
            SimulationSkaterAssemblyPool::try_new(&manager, &backend, flags, true),
            Err(SimulationSkaterPoolError::UnsupportedParallel)
        ));
    }
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(
        manager
            .try_allocate_assembly_world_index(backend.authority())
            .unwrap(),
        0
    );
}

#[test]
fn foreign_backend_construction_and_refresh_refuse_and_clear_active_freshness() {
    let (backend, mut manager) = domain();
    let (foreign_backend, _) = domain();
    assert!(matches!(
        SimulationSkaterAssemblyPool::try_new(&manager, &foreign_backend, 3, false),
        Err(SimulationSkaterPoolError::ForeignBackend)
    ));
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 3, false).unwrap();
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([].as_slice()));
    assert!(matches!(
        pool.try_refresh(&foreign_backend),
        Err(SimulationSkaterPoolError::ForeignBackend)
    ));
    assert!(pool.active_ordinals().is_none());
    assert!(pool.active_frame().is_none());
    // Both domains begin at the same numerical epoch; authority, not frame
    // number equality, rejects the foreign refresh.
    assert_eq!(
        backend.collision_body_pool().frame_identity(),
        foreign_backend.collision_body_pool().frame_identity()
    );
    pool.try_refresh(&backend).unwrap();
    assert_eq!(
        pool.active_frame(),
        backend.collision_body_pool().frame_identity()
    );
    assert_eq!(
        manager
            .try_allocate_assembly_world_index(backend.authority())
            .unwrap(),
        0
    );
}

#[test]
fn empty_end_update_publishes_empty_active_list_in_the_actual_current_frame() {
    let (backend, manager) = domain();
    let before = backend.rigid_body_snapshot();
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 3, false).unwrap();
    assert!(pool.active_frame().is_none());
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.active_ordinals(), Some([].as_slice()));
    assert_eq!(
        pool.active_frame(),
        backend.collision_body_pool().frame_identity()
    );
    assert_eq!(pool.publications().len(), 0);
    // Repeated empty refresh does not invent a member, body, or new epoch.
    pool.try_refresh(&backend).unwrap();
    assert_eq!(pool.len(), 0);
    assert_eq!(
        pool.active_frame(),
        backend.collision_body_pool().frame_identity()
    );
    assert_eq!(backend.rigid_body_snapshot(), before);
}

#[test]
fn unissued_host_capability_never_resolves_or_detaches_the_manager_static_body() {
    let (mut backend, mut manager) = domain();
    let before = backend.rigid_body_snapshot();
    let mut pool = SimulationSkaterAssemblyPool::try_new(&manager, &backend, 3, false).unwrap();
    // Deliberately unissued test capability, not a fake production member or
    // publication. The owner must reject it before any body-removal operation.
    let absent = SimulationSkaterAssemblyHandle(Arc::new(()));
    assert!(matches!(
        pool.session(&absent),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert!(matches!(
        pool.publication(&absent),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert!(matches!(
        pool.world_index(&absent),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert!(matches!(
        pool.try_remove(&mut backend, &absent),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert!(matches!(
        pool.try_remove_post_drives(&mut backend, &absent),
        Err(SimulationSkaterPoolError::MissingMember)
    ));
    assert_eq!(pool.len(), 0);
    assert_eq!(backend.rigid_body_snapshot(), before);
    assert_eq!(
        manager
            .try_allocate_assembly_world_index(backend.authority())
            .unwrap(),
        0
    );
}
