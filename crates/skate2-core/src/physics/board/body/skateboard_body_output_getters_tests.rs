use super::*;
use crate::skeleton_root_transforms::SkeletonRootVector;
#[derive(Default)]
struct Numeric {
    calls: Vec<&'static str>,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = ();
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, ()> {
        self.calls.push("dot");
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, ()> {
        self.calls.push("rsqrt");
        Ok(1.0 / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, ()> {
        panic!()
    }
}
#[test]
fn false_contact_does_not_require_unwritten_relative_velocity_or_numeric() {
    let mut state = SkateboardContactSurface::new();
    state.collision_state.in_contact[6] = Some(false);
    let mut numeric = Numeric::default();
    assert_eq!(
        state
            .try_part_slide_speed(6, &mut numeric)
            .unwrap()
            .to_bits(),
        0
    );
    assert!(numeric.calls.is_empty());
    state.collision_state.in_contact[6] = Some(true);
    assert_eq!(
        state.try_part_slide_speed(6, &mut numeric),
        Err(BoardSlideSpeedError::Unknown("selected relative velocity"))
    );
    assert!(numeric.calls.is_empty());
}
#[test]
fn true_contact_projects_relative_velocity_and_refines_zero_unconditionally() {
    let mut state = SkateboardContactSurface::new();
    state.collision_state.in_contact[6] = Some(true);
    state.collision_state.relative_velocity[6] = Some(SkeletonRootVector {
        xyz: [3.0, 4.0, 0.0],
        w: None,
    });
    state.collision_state.normal[6] = Some(SkeletonRootVector {
        xyz: [0.0, 1.0, 0.0],
        w: None,
    });
    let mut numeric = Numeric::default();
    let speed = state.try_part_slide_speed(6, &mut numeric).unwrap();
    assert!((speed - 3.0).abs() < 0.000_001);
    assert_eq!(numeric.calls, ["dot", "dot", "rsqrt"]);
    state.collision_state.relative_velocity[6] = Some(SkeletonRootVector {
        xyz: [0.0, 4.0, 0.0],
        w: None,
    });
    numeric.calls.clear();
    assert_eq!(
        state
            .try_part_slide_speed(6, &mut numeric)
            .unwrap()
            .to_bits(),
        0
    );
    assert_eq!(numeric.calls, ["dot", "dot", "rsqrt"]);
}
struct Attributes {
    calls: Vec<&'static str>,
    fail_second: bool,
}
impl SkateboardAttributeRead for Attributes {
    type Error = &'static str;
    type Pointer = u8;
    fn lookup(&mut self, class: SkateboardAttributeClass, key: u64) -> Result<u8, Self::Error> {
        match class {
            SkateboardAttributeClass::Wheels => {
                assert_eq!(key, 0x1264_6f5e_842c_4db3);
                self.calls.push("wheel lookup");
                Ok(1)
            }
            SkateboardAttributeClass::Trucks => {
                assert_eq!(key, 0x2be0_8dd7_a920_7622);
                self.calls.push("truck lookup");
                if self.fail_second {
                    Err("truck")
                } else {
                    Ok(2)
                }
            }
            _ => panic!("unexpected global class"),
        }
    }
    fn float(&mut self, p: &u8) -> Result<f32, Self::Error> {
        if *p == 1 {
            self.calls.push("wheel float");
            Ok(0.25)
        } else {
            self.calls.push("truck float");
            Ok(-0.5)
        }
    }
    fn byte(&mut self, _: &u8) -> Result<u8, Self::Error> {
        panic!()
    }
    fn integer(&mut self, _: &u8) -> Result<i32, Self::Error> {
        panic!()
    }
}
#[test]
fn both_attribute_pointers_precede_either_float_read() {
    let mut attrs = Attributes {
        calls: vec![],
        fail_second: true,
    };
    assert_eq!(try_deck_to_wheel_base(&mut attrs), Err("truck"));
    assert_eq!(attrs.calls, ["wheel lookup", "truck lookup"]);
    attrs.fail_second = false;
    attrs.calls.clear();
    assert_eq!(
        try_deck_to_wheel_base(&mut attrs).unwrap().to_bits(),
        0.75_f32.to_bits()
    );
    assert_eq!(
        attrs.calls,
        ["wheel lookup", "truck lookup", "wheel float", "truck float"]
    );
}
