use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;

#[test]
fn apex_latches_only_strictly_negative_y_and_skips_later_velocity_reads() {
    for y in [0.0, -0.0, 2.0, f32::NAN, f32::INFINITY] {
        let mut reached = false;
        try_latch_apex(&mut reached, || Ok::<_, ()>(y)).unwrap();
        assert!(!reached, "unexpected apex at bits {:08x}", y.to_bits());
    }
    for y in [-1.0, f32::NEG_INFINITY] {
        let mut reached = false;
        try_latch_apex(&mut reached, || Ok::<_, ()>(y)).unwrap();
        assert!(reached);
        try_latch_apex(&mut reached, || -> Result<f32, ()> {
            panic!("native reachedApex arm must skip body velocity")
        })
        .unwrap();
    }
}

#[test]
fn failed_velocity_does_not_commit_apex() {
    let mut reached = false;
    assert_eq!(
        try_latch_apex(&mut reached, || Err("velocity")),
        Err("velocity")
    );
    assert!(!reached);
}

struct FillSource {
    calls: Vec<&'static str>,
    fail_at: Option<&'static str>,
}
impl FillSource {
    fn read(&mut self, field: &'static str) -> Result<(), &'static str> {
        self.calls.push(field);
        if self.fail_at == Some(field) {
            Err(field)
        } else {
            Ok(())
        }
    }
}
impl AirFillSource for FillSource {
    type Error = &'static str;
    fn reached_apex(&mut self) -> Result<bool, Self::Error> {
        self.read("apex")?;
        Ok(true)
    }
    fn max_y(&mut self) -> Result<f32, Self::Error> {
        self.read("max")?;
        Ok(13.5)
    }
    fn start_y(&mut self) -> Result<f32, Self::Error> {
        self.read("start")?;
        Ok(15.0)
    }
    fn landing_normal(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.read("normal")?;
        Ok(SkeletonRootVector::known([
            0.25,
            -0.0,
            -0.5,
            f32::from_bits(0x3f12_3456),
        ]))
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut output = PhysicalPlayerPhysOut::new();
    output
        .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    output
}
fn source(fail_at: Option<&'static str>) -> FillSource {
    FillSource {
        calls: Vec::new(),
        fail_at,
    }
}

#[test]
fn fill_keeps_apex_and_height_when_last_source_read_fails() {
    let mut output = output();
    let previous_normal = output.concrete.air.as_ref().unwrap().landing_normal;
    let mut source = source(Some("normal"));
    assert!(matches!(
        output.try_fill_air_state(&mut source),
        Err(AirFillError::Source("normal"))
    ));
    assert_eq!(source.calls, ["apex", "max", "start", "normal"]);
    let air = output.concrete.air.as_ref().unwrap();
    assert!(air.is_at_apex);
    assert_eq!(air.jump_height.to_bits(), (-1.5_f32).to_bits());
    assert_eq!(air.landing_normal, previous_normal);
}

#[test]
fn fill_start_read_failure_preserves_only_first_store() {
    let mut output = output();
    let previous = output.concrete.air.as_ref().unwrap().clone();
    let mut source = source(Some("start"));
    assert!(matches!(
        output.try_fill_air_state(&mut source),
        Err(AirFillError::Source("start"))
    ));
    let air = output.concrete.air.as_ref().unwrap();
    assert!(air.is_at_apex);
    assert_eq!(air.jump_height.to_bits(), previous.jump_height.to_bits());
    assert_eq!(air.landing_normal, previous.landing_normal);
    assert_eq!(source.calls, ["apex", "max", "start"]);
}

#[test]
fn fill_copies_all_normal_lanes_without_normalizing_or_clamping_height() {
    let mut output = output();
    let mut source = source(None);
    output.try_fill_air_state(&mut source).unwrap();
    let air = output.concrete.air.as_ref().unwrap();
    assert!(air.is_at_apex);
    assert_eq!(air.jump_height.to_bits(), (-1.5_f32).to_bits());
    assert_eq!(
        air.landing_normal.xyz.map(f32::to_bits),
        [0x3e80_0000, 0x8000_0000, 0xbf00_0000]
    );
    assert_eq!(air.landing_normal.w.unwrap().to_bits(), 0x3f12_3456);
    assert_eq!(source.calls, ["apex", "max", "start", "normal"]);
}

#[test]
fn locked_output_rejects_before_any_source_read() {
    let mut output = output();
    output.lock_after_update1().unwrap();
    let mut source = source(None);
    assert!(matches!(
        output.try_fill_air_state(&mut source),
        Err(AirFillError::Output(_))
    ));
    assert!(source.calls.is_empty());
    assert!(!output.concrete.air.as_ref().unwrap().is_at_apex);
}
