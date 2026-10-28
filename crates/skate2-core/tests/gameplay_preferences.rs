use skate2_core::ui::{
    gameplay::{Difficulty, GameplayConfig, GameplaySettings, PhysicsMode},
    options::{Action, Controller, Page, ProfileSettings, Row, VideoScreen},
    pause_menu::PauseMenuInput,
};

fn press(menu: &mut Controller, profile: &mut ProfileSettings, input: PauseMenuInput) -> Action {
    menu.update(input, profile)
}

fn choose(menu: &mut Controller, profile: &mut ProfileSettings, target: Row) {
    for _ in 0..menu.snapshot().rows().len() {
        if menu.snapshot().rows()[menu.snapshot().selected] == target {
            return;
        }
        press(
            menu,
            profile,
            PauseMenuInput {
                down: true,
                ..Default::default()
            },
        );
    }
    panic!("menu does not offer {target:?}");
}

#[test]
fn both_entry_points_offer_independent_physics_and_difficulty_selectors() {
    for (in_game, screen) in [
        (false, VideoScreen::MainMenu),
        (true, VideoScreen::FreeSkate),
    ] {
        let mut menu = Controller::default();
        let mut profile = ProfileSettings::default();
        menu.open(in_game, screen, &profile);
        menu.acknowledge_intro();
        choose(&mut menu, &mut profile, Row::VideoSettings);
        press(
            &mut menu,
            &mut profile,
            PauseMenuInput {
                select: true,
                ..Default::default()
            },
        );
        assert_eq!(menu.snapshot().page, Page::Video);
        choose(&mut menu, &mut profile, Row::PhysicsMode);
        let original_profile = profile;
        for _ in 0..3 {
            press(
                &mut menu,
                &mut profile,
                PauseMenuInput {
                    right: true,
                    ..Default::default()
                },
            );
            let mode = menu.snapshot().gameplay.physics_mode;
            press(
                &mut menu,
                &mut profile,
                PauseMenuInput {
                    down: true,
                    ..Default::default()
                },
            );
            for expected in Difficulty::ALL {
                assert_eq!(
                    menu.snapshot().gameplay,
                    GameplaySettings {
                        physics_mode: mode,
                        difficulty: expected
                    }
                );
                press(
                    &mut menu,
                    &mut profile,
                    PauseMenuInput {
                        right: true,
                        ..Default::default()
                    },
                );
            }
            assert_eq!(
                menu.snapshot().gameplay.difficulty,
                Difficulty::Skate2Default
            );
            press(
                &mut menu,
                &mut profile,
                PauseMenuInput {
                    up: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(menu.snapshot().gameplay.physics_mode, PhysicsMode::Skate2);
        assert_eq!(profile, original_profile);
        assert!(!menu.display_dirty());
    }
}

#[test]
fn saved_combinations_survive_reopening_without_touching_profiles() {
    let root = std::env::var_os("SKATE2_SAVE_ROOT")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(format!("gameplay-preferences-🛹-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("gameplay.json");
    let result = std::panic::catch_unwind(|| {
        assert_eq!(
            GameplayConfig::load_from(&root.join("missing.json")).unwrap(),
            GameplayConfig::default()
        );
        for mode in PhysicsMode::ALL {
            for difficulty in Difficulty::ALL {
                let config = GameplayConfig {
                    settings: GameplaySettings {
                        physics_mode: mode,
                        difficulty,
                    },
                };
                config.save_to(&path).unwrap();
                assert_eq!(GameplayConfig::load_from(&path).unwrap(), config);
                let mut menu = Controller::default();
                menu.load_gameplay(GameplayConfig::load_from(&path).unwrap().settings);
                menu.open(false, VideoScreen::MainMenu, &ProfileSettings::default());
                assert_eq!(menu.snapshot().gameplay, config.settings);
                assert!(!menu.gameplay_dirty());
            }
        }
        std::fs::write(&path, r#"{"physics_mode":"unimplemented-game"}"#).unwrap();
        assert_eq!(
            GameplayConfig::load_from(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );
    });
    // Remove only this test's explicitly named file and empty directory.
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
    result.unwrap();
}
