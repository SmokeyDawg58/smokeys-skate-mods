use super::{SkateboardSpeedWobble, WobbleAttributes, WobbleInput, WobbleServices};
use crate::attribute_runtime::tests::graph;
use std::sync::Arc;

#[derive(Default)]
struct Attributes {
    events: Vec<(char, usize)>,
    count: usize,
}
impl WobbleAttributes for Attributes {
    type Error = ();
    type Scalar = usize;
    fn capture(&mut self, _: u64) -> Result<usize, ()> {
        let index = self.count;
        self.count += 1;
        self.events.push(('c', index));
        Ok(index)
    }
    fn load(&mut self, index: &usize) -> Result<f32, ()> {
        self.events.push(('l', *index));
        // COM range 0..1, threshold10, all active multipliers1.
        Ok([0.0, 1.0, 0.0, 0.0, 10.0, 0.0, 1.0, 1.0, 1.0, 2.0][*index])
    }
    fn graph(&mut self, offset: usize, _: f32) -> Result<f32, ()> {
        self.events.push(('g', offset));
        Ok(1.0)
    }
}
#[derive(Default)]
struct Services {
    events: Vec<&'static str>,
    fail_overlay: bool,
}
impl WobbleServices for Services {
    type Error = &'static str;
    fn sine(&mut self, _: f32) -> Result<f32, Self::Error> {
        self.events.push("sine");
        Ok(0.25)
    }
    fn draw_enabled(&mut self) -> Result<bool, Self::Error> {
        self.events.push("draw");
        Ok(self.fail_overlay)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.events.push("overlay");
        Err("missing overlay")
    }
}
fn owner() -> SkateboardSpeedWobble {
    SkateboardSpeedWobble::new(&Arc::new(graph(super::super::SPEED_WOBBLE_CLASS, vec![])))
}
fn input(speed: f32) -> WobbleInput {
    WobbleInput {
        tilt: 4.0,
        speed,
        com_to_deck_length: 0.5,
        truck_tightness: 0.0,
        dt: 0.125,
    }
}
#[test]
fn inactive_nan_preserves_owner_and_skips_active_services_after_ordered_captures() {
    let mut owner = owner();
    let mut attributes = Attributes::default();
    let mut services = Services::default();
    assert_eq!(
        owner
            .try_calculate(input(f32::NAN), &mut attributes, &mut services)
            .unwrap()
            .to_bits(),
        4.0f32.to_bits()
    );
    assert!(services.events.is_empty());
    assert_eq!(
        attributes.events,
        [
            ('c', 0),
            ('c', 1),
            ('c', 2),
            ('l', 2),
            ('l', 0),
            ('l', 1),
            ('c', 3),
            ('c', 4),
            ('c', 5),
            ('l', 3),
            ('l', 5),
            ('l', 4)
        ]
    );
    assert_eq!(owner.state_bits(), (false, [0; 6], None));
}
#[test]
fn activation_stores_offset_not_combined_and_keeps_amplitude_unknown() {
    let mut owner = owner();
    let mut attributes = Attributes::default();
    let mut services = Services::default();
    let returned = owner
        .try_calculate(input(11.0), &mut attributes, &mut services)
        .unwrap();
    assert_eq!(returned.to_bits(), 2.25f32.to_bits());
    assert_eq!(owner.wobble_tilt().to_bits(), 0.5f32.to_bits());
    assert_eq!(owner.time.to_bits(), 0.125f32.to_bits());
    assert_eq!(owner.frequency.to_bits(), 0);
    assert!(owner.amplitude.is_none());
    assert_eq!(services.events, ["sine", "draw"]);
    assert_eq!(
        &attributes.events[17..22],
        &[('g', 0x40), ('g', 0), ('l', 8), ('g', 0xc0), ('g', 0x80)]
    );
}
#[test]
fn missing_overlay_retains_angle_effect_and_offset_before_time_increment() {
    let mut owner = owner();
    let mut services = Services {
        fail_overlay: true,
        ..Services::default()
    };
    assert!(
        owner
            .try_calculate(input(11.0), &mut Attributes::default(), &mut services)
            .is_err()
    );
    assert!(owner.active);
    assert_eq!(owner.time.to_bits(), 0);
    assert_eq!(owner.effect.to_bits(), 1.0f32.to_bits());
    assert_eq!(owner.wobble_tilt.to_bits(), 0.5f32.to_bits());
    assert_ne!(owner.angle_position.to_bits(), 0);
    assert_eq!(services.events, ["sine", "draw", "overlay"]);
}
#[test]
fn active_drop_below_threshold_returns_computed_value_after_reset() {
    let mut owner = owner();
    owner.seed_reset_fields_for_test(3.0, Some(7.0));
    let returned = owner
        .try_calculate(
            input(9.0),
            &mut Attributes::default(),
            &mut Services::default(),
        )
        .unwrap();
    assert_eq!(returned.to_bits(), 2.25f32.to_bits());
    assert_eq!(owner.state_bits(), (false, [0; 6], Some(7.0f32.to_bits())));
}
