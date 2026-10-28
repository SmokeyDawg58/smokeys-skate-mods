use super::*;
use crate::{
    attribute_runtime::tests::{collection, definition, graph, node, set_dependency_bytes},
    player::{CharacterRegistrationRequest, MarqueeDefinition, NO_PEER_ID},
    players_backend_settings::PhysicsSetting,
    simulation_assembly_backend::constraint_session::world_services::PhysicalGameInfo,
    skater_manager_local_identity::LocalBackendOwner,
};
const RECIPE: u64 = 0xfedc_ba98_7654_3210;
const ASSET: u64 = 0x1234_5678_1111_2222;

fn vault(regular: u8, hat: Option<u32>) -> Arc<AttributeRuntimeVault> {
    let mut vault = graph(MARQUEE_CLASS, vec![definition(HAT_KEY, 0, 4)]);
    collection(
        &mut vault,
        MARQUEE_CLASS,
        RECIPE,
        0,
        Some(0),
        hat.map(|value| vec![node(HAT_KEY, 0x40, value)])
            .unwrap_or_default(),
    );
    let mut bytes = vec![0; 256];
    bytes[16] = regular;
    set_dependency_bytes(&mut vault, 1, bytes);
    Arc::new(vault)
}
fn services(vault: Arc<AttributeRuntimeVault>) -> PlayerAttributeServices {
    let backend =
        LocalBackendOwner::construct_static_fields(PhysicalGameInfo::construct_network_field());
    let skaters = SkaterManagerLocalIdentity::construct_local_fields(backend);
    skaters.try_reset_game_before_first_spawn().unwrap();
    PlayerAttributeServices {
        profile: crate::players_backend_settings::create::ProfileNameOwner::construct_name(),
        vault,
        settings: PlayersBackendSettings::construct_fields(),
        skaters,
    }
}
fn pending(player: u32) -> PlayerLoaderSlot {
    let mut slot = PlayerLoaderSlot::new(player);
    slot.set_marquee(RECIPE, NO_PEER_ID);
    slot.begin_marquee_load(
        &MarqueeDefinition {
            id: RECIPE,
            schema_name: "fixture".into(),
        },
        12,
    )
    .unwrap();
    assert_eq!(
        slot.character_loaded(12, ASSET, 13).unwrap(),
        PlayerLoadAction::ApplyAttributes {
            character_id: ASSET,
            player_index: player
        }
    );
    slot
}

#[test]
fn native_backend_defaults_do_not_invent_a_character_and_reset_preserves_recipe() {
    let settings = PlayersBackendSettings::construct_fields();
    assert_eq!(
        settings.try_character(0),
        Err(PlayersSettingsError::UnwrittenCharacter(0))
    );
    assert_eq!(settings.try_stance_byte(0).unwrap(), 0);
    assert!(settings.try_character_is_male(0).unwrap());
    assert_eq!(settings.try_style(0).unwrap(), 0);
    assert_eq!(settings.try_gestures(0).unwrap(), [0, 1, 2, 3]);
    assert_eq!(settings.try_player_type(0).unwrap(), 0);
    settings.try_set_character(u32::MAX, RECIPE).unwrap();
    assert_eq!(
        settings.try_character(u32::MAX),
        Err(PlayersSettingsError::InvalidPlayer(u32::MAX))
    );
    assert_eq!(
        settings.try_set_stance_byte(u32::MAX, 1),
        Err(PlayersSettingsError::InvalidPlayer(u32::MAX))
    );
    settings.try_set_character(8, RECIPE).unwrap();
    settings.try_set_stance_byte(8, 0x80).unwrap();
    settings.try_set_hat(8, 99).unwrap();
    settings.try_reset_player_settings(8).unwrap();
    assert_eq!(settings.try_character(8).unwrap(), RECIPE);
    assert_eq!(settings.try_stance_byte(8).unwrap(), 0);
    assert_eq!(settings.try_hat(8).unwrap(), 0);
}

#[test]
fn actual_first_load_uses_cached_stance_and_fetched_hat_before_registration() {
    let services = services(vault(0x80, Some(0xfedc_ba98)));
    let mut slot = pending(0);
    assert!(matches!(
        slot.character_registered(13, ASSET),
        Err(PlayerLoaderError::UnexpectedRegistration)
    ));
    // Requested selection can change while the completed load still applies current.
    slot.set_marquee(7, 9);
    assert_eq!(
        slot.try_apply_loaded_attributes(&services).unwrap(),
        PlayerLoadAction::Register(CharacterRegistrationRequest {
            request_id: 13,
            character_id: ASSET,
            peer_id: NO_PEER_ID,
            player_index: 0,
            ghost_system_enabled: false,
            apply_attributes: true,
        })
    );
    assert_eq!(services.settings.try_character(0).unwrap(), RECIPE);
    assert_eq!(services.settings.try_stance_byte(0).unwrap(), 0x80);
    assert_eq!(services.settings.try_hat(0).unwrap(), 0xfedc_ba98);
    assert_eq!(
        services
            .settings
            .try_get(0, PhysicsSetting::TruckTightness)
            .unwrap()
            .to_bits(),
        0x3e99_999a
    );
    assert_eq!(
        services.settings.try_character(8),
        Err(PlayersSettingsError::UnwrittenCharacter(8))
    );
    assert_eq!(
        slot.character_registered(13, ASSET).unwrap(),
        PlayerLoadAction::Complete {
            character_id: ASSET
        }
    );
}

#[test]
fn live_ghost_owner_applies_the_same_selection_to_slot_eight() {
    let services = services(vault(1, None));
    services
        .skaters
        .backend()
        .try_apply_debug_menu_ghost_sample(true)
        .unwrap();
    let action = pending(0).try_apply_loaded_attributes(&services).unwrap();
    assert!(matches!(
        action,
        PlayerLoadAction::Register(CharacterRegistrationRequest {
            ghost_system_enabled: true,
            apply_attributes: true,
            ..
        })
    ));
    assert_eq!(services.settings.try_character(8).unwrap(), RECIPE);
    assert_eq!(services.settings.try_stance_byte(8).unwrap(), 1);
    assert_eq!(services.settings.try_hat(8).unwrap(), 0);
}

#[test]
fn missing_marquee_uses_generated_defaults_and_invalid_index_retains_loaded_asset() {
    let services = services(Arc::new(graph(MARQUEE_CLASS, vec![])));
    let mut slot = pending(0);
    slot.try_apply_loaded_attributes(&services).unwrap();
    assert_eq!(services.settings.try_character(0).unwrap(), RECIPE);
    assert_eq!(services.settings.try_stance_byte(0).unwrap(), 0);
    assert_eq!(services.settings.try_hat(0).unwrap(), 0);
    let mut invalid = pending(9);
    assert!(matches!(
        invalid.try_apply_loaded_attributes(&services),
        Err(PlayerAttributesError::Settings(
            PlayersSettingsError::InvalidPlayer(9)
        ))
    ));
    assert_eq!(
        invalid.phase(),
        PlayerLoadPhase::ApplyingAttributes {
            request_id: 13,
            character_id: ASSET
        }
    );
}

#[test]
fn stale_completion_does_not_enter_attribute_work() {
    let services = services(vault(1, Some(2)));
    let mut slot = PlayerLoaderSlot::new(0);
    slot.set_marquee(RECIPE, NO_PEER_ID);
    slot.begin_marquee_load(
        &MarqueeDefinition {
            id: RECIPE,
            schema_name: "fixture".into(),
        },
        12,
    )
    .unwrap();
    assert_eq!(
        slot.character_loaded(11, ASSET, 13).unwrap(),
        PlayerLoadAction::UnloadStale {
            character_id: ASSET
        }
    );
    assert!(matches!(
        slot.try_apply_loaded_attributes(&services),
        Err(PlayerAttributesError::WrongPhase(
            PlayerLoadPhase::LoadingAsset { request_id: 12 }
        ))
    ));
    assert_eq!(
        services.settings.try_character(0),
        Err(PlayersSettingsError::UnwrittenCharacter(0))
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn mounted_marquee_produces_backend_selection_before_first_registration() {
    let vault =
        crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[MARQUEE_CLASS]);
    let services = services(vault);
    let mut slot = PlayerLoaderSlot::new(0);
    slot.set_marquee(crate::player::DEFAULT_MARQUEE_ID, NO_PEER_ID);
    slot.begin_marquee_load(
        &MarqueeDefinition {
            id: crate::player::DEFAULT_MARQUEE_ID,
            schema_name: "tutorial_male".into(),
        },
        1,
    )
    .unwrap();
    slot.character_loaded(1, ASSET, 2).unwrap();
    assert!(matches!(
        slot.try_apply_loaded_attributes(&services).unwrap(),
        PlayerLoadAction::Register(_)
    ));
    assert_eq!(
        services.settings.try_character(0).unwrap(),
        crate::player::DEFAULT_MARQUEE_ID
    );
    assert_eq!(services.settings.try_hat(0).unwrap(), 0);
}
