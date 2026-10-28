use super::*;
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    simulation_assembly_backend::constraint_session::physical_grounded::wipeout::post_solve::{
        MINIMUM_TIME, REST_SPEED, WipeoutPostSource, try_update_wipeout_post,
    },
};
struct Source {
    reads: Vec<&'static str>,
    fail: Option<&'static str>,
    hips: f32,
    neck: f32,
    rest: f32,
    slow: f32,
    minimum: f32,
}
impl Source {
    fn new() -> Self {
        Self {
            reads: vec![],
            fail: None,
            hips: 0.25,
            neck: 0.25,
            rest: 1.0,
            slow: 2.0,
            minimum: 0.5,
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
impl WipeoutPostSource for Source {
    type Error = &'static str;
    fn slow_speed(&mut self) -> Result<f32, Self::Error> {
        self.read("slow")?;
        Ok(self.slow)
    }
    fn global(&mut self, key: u64) -> Result<f32, Self::Error> {
        match key {
            REST_SPEED => {
                self.read("rest")?;
                Ok(self.rest)
            }
            MINIMUM_TIME => {
                self.read("minimum")?;
                Ok(self.minimum)
            }
            _ => panic!("unexpected global"),
        }
    }
    fn velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        let (value, label) = match part {
            23 => (self.hips, "hips"),
            1 => (self.neck, "neck"),
            _ => panic!("unexpected part"),
        };
        self.read(label)?;
        Ok(SkeletonRootVector {
            xyz: [value, 0.0, 0.0],
            w: None,
        })
    }
    fn squared_speed(&mut self, value: SkeletonRootVector) -> Result<f32, Self::Error> {
        self.read("dot")?;
        HostReckoningNumeric
            .dot_vector(value, value)
            .map_err(|_| "numeric")
    }
}
#[test]
fn late_read_failure_retains_both_flags_and_fast_hips_do_not_skip_neck() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.over = true;
    fields.slow = true;
    fields.time_in_state = 1.0;
    let mut source = Source::new();
    source.hips = 10.0;
    source.fail = Some("neck");
    assert_eq!(
        try_update_wipeout_post(&mut fields, &mut source),
        Err("neck")
    );
    assert!(fields.over && fields.slow);
    assert_eq!(
        source.reads,
        ["slow", "rest", "minimum", "hips", "dot", "neck"]
    );
    source.fail = None;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && !fields.slow);
}
#[test]
fn rest_and_slow_are_independent_and_all_gates_are_ordered_strict() {
    let mut fields = WipeoutGroundFields::constructed();
    fields.time_in_state = 1.0;
    let mut source = Source::new();
    source.neck = 2.0;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && fields.slow);
    source.neck = 0.25;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(fields.over && fields.slow);
    source.hips = 1.0;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && fields.slow);
    fields.time_in_state = source.minimum;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && !fields.slow);
    fields.time_in_state = f32::NAN;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && !fields.slow);
    fields.time_in_state = 1.0;
    source.hips = f32::NAN;
    try_update_wipeout_post(&mut fields, &mut source).unwrap();
    assert!(!fields.over && !fields.slow);
}
