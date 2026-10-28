use super::*;
use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};

struct Numeric {
    fail: bool,
    calls: Vec<&'static str>,
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
        panic!("dot3")
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("reciprocal");
        if self.fail {
            Err("estimate")
        } else {
            Ok(value.recip())
        }
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.calls.push("rsqrt");
        Ok(value.sqrt().recip())
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric
            .truncate_i32(value)
            .map_err(|_| "integer")
    }
}
impl PresentationMatrixNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.calls.push("dot4");
        Ok(((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]) + a[3] * b[3])
    }
}
fn sqt(translation: [f32; 4]) -> AndaleSqt {
    AndaleSqt {
        scale: [f32::NAN; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation,
    }
}

#[test]
fn actual_hips_and_board_rows_change_without_touching_other_bones() {
    let untouched = sqt([91.0; 4]);
    let mut replay = [
        sqt([2.0, 3.0, 4.0, f32::NAN]),
        sqt([7.0, 9.0, 11.0, f32::NAN]),
        untouched,
    ];
    let mut hips = [0.0; 16];
    let mut numeric = Numeric {
        fail: false,
        calls: vec![],
    };
    try_move_root_to_hips(&mut replay, 0, 1, &mut hips, &mut numeric).unwrap();
    assert_eq!(
        hips[12..16]
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>(),
        [2.0f32, 3.0, 4.0, 1.0].map(f32::to_bits)
    );
    assert_eq!(
        replay[0].translation.map(f32::to_bits),
        [0.0f32, 0.0, 0.0, 1.0].map(f32::to_bits)
    );
    assert_eq!(
        replay[1].translation.map(f32::to_bits),
        [5.0f32, 6.0, 7.0, 1.0].map(f32::to_bits)
    );
    assert_eq!(
        replay[1].scale.map(f32::to_bits),
        [1.0f32; 4].map(f32::to_bits)
    );
    assert_eq!(
        replay[2].scale.map(f32::to_bits),
        untouched.scale.map(f32::to_bits)
    );
    assert_eq!(
        replay[2].translation.map(f32::to_bits),
        untouched.translation.map(f32::to_bits)
    );
    assert_eq!(
        numeric.calls,
        ["dot4", "reciprocal", "rsqrt", "rsqrt", "rsqrt", "rsqrt"]
    );
}

#[test]
fn inverse_failure_keeps_hips_publication_and_reset_but_not_board_reset() {
    let mut replay = [sqt([2.0; 4]), sqt([7.0; 4])];
    let mut hips = [0.0; 16];
    let mut numeric = Numeric {
        fail: true,
        calls: vec![],
    };
    assert!(matches!(
        try_move_root_to_hips(&mut replay, 0, 1, &mut hips, &mut numeric),
        Err(MoveRootError::Numeric("estimate"))
    ));
    assert_eq!(hips[12].to_bits(), 2.0f32.to_bits());
    assert_eq!(
        replay[0].scale.map(f32::to_bits),
        [1.0f32; 4].map(f32::to_bits)
    );
    assert!(replay[1].scale.into_iter().all(f32::is_nan));
}

#[test]
fn later_missing_board_does_not_roll_back_hips_reset() {
    let mut replay = [sqt([2.0; 4])];
    let mut hips = [0.0; 16];
    let mut numeric = Numeric {
        fail: false,
        calls: vec![],
    };
    assert!(matches!(
        try_move_root_to_hips(&mut replay, 0, 1, &mut hips, &mut numeric),
        Err(MoveRootError::MissingBone(1))
    ));
    assert_eq!(
        replay[0].scale.map(f32::to_bits),
        [1.0f32; 4].map(f32::to_bits)
    );
    assert!(numeric.calls.is_empty());
}
