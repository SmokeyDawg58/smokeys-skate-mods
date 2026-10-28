use super::{CHANGE_LIMIT, PushScalar, PushSource, try_calculate};
use crate::{
    skateboard_foot_forces::SkateboardFootForce, skeleton_root_transforms::SkeletonRootVector,
};

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent source branch test controls"
)]
struct Input {
    pushing: bool,
    blocked: bool,
    aborted: bool,
    values: [f32; 5],
    change_limit: f32,
    speed_limit: f32,
    direction: SkeletonRootVector,
    fail_direction: bool,
    events: Vec<&'static str>,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            pushing: true,
            blocked: false,
            aborted: false,
            values: [9.0, 3.0, 3.0, 2.0, 0.5],
            change_limit: 4.0,
            speed_limit: 20.0,
            direction: SkeletonRootVector {
                xyz: [1.0, -2.0, 3.0],
                w: None,
            },
            fail_direction: false,
            events: Vec::new(),
        }
    }
}
impl PushSource for Input {
    type Error = &'static str;
    type Pointer = u64;
    fn pushing(&mut self) -> Result<bool, Self::Error> {
        self.events.push("push");
        Ok(self.pushing)
    }
    fn dont_push(&mut self) -> Result<bool, Self::Error> {
        self.events.push("blocked");
        Ok(self.blocked)
    }
    fn request_abort_push(&mut self) -> Result<(), Self::Error> {
        self.events.push("abort");
        self.aborted = true;
        Ok(())
    }
    fn scalar(&mut self, field: PushScalar) -> Result<f32, Self::Error> {
        let (i, name) = match field {
            PushScalar::AnimationSpeed => (0, "animation"),
            PushScalar::ForwardSpeed => (1, "forward"),
            PushScalar::AbsoluteSpeed => (2, "absolute"),
            PushScalar::Mass => (3, "mass"),
            PushScalar::TimeDelta => (4, "dt"),
        };
        self.events.push(name);
        Ok(self.values[i])
    }
    fn capture(&mut self, key: u64) -> Result<u64, Self::Error> {
        self.events.push(if key == CHANGE_LIMIT {
            "capture change"
        } else {
            "capture speed"
        });
        Ok(key)
    }
    fn load(&mut self, pointer: &u64) -> Result<f32, Self::Error> {
        self.events.push(if *pointer == CHANGE_LIMIT {
            "load change"
        } else {
            "load speed"
        });
        Ok(if *pointer == CHANGE_LIMIT {
            self.change_limit
        } else {
            self.speed_limit
        })
    }
    fn direction(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        self.events.push("direction");
        if self.fail_direction {
            Err("direction")
        } else {
            Ok(self.direction)
        }
    }
}
fn output() -> SkateboardFootForce {
    SkateboardFootForce {
        world_space_force: SkeletonRootVector::known([99.0; 4]),
        local_space_position: SkeletonRootVector::known([88.0; 4]),
    }
}
fn assert_zero(vector: SkeletonRootVector) {
    assert_eq!(vector.xyz.map(f32::to_bits), [0; 3]);
    assert_eq!(vector.w.map(f32::to_bits), Some(0));
}
#[test]
fn lazy_gates_clear_outputs_and_publish_actual_abort_only_when_blocked() {
    let mut input = Input {
        pushing: false,
        ..Input::default()
    };
    let mut force = output();
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(input.events, ["push"]);
    assert_zero(force.world_space_force);
    assert_zero(force.local_space_position);
    input.pushing = true;
    input.blocked = true;
    input.events.clear();
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(input.events, ["push", "blocked", "abort"]);
    assert!(input.aborted);
}
#[test]
fn live_order_and_full_lane_knowledge_survive_scaling() {
    let mut input = Input::default();
    let mut force = output();
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(
        force.world_space_force.xyz.map(f32::to_bits),
        [16.0_f32, -32.0, 48.0].map(f32::to_bits)
    );
    assert_eq!(force.world_space_force.w, None);
    assert_eq!(
        input.events,
        [
            "push",
            "blocked",
            "animation",
            "forward",
            "capture change",
            "load change",
            "absolute",
            "capture speed",
            "load speed",
            "mass",
            "dt",
            "direction"
        ]
    );
    input.direction.w = Some(-0.0);
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(
        force.world_space_force.w.map(f32::to_bits),
        Some((-0.0_f32).to_bits())
    );
}
#[test]
fn negative_limit_uses_two_original_comparisons_and_nan_difference_clears() {
    let mut input = Input {
        change_limit: -4.0,
        values: [5.0, 3.0, 3.0, 2.0, 0.5],
        ..Input::default()
    };
    let mut force = output();
    try_calculate(&mut input, &mut force).unwrap();
    // original change2 is both above-4 and below+4; lower store wins.
    assert_eq!(force.world_space_force.xyz[0].to_bits(), 16.0_f32.to_bits());
    input.values[0] = f32::NAN;
    input.change_limit = 4.0;
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(force.world_space_force.xyz[0].to_bits(), 0);
    input.values[0] = 9.0;
    input.speed_limit = f32::NAN;
    try_calculate(&mut input, &mut force).unwrap();
    assert_eq!(force.world_space_force.xyz[0].to_bits(), 16.0_f32.to_bits());
}
#[test]
fn source_zero_writes_survive_missing_late_direction() {
    let mut input = Input {
        fail_direction: true,
        ..Input::default()
    };
    let mut force = output();
    assert_eq!(try_calculate(&mut input, &mut force), Err("direction"));
    assert_zero(force.world_space_force);
    assert_zero(force.local_space_position);
    assert_eq!(input.events.last(), Some(&"direction"));
}
