//! Actual retained Actor adapter for gesture callbacks and conditions.
use super::gesture::{
    self, GestureTrickError, GestureTrickInstance, GestureTrickSource, TrickIntents,
};
use super::{ActionMotionIntentError, ActorConstruction, ActorIdentity, check_action};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    actor_graph_construction::ActorGraphIntents,
    debug_overlay::DebugOverlaySlot,
    generated_action_load::construct::{BehaviourPayload, GestureGroup},
    gesture_trick_mapping::{
        GestureTrickMapping, MappingError,
        debug::{GestureMappingDebugPosition, LiveGestureMappingDebug, MappingDebugError},
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation::FastString36,
};
use std::sync::{Arc, Mutex};
#[path = "actor_action_gesture_condition.rs"]
mod condition;

/// Selected same-manager mapping member, not a whole `SkaterManager` completion.
/// Construct once through825fb468's actual mapping constructor; share across Actors.
pub(crate) struct GestureTrickRuntimeServices {
    mapping: Arc<GestureTrickMapping>,
    cvars: InputCvarReferences,
    overlay: DebugOverlaySlot,
    position: GestureMappingDebugPosition,
}
impl GestureTrickRuntimeServices {
    pub(crate) fn try_capture_mapping_member(
        mapping: Arc<GestureTrickMapping>,
        cvars: InputCvarReferences,
        overlay: DebugOverlaySlot,
        position: GestureMappingDebugPosition,
    ) -> Result<Self, MappingError<std::convert::Infallible>> {
        if !mapping.is_initialized() {
            return Err(MappingError::NotInitialized);
        }
        Ok(Self {
            mapping,
            cvars,
            overlay,
            position,
        })
    }
}
#[derive(Debug)]
pub(crate) enum GestureRuntimeError {
    Intent(ActionMotionIntentError),
    AnimationPhysics(
        crate::actor_animation_construction::physics_update_data::AnimationPhysicsDataError,
    ),
    MissingManagerMapping,
    Mapping(MappingError<MappingDebugError<InputCvarError>>),
    Held(MappingError<std::convert::Infallible>),
}
impl std::fmt::Display for GestureRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intent(e) => write!(f, "gesture runtime intent: {e}"),
            Self::AnimationPhysics(e) => write!(f, "animation condition: {e}"),
            Self::MissingManagerMapping => write!(f, "gesture runtime manager mapping missing"),
            Self::Mapping(e) => write!(f, "gesture runtime mapping: {e}"),
            Self::Held(e) => write!(f, "gesture runtime held mapping: {e}"),
        }
    }
}
#[derive(Clone)]
struct GraphHandle(Arc<()>);
struct Source<'a> {
    actor: &'a ActorConstruction,
    services: Option<&'a Arc<GestureTrickRuntimeServices>>,
}
impl ActorConstruction {
    /// `IsMirrored::ActivationFunc82c9a210` resolves the same `ISkaterAnim`,
    /// then `GetIsMirrored82cb7598` reads its retained player mirror bit.
    pub(crate) fn action_runtime_is_mirrored(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, GestureRuntimeError> {
        self.check_action_runtime_context(context)
            .map_err(GestureRuntimeError::Intent)?;
        self.runtime_is_mirrored_from_checked_context()
    }

    /// IsDismounting82c9ad78 resolves the same Action-bound ISkaterAnim,
    /// then calls its virtual IsDismounting82cb7940.
    pub(crate) fn action_runtime_is_dismounting(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, GestureRuntimeError> {
        self.check_action_runtime_context(context)
            .map_err(GestureRuntimeError::Intent)?;
        self.runtime_is_dismounting_from_checked_context()
    }

    /// Motion's IsDismounting condition invokes the same ISkaterAnim virtual
    /// after its distinct Motion controller context has been checked.
    pub(crate) fn runtime_is_dismounting_from_checked_context(
        &self,
    ) -> Result<bool, GestureRuntimeError> {
        let mut source = Source {
            actor: self,
            services: None,
        };
        let animation = source.resolve_animation()?;
        let a = animation.lock().map_err(|_| {
            GestureRuntimeError::Intent(ActionMotionIntentError::Actor(
                super::ActorConstructionError::Poisoned,
            ))
        })?;
        a.try_is_dismounting()
            .map_err(GestureRuntimeError::AnimationPhysics)
    }

    /// Motion IsLanding82c9d3c0 resolves the same ISkaterAnim interface.
    pub(crate) fn runtime_is_landing_from_checked_context(
        &self,
    ) -> Result<bool, GestureRuntimeError> {
        let mut source = Source {
            actor: self,
            services: None,
        };
        let animation = source.resolve_animation()?;
        let a = animation.lock().map_err(|_| {
            GestureRuntimeError::Intent(ActionMotionIntentError::Actor(
                super::ActorConstructionError::Poisoned,
            ))
        })?;
        a.try_is_landing()
            .map_err(GestureRuntimeError::AnimationPhysics)
    }

    /// Motion IsRunningOut82c9d538 resolves the same ISkaterAnim interface.
    pub(crate) fn runtime_is_running_out_from_checked_context(
        &self,
    ) -> Result<bool, GestureRuntimeError> {
        let mut source = Source {
            actor: self,
            services: None,
        };
        let animation = source.resolve_animation()?;
        let a = animation.lock().map_err(|_| {
            GestureRuntimeError::Intent(ActionMotionIntentError::Actor(
                super::ActorConstructionError::Poisoned,
            ))
        })?;
        a.try_is_running_out()
            .map_err(GestureRuntimeError::AnimationPhysics)
    }

    /// Motion conditions also query the same ISkaterAnim mirror bit with a
    /// Motion context. Their caller validates that context first.
    pub(crate) fn runtime_is_mirrored_from_checked_context(
        &self,
    ) -> Result<bool, GestureRuntimeError> {
        let mut source = Source {
            actor: self,
            services: None,
        };
        let animation = source.resolve_animation()?;
        source.mirrored(&animation)
    }
}
impl Source<'_> {
    fn action_read<T>(
        &self,
        handle: &GraphHandle,
        read: impl FnOnce(&ActorGraphIntents) -> Result<T, GestureRuntimeError>,
    ) -> Result<T, GestureRuntimeError> {
        use ActionMotionIntentError as E;
        let state = self
            .actor
            .prefix()
            .map_err(|e| GestureRuntimeError::Intent(E::Actor(e)))?;
        let graph = state
            .graphs
            .as_ref()
            .and_then(|g| g.action())
            .ok_or(GestureRuntimeError::Intent(E::MissingAction))?;
        if !Arc::ptr_eq(&handle.0, graph.occurrence()) {
            return Err(GestureRuntimeError::Intent(E::ForeignAction));
        }
        read(
            graph
                .intents
                .as_ref()
                .ok_or(GestureRuntimeError::Intent(E::MissingAction))?,
        )
    }
    fn motion_write<T>(
        &self,
        handle: &GraphHandle,
        write: impl FnOnce(&mut ActorGraphIntents) -> Result<T, GestureRuntimeError>,
    ) -> Result<T, GestureRuntimeError> {
        use ActionMotionIntentError as E;
        let mut state = self
            .actor
            .prefix()
            .map_err(|e| GestureRuntimeError::Intent(E::Actor(e)))?;
        let graph = state
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(GestureRuntimeError::Intent(E::MissingMotion))?;
        if !Arc::ptr_eq(&handle.0, graph.occurrence()) {
            return Err(GestureRuntimeError::Intent(E::ForeignMotion));
        }
        write(
            graph
                .intents
                .as_mut()
                .ok_or(GestureRuntimeError::Intent(E::MissingMotion))?,
        )
    }
}
impl GestureTrickSource for Source<'_> {
    type Error = GestureRuntimeError;
    type Action = GraphHandle;
    type Motion = Option<GraphHandle>;
    type Animation = Arc<Mutex<MountedActorAnimation>>;
    type ActionMap = GraphHandle;
    type MotionMap = GraphHandle;
    type Mapping = Arc<GestureTrickRuntimeServices>;
    fn resolve_action(&mut self) -> Result<Self::Action, Self::Error> {
        let state = self
            .actor
            .prefix()
            .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Actor(e)))?;
        check_action(&state).map_err(Self::Error::Intent)?;
        Ok(GraphHandle(Arc::clone(
            state
                .graphs
                .as_ref()
                .and_then(|g| g.action())
                .expect("checked Action")
                .occurrence(),
        )))
    }
    fn resolve_motion(&mut self) -> Result<Self::Motion, Self::Error> {
        use ActionMotionIntentError as E;
        let state = self
            .actor
            .prefix()
            .map_err(|e| Self::Error::Intent(E::Actor(e)))?;
        let cached = state
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| state.motion_graph_published);
        let registered = state.graph_interfaces[1].as_ref();
        match (cached, registered) {
            (None, None) => Ok(None),
            (Some(graph), Some(interface)) if Arc::ptr_eq(graph.occurrence(), interface) => {
                Ok(Some(GraphHandle(Arc::clone(graph.occurrence()))))
            }
            _ => Err(Self::Error::Intent(E::ForeignMotion)),
        }
    }
    fn resolve_animation(&mut self) -> Result<Self::Animation, Self::Error> {
        use ActionMotionIntentError as E;
        let state = self
            .actor
            .prefix()
            .map_err(|e| Self::Error::Intent(E::Actor(e)))?;
        let animation = state
            .animation
            .as_ref()
            .filter(|_| state.animation_published)
            .ok_or(Self::Error::Intent(E::MissingAnimation))?;
        {
            let a = animation.lock().map_err(|_| {
                Self::Error::Intent(E::Actor(super::ActorConstructionError::Poisoned))
            })?;
            if !state.animation_interfaces[2]
                .as_ref()
                .is_some_and(|i| Arc::ptr_eq(i, a.occurrence()))
            {
                return Err(Self::Error::Intent(E::ForeignAnimation));
            }
        }
        Ok(Arc::clone(animation))
    }
    fn require_begin_interfaces(
        &mut self,
        _: &Self::Action,
        motion: &Self::Motion,
        _: &Self::Animation,
    ) -> Result<(), Self::Error> {
        self.require_motion(motion)
    }
    fn require_motion(&mut self, motion: &Self::Motion) -> Result<(), Self::Error> {
        motion
            .as_ref()
            .ok_or(Self::Error::Intent(ActionMotionIntentError::MissingMotion))
            .map(|_| ())
    }
    fn motion_map(&mut self, motion: &Self::Motion) -> Result<Self::MotionMap, Self::Error> {
        let h = motion
            .as_ref()
            .ok_or(Self::Error::Intent(ActionMotionIntentError::MissingMotion))?;
        self.motion_write(h, |_| Ok(()))?;
        Ok(h.clone())
    }
    fn action_map(&mut self, action: &Self::Action) -> Result<Self::ActionMap, Self::Error> {
        self.action_read(action, |_| Ok(()))?;
        Ok(action.clone())
    }
    fn mapping(&mut self) -> Result<Self::Mapping, Self::Error> {
        self.services
            .cloned()
            .ok_or(Self::Error::MissingManagerMapping)
    }
    fn gesture_intents(
        &mut self,
        mapping: &Self::Mapping,
        intents: &Self::ActionMap,
        group: GestureGroup,
    ) -> Result<Option<TrickIntents>, Self::Error> {
        let empty = ActorGraphIntents::native_name(b"");
        let mut pair = TrickIntents {
            ordinary: empty,
            mirrored: empty,
        };
        let mut debug = LiveGestureMappingDebug {
            switch: || mapping.cvars.try_gesture_mapping_debug(),
            overlay: &mapping.overlay,
            position: &mapping.position,
        };
        let found = self.action_read(intents, |intents| {
            mapping
                .mapping
                .try_gesture_intents(intents, group, Some(&mut pair), &mut debug)
                .map_err(Self::Error::Mapping)
        })?;
        Ok(found.then_some(pair))
    }
    fn mirrored(&mut self, animation: &Self::Animation) -> Result<bool, Self::Error> {
        use ActionMotionIntentError as E;
        let a = animation
            .lock()
            .map_err(|_| Self::Error::Intent(E::Actor(super::ActorConstructionError::Poisoned)))?;
        let p = a.player().ok_or(Self::Error::Intent(E::MissingAnimation))?;
        if p.flags_known_mask & 0x4000_0000 == 0 {
            return Err(Self::Error::Intent(E::UnknownAnimationFlags));
        }
        Ok(p.flags_bits & 0x4000_0000 != 0)
    }
    fn dont_mirror(&mut self, intents: &Self::ActionMap) -> Result<bool, Self::Error> {
        self.action_read(intents, |i| {
            Ok(i.get(&ActorGraphIntents::native_name(b"DontMirrorTrick"))
                .is_some())
        })
    }
    fn insert(
        &mut self,
        map: &Self::MotionMap,
        name: &FastString36,
        value: f32,
    ) -> Result<(), Self::Error> {
        self.motion_write(map, |i| {
            i.insert_or_assign(*name, value)
                .map_err(|e| Self::Error::Intent(ActionMotionIntentError::Insert(e)))
        })
    }
    fn erase(&mut self, map: &Self::MotionMap, name: &FastString36) -> Result<(), Self::Error> {
        self.motion_write(map, |i| {
            i.erase(name);
            Ok(())
        })
    }
    fn held_intent(
        &mut self,
        mapping: &Self::Mapping,
        name: &FastString36,
        destination: &mut FastString36,
    ) -> Result<bool, Self::Error> {
        mapping
            .mapping
            .try_held_intent(name, destination)
            .map_err(Self::Error::Held)
    }
}
impl ActorConstruction {
    fn check_gesture_context(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&GestureTrickInstance>,
    ) -> Result<(), GestureRuntimeError> {
        if let Some(i) = instance {
            self.check_action_instance_context(context, i.token())
                .map_err(GestureRuntimeError::Intent)
        } else {
            if context.instance().is_some() {
                return Err(GestureRuntimeError::Intent(
                    ActionMotionIntentError::ForeignContext,
                ));
            }
            self.check_action_runtime_context(context)
                .map_err(GestureRuntimeError::Intent)
        }
    }
    pub(crate) fn action_gesture_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &BehaviourPayload,
        instance: Option<&mut GestureTrickInstance>,
        services: Option<&Arc<GestureTrickRuntimeServices>>,
    ) -> Result<(), GestureTrickError<GestureRuntimeError>> {
        self.check_gesture_context(context, instance.as_deref())
            .map_err(GestureTrickError::Source)?;
        let BehaviourPayload::GestureIntent {
            group,
            has_override,
            override_name,
        } = payload
        else {
            return Err(GestureTrickError::Source(GestureRuntimeError::Intent(
                ActionMotionIntentError::WrongPayload,
            )));
        };
        gesture::begin(
            &mut Source {
                actor: self,
                services,
            },
            instance,
            *group,
            has_override.then_some(override_name),
        )
    }
    pub(crate) fn action_gesture_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut GestureTrickInstance>,
    ) -> Result<(), GestureTrickError<GestureRuntimeError>> {
        self.check_gesture_context(context, instance.as_deref())
            .map_err(GestureTrickError::Source)?;
        gesture::update(
            &mut Source {
                actor: self,
                services: None,
            },
            instance,
        )
    }
    pub(crate) fn action_gesture_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&GestureTrickInstance>,
    ) -> Result<(), GestureTrickError<GestureRuntimeError>> {
        self.check_gesture_context(context, instance)
            .map_err(GestureTrickError::Source)?;
        gesture::end(
            &mut Source {
                actor: self,
                services: None,
            },
            instance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gesture_trick_mapping::MappingAllocation;
    #[test]
    fn capability_retains_the_same_initialized_member_without_preflighting_debug() {
        let mut mapping = GestureTrickMapping::new();
        mapping
            .try_initialize::<()>(|_, _, _| Ok(MappingAllocation::Host))
            .unwrap();
        let mapping = Arc::new(mapping);
        let retained = Arc::clone(&mapping);
        let services = GestureTrickRuntimeServices::try_capture_mapping_member(
            mapping,
            InputCvarReferences::new_static_initialized(),
            DebugOverlaySlot::default(),
            GestureMappingDebugPosition::new(),
        )
        .unwrap();
        assert!(Arc::ptr_eq(&services.mapping, &retained));
        // Source capability capture is not a debug query; default-null holder remains null.
        assert!(services.overlay.require().is_err());
    }
    #[test]
    fn incomplete_member_is_not_published_as_a_mapping_capability() {
        let retained = Arc::new(GestureTrickMapping::new());
        assert!(matches!(
            GestureTrickRuntimeServices::try_capture_mapping_member(
                Arc::clone(&retained),
                InputCvarReferences::new_static_initialized(),
                DebugOverlaySlot::default(),
                GestureMappingDebugPosition::new(),
            ),
            Err(MappingError::NotInitialized)
        ));
        assert!(!retained.is_initialized());
    }
}
