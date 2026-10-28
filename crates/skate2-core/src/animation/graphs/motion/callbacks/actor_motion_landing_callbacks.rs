//! Same retained Motion/animation owners for native landing callbacks.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{generated_motion_load::conditions::LandingType, play_animation::FastString30};
#[path = "motion_disable_tricks.rs"]
pub(crate) mod timer;
impl ActorConstruction {
    /// `OBAir::Begin82ca5bc8` / `End82ca5cc0` validate the cached
    /// ISkaterAnim interface, then call `SkaterAnim::SetLanding82cb7690`.
    pub(crate) fn motion_set_obair_landing(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        landing: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let owner = self.lifecycle_animation(2)?;
        owner
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_obair_landing_bit(landing)
            .map_err(E::MissingOwner)
    }

    fn motion_landing_flag(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        mask: u32,
        value: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("landing Motion state"))?;
        state.flags.bits = (state.flags.bits & !mask) | if value { mask } else { 0 };
        state.flags.known_mask |= mask;
        Ok(())
    }
    /// `SetIsLanding`825fe078 writes only bit29 of the same Motion flags.
    pub(crate) fn motion_set_is_landing(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.motion_landing_flag(context, 0x2000_0000, value)
    }
    /// `SetAllowTrick`825fe000 writes only bit25 of the same Motion flags.
    pub(crate) fn motion_set_allow_trick(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.motion_landing_flag(context, 0x0200_0000, value)
    }
    /// `SetIsDoingTrick`825fe0c8 writes bit27 of Motion flags.
    pub(crate) fn motion_set_is_doing_trick(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.motion_landing_flag(context, 0x0800_0000, value)
    }
    /// `HasLandingType`82c9cff0 reads Animation+60 after the actual physical check.
    pub(crate) fn motion_has_landing_type(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        landing: LandingType,
    ) -> Result<bool, E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        Ok(physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .sketchyness_anim_result
            .to_ne_bytes()
            == (landing as u32).to_ne_bytes())
    }
    /// `SetLandingData::Begin`82ca2038, ordered interface checks then two attribute writes.
    pub(crate) fn motion_set_landing_data(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("SetLandingData entity"));
        }
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let animation = self.lifecycle_animation(0)?;
        self.check_motion_runtime_context(context)?;
        let skater = self.lifecycle_animation(2)?;
        let scalar = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_animation_output()
            .map_err(E::PhysOut)?
            .sketchyness_anim_scalar;
        let mirrored = {
            let skater = skater.lock().map_err(|_| E::Lock)?;
            let player = skater
                .player()
                .ok_or(E::MissingOwner("landing SkaterAnim"))?;
            if player.flags_known_mask & 0x4000_0000 == 0 {
                return Err(E::MissingOwner("landing mirrored flag"));
            }
            player.flags_bits & 0x4000_0000 != 0
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(
                FastString30::encode("Spin").expect("native literal"),
                if mirrored { scalar } else { -scalar },
                false,
                -1,
            )
            .map_err(E::AnimationWrite)?;
        let velocity = self
            .prefix()
            .map_err(E::Actor)?
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .and_then(|g| g.state.as_ref())
            .ok_or(E::MissingOwner("landing Motion state"))?
            .last_good_landing_velocity;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(
                FastString30::encode("AvgVelY").expect("native literal"),
                velocity,
                false,
                -1,
            )
            .map_err(E::AnimationWrite)
    }
}
