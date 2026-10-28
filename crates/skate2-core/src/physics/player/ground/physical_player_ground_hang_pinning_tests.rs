//! Proposed test child of `physics_state`, retaining its actual private ctor.
use super::PhysicsGroundFields;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::ground_hang_pinning::{
        GroundHangPinSource, HangPinError, HangPinPending, try_consider_pinning,
        try_hang_and_halfpipe,
    },
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
struct Source {
    hang: i32,
    wipeout: i32,
    frames: i32,
    flags: u32,
    events: Vec<&'static str>,
    velocity: Option<Vector>,
    fail_velocity: bool,
}
impl Source {
    fn new() -> Self {
        Self {
            hang: 20,
            wipeout: 20,
            frames: 0,
            flags: 0x0100_0000,
            events: Vec::new(),
            velocity: None,
            fail_velocity: false,
        }
    }
}
impl GroundHangPinSource for Source {
    type Error = &'static str;
    fn hang_flags(&mut self) -> Result<u32, Self::Error> {
        self.events.push("hang_flags");
        Ok(0)
    }
    fn hang_frames(&mut self) -> Result<i32, Self::Error> {
        self.events.push("hang_frames");
        Ok(self.hang)
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.events.push("speed");
        Ok(f32::NAN)
    }
    fn wipeout_frames(&mut self) -> Result<i32, Self::Error> {
        self.events.push("wipeout");
        Ok(self.wipeout)
    }
    fn teleport_frames(&mut self) -> Result<i32, Self::Error> {
        self.events.push("teleport");
        Ok(self.frames)
    }
    fn maximum_pin_frames(&mut self) -> Result<i32, Self::Error> {
        self.events.push("pin_max");
        Ok(90)
    }
    fn maximum_pin_start_frames(&mut self) -> Result<i32, Self::Error> {
        self.events.push("pin_start");
        Ok(30)
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.events.push("controls");
        Ok(self.flags)
    }
    fn time_delta(&mut self) -> Result<f32, Self::Error> {
        self.events.push("dt");
        Ok(0.5)
    }
    fn deck_y(&mut self) -> Result<f32, Self::Error> {
        self.events.push("y");
        Ok(3.0)
    }
    fn deck_position(&mut self) -> Result<Vector, Self::Error> {
        self.events.push("position");
        Ok(Vector {
            xyz: [1.0, 3.0, 2.0],
            w: None,
        })
    }
    fn set_velocity(&mut self, v: Vector) -> Result<(), Self::Error> {
        self.events.push("velocity");
        if self.fail_velocity {
            return Err("velocity");
        }
        self.velocity = Some(v);
        Ok(())
    }
}
#[test]
fn constructor_flags_still_read_live_thresholds_and_decrement_old_pulse() {
    let mut g = PhysicsGroundFields::new(1);
    g.finish_enter();
    g.hang_pulse_countdown = Some(2);
    g.hung_up_counter = 9;
    let mut s = Source::new();
    try_hang_and_halfpipe::<_, ()>(&mut g, &mut s).unwrap();
    assert_eq!(g.hang_pulse_countdown, Some(1));
    assert_eq!(g.hung_up_counter, 0);
    assert_eq!(
        s.events,
        [
            "hang_flags",
            "hang_frames",
            "hang_flags",
            "speed",
            "wipeout",
            "hang_flags"
        ]
    );
}
#[test]
fn mutable_thresholds_cannot_turn_into_fabricated_idle_success() {
    let mut g = PhysicsGroundFields::new(1);
    g.finish_enter();
    g.hang_timer = Some(8);
    let mut s = Source::new();
    s.hang = 0;
    assert!(matches!(
        try_hang_and_halfpipe::<_, ()>(&mut g, &mut s),
        Err(HangPinError::Pending(HangPinPending::PulseProducer))
    ));
    assert_eq!(g.hang_timer, Some(0));
    assert_eq!(s.events, ["hang_flags", "hang_frames"]);
    s = Source::new();
    s.wipeout = -1;
    g.hung_up_counter = 9;
    assert!(matches!(
        try_hang_and_halfpipe::<_, ()>(&mut g, &mut s),
        Err(HangPinError::Pending(HangPinPending::Wipeout))
    ));
    assert_eq!(g.hung_up_counter, 0);
}
#[test]
fn pinning_preserves_unknown_w_and_skips_start_cvar_after_previous_pin() {
    let mut g = PhysicsGroundFields::new(1);
    g.spawn_coordinates_set = true;
    g.was_pinning = true;
    g.valid_spawn_x = 5.0;
    g.valid_spawn_z = 7.0;
    let mut s = Source::new();
    s.frames = 1;
    try_consider_pinning(&mut g, &mut s, &mut HostReckoningNumeric).unwrap();
    let v = s.velocity.unwrap();
    assert_eq!(
        v.xyz.map(f32::to_bits),
        [8.0_f32, 0.0, 10.0].map(f32::to_bits)
    );
    assert_eq!(v.w, None);
    assert!(g.is_pinning && g.was_pinning);
    assert_eq!(
        s.events,
        [
            "teleport", "pin_max", "controls", "dt", "y", "position", "velocity"
        ]
    );
}
#[test]
fn failed_velocity_does_not_publish_pinning_and_nonhuman_still_reads_frames() {
    let mut g = PhysicsGroundFields::new(1);
    g.spawn_coordinates_set = true;
    let mut s = Source::new();
    s.fail_velocity = true;
    assert!(try_consider_pinning(&mut g, &mut s, &mut HostReckoningNumeric).is_err());
    assert!(!g.is_pinning && !g.was_pinning);
    g.is_human = 0;
    g.is_pinning = true;
    s.events.clear();
    try_consider_pinning(&mut g, &mut s, &mut HostReckoningNumeric).unwrap();
    assert!(g.was_pinning);
    assert_eq!(s.events, ["teleport"]);
}
