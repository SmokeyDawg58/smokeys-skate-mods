//! `Toolkit_CalcPushAcceleration82b272f8`; r3 is the force result, r4 the board.
//! Unregistered draft: the caller retains this output until its later queue add.
use crate::{
    skateboard_foot_forces::SkateboardFootForce, skeleton_root_transforms::SkeletonRootVector,
};

#[path = "skateboard_push_attributes.rs"]
pub(crate) mod attributes;

pub(crate) const CHANGE_LIMIT: u64 = 0x76fe_ee54_a3ad_2af5;
pub(crate) const SPEED_LIMIT: u64 = 0xa9b5_32eb_bed1_6f38;

#[derive(Clone, Copy, Debug)]
pub(crate) enum PushScalar {
    AnimationSpeed,
    ForwardSpeed,
    AbsoluteSpeed,
    Mass,
    TimeDelta,
}

/// Methods are source load sites, including capture versus dereference of the
/// second tuning pointer. Abort writes the existing board flag owner.
pub(crate) trait PushSource {
    type Error;
    type Pointer;
    fn pushing(&mut self) -> Result<bool, Self::Error>;
    fn dont_push(&mut self) -> Result<bool, Self::Error>;
    fn request_abort_push(&mut self) -> Result<(), Self::Error>;
    fn scalar(&mut self, field: PushScalar) -> Result<f32, Self::Error>;
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn load(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn direction(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

pub(crate) fn try_calculate<S: PushSource>(
    source: &mut S,
    output: &mut SkateboardFootForce,
) -> Result<(), S::Error> {
    let trace = std::env::var_os("SKATE2_TRACE_PUSH_FORCE").is_some();
    // 27348/2734c local position (the same +0 twice), then 27350 force.
    output.local_space_position = SkeletonRootVector::known([0.0; 4]);
    output.world_space_force = SkeletonRootVector::known([0.0; 4]);
    if !source.pushing()? {
        if trace {
            eprintln!("push calculation: not pushing");
        }
        return Ok(());
    }
    if source.dont_push()? {
        if trace {
            eprintln!("push calculation: dont_push gate");
        }
        source.request_abort_push()?;
        return Ok(());
    }
    let animation_speed = source.scalar(PushScalar::AnimationSpeed)?;
    let forward_speed = source.scalar(PushScalar::ForwardSpeed)?;
    let difference = animation_speed - forward_speed;
    // 2739c tests GT: unordered and both signed zeros take the +0 store.
    let mut change = if difference > 0.0 { difference } else { 0.0 };
    let pointer = source.capture(CHANGE_LIMIT)?;
    let limit = source.load(&pointer)?;
    if trace {
        eprintln!("push calculation: raw_change={change} change_limit={limit}");
    }
    let above = change > limit;
    let negative_limit = -limit;
    let below = change < negative_limit;
    if above {
        change = limit;
    }
    // Both comparisons used the original value. This is not an else-if clamp.
    if below {
        change = negative_limit;
    }
    let speed = source.scalar(PushScalar::AbsoluteSpeed)?;
    let pointer = source.capture(SPEED_LIMIT)?;
    let proposed_speed = speed + change; // 274b8 precedes pointer lfs274bc.
    let limit = source.load(&pointer)?;
    // 4099000c at274c4 is NOT GT; unordered bypasses the correction.
    if proposed_speed > limit {
        let remaining = limit - speed;
        change = if remaining >= 0.0 { remaining } else { 0.0 };
    }
    let mass = source.scalar(PushScalar::Mass)?;
    let impulse = mass * change;
    let dt = source.scalar(PushScalar::TimeDelta)?;
    let direction = source.direction()?; // 274ec before scalar divide274f0.
    let force = impulse / dt;
    if trace {
        eprintln!(
            "push calculation: animation_speed={animation_speed} forward_speed={forward_speed} change={change} absolute_speed={speed} speed_limit={limit} mass={mass} dt={dt} direction={:?} force={force}",
            direction.xyz
        );
    }
    output.world_space_force = SkeletonRootVector {
        xyz: direction.xyz.map(|lane| lane * force),
        w: direction.w.map(|lane| lane * force),
    };
    Ok(())
}

#[cfg(test)]
#[path = "skateboard_push_acceleration_tests.rs"]
mod tests;
