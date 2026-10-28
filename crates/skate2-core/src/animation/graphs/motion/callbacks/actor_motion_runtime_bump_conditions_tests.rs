use super::*;
use crate::reckoning_numeric::ReckoningVector;
struct Source {
    reads: Vec<&'static str>,
    enabled: bool,
    fail: Option<&'static str>,
    threshold: f32,
    sampled: Option<SkeletonRootVector>,
}
impl Source {
    fn new() -> Self {
        Self {
            reads: vec![],
            enabled: true,
            fail: None,
            threshold: 10.0,
            sampled: None,
        }
    }
    fn read(&mut self, label: &'static str) -> Result<(), &'static str> {
        self.reads.push(label);
        if self.fail == Some(label) {
            Err(label)
        } else {
            Ok(())
        }
    }
}
impl BumpSource for Source {
    type Error = &'static str;
    type Pointer = u64;
    fn enabled(&mut self) -> Result<bool, Self::Error> {
        self.read("enable")?;
        Ok(self.enabled)
    }
    fn acceleration(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("physical")?;
        Ok(SkeletonRootVector {
            xyz: [3.0, 4.0, 0.0],
            w: None,
        })
    }
    fn lookup(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.read(if key == X_SCALE {
            "capture scale"
        } else {
            "capture threshold"
        })?;
        Ok(key)
    }
    fn value(&mut self, p: &u64) -> Result<f32, Self::Error> {
        self.read(if *p == X_SCALE { "scale" } else { "threshold" })?;
        Ok(if *p == X_SCALE { 2.0 } else { self.threshold })
    }
    fn squared(&mut self, v: SkeletonRootVector) -> Result<f32, Self::Error> {
        self.read("dot")?;
        self.sampled = Some(v);
        Ok(crate::reckoning_numeric::host_dot3(
            v.operand(),
            v.operand(),
        ))
    }
    fn estimate(&mut self, s: f32) -> Result<f32, Self::Error> {
        self.read("estimate")?;
        Ok(1.0 / s.sqrt())
    }
}
#[test]
fn disabled_bump_never_reads_physical_or_attributes() {
    let mut s = Source::new();
    s.enabled = false;
    s.fail = Some("physical");
    assert!(!try_bumped(&mut s).unwrap());
    assert_eq!(s.reads, ["enable"]);
}
#[test]
fn bump_captures_threshold_before_dot_and_reads_value_before_estimate() {
    let mut s = Source::new();
    assert!(!try_bumped(&mut s).unwrap());
    assert_eq!(
        s.reads,
        [
            "enable",
            "physical",
            "capture scale",
            "scale",
            "capture threshold",
            "dot",
            "threshold",
            "estimate"
        ]
    );
    let v = s.sampled.unwrap();
    assert_eq!(
        v.xyz.map(f32::to_bits),
        [6.0_f32, 4.0, 0.0].map(f32::to_bits)
    );
    assert!(v.w.is_none());
    s.reads.clear();
    s.fail = Some("threshold");
    assert_eq!(try_bumped(&mut s), Err("threshold"));
    assert_eq!(s.reads.last(), Some(&"threshold"));
    assert!(!s.reads.contains(&"estimate"));
    s.fail = None;
    s.threshold = f32::NAN;
    assert!(!try_bumped(&mut s).unwrap());
    s.threshold = 5.0;
    assert!(try_bumped(&mut s).unwrap());
}
#[test]
fn surface_slope_uses_y_degrees_and_source_comparison_without_domain_clamp() {
    use crate::generated_action_load::construct::ComparisonOp;
    let cmp = Comparison {
        op: ComparisonOp::Equal,
        absolute: false,
        reference: 90.0,
    };
    assert!(
        slope(
            SkeletonRootVector {
                xyz: [f32::NAN, 0.0, f32::INFINITY],
                w: None
            },
            cmp,
            &mut HostReckoningNumeric
        )
        .unwrap()
    );
    assert!(
        !slope(
            SkeletonRootVector::known([0.0, 2.0, 0.0, 0.0]),
            cmp,
            &mut HostReckoningNumeric
        )
        .unwrap()
    );
    assert!(
        !slope(
            SkeletonRootVector::known([0.0, f32::NAN, 0.0, 0.0]),
            cmp,
            &mut HostReckoningNumeric
        )
        .unwrap()
    );
}
