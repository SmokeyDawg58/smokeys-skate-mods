use super::*;

#[test]
fn wheel_contact_timer_uses_plain_add_or_positive_zero_without_sanitizing_old_value() {
    let mut fields = SkateboardBodyPostFields::from_constructor();
    assert_eq!(fields.time_since_wheels_touching.to_bits(), 0);
    fields.time_since_wheels_touching = -3.0;
    fields.advance_wheel_contact_timer(0.25);
    assert_eq!(
        fields.time_since_wheels_touching.to_bits(),
        (-2.75_f32).to_bits()
    );
    fields.time_since_wheels_touching = f32::NAN;
    fields.advance_wheel_contact_timer(0.25);
    assert!(fields.time_since_wheels_touching.is_nan());
    for count in [1, 4, u8::MAX] {
        fields.num_wheels = count;
        fields.time_since_wheels_touching = f32::NAN;
        fields.advance_wheel_contact_timer(f32::NAN);
        assert_eq!(fields.time_since_wheels_touching.to_bits(), 0);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Lookup(Class, u64),
    Float(usize),
    Byte(usize),
}

struct Reader {
    events: Vec<Event>,
    next_pointer: usize,
    words: [u32; 11],
    byte: u8,
    fail_at: Option<usize>,
}

impl Reader {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            next_pointer: 0,
            byte: 1,
            fail_at: None,
            // Independent unusual-value oracle: no material arbitration or
            // finite checks are permitted in this source copy-only suffix.
            words: [
                0xbf80_0000,
                0x4000_0000,
                0x8000_0000,
                0x7fc0_1234,
                0x4040_0000,
                0x4080_0000,
                0x40a0_0000,
                0x40c0_0000,
                0x40e0_0000,
                0x4100_0000,
                0x4110_0000,
            ],
        }
    }
    fn event(&mut self, event: Event) -> Result<(), usize> {
        let index = self.events.len();
        self.events.push(event);
        if self.fail_at == Some(index) {
            Err(index)
        } else {
            Ok(())
        }
    }
}

impl SkateboardAttributeRead for Reader {
    type Pointer = usize;
    type Error = usize;
    fn lookup(&mut self, class: Class, key: u64) -> Result<usize, usize> {
        self.event(Event::Lookup(class, key))?;
        let pointer = self.next_pointer;
        self.next_pointer += 1;
        Ok(pointer)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, usize> {
        self.event(Event::Float(*pointer))?;
        Ok(f32::from_bits(self.words[*pointer]))
    }
    fn byte(&mut self, pointer: &usize) -> Result<u8, usize> {
        self.event(Event::Byte(*pointer))?;
        Ok(self.byte)
    }
    fn integer(&mut self, _: &usize) -> Result<i32, usize> {
        panic!("this source prefix has no integer load")
    }
}

fn material_bits(material: AssemblyPartMaterial) -> [u32; 3] {
    [
        material.static_friction.to_bits(),
        material.dynamic_friction.to_bits(),
        material.restitution.to_bits(),
    ]
}

fn inventory_bits(materials: &SkateboardBodyMaterials) -> [[u32; 3]; 7] {
    [
        *materials.wheel(),
        *materials.deck(),
        *materials.truck(),
        *materials.wheel_grind(),
        *materials.deck_grind(),
        *materials.truck_grind(),
        *materials.wipeout(),
    ]
    .map(material_bits)
}

fn expected_material_events() -> Vec<Event> {
    // Independently transcribed source call order, not the implementation's keys.
    [
        (Class::Wheels, 0x7d77_7041_dbfa_9234),
        (Class::Wheels, 0x2a10_82dd_feef_3ea7),
        (Class::Deck, 0x254d_958a_72d5_9a2d),
        (Class::Wheels, 0x5d09_5e63_6526_0b84),
        (Class::Trucks, 0x3298_05a8_2950_ab54),
        (Class::Deck, 0xaae1_43f5_64f2_63aa),
        (Class::Trucks, 0xb34a_6487_0f3f_5944),
        (Class::Deck, 0xac32_e9a7_f005_6c6d),
        (Class::Trucks, 0x7e13_5b94_4834_707e),
        (Class::Wipeout, 0x9904_48fc_2357_51f6),
        (Class::Wipeout, 0xd0fb_2f7d_77b0_bea4),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (class, key))| [Event::Lookup(class, key), Event::Float(index)])
    .collect()
}

#[test]
fn seven_source_constructor_materials_are_not_zero_fallbacks() {
    assert_eq!(
        inventory_bits(&SkateboardBodyMaterials::new()),
        [[0x3f00_0000, 0x3e99_999a, 0x3ecc_cccd]; 7]
    );
}

#[test]
fn early_byte_then_distance_preserves_nonzero_and_raw_float() {
    for byte in [0, 1, 128, 255] {
        let mut reader = Reader::new();
        reader.byte = byte;
        reader.words[1] = 0x8000_0000;
        let mut joints = None;
        let mut distance = None;
        try_read_early(&mut reader, &mut joints, &mut distance).unwrap();
        assert_eq!(joints, Some(if byte == 0 { 0 } else { 6 }));
        assert_eq!(distance.map(f32::to_bits), Some(0x8000_0000));
        assert_eq!(
            reader.events,
            [
                Event::Lookup(Class::Trucks, 0xb1cb_e3e9_29af_c8a0),
                Event::Byte(0),
                Event::Lookup(Class::Trucks, 0x2be0_8dd7_a920_7622),
                Event::Float(1),
            ]
        );
    }
}

#[test]
fn early_checked_failures_retain_only_completed_scalar_stores() {
    for failure in 0..4 {
        let mut reader = Reader::new();
        reader.fail_at = Some(failure);
        let mut joints = None;
        let mut distance = None;
        assert_eq!(
            try_read_early(&mut reader, &mut joints, &mut distance),
            Err(failure)
        );
        assert_eq!(joints, if failure >= 2 { Some(6) } else { None });
        assert!(distance.is_none());
        assert_eq!(reader.events.len(), failure + 1);
    }
}

#[test]
fn suffix_copies_inverted_friction_nan_and_signed_zero_without_rebinding() {
    let mut reader = Reader::new();
    let mut materials = SkateboardBodyMaterials::new();
    let original = [
        Arc::as_ptr(&materials.wheel),
        Arc::as_ptr(&materials.deck),
        Arc::as_ptr(&materials.truck),
        Arc::as_ptr(&materials.wheel_grind),
        Arc::as_ptr(&materials.deck_grind),
        Arc::as_ptr(&materials.truck_grind),
        Arc::as_ptr(&materials.wipeout),
    ];
    try_read_materials(&mut reader, &mut materials).unwrap();
    assert_eq!(
        original,
        [
            Arc::as_ptr(&materials.wheel),
            Arc::as_ptr(&materials.deck),
            Arc::as_ptr(&materials.truck),
            Arc::as_ptr(&materials.wheel_grind),
            Arc::as_ptr(&materials.deck_grind),
            Arc::as_ptr(&materials.truck_grind),
            Arc::as_ptr(&materials.wipeout)
        ]
    );
    assert_eq!(reader.events, expected_material_events());
    assert_eq!(
        inventory_bits(&materials),
        [
            [0xbf80_0000, 0x4000_0000, 0x7fc0_1234],
            [0x4080_0000, 0x40c0_0000, 0x8000_0000],
            [0x40a0_0000, 0x40e0_0000, 0x4040_0000],
            [0; 3],
            [0; 3],
            [0; 3],
            [0x4100_0000, 0x4100_0000, 0x4110_0000],
        ]
    );
    // This test exercises only the real retained-record kernel; no invented
    // Part material pointer or arbitrary production session is constructed.
}

#[test]
fn every_suffix_failure_leaves_all_seven_constructor_triples_intact() {
    let expected = expected_material_events();
    for failure in 0..22 {
        let mut reader = Reader::new();
        reader.fail_at = Some(failure);
        let mut materials = SkateboardBodyMaterials::new();
        assert_eq!(
            try_read_materials(&mut reader, &mut materials),
            Err(failure)
        );
        assert_eq!(reader.events, expected[..=failure]);
        assert_eq!(
            inventory_bits(&materials),
            [[0x3f00_0000, 0x3e99_999a, 0x3ecc_cccd]; 7]
        );
    }
}

#[test]
fn missing_real_vault_nodes_use_zero_not_constructor_defaults() {
    let vault = Arc::new(crate::attribute_runtime::tests::graph(
        Class::Wipeout.key(),
        vec![],
    ));
    let mut reader = SkateboardAttributes::bind(&vault);
    let mut materials = SkateboardBodyMaterials::new();
    try_read_materials(&mut reader, &mut materials).unwrap();
    assert_eq!(inventory_bits(&materials), [[0; 3]; 7]);
}

#[test]
fn real_reader_failure_keeps_builder_inputs_and_forbids_retry_or_handoff() {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    // Real typed immutable Vault machinery, with a deliberately malformed
    // one-byte distance node. No body geometry or pool capabilities are forged.
    let enable = 0xb1cb_e3e9_29af_c8a0;
    let distance = 0x2be0_8dd7_a920_7622;
    let mut vault = graph(
        Class::Trucks.key(),
        vec![definition(enable, 4, 1), definition(distance, 4, 1)],
    );
    collection(
        &mut vault,
        Class::Trucks.key(),
        SkateboardAttributes::DEFAULT_COLLECTION,
        0,
        None,
        vec![
            node(enable, 0x40, 0x0100_0000),
            node(distance, 0x40, 0x7f00_0000),
        ],
    );
    let vault = Arc::new(vault);
    let retained = Arc::downgrade(&vault);
    let mut construction = SkateboardBodyConstruction::new(&vault);
    drop(vault);
    assert!(retained.upgrade().is_some());
    assert_eq!(construction.phase(), SkateboardBodyConstructionPhase::Fresh);
    assert!(construction.standard_material_owners().is_none());
    assert!(construction.session().is_none());
    assert!(!construction.remain_on_reset());
    assert!(matches!(
        construction.try_construct(),
        Err(SkateboardBodyConstructionError::Attribute(
            SkateboardAttributeError::ValueSize {
                expected: 4,
                actual: 1
            }
        ))
    ));
    assert_eq!(construction.joint_count(), Some(6));
    assert!(construction.standard_material_owners().is_none());
    assert!(construction.wheel_distance().is_none());
    assert!(construction.group().is_none());
    assert!(construction.part_user_data().is_none());
    assert!(construction.wheels.is_none());
    assert_eq!(
        inventory_bits(construction.materials()),
        [[0x3f00_0000, 0x3e99_999a, 0x3ecc_cccd]; 7]
    );
    assert!(matches!(
        construction.try_construct(),
        Err(SkateboardBodyConstructionError::WrongPhase(
            SkateboardBodyConstructionPhase::EarlyAttributes
        ))
    ));
    assert!(matches!(
        construction.take_session(),
        Err(SkateboardBodyConstructionError::WrongPhase(
            SkateboardBodyConstructionPhase::EarlyAttributes
        ))
    ));
    drop(construction);
    assert!(retained.upgrade().is_none());
}
