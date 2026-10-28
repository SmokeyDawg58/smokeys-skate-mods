use super::*;
use crate::{
    physical_player_phys_out::{PhysOutRemainder, PhysOutRemainderStage},
    reckoning_numeric::HostReckoningNumeric,
};
use std::{cell::RefCell, rc::Rc};
struct Remainder;
impl PhysOutRemainder for Remainder {
    type Error = ();
    fn run(&mut self, _: PhysOutRemainderStage) -> Result<(), ()> {
        Ok(())
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut o = PhysicalPlayerPhysOut::new();
    o.try_construct(&mut Remainder).unwrap();
    let g = o.sample_write_guard().unwrap();
    o.publish_motion_vector(
        &g,
        PhysOutMotionVector::DeckVelocity,
        SkeletonRootVector {
            xyz: [3.0, 2.0, 4.0],
            w: None,
        },
    )
    .unwrap();
    o
}
struct Source {
    log: Rc<RefCell<Vec<&'static str>>>,
}
impl PhysOutSurfaceMotionSource for Source {
    type Error = &'static str;
    type Reckoning = ();
    type Processed = ();
    fn capture_reckoning(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("reckoning");
        Ok(())
    }
    fn capture_processed(&mut self) -> Result<(), Self::Error> {
        self.log.borrow_mut().push("processed");
        Ok(())
    }
    fn ground_normal(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push("normal");
        Ok(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn deck_z(&mut self, (): &()) -> Result<SkeletonRootVector, Self::Error> {
        self.log.borrow_mut().push("deck");
        Ok(SkeletonRootVector {
            xyz: [0.0, 0.0, 2.0],
            w: None,
        })
    }
    fn ground_transform(&mut self, (): &()) -> Result<SkeletonRootMatrix, Self::Error> {
        self.log.borrow_mut().push("transform");
        Ok(SkeletonRootMatrix::from_columns(
            [SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]); 4],
        ))
    }
}
struct Numeric {
    log: Rc<RefCell<Vec<&'static str>>>,
    dot_count: usize,
    fail: usize,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("dot");
        self.dot_count += 1;
        if self.dot_count == self.fail {
            return Err("dot");
        }
        HostReckoningNumeric.dot3(a, b).map_err(|_| "host")
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.log.borrow_mut().push("seed");
        HostReckoningNumeric
            .reciprocal_sqrt_estimate(v)
            .map_err(|_| "host")
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        HostReckoningNumeric
            .reciprocal_estimate(v)
            .map_err(|_| "host")
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(v).map_err(|_| "host")
    }
}
fn services(fail: usize) -> (Source, Numeric) {
    let log = Rc::new(RefCell::new(vec![]));
    (
        Source {
            log: Rc::clone(&log),
        },
        Numeric {
            log,
            dot_count: 0,
            fail,
        },
    )
}
#[test]
fn actual_projection_and_source_order_preserve_unknown_w() {
    let mut o = output();
    let (mut s, mut n) = services(0);
    o.try_publish_surface_motion(&mut s, &mut n).unwrap();
    let v = o
        .motion_vector(PhysOutMotionVector::SurfaceVelocity)
        .unwrap();
    assert_eq!(
        v.value.xyz.map(f32::to_bits),
        [3.0f32, 0.0, 4.0].map(f32::to_bits)
    );
    assert_eq!(v.value.w, None);
    assert_eq!(
        o.motion_scalar(PhysOutMotionScalar::SurfaceSpeed)
            .unwrap()
            .value
            .to_bits(),
        5.0f32.to_bits()
    );
    let log = s.log.borrow();
    assert_eq!(
        &log[..8],
        [
            "reckoning",
            "processed",
            "normal",
            "dot",
            "deck",
            "dot",
            "dot",
            "seed"
        ]
    );
    let transform_index = log.iter().position(|e| *e == "transform").unwrap();
    assert_eq!(log[transform_index + 1], "dot");
}
#[test]
fn first_dot_failure_does_not_read_deck_or_publish() {
    let mut o = output();
    let (mut s, mut n) = services(1);
    assert_eq!(
        o.try_publish_surface_motion(&mut s, &mut n),
        Err(PhysOutSurfaceMotionError::Numeric("dot"))
    );
    assert_eq!(*s.log.borrow(), ["reckoning", "processed", "normal", "dot"]);
    assert_ne!(
        o.motion_vector(PhysOutMotionVector::SurfaceVelocity)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
}
#[test]
fn failed_discarded_normalizer_remains_observable() {
    let mut o = output();
    let (mut s, mut n) = services(3);
    assert!(o.try_publish_surface_motion(&mut s, &mut n).is_err());
    assert!(!s.log.borrow().contains(&"transform"));
    assert_ne!(
        o.motion_scalar(PhysOutMotionScalar::SurfaceSpeed)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
}
#[test]
fn speed_dot_failure_preserves_vector_and_angle() {
    let mut probe = output();
    let (mut s, mut n) = services(0);
    probe.try_publish_surface_motion(&mut s, &mut n).unwrap();
    let final_dot = n.dot_count;
    let mut o = output();
    let (mut s, mut n) = services(final_dot);
    assert!(o.try_publish_surface_motion(&mut s, &mut n).is_err());
    assert_eq!(
        o.motion_vector(PhysOutMotionVector::SurfaceVelocity)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
    assert_eq!(
        o.motion_scalar(PhysOutMotionScalar::SurfaceAngle)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
    assert_ne!(
        o.motion_scalar(PhysOutMotionScalar::SurfaceSpeed)
            .unwrap()
            .origin,
        PhysOutOrigin::Publication
    );
}
#[test]
fn locked_output_refuses_before_any_source_capture() {
    let mut o = output();
    o.lock_after_update1().unwrap();
    let (mut s, mut n) = services(0);
    assert_eq!(
        o.try_publish_surface_motion(&mut s, &mut n),
        Err(PhysOutSurfaceMotionError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert!(s.log.borrow().is_empty());
}
#[test]
fn floor_wrap_edges_and_sign_xor_are_preserved() {
    assert_eq!(
        wrap_turns_to_radians(0.5).to_bits(),
        f32::from_bits(0x4049_0fdb).to_bits()
    );
    assert_eq!(
        wrap_turns_to_radians(-0.25).to_bits(),
        (-f32::from_bits(0x3fc9_0fdb)).to_bits()
    );
    let v = negate(SkeletonRootVector::known([
        -0.0,
        0.0,
        f32::from_bits(0x7fc1_2345),
        -0.0,
    ]));
    assert_eq!(v.xyz.map(f32::to_bits), [0, 0x8000_0000, 0xffc1_2345]);
    assert_eq!(v.w.unwrap().to_bits(), 0);
}
