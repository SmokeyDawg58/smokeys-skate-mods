use super::*;
use crate::attribute_runtime::tests::{collection, definition, graph, node};

fn attributes(threshold: Option<f32>) -> DerivedInputAttributes {
    let mut vault = graph(
        INPUT_LISTENER_CLASS,
        vec![definition(MAGNITUDE_THRESHOLD, 4, 4)],
    );
    collection(
        &mut vault,
        INPUT_LISTENER_CLASS,
        DEFAULT_COLLECTION,
        0,
        None,
        threshold
            .map(|value| vec![node(MAGNITUDE_THRESHOLD, 0x40, value.to_bits())])
            .unwrap_or_default(),
    );
    DerivedInputAttributes::new(&Arc::new(vault))
}
struct Input {
    values: Vec<f32>,
    calls: Vec<ControllerControl>,
    fail: Option<usize>,
}
impl Input {
    fn new() -> Self {
        Self {
            values: vec![0.0; 26],
            calls: Vec::new(),
            fail: None,
        }
    }
}
impl TryMappedControllerInput for Input {
    type Error = &'static str;
    fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error> {
        let index = self.calls.len();
        self.calls.push(control);
        if self.fail == Some(index) {
            Err("map")
        } else {
            Ok(self.values[index])
        }
    }
}
#[derive(Default)]
struct Numeric {
    squared: Vec<u32>,
    fail: Option<usize>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _a: [f32; 4], _b: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("source has scalar squared lengths")
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        let index = self.squared.len();
        self.squared.push(value.to_bits());
        if self.fail == Some(index) {
            Err("numeric")
        } else {
            Ok(1.0 / value.sqrt())
        }
    }
    fn reciprocal_estimate(&mut self, _value: f32) -> Result<f32, Self::Error> {
        panic!("not in Derived update")
    }
    fn truncate_i32(&mut self, _value: f32) -> Result<i32, Self::Error> {
        panic!("not in Derived update")
    }
}

#[test]
fn constructor_retains_source_half_max_edge_ages_and_zero_held_timers() {
    let derived = DerivedControllerInput::new();
    assert_eq!(derived.stick_edge_age.map(f32::to_bits), [0x7eff_ffff; 4]);
    assert_eq!(derived.trigger_held.map(f32::to_bits), [0; 2]);
    assert_eq!(derived.face_held.map(f32::to_bits), [0; 4]);
    assert_eq!(derived.magnitude_held.map(f32::to_bits), [0; 2]);
    assert_eq!(derived.current, RawControllerInput::default());
    assert_eq!(derived.previous, RawControllerInput::default());
}

#[test]
fn fresh_map_reads_source_order_and_exact_trigger_predicate_preserve_independent_timers() {
    let mut derived = DerivedControllerInput::new();
    derived.current.left_stick_x = 9.0;
    derived.trigger_held = [1.0, 2.0];
    derived.stick_edge_age = [1.0; 4];
    let mut input = Input::new();
    input.values[4] = 1.0;
    input.values[5] = f32::from_bits(1.0_f32.to_bits() - 1);
    input.values[6] = 1.0;
    // Raw faces are zero, but the later A/B/X/Y samples are fresh.
    input.values[18] = 1.0;
    input.values[19] = f32::NAN;
    input.values[20] = -0.0;
    input.values[21] = -1.0;
    input.values[22] = 0.75;
    let mut numeric = Numeric::default();
    derived
        .try_update(
            &mut input,
            0.25,
            ControllerInputActivity::default(),
            Some(&attributes(Some(0.5))),
            &mut numeric,
        )
        .unwrap();
    assert_eq!(
        input
            .calls
            .iter()
            .map(|control| *control as u8)
            .collect::<Vec<_>>(),
        [
            74, 75, 77, 78, 80, 81, 76, 79, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 90, 91, 88, 89,
            74, 75, 77, 78
        ]
    );
    assert_eq!(derived.previous.left_stick_x.to_bits(), 9.0_f32.to_bits());
    assert_eq!(
        derived.trigger_held.map(f32::to_bits),
        [1.25_f32.to_bits(), 0]
    );
    assert_eq!(
        derived.stick_edge_age.map(f32::to_bits),
        [
            0,
            1.25_f32.to_bits(),
            1.25_f32.to_bits(),
            1.25_f32.to_bits()
        ]
    );
    assert_eq!(
        derived.face_held.map(f32::to_bits),
        [
            0.25_f32.to_bits(),
            0.25_f32.to_bits(),
            0,
            0.25_f32.to_bits()
        ]
    );
    assert_eq!(
        derived.magnitude_held.map(f32::to_bits),
        [0.25_f32.to_bits(), 0]
    );
    assert!(!derived.current.face_a);
}

#[test]
fn scalar_fused_squared_lengths_and_late_missing_global_preserve_prior_prefix() {
    let mut derived = DerivedControllerInput::new();
    derived.magnitude_held = [3.0, 4.0];
    let mut input = Input::new();
    input.values[18] = 1.0;
    input.values[22] = f32::from_bits(0x3f01_2345);
    input.values[23] = f32::from_bits(0x3e91_2345);
    input.values[24] = 0.3;
    input.values[25] = 0.4;
    let [lx, ly, rx, ry] = [
        input.values[22],
        input.values[23],
        input.values[24],
        input.values[25],
    ];
    let mut numeric = Numeric::default();
    assert!(matches!(
        derived.try_update(
            &mut input,
            0.5,
            ControllerInputActivity::default(),
            None,
            &mut numeric
        ),
        Err(DerivedInputError::MissingGlobalAttributes)
    ));
    assert_eq!(
        numeric.squared,
        [
            ly.mul_add(ly, lx * lx).to_bits(),
            ry.mul_add(ry, rx * rx).to_bits()
        ]
    );
    assert_eq!(derived.face_held[0].to_bits(), 0.5_f32.to_bits());
    assert_eq!(
        derived.magnitude_held.map(f32::to_bits),
        [3.0_f32.to_bits(), 4.0_f32.to_bits()]
    );
}

#[test]
fn raw_and_late_sample_failures_keep_completed_fields_without_sampling_suffix() {
    for fail in [1, 19] {
        let mut derived = DerivedControllerInput::new();
        derived.current.left_stick_x = 7.0;
        derived.face_held = [10.0; 4];
        let mut input = Input::new();
        input.values[0] = 2.0;
        input.values[18] = 1.0;
        input.fail = Some(fail);
        let mut numeric = Numeric::default();
        assert!(matches!(
            derived.try_update(
                &mut input,
                0.25,
                ControllerInputActivity::default(),
                Some(&attributes(None)),
                &mut numeric
            ),
            Err(DerivedInputError::Input("map"))
        ));
        assert_eq!(input.calls.len(), fail + 1);
        assert_eq!(derived.previous.left_stick_x.to_bits(), 7.0_f32.to_bits());
        assert_eq!(derived.current.left_stick_x.to_bits(), 2.0_f32.to_bits());
        assert_eq!(
            derived.face_held[0].to_bits(),
            if fail == 19 {
                10.25_f32.to_bits()
            } else {
                10.0_f32.to_bits()
            }
        );
        assert_eq!(derived.face_held[1].to_bits(), 10.0_f32.to_bits());
        assert!(numeric.squared.is_empty());
    }
}

#[test]
fn falling_edges_and_unordered_threshold_reset_both_magnitude_timers() {
    let mut derived = DerivedControllerInput::new();
    derived.current.left_stick_in = true;
    derived.current.right_stick_in = true;
    derived.stick_edge_age = [2.0; 4];
    derived.magnitude_held = [3.0, 4.0];
    derived
        .try_update(
            &mut Input::new(),
            0.5,
            ControllerInputActivity::default(),
            Some(&attributes(Some(f32::NAN))),
            &mut Numeric::default(),
        )
        .unwrap();
    assert_eq!(
        derived.stick_edge_age.map(f32::to_bits),
        [2.5_f32.to_bits(), 0, 2.5_f32.to_bits(), 0]
    );
    assert_eq!(derived.magnitude_held.map(f32::to_bits), [0, 0]);
}

#[test]
fn unordered_fresh_magnitude_resets_its_age_without_resetting_other_stick() {
    let mut derived = DerivedControllerInput::new();
    derived.magnitude_held = [3.0, 4.0];
    let mut input = Input::new();
    input.values[22] = f32::NAN;
    input.values[24] = 1.0;
    derived
        .try_update(
            &mut input,
            0.5,
            ControllerInputActivity::default(),
            Some(&attributes(Some(0.25))),
            &mut Numeric::default(),
        )
        .unwrap();
    assert_eq!(
        derived.magnitude_held.map(f32::to_bits),
        [0, 4.5_f32.to_bits()]
    );
}

#[test]
fn missing_attribute_falls_back_zero_and_numeric_failure_precedes_owner_check() {
    let mut derived = DerivedControllerInput::new();
    let mut input = Input::new();
    input.values[22] = 1.0;
    derived
        .try_update(
            &mut input,
            1.0,
            ControllerInputActivity::default(),
            Some(&attributes(None)),
            &mut Numeric::default(),
        )
        .unwrap();
    assert_eq!(
        derived.magnitude_held.map(f32::to_bits),
        [1.0_f32.to_bits(), 0]
    );
    let mut numeric = Numeric {
        fail: Some(1),
        ..Numeric::default()
    };
    assert!(matches!(
        derived.try_update(
            &mut Input::new(),
            1.0,
            ControllerInputActivity::default(),
            None,
            &mut numeric
        ),
        Err(DerivedInputError::Numeric("numeric"))
    ));
    assert_eq!(numeric.squared.len(), 2);
    assert_eq!(
        derived.magnitude_held.map(f32::to_bits),
        [1.0_f32.to_bits(), 0]
    );
}
