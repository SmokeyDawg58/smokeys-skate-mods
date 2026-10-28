//! Standing-jump Descent315 behaviours: `StandingJumpLandingCOG` Begin82cb44b8
//! (Update/End are the shared BLR825187c0) and `StandingJumpDescent`
//! Create827126f8, Begin82c95bf8, Update82ca6148 (thunk82ca6aa0); plus
//! LandingOnBoard28's `LandOnBoard` Begin82ca5fa8 (Update/End BLR).
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::play_animation::FastString30;

/// Update82ca62f8..6324: fsel pair clamps remaining/time-to-land into
/// [0.2, 1]; an unordered ratio selects 1. `ble` skips a non-positive or
/// unordered time to land, leaving the base speed untouched.
pub(crate) fn descent_speed(remaining: f32, time_to_land: f32) -> Option<f32> {
    if time_to_land > 0.0 {
        let ratio = remaining / time_to_land;
        let minimum = f32::from_bits(0x3e4c_cccd);
        let raised = if minimum - ratio >= 0.0 {
            minimum
        } else {
            ratio
        };
        Some(if 1.0 - raised >= 0.0 { raised } else { 1.0 })
    } else {
        None
    }
}

impl ActorConstruction {
    /// Begin82cb44b8: PhysOut_OffBoard+0x28 `mComHeightOnLanding` through
    /// `AddSettableAtt` (IAnimatable+0x48) with normalized false, sequence -1.
    /// Native skips a null UCOM lookup; an Actor always registers both.
    pub(crate) fn motion_standing_jump_landing_cog_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attribute: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let height = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .com_height_on_landing;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(attribute, height, false, -1)
            .map_err(E::AnimationWrite)
    }

    /// `LandOnBoard::Begin`82ca5fa8: the Actor's cached IPhysical and
    /// ISkaterAnim (asserted at MotionGraphBehaviours.cpp:7701), then
    /// FlipBoardBackwards (ISkaterAnim+0x24) when PhysOut_OffBoard+0x43
    /// `mLandOnSkateboardFlipBoard` is set.
    pub(crate) fn motion_land_on_board_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let skater = self.lifecycle_animation(2)?;
        let flip = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .land_on_skateboard_flip_board;
        if flip {
            skater
                .lock()
                .map_err(|_| E::Lock)?
                .try_flip_board_backwards()
                .map_err(E::AnimationPhysicsData)?;
        }
        Ok(())
    }

    /// Update82ca6148: the Actor's cached IPhysical and IAnimatable (asserted
    /// at 0x1e4a), base `GetLength - GetTime` before the PhysOut read, then
    /// `SetSpeed` on the same base animation.
    pub(crate) fn motion_standing_jump_descent_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let remaining = {
            let animation = animation.lock().map_err(|_| E::Lock)?;
            let root = animation
                .base()
                .ok_or(E::MissingOwner("StandingJumpDescent Animatable"))?
                .body_root
                .clone()
                .ok_or(E::MissingOwner("StandingJumpDescent base animation"))?;
            let length = animation
                .body_reference_clock(&root, false)
                .map_err(E::AnimationBody)?;
            let time = animation
                .body_reference_clock(&root, true)
                .map_err(E::AnimationBody)?;
            length - time
        };
        let time_to_land = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .time_to_land;
        if let Some(speed) = descent_speed(remaining, time_to_land) {
            animation
                .lock()
                .map_err(|_| E::Lock)?
                .try_set_base_animation_speed(speed)
                .map_err(E::BaseAnimationSpeed)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_clamps_and_non_positive_or_nan_time_to_land_skips() {
        assert_eq!(descent_speed(1.0, 0.0), None);
        assert_eq!(descent_speed(1.0, -1.0), None);
        assert_eq!(descent_speed(1.0, f32::NAN), None);
        assert_eq!(
            descent_speed(0.01, 1.0).map(f32::to_bits),
            Some(0x3e4c_cccd)
        );
        assert_eq!(
            descent_speed(5.0, 1.0).map(f32::to_bits),
            Some(1.0_f32.to_bits())
        );
        assert_eq!(
            descent_speed(0.5, 1.0).map(f32::to_bits),
            Some(0.5_f32.to_bits())
        );
        assert_eq!(
            descent_speed(f32::NAN, 1.0).map(f32::to_bits),
            Some(1.0_f32.to_bits())
        );
    }
}
