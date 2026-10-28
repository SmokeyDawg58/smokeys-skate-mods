use super::*;
use crate::{
    game_input_manager::{EncodedGameInputCommand, GameInputSyncLevel},
    game_input_recognizer_runtime_services::GameInputRuntimeServices,
    input_cvar_runtime::{InputCvarManager, InputCvarReferences},
};

fn map() -> Arc<SelectedInputMap> {
    let names = [
        "GP_LStickX",
        "GP_LStickY",
        "GP_LStickIn",
        "GP_RStickX",
        "GP_RStickY",
        "GP_RStickIn",
        "GP_LTrigger",
        "GP_RTrigger",
        "GP_LBumper",
        "GP_RBumper",
        "GP_UDPad",
        "GP_DDPad",
        "GP_LDPad",
        "GP_RDPad",
        "GP_XFace",
        "GP_YFace",
        "GP_AFace",
        "GP_BFace",
        "GP_LStickRawX",
        "GP_LStickRawY",
        "GP_SensorZ",
        "GP_SensorX",
        "DebugNext",
        "DebugPause",
        "DebugReplaceSkateboard",
        "DebugResetWorld",
    ];
    let mut text = names.map(|name| format!("{name} = 0.0f;")).join("\n");
    text.push_str("\nDebugPrev = 1.0f;");
    Arc::new(SelectedInputMap::parse_with_input_events(&text).unwrap())
}
fn vault() -> Arc<AttributeRuntimeVault> {
    Arc::new(crate::attribute_runtime::tests::graph(
        0x7f08_6875_d621_c88c,
        vec![],
    ))
}
fn manager() -> GameplayInputUpdateManager<()> {
    let mut manager = GameplayInputUpdateManager::new();
    manager
        .try_construct(|_| Ok::<_, Infallible>(b"pattern Flick coord 0 1 coord 0 -1".to_vec()))
        .unwrap();
    manager
}
struct NoSync;
impl GameInputSyncLevel for NoSync {
    type Error = Infallible;
    fn sync_level(&mut self) -> Result<i32, Self::Error> {
        panic!("CRC disabled must not read syncLevel")
    }
}
fn sample(commands: &mut GameInputCommandManager, index: u32) {
    // Empty Unpack retains the actual OnCommand local constructor values;
    // the receiver still performs its source 24-channel pad update.
    commands
        .try_do_command(
            &EncodedGameInputCommand {
                receiver_id: index + 10,
                kind: 1,
                payload: &[],
            },
            false,
            &mut NoSync,
        )
        .unwrap();
}

#[test]
fn load_failure_preserves_ordered_published_collections_without_reset() {
    let mut manager = GameplayInputUpdateManager::<()>::new();
    let mut paths = Vec::new();
    let result = manager.try_construct(|path| {
        paths.push(path.to_owned());
        if paths.len() == 4 {
            return Err("fourth");
        }
        Ok(b"pattern Test coord 0 1 coord 0 -1".to_vec())
    });
    assert!(matches!(
        result,
        Err(GameplayInputLifecycleError::Load("fourth"))
    ));
    assert_eq!(
        paths,
        FILES[..4]
            .iter()
            .map(|(path, _)| (*path).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(manager.loaded_patterns(InputPatternStick::Right).len(), 3);
    assert!(manager.loaded_patterns(InputPatternStick::Left).is_empty());
    assert!(manager.player(0).is_none());
}

#[test]
fn reset_replaces_all_nine_players_and_invalidates_retained_old_alias() {
    let mut manager = manager();
    let mut commands = GameInputCommandManager::new();
    let map = map();
    let vault = vault();
    manager.try_reset(&map, &vault, &mut commands).unwrap();
    assert_eq!(manager.players.len(), 9);
    let old = Arc::clone(manager.player(0).unwrap());
    for index in 0..9 {
        sample(&mut commands, index);
        let player = manager.player(index as usize).unwrap().lock().unwrap();
        assert_eq!(player.player_index(), index);
        assert_eq!(player.try_native_update_eligible(1).unwrap(), index != 8);
        assert_eq!(player.try_native_update_eligible(2).unwrap(), index == 8);
    }
    manager.try_reset(&map, &vault, &mut commands).unwrap();
    assert!(!Arc::ptr_eq(&old, manager.player(0).unwrap()));
    assert!(matches!(
        old.lock().unwrap().pad(),
        Err(GameInputPlayerError::ConstructionRequired)
    ));
    sample(&mut commands, 0);
    assert_eq!(
        manager
            .player(0)
            .unwrap()
            .lock()
            .unwrap()
            .pad()
            .unwrap()
            .num_buttons(),
        24
    );
}

#[test]
fn overlay_precedes_empty_list_and_count_mask_gates_precede_cvars() {
    let mut manager = manager();
    let refs = InputCvarReferences::new_static_initialized();
    let vault = vault();
    let overlay = DebugOverlaySlot::default();
    let mut services = GameInputRuntimeServices::new(&refs, &vault, &overlay, None);
    assert!(matches!(
        manager.try_update(0, &overlay, &mut services),
        Err(GameplayInputUpdateError::Overlay(
            DebugOverlayError::Missing
        ))
    ));
    overlay.try_construct().unwrap();
    let mut commands = GameInputCommandManager::new();
    manager.try_reset(&map(), &vault, &mut commands).unwrap();
    assert_eq!(manager.try_update(3, &overlay, &mut services).unwrap(), 0);
    sample(&mut commands, 0);
    assert_eq!(manager.try_update(2, &overlay, &mut services).unwrap(), 0);
    assert!(matches!(
        manager.try_update(1, &overlay, &mut services),
        Err(GameplayInputUpdateError::Recognizer(
            GameInputRecognizerError::Source(_)
        ))
    ));
}

#[test]
fn late_player_failure_retains_earlier_event_and_does_not_flush() {
    let mut manager = manager();
    let vault = vault();
    let mut commands = GameInputCommandManager::new();
    manager.try_reset(&map(), &vault, &mut commands).unwrap();
    sample(&mut commands, 0);
    let later = Arc::clone(manager.player(1).unwrap());
    assert!(
        std::thread::spawn(move || {
            let _guard = later.lock().unwrap();
            panic!("late player fault");
        })
        .join()
        .is_err()
    );
    let refs = InputCvarReferences::new_static_initialized();
    let mut cvars = InputCvarManager::new();
    cvars.try_link(&refs).unwrap();
    let overlay = DebugOverlaySlot::default();
    overlay.try_construct().unwrap();
    let mut services = GameInputRuntimeServices::new(&refs, &vault, &overlay, None);
    assert!(matches!(
        manager.try_update(1, &overlay, &mut services),
        Err(GameplayInputUpdateError::PlayerLock)
    ));
    assert_eq!(manager.route.pending(), 1);
}

#[test]
fn both_flag_categories_enqueue_before_the_single_successful_flush() {
    let mut manager = manager();
    let vault = vault();
    let mut commands = GameInputCommandManager::new();
    manager.try_reset(&map(), &vault, &mut commands).unwrap();
    sample(&mut commands, 0);
    sample(&mut commands, 8);
    let refs = InputCvarReferences::new_static_initialized();
    let mut cvars = InputCvarManager::new();
    cvars.try_link(&refs).unwrap();
    let overlay = DebugOverlaySlot::default();
    overlay.try_construct().unwrap();
    let mut services = GameInputRuntimeServices::new(&refs, &vault, &overlay, None);
    assert_eq!(manager.try_update(3, &overlay, &mut services).unwrap(), 2);
    assert_eq!(manager.route.pending(), 0);
}
