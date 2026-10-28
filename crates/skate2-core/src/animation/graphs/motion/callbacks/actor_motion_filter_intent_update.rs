//! FilterMotionGraphIntent::Update82cd5f40 for selected loaded occurrences.
use super::{ActorConstruction, ActorIdentity};
use crate::{
    generated_motion_load::{
        construct::filter::{BlendCurve, FilteredIntent},
        runtime::MotionUpdateError,
    },
    motion_graph_entity_authority::{BoundMotionGraphContext, MotionGraphInstanceToken},
};
use std::sync::Arc;
#[cfg(test)]
#[path = "actor_motion_filter_intent_update_tests.rs"]
mod tests;

/// Raw 82cd5f9c..5fa4 commits elapsed time before entity lookup.
pub(crate) fn with_elapsed_time<T, E>(
    data: &mut [f32; 3],
    dt: f32,
    after_time: impl FnOnce(&mut [f32; 3]) -> Result<T, E>,
) -> Result<T, E> {
    data[0] += dt;
    after_time(data)
}

impl ActorConstruction {
    pub(crate) fn motion_filter_intent_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        token: &MotionGraphInstanceToken,
        payload: &FilteredIntent,
        data: &mut [f32; 3],
    ) -> Result<(), MotionUpdateError> {
        use MotionUpdateError as E;
        // Caller has already added controller dt to Data+8. Native resolves
        // IMotionGraph and ISkaterMotionGraph after that write.
        if !context
            .instance()
            .is_some_and(|current| current.is_same_issuance(token))
        {
            return Err(E::ForeignOwner("FilterMotionGraphIntent instance"));
        }
        self.check_motion_runtime_context(context)?;
        let source = {
            let state = self.prefix().map_err(E::Actor)?;
            state
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.intents.as_ref())
                .ok_or(E::MissingOwner("Motion intents"))?
                .get(&payload.intent)
        };
        // Present 82cd611c is scale * source; absent 82cd6168 is default * scale.
        let input = match source {
            Some(value) => payload.scale * value,
            None => payload.default_value * payload.scale,
        };

        // ApplyFilters82ca24f0 resolves ISkaterAnim even when every parsed
        // filter is None, and samples fakie before mirror unconditionally.
        let target = {
            let state = self.prefix().map_err(E::Actor)?;
            let animation = state
                .animation
                .as_ref()
                .filter(|_| state.animation_published)
                .ok_or(E::MissingOwner("Filter ISkaterAnim"))?;
            let animation = animation.lock().map_err(|_| E::Lock)?;
            if !state.animation_interfaces[2]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, animation.occurrence()))
            {
                return Err(E::ForeignOwner("Filter ISkaterAnim"));
            }
            let mut value =
                super::action_motion_intent_bridge::apply_filter(input, payload.filter1);
            value = super::action_motion_intent_bridge::apply_filter(value, payload.filter2);
            let player = animation
                .player()
                .ok_or(E::MissingOwner("Filter ISkaterAnim player"))?;
            if player.flags_known_mask & 0x2000_0000 == 0 {
                return Err(E::MissingOwner("Filter riding fakie flag"));
            }
            if player.flags_bits & 0x2000_0000 != 0 {
                value =
                    super::action_motion_intent_bridge::apply_filter(value, payload.fakie_filter);
            }
            if player.flags_known_mask & 0x4000_0000 == 0 {
                return Err(E::MissingOwner("Filter mirror flag"));
            }
            if player.flags_bits & 0x4000_0000 != 0 {
                value =
                    super::action_motion_intent_bridge::apply_filter(value, payload.mirror_filter);
            }
            value
        };

        let mut blend = choose_blend(
            source.is_some(),
            target,
            data[2],
            payload.blend_rising,
            payload.blend_falling,
            payload.blend_out.present.then_some(payload.blend_out.value),
        );
        if source.is_some() && payload.ramp.present {
            // Update82cd6154 calls Curve::Get only in the source-present arm.
            blend *= curve_get(&payload.curve, data[0]);
        }
        blend_and_publish(
            data,
            target,
            blend,
            payload
                .clamp_acceleration
                .present
                .then_some(payload.clamp_acceleration.value),
            payload
                .clamp_velocity
                .present
                .then_some(payload.clamp_velocity.value),
            |value| {
                let mut state = self.prefix().map_err(E::Actor)?;
                state
                    .graphs
                    .as_mut()
                    .and_then(|g| g.motion_mut())
                    .and_then(|g| g.filtered_intents.as_mut())
                    .ok_or(E::MissingOwner("Filtered Motion intents"))?
                    .insert_or_assign(payload.output_intent, value)
                    .map_err(|_| E::Capacity)
            },
        )
    }
}

/// Curve::Get82ca8c48 finds the first x >= sample and clamps the endpoints.
/// Its interior segment uses fdivs followed by fmadds on the stored points.
fn curve_get(curve: &BlendCurve, sample: f32) -> f32 {
    let first = curve.coordinates[0].expect("constructed curve has first point");
    if sample <= first[0] {
        return first[1];
    }
    for index in 1..curve.count {
        let end = curve.coordinates[index].expect("constructed curve has active point");
        if sample <= end[0] {
            let start = curve.coordinates[index - 1].expect("constructed curve has active point");
            let slope = (end[1] - start[1]) / (end[0] - start[0]);
            return slope.mul_add(sample - start[0], start[1]);
        }
    }
    curve.coordinates[curve.count - 1].expect("constructed curve has last point")[1]
}

/// Raw 82cd6130/6190 ordered comparisons; absent blend-out skips comparison.
fn choose_blend(
    present: bool,
    target: f32,
    old: f32,
    rising: f32,
    falling: f32,
    blend_out: Option<f32>,
) -> f32 {
    if !present {
        if let Some(out) = blend_out {
            return out;
        }
    }
    if target < old { falling } else { rising }
}

/// Raw 82cd61a4..6238: numeric Data writes precede output-map lookup/write.
fn blend_and_publish<E>(
    data: &mut [f32; 3],
    target: f32,
    blend: f32,
    clamp_acc: Option<f32>,
    clamp_vel: Option<f32>,
    publish: impl FnOnce(f32) -> Result<(), E>,
) -> Result<(), E> {
    let old = data[2];
    let weighted_target = blend * target;
    let mut delta = (1.0 - blend).mul_add(old, weighted_target) - old;
    if let Some(acc) = clamp_acc {
        let lower = data[1] - acc;
        let upper = data[1] + acc;
        delta = if lower - delta >= 0.0 { lower } else { delta };
        delta = if upper - delta >= 0.0 { delta } else { upper };
    }
    if let Some(vel) = clamp_vel {
        let lower = -vel;
        let upper = vel;
        delta = if lower - delta >= 0.0 { lower } else { delta };
        delta = if upper - delta >= 0.0 { delta } else { upper };
    }
    let value = delta + old;
    data[1] = delta;
    data[2] = value;
    publish(value)
}
