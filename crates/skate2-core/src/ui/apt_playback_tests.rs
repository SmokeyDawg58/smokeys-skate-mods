use super::*;
use skate2_formats::apt::{ActionStream, Control, Frame};

#[test]
fn authored_clock_keeps_remainder_and_catches_up() {
    let mut clock = MovieClock::default();
    let interval = std::num::NonZeroU32::new(16).unwrap();
    assert_eq!(clock.advance(7, interval, false), 0);
    assert_eq!(clock.advance(8, interval, false), 0);
    assert_eq!(clock.advance(2, interval, false), 1);
    assert_eq!(clock.remainder_ms(), 1);
    assert_eq!(clock.advance(50, interval, false), 3);
    assert_eq!(clock.remainder_ms(), 3);
    assert_eq!(clock.advance(50, interval, true), 1);
    assert_eq!(clock.remainder_ms(), 37);
    assert_eq!(clock.advance(0, interval, false), 2);
    assert_eq!(clock.remainder_ms(), 5);
}

#[test]
fn stop_then_same_label_play_resumes_without_replaying_current_actions() {
    let movie = Movie {
        frames: vec![
            Frame {
                controls: vec![
                    Control::Label("start".into()),
                    Control::Action(ActionStream { offset: 10 }),
                ],
            },
            Frame {
                controls: vec![Control::Action(ActionStream { offset: 20 })],
            },
        ],
    };
    let mut clip = ClipPlayback::default();
    assert_eq!(
        clip.tick(&movie).unwrap(),
        [FrameEffect::Action(ActionStream { offset: 10 })]
    );
    clip.stop();
    assert!(clip.tick(&movie).unwrap().is_empty());
    assert!(clip.goto_label(&movie, "start", true).unwrap().is_empty());
    assert_eq!(
        clip.tick(&movie).unwrap(),
        [FrameEffect::Action(ActionStream { offset: 20 })]
    );
    assert_eq!(
        clip.tick(&movie).unwrap(),
        [FrameEffect::Action(ActionStream { offset: 10 })]
    );
}
