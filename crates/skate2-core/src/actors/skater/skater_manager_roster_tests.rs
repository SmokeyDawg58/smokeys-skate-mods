use super::*;
#[test]
fn constructor_null_ghost_and_empty_roster_do_not_manufacture_completed_observers() {
    let game=crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo::construct_network_field();
    let manager = SkaterManagerLocalIdentity::construct_local_fields(
        crate::skater_manager_local_identity::LocalBackendOwner::construct_static_fields(game),
    );
    assert!(manager.try_ghost_is_null().unwrap());
    assert_eq!(manager.try_count().unwrap(), 0);
    assert!(manager.try_actor(0).unwrap().is_none());
    assert!(matches!(
        manager.observer_evidence(),
        Err(super::super::observers::SkaterObserverError::PendingWorkspaceObservers)
    ));
}
