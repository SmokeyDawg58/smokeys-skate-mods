//! Locomotion778 Begin82cb2888, Update82ca35e8, End82ca37d8 and hook82ca6cc0.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    generated_motion_load::hooks::HookPayload, input_cvar_runtime::InputCvarReferences,
    play_animation::FastString30,
};
impl ActorConstruction {
    /// RunoutFront Begin82ca57e8/End82ca58e0 -> ISkaterAnim+98,
    /// SetRunningOutFront82cb76a8, the retained player flag0x4000.
    pub(crate) fn motion_set_runout_front(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(2)?
            .lock().map_err(|_| E::Lock)?
            .motion_locomotion_flag(0x4000, value)
    }
    /// Dismount Begin82ca55f8/End82ca56f0 -> ISkaterAnim+90,
    /// SetDismounting82cb7678 (insrwi bit14), the retained player flag0x20000.
    pub(crate) fn motion_set_dismounting(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .motion_locomotion_flag(0x20000, value)
    }

    pub(crate) fn motion_trucks_or_deck_contact(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        self.cached_physical_allocation().map_err(E::Physical)?
            .owner().phys_out().lock().map_err(|_| E::Lock)?
            .try_motion_trucks_or_deck_contact().map_err(E::PhysOut)
    }
    /// CanLandOnBoard.Activation82c9b588 samples PhysOut.OffBoard+42.
    pub(crate) fn motion_can_land_on_board_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let result = self
            .cached_physical_allocation()
            .map_err(E::Physical)?
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .can_land_on_skateboard;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(result)
    }

    /// IsInJumpAir.Activation82c9d8e8 queries landing time even on a false foot result.
    pub(crate) fn motion_jump_air_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        foot: Option<crate::generated_motion_load::conditions::Foot>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let foot = foot.ok_or(E::MissingOwner("authored jump foot"))?;
        let in_air = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_is_in_jump_air(foot as i32)
            .map_err(E::AnimationPhysicsData)?;
        let close_time = refs.try_jump_close_to_landing().map_err(E::Cvar)?;
        let close = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_time_to_foot_change()
            .map_err(E::AnimationPhysicsData)?
            < close_time;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
            if in_air {
                self.motion_locomotion_debug()?;
            }
        }
        Ok(in_air && !close)
    }

    pub(crate) fn motion_air_mount_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let result = self
            .lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .try_is_air_mount_requested()
            .map_err(E::AnimationPhysicsData)?;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(result)
    }

    pub(crate) fn motion_locomotion_apex_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let _physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let result = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_is_past_locomotion_apex()
            .map_err(E::AnimationPhysicsData)?;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(result)
    }

    /// IsRidingSwitch.Activation82c9a118 -> checked ISkaterAnim.GetIsSwitch.
    pub(crate) fn motion_riding_switch_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .try_is_riding_switch()
            .map_err(E::AnimationPhysicsData)
    }

    /// Conditions82c94d88/82c94fe8 resolve the native animation query, then debug.
    pub(crate) fn motion_locomotion_foot_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        foot: Option<crate::generated_motion_load::conditions::Foot>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let _physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let foot = foot.ok_or(E::MissingOwner("authored locomotion foot"))?;
        let result = animation
            .lock()
            .map_err(|_| E::Lock)?
            .motion_locomotion_foot_down(foot as i32)?;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(result)
    }
    pub(crate) fn motion_sprint_mount_condition(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let result = self
            .lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .motion_trying_to_sprint()?;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(result)
    }
    /// SteppingBoardNoBoard.Update82ca68c8: PhysOut.OffBoard+3f selects
    /// the zero-valued IAnimatable attribute; Begin/End are shared BLR.
    pub(crate) fn motion_stepping_board_attribute(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        board: FastString30,
        no_board: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        let holding = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .is_holding_skateboard;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(if holding { board } else { no_board }, 0.0, false, -1)
            .map_err(E::AnimationWrite)
    }

    pub(crate) fn motion_locomotion_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let _animatable = self.lifecycle_animation(0)?;
        let animation = self.lifecycle_animation(2)?;
        let velocity = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_camera_system_vector(
                crate::physical_player_phys_out::PhysOutSystemVector::CentreOfMassVelocity,
            )
            .map_err(E::PhysOut)?;
        let world = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_root_matrix;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_is_in_locomotion(true, Some(world), Some(velocity))
            .map_err(|e| E::Locomotion(Box::new(e)))?;
        // Native Attribute+35 is mType; Begin requires scalar type0.
        // scalar_value enforces that exact typed-output invariant.
        let attribute = self.runtime_required_tree_attribute_from_checked_context(
            FastString30::encode("Impact").expect("native literal"),
        )?;
        let mut impact = 0.0;
        if let Some(attribute) = attribute {
            impact = attribute
                .scalar_value()
                .ok_or(E::PendingAnimationOperation("scalar Impact tree attribute"))?;
            let two = impact
                > refs
                    .try_two_foot_landing_impact_threshold()
                    .map_err(E::Cvar)?;
            animation
                .lock()
                .map_err(|_| E::Lock)?
                .motion_locomotion_flag(2, two)?;
        }
        let mut guard = animation.lock().map_err(|_| E::Lock)?;
        let selection = if guard
            .try_is_dismounting()
            .map_err(E::AnimationPhysicsData)?
        {
            "Dismount"
        } else if guard.running_out_front().map_err(E::AnimationPhysicsData)? {
            "RunoutFront"
        } else if guard.running_out_back().map_err(E::AnimationPhysicsData)? {
            "RunoutBack"
        } else if guard.try_is_landing().map_err(E::AnimationPhysicsData)? {
            if impact
                > refs
                    .try_two_foot_landing_impact_threshold()
                    .map_err(E::Cvar)?
            {
                "Land"
            } else {
                "Land1FT"
            }
        } else {
            return Ok(());
        };
        guard.motion_set_construction_attribute(
            FastString30::encode("StartSelected").expect("native literal"),
            FastString30::encode(selection).expect("native literal"),
        )
    }
    pub(crate) fn motion_locomotion_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(2)?;
        let left = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_foot_left_bone_pos;
        let right = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_foot_right_bone_pos;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .motion_locomotion_feet(left, right)
    }
    pub(crate) fn motion_locomotion_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_is_in_locomotion(false, None, None)
            .map_err(|e| E::Locomotion(Box::new(e)))
    }
    pub(crate) fn motion_initialize_locomotion(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        payload: &HookPayload,
    ) -> Result<(), E> {
        let HookPayload::InitializeLocomotion {
            landing,
            running_out_back,
            running_out_front,
            dismounting,
            starting_foot,
        } = payload
        else {
            return Err(E::MissingOwner("InitializeLocomotion hook"));
        };
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let mut guard = animation.lock().map_err(|_| E::Lock)?;
        guard.motion_locomotion_flag(0x8000, *landing)?;
        guard.motion_locomotion_flag(0x2000, *running_out_back)?;
        guard.motion_locomotion_flag(0x4000, *running_out_front)?;
        guard.motion_locomotion_flag(0x20000, *dismounting)?;
        guard.motion_starting_locomotion_foot(*starting_foot)
    }
}

impl ActorConstruction {
    pub(crate) fn motion_locomotion_valid(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(2)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let viable = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .is_locomotion_viable;
        let valid = animation
            .lock()
            .map_err(|_| E::Lock)?
            .motion_locomotion_valid()?;
        if refs.try_debug_loco_common_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(viable & valid)
    }
    pub(crate) fn motion_locomotion_quit_not_allowed(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        refs: &InputCvarReferences,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let allowed = self
            .lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .motion_locomotion_quit_allowed()?;
        if refs.try_debug_loco_motion_conditions().map_err(E::Cvar)? {
            self.motion_locomotion_debug()?;
        }
        Ok(!allowed)
    }
    fn motion_locomotion_debug(&self) -> Result<(), E> {
        let services = self.action_runtime_services().map_err(E::Actor)?;
        services
            .as_ref()
            .ok_or(E::MissingOwner("locomotion debug services"))?
            .overlay
            .require()
            .map_err(E::AirLegOverlay)?;
        Ok(())
    }
}
