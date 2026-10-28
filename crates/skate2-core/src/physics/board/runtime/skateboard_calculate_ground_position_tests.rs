use super::*;
struct Source {
    columns: [SkeletonRootVector; 3],
    points: [[f32; 3]; 4],
    calls: Vec<usize>,
    fail: Option<usize>,
}
impl Source {
    fn new() -> Self {
        Self {
            columns: [
                SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
                SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
                SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
            ],
            points: [
                [0.0, 4.0, 0.0],
                [4.0, 3.0, 4.0],
                [8.0, 2.0, 8.0],
                [12.0, 1.0, 12.0],
            ],
            calls: Vec::new(),
            fail: None,
        }
    }
    fn call(&mut self, site: usize) -> Result<(), usize> {
        self.calls.push(site);
        if self.fail == Some(site) {
            Err(site)
        } else {
            Ok(())
        }
    }
}
impl SkateboardGroundPositionSource for Source {
    type Ground = ();
    type Body = ();
    type Error = usize;
    fn capture_ground(&mut self) -> Result<(), usize> {
        self.call(0)
    }
    fn ground_column(&mut self, (): &(), column: usize) -> Result<SkeletonRootVector, usize> {
        self.call(1 + column)?;
        Ok(self.columns[column])
    }
    fn capture_body(&mut self) -> Result<(), usize> {
        self.call(4)
    }
    fn body_position(&mut self, (): &(), part: usize) -> Result<[f32; 3], usize> {
        self.call(5 + part)?;
        Ok(self.points[part])
    }
}
#[test]
fn wheel_average_and_minimum_height_use_actual_basis() {
    let mut s = Source::new();
    let mut out = SkeletonRootVector::known([99.0; 4]);
    try_calculate_ground_position(&mut s, &mut out).unwrap();
    assert_eq!(
        out.xyz.map(f32::to_bits),
        [6.0_f32, 1.0, 6.0].map(f32::to_bits)
    );
    assert_eq!(out.w.unwrap().to_bits(), 0);
    assert_eq!(s.calls, (0..9).collect::<Vec<_>>());
    // Ground up is physical +X; choose lowest X, average physical Y/Z.
    s.columns = [
        SkeletonRootVector::known([0.0, -1.0, 0.0, 0.0]),
        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
    ];
    try_calculate_ground_position(&mut s, &mut out).unwrap();
    assert_eq!(
        out.xyz.map(f32::to_bits),
        [0.0_f32, 2.5, 6.0].map(f32::to_bits)
    );
}
#[test]
fn all_source_callback_refusals_retain_entry_or_zero() {
    for fail in 0..9 {
        let mut s = Source::new();
        s.fail = Some(fail);
        let entry = SkeletonRootVector {
            xyz: [9.0; 3],
            w: None,
        };
        let mut out = entry;
        assert_eq!(try_calculate_ground_position(&mut s, &mut out), Err(fail));
        assert_eq!(s.calls, (0..=fail).collect::<Vec<_>>());
        let expected = if fail < 4 {
            entry
        } else {
            SkeletonRootVector::known([0.0; 4])
        };
        assert_eq!(out, expected);
    }
}
#[test]
fn unknown_basis_w_survives_while_body_w_is_not_required() {
    let mut s = Source::new();
    s.columns[1].w = None;
    let mut out = SkeletonRootVector::known([99.0; 4]);
    try_calculate_ground_position(&mut s, &mut out).unwrap();
    assert_eq!(
        out.xyz.map(f32::to_bits),
        [6.0_f32, 1.0, 6.0].map(f32::to_bits)
    );
    assert!(out.w.is_none());
}
#[test]
fn source_minimum_fsel_nan_and_signed_zero_are_asymmetric() {
    assert_eq!(
        lowest_y([f32::NAN, 4.0, 3.0, 2.0]).to_bits(),
        2.0_f32.to_bits()
    );
    assert!(lowest_y([1.0, f32::NAN, 3.0, 2.0]).is_nan());
    assert_eq!(
        lowest_y([4.0, 3.0, f32::NAN, 2.0]).to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        lowest_y([4.0, 3.0, 2.0, f32::NAN]).to_bits(),
        2.0_f32.to_bits()
    );
    assert_eq!(
        lowest_y([0.0, -0.0, 0.0, -0.0]).to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(fsel(-0.0, -0.0, 0.0).to_bits(), (-0.0_f32).to_bits());
    assert_eq!(fsel(f32::NAN, 7.0, 9.0).to_bits(), 9.0_f32.to_bits());
    // Unlike an imposed affine zero, the source zero-coefficient product
    // legitimately becomes NaN for infinity.
    let t = [
        SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
        SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
        SkeletonRootVector::known([0.0, 0.0, 1.0, 0.0]),
    ];
    assert!(rotate(t, [f32::INFINITY, 0.0, 0.0]).w.unwrap().is_nan());
}
