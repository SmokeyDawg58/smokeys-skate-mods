use super::*;
use std::fmt::Write;

pub(crate) fn config(first: &str) -> String {
    let mut text = String::from("Unrelated = MAX(Button0, Button1) ? Button2.held : Button3;\n");
    for (index, name) in NAMES.iter().enumerate() {
        writeln!(
            text,
            "{name} = {};",
            if index == 0 { first } else { "0.0f" }
        )
        .unwrap();
    }
    text
}

#[test]
fn selected_debug_maps_require_authored_names_and_read_pressed_not_value() {
    let mut text = config("0.0");
    assert!(matches!(
        SelectedInputMap::parse_with_input_events(&text),
        Err(InputMapParseError::Missing(_))
    ));
    text.push_str("Alias=Button2; DebugPrev=Alias.pressed; DebugNext=Button1.pressed; DebugPause=0.0; DebugReplaceSkateboard=0.0; DebugResetWorld=0.0;");
    let map = SelectedInputMap::parse_with_input_events(&text).unwrap();
    let mut pad = Pad::new();
    pad.values[2] = 0.0;
    pad.values[1] = 1.0;
    assert_eq!(
        map.try_value(59, &mut pad).unwrap().to_bits(),
        1.0f32.to_bits()
    );
    assert_eq!(map.try_value(60, &mut pad).unwrap().to_bits(), 0);
    assert_eq!(pad.calls, ["count", "pressed2", "count", "pressed1"]);
    assert!(matches!(
        SelectedInputMap::parse(&text)
            .unwrap()
            .try_value(59, &mut pad),
        Err(InputMapReadError::UnsupportedMap(59))
    ));
}
struct Pad {
    count: Result<i32, &'static str>,
    values: [f32; 24],
    calls: Vec<String>,
    fail: Option<usize>,
}
impl Pad {
    fn new() -> Self {
        Self {
            count: Ok(24),
            values: [0.0; 24],
            calls: Vec::new(),
            fail: None,
        }
    }
}
impl InputMapPadValues for Pad {
    type Error = &'static str;
    fn pressed(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.calls.push(format!("pressed{index}"));
        if self.fail == Some(index) {
            Err("read")
        } else {
            Ok(index == 2)
        }
    }
    fn released(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.pressed(index)
    }
    fn repeat(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.pressed(index)
    }
    fn held(&mut self, index: usize) -> Result<bool, Self::Error> {
        self.calls.push(format!("held{index}"));
        if self.fail == Some(index) {
            Err("read")
        } else {
            Ok(self.values[index] != 0.0)
        }
    }
    fn num_buttons(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("count".into());
        self.count
    }
    fn value(&mut self, index: usize) -> Result<f32, Self::Error> {
        self.calls.push(format!("value{index}"));
        if self.fail == Some(index) {
            Err("read")
        } else {
            Ok(self.values[index])
        }
    }
}
#[test]
fn session_marker_states_read_the_right_operand_only_while_bumper_held() {
    let mut text = config("0.0");
    text.push_str("DebugPrev=0.0; DebugNext=0.0; DebugPause=0.0; DebugReplaceSkateboard=0.0; DebugResetWorld=0.0;");
    assert!(matches!(
        SelectedInputMap::parse_with_player_ui_inputs(&text),
        Err(InputMapParseError::Missing(_))
    ));
    text.push_str("LBumper = Button8; TopLeftShoulderHeld = LBumper.held; DPadU = Button0; DPadD = Button1; SessionMarkerSet = LBumper.held && DPadD.pressed; SessionMarkerUse = LBumper.HELD && DPadU.held;");
    let map = SelectedInputMap::parse_with_player_ui_inputs(&text).unwrap();
    let mut pad = Pad::new();
    assert_eq!(map.try_value(52, &mut pad).unwrap().to_bits(), 0);
    assert_eq!(pad.calls, ["count", "held8"]);
    pad.calls.clear();
    pad.values[8] = 1.0;
    assert_eq!(map.try_value(29, &mut pad).unwrap(), 1.0);
    assert_eq!(pad.calls, ["count", "held8"]);
    pad.calls.clear();
    pad.values[0] = 0.5;
    assert_eq!(
        map.try_value(53, &mut pad).unwrap().to_bits(),
        1.0f32.to_bits()
    );
    assert_eq!(pad.calls, ["count", "held8", "count", "held0"]);
    // This fixture reports pressed only for button 2, so DPadD stays clear.
    pad.calls.clear();
    assert_eq!(map.try_value(52, &mut pad).unwrap().to_bits(), 0);
    assert_eq!(pad.calls, ["count", "held8", "count", "pressed1"]);
    assert!(matches!(
        SelectedInputMap::parse_with_input_events(&text)
            .unwrap()
            .try_value(52, &mut pad),
        Err(InputMapReadError::UnsupportedMap(52))
    ));
}
#[test]
fn source_aliases_case_and_operand_reads_are_retained() {
    let text = format!(
        "Right = Button16; Left = Button17;\n{}",
        config("rIgHt - LEFT")
    );
    let map = SelectedInputMap::parse(&text).unwrap();
    let mut pad = Pad::new();
    pad.values[16] = 0.75;
    pad.values[17] = 0.125;
    assert_eq!(
        map.try_value(74, &mut pad).unwrap().to_bits(),
        0.625_f32.to_bits()
    );
    assert_eq!(pad.calls, ["count", "value16", "count", "value17"]);
    pad.values[16] = 0.5;
    assert_eq!(
        map.try_value(74, &mut pad).unwrap().to_bits(),
        0.375_f32.to_bits()
    );
}
#[test]
fn missing_device_is_not_an_owned_empty_pad_and_failure_stops_at_source_read() {
    let map = SelectedInputMap::parse(&config("Button16 - Button17")).unwrap();
    let mut pad = Pad::new();
    pad.count = Err("unavailable");
    assert_eq!(
        map.try_value(74, &mut pad),
        Err(InputMapReadError::Pad("unavailable"))
    );
    assert_eq!(pad.calls, ["count"]);
    pad.count = Ok(0);
    pad.calls.clear();
    assert_eq!(map.try_value(74, &mut pad).unwrap().to_bits(), 0);
    assert_eq!(pad.calls, ["count", "count"]);
    pad.count = Ok(24);
    pad.fail = Some(16);
    pad.calls.clear();
    assert_eq!(
        map.try_value(74, &mut pad),
        Err(InputMapReadError::Pad("read"))
    );
    assert_eq!(pad.calls, ["count", "value16"]);
    pad.fail = Some(17);
    pad.calls.clear();
    assert_eq!(
        map.try_value(74, &mut pad),
        Err(InputMapReadError::Pad("read"))
    );
    assert_eq!(pad.calls, ["count", "value16", "count", "value17"]);
}
#[test]
fn original_float_operations_keep_signed_zero_and_ieee_nonfinite_behavior() {
    let map = SelectedInputMap::parse(&config("Button16 - Button17")).unwrap();
    let mut pad = Pad::new();
    pad.values[16] = -0.0;
    assert_eq!(
        map.try_value(74, &mut pad).unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    pad.values[16] = f32::INFINITY;
    pad.values[17] = f32::INFINITY;
    assert!(map.try_value(74, &mut pad).unwrap().is_nan());
    let map = SelectedInputMap::parse(&config("Button16")).unwrap();
    pad.values[16] = f32::from_bits(0xffc1_2345);
    assert_eq!(map.try_value(74, &mut pad).unwrap().to_bits(), 0xffc1_2345);
    let map = SelectedInputMap::parse(&config("-0.0f")).unwrap();
    pad.calls.clear();
    assert_eq!(
        map.try_value(74, &mut pad).unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert!(pad.calls.is_empty());
}
#[test]
fn decimal_exponent_works_and_nested_operations_refuse_unimplemented_staging() {
    let map = SelectedInputMap::parse(&config("1.0f - 2.5e-1f")).unwrap();
    assert_eq!(
        map.try_value(74, &mut Pad::new()).unwrap().to_bits(),
        0.75_f32.to_bits()
    );
    for expression in [
        "Button0 - (Button1 - Button2)",
        "1 - 0.25 - 0.125",
        "-(Button1 - Button2)",
    ] {
        assert!(matches!(
            SelectedInputMap::parse(&config(expression)),
            Err(InputMapParseError::Unsupported { .. })
        ));
    }
    assert!(matches!(
        SelectedInputMap::parse(&format!("A = B; B = Button1; {}", config("A"))),
        Err(InputMapParseError::Missing(_))
    ));
}

#[test]
fn selected_unsupported_missing_cycles_duplicates_and_syntax_refuse() {
    assert!(matches!(
        SelectedInputMap::parse(&config("Button16 * Button17")),
        Err(InputMapParseError::Unsupported { .. })
    ));
    assert!(matches!(
        SelectedInputMap::parse(&config("Absent")),
        Err(InputMapParseError::Missing(_))
    ));
    assert!(matches!(
        SelectedInputMap::parse(&format!("A = B; B = A; {}", config("A"))),
        Err(InputMapParseError::Missing(_))
    ));
    assert!(matches!(
        SelectedInputMap::parse(&format!("A = Button1; a = Button2; {}", config("A"))),
        Err(InputMapParseError::Duplicate(_))
    ));
    assert!(matches!(
        SelectedInputMap::parse(&config("1e+")),
        Err(InputMapParseError::Syntax { .. })
    ));
    assert!(matches!(
        SelectedInputMap::parse(&config("Button154")),
        Err(InputMapParseError::Unsupported { .. })
    ));
    assert!(matches!(
        SelectedInputMap::parse("GP_LStickX = Button1"),
        Err(InputMapParseError::Syntax { .. })
    ));
    assert!(matches!(
        SelectedInputMap::parse(&config("Button16.value")),
        Err(InputMapParseError::Unsupported { .. })
    ));
}
#[test]
fn invalid_count_and_unselected_id_refuse_without_value_reads() {
    let map = SelectedInputMap::parse(&config("Button16")).unwrap();
    let mut pad = Pad::new();
    assert_eq!(
        map.try_value(73, &mut pad),
        Err(InputMapReadError::UnsupportedMap(73))
    );
    assert!(pad.calls.is_empty());
    pad.count = Ok(-1);
    assert_eq!(
        map.try_value(74, &mut pad),
        Err(InputMapReadError::ButtonRange {
            index: 16,
            count: -1
        })
    );
    assert_eq!(pad.calls, ["count"]);
    pad.count = Ok(16);
    pad.calls.clear();
    assert_eq!(
        map.try_value(74, &mut pad),
        Err(InputMapReadError::ButtonRange {
            index: 16,
            count: 16
        })
    );
    assert_eq!(pad.calls, ["count"]);
}

/// The existing mounted-original corpus supplies bytes from data/config/input.cfg.
/// This helper does not embed or replace the original asset or poll hardware.
pub(crate) fn check_original(text: &str) {
    let map = SelectedInputMap::parse(text).unwrap();
    let mut pad = Pad::new();
    pad.values = std::array::from_fn(|index| f32::from(u8::try_from(index).unwrap()) / 32.0);
    let direct = [6, 7, 10, 11, 8, 9, 0, 1, 2, 3, 14, 15, 12, 13];
    for (id, index) in [76, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91]
        .into_iter()
        .zip(direct)
    {
        let expected = pad.values[index];
        pad.calls.clear();
        assert_eq!(
            map.try_value(id, &mut pad).unwrap().to_bits(),
            expected.to_bits()
        );
        assert_eq!(pad.calls, ["count".to_owned(), format!("value{index}")]);
    }
    for (id, left, right) in [(74, 16, 17), (75, 18, 19), (77, 20, 21), (78, 22, 23)] {
        let expected = pad.values[left] - pad.values[right];
        pad.calls.clear();
        assert_eq!(
            map.try_value(id, &mut pad).unwrap().to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            pad.calls,
            [
                "count".to_owned(),
                format!("value{left}"),
                "count".to_owned(),
                format!("value{right}")
            ]
        );
    }
    let mut raw = crate::controller_input::RawControllerInput::default();
    raw.try_update(
        &mut MappedPadInput {
            map: &map,
            pad: &mut pad,
        },
        &crate::controller_input::RawControllerInput::default(),
        false,
        crate::controller_input::ControllerInputActivity::default(),
    )
    .unwrap();
    assert_eq!(raw.left_stick_x.to_bits(), (-1.0_f32 / 32.0).to_bits());
    assert_eq!(raw.right_stick_y.to_bits(), (-1.0_f32 / 32.0).to_bits());
    assert_eq!(raw.left_trigger.to_bits(), (10.0_f32 / 32.0).to_bits());
    assert!(raw.left_bumper && !raw.right_bumper);
    assert!(!raw.dpad_up && raw.dpad_down && raw.face_a);
    for id in 92..96 {
        pad.calls.clear();
        assert_eq!(map.try_value(id, &mut pad).unwrap().to_bits(), 0);
        assert!(pad.calls.is_empty());
    }
}
