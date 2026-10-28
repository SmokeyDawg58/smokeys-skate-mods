use super::*;

/// Frames submitted 4.5 ms after each 60 Hz tick is due, restored by a clock stepped
/// at `render_hz`: the shown simulation time of each rendered frame, in seconds.
fn shown_times(render_hz: f64, seconds: f64) -> Vec<f64> {
    let mut frames = PresentationFrames::new(4);
    let mut submitted = 0_u32;
    let written = |time: f64| ((time - 0.0045) * 60.0).floor() as i64;
    let mut time = 0.05;
    while i64::from(submitted) <= written(time) {
        frames.submit(());
        submitted += 1;
    }
    let mut clock = RealTimeController::new(&frames, NORMAL_UPDATE_RATE);
    let mut shown = Vec::new();
    while time < seconds {
        let dt = 1.0 / render_hz;
        time += dt;
        while i64::from(submitted) <= written(time) {
            frames.submit(());
            submitted += 1;
        }
        clock.update(dt as f32, &frames, NORMAL_UPDATE_RATE);
        let info = clock.time_info();
        shown.push((f64::from(info.frame) - 1.0 + f64::from(info.fraction)) / 60.0);
    }
    shown
}

fn steps(shown: &[f64]) -> Vec<f64> {
    shown
        .windows(2)
        .skip(10)
        .map(|pair| pair[1] - pair[0])
        .collect()
}

#[test]
fn sixty_hertz_shows_every_tick_once() {
    for step in steps(&shown_times(60.0, 2.0)) {
        assert!((step - 1.0 / 60.0).abs() < 1e-4, "step {step}");
    }
}

#[test]
fn one_hundred_twenty_hertz_shows_even_halves() {
    for step in steps(&shown_times(120.0, 2.0)) {
        assert!((step - 1.0 / 120.0).abs() < 1e-4, "step {step}");
    }
}

#[test]
fn faster_rates_never_show_an_earlier_moment() {
    for hz in [144.0, 165.0, 240.0, 1000.0] {
        for step in steps(&shown_times(hz, 2.0)) {
            assert!(step > -1e-9, "{hz} Hz step {step}");
        }
    }
}

#[test]
fn a_new_frame_restores_the_newest() {
    let mut frames = PresentationFrames::new(2);
    frames.submit('a');
    let mut clock = RealTimeController::new(&frames, NORMAL_UPDATE_RATE);
    frames.submit('b');
    frames.submit('c');
    clock.update(0.001, &frames, NORMAL_UPDATE_RATE);
    let info = clock.time_info();
    assert_eq!(frames.get(info.frame), Some(&'c'));
    assert_eq!(frames.previous(info.frame), Some(&'b'));
    assert_eq!(info.delta, 1.0);
}

#[test]
fn an_unchanged_frame_accumulates_its_phase_up_to_one() {
    let mut frames = PresentationFrames::new(2);
    frames.submit(());
    let mut clock = RealTimeController::new(&frames, NORMAL_UPDATE_RATE);
    frames.submit(());
    clock.update(0.004, &frames, NORMAL_UPDATE_RATE);
    let first = clock.time_info();
    clock.update(0.004, &frames, NORMAL_UPDATE_RATE);
    let second = clock.time_info();
    assert_eq!(second.frame, first.frame);
    assert!(second.fraction > first.fraction);
    for _ in 0..8 {
        clock.update(0.004, &frames, NORMAL_UPDATE_RATE);
    }
    assert_eq!(clock.time_info().fraction, 1.0);
}

#[test]
fn skip_forward_stops_at_the_newest_frame() {
    let mut frames = PresentationFrames::new(3);
    for _ in 0..5 {
        frames.submit(());
    }
    assert_eq!(frames.last_written(), 4);
    assert_eq!(frames.skip_forward(2, 0), 2);
    assert_eq!(frames.skip_forward(2, 1), 3);
    assert_eq!(frames.skip_forward(2, 9), 4);
    assert_eq!(frames.skip_forward(0, 1), 3);
}
