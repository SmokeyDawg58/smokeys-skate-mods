#![expect(
    clippy::float_cmp,
    reason = "exact source branch and copied matrix assertions"
)]
use super::*;
use std::{cell::RefCell, rc::Rc};

fn fixture(hex: &str) -> HatLayout {
    assert_eq!(hex.len(), 96);
    HatLayout {
        bytes: std::array::from_fn(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap()),
    }
}

// Original raw db.big BIN ranges 0x9ccb0 and 0x9cce0 (48 bytes each).
const OTHER: &str = concat!(
    "3CF5C28F00000000BCA3D70A00000000",
    "3E99999A3FCCCCCD3E80000000000000",
    "3CA3D70A3E3851EC0000000000000000"
);
const FRONT: &str = concat!(
    "3CF5C28F3DE147AE0000000000000000",
    "3CF5C28F3FCCCCCD3E80000000000000",
    "3CA3D70A3DB851EC0FCFC1285FF9B2EB"
);

#[test]
fn original_layout_variants_retain_tail_and_source_collection_branch() {
    assert_eq!(HatAttributes::CLASS, 0x03df_76a4_7692_efff);
    for raw in [0, 1, 2, 3, 5, 6, u32::MAX] {
        assert_eq!(HatAttributes::collection(raw), 0x2d64_3131_0b6e_1662);
    }
    assert_eq!(HatAttributes::collection(4), 0x3e30_b6a3_73a5_8559);
    for (hex, radius, angle_x, pos_y, tail) in [
        (OTHER, 0x3e38_51ec, 0x3e99_999a, 0, [0; 8]),
        (
            FRONT,
            0x3db8_51ec,
            0x3cf5_c28f,
            0x3de1_47ae,
            [0x0f, 0xcf, 0xc1, 0x28, 0x5f, 0xf9, 0xb2, 0xeb],
        ),
    ] {
        let layout = fixture(hex);
        assert_eq!(layout.float(0x24).to_bits(), radius);
        assert_eq!(layout.float(0x10).to_bits(), angle_x);
        assert_eq!(layout.float(4).to_bits(), pos_y);
        assert_eq!(&layout.bytes()[40..], &tail);
        let stage = HatCylinder::try_create(&layout, &mut || true, &mut HostHatNumeric).unwrap();
        assert_eq!(stage.cylinder().shape_words[1], radius);
        assert_eq!(stage.cylinder().shape_words[0], 0x3c23_d70a);
        assert_eq!(stage.debug().radius.to_bits(), radius);
        assert_eq!(stage.debug().half_length.to_bits(), 0x3c23_d70a);
        assert_eq!(
            stage.debug().transform.map(f32::to_bits),
            stage.cylinder().transform.map(f32::to_bits)
        );
    }
}

struct Spy {
    events: Rc<RefCell<Vec<String>>>,
    calls: usize,
    fail: Option<usize>,
    pairs: Option<[(f32, f32); 3]>,
}
impl SerialisedAssemblyBBoxMinMaxNumeric for Spy {
    type Error = &'static str;
    fn minimum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostHatNumeric.minimum(left, right).unwrap())
    }
    fn maximum(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        Ok(HostHatNumeric.maximum(left, right).unwrap())
    }
}
impl SerialisedAssemblyPartBBoxNumeric for Spy {
    fn sum_three_products(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(HostHatNumeric.sum_three_products(left, right).unwrap())
    }
    fn reciprocal_square_root_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        Ok(HostHatNumeric
            .reciprocal_square_root_estimate(value)
            .unwrap())
    }
}
impl HatNumeric for Spy {
    fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        self.events
            .borrow_mut()
            .push(format!("trig:{:08x}", angle.to_bits()));
        let index = self.calls;
        self.calls += 1;
        if self.fail == Some(index) {
            return Err("trig unavailable");
        }
        Ok(self.pairs.unwrap_or([(2.0, 3.0), (5.0, 7.0), (11.0, 13.0)])[index])
    }
}

#[test]
fn samples_bracket_three_trig_calls_and_full_lane_euler_has_source_graph() {
    let mut layout = fixture(OTHER);
    layout.bytes[12..16].copy_from_slice(&0x8000_0000_u32.to_be_bytes());
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut numeric = Spy {
        events: Rc::clone(&events),
        calls: 0,
        fail: None,
        pairs: None,
    };
    let mut samples = [true, false].into_iter();
    let mut enable = || {
        let value = samples.next().unwrap();
        events.borrow_mut().push(format!("enable:{value}"));
        value
    };
    let stage = HatCylinder::try_create(&layout, &mut enable, &mut numeric).unwrap();
    assert_eq!(
        *events.borrow(),
        [
            "enable:true",
            "trig:3e99999a",
            "trig:3fcccccd",
            "trig:3e800000",
            "enable:false"
        ]
    );
    assert!(stage.initial_collision_enabled());
    assert_eq!(stage.cylinder().flags, 0);
    assert_eq!(
        stage.cylinder().transform[..12],
        [
            91.0, 77.0, -5.0, 91.0, 97.0, 149.0, 14.0, 97.0, 217.0, 139.0, 21.0, 217.0
        ]
    );
    assert_eq!(
        stage.cylinder().transform[12..]
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        [0x3cf5_c28f, 0, 0xbca3_d70a, 0x8000_0000]
    );
}

#[test]
fn trig_failure_does_not_sample_second_enable_or_complete_geometry() {
    for failed in 0..3 {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut numeric = Spy {
            events: Rc::clone(&events),
            calls: 0,
            fail: Some(failed),
            pairs: None,
        };
        let mut samples = 0;
        let result = HatCylinder::try_create(
            &fixture(OTHER),
            &mut || {
                samples += 1;
                true
            },
            &mut numeric,
        );
        assert_eq!(result.unwrap_err(), "trig unavailable");
        assert_eq!(samples, 1);
        assert_eq!(numeric.calls, failed + 1);
    }
}

#[test]
fn completed_owner_keeps_disabled_cylinder_in_bounds_and_sphere_in_source_order() {
    let make = |enabled| {
        HatCylinder::try_create(&fixture(FRONT), &mut || enabled, &mut HostHatNumeric)
            .unwrap()
            .try_finish(0.01, &mut HostHatNumeric)
            .unwrap()
    };
    let enabled = make(true);
    let disabled = make(false);
    assert_eq!(enabled.bounds(), disabled.bounds());
    assert_eq!(
        disabled.children()[0].volume_type(),
        Some(VolumeType::Cylinder)
    );
    assert_eq!(disabled.children()[0].flags, 0);
    assert_eq!(
        disabled.children()[1].volume_type(),
        Some(VolumeType::Sphere)
    );
    assert_eq!(disabled.children()[1].flags, 1);
    assert_eq!(disabled.children()[1].transform, FRESH_VOLUME_TRANSFORM);
    assert_eq!(disabled.children()[1].radius.to_bits(), 0.01_f32.to_bits());
    let sphere = try_get_primitive_bbox(&disabled.children()[1], None, 0, &mut HostHatNumeric)
        .unwrap()
        .unwrap();
    assert_ne!(disabled.bounds(), sphere);
    assert_eq!(disabled.frame(), &FRESH_VOLUME_TRANSFORM);
    assert_eq!(disabled.outer_metadata().flags, 1);
    assert_eq!(disabled.outer().group_id, 0);
    assert_eq!(disabled.outer().surface_id, 0);
    assert_eq!(disabled.outer().radius.to_bits(), 0);
    assert!(!disabled.initial_collision_enabled());
    assert_eq!(
        disabled.debug().transform.map(f32::to_bits),
        disabled.children()[0].transform.map(f32::to_bits)
    );
}

#[test]
fn zero_angles_preserve_noncanonical_axis_w_and_signed_zero_translation() {
    let mut layout = HatLayout { bytes: [0; 48] };
    layout.bytes[12..16].copy_from_slice(&0x8000_0000_u32.to_be_bytes());
    let stage = HatCylinder::try_create(&layout, &mut || true, &mut HostHatNumeric).unwrap();
    let frame = stage.cylinder().transform;
    assert_eq!([frame[3], frame[7], frame[11]], [1.0, 0.0, 0.0]);
    assert_eq!(frame[2].to_bits(), 0x8000_0000);
    assert_eq!(frame[15].to_bits(), 0x8000_0000);
}

#[test]
fn euler_positive_term_is_fused_not_rounded_multiply_then_add() {
    let successor = f32::from_bits(0x3f80_0001);
    let negative_next = f32::from_bits(0xbf80_0002);
    let mut numeric = Spy {
        events: Rc::default(),
        calls: 0,
        fail: None,
        pairs: Some([(successor, negative_next), (successor, 1.0), (1.0, 1.0)]),
    };
    let stage = HatCylinder::try_create(&fixture(OTHER), &mut || true, &mut numeric).unwrap();
    // (1+2^-23)^2 -(1+2^-22) = 2^-46 with a single fused rounding.
    assert_eq!(stage.cylinder().transform[5].to_bits(), 0x2880_0000);
    assert_eq!(((successor * successor) + negative_next).to_bits(), 0);
}
