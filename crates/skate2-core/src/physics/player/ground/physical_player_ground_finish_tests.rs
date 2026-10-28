//! Tests using actual retained Ground fields.
use super::PhysicsGroundFields;
use crate::simulation_assembly_backend::constraint_session::ground_finish::{
    GroundFinishSource, try_finish_ground,
};
struct Source {
    events: Vec<&'static str>,
    active: bool,
    fail_dt: bool,
}
impl GroundFinishSource for Source {
    type Error = &'static str;
    fn request_push_exit(&mut self) -> Result<(), Self::Error> {
        self.events.push("push");
        Ok(())
    }
    fn mark_add_skateboard_error(&mut self) -> Result<(), Self::Error> {
        self.events.push("error");
        Ok(())
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        if self.fail_dt { Err("dt") } else { Ok(0.25) }
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.events.push("flags");
        Ok(if self.active { 0x0100_0000 } else { 1 })
    }
    fn invalidate_grab_query(&mut self) -> Result<(), Self::Error> {
        self.events.push("invalidate");
        Ok(())
    }
    fn check_for_grabbable_object(
        &mut self,
        _: &mut PhysicsGroundFields,
    ) -> Result<(), Self::Error> {
        self.events.push("search");
        Ok(())
    }
}
#[test]
fn real_high_byte_branch_searches_after_prefix_and_does_not_invalidate() {
    let mut g = PhysicsGroundFields::new(1);
    g.request_push_stop = true;
    g.time_in_state = 1.0;
    let mut s = Source {
        events: Vec::new(),
        active: true,
        fail_dt: false,
    };
    try_finish_ground(&mut g, &mut s).unwrap();
    assert_eq!(g.time_in_state.to_bits(), 1.25_f32.to_bits());
    assert_eq!(s.events, ["push", "error", "dt", "flags", "search"]);
    s.active = false;
    s.events.clear();
    g.request_push_stop = false;
    try_finish_ground(&mut g, &mut s).unwrap();
    assert_eq!(s.events, ["error", "dt", "flags", "invalidate"]);
}
#[test]
fn late_dt_failure_preserves_earlier_skeleton_calls_and_old_timer() {
    let mut g = PhysicsGroundFields::new(1);
    g.request_push_stop = true;
    g.time_in_state = 4.0;
    let mut s = Source {
        events: Vec::new(),
        active: false,
        fail_dt: true,
    };
    assert!(try_finish_ground(&mut g, &mut s).is_err());
    assert_eq!(s.events, ["push", "error", "dt"]);
    assert_eq!(g.time_in_state.to_bits(), 4.0_f32.to_bits());
}
