//! Mapped player controls and the source bumper arbitration step.
//!
//! This starts at completed `cInputMap` expression results. Hardware polling,
//! pad normalization, mapping expressions, and gesture recognition are not
//! implemented here. Reference: `RawControllerInput::Update`, `0x825EE088`.

/// The gameplay portion of PDB `Sk8::Input::eInputStates`.
/// IDs are map identities, not hardware button indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ControllerControl {
    LeftStickX = 74,
    LeftStickY = 75,
    LeftStickIn = 76,
    RightStickX = 77,
    RightStickY = 78,
    RightStickIn = 79,
    LeftTrigger = 80,
    RightTrigger = 81,
    LeftBumper = 82,
    RightBumper = 83,
    DpadUp = 84,
    DpadDown = 85,
    DpadLeft = 86,
    DpadRight = 87,
    FaceX = 88,
    FaceY = 89,
    FaceA = 90,
    FaceB = 91,
}

/// Supplies a completed map-expression value for one gameplay control.
///
/// Each invocation corresponds to one native map evaluation with the same
/// bound pad, map, and four global arguments. A provider must supply those
/// semantics; this trait does not reconstruct `cInputExec` or poll a device.
/// Button values are converted by [`RawControllerInput::update`] with the
/// native `GetState` nonzero predicate (`0x8298C8C8`).
pub trait MappedControllerInput {
    fn mapped_value(&mut self, control: ControllerControl) -> f32;
}

/// Fallible map evaluation for a retained runtime input provider.
/// Missing owners or unsupported map expressions must remain errors.
pub trait TryMappedControllerInput {
    type Error;
    /// # Errors
    /// Returns the provider error if the current expression cannot be evaluated.
    fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error>;
}

struct InfallibleMappedInput<'a, T>(&'a mut T);
impl<T: MappedControllerInput> TryMappedControllerInput for InfallibleMappedInput<'_, T> {
    type Error = std::convert::Infallible;
    fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error> {
        Ok(self.0.mapped_value(control))
    }
}

/// Precomputed expression results for map IDs 74 through 91, in ID order.
/// These are already mapped values, not an Xbox/PC hardware packet.
#[derive(Clone, Copy, Debug)]
pub struct MappedControllerSnapshot {
    values: [f32; 18],
}

impl MappedControllerSnapshot {
    #[must_use]
    pub const fn new(values_in_map_id_order: [f32; 18]) -> Self {
        Self {
            values: values_in_map_id_order,
        }
    }
}

impl MappedControllerInput for MappedControllerSnapshot {
    fn mapped_value(&mut self, control: ControllerControl) -> f32 {
        self.values[control as usize - ControllerControl::LeftStickX as usize]
    }
}

/// Actor state used when both mapped bumpers are held.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ControllerInputActivity {
    pub pushing: bool,
    pub skitching: bool,
}

/// Semantic fields of the native 0x1c-byte raw controller input.
///
/// Reference: `Sk8::Skater::DerivedControllerInput::RawControllerInput`,
/// `Update` at `0x825EE088`. Rust does not preserve the packed layout or its
/// twenty unused bits. `Default` is a host zero-state convenience; no original
/// constructor or reset is claimed. "Raw" here still means *mapped* controls.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawControllerInput {
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,
    pub left_trigger: f32,
    pub right_trigger: f32,
    pub left_stick_in: bool,
    pub right_stick_in: bool,
    pub left_bumper: bool,
    pub right_bumper: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub face_x: bool,
    pub face_y: bool,
    pub face_a: bool,
    pub face_b: bool,
}

impl RawControllerInput {
    /// Updates mapped fields in native order and resolves a bumper conflict.
    ///
    /// Reference: `0x825EE088..0x825EE43C`. `use_sensors` is deliberately
    /// unused by the reference Xbox 360 body. `previous` is the preceding
    /// post-arbitration raw state; its copying belongs to the caller at
    /// `0x825F1C6C..0x825F1C88`. No deadzone or float clamp occurs here.
    pub fn update(
        &mut self,
        input: &mut impl MappedControllerInput,
        previous: &Self,
        use_sensors: bool,
        activity: ControllerInputActivity,
    ) {
        match self.try_update(
            &mut InfallibleMappedInput(input),
            previous,
            use_sensors,
            activity,
        ) {
            Ok(()) => {}
            Err(never) => match never {},
        }
    }

    /// Same source update, retaining completed field writes if a runtime
    /// collaborator refuses. Failure is a host boundary, not a native recovery.
    /// No later control is sampled after refusal; no rollback or zero fallback.
    /// # Errors
    /// Returns the first map-provider failure, preserving prior writes.
    pub fn try_update<I: TryMappedControllerInput>(
        &mut self,
        input: &mut I,
        previous: &Self,
        _use_sensors: bool,
        activity: ControllerInputActivity,
    ) -> Result<(), I::Error> {
        self.left_stick_x = input.try_mapped_value(ControllerControl::LeftStickX)?;
        self.left_stick_y = input.try_mapped_value(ControllerControl::LeftStickY)?;
        self.right_stick_x = input.try_mapped_value(ControllerControl::RightStickX)?;
        self.right_stick_y = input.try_mapped_value(ControllerControl::RightStickY)?;
        self.left_trigger = input.try_mapped_value(ControllerControl::LeftTrigger)?;
        self.right_trigger = input.try_mapped_value(ControllerControl::RightTrigger)?;
        self.left_stick_in = input.try_mapped_value(ControllerControl::LeftStickIn)? != 0.0;
        self.right_stick_in = input.try_mapped_value(ControllerControl::RightStickIn)? != 0.0;
        self.left_bumper = input.try_mapped_value(ControllerControl::LeftBumper)? != 0.0;
        self.right_bumper = input.try_mapped_value(ControllerControl::RightBumper)? != 0.0;

        if self.left_bumper && self.right_bumper {
            if previous.left_bumper != previous.right_bumper
                && (activity.pushing || activity.skitching)
            {
                self.left_bumper = previous.left_bumper;
                self.right_bumper = previous.right_bumper;
            } else {
                self.right_bumper = false;
            }
        }

        self.dpad_up = input.try_mapped_value(ControllerControl::DpadUp)? != 0.0;
        self.dpad_down = input.try_mapped_value(ControllerControl::DpadDown)? != 0.0;
        self.dpad_left = input.try_mapped_value(ControllerControl::DpadLeft)? != 0.0;
        self.dpad_right = input.try_mapped_value(ControllerControl::DpadRight)? != 0.0;
        self.face_x = input.try_mapped_value(ControllerControl::FaceX)? != 0.0;
        self.face_y = input.try_mapped_value(ControllerControl::FaceY)? != 0.0;
        self.face_a = input.try_mapped_value(ControllerControl::FaceA)? != 0.0;
        self.face_b = input.try_mapped_value(ControllerControl::FaceB)? != 0.0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buttons(raw: RawControllerInput) -> [bool; 12] {
        [
            raw.left_stick_in,
            raw.right_stick_in,
            raw.left_bumper,
            raw.right_bumper,
            raw.dpad_up,
            raw.dpad_down,
            raw.dpad_left,
            raw.dpad_right,
            raw.face_x,
            raw.face_y,
            raw.face_a,
            raw.face_b,
        ]
    }

    fn axes(raw: RawControllerInput) -> [f32; 6] {
        [
            raw.left_stick_x,
            raw.left_stick_y,
            raw.right_stick_x,
            raw.right_stick_y,
            raw.left_trigger,
            raw.right_trigger,
        ]
    }

    #[test]
    fn reads_exact_native_mapping_order_once_each() {
        struct Ordered {
            ids: Vec<u8>,
        }
        impl MappedControllerInput for Ordered {
            fn mapped_value(&mut self, control: ControllerControl) -> f32 {
                self.ids.push(control as u8);
                f32::from(control as u8)
            }
        }
        let mut input = Ordered { ids: Vec::new() };
        let mut raw = RawControllerInput::default();
        raw.update(
            &mut input,
            &RawControllerInput::default(),
            false,
            ControllerInputActivity::default(),
        );
        assert_eq!(
            input.ids,
            [
                74, 75, 77, 78, 80, 81, 76, 79, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91
            ]
        );
        assert_eq!(
            axes(raw).map(f32::to_bits),
            [74.0_f32, 75.0, 77.0, 78.0, 80.0, 81.0].map(f32::to_bits)
        );
    }

    #[test]
    fn copies_every_scalar_bit_pattern_without_clamping() {
        let patterns = [
            0,
            0x8000_0000,
            0x7f80_0000,
            0xff80_0000,
            0x7fc1_2345,
            0xffc5_4321,
            1,
            0x8000_0001,
            0x4120_0000,
            0xc120_0000,
        ];
        for offset in 0..patterns.len() {
            let mut values = [0.0; 18];
            let expected: [u32; 6] =
                std::array::from_fn(|axis| patterns[(offset + axis) % patterns.len()]);
            for (index, bits) in [0, 1, 3, 4, 6, 7].into_iter().zip(expected) {
                values[index] = f32::from_bits(bits);
            }
            let mut raw = RawControllerInput::default();
            raw.update(
                &mut MappedControllerSnapshot::new(values),
                &RawControllerInput::default(),
                false,
                ControllerInputActivity::default(),
            );
            assert_eq!(axes(raw).map(f32::to_bits), expected);
        }
    }

    #[test]
    fn each_mapped_button_uses_nonzero_including_nan_and_subnormal() {
        for (button, index) in [2, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
            .into_iter()
            .enumerate()
        {
            for (bits, expected) in [
                (0, false),
                (0x8000_0000, false),
                (1, true),
                (0x8000_0001, true),
                (0x3e80_0000, true),
                (0xbf80_0000, true),
                (0x7f80_0000, true),
                (0xff80_0000, true),
                (0x7fc1_2345, true),
                (0xffc1_2345, true),
            ] {
                let mut values = [0.0; 18];
                values[index] = f32::from_bits(bits);
                let mut raw = RawControllerInput::default();
                raw.update(
                    &mut MappedControllerSnapshot::new(values),
                    &RawControllerInput::default(),
                    false,
                    ControllerInputActivity::default(),
                );
                let mut expected_buttons = [false; 12];
                expected_buttons[button] = expected;
                assert_eq!(buttons(raw), expected_buttons, "map slot {index}, {bits:x}");
            }
        }
    }

    #[test]
    fn exhaustive_bumper_arbitration_and_sensor_inertness() {
        // Rows are previous [none, left, right, both]. Columns distinguish
        // inactive from pushing/skitching. The table comes from PPC branches.
        let conflict_expected = [
            [(true, false), (true, false)],
            [(true, false), (true, false)],
            [(true, false), (false, true)],
            [(true, false), (true, false)],
        ];
        for (previous_bits, conflict_row) in conflict_expected.iter().enumerate() {
            let previous = RawControllerInput {
                left_bumper: previous_bits & 1 != 0,
                right_bumper: previous_bits & 2 != 0,
                ..RawControllerInput::default()
            };
            for current_bits in 0..4 {
                for activity_bits in 0..4 {
                    let activity = ControllerInputActivity {
                        pushing: activity_bits & 1 != 0,
                        skitching: activity_bits & 2 != 0,
                    };
                    let mut values = [0.0; 18];
                    values[8] = f32::from(u8::from(current_bits & 1 != 0));
                    values[9] = f32::from(u8::from(current_bits & 2 != 0));
                    let expected = if current_bits == 3 {
                        conflict_row[usize::from(activity_bits != 0)]
                    } else {
                        (current_bits & 1 != 0, current_bits & 2 != 0)
                    };
                    let mut outcomes = Vec::new();
                    for use_sensors in [false, true] {
                        let mut raw = RawControllerInput::default();
                        raw.update(
                            &mut MappedControllerSnapshot::new(values),
                            &previous,
                            use_sensors,
                            activity,
                        );
                        assert_eq!((raw.left_bumper, raw.right_bumper), expected);
                        outcomes.push(raw);
                    }
                    assert_eq!(outcomes[0], outcomes[1]);
                }
            }
        }
    }

    #[test]
    fn next_sample_uses_previous_arbitrated_bumper_and_replaces_other_fields() {
        let activity = ControllerInputActivity {
            pushing: true,
            skitching: false,
        };
        let mut raw = RawControllerInput::default();
        let mut first = [0.0; 18];
        first[9] = 1.0;
        first[0] = -2.0;
        first[16] = 1.0;
        raw.update(
            &mut MappedControllerSnapshot::new(first),
            &RawControllerInput::default(),
            false,
            activity,
        );
        let previous = raw;
        let mut both = [0.0; 18];
        both[8] = 1.0;
        both[9] = 1.0;
        raw.update(
            &mut MappedControllerSnapshot::new(both),
            &previous,
            false,
            activity,
        );
        assert!(!raw.left_bumper && raw.right_bumper);
        assert!(!raw.face_a);
        assert_eq!(raw.left_stick_x.to_bits(), 0);
        let previous = raw;
        raw.update(
            &mut MappedControllerSnapshot::new(both),
            &previous,
            false,
            ControllerInputActivity::default(),
        );
        assert!(raw.left_bumper && !raw.right_bumper);
    }
    #[test]
    fn runtime_refusal_retains_prefix_and_stops_before_later_controls() {
        struct Refusing {
            seen: Vec<ControllerControl>,
        }
        impl TryMappedControllerInput for Refusing {
            type Error = ControllerControl;
            fn try_mapped_value(&mut self, control: ControllerControl) -> Result<f32, Self::Error> {
                self.seen.push(control);
                if control == ControllerControl::DpadDown {
                    Err(control)
                } else {
                    Ok(1.0)
                }
            }
        }
        let mut input = Refusing { seen: Vec::new() };
        let mut raw = RawControllerInput {
            dpad_left: true,
            ..RawControllerInput::default()
        };
        assert_eq!(
            raw.try_update(
                &mut input,
                &RawControllerInput::default(),
                false,
                ControllerInputActivity::default()
            ),
            Err(ControllerControl::DpadDown)
        );
        assert!(
            raw.left_bumper && !raw.right_bumper,
            "arbitration precedes the failed read"
        );
        assert!(raw.dpad_up && !raw.dpad_down && raw.dpad_left);
        assert!(!raw.face_a);
        assert_eq!(
            input.seen.iter().map(|c| *c as u8).collect::<Vec<_>>(),
            [74, 75, 77, 78, 80, 81, 76, 79, 82, 83, 84, 85]
        );
        assert_eq!(axes(raw).map(f32::to_bits), [1.0_f32.to_bits(); 6]);
    }
}
