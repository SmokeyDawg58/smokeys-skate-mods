//! SkitchShimmying Begin82ca76f8, Update82c95f88, End82c963b8.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E,
    skitching::scalar,
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    generated_motion_load::construct::vault::ShimmyDirection,
    input_cvar_runtime::{InputCvarReferences, skitch::SkitchBool},
    play_animation::{FastString30, FastString36},
};
const MIN: u64 = 0xb2d3_a764_e1d8_6d13;
const MAX: u64 = 0xf146_cfa3_b035_29a5;
const DELTA: u64 = 0x1c7d_f104_e241_01ff;
impl ActorConstruction {
    pub(crate) fn motion_skitch_shimmy_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        speed: &mut f32,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        if self
            .prefix()
            .map_err(E::Actor)?
            .physical_interface
            .is_none()
        {
            return Ok(());
        }
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        *speed = p
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .skitch_hand_speed;
        Ok(())
    }
    pub(crate) fn motion_skitch_shimmy_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        a: &AttributeRuntimeInstance,
        direction: ShimmyDirection,
        speed: &mut f32,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        {
            let actor = self.prefix().map_err(E::Actor)?;
            if actor.physical_interface.is_none()
                || actor.animation_interfaces[0].is_none()
                || actor.animation_interfaces[1].is_none()
            {
                return Ok(());
            }
        }
        let p = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        self.lifecycle_animation(1)?;
        let hand_speed = p
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .skitch_hand_speed;
        // Raw96040..64 clamps against the CURRENT speed, not the retained
        // normalized value. Do not "fix" this to introduce temporal smoothing.
        let maximum = scalar(a, DELTA)? + hand_speed;
        let minimum = hand_speed - scalar(a, DELTA)?;
        let bounded = crate::physical_player_grind_manager::math::fsel(
            minimum - hand_speed,
            minimum,
            hand_speed,
        );
        let bounded =
            crate::physical_player_grind_manager::math::fsel(maximum - bounded, bounded, maximum)
                .abs();
        let maximum = scalar(a, MAX)?;
        let minimum = scalar(a, MIN)?;
        let bounded =
            crate::physical_player_grind_manager::math::fsel(minimum - bounded, minimum, bounded);
        let bounded =
            crate::physical_player_grind_manager::math::fsel(maximum - bounded, bounded, maximum);
        *speed = if bounded <= scalar(a, MIN)? {
            0.0
        } else {
            let minimum = scalar(a, MIN)?;
            let maximum = scalar(a, MAX)?;
            (bounded - minimum) / (maximum - scalar(a, MIN)?)
        };
        let names = [
            FastString36::encode("SKCH_2H_SHIMMY_LEFT_CHANNEL").unwrap(),
            FastString36::encode("SKCH_2H_SHIMMY_RIGHT_CHANNEL").unwrap(),
        ];
        let mut animation = animation.lock().map_err(|_| E::Lock)?;
        let channels = animation
            .initialization()
            .and_then(|i| i.channels.as_ref())
            .ok_or(E::MissingOwner("shimmy channels"))?;
        let present = names.map(|name| channels.dictionary.contains_key(&name));
        let selected = usize::from(direction == ShimmyDirection::Right);
        if present[1 - selected] {
            animation
                .try_remove_channel(&names[1 - selected])
                .map_err(E::AnimationChannel)?;
        }
        if !present[selected] {
            animation
                .try_new_channel_with_cycle(names[selected], names[selected], 0.2, 0.2, false, true)
                .map_err(E::AnimationChannel)?;
        }
        animation
            .try_add_gesture_attribute(
                FastString30::encode("ShimmySpeed").unwrap(),
                *speed,
                true,
                -1,
            )
            .map_err(E::AnimationWrite)?;
        drop(animation);
        self.skitch_debug_overlay(
            refs.try_skitch_bool(SkitchBool::kbSkitchShimmyDebugDisplay)
                .map_err(E::Cvar)?,
        )
    }
}
