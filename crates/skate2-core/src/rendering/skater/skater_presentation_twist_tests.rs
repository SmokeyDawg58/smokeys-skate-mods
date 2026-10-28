use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

pub(crate) struct Numeric {
    pub(crate) dots: usize,
    pub(crate) fail_dot: Option<usize>,
    pub(crate) norms: usize,
}
impl Numeric {
    pub(crate) fn new() -> Self {
        Self {
            dots: 0,
            fail_dot: None,
            norms: 0,
        }
    }
}
impl ReckoningNumeric for Numeric {
    type Error = &'static str;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.dots += 1;
        if self.fail_dot == Some(self.dots) {
            return Err("dot");
        }
        Ok((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(v.recip())
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        Ok(v.sqrt().recip())
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        HostReckoningNumeric.truncate_i32(v).map_err(|_| "integer")
    }
}
impl PresentationMatrixNumeric for Numeric {
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.norms += 1;
        Ok(((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]) + a[3] * b[3])
    }
}
pub(crate) struct Rounding(pub(crate) usize);
impl ReckoningSinCosNumeric for Rounding {
    type Error = &'static str;
    fn round_nearest_even(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.0 += 1;
        Ok(v.round_ties_even())
    }
}
pub(crate) fn pose(rotation: [f32; 4]) -> AndaleSqt {
    AndaleSqt {
        scale: [2.0, 3.0, 4.0, 5.0],
        rotation,
        translation: [6.0, 7.0, 8.0, 9.0],
    }
}
pub(crate) fn close(actual: [f32; 4], expected: [f32; 4]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "{actual} != {expected}"
        );
    }
}

#[test]
fn swing_is_removed_before_scaling_the_remaining_x_twist() {
    let bind = [pose([0.0, 0.0, 0.0, 1.0]); 2];
    let mut replay = [pose([0.5, 0.5, 0.5, 0.5]), pose([91.0; 4])];
    let mut numeric = Numeric::new();
    let mut rounding = Rounding(0);
    try_twist(&bind, &mut replay, 0, 1, 0.5, &mut numeric, &mut rounding).unwrap();
    close(replay[1].rotation, [0.382_683_43, 0.0, 0.0, 0.923_879_5]);
    assert_eq!(
        replay[1].scale.map(f32::to_bits),
        bind[1].scale.map(f32::to_bits)
    );
    assert_eq!(
        replay[1].translation.map(f32::to_bits),
        bind[1].translation.map(f32::to_bits)
    );
    assert_eq!(numeric.dots, 3);
    assert_eq!(numeric.norms, 1);
    assert_eq!(rounding.0, 4);
}

#[test]
fn failing_final_dot_preserves_native_intermediate_twist_store() {
    let bind = [pose([0.0, 0.0, 0.0, 1.0]), pose([0.0, 0.0, 1.0, 0.0])];
    let mut replay = [pose([0.5, 0.5, 0.5, 0.5]), pose([91.0; 4])];
    let mut numeric = Numeric::new();
    numeric.fail_dot = Some(3);
    assert!(matches!(
        try_twist(
            &bind,
            &mut replay,
            0,
            1,
            0.5,
            &mut numeric,
            &mut Rounding(0)
        ),
        Err(TwistError::Numeric("dot"))
    ));
    close(replay[1].rotation, [0.382_683_43, 0.0, 0.0, 0.923_879_5]);
}

#[test]
fn missing_bind_destination_is_checked_after_intermediate_store() {
    let bind = [pose([0.0, 0.0, 0.0, 1.0])];
    let mut replay = [pose([0.5, 0.5, 0.5, 0.5]), pose([91.0; 4])];
    assert!(matches!(
        try_twist(
            &bind,
            &mut replay,
            0,
            1,
            0.5,
            &mut Numeric::new(),
            &mut Rounding(0)
        ),
        Err(TwistError::BindBone(1))
    ));
    close(replay[1].rotation, [0.382_683_43, 0.0, 0.0, 0.923_879_5]);
}

#[test]
fn zero_transverse_length_has_no_invented_identity_shortcut() {
    let bind = [pose([0.0, 0.0, 0.0, 1.0]); 2];
    let mut replay = bind;
    let mut numeric = Numeric::new();
    try_twist(
        &bind,
        &mut replay,
        0,
        1,
        0.5,
        &mut numeric,
        &mut Rounding(0),
    )
    .unwrap();
    assert!(replay[1].rotation.into_iter().all(f32::is_nan));
    assert_eq!(numeric.dots, 3);
}
