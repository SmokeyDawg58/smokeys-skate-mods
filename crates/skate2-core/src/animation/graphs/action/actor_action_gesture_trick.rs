//! Original gesture-to-trick lifecycle. Proposed intent-bridge child, unregistered.
use crate::{
    generated_action_load::construct::GestureGroup,
    motion_graph_entity_authority::MotionGraphInstanceToken, play_animation::FastString36,
};

pub(crate) use crate::gesture_trick_mapping::TrickIntents;
pub(crate) struct GestureTrickInstance {
    token: MotionGraphInstanceToken,
    first_update: bool,        //+8
    found: bool,               //+9
    gesture: FastString36,     //+c
    held_intent: FastString36, //+24
    is_held: bool,             //+3c
}
impl GestureTrickInstance {
    /// Successful allocation82cba2b8 (0x40/16), data ctor82cba1d8.
    pub(crate) fn new() -> Self {
        let empty = crate::actor_graph_construction::ActorGraphIntents::native_name(b"");
        Self {
            token: MotionGraphInstanceToken::issue(),
            first_update: true,
            found: false,
            gesture: empty,
            held_intent: empty,
            is_held: false,
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
}

/// Handles must retain the same resolved interfaces/maps throughout a callback.
/// The mapping implementation must use original map iteration and debug ordering.
pub(crate) trait GestureTrickSource {
    type Error;
    type Action;
    type Motion;
    type Animation;
    type ActionMap;
    type MotionMap;
    type Mapping;
    fn resolve_action(&mut self) -> Result<Self::Action, Self::Error>;
    fn resolve_motion(&mut self) -> Result<Self::Motion, Self::Error>;
    fn resolve_animation(&mut self) -> Result<Self::Animation, Self::Error>;
    fn require_begin_interfaces(
        &mut self,
        action: &Self::Action,
        motion: &Self::Motion,
        animation: &Self::Animation,
    ) -> Result<(), Self::Error>;
    fn require_motion(&mut self, motion: &Self::Motion) -> Result<(), Self::Error>;
    fn motion_map(&mut self, motion: &Self::Motion) -> Result<Self::MotionMap, Self::Error>;
    fn action_map(&mut self, action: &Self::Action) -> Result<Self::ActionMap, Self::Error>;
    fn mapping(&mut self) -> Result<Self::Mapping, Self::Error>;
    fn gesture_intents(
        &mut self,
        mapping: &Self::Mapping,
        intents: &Self::ActionMap,
        group: GestureGroup,
    ) -> Result<Option<TrickIntents>, Self::Error>;
    fn mirrored(&mut self, animation: &Self::Animation) -> Result<bool, Self::Error>;
    fn dont_mirror(&mut self, intents: &Self::ActionMap) -> Result<bool, Self::Error>;
    fn insert(
        &mut self,
        map: &Self::MotionMap,
        name: &FastString36,
        value: f32,
    ) -> Result<(), Self::Error>;
    fn erase(&mut self, map: &Self::MotionMap, name: &FastString36) -> Result<(), Self::Error>;
    fn held_intent(
        &mut self,
        mapping: &Self::Mapping,
        name: &FastString36,
        destination: &mut FastString36,
    ) -> Result<bool, Self::Error>;
}
#[derive(Debug, PartialEq)]
pub(crate) enum GestureTrickError<E> {
    Source(E),
    NullInstance,
}
impl<E: std::fmt::Debug> std::fmt::Display for GestureTrickError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gesture trick: {self:?}")
    }
}
pub(crate) fn begin<S: GestureTrickSource>(
    source: &mut S,
    instance: Option<&mut GestureTrickInstance>,
    group: GestureGroup,
    override_name: Option<&FastString36>,
) -> Result<(), GestureTrickError<S::Error>> {
    use GestureTrickError as E;
    let action = source.resolve_action().map_err(E::Source)?;
    let motion = source.resolve_motion().map_err(E::Source)?;
    let animation = source.resolve_animation().map_err(E::Source)?;
    source
        .require_begin_interfaces(&action, &motion, &animation)
        .map_err(E::Source)?;
    let destination = source.motion_map(&motion).map_err(E::Source)?;
    let intents = source.action_map(&action).map_err(E::Source)?;
    let mapping = source.mapping().map_err(E::Source)?;
    let Some(pair) = source
        .gesture_intents(&mapping, &intents, group)
        .map_err(E::Source)?
    else {
        return Ok(());
    };
    let instance = instance.ok_or(E::NullInstance)?; //found store5050, only after successful lookup
    instance.found = true;
    let selected = if let Some(name) = override_name {
        *name
    } else if source.mirrored(&animation).map_err(E::Source)?
        && !source.dont_mirror(&intents).map_err(E::Source)?
    {
        pair.mirrored
    } else {
        pair.ordinary
    };
    source
        .insert(&destination, &selected, 0.0)
        .map_err(E::Source)?;
    instance.gesture = selected; //after successful insertion/value store
    let is_held = source
        .held_intent(&mapping, &instance.gesture, &mut instance.held_intent)
        .map_err(E::Source)?;
    instance.is_held = is_held; //51a0 after GetHeld output stores
    Ok(())
}
pub(crate) fn update<S: GestureTrickSource>(
    source: &mut S,
    instance: Option<&mut GestureTrickInstance>,
) -> Result<(), GestureTrickError<S::Error>> {
    use GestureTrickError as E;
    let instance = instance.ok_or(E::NullInstance)?;
    if !instance.first_update && instance.found {
        let motion = source.resolve_motion().map_err(E::Source)?;
        source.require_motion(&motion).map_err(E::Source)?;
        if instance.is_held {
            let map = source.motion_map(&motion).map_err(E::Source)?;
            source.erase(&map, &instance.gesture).map_err(E::Source)?;
            source
                .insert(&map, &instance.held_intent, 0.0)
                .map_err(E::Source)?;
        }
    }
    instance.first_update = false;
    Ok(())
}
pub(crate) fn end<S: GestureTrickSource>(
    source: &mut S,
    instance: Option<&GestureTrickInstance>,
) -> Result<(), GestureTrickError<S::Error>> {
    use GestureTrickError as E;
    let instance = instance.ok_or(E::NullInstance)?;
    if !instance.found {
        return Ok(());
    }
    let motion = source.resolve_motion().map_err(E::Source)?;
    source.require_motion(&motion).map_err(E::Source)?;
    if instance.is_held {
        let map = source.motion_map(&motion).map_err(E::Source)?;
        source
            .erase(&map, &instance.held_intent)
            .map_err(E::Source)?;
    }
    let map = source.motion_map(&motion).map_err(E::Source)?; //fresh second GetMotionGraphIntents
    source.erase(&map, &instance.gesture).map_err(E::Source)?;
    Ok(())
}
#[cfg(test)]
#[path = "actor_action_gesture_trick_tests.rs"]
mod tests;
