//! Proposed `actor_construction::motion_runtime_session::grinding` child.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    actor_animation_construction::blend_anim::BodyAnimationReference,
    actor_construction::animation_mount::MountedActorAnimation,
    actor_graph_construction::ActorSkaterMotionGraph,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    debug_overlay::DebugOverlaySlot,
    generated_motion_load::MotionLoadError,
    input_cvar_runtime::{InputCvarReferences, grind_states::GrindCvar},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    play_animation::{FastString30, FastString36},
};
use std::sync::{Arc, Mutex};
#[path = "motion_grinding.rs"]
pub(crate) mod kernel;
use kernel::{Callback, Collection, GrindingCvar, GrindingSource};

/// GlobalAttributeCollection+0x138 and +0x154: `anim_motion` (ctor828b1590)
/// 0x861925f635314abd bound at 828b2e30, then 0x3cce4d5548190472 at 828b3028.
pub(crate) struct GrindingMotionAttributes {
    vault: Arc<AttributeRuntimeVault>,
    control_fade: AttributeRuntimeInstance,
    crouch: AttributeRuntimeInstance,
}
impl GrindingMotionAttributes {
    pub(crate) fn try_bind(
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<Self, AttributeRuntimeError> {
        let control_fade = vault.instance(0x90c3_decf_0105_631e, 0x8619_25f6_3531_4abd);
        control_fade.try_cached_layout(0x5c0)?;
        let crouch = vault.instance(0x90c3_decf_0105_631e, 0x3cce_4d55_4819_0472);
        crouch.try_cached_layout(0x5c0)?;
        Ok(Self {
            vault: Arc::clone(vault),
            control_fade,
            crouch,
        })
    }
}

/// Short-lived per-callback view; each native callback re-resolves its UCOM
/// interfaces, so nothing here outlives the call.
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    attributes: Option<&'a GrindingMotionAttributes>,
    overlay: Option<&'a DebugOverlaySlot>,
    cvars: Option<&'a InputCvarReferences>,
    dt: f32,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    animation: Option<Arc<Mutex<MountedActorAnimation>>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("grinding IPhysical"))
    }
    fn animation(&self) -> Result<&Arc<Mutex<MountedActorAnimation>>, Error> {
        self.animation
            .as_ref()
            .ok_or(Error::MissingOwner("grinding IAnimatable"))
    }
    fn cvars(&self) -> Result<&InputCvarReferences, Error> {
        self.cvars.ok_or(Error::MissingOwner("grinding live cvars"))
    }
    fn with_motion_graph<T>(
        &self,
        read: impl FnOnce(&mut ActorSkaterMotionGraph<ActorIdentity>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let captured = self
            .motion
            .as_ref()
            .ok_or(Error::MissingOwner("grinding captured Motion"))?;
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("grinding Motion"))?;
        if !Arc::ptr_eq(captured, graph.occurrence()) {
            return Err(Error::ForeignOwner("grinding captured Motion"));
        }
        read(graph)
    }
}
impl GrindingSource for Source<'_> {
    type Error = Error;
    type Base = BodyAnimationReference;
    fn resolve(&mut self, callback: Callback) -> Result<(), Error> {
        use Callback as C;
        self.actor.check_motion_runtime_context(self.context)?;
        if !matches!(callback, C::ControlFadeUpdate) {
            self.physical = Some(Arc::clone(
                self.actor
                    .cached_physical_allocation()
                    .map_err(Error::Physical)?
                    .owner(),
            ));
        }
        let animatable = matches!(
            callback,
            C::GrindCrouchUpdate | C::ControlFadeBegin | C::ControlFadeUpdate
        );
        let skater_anim = matches!(
            callback,
            C::CreateGrindAttributesUpdate | C::ControlFadeBegin | C::ControlFadeUpdate
        );
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        if animatable || skater_anim {
            let retained = actor
                .animation
                .as_ref()
                .filter(|_| actor.animation_published)
                .ok_or(Error::MissingOwner("grinding IAnimatable"))?;
            let animation = retained.lock().map_err(|_| Error::Lock)?;
            if !animation
                .entity()
                .is_same_issuance(&self.actor.authority.entity_handle())
            {
                return Err(Error::ForeignOwner("grinding animation entity"));
            }
            for (index, needed, name) in [
                (0, animatable, "IAnimatable"),
                (2, skater_anim, "ISkaterAnim"),
            ] {
                if needed
                    && !actor.animation_interfaces[index]
                        .as_ref()
                        .is_some_and(|id| Arc::ptr_eq(id, animation.occurrence()))
                {
                    return Err(Error::ForeignOwner(name));
                }
            }
            drop(animation);
            self.animation = Some(Arc::clone(retained));
        }
        // IMotionGraph and ISkaterMotionGraph are the published graph checked above.
        let motion = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .ok_or(Error::MissingOwner("grinding Motion graph"))?;
        self.motion = Some(Arc::clone(motion.occurrence()));
        Ok(())
    }
    fn is_grinding(&mut self) -> Result<bool, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_filtered_output()
            .map_err(Error::PhysOut)?
            .is_grinding)
    }
    fn skating_grind_name(&mut self) -> Result<FastString30, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_filtered_output()
            .map_err(Error::PhysOut)?
            .skating_grind_name)
    }
    fn grind_crouch(&mut self) -> Result<f32, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_filtered_output()
            .map_err(Error::PhysOut)?
            .grind_crouch)
    }
    fn skateboard_flipped(&mut self) -> Result<bool, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        output
            .try_motion_skateboard_flipped()
            .map_err(Error::PhysOut)
    }
    fn deck_velocity(&mut self) -> Result<[f32; 3], Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_motion_deck_velocity()
            .map_err(Error::PhysOut)?
            .xyz)
    }
    fn deck_nose_direction(&mut self) -> Result<[f32; 3], Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_camera_board_output()
            .map_err(Error::PhysOut)?
            .deck_nose_dir
            .xyz)
    }
    fn centre_of_mass_to_deck(&mut self) -> Result<f32, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_animation_output()
            .map_err(Error::PhysOut)?
            .centre_of_mass_to_deck)
    }
    fn shoulder_twist(&mut self) -> Result<f32, Error> {
        let output = self.physical()?.phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_skeleton_output()
            .map_err(Error::PhysOut)?
            .shoulder_twist)
    }
    fn mirrored(&mut self) -> Result<bool, Error> {
        let animation = self.animation()?.lock().map_err(|_| Error::Lock)?;
        let player = animation
            .player()
            .ok_or(Error::MissingOwner("grinding SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(Error::MissingOwner("grinding mirrored flag"));
        }
        Ok(player.flags_bits & 0x4000_0000 != 0)
    }
    fn motion_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Error> {
        self.with_motion_graph(|graph| {
            graph
                .state
                .as_mut()
                .ok_or(Error::MissingOwner("grinding Motion state"))?
                .try_append_motion_attribute(name, value)
        })
    }
    fn intent(&mut self, name: FastString36) -> Result<Option<f32>, Error> {
        self.with_motion_graph(|graph| {
            Ok(graph
                .intents
                .as_ref()
                .ok_or(Error::MissingOwner("grinding Motion intents"))?
                .get(&name))
        })
    }
    fn scalar(&mut self, collection: Collection, key: u64) -> Result<f32, Error> {
        let attributes = self.attributes.ok_or(Error::MissingOwner(
            "GlobalAttributeCollection grinding anim_motion",
        ))?;
        // Port-side identity check against the actual IPhysical inputs; the
        // GrindControlFade Update itself never queries IPhysical.
        let physical = self
            .actor
            .cached_physical_allocation()
            .map_err(Error::Physical)?;
        if !Arc::ptr_eq(&attributes.vault, &physical.owner().inputs().vault) {
            return Err(Error::ForeignOwner("grinding anim_motion Vault"));
        }
        let instance = match collection {
            Collection::GrindCrouch => &attributes.crouch,
            Collection::ControlFade => &attributes.control_fade,
        };
        let word = instance
            .try_scalar_word(key, 0)
            .map_err(|error| Error::Load(MotionLoadError::AttributeRuntime(error)))?
            .unwrap_or(0); // accessor DefaultDataArea4
        Ok(f32::from_bits(word))
    }
    fn cvar(&mut self, cvar: GrindingCvar) -> Result<bool, Error> {
        self.cvars()?
            .try_grind_bool(match cvar {
                GrindingCvar::UsePhysInducedGrindCrouch => GrindCvar::kUsePhysInducedGrindCrouch,
                GrindingCvar::ShowGrindCrouchText => GrindCvar::kShowGrindCrouchText,
                GrindingCvar::ShowGrindControlFade => GrindCvar::kShowGrindControlFade,
            })
            .map_err(Error::Cvar)
    }
    fn dt(&mut self) -> Result<f32, Error> {
        Ok(self.dt)
    }
    fn settable(&mut self, name: FastString30, value: f32, normalized: bool) -> Result<(), Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_add_gesture_attribute(name, value, normalized, -1)
            .map_err(Error::AnimationWrite)
    }
    fn set_attributes(&mut self) -> Result<(), Error> {
        let cvars = self.cvars()?;
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_skater_set_attributes(cvars)
            .map_err(Error::AnimationPhase)
    }
    fn base_animation(&mut self) -> Result<BodyAnimationReference, Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .base()
            .and_then(|base| base.body_root.clone())
            .ok_or(Error::MissingOwner("grinding base animation"))
    }
    fn base_attribute(
        &mut self,
        base: &BodyAnimationReference,
        name: FastString30,
    ) -> Result<Option<f32>, Error> {
        self.animation()?
            .lock()
            .map_err(|_| Error::Lock)?
            .motion_base_attribute(base, name, 0x0f)?
            .map(|attribute| {
                attribute
                    .scalar_value()
                    .ok_or(Error::MissingOwner("grinding scalar base attribute"))
            })
            .transpose()
    }
    fn debug_overlay(&mut self) -> Result<(), Error> {
        self.overlay
            .ok_or(Error::MissingOwner("grinding DebugOverlay"))?
            .require()
            .map_err(|_| Error::MissingOwner("grinding DebugOverlay"))?;
        Ok(())
    }
    fn assertion(&mut self, site: &'static str) -> Error {
        Error::MissingOwner(site)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Begin,
    Update,
}

/// Services shared by the three grinding behaviours.
pub(crate) struct GrindingRuntime<'a> {
    pub(crate) attributes: Option<&'a GrindingMotionAttributes>,
    pub(crate) overlay: Option<&'a DebugOverlaySlot>,
    pub(crate) cvars: Option<&'a InputCvarReferences>,
    pub(crate) dt: f32,
}

impl ActorConstruction {
    fn grinding_source<'a>(
        &'a self,
        context: &'a BoundMotionGraphContext<ActorIdentity>,
        runtime: &GrindingRuntime<'a>,
    ) -> Source<'a> {
        Source {
            actor: self,
            context,
            attributes: runtime.attributes,
            overlay: runtime.overlay,
            cvars: runtime.cvars,
            dt: runtime.dt,
            physical: None,
            animation: None,
            motion: None,
        }
    }

    /// `CurrentGrabType::ActivationFunc`82c950d0: UCOM ISkaterAnim, false when
    /// absent, then `IsCurrentGrabType`82cb77a0 (+0xee8 == grab).
    pub(crate) fn motion_runtime_current_grab_type(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        grab: crate::generated_motion_load::construct::names::GrabType,
    ) -> Result<bool, Error> {
        self.check_motion_runtime_context(context)?;
        if self.prefix().map_err(Error::Actor)?.animation_interfaces[2].is_none() {
            return Ok(false);
        }
        let current = self
            .lifecycle_animation(2)?
            .lock()
            .map_err(|_| Error::Lock)?
            .try_grab_type()
            .map_err(Error::MissingOwner)?;
        Ok(current == grab as u32)
    }

    /// `IsGrindBluntingBackslash::ActivationFunc`82ca7d30: asserted IPhysical,
    /// PhysOut Grinds+0x88 `mGrindType` == `GRIND_TYPE_BACKSLASH` (4).
    pub(crate) fn motion_runtime_is_grind_blunting_backslash(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, Error> {
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(Error::Physical)?;
        let output = physical.owner().phys_out();
        let output = output.lock().map_err(|_| Error::Lock)?;
        Ok(output
            .try_camera_grinds_output()
            .map_err(Error::PhysOut)?
            .grind_type
            == crate::physics::GrindType::Backslash)
    }

    /// CreateGrindAttributes: null CreateInstance82bb3da8, Begin/End 825187c0.
    pub(crate) fn motion_runtime_create_grind_attributes(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        runtime: &GrindingRuntime<'_>,
    ) -> Result<(), Error> {
        kernel::create_grind_attributes_update(&mut self.grinding_source(context, runtime))
    }

    pub(crate) fn motion_runtime_control_grind_crouch(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        runtime: &GrindingRuntime<'_>,
        instance: Option<&mut kernel::GrindCrouchInstance>,
        drive_distance_to_com: FastString30,
        phase: Phase,
    ) -> Result<(), Error> {
        let mut source = self.grinding_source(context, runtime);
        match phase {
            Phase::Begin => kernel::grind_crouch_begin(&mut source, instance),
            Phase::Update => {
                kernel::grind_crouch_update(&mut source, instance, drive_distance_to_com)
            }
        }
    }

    pub(crate) fn motion_runtime_grind_control_fade(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        runtime: &GrindingRuntime<'_>,
        instance: Option<&mut kernel::GrindControlFadeInstance>,
        names: &kernel::ControlFadeNames,
        phase: Phase,
    ) -> Result<(), Error> {
        let mut source = self.grinding_source(context, runtime);
        match phase {
            Phase::Begin => kernel::control_fade_begin(&mut source, instance, names),
            Phase::Update => kernel::control_fade_update(&mut source, instance, names),
        }
    }
}
