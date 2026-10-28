use super::*;
#[test]
fn actual_registration_retains_queue_and_census_initial_flag_without_local_actor() {
    let game=crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field();
    let manager = SkaterManagerLocalIdentity::construct_local_fields(
        super::super::LocalBackendOwner::construct_static_fields(game),
    );
    assert!(matches!(
        manager.observer_snapshot(),
        Err(SkaterObserverError::PendingWorkspaceObservers)
    ));
    let queue = ChallengeEventQueue::try_construct().unwrap();
    let earlier = ChallengeEventBroadcaster::construct(Arc::clone(&queue));
    earlier.send(SkaterNotification::Created, 7).unwrap();
    manager
        .try_register_startup_observers(Arc::clone(&queue))
        .unwrap();
    let (flags, retained, camera_null) = manager.observer_evidence().unwrap();
    assert_eq!(flags, 0xfe);
    assert!(Arc::ptr_eq(&retained, &queue));
    assert_eq!(queue.snapshot().unwrap().2.len(), 1);
    assert!(camera_null);
    assert!(manager.try_local_actor().unwrap().is_none());
    assert!(matches!(
        manager.try_register_startup_observers(queue),
        Err(SkaterObserverError::AlreadyRegistered)
    ));
}
