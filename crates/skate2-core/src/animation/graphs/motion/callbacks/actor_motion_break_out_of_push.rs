//! `BreakOutOfPush::ActivationFunc82c9baa8` on retained graph and animation owners.
use super::{
    ActorConstruction, ActorIdentity, MotionUpdateError as Error,
    play_animation::PlayAnimationActor,
};
use crate::{
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation_lifecycle::{
        PlayAnimationInterfaceKind, PlayAnimationInterfaceResolution, PlayAnimationLifecycleBackend,
    },
};

/// Raw `fmuls`, `fcmpu`, `blt` at 82c9bc34..48: unordered time is not less.
/// The final bit test at 82c9bc4c..60 checks `PushState::mContinuePush`.
fn reached_breakout_time<E>(
    time: f32,
    threshold: f32,
    continue_push: impl FnOnce() -> Result<bool, E>,
) -> Result<bool, E> {
    if time < threshold {
        return Ok(false);
    }
    Ok(!continue_push()?)
}

impl ActorConstruction {
    pub(crate) fn motion_runtime_break_out_of_push(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, Error> {
        let mut source = PlayAnimationActor {
            actor: self,
            context,
        };
        if source.resolve_interface(PlayAnimationInterfaceKind::Animatable)?
            != PlayAnimationInterfaceResolution::Resolved
        {
            return Err(Error::ForeignOwner("BreakOutOfPush IAnimatable"));
        }
        if source.resolve_interface(PlayAnimationInterfaceKind::SkaterMotionGraph)?
            != PlayAnimationInterfaceResolution::Resolved
        {
            return Err(Error::ForeignOwner("BreakOutOfPush ISkaterMotionGraph"));
        }
        // Native gets the PushState pointer before GetBaseAnim, but loads
        // mOutFactor from it only after the animation length call.
        {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("BreakOutOfPush PushState"))?;
        }
        let owner = source.animation()?;
        let (root, length) = {
            let animation = owner.lock().map_err(|_| Error::Lock)?;
            let base = animation
                .base()
                .ok_or(Error::MissingOwner("BreakOutOfPush Animatable"))?;
            let root = base
                .body_root
                .clone()
                .ok_or(Error::MissingOwner("BreakOutOfPush base animation"))?;
            let length = animation
                .body_reference_clock(&root, false)
                .map_err(Error::AnimationBody)?;
            (root, length)
        };
        // Native loads mOutFactor only after GetLength and multiplies before GetTime.
        // Drop the animation lock for the Actor state read to preserve lock order.
        let out_factor = {
            let actor = self.prefix().map_err(Error::Actor)?;
            actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("BreakOutOfPush PushState"))?
                .push[0]
        };
        let threshold = out_factor * length;
        let time = {
            let animation = owner.lock().map_err(|_| Error::Lock)?;
            animation
                .body_reference_clock(&root, true)
                .map_err(Error::AnimationBody)?
        };
        reached_breakout_time(time, threshold, || {
            let actor = self.prefix().map_err(Error::Actor)?;
            let bits = actor
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.motion())
                .and_then(|motion| motion.state.as_ref())
                .ok_or(Error::MissingOwner("BreakOutOfPush PushState"))?
                .push_bits;
            if bits.known_mask & 0x8000_0000 == 0 {
                return Err(Error::MissingOwner("BreakOutOfPush mContinuePush"));
            }
            if bits.bits & 0x8000_0000 == 0 && std::env::var_os("SKATE2_TRACE_PUSH_INTERRUPTION").is_some() {
                eprintln!("push interruption BreakOutOfPush: time={time} length={length} out_factor={out_factor} threshold={threshold} continue=false");
            }
            Ok(bits.bits & 0x8000_0000 != 0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::reached_breakout_time;

    #[test]
    fn native_breakout_uses_threshold_unordered_branch_and_continue_bit() {
        assert!(!reached_breakout_time::<()>(0.4, 0.5, || panic!("short circuit")).unwrap());
        assert!(reached_breakout_time(0.5, 0.5, || Ok::<_, ()>(false)).unwrap());
        assert!(!reached_breakout_time(0.5, 0.5, || Ok::<_, ()>(true)).unwrap());
        assert!(reached_breakout_time(f32::NAN, 0.5, || Ok::<_, ()>(false)).unwrap());
        assert!(reached_breakout_time(0.5, f32::NAN, || Ok::<_, ()>(false)).unwrap());
    }
}
