//! `PushOut::Begin82cafa80`: sample `GetFootPos825f5100` and publish the
//! forward distance and height to animation OutDistanceX, then OutDistanceY.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation, play_animation::FastString30,
};
use std::sync::{Arc, Mutex};

pub(crate) trait PushOutSource {
    type Error;
    fn resolve_interfaces(&mut self) -> Result<(), Self::Error>;
    fn foot_pos(&mut self, right: bool) -> Result<[f32; 2], Self::Error>;
    fn add_attribute(&mut self, name: &'static str, value: f32) -> Result<(), Self::Error>;
}

/// Create `82bb3da8` stores NULL; Update/End `825187c0` are BLR. Only Begin
/// has effects, with the right-foot flag from the loaded behaviour attribute.
pub(crate) fn begin<S: PushOutSource>(source: &mut S, right: bool) -> Result<(), S::Error> {
    source.resolve_interfaces()?;
    let [x, y] = source.foot_pos(right)?;
    source.add_attribute("OutDistanceX", x)?;
    source.add_attribute("OutDistanceY", y)
}

struct ActorSource<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
}

impl PushOutSource for ActorSource<'_> {
    type Error = E;

    fn resolve_interfaces(&mut self) -> Result<(), E> {
        // Native resolves the IAnimatable before the same Actor's Motion graph.
        self.animation = Some(self.actor.lifecycle_animation(0)?);
        self.actor.check_motion_runtime_context(self.context)
    }

    fn foot_pos(&mut self, right: bool) -> Result<[f32; 2], E> {
        self.actor.motion_runtime_foot_pos(right)
    }

    fn add_attribute(&mut self, name: &'static str, value: f32) -> Result<(), E> {
        let animation = self
            .animation
            .as_ref()
            .ok_or(E::MissingOwner("PushOut IAnimatable"))?;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(
                FastString30::encode(name).expect("native PushOut literal"),
                value,
                false,
                -1,
            )
            .map_err(E::AnimationWrite)
    }
}

impl ActorConstruction {
    pub(crate) fn motion_runtime_push_out_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        right: bool,
    ) -> Result<(), E> {
        begin(
            &mut ActorSource {
                actor: self,
                context,
                animation: None,
            },
            right,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Source {
        calls: Vec<&'static str>,
        right: Option<bool>,
        attributes: Vec<(&'static str, f32)>,
    }

    impl PushOutSource for Source {
        type Error = ();

        fn resolve_interfaces(&mut self) -> Result<(), Self::Error> {
            self.calls.push("interfaces");
            Ok(())
        }

        fn foot_pos(&mut self, right: bool) -> Result<[f32; 2], Self::Error> {
            self.calls.push("foot");
            self.right = Some(right);
            Ok([1.25, -0.75])
        }

        fn add_attribute(&mut self, name: &'static str, value: f32) -> Result<(), Self::Error> {
            self.calls.push(name);
            self.attributes.push((name, value));
            Ok(())
        }
    }

    #[test]
    fn begin_uses_loaded_side_and_publishes_two_native_names_in_order() {
        let mut source = Source::default();
        begin(&mut source, false).unwrap();
        assert_eq!(source.right, Some(false));
        assert_eq!(
            source.calls,
            ["interfaces", "foot", "OutDistanceX", "OutDistanceY"]
        );
        assert_eq!(
            source.attributes,
            [("OutDistanceX", 1.25), ("OutDistanceY", -0.75)]
        );
    }
}
