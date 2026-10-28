use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
#[derive(Default)]
struct Source {
    calls: Vec<String>,
    missing_backend: bool,
    debug_sim: bool,
    captures: u32,
}
impl Update5OuterLocalitySource for Source {
    type Error = &'static str;
    type Processed = u32;
    fn processed_flag(&mut self, offset: u32, bit: u32) -> Result<bool, Self::Error> {
        self.calls.push(format!("{offset:x}:{bit}"));
        Ok(true)
    }
    fn debug_render_local_sim_player(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("debug".into());
        Ok(self.debug_sim)
    }
    fn is_local_player(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("local".into());
        if self.missing_backend {
            Err("backend")
        } else {
            Ok(true)
        }
    }
    fn is_local_sim_player(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("local_sim".into());
        if self.missing_backend {
            Err("backend")
        } else {
            Ok(true)
        }
    }
    fn state_count(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("count".into());
        Ok(19)
    }
    fn collide_with_world_only(&mut self) -> Result<bool, Self::Error> {
        self.calls.push("collide".into());
        Ok(true)
    }
    fn turn(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("turn".into());
        Ok(-0.0)
    }
    fn capture_processed(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("capture".into());
        self.captures += 1;
        Ok(self.captures)
    }
    fn ground_position(&mut self, owner: &u32) -> Result<SkeletonRootVector, Self::Error> {
        assert_eq!(*owner, 1);
        self.calls.push("ground".into());
        Ok(SkeletonRootVector::known([2.0; 4]))
    }
    fn captured_flag(&mut self, owner: &u32, offset: u32, bit: u32) -> Result<bool, Self::Error> {
        assert_eq!((*owner, offset, bit), (2, 0x8d0, 15));
        self.calls.push("captured_hippy".into());
        Ok(true)
    }
    fn jump_strength(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("jump".into());
        Ok(0.75)
    }
    fn balance(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("balance".into());
        Ok(f32::NAN)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn source_order_fresh_hippy_capture_and_nan_manual() {
    for debug_sim in [false, true] {
        let mut out = output();
        let mut source = Source {
            debug_sim,
            ..Source::default()
        };
        out.try_publish_update5_outer_locality(&mut source).unwrap();
        let expected = [
            "8c4:29",
            "8c4:30",
            "8c4:25",
            "8c4:27",
            "8c4:26",
            "8c4:28",
            "debug",
            if debug_sim { "local_sim" } else { "local" },
            "count",
            "collide",
            "8cc:26",
            "8c8:12",
            "8c4:21",
            "8c4:22",
            "turn",
            "capture",
            "ground",
            "capture",
            "captured_hippy",
            "jump",
            "8c4:18",
            "balance",
        ];
        assert_eq!(source.calls, expected);
        let state = out.concrete.state.as_ref().unwrap();
        assert!(state.is_manual && state.is_debug_render_player && state.is_wiping_out);
        assert_eq!(state.state_update_count, 19);
        assert_eq!(
            out.concrete.ground.as_ref().unwrap().turn.to_bits(),
            (-0.0f32).to_bits()
        );
        assert_eq!(
            out.concrete
                .ground
                .as_ref()
                .unwrap()
                .jump_strength
                .to_bits(),
            0.75_f32.to_bits()
        );
    }
}
#[test]
fn missing_backend_preserves_six_completed_flag_stores() {
    let mut out = output();
    let mut source = Source {
        missing_backend: true,
        debug_sim: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_publish_update5_outer_locality(&mut source),
        Err(Update5OuterLocalityError::Source("backend"))
    ));
    let state = out.concrete.state.as_ref().unwrap();
    assert!(
        state.is_manual_braking
            && state.is_foot_braking
            && state.is_physically_pushing
            && state.is_generally_pushing
            && state.is_generally_pushing_right_foot
            && state.is_generally_braking
    );
    assert!(!state.is_debug_render_player);
    assert_eq!(state.state_update_count, 0);
    assert_eq!(source.calls.last().unwrap(), "local_sim");
}
#[test]
fn first_source_flag_read_precedes_locked_output_failure() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        out.try_publish_update5_outer_locality(&mut source),
        Err(Update5OuterLocalityError::Output(_))
    ));
    assert_eq!(source.calls, ["8c4:29"]);
}
