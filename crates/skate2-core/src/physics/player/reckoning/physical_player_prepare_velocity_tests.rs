use super::*;
use crate::reckoning_numeric::HostReckoningNumeric;

struct Source {
    calls: Vec<&'static str>,
    prepare: bool,
    velocity: SkeletonRootVector,
    surface: SkeletonRootVector,
    spin: f32,
    fail: Option<&'static str>,
    captures: usize,
    published_count: Option<u32>,
    published_velocity: Option<SkeletonRootVector>,
}
impl Source {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            prepare: true,
            velocity: SkeletonRootVector {
                xyz: [1.0, 2.0, 3.0],
                w: None,
            },
            surface: SkeletonRootVector::known([0.0, 1.0, 0.0, 2.0]),
            spin: 1.0,
            fail: None,
            captures: 0,
            published_count: None,
            published_velocity: None,
        }
    }
    fn call(&mut self, name: &'static str) -> Result<(), &'static str> {
        self.calls.push(name);
        if self.fail == Some(name) {
            Err(name)
        } else {
            Ok(())
        }
    }
}
impl PhysicalPlayerPrepareVelocitySource for Source {
    type Processed = usize;
    type Error = &'static str;
    fn capture_processed(&mut self) -> Result<usize, Self::Error> {
        self.captures += 1;
        self.call(match self.captures {
            1 => "capture1",
            2 => "capture2",
            _ => "capture3",
        })?;
        Ok(self.captures)
    }
    fn prepare(&mut self, token: &usize) -> Result<bool, Self::Error> {
        assert_eq!(*token, 1);
        self.call("prepare")?;
        Ok(self.prepare)
    }
    fn velocity(&mut self, token: &usize) -> Result<SkeletonRootVector, Self::Error> {
        assert_eq!(*token, 1);
        self.call("velocity")?;
        Ok(self.velocity)
    }
    fn surface_velocity(&mut self, token: &usize) -> Result<SkeletonRootVector, Self::Error> {
        assert_eq!(*token, 1);
        self.call("surface")?;
        Ok(self.surface)
    }
    fn spin(&mut self, token: &usize) -> Result<f32, Self::Error> {
        assert_eq!(*token, 2);
        self.call("spin")?;
        Ok(self.spin)
    }
    fn publish_spin_count(&mut self, token: &usize, count: u32) -> Result<(), Self::Error> {
        assert_eq!(*token, 2);
        self.call("count")?;
        self.published_count = Some(count);
        Ok(())
    }
    fn publish_prepare_velocity(
        &mut self,
        token: &usize,
        velocity: SkeletonRootVector,
    ) -> Result<(), Self::Error> {
        assert_eq!(*token, 3);
        self.call("publish")?;
        self.published_velocity = Some(velocity);
        Ok(())
    }
}
fn initialized() -> PhysicalPlayerPrepareVelocity {
    let mut state = PhysicalPlayerPrepareVelocity::uninitialized();
    state.initialize_constructor().unwrap();
    state
}
fn bits(value: SkeletonRootVector) -> ([u32; 3], Option<u32>) {
    (value.xyz.map(f32::to_bits), value.w.map(f32::to_bits))
}
#[test]
fn constructor_is_explicit_and_prepare_copies_full_known_mask_before_spin() {
    let mut state = PhysicalPlayerPrepareVelocity::uninitialized();
    let mut source = Source::new();
    assert_eq!(
        state.try_update(&mut source, &mut HostReckoningNumeric),
        Err(PhysicalPlayerPrepareVelocityError::Uninitialized)
    );
    assert!(source.calls.is_empty());
    assert!(state.velocity().is_none());
    state.initialize_constructor().unwrap();
    assert_eq!(bits(state.velocity().unwrap()), ([0; 3], Some(0)));
    assert_eq!(state.last_spin().unwrap().to_bits(), 0);
    assert_eq!(state.same_direction_frames(), Some(0));
    assert_eq!(
        state.initialize_constructor(),
        Err(PrepareVelocityAlreadyInitialized)
    );
    state
        .try_update(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(bits(state.velocity().unwrap()), bits(source.velocity));
    assert_eq!(
        source.calls,
        [
            "capture1", "prepare", "velocity", "capture2", "spin", "count", "capture3", "publish"
        ]
    );
    assert_eq!(
        bits(source.published_velocity.unwrap()),
        bits(source.velocity)
    );
    assert!(state.complete());
}
#[test]
fn late_source_refusals_preserve_exact_history_and_spin_prefixes() {
    for fail in ["capture2", "spin", "count", "capture3", "publish"] {
        let mut state = initialized();
        let mut source = Source::new();
        source.fail = Some(fail);
        assert_eq!(
            state.try_update(&mut source, &mut HostReckoningNumeric),
            Err(PhysicalPlayerPrepareVelocityError::Source(fail))
        );
        assert_eq!(bits(state.velocity().unwrap()), bits(source.velocity));
        let spin_written = ["count", "capture3", "publish"].contains(&fail);
        assert_eq!(
            state.last_spin().unwrap().to_bits(),
            if spin_written { 1.0_f32.to_bits() } else { 0 }
        );
        assert!(source.published_velocity.is_none());
        assert!(!state.complete());
        assert_eq!(source.calls.last(), Some(&fail));
    }
}
#[test]
fn decay_uses_original_fixed_step_full4_and_preserves_unknown_w() {
    for w in [Some(2.0), None] {
        let mut state = initialized();
        let mut source = Source::new();
        source.prepare = false;
        source.surface.w = w;
        state
            .try_update(&mut source, &mut HostReckoningNumeric)
            .unwrap();
        let expected_y = (-f32::from_bits(0x411c_cccd)).mul_add(f32::from_bits(0x3c88_8889), 0.0);
        assert_eq!(
            state.velocity().unwrap().xyz[1].to_bits(),
            expected_y.to_bits()
        );
        assert_eq!(
            state.velocity().unwrap().w.map(f32::to_bits),
            w.map(|w| (w * -f32::from_bits(0x411c_cccd))
                .mul_add(f32::from_bits(0x3c88_8889), 0.0)
                .to_bits())
        );
        assert!(!source.calls.contains(&"velocity"));
    }
}
#[test]
fn ordered_positive_spin_wraps_but_zero_and_unordered_reset() {
    for (old, spin, expected) in [
        (1.0, 1.0, 8),
        (-1.0, -1.0, 8),
        (1.0, -1.0, 0),
        (f32::INFINITY, 0.0, 0),
        (1.0, f32::NAN, 0),
    ] {
        let mut state = initialized();
        state.last_spin = Some(old);
        state.same_direction_frames = Some(7);
        let mut source = Source::new();
        source.spin = spin;
        state
            .try_update(&mut source, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(state.same_direction_frames(), Some(expected));
        assert_eq!(state.last_spin().unwrap().to_bits(), spin.to_bits());
    }
    let mut state = initialized();
    state.last_spin = Some(1.0);
    state.same_direction_frames = Some(u32::MAX);
    let mut source = Source::new();
    state
        .try_update(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(state.same_direction_frames(), Some(0));
}
#[test]
fn normalize_failure_precedes_history_store_and_zero_fallback_is_full4_known() {
    struct Fail;
    impl ReckoningNumeric for Fail {
        type Error = &'static str;
        fn dot3(&mut self, _: [f32; 4], _: [f32; 4]) -> Result<f32, Self::Error> {
            Err("dot")
        }
        fn reciprocal_sqrt_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("after dot refusal")
        }
        fn reciprocal_estimate(&mut self, _: f32) -> Result<f32, Self::Error> {
            panic!("not used")
        }
        fn truncate_i32(&mut self, _: f32) -> Result<i32, Self::Error> {
            panic!("not used")
        }
    }
    let mut state = initialized();
    let mut source = Source::new();
    source.prepare = false;
    assert_eq!(
        state.try_update(&mut source, &mut Fail),
        Err(PhysicalPlayerPrepareVelocityError::Numeric("dot"))
    );
    assert_eq!(bits(state.velocity().unwrap()), ([0; 3], Some(0)));
    assert!(!source.calls.contains(&"spin"));
    let mut source = Source::new();
    source.prepare = false;
    source.surface = SkeletonRootVector {
        xyz: [0.0; 3],
        w: None,
    };
    state
        .try_update(&mut source, &mut HostReckoningNumeric)
        .unwrap();
    assert_eq!(bits(state.velocity().unwrap()), ([0; 3], Some(0)));
}
