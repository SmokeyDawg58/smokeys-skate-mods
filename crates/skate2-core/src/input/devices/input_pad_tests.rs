use super::*;
struct Device {
    guitar: bool,
    remap: bool,
    events: Vec<&'static str>,
}
impl InputPadDeviceEffects for Device {
    type Error = ();
    type Remap = ();
    fn is_guitar(&mut self) -> Result<bool, ()> {
        self.events.push("guitar");
        Ok(self.guitar)
    }
    fn capture_remap(&mut self) -> Result<Option<()>, ()> {
        self.events.push("capture");
        Ok(self.remap.then_some(()))
    }
    fn remap(&mut self, (): &(), player: u32, values: &mut [f32; 24]) -> Result<(), ()> {
        self.events.push("remap");
        assert_eq!(player, 0);
        assert_eq!(values[10].to_bits(), 0);
        values[3] = 0.75;
        Ok(())
    }
}
struct Numeric {
    seeds: Vec<u32>,
    fail: usize,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = usize;
    fn dot3(&mut self, _: [f32; 3], _: [f32; 3]) -> Result<f32, usize> {
        panic!("pad square uses scalar FMA")
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, usize> {
        self.seeds.push(v.to_bits());
        if self.seeds.len() == self.fail {
            Err(self.fail)
        } else {
            Ok(1.0 / v.sqrt())
        }
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, usize> {
        panic!("pad uses scalar division")
    }
}
fn packet() -> XboxPadSample {
    XboxPadSample {
        buttons: 0,
        left_trigger: 0,
        right_trigger: 0,
        left_x: 0,
        left_y: 0,
        right_x: 0,
        right_y: 0,
    }
}
fn services() -> (Numeric, Device) {
    (
        Numeric {
            seeds: vec![],
            fail: 0,
        },
        Device {
            guitar: false,
            remap: false,
            events: vec![],
        },
    )
}
#[test]
fn digital_bits_and_triggers_are_exact_and_axes_zero_still_estimate() {
    for bit in 0u8..16 {
        let mut sample = packet();
        sample.buttons = 1u16 << bit;
        sample.left_trigger = 255;
        sample.right_trigger = 127;
        let (mut n, mut d) = services();
        let mut out = [-99.0; 24];
        try_convert_xbox_pad(sample, &mut out, &mut n, &mut d).unwrap();
        for channel in (0usize..10).chain(12..16) {
            assert_eq!(
                out[channel].to_bits(),
                if channel == usize::from(bit) {
                    1.0f32.to_bits()
                } else {
                    0
                }
            );
        }
        assert_eq!(out[10].to_bits(), 1.0f32.to_bits());
        assert_eq!(
            out[11].to_bits(),
            (127.0 * f32::from_bits(0x3b80_8081)).to_bits()
        );
        assert!(out[16..].iter().all(|v| v.to_bits() == 0));
        assert_eq!(n.seeds, [0, 0]);
    }
}
#[test]
fn signed_extrema_deadzone_and_half_axis_order() {
    let (mut n, mut d) = services();
    let mut s = packet();
    s.left_x = -32768;
    s.left_y = 0;
    s.right_x = 0;
    s.right_y = 32767;
    let mut out = [0.0; 24];
    try_convert_xbox_pad(s, &mut out, &mut n, &mut d).unwrap();
    assert!((out[17] - 1.0).abs() < 1e-6);
    assert!((out[22] - 1.0).abs() < 1e-6);
    for i in [16, 18, 19, 20, 21, 23] {
        assert_eq!(out[i].to_bits(), 0);
    }
    s.left_x = 8192;
    s.right_y = 0;
    try_convert_xbox_pad(s, &mut out, &mut n, &mut d).unwrap();
    assert_eq!(out[16].to_bits(), 0);
    assert_eq!(out[17].to_bits(), 0);
    s.left_x = 16384;
    try_convert_xbox_pad(s, &mut out, &mut n, &mut d).unwrap();
    assert!((out[16] - f32::from_bits(0x3eb6_db6e)).abs() < 1e-6);
}
#[test]
fn fma_squared_input_is_not_two_rounded_squares() {
    let (mut n, mut d) = services();
    let mut s = packet();
    s.left_x = 20001;
    s.left_y = 16599;
    try_convert_xbox_pad(s, &mut [0.0; 24], &mut n, &mut d).unwrap();
    assert_eq!(n.seeds[0], 0x3f21_1153);
    assert_ne!(n.seeds[0], 0x3f21_1154);
}
#[test]
fn right_seed_failure_retains_digital_and_left_stores_only() {
    let (mut n, mut d) = services();
    n.fail = 2;
    let mut s = packet();
    s.left_x = 32767;
    s.buttons = 1;
    let mut out = [42.0; 24];
    assert_eq!(
        try_convert_xbox_pad(s, &mut out, &mut n, &mut d),
        Err(InputPadConversionError::Numeric(2))
    );
    assert_eq!(out[0].to_bits(), 1.0f32.to_bits());
    assert!((out[16] - 1.0).abs() < 1e-6);
    assert!(out[20..].iter().all(|v| v.to_bits() == 42.0f32.to_bits()));
    assert!(d.events.is_empty());
}
#[test]
fn guitar_overwrites_before_actual_remap() {
    let (mut n, mut d) = services();
    d.guitar = true;
    d.remap = true;
    let mut s = packet();
    s.left_trigger = 255;
    s.right_trigger = 255;
    s.right_x = -32768;
    s.right_y = 32767;
    let mut out = [0.0; 24];
    try_convert_xbox_pad(s, &mut out, &mut n, &mut d).unwrap();
    for i in [10, 11, 21, 22] {
        assert_eq!(out[i].to_bits(), 0);
    }
    assert_eq!(out[3].to_bits(), 0.75f32.to_bits());
    assert_eq!(d.events, ["guitar", "capture", "remap"]);
}
#[test]
fn pad_constructor_empty_branch_precedes_invalid_index() {
    let mut pad = InputPad::new();
    assert_eq!(pad.num_buttons(), 0);
    assert_eq!(pad.value(-1).unwrap().to_bits(), 0);
    assert!(!pad.held(99).unwrap());
    assert!(pad.buttons.iter().all(Option::is_none));
    assert_eq!(
        pad.try_update(&[], 25),
        Err(InputPadError::CountTooLarge(25))
    );
    assert_eq!(
        pad.try_update(&[], -1),
        Err(InputPadError::NegativeCount(-1))
    );
    assert!(pad.try_update(&[], 1).is_err());
    assert_eq!(pad.num_buttons(), 0);
    pad.try_update(&[0.0], 1).unwrap();
    assert!(pad.value(-1).is_err());
    assert!(pad.value(1).is_err());
}
#[test]
fn immediate_value_but_fourth_update_first_press_and_release_gate() {
    let mut pad = InputPad::new();
    for _ in 0..3 {
        pad.try_update(&[1.0], 1).unwrap();
        assert_eq!(pad.value(0).unwrap().to_bits(), 1.0f32.to_bits());
        assert!(!pad.held(0).unwrap());
        assert!(!pad.pressed(0).unwrap());
    }
    pad.try_update(&[1.0], 1).unwrap();
    assert!(pad.held(0).unwrap());
    assert!(pad.pressed(0).unwrap());
    assert!(pad.repeat(0).unwrap());
    for _ in 0..3 {
        pad.try_update(&[0.0], 1).unwrap();
        assert_eq!(pad.value(0).unwrap().to_bits(), 0);
        assert!(pad.held(0).unwrap());
        assert!(!pad.released(0).unwrap());
    }
    pad.try_update(&[0.0], 1).unwrap();
    assert!(!pad.held(0).unwrap());
    assert!(pad.released(0).unwrap());
    assert!(!pad.repeat(0).unwrap());
}
#[test]
fn repeats_after_twenty_four_then_twelve_updates() {
    let mut pad = InputPad::new();
    for _ in 0..4 {
        pad.try_update(&[1.0], 1).unwrap();
    }
    assert!(pad.repeat(0).unwrap());
    for period in [24, 12, 12] {
        for step in 1..=period {
            pad.try_update(&[1.0], 1).unwrap();
            assert_eq!(pad.repeat(0).unwrap(), step == period);
            assert!(!pad.pressed(0).unwrap());
        }
    }
}
#[test]
fn shrink_preserves_hidden_records_and_regrowth_clears_only_newly_exposed() {
    let mut pad = InputPad::new();
    for _ in 0..4 {
        pad.try_update(&[1.0, 1.0], 2).unwrap();
    }
    pad.try_update(&[1.0], 1).unwrap();
    assert!(pad.buttons[1].unwrap().held);
    pad.try_update(&[1.0, 1.0], 2).unwrap();
    assert!(pad.held(0).unwrap());
    assert!(!pad.held(1).unwrap());
    pad.try_update(&[], 0).unwrap();
    assert!(pad.buttons[0].unwrap().held);
    assert!(!pad.released(0).unwrap());
    pad.try_update(&[1.0], 1).unwrap();
    assert!(!pad.held(0).unwrap());
}
#[test]
fn nan_and_threshold_are_not_button_held_and_values_are_not_sanitized() {
    let mut pad = InputPad::new();
    for _ in 0..4 {
        pad.try_update(&[0.5, f32::from_bits(0x7fc1_2345)], 2)
            .unwrap();
    }
    assert!(!pad.held(0).unwrap());
    assert!(!pad.held(1).unwrap());
    assert_eq!(pad.value(1).unwrap().to_bits(), 0x7fc1_2345);
}
#[test]
fn counters_preserve_signed_compare_and_wrapping_repeat_decrement() {
    let mut b = Button::ZERO;
    b.transition_counter = -1;
    b.update(1.0);
    assert_eq!(b.transition_counter, 0);
    assert!(!b.held);
    b.held = true;
    b.repeat_counter = i32::MIN;
    b.update(1.0);
    assert_eq!(b.repeat_counter, i32::MAX);
    assert!(!b.repeat);
}

/// Controlled physical packet -> source converter -> SAME retained pad -> actual
/// mounted Xbox mappings -> source raw controller. Does not simulate cache timing.
pub(crate) fn check_original_map(text: &str) {
    use crate::{
        controller_input::{ControllerInputActivity, RawControllerInput},
        input_map::{MappedPadInput, SelectedInputMap},
    };
    let map = SelectedInputMap::parse(text).unwrap();
    let mut sample = packet();
    sample.buttons = (1 << 8) | (1 << 9) | (1 << 12);
    sample.left_trigger = 255;
    sample.left_x = 16384;
    sample.right_y = -32768;
    let (mut numeric, mut device) = services(); // explicitly controlled normal device
    let mut values = [f32::NAN; 24];
    try_convert_xbox_pad(sample, &mut values, &mut numeric, &mut device).unwrap();
    let mut pad = InputPad::new();
    let mut raw = RawControllerInput::default();
    for frame in 0..4 {
        pad.try_update(&values, 24).unwrap();
        let previous = raw;
        raw.try_update(
            &mut MappedPadInput {
                map: &map,
                pad: &mut pad,
            },
            &previous,
            false,
            ControllerInputActivity::default(),
        )
        .unwrap();
        assert!(
            raw.face_a,
            "map GetState reads Value even before pad Held transition"
        );
        assert_eq!(pad.held(12).unwrap(), frame == 3);
        assert_eq!(pad.pressed(12).unwrap(), frame == 3);
        assert!(raw.left_bumper && !raw.right_bumper);
        assert_eq!(raw.left_trigger.to_bits(), 1.0_f32.to_bits());
        assert!((raw.left_stick_x - (0.25_f32 * f32::from_bits(0x3fb6_db6e))).abs() < 0.000_001);
        assert!((raw.right_stick_y + 1.0).abs() < 0.000_001);
    }
    sample.buttons = 0;
    try_convert_xbox_pad(sample, &mut values, &mut numeric, &mut device).unwrap();
    pad.try_update(&values, 24).unwrap();
    let previous = raw;
    raw.try_update(
        &mut MappedPadInput {
            map: &map,
            pad: &mut pad,
        },
        &previous,
        false,
        ControllerInputActivity::default(),
    )
    .unwrap();
    assert!(!raw.face_a);
    assert!(
        pad.held(12).unwrap(),
        "transition suppression retains Held while Value is zero"
    );
    assert!(!pad.released(12).unwrap());
    println!(
        "original Xbox input.cfg + raw integer packet -> source radial/trigger conversion -> retained pad Value/edge distinction -> source controller arbitration; controlled four Pad updates, no poll/cache/Actor timing claim"
    );
}
