#![expect(
    clippy::float_cmp,
    reason = "exact source-value and copied-field discriminators; signed-zero and fused-rounding checks use bits explicitly"
)]

use super::*;
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Lookup(SkateboardAttributeClass, u64),
    Float(u64),
    Byte(u64),
    Integer(u64),
    Cos(u32),
    Sin(u32),
    SinCos(u32),
    HalfReciprocal(u32),
    Dot([u32; 4]),
    Rsqrt(u32),
    Divide(u32, u32),
    Sqrt(u32),
    MassEstimate(SkateboardWheelInertiaAxis),
}
type Trace = Rc<RefCell<Vec<Event>>>;

struct Attributes {
    words: BTreeMap<u64, u32>,
    enables: VecDeque<u8>,
    log: Trace,
    mutate_delayed: bool,
}

impl Attributes {
    fn source_words(log: Trace) -> Self {
        // Scalar fixture copied from the source packet, not a mounted-corpus
        // provenance assertion. Root owns the independent real Vault test.
        Self {
            words: BTreeMap::from([
                (WIDTH, 0x3e75_c28f),
                (LENGTH, 0x3f17_0a3d),
                (THICKNESS, 0x3c75_c28f),
                (EXTENT, 0x3e28_f5c3),
                (SUBDIVISIONS, 5),
                (BOX_TRIANGLE_ENABLE, 0x0100_0000),
                (CAPSULE_ENABLE, 0x0100_0000),
                (POSITIVE_TILT, 0x4150_0000),
                (NEGATIVE_TILT, 0x4148_0000),
                (DYNAMIC_FRICTION, 0x3dcc_cccd),
                (STATIC_FRICTION, 0x3dcc_cccd),
                (RESTITUTION, 0),
                (ANGULAR_DRAG, 0x3ee6_6666),
                (MASS, 0x3f99_999a),
                (WORLD_MASS_MULTIPLIER, 0x40a0_0000),
            ]),
            enables: VecDeque::new(),
            log,
            mutate_delayed: false,
        }
    }
}

impl SkateboardAttributeRead for Attributes {
    type Pointer = u64;
    type Error = &'static str;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u64, Self::Error> {
        self.log.borrow_mut().push(Event::Lookup(class, key));
        if self.mutate_delayed && key == STATIC_FRICTION {
            self.words.insert(DYNAMIC_FRICTION, 0.75_f32.to_bits());
        }
        if self.mutate_delayed && key == WORLD_MASS_MULTIPLIER {
            self.words.insert(MASS, 2.0_f32.to_bits());
        }
        Ok(key)
    }
    fn float(&mut self, pointer: &u64) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Float(*pointer));
        self.words
            .get(pointer)
            .copied()
            .map(f32::from_bits)
            .ok_or("missing fixture word")
    }
    fn byte(&mut self, pointer: &u64) -> Result<u8, Self::Error> {
        self.log.borrow_mut().push(Event::Byte(*pointer));
        Ok(self
            .enables
            .pop_front()
            .unwrap_or(self.words[pointer].to_be_bytes()[0]))
    }
    fn integer(&mut self, pointer: &u64) -> Result<i32, Self::Error> {
        self.log.borrow_mut().push(Event::Integer(*pointer));
        Ok(i32::from_be_bytes(self.words[pointer].to_be_bytes()))
    }
}

struct Numeric {
    log: Trace,
    fail_trig: bool,
    half_seed: Option<f32>,
    zero_pairs: VecDeque<f32>,
}
impl Numeric {
    fn new(log: Trace) -> Self {
        Self {
            log,
            fail_trig: false,
            half_seed: None,
            zero_pairs: VecDeque::new(),
        }
    }
}
impl SerialisedAssemblyBBoxMinMaxNumeric for Numeric {
    type Error = &'static str;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostSkateboardDeckNumeric.minimum(left, right).unwrap())
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostSkateboardDeckNumeric.maximum(left, right).unwrap())
    }
}
impl SerialisedAssemblyPartBBoxNumeric for Numeric {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.log
            .borrow_mut()
            .push(Event::Dot(left.map(f32::to_bits)));
        Ok(HostSkateboardDeckNumeric
            .sum_three_products(left, right)
            .unwrap())
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Rsqrt(value.to_bits()));
        Ok(value.sqrt().recip())
    }
}
impl SimulationVolumeIntegralNumeric for Numeric {
    fn reciprocal_estimate(
        &mut self,
        _site: SimulationVolumeReciprocalSite,
        value: f32,
    ) -> Result<f32, Self::Error> {
        Ok(value.recip())
    }
    fn scalar_sqrt(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Sqrt(value.to_bits()));
        Ok(value.sqrt())
    }
    fn scalar_divide(&mut self, numerator: f32, denominator: f32) -> Result<f32, Self::Error> {
        self.log
            .borrow_mut()
            .push(Event::Divide(numerator.to_bits(), denominator.to_bits()));
        Ok(numerator / denominator)
    }
}
impl SkateboardWheelMassNumeric for Numeric {
    fn reciprocal_estimate(&mut self, axis: SkateboardWheelInertiaAxis, value: f32) -> f32 {
        self.log.borrow_mut().push(Event::MassEstimate(axis));
        value.recip()
    }
}
impl SkateboardDeckNumeric for Numeric {
    fn cosine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Cos(angle.to_bits()));
        if self.fail_trig {
            return Err("trig unavailable");
        }
        Ok(if angle == 0.0 {
            self.zero_pairs.pop_front().unwrap_or_else(|| angle.cos())
        } else {
            angle.cos()
        })
    }
    fn sine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push(Event::Sin(angle.to_bits()));
        Ok(if angle == 0.0 {
            self.zero_pairs.pop_front().unwrap_or_else(|| angle.sin())
        } else {
            angle.sin()
        })
    }
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        self.log.borrow_mut().push(Event::SinCos(angle.to_bits()));
        if self.fail_trig {
            Err("trig unavailable")
        } else {
            Ok(angle.sin_cos())
        }
    }
    fn capsule_half_reciprocal_estimate(&mut self, denominator: f32) -> Result<f32, Self::Error> {
        self.log
            .borrow_mut()
            .push(Event::HalfReciprocal(denominator.to_bits()));
        Ok(self.half_seed.unwrap_or_else(|| denominator.recip()))
    }
}

fn create() -> (SkateboardDeckDefinition, Trace) {
    let log = Trace::default();
    let mut attributes = Attributes::source_words(log.clone());
    let deck =
        SkateboardDeckDefinition::try_create(&mut attributes, &mut Numeric::new(log.clone()), 6)
            .unwrap();
    (deck, log)
}

#[test]
fn thirteen_actual_children_reach_mass_and_forced_identity() {
    let (deck, _) = create();
    assert_eq!(deck.aggregate().children().len(), 13);
    assert_eq!(deck.category_counts(), [5, 5, 3]);
    assert_eq!(deck.category_first_indices(), [4, 3, 0]);
    assert_eq!(deck.aggregate().tag_bits(), 4);
    assert_eq!(deck.aggregate().flags(), 1);
    assert_eq!(deck.aggregate().group_id(), 0);
    assert_eq!(deck.aggregate().surface_id(), 0);
    assert_eq!(deck.aggregate().radius().to_bits(), 0);
    assert_eq!(deck.debug_range(), 6..19);
    assert_eq!(deck.debug_objects().len(), 13);
    for (offset, object) in deck.debug_objects().iter().enumerate() {
        assert_eq!(object.index, 6 + u32::try_from(offset).unwrap());
        match (&object.shape, offset) {
            (SkateboardDeckDebugShape::Box { dimensions }, 0) => {
                assert_eq!(object.transform, FRESH_VOLUME_TRANSFORM);
                assert_eq!(
                    *dimensions,
                    [
                        f32::from_bits(0x3e75_c28f) * 0.5,
                        f32::from_bits(0x3c75_c28f) * 0.5,
                        f32::from_bits(0x3f17_0a3d) * 0.5,
                        0.0
                    ]
                );
            }
            (
                SkateboardDeckDebugShape::Capsule {
                    radius,
                    half_length,
                },
                1 | 2,
            ) => {
                assert_eq!(
                    object.transform,
                    deck.aggregate().children()[offset].transform
                );
                assert_eq!(*radius, f32::from_bits(0x3c75_c28f) * 0.5);
                assert!(*half_length > 0.0);
            }
            (SkateboardDeckDebugShape::Triangle { points, radius }, 3..=12) => {
                assert_eq!(object.transform, FRESH_VOLUME_TRANSFORM);
                assert_eq!(
                    points.as_flattened(),
                    &deck.aggregate().children()[offset].transform[..12]
                );
                assert_eq!(*radius, f32::from_bits(0x3c75_c28f) * 0.5);
            }
            _ => panic!("debug shape must follow the original child order"),
        }
    }
    assert_eq!(deck.inverse_body_transform(), Some(&FRESH_VOLUME_TRANSFORM));
    assert_eq!(deck.inverse_tensor_w(), deck.inertia().inverse_tensor[0]);
    assert!(!deck.is_static_body());
    assert!(deck.principal_mass().volume > 0.0);
    assert!(
        deck.principal_mass()
            .diagonal
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
    );
    assert_eq!(
        deck.inertia().inverse_mass.to_bits(),
        (1.0_f32 / (f32::from_bits(0x3f99_999a) * 5.0)).to_bits()
    );
    assert_eq!(
        deck.inertia().angular_drag.to_bits(),
        (f32::from_bits(0x3ee6_6666) * f32::from_bits(0x426f_ffff)).to_bits()
    );
    assert_eq!(deck.inertia().linear_drag, 0.0);
    assert_eq!(deck.inertia().max_velocity, f32::MAX);
    let mut numeric = HostSkateboardDeckNumeric;
    let expected = try_compute_mapped_array_integral(
        deck.aggregate().children(),
        Some(&FRESH_VOLUME_TRANSFORM),
        &mut numeric,
    )
    .unwrap();
    assert_eq!(&expected.integral, deck.integral());
    let central_only = try_compute_mapped_array_integral(
        &deck.aggregate().children()[..1],
        Some(&FRESH_VOLUME_TRANSFORM),
        &mut numeric,
    )
    .unwrap();
    assert!(deck.integral().columns[3][3] > central_only.integral.columns[3][3]);
}

#[test]
fn flags_edge_cosines_dirty_normal_and_interleaved_winding_survive() {
    let (deck, _) = create();
    let children = deck.aggregate().children();
    assert_eq!(children[0].volume_type(), Some(VolumeType::Box));
    assert!(
        children[1..3]
            .iter()
            .all(|volume| volume.volume_type() == Some(VolumeType::Capsule))
    );
    for (ordinal, child) in children.iter().enumerate().skip(3) {
        assert_eq!(child.flags, 0x3e3);
        assert_eq!(child.shape_words.map(f32::from_bits), [1.0, -1.0, 1.0]);
        assert_eq!(child.transform[12..], [0.0; 4]);
        assert_eq!(child.transform[0], 0.0);
        assert_eq!(child.transform[1], 0.0);
        assert_eq!(child.transform[2].is_sign_negative(), ordinal % 2 == 1);
        if ordinal > 4 {
            assert_eq!(
                child.transform[4..8],
                children[ordinal - 2].transform[8..12]
            );
        }
    }
    for child in &children[1..3] {
        assert_eq!(child.transform[..12], FRESH_VOLUME_TRANSFORM[..12]);
        assert_eq!(child.transform[15], 0.0);
    }
    assert_eq!(
        children[1].transform[12].to_bits(),
        (f32::from_bits(0x3e75_c28f) * 0.5 - f32::from_bits(0x3c75_c28f) * 0.5).to_bits()
    );
    assert!(children[3].transform[9] > 0.0);
    assert!(children[4].transform[9] > 0.0);
    assert_ne!(children[3].transform[9], children[4].transform[9]);
}

#[test]
fn all_twenty_six_fetches_and_delayed_pointer_loads_are_observable() {
    use SkateboardAttributeClass::{Deck, World};
    let (_, log) = create();
    let log = log.borrow();
    let actual: Vec<_> = log
        .iter()
        .filter_map(|event| {
            if let Event::Lookup(class, key) = event {
                Some((*class, *key))
            } else {
                None
            }
        })
        .collect();
    let mut expected = vec![
        (Deck, WIDTH),
        (Deck, LENGTH),
        (Deck, THICKNESS),
        (Deck, EXTENT),
        (Deck, SUBDIVISIONS),
        (Deck, BOX_TRIANGLE_ENABLE),
        (Deck, CAPSULE_ENABLE),
        (Deck, CAPSULE_ENABLE),
        (Deck, POSITIVE_TILT),
        (Deck, NEGATIVE_TILT),
    ];
    expected.extend(std::iter::repeat_n((Deck, BOX_TRIANGLE_ENABLE), 10));
    expected.extend([
        (Deck, DYNAMIC_FRICTION),
        (Deck, STATIC_FRICTION),
        (Deck, RESTITUTION),
        (Deck, ANGULAR_DRAG),
        (Deck, MASS),
        (World, WORLD_MASS_MULTIPLIER),
    ]);
    assert_eq!(actual, expected);
    assert!(log.windows(4).any(|events| events
        == [
            Event::Lookup(Deck, DYNAMIC_FRICTION),
            Event::Lookup(Deck, STATIC_FRICTION),
            Event::Float(DYNAMIC_FRICTION),
            Event::Float(STATIC_FRICTION)
        ]));
    assert!(log.windows(4).any(|events| events
        == [
            Event::Lookup(Deck, MASS),
            Event::Lookup(World, WORLD_MASS_MULTIPLIER),
            Event::Float(WORLD_MASS_MULTIPLIER),
            Event::Float(MASS)
        ]));
    drop(log);
    let trace = Trace::default();
    let mut attributes = Attributes::source_words(trace.clone());
    attributes.mutate_delayed = true;
    let deck =
        SkateboardDeckDefinition::try_create(&mut attributes, &mut Numeric::new(trace), 6).unwrap();
    assert_eq!(deck.material().dynamic_friction, 0.75);
    assert_eq!(deck.material().static_friction, f32::from_bits(0x3dcc_cccd));
    assert_eq!(deck.inertia().inverse_mass, 0.1);
}

#[test]
fn trig_and_capsule_calls_keep_source_order_even_unused_half_pi() {
    let (_, log) = create();
    let log = log.borrow();
    let trig: Vec<_> = log
        .iter()
        .filter(|event| matches!(event, Event::Cos(_) | Event::Sin(_) | Event::SinCos(_)))
        .cloned()
        .collect();
    let mut expected = vec![
        Event::SinCos(0x3fc9_0fdb),
        Event::Cos(0),
        Event::Sin(0),
        Event::Cos(0),
        Event::Sin(0),
    ];
    for degrees in [f32::from_bits(0x4150_0000), f32::from_bits(0x4148_0000)] {
        let bits = (degrees * f32::from_bits(0x3c8e_fa35)).to_bits();
        expected.extend([Event::Cos(bits), Event::Sin(bits)]);
    }
    for index in [1.0_f32, 2.0, 3.0, 4.0, 5.0] {
        for pi in [f32::from_bits(0xc049_0fdb), f32::from_bits(0x4049_0fdb)] {
            let bits = ((index * pi) / 5.0).to_bits();
            expected.extend([Event::Cos(bits), Event::Sin(bits)]);
        }
    }
    assert_eq!(trig, expected);
    let first_capsule = log
        .iter()
        .position(|event| matches!(event, Event::HalfReciprocal(_)))
        .unwrap();
    assert_eq!(log[first_capsule - 1], Event::SinCos(0x3fc9_0fdb));
    assert!(matches!(log[first_capsule + 1], Event::Dot(_)));
    assert!(matches!(log[first_capsule + 2], Event::Rsqrt(_)));
    assert!(matches!(log[first_capsule + 3], Event::Divide(_, _)));
    assert!(matches!(log[first_capsule + 4], Event::Sqrt(_)));
    assert_eq!(
        log[first_capsule + 5],
        Event::Lookup(SkateboardAttributeClass::Deck, CAPSULE_ENABLE)
    );
    assert_eq!(log[first_capsule + 6], Event::Byte(CAPSULE_ENABLE));
    assert!(matches!(log[first_capsule + 7], Event::Dot(_)));
    assert!(matches!(log[first_capsule + 8], Event::Rsqrt(_)));
    assert_eq!(
        log.iter()
            .filter(|event| matches!(event, Event::HalfReciprocal(_)))
            .count(),
        2
    );
    assert_eq!(
        log.iter()
            .filter(|event| matches!(event, Event::Rsqrt(_)))
            .count(),
        4
    );
    assert_eq!(
        &log[log.len() - 3..],
        [
            Event::MassEstimate(SkateboardWheelInertiaAxis::Y),
            Event::MassEstimate(SkateboardWheelInertiaAxis::X),
            Event::MassEstimate(SkateboardWheelInertiaAxis::Z)
        ]
    );
}

#[test]
fn enable_reads_are_per_child_nonzero_bytes_and_do_not_filter_bounds_or_mass() {
    let (enabled, _) = create();
    let trace = Trace::default();
    let mut attributes = Attributes::source_words(trace.clone());
    attributes.enables = VecDeque::from([0, 2, 0, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mixed =
        SkateboardDeckDefinition::try_create(&mut attributes, &mut Numeric::new(trace), 6).unwrap();
    assert_eq!(
        mixed
            .aggregate()
            .children()
            .iter()
            .map(|child| child.flags & 1)
            .collect::<Vec<_>>(),
        [0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(mixed.aggregate().bounds(), enabled.aggregate().bounds());
    assert_eq!(mixed.integral(), enabled.integral());
    assert_eq!(mixed.principal_mass(), enabled.principal_mass());
}

#[test]
fn independent_initial_trig_pairs_are_not_replaced_with_exact_endpoints() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(trace.clone());
    numeric.zero_pairs = VecDeque::from([0.8, 0.2, 0.7, 0.3]);
    let deck =
        SkateboardDeckDefinition::try_create(&mut Attributes::source_words(trace), &mut numeric, 6)
            .unwrap();
    let lateral = f32::from_bits(0x3e75_c28f) * 0.5 - f32::from_bits(0x3c75_c28f) * 0.5;
    assert_eq!(deck.aggregate().children()[3].transform[4], 0.8 * lateral);
    assert_eq!(deck.aggregate().children()[4].transform[4], 0.7 * lateral);
    assert_ne!(deck.aggregate().children()[3].transform[5], 0.0);
    assert_ne!(deck.aggregate().children()[4].transform[5], 0.0);
}

#[test]
fn capsule_estimates_are_refined_and_zero_horizontal_keeps_identity() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(trace.clone());
    numeric.half_seed = Some(0.4);
    let points = [[0.25, 0.0, 3.0, 0.0], [0.25, 0.0, -3.0, 0.0]];
    let capsule = initialize_capsule(points, 0.2, &mut numeric).unwrap();
    let seed1 = 0.4_f32.mul_add((-0.4_f32).mul_add(2.0, 1.0), 0.4);
    let seed2 = seed1.mul_add((-seed1).mul_add(2.0, 1.0), seed1);
    assert_eq!(
        capsule.transform[14].to_bits(),
        (3.0 + seed2 * -6.0).to_bits()
    );
    assert_ne!(capsule.transform[14], 0.0);
    assert_eq!(capsule.transform[..12], FRESH_VOLUME_TRANSFORM[..12]);
    let zero = initialize_capsule([[1.0, 2.0, 3.0, 0.0]; 2], 0.2, &mut numeric).unwrap();
    assert_eq!(zero.shape_words[0], 0);
    assert_eq!(zero.transform[..12], FRESH_VOLUME_TRANSFORM[..12]);
    assert!(
        trace
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::Rsqrt(0)))
    );
}

#[test]
fn host_capacity_errors_are_explicit_and_zero_count_is_not_negative_clamp() {
    for (count, initial) in [(u32::MAX, 6), (5, 20), (0x7fff_ffff, 6)] {
        let trace = Trace::default();
        let mut attributes = Attributes::source_words(trace.clone());
        attributes.words.insert(SUBDIVISIONS, count);
        let result = SkateboardDeckDefinition::try_create(
            &mut attributes,
            &mut Numeric::new(trace.clone()),
            initial,
        );
        match count {
            u32::MAX => assert!(matches!(
                result,
                Err(SkateboardDeckError::NegativeSubdivisions(-1))
            )),
            5 => assert!(matches!(
                result,
                Err(SkateboardDeckError::DebugCapacity {
                    initial: 20,
                    children: 13
                })
            )),
            _ => assert!(matches!(result, Err(SkateboardDeckError::CountOverflow))),
        }
        assert_eq!(
            trace
                .borrow()
                .iter()
                .filter(|event| matches!(event, Event::Lookup(_, _)))
                .count(),
            5
        );
        assert!(
            !trace
                .borrow()
                .iter()
                .any(|event| matches!(event, Event::SinCos(_)))
        );
    }
    let trace = Trace::default();
    let mut attributes = Attributes::source_words(trace.clone());
    attributes.words.insert(SUBDIVISIONS, 0);
    let deck =
        SkateboardDeckDefinition::try_create(&mut attributes, &mut Numeric::new(trace.clone()), 29)
            .unwrap();
    assert_eq!(deck.aggregate().children().len(), 3);
    assert_eq!(deck.debug_range(), 29..32);
    assert_eq!(deck.category_counts(), [0, 0, 3]);
    assert_eq!(
        trace
            .borrow()
            .iter()
            .filter(|event| matches!(event, Event::Cos(_)))
            .count(),
        4
    );
}

#[test]
fn unavailable_numeric_service_returns_error_without_publishing_owner() {
    let trace = Trace::default();
    let mut numeric = Numeric::new(trace.clone());
    numeric.fail_trig = true;
    let result = SkateboardDeckDefinition::try_create(
        &mut Attributes::source_words(trace.clone()),
        &mut numeric,
        6,
    );
    assert!(matches!(
        result,
        Err(SkateboardDeckError::Numeric("trig unavailable"))
    ));
    assert!(
        !trace
            .borrow()
            .iter()
            .any(|event| matches!(event, Event::HalfReciprocal(_)))
    );
}

#[test]
fn both_rim_z_offsets_preserve_fusion_with_analytic_bit_oracles() {
    // (1+2^-23)(1-2^-23)-1 = -2^-46 exactly. Rounding the
    // product before subtracting one instead loses the entire residual.
    // These expected words come from that identity, not the geometry helper.
    let high = f32::from_bits(0x3f80_0001);
    let low = f32::from_bits(0x3f7f_fffe);
    let negative = negative_rim((high, 1.0), (0.5, low), 1.0, 1.0, 1.0);
    assert_eq!(negative[2].to_bits(), 0xa880_0000);
    assert_eq!(((high * low) - 1.0).to_bits(), 0);
    let positive = positive_rim((high, 1.0), (0.5, -low), 1.0, 1.0, 1.0);
    assert_eq!(positive[2].to_bits(), 0x2880_0000);
    assert_eq!(((high * -low) + 1.0).to_bits(), 0);
    // Separately reviewed CRT fmaf fixture with positive dimensions and
    // ordinary fan signs; host arithmetic oracle, not a captured Xenon trace.
    let extent = f32::from_bits(0x3fe0_5dee);
    let cosine = f32::from_bits(0x3f1a_9cb3);
    let length = f32::from_bits(0x3fd6_b60e);
    assert_eq!(
        positive_rim((1.0, 0.0), (0.0, cosine), 1.0, extent, length)[2].to_bits(),
        0x402f_1bf3
    );
    assert_eq!(((extent * cosine) + length).to_bits(), 0x402f_1bf2);
    assert_eq!(
        negative_rim((-1.0, 0.0), (0.0, cosine), 1.0, extent, length)[2].to_bits(),
        0xc02f_1bf3
    );
    assert_eq!(((-extent * cosine) - length).to_bits(), 0xc02f_1bf2);
}
