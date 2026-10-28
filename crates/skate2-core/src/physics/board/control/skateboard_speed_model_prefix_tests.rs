use super::{SkateboardEntryState, SpeedModelPrefixSource};
struct Input {
    flags: u32,
    extra: u32,
    wheels: i32,
    speed: f32,
    limit: f32,
    fail_limit: bool,
    events: Vec<&'static str>,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            flags: 0,
            extra: 0,
            wheels: 4,
            speed: 5.0,
            limit: 2.0,
            fail_limit: false,
            events: Vec::new(),
        }
    }
}
impl SpeedModelPrefixSource for Input {
    type Error = &'static str;
    type Surface = ();
    fn flags_8c4(&mut self) -> Result<u32, Self::Error> {
        self.events.push("flags");
        Ok(self.flags)
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.events.push("extra");
        Ok(self.extra)
    }
    fn wheel_count(&mut self) -> Result<i32, Self::Error> {
        self.events.push("wheels");
        Ok(self.wheels)
    }
    fn capture_surface(&mut self) -> Result<(), Self::Error> {
        self.events.push("surface");
        Ok(())
    }
    fn forward_speed(&mut self) -> Result<f32, Self::Error> {
        self.events.push("speed");
        Ok(self.speed)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(0.25)
    }
    fn maximum_speed_difference(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.events.push("layout");
        if self.fail_limit {
            Err("layout")
        } else {
            Ok(self.limit)
        }
    }
}
#[test]
fn inhibited_still_reads_wheels_and_updates_real_collision_target() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = 99.0;
    state.reset_target_after_collision = true;
    let mut input = Input {
        flags: 0x0200_0000,
        speed: -5.0,
        ..Input::default()
    };
    let result = state.try_speed_model_prefix(&mut input).unwrap();
    assert!(!result.should_apply_force);
    assert_eq!(
        input.events,
        ["flags", "wheels", "surface", "speed", "dt", "layout"]
    );
    assert_eq!(state.target_speed.to_bits(), 5.0_f32.to_bits());
    assert!(!state.reset_target_after_collision);
}
#[test]
fn ordinary_local_clamp_does_not_publish_final_target() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = 99.0;
    let mut input = Input::default();
    let result = state.try_speed_model_prefix(&mut input).unwrap();
    assert!(result.should_apply_force);
    assert_eq!(result.starting_target.to_bits(), 7.0_f32.to_bits());
    assert_eq!(state.target_speed.to_bits(), 99.0_f32.to_bits());
    assert_eq!(
        input.events,
        [
            "flags", "extra", "wheels", "surface", "speed", "dt", "layout"
        ]
    );
}
#[test]
fn missing_surface_value_preserves_both_source_stores() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = 99.0;
    state.reset_target_after_collision = true;
    let mut input = Input {
        fail_limit: true,
        ..Input::default()
    };
    assert!(state.try_speed_model_prefix(&mut input).is_err());
    assert_eq!(state.target_speed.to_bits(), 99.0_f32.to_bits());
    assert!(state.reset_target_after_collision);
}
#[test]
fn negative_and_nan_limits_preserve_fsel_order() {
    let mut state = SkateboardEntryState::new();
    state.target_speed = 4.0;
    let mut input = Input {
        limit: -2.0,
        ..Input::default()
    };
    assert_eq!(
        state
            .try_speed_model_prefix(&mut input)
            .unwrap()
            .starting_target
            .to_bits(),
        3.0_f32.to_bits()
    );
    input.limit = f32::NAN;
    assert!(
        state
            .try_speed_model_prefix(&mut input)
            .unwrap()
            .starting_target
            .is_nan()
    );
}
