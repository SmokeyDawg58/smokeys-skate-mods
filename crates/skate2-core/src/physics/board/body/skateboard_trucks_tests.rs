//! Synthetic scripted reads exercise the native lookup/load chronology.
//! These changing values are test discriminators, not a mutable Vault service
//! or original runtime mutation trace. Mounted truck tuning is a separate test.

#![expect(
    clippy::float_cmp,
    reason = "exact source-value assertions; exceptional payload checks explicitly compare bits"
)]

use super::*;
use crate::skateboard_wheels::SkateboardWheelInertiaAxis;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Lookup(Class, u64),
    Float(usize),
    Byte(usize),
    Integer(usize),
    Estimate(SkateboardWheelInertiaAxis),
}

const LOOKUPS: [(Class, u64); 15] = [
    (Class::Trucks, RADIUS_MULTIPLIER),
    (Class::Trucks, WIDTH_MULTIPLIER),
    (Class::Wheels, WHEEL_RADIUS),
    (Class::Wheels, WHEEL_HALF_WIDTH),
    (Class::Trucks, COLLISION_ENABLED),
    (Class::Trucks, DYNAMIC_FRICTION),
    (Class::Trucks, STATIC_FRICTION),
    (Class::Trucks, RESTITUTION),
    (Class::Trucks, DYNAMIC_FRICTION),
    (Class::Trucks, STATIC_FRICTION),
    (Class::Trucks, RESTITUTION),
    (Class::Trucks, MASS),
    (Class::World, WORLD_MASS_MULTIPLIER),
    (Class::Trucks, MASS),
    (Class::World, WORLD_MASS_MULTIPLIER),
];

struct Script {
    words: [u32; 15],
    next_lookup: usize,
    /// A later lookup may change an earlier pointer's contents. This catches
    /// replacing a retained pointer by an early scalar load.
    mutations: Vec<(usize, usize, u32)>,
    events: Rc<RefCell<Vec<Event>>>,
    fail_float: Option<usize>,
}

impl Script {
    fn new() -> Self {
        Self {
            words: [
                2.0_f32, 3.0, 4.0, 5.0, 0.0, 0.7, 0.5, 0.25, 1.1, 0.9, 0.5, 2.0, 3.0, 4.0, 5.0,
            ]
            .map(f32::to_bits),
            next_lookup: 0,
            mutations: vec![],
            events: Rc::new(RefCell::new(vec![])),
            fail_float: None,
        }
    }

    fn numeric(&self) -> Numeric {
        Numeric(Rc::clone(&self.events))
    }

    fn finish(&self) {
        assert_eq!(self.next_lookup, LOOKUPS.len());
    }
}

impl SkateboardAttributeRead for Script {
    type Pointer = usize;
    type Error = &'static str;

    fn lookup(&mut self, class: Class, key: u64) -> Result<Self::Pointer, Self::Error> {
        let ordinal = self.next_lookup;
        assert_eq!(
            LOOKUPS.get(ordinal),
            Some(&(class, key)),
            "lookup ordinal{ordinal}"
        );
        self.events.borrow_mut().push(Event::Lookup(class, key));
        self.next_lookup += 1;
        for &(when, target, word) in &self.mutations {
            if when == ordinal {
                assert!(
                    target < ordinal,
                    "mutation names an already returned pointer"
                );
                self.words[target] = word;
            }
        }
        Ok(ordinal)
    }

    fn float(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error> {
        assert!(*pointer < self.next_lookup);
        self.events.borrow_mut().push(Event::Float(*pointer));
        if self.fail_float == Some(*pointer) {
            return Err("injected float load");
        }
        Ok(f32::from_bits(self.words[*pointer]))
    }

    fn byte(&mut self, pointer: &Self::Pointer) -> Result<u8, Self::Error> {
        assert!(*pointer < self.next_lookup);
        self.events.borrow_mut().push(Event::Byte(*pointer));
        Ok(self.words[*pointer].to_be_bytes()[0])
    }

    fn integer(&mut self, pointer: &Self::Pointer) -> Result<i32, Self::Error> {
        self.events.borrow_mut().push(Event::Integer(*pointer));
        Ok(i32::from_be_bytes(self.words[*pointer].to_be_bytes()))
    }
}

struct Numeric(Rc<RefCell<Vec<Event>>>);

impl SkateboardWheelMassNumeric for Numeric {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, denominator: f32) -> f32 {
        self.0.borrow_mut().push(Event::Estimate(axis));
        denominator.recip()
    }
}

fn create(script: &mut Script, initial: u32) -> SkateboardTruckDefinitions {
    let mut numeric = script.numeric();
    SkateboardTruckDefinitions::try_create(script, initial, &mut numeric).unwrap()
}

#[test]
fn fifteen_lookups_preserve_delayed_material_mass_loads_and_two_mass_call_positions() {
    let mut script = Script::new();
    create(&mut script, 4);
    script.finish();
    let mut expected = Vec::new();
    for (ordinal, &(class, key)) in LOOKUPS.iter().take(4).enumerate() {
        expected.push(Event::Lookup(class, key));
        expected.push(Event::Float(ordinal));
    }
    expected.extend([
        Event::Lookup(Class::Trucks, COLLISION_ENABLED),
        Event::Byte(4),
    ]);
    for first in [5, 8] {
        expected.extend([
            Event::Lookup(Class::Trucks, DYNAMIC_FRICTION),
            Event::Lookup(Class::Trucks, STATIC_FRICTION),
            Event::Float(first),
            Event::Float(first + 1),
            Event::Lookup(Class::Trucks, RESTITUTION),
            Event::Float(first + 2),
        ]);
    }
    for first in [11, 13] {
        expected.extend([
            Event::Lookup(Class::Trucks, MASS),
            Event::Lookup(Class::World, WORLD_MASS_MULTIPLIER),
            Event::Float(first + 1),
            Event::Float(first),
            Event::Estimate(SkateboardWheelInertiaAxis::Y),
            Event::Estimate(SkateboardWheelInertiaAxis::X),
            Event::Estimate(SkateboardWheelInertiaAxis::Z),
        ]);
    }
    assert_eq!(*script.events.borrow(), expected);
}

#[test]
fn retained_pointers_observe_later_lookup_changes_and_each_part_resamples() {
    let mut script = Script::new();
    script.mutations = vec![
        (6, 5, 0.8_f32.to_bits()),
        (9, 8, 1.2_f32.to_bits()),
        (12, 11, 4.0_f32.to_bits()),
        (14, 13, 7.0_f32.to_bits()),
    ];
    let definitions = create(&mut script, 4);
    script.finish();
    let [first, second] = definitions.parts();
    assert_eq!(
        first.material().dynamic_friction.to_bits(),
        0.8_f32.to_bits()
    );
    assert_eq!(
        second.material().dynamic_friction.to_bits(),
        1.2_f32.to_bits()
    );
    assert_eq!(first.material().static_friction, 0.5);
    assert_eq!(second.material().static_friction, 0.9);
    assert_eq!(first.material().restitution, 0.25);
    assert_eq!(second.material().restitution, 0.5);
    assert_eq!(
        first.inertia().inverse_mass.to_bits(),
        (1.0_f32 / 12.0).to_bits()
    );
    assert_eq!(
        second.inertia().inverse_mass.to_bits(),
        (1.0_f32 / 35.0).to_bits()
    );
    assert_ne!(
        first.inertia().inverse_tensor.map(f32::to_bits),
        second.inertia().inverse_tensor.map(f32::to_bits)
    );
}

#[test]
fn enabled_uses_first_byte_not_integer_or_float_truthiness() {
    for (word, enabled) in [
        (0x0000_0001, false),
        (0x0100_0000, true),
        (0xff00_0000, true),
        (0, false),
    ] {
        let mut script = Script::new();
        script.words[4] = word;
        let definitions = create(&mut script, 0);
        assert_eq!(definitions.parts()[0].volume().flags, u32::from(enabled));
        assert_eq!(
            script
                .events
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Byte(4)))
                .count(),
            1
        );
        assert!(
            !script
                .events
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::Integer(_) | Event::Float(4)))
        );
    }
}

#[test]
fn truck_materials_do_not_apply_wheel_friction_clamp_or_sanitize_bits() {
    let mut script = Script::new();
    script.words[5] = 1.0_f32.to_bits();
    script.words[6] = 0x8000_0000;
    script.words[7] = 0x8000_0000;
    script.words[8] = 0x7fc1_2345;
    script.words[9] = 0x7fc5_4321;
    script.words[10] = 0x7fc6_5432;
    let definitions = create(&mut script, 0);
    let first = definitions.parts()[0].material();
    let second = definitions.parts()[1].material();
    assert_eq!(
        [
            first.dynamic_friction.to_bits(),
            first.static_friction.to_bits(),
            first.restitution.to_bits()
        ],
        [0x3f80_0000, 0x8000_0000, 0x8000_0000]
    );
    assert_eq!(
        [
            second.dynamic_friction.to_bits(),
            second.static_friction.to_bits(),
            second.restitution.to_bits()
        ],
        [0x7fc1_2345, 0x7fc5_4321, 0x7fc6_5432]
    );
}

#[test]
fn both_truck_parts_share_fresh_group_zero_capsule_and_exact_default_controls() {
    let mut script = Script::new();
    script.words[4] = 0x0100_0000;
    let definitions = create(&mut script, 4);
    let [first, second] = definitions.parts();
    assert_eq!([first.ordinal(), second.ordinal()], [4, 5]);
    assert!(Arc::ptr_eq(first.volume(), second.volume()));
    assert_eq!(first.volume().volume_type(), Some(VolumeType::Capsule));
    assert_eq!(first.volume().radius, 4.0);
    assert_eq!(first.volume().shape_words[0], 7.5_f32.to_bits());
    assert_eq!(
        (
            first.volume().group_id,
            first.volume().surface_id,
            first.volume().flags
        ),
        (0, 0, 1)
    );
    assert_eq!(
        first.volume().transform.map(f32::to_bits),
        FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
    );
    for part in definitions.parts() {
        assert_eq!(
            part.inertia().max_omega.to_bits(),
            0x7f7f_ffff,
            "truck does not get wheel-specific maxOmega"
        );
        assert_eq!(part.inertia().max_velocity.to_bits(), 0x7f7f_ffff);
        assert_eq!(part.inertia().linear_drag.to_bits(), 0);
        assert_eq!(part.inertia().angular_drag.to_bits(), 0);
        assert_eq!(
            part.inverse_tensor_w().to_bits(),
            part.inertia().inverse_tensor[0].to_bits()
        );
        assert!(part.inverse_body_transform().is_none());
        assert!(!part.is_static_body());
    }
}

#[test]
fn radius_and_half_height_multiply_before_half_without_reassociation() {
    for radius in [true, false] {
        let mut script = Script::new();
        if radius {
            script.words[0] = 2.0_f32.to_bits();
            script.words[2] = f32::MAX.to_bits();
        } else {
            script.words[1] = f32::MAX.to_bits();
            script.words[3] = 2.0_f32.to_bits();
        }
        let definitions = create(&mut script, 0);
        let volume = definitions.parts()[0].volume();
        let value = if radius {
            volume.radius
        } else {
            f32::from_bits(volume.shape_words[0])
        };
        assert_eq!(
            value,
            f32::INFINITY,
            "source first product overflows before multiplication by half"
        );
        assert!(
            ((f32::MAX * 0.5) * 2.0).is_finite(),
            "reassociated alternative differs"
        );
        if radius {
            assert_eq!(f32::from_bits(volume.shape_words[0]), 7.5);
        } else {
            assert_eq!(volume.radius, 4.0);
        }
    }
}

#[test]
fn debug_capsules_retain_white_identity_and_uninitialized_triangle_points() {
    let definitions = create(&mut Script::new(), 4);
    assert_eq!(SkateboardTruckDebugCapsule::NATIVE_TYPE, 2);
    assert_eq!(definitions.debug_end(), 6);
    for (part, index) in definitions.parts().iter().zip([4, 5]) {
        let debug = part.debug();
        assert_eq!(debug.index, index);
        assert_eq!(
            debug.offset.map(f32::to_bits),
            FRESH_VOLUME_TRANSFORM.map(f32::to_bits)
        );
        assert_eq!(debug.offset[15].to_bits(), 0);
        assert_eq!(debug.color.map(f32::to_bits), [0x3f80_0000; 4]);
        assert_eq!(debug.size.map(f32::to_bits), [0; 4]);
        assert_eq!(debug.radius, 4.0);
        assert_eq!(debug.half_length, 7.5);
        assert_eq!(debug.triangle_points, None);
    }
}

#[test]
fn debug_capacity_accepts_thirty_and_refuses_thirty_one_before_any_callback() {
    let mut script = Script::new();
    let definitions = create(&mut script, 30);
    script.finish();
    assert_eq!(definitions.debug_end(), 32);
    assert_eq!(
        [
            definitions.parts()[0].debug().index,
            definitions.parts()[1].debug().index
        ],
        [30, 31]
    );
    for initial in [31, u32::MAX] {
        let mut script = Script::new();
        let mut numeric = script.numeric();
        assert!(
            matches!(SkateboardTruckDefinitions::try_create(&mut script, initial, &mut numeric), Err(SkateboardTruckDefinitionError::DebugCapacity { initial_count }) if initial_count == initial)
        );
        assert_eq!(script.next_lookup, 0);
        assert!(script.events.borrow().is_empty());
    }
}

#[test]
fn delayed_float_error_is_not_hidden_by_an_earlier_cached_read() {
    let mut script = Script::new();
    script.fail_float = Some(5);
    let mut numeric = script.numeric();
    assert!(matches!(
        SkateboardTruckDefinitions::try_create(&mut script, 4, &mut numeric),
        Err(SkateboardTruckDefinitionError::Attribute(
            "injected float load"
        ))
    ));
    assert_eq!(
        script.next_lookup, 7,
        "both friction pointers obtained before dynamic load fails"
    );
    assert!(matches!(
        script.events.borrow().last(),
        Some(Event::Float(5))
    ));
    assert!(
        !script
            .events
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Estimate(_)))
    );
}
