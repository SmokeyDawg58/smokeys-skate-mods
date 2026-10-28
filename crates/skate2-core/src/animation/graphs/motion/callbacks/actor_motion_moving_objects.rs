//! MovingObject animation behaviour and its animation-end selection predicate.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    actor_animation_construction::moving_objects::MovingObjectsCoefficients,
    generated_motion_load::{
        MotionLoadError,
        construct::vault::{MovingDirection, MovingObject},
    },
    input_cvar_runtime::InputCvarReferences,
    physics::PointGraph,
    play_animation::FastString30,
    reckoning_numeric::{HostReckoningNumeric, wrap_turns_to_radians},
};
#[path = "actor_motion_moving_objects_math.rs"]
mod math;

impl ActorConstruction {
    /// MovingObject.Update82ca3a30, including the final coefficient publication.
    pub(crate) fn motion_moving_object_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        behaviour: &MovingObject,
        instance: &mut MovingObjectsCoefficients,
        cvars: &InputCvarReferences,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(2)?;
        let animatable = self.lifecycle_animation(0)?;
        let (movement, force, crouch) = {
            let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
            let offboard = output.try_offboard_output().map_err(E::PhysOut)?;
            (
                [offboard.anim_obj_mov_x, offboard.anim_obj_mov_z],
                [offboard.anim_obj_force_x, offboard.anim_obj_force_z],
                offboard.anim_crouch,
            )
        };
        let (raw_speed, aligned_force) =
            math::speed_and_aligned_force(movement, force, &mut HostReckoningNumeric)
                .map_err(E::ConditionNumeric)?;
        let strength_target = moving_strength_curve(behaviour)?.evaluate(aligned_force);
        let strength_delta = cvars.try_mo_strength_max_delta().map_err(E::Cvar)?;
        let strength = math::bound(
            strength_target,
            instance.last_strength - strength_delta,
            instance.last_strength + strength_delta,
        );
        instance.last_strength = strength;
        // Native stores strength before sampling the next cvar targets.
        let [raw_speed_max, crouch_delta, speed_delta] =
            cvars.try_mo_motion_limits().map_err(E::Cvar)?;
        let speed_target = crate::camera_compass::runtime::math::unit_fraction(raw_speed / raw_speed_max);
        let smoothed_crouch = math::bound(
            crouch,
            instance.last_crouch - crouch_delta,
            instance.last_crouch + crouch_delta,
        );
        instance.last_crouch = smoothed_crouch;
        let speed = math::bound(
            speed_target,
            instance.last_speed - speed_delta,
            instance.last_speed + speed_delta,
        );

        let mut last_angle = instance.last_full_angle;
        if (math::is_pull(instance.last_direction) && math::is_push(instance.current_direction))
            || (math::is_push(instance.last_direction) && math::is_pull(instance.current_direction))
        {
            if instance.flags_known_mask & 0x80 == 0 {
                return Err(E::MissingOwner("moving-object handled push/pull flag"));
            }
            if instance.flags_bits & 0x80 == 0 {
                instance.flags_bits |= 0x80;
                last_angle = math::angle_wrap(last_angle + math::PI);
            }
        }
        let direction = behaviour.direction as u32;
        let angle_step = if math::is_transition(direction) {
            let owner = animatable.lock().map_err(|_| E::Lock)?;
            let root = owner
                .base()
                .and_then(|base| base.body_root.as_ref())
                .ok_or(E::MissingOwner("moving-object base animation"))?;
            let time = owner
                .body_reference_clock(root, true)
                .map_err(E::AnimationBody)?;
            let length = owner
                .body_reference_clock(root, false)
                .map_err(E::AnimationBody)?;
            let remaining = (length - time)
                + if behaviour.blend_out_over_next_cycle {
                    length
                } else {
                    0.0
                };
            instance.blend_target_angle = 0.0;
            let frames = remaining * f32::from_bits(0x426f_ffff);
            let frames = if 1.0 - frames >= 0.0 { 1.0 } else { frames };
            if math::is_pull(direction) {
                instance.blend_target_angle = math::PI;
            }
            math::angle_wrap(instance.blend_target_angle - last_angle) / frames
        } else {
            let heading = crate::camera_compass::runtime::math::camera_atan2_pending(
                -movement[0],
                movement[1],
                &mut HostReckoningNumeric,
            )
            .map_err(E::ConditionNumeric)?
            .finish();
            let limit = cvars.try_mo_angle_max_delta().map_err(E::Cvar)? * math::DEGREES_TO_RADIANS;
            math::bound(math::angle_wrap(heading - last_angle), -limit, limit)
        };
        let (angle, coefficient) = math::directional_angle(
            direction,
            math::angle_wrap(last_angle + angle_step),
            instance.last_angle,
        );
        instance.last_full_angle = angle;
        instance.last_speed = speed;
        instance.last_angle = coefficient;
        {
            let mut owner = animatable.lock().map_err(|_| E::Lock)?;
            // AddSettableAtt virtual+48 (bool=false, index=-1) in native order.
            // The lower-case authored crouch and global Crouch are distinct.
            for (name, value) in [
                (behaviour.angle, coefficient),
                (behaviour.strength, strength),
                (behaviour.crouch, smoothed_crouch),
                (
                    FastString30::encode("Crouch").expect("native literal"),
                    crouch,
                ),
                (
                    FastString30::encode("Speed").expect("native literal"),
                    speed,
                ),
            ] {
                owner
                    .try_add_gesture_attribute(name, value, false, -1)
                    .map_err(E::AnimationWrite)?;
            }
        }
        let result = animation
            .lock()
            .map_err(|_| E::Lock)?
            .set_moving_objects_coefficients(*instance)
            .map_err(E::MissingOwner);
        result
    }

    /// Initialize82cb3a38 then Begin82cb3b40. The instance receives the last
    /// published coefficients; Begin does not publish its changes back yet.
    pub(crate) fn motion_moving_object_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        direction: MovingDirection,
        instance: &mut MovingObjectsCoefficients,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        *instance = self
            .lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .moving_objects_coefficients()
            .map_err(E::MissingOwner)?;
        instance.last_direction = instance.current_direction;
        instance.current_direction = direction as u32;
        instance.flags_bits &= 0x7f;
        instance.flags_known_mask |= 0x80;
        // Native resolves Physical even though Begin makes no call through it.
        let _physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let owner = self.lifecycle_animation(2)?;
        let mut animation = owner.lock().map_err(|_| E::Lock)?;
        let mirror = match direction {
            MovingDirection::PushRight
            | MovingDirection::PushRightToLeft
            | MovingDirection::PushToPullRight
            | MovingDirection::PullRight
            | MovingDirection::PullRightToLeft
            | MovingDirection::PullToPushRight => {
                animation.try_foot_forward().map_err(E::MissingOwner)? == 0
            }
            MovingDirection::PushLeft
            | MovingDirection::PushLeftToRight
            | MovingDirection::PushToPullLeft
            | MovingDirection::PullLeft
            | MovingDirection::PullLeftToRight
            | MovingDirection::PullToPushLeft => {
                animation.try_foot_forward().map_err(E::MissingOwner)? == 1
            }
            MovingDirection::Still => false,
        };
        animation
            .set_mirror_next_anim(mirror, mirror)
            .map_err(E::MissingOwner)
    }

    /// AngleAdjustByAnimationEndLessThan.Activation82c9db98. Reads the retained
    /// coefficient target/current angles and the actual base animation clock.
    pub(crate) fn motion_moving_object_angle_adjust_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        max_delta_angle: f32,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let _physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let _animatable = self.lifecycle_animation(0)?;
        let owner = self.lifecycle_animation(2)?;
        let animation = owner.lock().map_err(|_| E::Lock)?;
        let coefficients = animation
            .moving_objects_coefficients()
            .map_err(E::MissingOwner)?;
        let root = animation
            .base()
            .and_then(|base| base.body_root.as_ref())
            .ok_or(E::MissingOwner("moving-object base animation"))?;
        let time = animation
            .body_reference_clock(root, true)
            .map_err(E::AnimationBody)?;
        let length = animation
            .body_reference_clock(root, false)
            .map_err(E::AnimationBody)?;
        Ok(angle_adjust_by_animation_end_less_than(
            &coefficients,
            time,
            length,
            max_delta_angle,
        ))
    }
}

/// Full 64-bit GetAttributePointer key reconstructed at82ca3e28..40. Native
/// PointGraphData8 has a16-byte header, X at+10, Y at+30; DefaultDataArea is0x50.
fn moving_strength_curve(behaviour: &MovingObject) -> Result<PointGraph<8>, E> {
    let data = behaviour
        .attributes
        .try_get_data(0xc3a1_5e5d_e438_066f, 0)
        .map_err(|error| E::Load(MotionLoadError::AttributeRuntime(error)))?;
    let Some(data) = data else {
        return Ok(PointGraph {
            x: [0.0; 8],
            y: [0.0; 8],
        });
    };
    let bytes = data
        .bytes()
        .get(..0x50)
        .ok_or(E::MissingOwner("moving-object strength graph layout"))?;
    let word = |offset| {
        f32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("checked graph size"),
        )
    };
    Ok(PointGraph {
        x: std::array::from_fn(|i| word(0x10 + i * 4)),
        y: std::array::from_fn(|i| word(0x30 + i * 4)),
    })
}

fn angle_adjust_by_animation_end_less_than(
    coefficients: &MovingObjectsCoefficients,
    time: f32,
    length: f32,
    max_delta_degrees: f32,
) -> bool {
    let frames = (length - time) * f32::from_bits(0x426f_ffff);
    let frames = if 1.0 - frames >= 0.0 { 1.0 } else { frames };
    let inverse_frames = 1.0 / frames;
    let delta = coefficients.blend_target_angle - coefficients.last_full_angle;
    let angle = wrap_turns_to_radians(delta * f32::from_bits(0x3e22_f983));
    // Raw VMX82c9de8c..98: absolute wrapped angle MULTIPLIED by 1/frames.
    // HexRays loses this factor and misleadingly prints only 1/frames < max.
    let amount = inverse_frames * f32::from_bits(angle.to_bits() & 0x7fff_ffff);
    amount < max_delta_degrees * f32::from_bits(0x3c8e_fa35)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angle_budget_uses_wrapped_distance_and_remaining_frames() {
        let mut data = MovingObjectsCoefficients::after_construction();
        data.blend_target_angle = 1.0;
        assert!(angle_adjust_by_animation_end_less_than(
            &data, 0.0, 1.0, 2.0
        ));
        assert!(!angle_adjust_by_animation_end_less_than(
            &data, 0.99, 1.0, 2.0
        ));
        data.last_full_angle = 3.13;
        data.blend_target_angle = -3.13;
        assert!(angle_adjust_by_animation_end_less_than(
            &data, 1.0, 1.0, 2.0
        ));
        assert!(!angle_adjust_by_animation_end_less_than(
            &data,
            f32::NAN,
            1.0,
            2.0
        ));
        data.last_full_angle = data.blend_target_angle;
        assert!(!angle_adjust_by_animation_end_less_than(
            &data, 1.0, 1.0, 0.0
        ));
    }
}
