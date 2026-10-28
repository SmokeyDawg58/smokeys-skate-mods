use super::*;
use crate::simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo;
fn backend() -> Arc<LocalBackendOwner> {
    LocalBackendOwner::construct_static_fields(PhysicalGameInfo::construct_network_field())
}
#[test]
fn actual_default_profile_creates_same_row_peer_rank_and_counts() {
    let settings = PlayersBackendSettings::construct_fields();
    let profile = ProfileNameOwner::construct_name();
    assert_eq!(settings.try_player_type(0).unwrap(), 0);
    assert_eq!(settings.try_peer_id(0).unwrap(), -1);
    settings
        .try_create_offline_human(0, &profile, &backend())
        .unwrap();
    assert_eq!(settings.try_player_type(0).unwrap(), 1);
    assert_eq!(settings.try_peer_id(0).unwrap(), -1);
    let rows = settings.players.lock().unwrap();
    assert_eq!(rows[0].creation.rank, 1);
    assert_eq!(
        &rows[0].creation.short_name[..9],
        b"PLAYER 1\0".map(Some).as_slice()
    );
    assert_eq!(rows[0].creation.short_name, rows[0].creation.long_name);
    assert!(rows[0].creation.short_name[9..].iter().all(Option::is_none));
    drop(rows);
    let fields = settings.creation.lock().unwrap();
    assert_eq!((fields.total, fields.humans, fields.rank_map[0]), (1, 1, 0));
    drop(fields);
    assert!(matches!(
        settings.try_create_offline_human(0, &profile, &backend()),
        Err(PlayerCreateError::AlreadyPresent(0))
    ));
}
#[test]
fn later_name_failure_retains_source_type_peer_prefix_without_rank_count() {
    let settings = PlayersBackendSettings::construct_fields();
    let profile = ProfileNameOwner::construct_name();
    profile.name.lock().unwrap()[0] = None;
    assert!(matches!(
        settings.try_create_offline_human(0, &profile, &backend()),
        Err(PlayerCreateError::UnknownProfileName)
    ));
    assert_eq!(settings.try_player_type(0).unwrap(), 1);
    assert_eq!(settings.try_peer_id(0).unwrap(), -1);
    assert_eq!(settings.players.lock().unwrap()[0].creation.rank, 0);
    assert_eq!(settings.creation.lock().unwrap().total, 0);
}
#[test]
fn row_reset_preserves_name_tails_and_does_not_reset_global_counts() {
    let settings = PlayersBackendSettings::construct_fields();
    let profile = ProfileNameOwner::construct_name();
    settings
        .try_create_offline_human(0, &profile, &backend())
        .unwrap();
    settings.try_reset_player_settings(0).unwrap();
    let rows = settings.players.lock().unwrap();
    assert_eq!(rows[0].creation.short_name[0], Some(0));
    assert_eq!(rows[0].creation.short_name[1], Some(b'L'));
    assert_eq!(rows[0].creation.long_name[0], Some(b'P'));
    assert_eq!(rows[0].creation.rank, 0);
    drop(rows);
    assert_eq!(settings.creation.lock().unwrap().total, 1);
}
