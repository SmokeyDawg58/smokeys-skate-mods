use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;
use std::{cell::RefCell, rc::Rc};
struct Source {
    log: Rc<RefCell<Vec<&'static str>>>,
    vector: SkeletonRootVector,
    values: [f32; 4],
    mirror: bool,
    fail_y: bool,
    stores: Vec<(usize, u32)>,
}
impl BumpCoefficientSource for Source {
    type Error = &'static str;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("interfaces");
        Ok(())
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push("acceleration");
        Ok(self.vector)
    }
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error> {
        let (index, label) = match key {
            X_SCALE => (0, "scale"),
            MINIMUM => (1, "minimum"),
            MAXIMUM => (2, "maximum"),
            BASE => (3, "base"),
            _ => panic!("key"),
        };
        self.log.borrow_mut().push(label);
        Ok(self.values[index])
    }
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.log.borrow_mut().push("mirrored");
        Ok(self.mirror)
    }
    fn set_attribute(&mut self, index: usize, value: f32) -> Result<(), Self::Error> {
        self.log
            .borrow_mut()
            .push(if index == 0 { "X" } else { "Y" });
        if index == 1 && self.fail_y {
            return Err("Y");
        }
        self.stores.push((index, value.to_bits()));
        Ok(())
    }
}
struct Numeric(Rc<RefCell<Vec<&'static str>>>);
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.0.borrow_mut().push("dot");
        Ok(HostReckoningNumeric.dot3(a, b).unwrap())
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.0.borrow_mut().push("rsqrt");
        Ok(1. / v.sqrt())
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        unreachable!()
    }
    fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
        unreachable!()
    }
}
fn fixture() -> (Source, Numeric) {
    let log = Rc::new(RefCell::new(Vec::new()));
    (
        Source {
            log: Rc::clone(&log),
            vector: SkeletonRootVector {
                xyz: [0., 0., 2.],
                w: None,
            },
            values: [3., 0., 2., 0.25],
            mirror: true,
            fail_y: false,
            stores: vec![],
        },
        Numeric(log),
    )
}
#[test]
fn live_attributes_follow_length_and_mirror_precedes_x_then_z_publication() {
    let (mut source, mut numeric) = fixture();
    begin(&mut source, &mut numeric).unwrap();
    assert_eq!(
        *source.log.borrow(),
        [
            "interfaces",
            "acceleration",
            "scale",
            "dot",
            "rsqrt",
            "minimum",
            "maximum",
            "base",
            "dot",
            "rsqrt",
            "mirrored",
            "X",
            "Y"
        ]
    );
    assert_eq!(
        source.stores,
        [(0, (-0f32).to_bits()), (1, (-1f32).to_bits())]
    );
}
#[test]
fn zero_length_still_estimates_before_nan_fsel_and_normalize_fallback() {
    let (mut source, mut numeric) = fixture();
    source.vector.xyz = [0.; 3];
    source.values = [1., 0., 0., 0.25];
    source.mirror = false;
    begin(&mut source, &mut numeric).unwrap();
    assert_eq!(source.stores, [(0, 0), (1, 0)]);
    assert_eq!(
        source
            .log
            .borrow()
            .iter()
            .filter(|&&v| v == "rsqrt")
            .count(),
        1
    );
    // A nonzero vector with zero numerator/denominator exercises NaN clamp=>1.
    let (mut source, mut numeric) = fixture();
    source.values = [1., 2., 2., 0.25];
    source.mirror = false;
    begin(&mut source, &mut numeric).unwrap();
    assert_eq!(source.stores[1], (1, 1f32.to_bits()));
}
#[test]
fn second_write_failure_retains_the_first_attribute() {
    let (mut source, mut numeric) = fixture();
    source.fail_y = true;
    assert_eq!(begin(&mut source, &mut numeric), Err("Y"));
    assert_eq!(source.stores, [(0, (-0f32).to_bits())]);
}
