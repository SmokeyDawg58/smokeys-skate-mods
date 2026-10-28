use super::*;

struct Source {
    calls: Vec<&'static str>,
    delta: SkeletonRootVector,
    dt: f32,
    fail: Option<&'static str>,
    applied: Option<SkeletonRootVector>,
}
impl ComCorrectionSource for Source {
    type Error = &'static str;
    fn delta(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("delta");
        if self.fail == Some("delta") {
            Err("delta")
        } else {
            Ok(self.delta)
        }
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("time");
        if self.fail == Some("time") {
            Err("time")
        } else {
            Ok(self.dt)
        }
    }
    fn apply(&mut self, correction: SkeletonRootVector) -> Result<(), Self::Error> {
        self.calls.push("apply");
        if self.fail == Some("apply") {
            return Err("apply");
        }
        self.applied = Some(correction);
        Ok(())
    }
}
fn source() -> Source {
    Source {
        calls: vec![],
        delta: SkeletonRootVector::known([2.0, -3.0, 5.0, -0.0]),
        dt: 0.25,
        fail: None,
        applied: None,
    }
}
#[test]
fn native_byte_gate_accepts_any_nonzero_and_null_defaults_false() {
    assert!(!decode_enable(None).unwrap());
    assert!(!decode_enable(Some(&[0])).unwrap());
    assert!(decode_enable(Some(&[2])).unwrap());
    assert!(decode_enable(Some(&[255])).unwrap());
    assert!(matches!(
        decode_enable(Some(&[])),
        Err(AttributeRuntimeError::PayloadSize {
            expected: 1,
            actual: 0,
            ..
        })
    ));
}
#[test]
fn correction_uses_negative_full_delta_and_processed_time() {
    let mut s = source();
    try_correction(&mut s).unwrap();
    assert_eq!(s.calls, ["delta", "time", "apply"]);
    let value = s.applied.unwrap();
    assert_eq!(
        value.xyz.map(f32::to_bits),
        [-8.0_f32, 12.0, -20.0].map(f32::to_bits)
    );
    assert_eq!(value.w.unwrap().to_bits(), 0);
}
#[test]
fn reciprocal_is_rounded_before_vector_multiply() {
    let mut s = source();
    s.delta.xyz = [7.0, 5.0, -7.0];
    s.dt = 3.0;
    try_correction(&mut s).unwrap();
    let value = s.applied.unwrap();
    assert_eq!(
        value.xyz.map(f32::to_bits),
        [-2.333_333_5_f32, -1.666_666_7, 2.333_333_5].map(f32::to_bits)
    );
    assert_ne!(value.xyz[0].to_bits(), (-7.0_f32 / 3.0).to_bits());
}
#[test]
fn unknown_w_remains_unknown_and_negative_time_is_not_clamped() {
    let mut s = source();
    s.delta.w = None;
    s.dt = -0.5;
    try_correction(&mut s).unwrap();
    assert_eq!(
        s.applied.unwrap(),
        SkeletonRootVector {
            xyz: [4.0, -6.0, 10.0],
            w: None
        }
    );
}
#[test]
fn missing_delta_or_time_never_applies_and_body_refusal_keeps_read_order() {
    for (failure, expected) in [
        ("delta", vec!["delta"]),
        ("time", vec!["delta", "time"]),
        ("apply", vec!["delta", "time", "apply"]),
    ] {
        let mut s = source();
        s.fail = Some(failure);
        assert_eq!(try_correction(&mut s), Err(failure));
        assert_eq!(s.calls, expected);
        assert!(s.applied.is_none());
    }
}
#[test]
fn zero_time_propagates_ieee_values_to_actual_apply_boundary() {
    let mut s = source();
    s.delta.xyz[0] = 0.0;
    s.dt = 0.0;
    try_correction(&mut s).unwrap();
    let v = s.applied.unwrap();
    assert!(v.xyz[0].is_nan());
    assert_eq!(v.xyz[1].to_bits(), f32::INFINITY.to_bits());
    assert_eq!(v.xyz[2].to_bits(), f32::NEG_INFINITY.to_bits());
    assert!(v.w.unwrap().is_nan());
}
