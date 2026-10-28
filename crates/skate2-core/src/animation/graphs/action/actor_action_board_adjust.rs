//! Source BoardAdjust::Update82cd54c0, shared by the mounted Up and Down occurrences.
use super::{
    ActionMotionIntentError as E, ActorConstruction, ActorConstructionError, ActorGraphIntents,
    Arc, apply_filter, check_action, check_motion,
};
use crate::{
    generated_action_load::{
        BoardAdjustInstance,
        construct::{BehaviourPayload, Filter},
    },
    motion_graph_entity_authority::BoundMotionGraphContext,
    play_animation::FastString36,
};

impl ActorConstruction {
    /// `BoardAdjust::End` 0x82cc1f38: remove magnitude then angle.
    /// This callback does not read or reset the retained instance values.
    pub(crate) fn try_action_board_adjust_end(
        &self,
        context: &BoundMotionGraphContext<super::ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &BoardAdjustInstance,
    ) -> Result<(), E> {
        self.check_action_instance_context(context, instance.token())?;
        let BehaviourPayload::BoardAdjust {
            magnitude, angle, ..
        } = payload
        else {
            return Err(E::WrongPayload);
        };
        let mut state = self.prefix().map_err(E::Actor)?;
        check_motion(&state)?;
        let destination = state
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.intents.as_mut())
            .ok_or(E::MissingMotion)?;
        destination.erase(magnitude);
        destination.erase(angle);
        Ok(())
    }

    pub(crate) fn try_action_board_adjust_update(
        &self,
        context: &BoundMotionGraphContext<super::ActorIdentity>,
        payload: &BehaviourPayload,
        instance: &mut BoardAdjustInstance,
    ) -> Result<(), E> {
        self.check_action_instance_context(context, instance.token())?;
        let BehaviourPayload::BoardAdjust {
            magnitude,
            angle,
            filter,
            negate_on_mirror,
        } = payload
        else {
            return Err(E::WrongPayload);
        };
        let mut state = self.prefix().map_err(E::Actor)?;
        // Native resolves IActionGraph, IMotionGraph, and ISkaterAnim before maps.
        check_action(&state)?;
        check_motion(&state)?;
        let animation = state
            .animation
            .as_ref()
            .filter(|_| state.animation_published)
            .ok_or(E::MissingAnimation)?
            .clone();
        {
            let guard = animation
                .lock()
                .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
            if !state.animation_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, guard.occurrence()))
            {
                return Err(E::ForeignAnimation);
            }
        }
        // 82cd56ac..56bc obtains the Motion destination map before
        // 82cd56c0..56d4 obtains the Action source map and reads either intent.
        state
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .and_then(|motion| motion.intents.as_ref())
            .ok_or(E::MissingMotion)?;
        let source = state
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.action())
            .and_then(|action| action.intents.as_ref())
            .ok_or(E::MissingAction)?;
        // Both HasIntent calls occur, angle first, even when angle is absent.
        let input_angle = source.get(&ActorGraphIntents::native_name(b"BoardAdjustAngle"));
        let input_mag = source.get(&ActorGraphIntents::native_name(b"BoardAdjustMag"));
        let destination = state
            .graphs
            .as_mut()
            .and_then(|graphs| graphs.motion_mut())
            .and_then(|motion| motion.intents.as_mut())
            .ok_or(E::MissingMotion)?;
        publish_values(
            destination,
            instance,
            magnitude,
            angle,
            input_angle,
            input_mag,
            *filter,
            *negate_on_mirror,
            || {
                let guard = animation
                    .lock()
                    .map_err(|_| E::Actor(ActorConstructionError::Poisoned))?;
                let player = guard.player().ok_or(E::MissingAnimation)?;
                if player.flags_known_mask & 0x4000_0000 != 0x4000_0000 {
                    return Err(E::UnknownAnimationFlags);
                }
                Ok(player.flags_bits & 0x4000_0000 != 0)
            },
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source-separated maps, keys, and callback inputs"
)]
fn publish_values(
    destination: &mut ActorGraphIntents,
    instance: &mut BoardAdjustInstance,
    magnitude_name: &FastString36,
    angle_name: &FastString36,
    input_angle: Option<f32>,
    input_mag: Option<f32>,
    filter: Filter,
    negate_on_mirror: bool,
    mirrored: impl FnOnce() -> Result<bool, E>,
) -> Result<(), E> {
    let (Some(raw_angle), Some(raw_mag)) = (input_angle, input_mag) else {
        destination.erase(magnitude_name); // 82cd5900
        destination.erase(angle_name); // 82cd590c
        instance.reset_after_missing_intent(); // 82cd5910/14
        return Ok(());
    };
    destination
        .insert_or_assign(*magnitude_name, raw_mag)
        .map_err(E::Insert)?; // 82cd5734/3c, before filtering or mirror query
    let mut filtered = apply_filter(raw_angle, filter); // 82cd5748
    if negate_on_mirror && mirrored()? {
        filtered *= -1.0; // 82cd5784
    }
    let output_angle = instance
        .update_angle(filtered)
        .map_err(E::BoardAdjustState)?;
    destination
        .insert_or_assign(*angle_name, output_angle)
        .map_err(E::Insert) // 82cd5898/58bc/58e4
}

#[cfg(test)]
#[path = "actor_action_board_adjust_tests.rs"]
mod tests;
