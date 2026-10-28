#![expect(
    clippy::float_cmp,
    reason = "exact copied/source fields; exceptional values use explicit bit comparisons"
)]

use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Lookup(SkateboardAttributeClass, u64),
    Load(usize),
    SinCos(u32),
    Write(usize),
}
type Trace = Rc<RefCell<Vec<Event>>>;

#[test]
fn writer_failures_preserve_exact_completed_prefix_and_stop_later_reads() {
    let order = [6, 0, 1, 2, 3, 4, 5];
    for fail_write in 0..7 {
        let trace = Trace::default();
        let mut attributes = Attributes::source_fixture(trace.clone());
        let mut numeric = Numeric {
            values: Ok((0.25, 0.75)),
            log: trace.clone(),
        };
        let mut completed = Vec::new();
        let result = try_initialize_skateboard_transforms_with_writer(
            &mut attributes,
            &mut numeric,
            |part, matrix| {
                trace.borrow_mut().push(Event::Write(part));
                if completed.len() == fail_write {
                    return Err("write refused");
                }
                completed.push((part, matrix));
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(SkateboardInitialTransformError::Write("write refused"))
        ));
        assert_eq!(
            completed.iter().map(|(part, _)| *part).collect::<Vec<_>>(),
            order[..fail_write]
        );
        assert_eq!(attributes.looked_up, [0, 6, 8, 10, 12, 13, 14][fail_write]);
        assert_eq!(
            trace.borrow().last(),
            Some(&Event::Write(order[fail_write]))
        );
    }
}

#[test]
fn writer_keeps_actual_prior_part_writes_on_each_attribute_and_numeric_failure() {
    for fail_lookup in 0..14 {
        let trace = Trace::default();
        let mut attributes = Attributes::source_fixture(trace.clone());
        attributes.fail_at = Some(fail_lookup);
        let mut numeric = Numeric {
            values: Ok((0.25, 0.75)),
            log: trace,
        };
        let mut completed = Vec::new();
        let result = try_initialize_skateboard_transforms_with_writer(
            &mut attributes,
            &mut numeric,
            |part, matrix| {
                completed.push((part, matrix));
                Ok::<_, core::convert::Infallible>(())
            },
        );
        assert!(matches!(
            result,
            Err(SkateboardInitialTransformError::Attribute(_))
        ));
        let count = 1 + [6, 8, 10, 12, 13, 14]
            .iter()
            .filter(|&&n| n <= fail_lookup)
            .count();
        assert_eq!(
            completed.iter().map(|(part, _)| *part).collect::<Vec<_>>(),
            [6, 0, 1, 2, 3, 4, 5][..count]
        );
        assert_eq!(completed[0].1, FRESH_VOLUME_TRANSFORM);
    }
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    let mut numeric = Numeric {
        values: Err("trig unavailable"),
        log: trace,
    };
    let mut completed = Vec::new();
    let result = try_initialize_skateboard_transforms_with_writer(
        &mut attributes,
        &mut numeric,
        |part, _| {
            completed.push(part);
            Ok::<_, core::convert::Infallible>(())
        },
    );
    assert!(matches!(
        result,
        Err(SkateboardInitialTransformError::Numeric("trig unavailable"))
    ));
    assert_eq!(completed, [6, 0, 1, 2, 3, 4]);
    assert_eq!(attributes.looked_up, 13);
}

struct Attributes {
    words: [u32; 14],
    looked_up: usize,
    log: Trace,
    mutations: Vec<(usize, usize, u32)>,
    fail_at: Option<usize>,
}

impl Attributes {
    fn source_fixture(log: Trace) -> Self {
        // Original scalar fixture, not an asset-provenance oracle. Root owns
        // the independent mounted runtime-Vault/assembly admission test.
        Self {
            words: [
                0x3f17_0a3d,
                0xbd54_fdf4,
                0xbd54_fdf4,
                0x3f17_0a3d,
                0xbd67_6c8b,
                0x3dc2_8f5c,
                0xbd67_6c8b,
                0x3dc2_8f5c,
                0xbd67_6c8b,
                0x3dc2_8f5c,
                0xbd67_6c8b,
                0x3dc2_8f5c,
                0xbd67_6c8b,
                0xbd67_6c8b,
            ],
            looked_up: 0,
            log,
            mutations: Vec::new(),
            fail_at: None,
        }
    }
}

impl SkateboardAttributeRead for Attributes {
    type Pointer = usize;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<usize, Self::Error> {
        self.log.borrow_mut().push(Event::Lookup(class, key));
        if self.fail_at == Some(self.looked_up) {
            return Err("attribute unavailable");
        }
        for &(trigger, target, word) in &self.mutations {
            if trigger == self.looked_up {
                self.words[target] = word;
            }
        }
        let pointer = self.looked_up;
        self.looked_up += 1;
        Ok(pointer)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Load(*pointer));
        Ok(f32::from_bits(self.words[*pointer]))
    }
    fn byte(&mut self, _: &usize) -> Result<u8, Self::Error> {
        panic!("initial placement uses only scalar floats")
    }
    fn integer(&mut self, _: &usize) -> Result<i32, Self::Error> {
        panic!("initial placement uses only scalar floats")
    }
}

struct Numeric {
    values: Result<(f32, f32), &'static str>,
    log: Trace,
}
impl SkateboardInitialTransformNumeric for Numeric {
    type Error = &'static str;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        self.log.borrow_mut().push(Event::SinCos(angle.to_bits()));
        self.values
    }
}

#[test]
fn fourteen_lookups_and_delayed_loads_follow_live_source_order() {
    use SkateboardAttributeClass::{Deck, Trucks, Wheels};
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    let mut numeric = Numeric {
        values: Ok((0.25, 0.75)),
        log: trace.clone(),
    };
    let result = try_initialize_skateboard_transforms(&mut attributes, &mut numeric).unwrap();
    let mut expected = vec![
        Event::Lookup(Deck, DECK_LENGTH),
        Event::Lookup(Trucks, FRONT_OFFSET),
        Event::Load(0),
        Event::Load(1),
        Event::Lookup(Trucks, REAR_OFFSET),
        Event::Lookup(Deck, DECK_LENGTH),
        Event::Load(2),
        Event::Load(3),
    ];
    for ordinal in 0..4 {
        expected.extend([
            Event::Lookup(Trucks, VERTICAL_OFFSET),
            Event::Lookup(Wheels, LATERAL_OFFSET),
            Event::Load(5 + ordinal * 2),
            Event::Load(4 + ordinal * 2),
        ]);
    }
    expected.extend([
        Event::Lookup(Trucks, VERTICAL_OFFSET),
        Event::Load(12),
        Event::SinCos(0x3fc9_0fdb),
        Event::Lookup(Trucks, VERTICAL_OFFSET),
        Event::Load(13),
    ]);
    assert_eq!(*trace.borrow(), expected);
    assert_eq!(
        result.ordered_updates().map(|(index, _)| index),
        [6, 0, 1, 2, 3, 4, 5]
    );
    assert_eq!(result.ordered_updates()[0], (6, FRESH_VOLUME_TRANSFORM));
}

#[test]
fn initial_part_order_and_raw_rear_truck_rotation_are_not_normalized() {
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    let mut numeric = Numeric {
        values: Ok((0.25, 0.75)),
        log: trace,
    };
    let result = try_initialize_skateboard_transforms(&mut attributes, &mut numeric).unwrap();
    let parts = result.by_part();
    for &(ordinal, transform) in result.ordered_updates() {
        assert_eq!(parts[ordinal], transform);
    }
    assert_eq!(parts[6], FRESH_VOLUME_TRANSFORM);
    for transform in &parts[..5] {
        assert_eq!(transform[..12], FRESH_VOLUME_TRANSFORM[..12]);
    }
    assert_eq!(
        parts[5][..12],
        [
            0.75, 0.0, -0.25, 0.75, 0.0, 1.0, 0.0, 0.0, 0.25, 0.0, 0.75, 0.25
        ]
    );
    assert!(parts.iter().all(|transform| transform[15].to_bits() == 0));
    let front = f32::from_bits(0x3f17_0a3d).mul_add(0.5, f32::from_bits(0xbd54_fdf4));
    let rear = -front;
    assert_eq!(
        parts.map(|transform| transform[14]),
        [front, front, rear, rear, front, rear, 0.0]
    );
    let lateral = f32::from_bits(0x3dc2_8f5c);
    assert_eq!(
        parts.map(|transform| transform[12]),
        [-lateral, lateral, -lateral, lateral, 0.0, 0.0, 0.0]
    );
}

#[test]
fn retained_pointers_observe_mutation_and_each_part_samples_independently() {
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    attributes.words[1] = 0.5_f32.to_bits();
    attributes.words[3] = 4.0_f32.to_bits();
    attributes.mutations = vec![(1, 0, 2.0_f32.to_bits()), (3, 2, 0.25_f32.to_bits())];
    for (wheel, value) in [1.0_f32, 2.0, 3.0, 4.0].into_iter().enumerate() {
        attributes.words[5 + wheel * 2] = (value * 0.1).to_bits();
        attributes
            .mutations
            .push((5 + wheel * 2, 4 + wheel * 2, value.to_bits()));
    }
    attributes.words[12] = 5.0_f32.to_bits();
    attributes.words[13] = 6.0_f32.to_bits();
    let result = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut Numeric {
            values: Ok((1.0, 0.0)),
            log: trace,
        },
    )
    .unwrap();
    let parts = result.by_part();
    assert_eq!(
        parts.map(|transform| transform[13]),
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.0]
    );
    assert_eq!(
        parts.map(|transform| transform[14]),
        [1.5, 1.5, -2.25, -2.25, 1.5, -2.25, 0.0]
    );
    assert_eq!(parts[0][12], -0.1);
    assert_eq!(parts[1][12], 0.2);
    assert_eq!(parts[2][12], -(3.0 * 0.1));
    assert_eq!(parts[3][12], 0.4);
}

#[test]
fn host_fused_longitudinal_graph_is_not_split_into_multiply_add() {
    // Multiplication by 0.5 is exact for ordinary normals; this host-only
    // subnormal discriminator proves the code graph, not Xenon denormal/FPSCR.
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    attributes.words[0] = 3;
    attributes.words[3] = 3;
    attributes.words[1] = 0x8000_0001;
    attributes.words[2] = 0x8000_0001;
    let parts = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut Numeric {
            values: Ok((1.0, 0.0)),
            log: trace,
        },
    )
    .unwrap()
    .by_part();
    assert_eq!(parts[0][14].to_bits(), 0);
    assert_eq!(parts[2][14].to_bits(), 0x8000_0000);
    let split = f32::from_bits(3) * 0.5 + f32::from_bits(0x8000_0001);
    assert_eq!(split.to_bits(), 1);
}

#[test]
fn signed_zero_and_injected_nan_trig_bits_are_retained() {
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    attributes.words.fill(0);
    let nan = f32::from_bits(0x7fc1_2345);
    let parts = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut Numeric {
            values: Ok((-0.0, nan)),
            log: trace,
        },
    )
    .unwrap()
    .by_part();
    assert_eq!(parts[0][12].to_bits(), 0x8000_0000);
    assert_eq!(parts[1][12].to_bits(), 0);
    assert_eq!(parts[2][14].to_bits(), 0x8000_0000);
    assert_eq!(parts[5][0].to_bits(), nan.to_bits());
    assert_eq!(parts[5][3].to_bits(), nan.to_bits());
    assert_eq!(parts[5][10].to_bits(), nan.to_bits());
    assert_eq!(parts[5][2].to_bits(), 0);
    assert_eq!(parts[5][8].to_bits(), 0x8000_0000);
    assert_eq!(parts[5][11].to_bits(), 0x8000_0000);
}

#[test]
fn errors_do_not_publish_partial_owner_or_run_later_source_reads() {
    let trace = Trace::default();
    let mut attributes = Attributes::source_fixture(trace.clone());
    attributes.fail_at = Some(0);
    let result = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut Numeric {
            values: Ok((1.0, 0.0)),
            log: trace.clone(),
        },
    );
    assert!(matches!(
        result,
        Err(SkateboardInitialTransformError::Attribute(
            "attribute unavailable"
        ))
    ));
    assert_eq!(trace.borrow().len(), 1);
    trace.borrow_mut().clear();
    let mut attributes = Attributes::source_fixture(trace.clone());
    let result = try_initialize_skateboard_transforms(
        &mut attributes,
        &mut Numeric {
            values: Err("trig unavailable"),
            log: trace.clone(),
        },
    );
    assert!(matches!(
        result,
        Err(SkateboardInitialTransformError::Numeric("trig unavailable"))
    ));
    assert_eq!(attributes.looked_up, 13);
    assert_eq!(trace.borrow().last(), Some(&Event::SinCos(0x3fc9_0fdb)));
}

#[test]
fn real_culling_constructor_sets_all_directed_bits_but_not_padding() {
    let table = initialize_skateboard_culling_table();
    assert_eq!(table.rows, 7);
    assert_eq!(table.columns, 7);
    assert_eq!(table.array_size, 2);
    assert_eq!(table.words, [u32::MAX, 0x0001_ffff]);
    for first in 0..7 {
        for second in 0..7 {
            let bit = first * table.columns + second;
            assert_ne!(table.words[(bit >> 5) as usize] & (1 << (bit & 31)), 0);
        }
    }
    for bit in 49..64 {
        assert_eq!(table.words[bit >> 5] & (1 << (bit & 31)), 0);
    }
}

#[test]
fn host_provider_is_only_the_injected_trig_policy() {
    let (sine, cosine) = HostSkateboardInitialTransformNumeric
        .sine_cosine(f32::from_bits(0x3fc9_0fdb))
        .unwrap();
    assert_eq!((sine, cosine), f32::from_bits(0x3fc9_0fdb).sin_cos());
}
