use super::*;
use crate::{
    physical_player_phys_out::construct::PhysOutAllocation,
    skeleton_root_transforms::SkeletonRootVector,
};
#[derive(Default)]
struct Numeric {
    calls: usize,
    fail: bool,
}
impl SkeletonCollisionNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 3], b: [f32; 3]) -> Result<f32, Self::Error> {
        self.calls += 1;
        if self.fail {
            Err("dot")
        } else {
            Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
        }
    }
    fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal")
    }
    fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
        panic!("no reciprocal sqrt")
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    let guard = out.sample_write_guard().unwrap();
    out.publish_motion_vector(
        &guard,
        PhysOutMotionVector::DeckVelocity,
        SkeletonRootVector {
            xyz: [2.0, 3.0, 4.0],
            w: None,
        },
    )
    .unwrap();
    out.publish_motion_scalar(&guard, PhysOutMotionScalar::ZSpeed, 19.0)
        .unwrap();
    out
}
fn transform() -> SkeletonRootMatrix {
    SkeletonRootMatrix::from_columns(
        [SkeletonRootVector {
            xyz: [-1.0, 0.0, 0.0],
            w: None,
        }; 4],
    )
}
#[test]
fn locked_output_consumes_fresh_transform_before_refusal_without_dot() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut calls = 0;
    let mut n = Numeric::default();
    assert_eq!(
        out.try_fill_board_forward(
            || {
                calls += 1;
                Ok::<_, ()>(transform())
            },
            &mut n
        ),
        Err(BoardForwardOutputError::Access(
            PhysOutAccessError::LockedForWrite
        ))
    );
    assert_eq!(calls, 1);
    assert_eq!(n.calls, 0);
}
#[test]
fn late_dot_failure_preserves_old_scalar_and_unknown_w_does_not_block_success() {
    let mut out = output();
    let mut n = Numeric {
        fail: true,
        ..Numeric::default()
    };
    assert_eq!(
        out.try_fill_board_forward(|| Ok::<_, ()>(transform()), &mut n),
        Err(BoardForwardOutputError::Numeric("dot"))
    );
    assert_eq!(
        out.motion_scalar(PhysOutMotionScalar::ZSpeed)
            .unwrap()
            .value
            .to_bits(),
        19.0_f32.to_bits()
    );
    n.fail = false;
    out.try_fill_board_forward(|| Ok::<_, ()>(transform()), &mut n)
        .unwrap();
    assert_eq!(
        out.motion_scalar(PhysOutMotionScalar::ZSpeed)
            .unwrap()
            .value
            .to_bits(),
        (-2.0_f32).to_bits()
    );
    assert_eq!(n.calls, 2);
    assert_eq!(
        out.motion_vector(PhysOutMotionVector::DeckVelocity)
            .unwrap()
            .value
            .w,
        None
    );
}
