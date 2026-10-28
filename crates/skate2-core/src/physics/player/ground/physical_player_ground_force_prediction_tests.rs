//! Source-order tests for `ground_force_prediction`.
use super::{GroundForcePredictionSource, try_predict_force_offset};
use crate::{
    reckoning_numeric::HostReckoningNumeric, skeleton_root_transforms::SkeletonRootVector as V,
};
struct Source {
    events: Vec<&'static str>,
    output: Option<V>,
    missing_normal: bool,
}
impl GroundForcePredictionSource for Source {
    type Error = &'static str;
    fn mass(&mut self) -> Result<f32, Self::Error> {
        self.events.push("mass");
        Ok(2.0)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(0.5)
    }
    fn combined_forces(&mut self) -> Result<V, Self::Error> {
        self.events.push("sum");
        Ok(V {
            xyz: [8.0, 12.0, 16.0],
            w: None,
        })
    }
    fn ground_normal(&mut self) -> Result<V, Self::Error> {
        self.events.push("normal");
        if self.missing_normal {
            return Err("normal");
        }
        Ok(V::known([0.0, 1.0, 0.0, 0.0]))
    }
    fn add_offset_to_hooks(&mut self, v: V) -> Result<(), Self::Error> {
        self.events.push("hooks");
        self.output = Some(v);
        Ok(())
    }
}
#[test]
fn actual_projection_removes_normal_component_and_retains_unknown_w() {
    let mut s = Source {
        events: Vec::new(),
        output: None,
        missing_normal: false,
    };
    try_predict_force_offset(false, &mut s, &mut HostReckoningNumeric).unwrap();
    let v = s.output.unwrap();
    assert_eq!(
        v.xyz.map(f32::to_bits),
        [1.0_f32, 0.0, 2.0].map(f32::to_bits)
    );
    assert_eq!(v.w, None);
    assert_eq!(s.events, ["mass", "dt", "sum", "normal", "hooks"]);
}
#[test]
fn wallride_gate_reads_nothing_and_normal_failure_follows_queue_sample() {
    let mut s = Source {
        events: Vec::new(),
        output: None,
        missing_normal: true,
    };
    try_predict_force_offset(true, &mut s, &mut HostReckoningNumeric).unwrap();
    assert!(s.events.is_empty());
    assert!(try_predict_force_offset(false, &mut s, &mut HostReckoningNumeric).is_err());
    assert_eq!(s.events, ["mass", "dt", "sum", "normal"]);
    assert!(s.output.is_none());
}
