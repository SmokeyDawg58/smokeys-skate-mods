//! Proposed `motion_runtime_session` child: four native neutral lifecycle callbacks.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as E};
use crate::{
    actor_construction::animation_mount::MountedActorAnimation,
    generated_motion_load::construct::{TrickAugmentation, control::PhysicsAnimationMode},
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum WalkingWorldError {
    Missing(&'static str),
    Step(crate::simulation_assembly_backend::constraint_session::step_selector::StepSelectorError),
}

/// Must resolve `GetSkeleton_2` on this exact owner. A genuinely NULL Skeleton is
/// a successful no-op; an unavailable world capability is an error, not NULL.
pub(crate) trait ForcePhysicsWriter {
    fn walking_debug_overlay(
        &mut self,
    ) -> Result<crate::debug_overlay::DebugOverlaySlot, WalkingWorldError> {
        Err(WalkingWorldError::Missing("selected walking world"))
    }
    fn launch_walking_steps(
        &mut self,
        _animation: &Arc<Mutex<MountedActorAnimation>>,
        _first: usize,
        _count: usize,
        _world_only: bool,
    ) -> Result<(), WalkingWorldError> {
        Err(WalkingWorldError::Missing("selected walking StepSelector"))
    }

    fn write_skeleton_mode(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        mode: PhysicsAnimationMode,
    ) -> Result<(), E>;
}

impl ActorConstruction {
    /// InitMovingObjects.Begin82ca38d8: same-entity ISkaterAnim Get/reset/Set.
    pub(crate) fn motion_init_moving_objects(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let owner = self.lifecycle_animation(2)?;
        let mut animation = owner.lock().map_err(|_| E::Lock)?;
        let mut coefficients = animation.moving_objects_coefficients().map_err(E::MissingOwner)?;
        coefficients.reset();
        animation.set_moving_objects_coefficients(coefficients).map_err(E::MissingOwner)
    }

    /// ChooseRandomSketchyLanding.Begin82cb0ef0 / End82cb0e18.
    /// GetRandomInt825fe1a0 advances this Motion graph's existing RNG only.
    pub(crate) fn motion_sketchy_landing_attribute(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: FastString30,
        begin: bool,
    ) -> Result<(), E> {
        let animation = self.lifecycle_animation(0)?;
        self.check_motion_runtime_context(context)?;
        if !begin {
            return animation
                .lock()
                .map_err(|_| E::Lock)?
                .motion_remove_construction_attribute(name);
        }
        let value = {
            let mut actor = self.prefix().map_err(E::Actor)?;
            let state = actor
                .graphs
                .as_mut()
                .and_then(|g| g.motion_mut())
                .and_then(|g| g.state.as_mut())
                .ok_or(E::MissingOwner("sketchy landing Motion RNG"))?;
            let mut random = crate::native_skater_random::NativeSkaterRandom {
                seed: state.random_seed,
                index: state.random_index,
                values: state.random_values,
            };
            let value = random
                .try_next(&crate::native_skater_random::NativeRandomGate::production_shared())
                .map_err(|_| E::MissingOwner("native Random access is locked"))?
                as i32;
            state.random_index = random.index;
            state.random_values = random.values;
            // PPC neg wraps for INT_MIN; preserve the signed remainder and assert.
            let choice = value.wrapping_abs() % 5;
            if !(0..=5).contains(&choice) {
                return Err(E::PendingAnimationOperation(
                    "native sketchy landing random assertion",
                ));
            }
            FastString30::encode(&choice.to_string()).expect("single source digit")
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .motion_set_construction_attribute(name, value)
    }

    /// `ScoringTrick::Update`82cbec18 converts its five-word name twice,
    /// then `SetTrickDataTrick`825fe338 sets the trick bit and both names.
    pub(crate) fn motion_score_trick_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        name: &FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let mut words = [0; 6];
        words[..5].copy_from_slice(&name.words);
        let widened = FastString36 { words };
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("ScoringTrick Motion state"))?;
        state.trick_bits |= 0x0100_0000;
        state.trick_name = widened;
        state.trick_gesture = widened;
        Ok(())
    }

    /// ForcePhysics.Begin82ca2f98, physical/UCOM check then nullable Skeleton write.
    pub(crate) fn motion_force_physics_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        mode: PhysicsAnimationMode,
        writer: &mut dyn ForcePhysicsWriter,
    ) -> Result<(), E> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("ForcePhysics entity"));
        }
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        writer.write_skeleton_mode(physical.owner(), mode)
    }

    /// `SetDeckPitchAndYaw::Begin`82cb04d0: IPhysical, ISkaterAnim and IAnimatable
    /// are required, then fresh Skeleton output +218 yaw and +21c pitch go through
    /// the settable-attribute virtual (+48, not normalized, sequence -1).
    pub(crate) fn motion_set_deck_pitch_and_yaw_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        yaw: FastString30,
        pitch: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        self.lifecycle_animation(2)?;
        let animation = self.lifecycle_animation(0)?;
        let value = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_deck_yaw;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(yaw, value, false, -1)
            .map_err(E::AnimationWrite)?;
        let value = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::Lock)?
            .try_skeleton_output()
            .map_err(E::PhysOut)?
            .skeleton_deck_pitch;
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(pitch, value, false, -1)
            .map_err(E::AnimationWrite)
    }

    pub(crate) fn lifecycle_animation(
        &self,
        interface: usize,
    ) -> Result<Arc<Mutex<MountedActorAnimation>>, E> {
        let actor = self.prefix().map_err(E::Actor)?;
        let animation = actor
            .animation
            .as_ref()
            .filter(|_| actor.animation_published)
            .ok_or(E::MissingOwner("Motion lifecycle animation"))?;
        let retained = animation.lock().map_err(|_| E::Lock)?;
        if !actor.animation_interfaces[interface]
            .as_ref()
            .is_some_and(|id| Arc::ptr_eq(id, retained.occurrence()))
            || !retained
                .entity()
                .is_same_issuance(&self.authority.entity_handle())
        {
            return Err(E::ForeignOwner("Motion lifecycle animation interface"));
        }
        Ok(Arc::clone(animation))
    }

    /// UpdateIsWeightOnNose Begin82ca2cb0/End82ca2da8 require the actual
    /// ISkaterAnim interface, then pass true/false to its +0x14 setter.
    pub(crate) fn motion_set_weight_on_nose(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        value: bool,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(2)?
            .lock()
            .map_err(|_| E::Lock)?
            .try_set_weight_on_nose(value)
            .map_err(E::AnimationPhysicsData)
    }

    /// SetTrickAttr::Begin82cb2db8 sets the authored construction value.
    pub(crate) fn motion_set_trick_attribute_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        trick: FastString30,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        self.lifecycle_animation(0)?
            .lock()
            .map_err(|_| E::Lock)?
            .motion_set_construction_attribute(
                FastString30::encode("Trick").expect("source literal"),
                trick,
            )
    }

    /// End82cb2ea8 removes Trick only when cached defaultcyc exists.
    pub(crate) fn motion_set_trick_attribute_end(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<(), E> {
        self.check_motion_runtime_context(context)?;
        if self
            .runtime_required_tree_attribute_from_checked_context(
                FastString30::encode("defaultcyc").expect("source literal"),
            )?
            .is_some()
        {
            self.motion_clear_trick_attribute_end()?;
        }
        Ok(())
    }

    /// `ClearTrickAttr::End`82cb3020 resolves the actual Animatable before removal.
    pub(crate) fn motion_clear_trick_attribute_end(&self) -> Result<(), E> {
        self.lifecycle_animation(0)?
            .lock()
            .map_err(|_| E::Lock)?
            .motion_remove_construction_attribute(
                FastString30::encode("Trick").expect("source literal"),
            )
    }

    /// SetScoreAugmentation.Update82ca8228. Mirror field may be genuinely
    /// uninitialized by the source constructor; only its selected read fails.
    pub(crate) fn motion_score_augmentation_update(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        augmentation: TrickAugmentation,
        mirrored: Option<TrickAugmentation>,
    ) -> Result<(), E> {
        let animation = self.lifecycle_animation(2)?;
        self.check_motion_runtime_context(context)?;
        let mirror = {
            let animation = animation.lock().map_err(|_| E::Lock)?;
            let player = animation
                .player()
                .ok_or(E::MissingOwner("augmentation SkaterAnim"))?;
            if player.flags_known_mask & 0x4000_0000 == 0 {
                return Err(E::MissingOwner("augmentation mirrored flag"));
            }
            player.flags_bits & 0x4000_0000 != 0
        };
        let selected = if mirror {
            mirrored.ok_or(E::MissingOwner("SetScoreAugmentation mirror field"))?
        } else {
            augmentation
        };
        let (name, bit) = augmentation_fields(selected);
        let mut actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .and_then(|g| g.state.as_mut())
            .ok_or(E::MissingOwner("augmentation Motion state"))?;
        if let Some(name) = name {
            state.trick_name = FastString36::encode(name).expect("source score name");
        }
        state.trick_bits |= bit;
        Ok(())
    }

    /// EndShimmy.Begin82c964c8 uses nullable UCOM lookup, not required cached
    /// `ISkaterAnim`. Both ends pass current-frame=true, exactly like `BodySpin`.
    pub(crate) fn motion_end_shimmy_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        blend_out: f32,
    ) -> Result<(), E> {
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(E::ForeignOwner("EndShimmy entity"));
        }
        let animation = {
            let actor = self.prefix().map_err(E::Actor)?;
            let Some(interface) = &actor.animation_interfaces[1] else {
                return Ok(());
            };
            let animation = actor
                .animation
                .as_ref()
                .ok_or(E::MissingOwner("EndShimmy channel owner"))?;
            if !Arc::ptr_eq(
                interface,
                animation.lock().map_err(|_| E::Lock)?.occurrence(),
            ) {
                return Err(E::ForeignOwner("EndShimmy channel interface"));
            }
            Arc::clone(animation)
        };
        let mut animation = animation.lock().map_err(|_| E::Lock)?;
        for name in [
            "SKCH_2H_SHIMMY_LEFT_CHANNEL",
            "SKCH_2H_SHIMMY_RIGHT_CHANNEL",
        ] {
            animation
                .try_end_body_spin_channel(
                    &FastString36::encode(name).expect("source shimmy channel"),
                    blend_out,
                )
                .map_err(E::AnimationWrite)?;
        }
        Ok(())
    }

    /// SetDistComToBoard.Begin82cb1208. Live cvar is read even when the output
    /// subsequently replaces it. `AddSettableAtt` is unnormalized, sequence -1.
    pub(crate) fn motion_dist_com_to_board_begin(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attribute: FastString30,
        set_manually: bool,
        adjust_for_velocity: bool,
        cvars: Option<&InputCvarReferences>,
        dt: f32,
    ) -> Result<(), E> {
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        let mut value = cvars
            .ok_or(E::MissingOwner("SetDistComToBoard cvars"))?
            .try_motion_dist_com_to_board()
            .map_err(E::Cvar)?;
        if !set_manually {
            value = physical
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| E::Lock)?
                .try_animation_output()
                .map_err(E::PhysOut)?
                .centre_of_mass_to_deck;
        }
        let adjustment = if adjust_for_velocity {
            let actor = self.prefix().map_err(E::Actor)?;
            let velocity = actor
                .graphs
                .as_ref()
                .and_then(|g| g.motion())
                .and_then(|g| g.state.as_ref())
                .ok_or(E::MissingOwner("Motion landing velocity"))?
                .last_good_landing_velocity;
            limited_landing_adjustment(dt, velocity)
        } else {
            0.0
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(attribute, value - adjustment, false, -1)
            .map_err(E::AnimationWrite)
    }

    /// `SetSpeed::Update` 0x82ca2358 resolves both Actor interfaces, then
    /// publishes the absolute Motion+0xa4 surface speed (or authored value).
    pub(crate) fn motion_runtime_set_speed(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        attribute: FastString30,
        set_manually: bool,
        set_to_value: f32,
    ) -> Result<(), E> {
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        self.check_motion_runtime_context(context)?;
        let animation = self.lifecycle_animation(0)?;
        let speed = if set_manually {
            set_to_value
        } else {
            physical
                .owner()
                .phys_out()
                .lock()
                .map_err(|_| E::Lock)?
                .try_motion_surface_speed()
                .map_err(E::PhysOut)?
        };
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_add_gesture_attribute(attribute, speed.abs(), false, -1)
            .map_err(E::AnimationWrite)
    }
}

/// Raw825f6ce0: named cases replace name before OR; reverts change only bits.
fn augmentation_fields(value: TrickAugmentation) -> (Option<&'static str>, u32) {
    use TrickAugmentation as A;
    match value {
        A::FsPowerslide => (Some("FSPowerslide"), 0x8000_0000),
        A::BsPowerslide => (Some("BSPowerslide"), 0x4000_0000),
        A::FsRevert => (None, 0x2000_0000),
        A::BsRevert => (None, 0x1000_0000),
        A::NoseManual => (Some("nosemanual"), 0x0800_0000),
        A::TailManual => (Some("tailmanual"), 0x0400_0000),
        A::Wipeout => (Some("wipeout"), 0x0080_0000),
        A::Landing => (Some("Land"), 0x0020_0000),
        A::RideIdle => (Some("ridingidle"), 0x0040_0000),
        A::Switching => (Some("switching"), 0x0010_0000),
    }
}
fn limited_landing_adjustment(dt: f32, velocity: f32) -> f32 {
    let delta = dt * velocity;
    let cap = f32::from_bits(0x3e19_999a);
    if cap - delta >= 0.0 { delta } else { cap }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "exact native FSEL negative result")]
mod tests {
    use super::*;
    #[test]
    fn landing_adjustment_preserves_negative_values_and_fsel_unordered_arm() {
        assert_eq!(limited_landing_adjustment(1.0, -2.0), -2.0);
        assert_eq!(
            limited_landing_adjustment(1.0, f32::NAN).to_bits(),
            0x3e19_999a
        );
        assert_eq!(limited_landing_adjustment(1.0, -0.0).to_bits(), 0x8000_0000);
    }
    #[test]
    fn augmentation_revert_does_not_replace_name_and_landing_spelling_is_preserved() {
        assert_eq!(
            augmentation_fields(TrickAugmentation::FsRevert),
            (None, 0x2000_0000)
        );
        assert_eq!(
            augmentation_fields(TrickAugmentation::Landing),
            (Some("Land"), 0x0020_0000)
        );
    }
}
