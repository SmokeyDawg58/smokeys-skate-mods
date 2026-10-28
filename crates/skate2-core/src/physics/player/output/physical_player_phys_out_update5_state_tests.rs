use super::*;
use crate::physical_player_phys_out::construct::PhysOutAllocation;
struct Source {
    types: std::collections::VecDeque<i32>,
    calls: Vec<&'static str>,
    fail_dt: bool,
}
impl Update5StateSource for Source {
    type Error = &'static str;
    fn current_type(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("type");
        Ok(self.types.pop_front().unwrap())
    }
    fn update_count(&mut self) -> Result<i32, Self::Error> {
        self.calls.push("count");
        Ok(27)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.calls.push("dt");
        if self.fail_dt { Err("dt") } else { Ok(-0.0) }
    }
}
fn output() -> PhysicalPlayerPhysOut {
    let mut out = PhysicalPlayerPhysOut::new();
    out.try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
        .unwrap();
    out
}
#[test]
fn spans_reread_actual_type_and_do_not_replace_earlier_type_store() {
    let mut out = output();
    let mut s = Source {
        types: [100, 504].into(),
        calls: vec![],
        fail_dt: false,
    };
    out.try_publish_update5_current_type(&mut s).unwrap();
    out.try_publish_update5_category_and_dt(&mut s).unwrap();
    let state = out.concrete.state.as_ref().unwrap();
    assert_eq!(
        (
            state.physics_state,
            state.physics_state_category,
            state.update_count
        ),
        (100, 500, 27)
    );
    assert!(state.is_offboard && state.is_offboard_ground);
    assert!(!out.concrete.air.as_ref().unwrap().is_skateboard_airborne);
    assert_eq!(
        out.concrete.physics.as_ref().unwrap().timestep.to_bits(),
        0x8000_0000
    );
    assert_eq!(s.calls, ["type", "type", "count", "dt"]);
}
#[test]
fn late_dt_failure_keeps_air_category_and_count_publications() {
    let mut out = output();
    let mut s = Source {
        types: [200].into(),
        calls: vec![],
        fail_dt: true,
    };
    assert!(matches!(
        out.try_publish_update5_category_and_dt(&mut s),
        Err(Update5StateOutputError::Source("dt"))
    ));
    assert!(out.concrete.air.as_ref().unwrap().is_skateboard_airborne);
    assert_eq!(
        out.concrete.state.as_ref().unwrap().physics_state_category,
        200
    );
    assert_eq!(out.concrete.state.as_ref().unwrap().update_count, 27);
}
#[test]
fn type_capture_precedes_locked_output_assertion() {
    let mut out = output();
    out.lock_after_update1().unwrap();
    let mut s = Source {
        types: [100].into(),
        calls: vec![],
        fail_dt: false,
    };
    assert!(matches!(
        out.try_publish_update5_current_type(&mut s),
        Err(Update5StateOutputError::Output(_))
    ));
    assert_eq!(s.calls, ["type"]);
    assert_eq!(out.concrete.state.as_ref().unwrap().physics_state, 0);
}
#[test]
fn category_uses_native_signed_assertion_and_ranges() {
    for (value, expected) in [
        (0, 0),
        (99, 0),
        (100, 100),
        (199, 100),
        (200, 200),
        (504, 500),
        (699, 600),
        (702, 700),
    ] {
        assert_eq!(category::<()>(value).unwrap(), expected);
    }
    for value in [-1, 703] {
        assert!(
            matches!(category::<()>(value),Err(Update5StateOutputError::InvalidType(actual))if actual==value)
        );
    }
}
