use super::*;
use crate::physical_player_phys_out::{PhysOutOrigin, PhysOutRemainder, PhysOutRemainderStage};
struct Remainder;
impl PhysOutRemainder for Remainder {
    type Error = ();
    fn run(&mut self, _: PhysOutRemainderStage) -> Result<(), ()> {
        Ok(())
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output.try_construct(&mut Remainder).unwrap();
    output
}
struct Numeric {
    fail: u8,
    calls: Vec<u8>,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = u8;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, u8> {
        self.calls.push(1);
        if self.fail == 1 {
            Err(1)
        } else {
            Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, u8> {
        self.calls.push(2);
        if self.fail == 2 {
            Err(2)
        } else {
            Ok(1.0 / v.sqrt())
        }
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, u8> {
        panic!("no reciprocal")
    }
}
fn vector() -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [3.0, 4.0, -0.0],
        w: None,
    }
}
#[test]
fn actual_field_graph_retains_unknown_w_and_publishes_speed() {
    let mut out = output();
    let mut n = Numeric {
        fail: 0,
        calls: vec![],
    };
    out.try_publish_selected_board_motion(vector(), || Ok::<_, ()>(vector()), &mut n)
        .unwrap();
    assert_eq!(n.calls, [1, 2]);
    assert_eq!(
        out.motion_vector(PhysOutMotionVector::DeckVelocity)
            .unwrap()
            .value
            .xyz
            .map(f32::to_bits),
        vector().xyz.map(f32::to_bits)
    );
    assert_eq!(
        out.motion_vector(PhysOutMotionVector::DeckVelocity)
            .unwrap()
            .value
            .w,
        None
    );
    assert_eq!(
        out.motion_scalar(PhysOutMotionScalar::DeckSpeed)
            .unwrap()
            .value
            .to_bits(),
        5.0f32.to_bits()
    );
}
#[test]
fn numeric_failures_preserve_only_source_prefix() {
    for fail in [1, 2] {
        let mut out = output();
        let mut n = Numeric {
            fail,
            calls: vec![],
        };
        assert_eq!(
            out.try_publish_selected_board_motion(vector(), || Ok::<_, ()>(vector()), &mut n),
            Err(PhysOutMotionError::Numeric(fail))
        );
        assert_eq!(
            out.motion_vector(PhysOutMotionVector::AngularVelocity)
                .unwrap()
                .origin,
            PhysOutOrigin::Publication
        );
        assert_eq!(
            out.motion_vector(PhysOutMotionVector::DeckVelocity)
                .unwrap()
                .origin
                == PhysOutOrigin::Publication,
            fail == 2
        );
        assert_ne!(
            out.motion_scalar(PhysOutMotionScalar::DeckSpeed)
                .unwrap()
                .origin,
            PhysOutOrigin::Publication
        );
    }
}
#[test]
fn locked_output_still_reads_angular_before_refusal() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut n = Numeric {
        fail: 0,
        calls: vec![],
    };
    let mut read = false;
    assert_eq!(
        out.try_publish_selected_board_motion(
            vector(),
            || {
                read = true;
                Ok::<_, ()>(vector())
            },
            &mut n
        ),
        Err(PhysOutMotionError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert!(read);
    assert!(n.calls.is_empty());
}
#[test]
fn zero_still_requests_estimate_and_selects_positive_zero() {
    let mut out = output();
    let mut n = Numeric {
        fail: 0,
        calls: vec![],
    };
    let zero = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    out.try_publish_selected_board_motion(zero, || Ok::<_, ()>(zero), &mut n)
        .unwrap();
    assert_eq!(n.calls, [1, 2]);
    assert_eq!(
        out.motion_scalar(PhysOutMotionScalar::DeckSpeed)
            .unwrap()
            .value
            .to_bits(),
        0
    );
}
