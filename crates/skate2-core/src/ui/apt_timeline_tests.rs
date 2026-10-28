use super::*;
use skate2_formats::apt::{Animation, Frame};

fn placement(depth: i32, character: u32, flags: u32, tx: f32) -> Control {
    Control::Place(Placement {
        flags,
        depth,
        character_id: character,
        matrix: Matrix { tx, ..IDENTITY },
        colour_scale: 0x80ff_ffff,
        colour_bias: 0,
        ratio: 0.,
        name: Some("child".into()),
        clip_depth: 0,
        events: Vec::new(),
        blend_mode: None,
    })
}

#[test]
fn jump_keeps_child_identity_and_does_not_execute_skipped_actions() {
    let movie = Movie {
        frames: vec![
            Frame {
                controls: vec![placement(1, 5, 6, 3.)],
            },
            Frame {
                controls: vec![
                    Control::Action(ActionStream { offset: 10 }),
                    placement(1, u32::MAX, 5, 9.),
                ],
            },
            Frame { controls: vec![] },
            Frame {
                controls: vec![Control::Action(ActionStream { offset: 20 })],
            },
        ],
    };
    let mut timeline = Timeline::default();
    timeline.goto(&movie, 0).unwrap();
    timeline.goto(&movie, 1).unwrap();
    // Non-adjacent jump starts from a MOVE at current frame1. The native
    // replacement helper updates existing geometry, not instance lifetime.
    let effects = timeline.goto(&movie, 3).unwrap();
    assert_eq!(
        timeline.entries()[&1].identity,
        PlacementIdentity {
            character: 5,
            frame: 0
        }
    );
    assert_eq!(timeline.entries()[&1].matrix.tx, 9.);
    assert!(
        effects
            .iter()
            .all(|e| !matches!(e, FrameEffect::Create { .. } | FrameEffect::Remove { .. }))
    );
    assert_eq!(
        effects.last(),
        Some(&FrameEffect::Action(ActionStream { offset: 20 }))
    );
    assert_eq!(
        effects
            .iter()
            .filter(|e| matches!(e, FrameEffect::Action(_)))
            .count(),
        1
    );
    assert!(timeline.goto(&movie, 3).unwrap().is_empty());
}

#[test]
fn rewind_restores_original_creation_and_removes_later_depths() {
    let movie = Movie {
        frames: vec![
            Frame {
                controls: vec![placement(1, 5, 6, 3.)],
            },
            Frame {
                controls: vec![Control::Remove { depth: 1 }],
            },
            Frame {
                controls: vec![placement(1, 5, 6, 8.), placement(2, 6, 6, 4.)],
            },
        ],
    };
    let mut timeline = Timeline::default();
    timeline.goto(&movie, 0).unwrap();
    timeline.goto(&movie, 2).unwrap();
    assert_eq!(timeline.entries()[&1].identity.frame, 2);
    let effects = timeline.goto(&movie, 0).unwrap();
    assert_eq!(timeline.entries().len(), 1);
    assert_eq!(timeline.entries()[&1].identity.frame, 0);
    assert_eq!(timeline.entries()[&1].matrix.tx, 3.);
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, FrameEffect::Remove { depth: 2, .. }))
    );
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, FrameEffect::Create { depth: 1, .. }))
    );
}

#[test]
fn invalid_jump_does_not_mutate_retained_display() {
    let movie = Movie {
        frames: vec![Frame {
            controls: vec![placement(1, 5, 6, 3.)],
        }],
    };
    let mut timeline = Timeline::default();
    timeline.goto(&movie, 0).unwrap();
    let entries = timeline.entries().clone();
    assert!(timeline.goto(&movie, 20).is_err());
    assert!(timeline.goto_label(&movie, "absent").is_err());
    assert_eq!(timeline.frame(), Some(0));
    assert_eq!(timeline.entries(), &entries);
}

fn corpus() -> Animation {
    let root = std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let bytes = std::fs::read(std::path::Path::new(&root).join("data/fe/screens.big")).unwrap();
    let archive = skate2_formats::big::BigArchive::parse(&bytes).unwrap();
    let read = |extension: &str| {
        let name = format!(r"data\fe\source\screens\hud2\hudphonelist.{extension}");
        let entry = archive.find_name(&name).unwrap();
        let data = &bytes[entry.data_range().unwrap()];
        if skate2_formats::refpack::decompressed_size(data).is_ok() {
            skate2_formats::refpack::decompress(data).unwrap()
        } else {
            data.to_vec()
        }
    };
    Animation::parse(&read("apt"), &read("const")).unwrap()
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_popup_label_changes_preserve_quick_open_and_hide_options() {
    let animation = corpus();
    let quick = animation.characters[76].as_ref().unwrap().movie().unwrap();
    let mut timeline = Timeline::default();
    timeline.goto_label(quick, "hidden").unwrap();
    let child_identity = timeline.entries()[&16].identity;
    let effects = timeline.goto_label(quick, "maximized").unwrap();
    assert_eq!(timeline.entries()[&16].identity, child_identity);
    assert_eq!(timeline.entries()[&16].colour_scale, u32::MAX);
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, FrameEffect::Create { depth: 16, .. }))
    );
    timeline.goto_label(quick, "hidden").unwrap();
    assert_eq!(timeline.entries()[&16].identity, child_identity);
    assert_eq!(timeline.entries()[&16].colour_scale >> 24, 0);

    let menu = animation.characters[75].as_ref().unwrap().movie().unwrap();
    let mut timeline = Timeline::default();
    timeline.goto_label(menu, "main").unwrap();
    let use_identity = timeline.entries()[&12].identity;
    timeline.goto_label(menu, "main2").unwrap();
    assert_eq!(timeline.entries()[&12].identity, use_identity);
    assert_eq!(timeline.entries()[&14].colour_scale >> 24, 0);
    assert_eq!(timeline.entries()[&28].colour_scale >> 24, 0);
    timeline.goto_label(menu, "main").unwrap();
    assert_eq!(timeline.entries()[&14].colour_scale, u32::MAX);
    assert_eq!(timeline.entries()[&28].colour_scale, u32::MAX);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_all_sprite_frames_and_rewinds_have_consistent_display_lists() {
    let animation = corpus();
    for (id, character) in animation.characters.iter().enumerate() {
        let Some(movie) = character.as_ref().and_then(|c| c.movie()) else {
            continue;
        };
        // Compare every authored target via sequential controls and a direct
        // jump from zero. This catches lost fields and accidental MOVE resets.
        let mut sequential = Timeline::default();
        for target in 0..movie.frames.len() {
            sequential.goto(movie, target).unwrap();
            let mut jump = Timeline::default();
            jump.goto(movie, 0).unwrap();
            jump.goto(movie, target).unwrap();
            assert_eq!(
                jump.entries(),
                sequential.entries(),
                "character {id}, frame {target}"
            );
        }
        for target in (0..movie.frames.len()).rev() {
            sequential.goto(movie, target).unwrap();
            let mut rebuilt = Timeline::default();
            rebuilt.goto(movie, target).unwrap();
            assert_eq!(
                sequential.entries(),
                rebuilt.entries(),
                "character {id}, rewind {target}"
            );
        }
    }
}
