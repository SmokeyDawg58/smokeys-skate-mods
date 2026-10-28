//! Proposed test child of `physical_player_physics_state` (uses its real ctor).
use super::PhysicsGroundFields;
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::antiflip_nudge::{
        NudgeSource, try_update_nudge,
    },
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_queue::SkateboardForceQueue,
    skeleton_root_transforms::SkeletonRootVector as Vector,
};
struct Source {
    minimum: i32,
    speed: f32,
    missing: Option<&'static str>,
    events: Vec<&'static str>,
}
impl Source {
    fn event(&mut self, s: &'static str) -> Result<(), &'static str> {
        self.events.push(s);
        if self.missing == Some(s) {
            Err(s)
        } else {
            Ok(())
        }
    }
}
impl NudgeSource for Source {
    type Error = &'static str;
    type Target = ();
    fn minimum_frames(&mut self) -> Result<i32, Self::Error> {
        self.event("minimum")?;
        Ok(self.minimum)
    }
    fn deck_speed(&mut self) -> Result<f32, Self::Error> {
        self.event("speed")?;
        Ok(self.speed)
    }
    fn maximum_speed(&mut self) -> Result<f32, Self::Error> {
        self.event("maximum")?;
        Ok(1.1)
    }
    fn deck_z(&mut self) -> Result<Vector, Self::Error> {
        self.event("z")?;
        Ok(Vector {
            xyz: [3.0, 4.0, 0.0],
            w: None,
        })
    }
    fn capture_strength(&mut self) -> Result<(), Self::Error> {
        self.event("capture")
    }
    fn load_strength(&mut self, (): &()) -> Result<f32, Self::Error> {
        self.event("strength")?;
        Ok(50.0)
    }
    fn debug(&mut self) -> Result<bool, Self::Error> {
        self.event("debug")?;
        Ok(true)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.event("overlay")
    }
}
fn source() -> Source {
    Source {
        minimum: 12,
        speed: 0.0,
        missing: None,
        events: vec![],
    }
}
fn ground() -> PhysicsGroundFields {
    let mut g = PhysicsGroundFields::new(1);
    g.anti_flip_torque = Some(Vector::known([1.0, 0.0, 0.0, 0.0]));
    g.anti_flip_frames = 12;
    g
}
#[test]
fn actual_nudge_appends_then_publishes_flag_before_debug_failure() {
    let mut g = ground();
    let mut s = source();
    s.missing = Some("overlay");
    let mut q = SkateboardForceQueue::new();
    assert!(try_update_nudge(&mut g, &mut s, &mut q, &mut HostReckoningNumeric).is_err());
    assert!(g.anti_flip_nudge);
    assert_eq!(g.anti_flip_frames, 13);
    assert_eq!(q.len(), 1);
    let record = q.records_for_test()[0];
    assert_eq!(record.kind, 0x11);
    assert!(record.force.world_space_force.w.is_none());
    assert!((record.force.world_space_force.xyz[0] + 50.0).abs() < 0.0001);
    assert_eq!(
        record
            .force
            .local_space_position
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert_eq!(
        s.events,
        [
            "minimum", "speed", "maximum", "z", "capture", "strength", "debug", "overlay"
        ]
    );
}
#[test]
fn native_full_queue_still_publishes_nudge_and_runs_debug() {
    let mut g = ground();
    let mut s = source();
    let mut q = SkateboardForceQueue::new();
    let zero = Vector::known([0.0; 4]);
    for _ in 0..21 {
        q.add(
            2,
            SkateboardFootForce {
                world_space_force: zero,
                local_space_position: zero,
            },
        );
    }
    try_update_nudge(&mut g, &mut s, &mut q, &mut HostReckoningNumeric).unwrap();
    assert_eq!(q.len(), 21);
    assert!(g.anti_flip_nudge);
    assert_eq!(s.events.last(), Some(&"overlay"));
}
#[test]
fn nan_torque_resets_counter_before_minimum_read_without_clearing_old_flag() {
    let mut g = ground();
    g.anti_flip_torque = Some(Vector::known([f32::NAN, 0.0, 0.0, 0.0]));
    g.anti_flip_nudge = true;
    let mut s = source();
    let mut q = SkateboardForceQueue::new();
    try_update_nudge(&mut g, &mut s, &mut q, &mut HostReckoningNumeric).unwrap();
    assert_eq!(g.anti_flip_frames, 0);
    assert!(g.anti_flip_nudge);
    assert_eq!(q.len(), 0);
    assert_eq!(s.events, ["minimum"]);
}
#[test]
fn counter_wraps_and_unordered_speed_skips_later_reads() {
    let mut g = ground();
    g.anti_flip_frames = i32::MAX;
    let mut s = source();
    let mut q = SkateboardForceQueue::new();
    try_update_nudge(&mut g, &mut s, &mut q, &mut HostReckoningNumeric).unwrap();
    assert_eq!(g.anti_flip_frames, i32::MIN);
    let mut g = ground();
    let mut s = source();
    s.speed = f32::NAN;
    try_update_nudge(&mut g, &mut s, &mut q, &mut HostReckoningNumeric).unwrap();
    assert_eq!(s.events, ["minimum", "speed", "maximum"]);
    assert_eq!(q.len(), 0);
}
