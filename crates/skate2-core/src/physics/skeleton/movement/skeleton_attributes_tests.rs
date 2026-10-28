//! Independent source-order/layout discriminators. Synthetic record bytes are
//! test markers, not a constructed `SkeletonBody` or substituted runtime tuning.
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Lookup(SkeletonAttributeClass, u64, usize),
    Float(usize),
    Layout(SkeletonAttributeClass),
}

struct Reader {
    events: Vec<Event>,
    lookups: usize,
    drives: Vec<u8>,
    bones: Vec<u8>,
    joints: Vec<u8>,
    fail_event: Option<usize>,
}

fn set_word(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

impl Reader {
    fn marked() -> Self {
        let mut drives = vec![0; 736];
        for row in 0..23 {
            for field in 0..6 {
                let value = f32::from(u16::try_from(row * 10 + field + 1).unwrap());
                set_word(&mut drives, row * 32 + field * 4, value.to_bits());
            }
            drives[row * 32 + 24] = 0x80 + u8::try_from(row).unwrap();
            drives[row * 32 + 25..row * 32 + 28].copy_from_slice(&[0x00, 0xa5, 0x5a]);
            set_word(
                &mut drives,
                row * 32 + 28,
                0x7fc1_0000 + u32::try_from(row).unwrap(),
            );
        }
        let mut bones = vec![0; 576];
        for row in 0..24 {
            set_word(
                &mut bones,
                row * 24,
                f32::from(u8::try_from(row + 1).unwrap()).to_bits(),
            );
            set_word(
                &mut bones,
                row * 24 + 4,
                f32::from(u8::try_from(row + 101).unwrap()).to_bits(),
            );
            bones[row * 24 + 8] = 0x80 + u8::try_from(row).unwrap();
            bones[row * 24 + 9] = 0xff - u8::try_from(row).unwrap();
            bones[row * 24 + 10..row * 24 + 12].copy_from_slice(&[0xa5, 0x5a]);
            set_word(
                &mut bones,
                row * 24 + 12,
                0x1234_0000 + u32::try_from(row).unwrap(),
            );
            set_word(
                &mut bones,
                row * 24 + 16,
                0x7fc2_0000 + u32::try_from(row).unwrap(),
            );
            set_word(
                &mut bones,
                row * 24 + 20,
                0x8000_0000 + u32::try_from(row).unwrap(),
            );
        }
        let mut joints = vec![0; 440];
        for row in 0..22 {
            joints[row * 20] = 0x80 + u8::try_from(row).unwrap();
            joints[row * 20 + 1..row * 20 + 4].copy_from_slice(&[0xcc, 0xdd, 0xee]);
            for (field, bits) in [
                0x7fc3_0000 + u32::try_from(row).unwrap(),
                0x8000_0000,
                0x7f80_0000,
                0xff80_0000,
            ]
            .into_iter()
            .enumerate()
            {
                set_word(&mut joints, row * 20 + 4 + field * 4, bits);
            }
        }
        Self {
            events: Vec::new(),
            lookups: 0,
            drives,
            bones,
            joints,
            fail_event: None,
        }
    }

    fn event(&mut self, event: Event) -> Result<(), &'static str> {
        let ordinal = self.events.len();
        self.events.push(event);
        if self.fail_event == Some(ordinal) {
            Err("injected read refusal")
        } else {
            Ok(())
        }
    }
}

impl SkeletonAttributeRead for Reader {
    type Pointer = usize;
    type Layout = Vec<u8>;
    type Error = &'static str;
    fn lookup(&mut self, class: SkeletonAttributeClass, key: u64) -> Result<usize, Self::Error> {
        let ordinal = self.lookups;
        self.lookups += 1;
        self.event(Event::Lookup(class, key, ordinal))?;
        Ok(ordinal)
    }
    fn float(&mut self, pointer: &usize) -> Result<f32, Self::Error> {
        self.event(Event::Float(*pointer))?;
        // Distinct values for repeated-key handles reveal dereference routing;
        // the final two source lookups supply independent scale factors.
        Ok(match *pointer {
            9 => 2.0,
            10 => 3.0,
            value => f32::from(u16::try_from(value + 1).unwrap()),
        })
    }
    fn layout(&mut self, class: SkeletonAttributeClass) -> Result<Vec<u8>, Self::Error> {
        self.event(Event::Layout(class))?;
        Ok(match class {
            SkeletonAttributeClass::Drives => self.drives.clone(),
            SkeletonAttributeClass::Bones => self.bones.clone(),
            SkeletonAttributeClass::Joints => self.joints.clone(),
            SkeletonAttributeClass::Skeleton => {
                panic!("InitAttribData does not fetch this cached layout")
            }
        })
    }
}

fn expected_events() -> Vec<Event> {
    use Event::{Float, Layout, Lookup};
    use SkeletonAttributeClass::{Bones, Drives, Joints, Skeleton};
    // Independently read 0x82b3c3b8..c648 and c948..c990, not implementation keys.
    vec![
        Lookup(Skeleton, 0x8a43_3894_d234_21e6, 0),
        Float(0),
        Lookup(Skeleton, 0xfd1e_0898_d6f2_f5b8, 1),
        Lookup(Skeleton, 0xfd1e_0898_d6f2_f5b8, 2),
        Float(2),
        Float(1),
        Lookup(Skeleton, 0x5e3e_f6cc_8922_8e59, 3),
        Float(3),
        Lookup(Skeleton, 0xb8d8_ecea_a7b3_b603, 4),
        Lookup(Skeleton, 0xb8d8_ecea_a7b3_b603, 5),
        Float(5),
        Float(4),
        Lookup(Skeleton, 0xbac7_c1be_3809_0b31, 6),
        Float(6),
        Lookup(Skeleton, 0xb086_b9c7_da14_b441, 7),
        Lookup(Skeleton, 0xb086_b9c7_da14_b441, 8),
        Float(7),
        Float(8),
        Layout(Drives),
        Lookup(Drives, 0x851d_fc1d_6128_976f, 9),
        Float(9),
        Lookup(Drives, 0x057e_ddfa_341f_9492, 10),
        Float(10),
        Layout(Bones),
        Layout(Joints),
    ]
}

#[test]
fn all_eleven_lookups_and_delayed_friction_loads_follow_source_order() {
    let mut reader = Reader::marked();
    let output = SkeletonBodyAttributeData::try_read(
        &mut reader,
        SkeletonDriveAttributes::constructor_value(),
    )
    .unwrap();
    assert_eq!(reader.events, expected_events());
    assert_eq!(reader.lookups, 11);
    assert_eq!(
        output.normal,
        SkeletonMaterial {
            restitution: 1.0,
            static_friction: 3.0,
            dynamic_friction: 2.0
        }
    );
    assert_eq!(
        output.ragdoll,
        SkeletonMaterial {
            restitution: 4.0,
            static_friction: 6.0,
            dynamic_friction: 5.0
        }
    );
    assert_eq!(
        output.ragdoll_head,
        SkeletonMaterial {
            restitution: 7.0,
            static_friction: 9.0,
            dynamic_friction: 8.0
        }
    );
}

#[test]
fn every_original_record_permutation_preserves_raw_fields_and_scales_only_six_drive_values() {
    // Independently verified unrolled source copies, SHA-256
    // 7d7a245d4ccdf66960975f564c795193ade652793fcbf803be330d2311b7d6a6.
    let drive_rows = [
        12, 13, 18, 19, 21, 16, 8, 9, 11, 6, 0, 1, 2, 3, 15, 20, 17, 14, 5, 10, 7, 4, 22,
    ];
    let bone_rows = [
        4, 13, 14, 19, 20, 22, 17, 9, 10, 12, 7, 0, 1, 2, 3, 16, 21, 18, 15, 6, 11, 8, 5, 23,
    ];
    let joint_rows = [
        12, 1, 16, 18, 14, 2, 9, 11, 7, 0, 3, 4, 5, 19, 17, 15, 13, 21, 10, 8, 6, 20,
    ];
    let root = SkeletonDriveAttributes {
        ragdoll_strength_start: -1.0,
        ragdoll_strength: -2.0,
        ragdoll_strength_controlled: -3.0,
        ragdoll_strength_end: -4.0,
        collision_strength: -5.0,
        collision_strength_root: -6.0,
        collision_compliant: 0xfd,
        padding: Some([0xab, 0xcd, 0xef]),
        collision_priority: -0.0,
    };
    let mut reader = Reader::marked();
    let output = SkeletonBodyAttributeData::try_read(&mut reader, root).unwrap();
    assert_eq!(output.drives[0], root);
    assert_eq!(output.drives[0].collision_priority.to_bits(), 0x8000_0000);
    for (part, row) in drive_rows.into_iter().enumerate() {
        let actual = output.drives[part + 1];
        let values = [
            actual.ragdoll_strength_start,
            actual.ragdoll_strength,
            actual.ragdoll_strength_controlled,
            actual.ragdoll_strength_end,
            actual.collision_strength,
            actual.collision_strength_root,
        ];
        for (field, actual) in values.into_iter().enumerate() {
            let original = f32::from(u16::try_from(row * 10 + field + 1).unwrap());
            let expected = original * if field < 4 { 2.0 } else { 3.0 };
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "drive part{} field{field}",
                part + 1
            );
        }
        assert_eq!(
            actual.collision_compliant,
            0x80 + u8::try_from(row).unwrap()
        );
        assert_eq!(actual.padding, Some([0, 0xa5, 0x5a]));
        assert_eq!(
            actual.collision_priority.to_bits(),
            0x7fc1_0000 + u32::try_from(row).unwrap()
        );
    }
    for (part, row) in bone_rows.into_iter().enumerate() {
        let actual = output.bones[part];
        assert_eq!(
            actual.mass_factor.to_bits(),
            f32::from(u8::try_from(row + 1).unwrap()).to_bits()
        );
        assert_eq!(
            actual.ragdoll_mass_factor.to_bits(),
            f32::from(u8::try_from(row + 101).unwrap()).to_bits()
        );
        assert_eq!(actual.has_collision, 0x80 + u8::try_from(row).unwrap());
        assert_eq!(actual.use_root_drive, 0xff - u8::try_from(row).unwrap());
        assert_eq!(actual.padding, [0xa5, 0x5a]);
        assert_eq!(
            actual.volume_type,
            0x1234_0000 + u32::try_from(row).unwrap()
        );
        assert_eq!(
            actual.volume_scalar.to_bits(),
            0x7fc2_0000 + u32::try_from(row).unwrap()
        );
        assert_eq!(
            actual.num_parents,
            0x8000_0000 + u32::try_from(row).unwrap()
        );
    }
    for (joint, row) in joint_rows.into_iter().enumerate() {
        let actual = output.joints[joint];
        assert_eq!(actual.ball_joint, 0x80 + u8::try_from(row).unwrap());
        assert_eq!(actual.padding, [0xcc, 0xdd, 0xee]);
        assert_eq!(
            [
                actual.swing_angle,
                actual.twist_angle,
                actual.swing_ragdoll,
                actual.twist_ragdoll
            ]
            .map(f32::to_bits),
            [
                0x7fc3_0000 + u32::try_from(row).unwrap(),
                0x8000_0000,
                0x7f80_0000,
                0xff80_0000
            ]
        );
    }
}

#[test]
fn every_read_failure_stops_at_its_original_call_prefix() {
    let expected = expected_events();
    for failure in 0..expected.len() {
        let mut reader = Reader::marked();
        reader.fail_event = Some(failure);
        assert!(matches!(
            SkeletonBodyAttributeData::try_read(
                &mut reader,
                SkeletonDriveAttributes::constructor_value()
            ),
            Err(SkeletonAttributeLoadError::Read("injected read refusal"))
        ));
        assert_eq!(reader.events, expected[..=failure]);
    }
}

#[test]
fn wrong_layout_size_stops_before_downstream_scalar_or_layout_reads() {
    use SkeletonAttributeClass::{Bones, Drives, Joints};
    for (class, size, stop) in [(Drives, 736, 18), (Bones, 576, 23), (Joints, 440, 24)] {
        for actual in [size - 1, size + 1] {
            let mut reader = Reader::marked();
            let bytes = match class {
                Drives => &mut reader.drives,
                Bones => &mut reader.bones,
                Joints => &mut reader.joints,
                SkeletonAttributeClass::Skeleton => unreachable!(),
            };
            bytes.resize(actual, 0);
            assert!(
                matches!(SkeletonBodyAttributeData::try_read(&mut reader, SkeletonDriveAttributes::constructor_value()),
                Err(SkeletonAttributeLoadError::LayoutSize { class: got, expected, actual: length }) if got == class && expected == size && length == actual)
            );
            assert_eq!(reader.events, expected_events()[..=stop]);
        }
    }
}

#[test]
fn absent_generated_layout_ranges_are_zero_and_constructor_root_padding_remains_unknown() {
    for (class, size) in [
        (SkeletonAttributeClass::Skeleton, 112),
        (SkeletonAttributeClass::Bones, 576),
        (SkeletonAttributeClass::Drives, 736),
        (SkeletonAttributeClass::Joints, 440),
    ] {
        let layout = SkeletonAttributeLayout {
            retained: None,
            size,
        };
        assert_eq!(layout.as_ref(), vec![0; size], "{class:?}");
    }
    let root = SkeletonDriveAttributes::constructor_value();
    assert_eq!(root.padding, None);
    assert_eq!(root.collision_compliant, 0);
    assert_eq!(
        [
            root.ragdoll_strength_start,
            root.ragdoll_strength,
            root.ragdoll_strength_controlled,
            root.ragdoll_strength_end,
            root.collision_strength,
            root.collision_strength_root,
            root.collision_priority
        ]
        .map(f32::to_bits),
        [0; 7]
    );
}

#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
#[expect(
    clippy::too_many_lines,
    reason = "one retained original Vault input-to-snapshot proof"
)]
fn mounted_original_vault_constructs_skeleton_attribute_snapshot_from_cached_layouts() {
    let vault = crate::attribute_runtime_corpus::mounted_physics_attributes_with_classes(&[
        0x9f86_2c2e_aa01_6ce5,
        0xf60d_fc08_2242_b489,
        0xfdf6_1860_0d39_6c3b,
        0x9ceb_4dce_6a21_3c4e,
    ]);
    let mut attributes = SkeletonAttributes::bind(&vault, SkeletonAttributes::DEFAULT_COLLECTION);
    let bones = attributes.layout(SkeletonAttributeClass::Bones).unwrap();
    // Independently read raw db.big BIN+0x9f2a8 layout, SHA-256
    // 156c7bce6b57cc7957bfa43333b5b0cc220f3c696db3c30705526d4cb04bfb06.
    for (row, expected) in [
        (
            4,
            [0x42c8_0000, 0x3f80_0000, 0x0001_0000, 3, 0x3f80_0000, 0],
        ),
        (
            13,
            [0x4080_0000, 0x3f80_0000, 0x0101_0000, 1, 0x3fc0_0000, 0],
        ),
        (
            23,
            [0x4200_0000, 0x3f80_0000, 0x0101_0000, 1, 0x3ff3_3333, 0],
        ),
    ] {
        let actual: [u32; 6] = std::array::from_fn(|word| {
            u32::from_be_bytes(
                bones.as_ref()[row * 24 + word * 4..row * 24 + word * 4 + 4]
                    .try_into()
                    .unwrap(),
            )
        });
        assert_eq!(actual, expected, "original generated bone layout row {row}");
    }
    // Raw source VLT slots: scales+5e100/+5e0d0; materials +5dde8,
    // +5dd88,+5ddf8,+5dd98,+5de08,+5dda8. Assertions are not producer inputs.
    assert_eq!(
        attributes
            .read_float(SkeletonAttributeClass::Drives, 0x851d_fc1d_6128_976f)
            .unwrap()
            .to_bits(),
        0x3f99_999a
    );
    assert_eq!(
        attributes
            .read_float(SkeletonAttributeClass::Drives, 0x057e_ddfa_341f_9492)
            .unwrap()
            .to_bits(),
        0x3f80_0000
    );
    let output = SkeletonBodyAttributeData::try_read(
        &mut attributes,
        SkeletonDriveAttributes::constructor_value(),
    )
    .unwrap();
    assert_eq!(
        [
            output.normal.restitution,
            output.normal.static_friction,
            output.normal.dynamic_friction
        ]
        .map(f32::to_bits),
        [0x3c23_d70a, 0, 0]
    );
    assert_eq!(
        [
            output.ragdoll.restitution,
            output.ragdoll.static_friction,
            output.ragdoll.dynamic_friction
        ]
        .map(f32::to_bits),
        [0x3c23_d70a, 0x3f73_3333, 0x3f73_3333]
    );
    assert_eq!(
        [
            output.ragdoll_head.restitution,
            output.ragdoll_head.static_friction,
            output.ragdoll_head.dynamic_friction
        ]
        .map(f32::to_bits),
        [0x3c23_d70a, 0x3ca3_d70a, 0x3ca3_d70a]
    );
    assert_eq!(
        output.bones.map(|bone| bone.ragdoll_mass_factor.to_bits()),
        [
            1., 1., 1., 4., 2., 2., 2., 4., 2., 2., 2., 1., 1., 1., 1., 4., 4., 1., 1., 4., 4., 1.,
            1., 1.
        ]
        .map(f32::to_bits)
    );
    assert_eq!(output.bones[0].mass_factor.to_bits(), 100.0_f32.to_bits());
    assert_eq!(
        (
            output.bones[0].has_collision,
            output.bones[0].use_root_drive,
            output.bones[0].volume_type
        ),
        (0, 1, 3)
    );
    assert_eq!(output.bones[1].mass_factor.to_bits(), 4.0_f32.to_bits());
    assert_eq!(output.bones[1].volume_scalar.to_bits(), 1.5_f32.to_bits());
    assert_eq!(output.bones[23].mass_factor.to_bits(), 32.0_f32.to_bits());
    assert_eq!(output.bones[23].volume_scalar.to_bits(), 0x3ff3_3333);
    assert_eq!(
        output.drives[0],
        SkeletonDriveAttributes::constructor_value()
    );
    let mut missing = SkeletonAttributes::bind(&vault, 0);
    let absent = missing
        .lookup(SkeletonAttributeClass::Skeleton, 0x8a43_3894_d234_21e6)
        .unwrap();
    assert!(absent.is_none());
    assert_eq!(missing.float(&absent).unwrap().to_bits(), 0);
    for class in [
        SkeletonAttributeClass::Bones,
        SkeletonAttributeClass::Drives,
        SkeletonAttributeClass::Joints,
    ] {
        assert!(
            missing
                .layout(class)
                .unwrap()
                .as_ref()
                .iter()
                .all(|&byte| byte == 0)
        );
    }
    drop(vault);
    let retained = SkeletonBodyAttributeData::try_read(
        &mut attributes,
        SkeletonDriveAttributes::constructor_value(),
    )
    .unwrap();
    assert_eq!(retained, output);
    println!(
        "raw original db.big -> retained four-class generated layouts/scalar FETCH -> 24 bones,23 copied drives + untouched root,22 joints; no body admission claim"
    );
}
