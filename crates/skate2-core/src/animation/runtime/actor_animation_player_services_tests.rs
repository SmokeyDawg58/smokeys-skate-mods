use super::*;
use crate::{
    actor_face_playback::FaceAnimationManagers,
    attribute_runtime::tests::graph,
    input_cvar_runtime::InputCvarManager,
    players_backend_settings::PlayersBackendSettings,
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skater_manager_local_identity::{LocalBackendOwner, SkaterManagerLocalIdentity},
};
fn players() -> PlayerAttributeServices {
    PlayerAttributeServices {
        profile: crate::players_backend_settings::create::ProfileNameOwner::construct_name(),
        vault: Arc::new(graph(0x91bc_a669_3efc_9aa7, vec![])),
        settings: PlayersBackendSettings::construct_fields(),
        skaters: SkaterManagerLocalIdentity::construct_local_fields(
            LocalBackendOwner::construct_static_fields(PhysicalGameInfo::construct_network_field()),
        ),
    }
}
#[test]
fn animation_queries_observe_same_backend_writes_and_full_character_identity() {
    let players = players();
    let startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    let cvars = InputCvarReferences::new_static_initialized();
    let mut source = ActorAnimationPlayerServices::try_bind(&players, &startup, &cvars).unwrap();
    assert!(matches!(
        source.skater_id(0),
        Err(PlayerAnimationError::Settings(
            PlayersSettingsError::UnwrittenCharacter(0)
        ))
    ));
    assert!(!source.is_stance_regular(0).unwrap());
    let same = Arc::clone(&players.settings);
    same.try_set_character(0, 0xaaaa_bbbb_ffff_ffff).unwrap();
    same.try_set_stance_byte(0, 0x80).unwrap();
    assert_eq!(source.skater_id(0).unwrap(), 0xaaaa_bbbb_ffff_ffff);
    assert_eq!(source.marquee_id(0).unwrap(), 0xaaaa_bbbb_ffff_ffff);
    assert!(source.is_stance_regular(0).unwrap());
    same.try_set_character(0, u64::MAX).unwrap();
    assert_eq!(source.skater_id(0).unwrap(), u64::MAX);
    assert_eq!(source.player_gestures(0).unwrap(), [0, 1, 2, 3]);
    assert_eq!(source.player_style(0).unwrap(), 0);
    assert!(source.character_is_male(0).unwrap());
}
#[test]
fn actual_local_reset_and_ghost_owner_drive_selection_without_constant_answers() {
    let players = players();
    let startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    let cvars = InputCvarReferences::new_static_initialized();
    let mut source = ActorAnimationPlayerServices::try_bind(&players, &startup, &cvars).unwrap();
    assert_eq!(source.local_player_index().unwrap(), u32::MAX);
    players.skaters.try_reset_game_before_first_spawn().unwrap();
    assert_eq!(source.local_player_index().unwrap(), 0);
    assert!(!source.is_network_game().unwrap());
    assert!(!source.ghost_system_enabled().unwrap());
    players
        .skaters
        .backend()
        .try_apply_debug_menu_ghost_sample(true)
        .unwrap();
    assert!(source.ghost_system_enabled().unwrap());
    assert!(Arc::ptr_eq(
        &ActorAnimationSelection::command_manager(&mut source).unwrap(),
        &startup.managers.command
    ));
    assert!(Arc::ptr_eq(
        &ActorAnimationStartSource::playback_manager(&mut source).unwrap(),
        &startup.managers.playback
    ));
    assert!(Arc::ptr_eq(
        &source.database_manager().unwrap(),
        &startup.managers.database
    ));
}
#[test]
fn constructor_cvars_remain_live_and_missing_job_owner_is_not_replaced() {
    let players = players();
    let startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    let cvars = InputCvarReferences::new_static_initialized();
    let mut source = ActorAnimationPlayerServices::try_bind(&players, &startup, &cvars).unwrap();
    assert!(matches!(
        source.read_use_jobs_for_skater_anim_update(),
        Err(PlayerAnimationError::Cvar(_))
    ));
    let mut manager = InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkUseJobsForSkaterAnimUpdate=0\r\nkUseSprintButtonInput=0\r\n",
        ))
        .unwrap();
    assert!(!source.read_use_jobs_for_skater_anim_update().unwrap());
    assert_eq!(source.use_sprint_button().unwrap(), 0);
    manager
        .try_deserialize_startup(Some(
            b"[Config]\r\nkUseJobsForSkaterAnimUpdate=1\r\nkUseSprintButtonInput=1\r\n",
        ))
        .unwrap();
    assert!(source.read_use_jobs_for_skater_anim_update().unwrap());
    assert_eq!(source.use_sprint_button().unwrap(), 1);
    assert!(matches!(
        source.require_job_manager(),
        Err(PlayerAnimationError::MissingJobManager)
    ));
}

#[test]
fn physical_flags_use_full_character_and_lazy_live_gate_before_human_selection() {
    let players = players();
    let startup = SimulationAnimationStartup::new(FaceAnimationManagers::new());
    let cvars = InputCvarReferences::new_static_initialized();
    let mut source = ActorAnimationPlayerServices::try_bind(&players, &startup, &cvars).unwrap();
    assert!(matches!(
        source.try_physical_constructor_flags(0),
        Err(PlayerAnimationError::Cvar(_))
    ));
    let mut manager = InputCvarManager::new();
    manager.try_link(&cvars).unwrap();
    // Constructor leaves character unwritten: default-true must reach its getter.
    assert!(matches!(
        source.try_physical_constructor_flags(0),
        Err(PlayerAnimationError::Settings(
            PlayersSettingsError::UnwrittenCharacter(0)
        ))
    ));
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkBouncyBigBlackEnabled=0\r\n"))
        .unwrap();
    assert_eq!(
        source.try_physical_constructor_flags(8).unwrap(),
        PhysicalConstructorFlags {
            bouncy_ragdoll: 0,
            is_human: 1
        }
    );
    // Human still queries backend for invalid index, despite disabled bounce.
    assert!(matches!(
        source.try_physical_constructor_flags(9),
        Err(PlayerAnimationError::Settings(
            PlayersSettingsError::InvalidPlayer(9)
        ))
    ));
    players
        .settings
        .try_create_offline_human(0, &players.profile, players.skaters.backend())
        .unwrap();
    manager
        .try_deserialize_startup(Some(b"[Config]\r\nkBouncyBigBlackEnabled=1\r\n"))
        .unwrap();
    players
        .settings
        .try_set_character(0, 0x0376_a7b6_e3aa_dc5c)
        .unwrap();
    assert_eq!(
        source.try_physical_constructor_flags(0).unwrap(),
        PhysicalConstructorFlags {
            bouncy_ragdoll: 1,
            is_human: 1
        }
    );
    players
        .settings
        .try_set_character(0, 0xffff_ffff_e3aa_dc5c)
        .unwrap();
    assert_eq!(
        source
            .try_physical_constructor_flags(0)
            .unwrap()
            .bouncy_ragdoll,
        0
    );
    // Ghost true must skip the unwritten character getter for another valid row.
    players
        .skaters
        .backend()
        .try_apply_debug_menu_ghost_sample(true)
        .unwrap();
    assert_eq!(
        source
            .try_physical_constructor_flags(1)
            .unwrap()
            .bouncy_ragdoll,
        0
    );
}
