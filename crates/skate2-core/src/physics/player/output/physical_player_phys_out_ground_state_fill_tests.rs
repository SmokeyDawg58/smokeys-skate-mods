use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
#[derive(Default)]
struct Source {
    calls: Vec<String>,
    fail_vector: bool,
    fail_second: bool,
    absolute_speed: Option<f32>,
}
impl GroundStateFillSource for Source {
    type Error = &'static str;
    type Pointer = u64;
    fn flag(&mut self, f: GroundFillFlag) -> Result<bool, Self::Error> {
        self.calls.push(format!("{f:?}"));
        Ok(true)
    }
    fn scalar(&mut self, f: GroundFillScalar) -> Result<f32, Self::Error> {
        self.calls.push(format!("{f:?}"));
        Ok(if matches!(f, GroundFillScalar::AbsoluteSpeed) {
            self.absolute_speed.unwrap_or(1.0)
        } else {
            1.0
        })
    }
    fn capture(&mut self, k: u64) -> Result<u64, Self::Error> {
        self.calls.push(format!("capture{k:x}"));
        if self.fail_second && k == 0x1e76_e849_db99_f955 {
            Err("second")
        } else {
            Ok(k)
        }
    }
    fn load(&mut self, k: &u64) -> Result<f32, Self::Error> {
        self.calls.push(format!("load{k:x}"));
        Ok(2.0)
    }
    fn anti_flip_torque(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.calls.push("vector".into());
        if self.fail_vector {
            Err("vector")
        } else {
            Ok(SkeletonRootVector {
                xyz: [1.0, 2.0, 3.0],
                w: Some(-0.0),
            })
        }
    }
    fn grab_type(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("type".into());
        Ok(2)
    }
    fn grab_object(&mut self) -> Result<u32, Self::Error> {
        self.calls.push("object".into());
        Ok(u32::MAX)
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn missing_torque_keeps_wall_exit_and_skips_later_reads() {
    let mut out = output();
    let mut source = Source {
        fail_vector: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_fill_ground_state(&mut source),
        Err(GroundStateFillError::Source("vector"))
    ));
    assert!(out.concrete.motion.as_ref().unwrap().is_push_accelerating);
    assert!(out.concrete.motion.as_ref().unwrap().is_at_pushable_speed);
    assert!(out.concrete.ground.as_ref().unwrap().wall_ride_exit);
    assert!(
        !out.concrete
            .ground
            .as_ref()
            .unwrap()
            .anti_flip_nudge_present
    );
    assert_eq!(
        source.calls,
        [
            "PushAccelerating",
            "capturea9b532ebbed16f38",
            "capture1e76e849db99f955",
            "load1e76e849db99f955",
            "loada9b532ebbed16f38",
            "AbsoluteSpeed",
            "Pinning",
            "WallRideExit",
            "vector"
        ]
    );
}
#[test]
fn second_capture_failure_preserves_first_push_store_without_loading_first() {
    let mut out = output();
    let mut source = Source {
        fail_second: true,
        ..Source::default()
    };
    assert!(matches!(
        out.try_fill_ground_state(&mut source),
        Err(GroundStateFillError::Source("second"))
    ));
    assert!(out.concrete.motion.as_ref().unwrap().is_push_accelerating);
    assert_eq!(source.calls.len(), 3);
}
#[test]
fn locked_output_rejects_before_any_ground_read() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut source = Source::default();
    assert!(matches!(
        out.try_fill_ground_state(&mut source),
        Err(GroundStateFillError::Output(_))
    ));
    assert!(source.calls.is_empty());
}
#[test]
fn full_vector_and_two_word_descriptor_are_published() {
    let mut out = output();
    let mut source = Source::default();
    out.try_fill_ground_state(&mut source).unwrap();
    assert_eq!(
        out.concrete
            .ground
            .as_ref()
            .unwrap()
            .anti_flip_torque
            .w
            .unwrap()
            .to_bits(),
        0x8000_0000
    );
    let desc = &out.concrete.state.as_ref().unwrap().grab_spline_desc;
    assert_eq!((desc.object_type, desc.object_id), (2, -1));
    assert!(out.concrete.intents.as_ref().unwrap().has_world_grab_intent);
    assert_eq!(
        &source.calls[source.calls.len() - 3..],
        ["GrabObject", "type", "object"]
    );
}

#[test]
fn push_speed_limit_is_strict_and_nan_does_not_become_pushable() {
    for (speed, expected) in [(3.999, true), (4.0, false), (f32::NAN, false)] {
        let mut out = output();
        let mut source = Source {
            absolute_speed: Some(speed),
            ..Source::default()
        };
        out.try_fill_ground_state(&mut source).unwrap();
        assert_eq!(
            out.concrete.motion.as_ref().unwrap().is_at_pushable_speed,
            expected
        );
    }
}
