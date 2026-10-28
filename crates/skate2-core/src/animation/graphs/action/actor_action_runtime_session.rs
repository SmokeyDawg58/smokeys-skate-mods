//! Same Actor retained Action-controller Update session; no controller removal.
#[cfg(test)]
#[path = "actor_action_runtime_corpus.rs"]
pub(crate) mod corpus;
#[cfg(test)]
#[path = "actor_action_ground_corpus.rs"]
pub(crate) mod ground_corpus;
#[cfg(test)]
#[path = "actor_action_released_trick_corpus.rs"]
pub(crate) mod released_trick_corpus;
use super::{ActorConstruction, ActorConstructionError, Arc};
use crate::{
    dynamic_state_graph_controller::DynamicStateGraphContext,
    generated_action_load::{GeneratedActionAllocation, runtime::ActionUpdateError},
    state_graph_context::StateGraphContextHandle,
};

/// Shared selected GlobalAttributeCollection+128/+13c/+190 capabilities. Bind once
/// from the runtime Vault and share this owner across participating Actors.
pub(crate) struct ActionRuntimeServices {
    pub(crate) manual_angle: super::motion_runtime_session::manual_angle::ManualAngleAttributes,
    pub(crate) gesture: Option<
        Arc<super::action_motion_intent_bridge::gesture_runtime::GestureTrickRuntimeServices>,
    >,
    pub(crate) body_flip: super::action_motion_intent_bridge::ground::runtime::BodyFlipAttributes,
    pub(crate) trick_height: super::motion_runtime_session::set_trick_height::TrickHeightAttributes,
    pub(crate) pushing: super::action_motion_intent_bridge::disable_push::PushingMotionAttributes,
    pub(crate) bumps: super::motion_runtime_session::bump_conditions::BumpMotionAttributes,
    pub(crate) skitching: super::motion_runtime_session::skitch_conditions::SkitchMotionAttributes,
    pub(crate) body_tilt: crate::input_cvar_runtime::body_tilt::BodyTiltMotionAttributes,
    pub(crate) power_sliding: crate::input_cvar_runtime::power_sliding::PowerSlidingAttributes,
    pub(crate) prepare_land:
        crate::input_cvar_runtime::body_tilt::prepare_land::PrepareLandAttributes,
    pub(crate) body_spin: crate::input_cvar_runtime::body_spin_motion::BodySpinMotionAttributes,
    pub(crate) crouching: crate::input_cvar_runtime::crouching::CrouchingAttributes,
    pub(crate) grinding: super::motion_runtime_session::grinding::GrindingMotionAttributes,
    pub(crate) turning: crate::input_cvar_runtime::set_turning::TurningAttributes,
    pub(crate) kickturn: super::motion_runtime_session::kickturn_steering::KickTurnMotionAttributes,
    pub(crate) finger_flip: super::motion_runtime_session::finger_flip_out::FingerFlipMotionAttributes,
    pub(crate) pumping: crate::input_cvar_runtime::motion_pumping::PumpingAttributes,
    pub(crate) wipeout: super::motion_runtime_session::wipeout::WipeoutAttributes,
    pub(crate) overlay: crate::debug_overlay::DebugOverlaySlot,
}
impl ActionRuntimeServices {
    pub(crate) fn try_bind(
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        overlay: crate::debug_overlay::DebugOverlaySlot,
    ) -> Result<Self, crate::attribute_runtime::AttributeRuntimeError> {
        Ok(Self {
            manual_angle: super::motion_runtime_session::manual_angle::ManualAngleAttributes::try_bind(vault)?,
            gesture: None,
            wipeout: super::motion_runtime_session::wipeout::WipeoutAttributes::bind(vault),
            bumps: super::motion_runtime_session::bump_conditions::BumpMotionAttributes::try_bind(
                vault,
            )?,
            prepare_land:
                crate::input_cvar_runtime::body_tilt::prepare_land::PrepareLandAttributes::bind(
                    vault,
                ),
            body_tilt: crate::input_cvar_runtime::body_tilt::BodyTiltMotionAttributes::try_bind(
                vault,
            )?,
            power_sliding:
                crate::input_cvar_runtime::power_sliding::PowerSlidingAttributes::try_bind(vault)?,
            body_spin:
                crate::input_cvar_runtime::body_spin_motion::BodySpinMotionAttributes::try_bind(
                    vault,
                )?,
            overlay,
            crouching: crate::input_cvar_runtime::crouching::CrouchingAttributes::try_bind(vault)?,
            grinding: super::motion_runtime_session::grinding::GrindingMotionAttributes::try_bind(
                vault,
            )?,
            turning: crate::input_cvar_runtime::set_turning::TurningAttributes::try_bind(vault)?,
            kickturn: super::motion_runtime_session::kickturn_steering::KickTurnMotionAttributes::try_bind(vault)?,
            finger_flip: super::motion_runtime_session::finger_flip_out::FingerFlipMotionAttributes::try_bind(vault)?,
            pumping: crate::input_cvar_runtime::motion_pumping::PumpingAttributes::try_bind(vault)?,
            skitching:
                super::motion_runtime_session::skitch_conditions::SkitchMotionAttributes::bind(
                    vault,
                ),
            body_flip:
                super::action_motion_intent_bridge::ground::runtime::BodyFlipAttributes::try_bind(
                    vault,
                )?,
            trick_height:
                super::motion_runtime_session::set_trick_height::TrickHeightAttributes::try_bind(
                    vault,
                )?,
            pushing:
                super::action_motion_intent_bridge::disable_push::PushingMotionAttributes::try_bind(
                    vault,
                )?,
        })
    }
}

#[derive(Debug)]
pub(crate) enum ActorActionRuntimeError {
    Actor(ActorConstructionError),
    MissingController,
    ForeignController,
    ControllerLock,
    ServicesAlreadyInstalled,
    Update(ActionUpdateError),
    UpdateAndWriteback {
        update: Box<ActionUpdateError>,
        writeback: Box<Self>,
    },
}

/// Actual corpus Actor remains registered but not cached until full `HiLOD` return.
#[cfg(test)]
pub(crate) fn check_unpublished(actor: &ActorConstruction) {
    use super::{
        action_motion_intent_bridge::ActionMotionIntentError,
        physical_mount::ActorPhysicalMountError,
    };
    use crate::actor_graph_construction::ActorGraphIntents;
    assert!(matches!(
        actor.cached_physical_allocation(),
        Err(ActorPhysicalMountError::ConstructorNotPublished)
    ));
    let (controller, action_before, motion_before, force_brake_present) = {
        let state = actor.prefix().unwrap();
        let graphs = state.graphs.as_ref().unwrap();
        let action = graphs.action().unwrap();
        let wrapper = action.controller.as_ref().unwrap();
        assert!(wrapper.context().instance().is_none());
        let snapshot = |map: &ActorGraphIntents| {
            map.iter()
                .map(|entry| (entry.name, entry.value.to_bits()))
                .collect::<Vec<_>>()
        };
        (
            wrapper.generated_action().unwrap(),
            snapshot(action.intents.as_ref().unwrap()),
            snapshot(graphs.motion().unwrap().intents.as_ref().unwrap()),
            action
                .intents
                .as_ref()
                .unwrap()
                .get(&ActorGraphIntents::native_name(b"ForceBrake"))
                .is_some(),
        )
    };
    {
        let retained = controller.lock().unwrap();
        assert_eq!(retained.current_state, u16::MAX);
        assert!(retained.condition_values.iter().all(Option::is_none));
    }
    let dt = 1.0_f32 / 60.0;
    assert!(matches!(
        actor.try_update_action_initial(dt),
        Err(ActorActionRuntimeError::Update(ActionUpdateError::Actor(
            ActionMotionIntentError::Physical(ActorPhysicalMountError::ConstructorNotPublished)
        )))
    ));
    {
        let retained = controller.lock().unwrap();
        assert_eq!(retained.current_state, u16::MAX);
        assert_eq!(retained.last_state, u16::MAX);
        assert_eq!(retained.delta_time.to_bits(), dt.to_bits());
        // First ForceBrake presence succeeds/caches. The next physical query
        // (condition1 if present, otherwise condition2) does not return.
        assert_eq!(retained.condition_active, [1, 0, 0, 0, 0]);
        assert_eq!(retained.condition_values[0], Some(force_brake_present));
        assert!(retained.condition_values[1..].iter().all(Option::is_none));
        assert!(retained.allow_evaluations);
        assert!(retained.time_in_state.is_empty());
        assert!(retained.current_behaviours.is_empty());
        assert!(
            retained
                .instance_slots
                .iter()
                .all(|slot| slot.behaviour.is_none() && slot.instance.is_none())
        );
    }
    let state = actor.prefix().unwrap();
    let graphs = state.graphs.as_ref().unwrap();
    let action = graphs.action().unwrap();
    let wrapper = action.controller.as_ref().unwrap();
    assert!(Arc::ptr_eq(
        &controller,
        &wrapper.generated_action().unwrap()
    ));
    assert!(wrapper.context().instance().is_none());
    assert_eq!(
        action
            .intents
            .as_ref()
            .unwrap()
            .iter()
            .map(|entry| (entry.name, entry.value.to_bits()))
            .collect::<Vec<_>>(),
        action_before
    );
    assert_eq!(
        graphs
            .motion()
            .unwrap()
            .intents
            .as_ref()
            .unwrap()
            .iter()
            .map(|entry| (entry.name, entry.value.to_bits()))
            .collect::<Vec<_>>(),
        motion_before
    );
}
impl std::fmt::Display for ActorActionRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "Action runtime Actor: {error}"),
            Self::Update(error) => write!(f, "Action runtime: {error}"),
            Self::UpdateAndWriteback { update, writeback } => write!(
                f,
                "Action Update failed: {update}; context publication failed: {writeback}"
            ),
            other => write!(f, "Action runtime session: {other:?}"),
        }
    }
}
impl ActorConstruction {
    pub(crate) fn try_install_action_services(
        &self,
        services: Arc<ActionRuntimeServices>,
    ) -> Result<(), ActorActionRuntimeError> {
        let mut retained = self
            .action_services
            .lock()
            .map_err(|_| ActorActionRuntimeError::ControllerLock)?;
        if let Some(current) = retained.as_ref() {
            return if Arc::ptr_eq(current, &services) {
                Ok(())
            } else {
                Err(ActorActionRuntimeError::ServicesAlreadyInstalled)
            };
        }
        *retained = Some(services);
        Ok(())
    }
    pub(crate) fn action_runtime_services(
        &self,
    ) -> Result<Option<Arc<ActionRuntimeServices>>, ActorConstructionError> {
        self.action_services
            .lock()
            .map(|s| s.clone())
            .map_err(|_| ActorConstructionError::Poisoned)
    }
    /// Same retained generated Action owner on every `Update82955c78`.
    pub(crate) fn try_update_action(&self, dt: f32) -> Result<(), ActorActionRuntimeError> {
        self.try_update_action_initial(dt)
    }

    pub(crate) fn try_update_action_initial(&self, dt: f32) -> Result<(), ActorActionRuntimeError> {
        self.try_update_action_initial_with_allocations(dt, &mut |_, _| {
            GeneratedActionAllocation::Host
        })
    }

    pub(crate) fn try_update_action_initial_with_allocations(
        &self,
        dt: f32,
        allocate: &mut impl FnMut(u16, usize) -> GeneratedActionAllocation,
    ) -> Result<(), ActorActionRuntimeError> {
        use ActorActionRuntimeError as E;
        let (controller, mut context) = {
            let state = self.prefix().map_err(E::Actor)?;
            let action = state
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.action())
                .filter(|action| state.action_graph_published && action.is_complete())
                .ok_or(E::MissingController)?;
            if !state.graph_interfaces[0]
                .as_ref()
                .is_some_and(|registered| Arc::ptr_eq(registered, action.occurrence()))
            {
                return Err(E::ForeignController);
            }
            let wrapper = action.controller.as_ref().ok_or(E::MissingController)?;
            (
                wrapper.generated_action().ok_or(E::MissingController)?,
                wrapper.context().copy_for_behaviour(),
            )
        };
        // Never hold Actor prefix while invoking actual callbacks: they resolve
        // this same Actor and its retained maps/physical/animation allocations.
        let mut retained = controller.lock().map_err(|_| E::ControllerLock)?;
        {
            let state = self.prefix().map_err(E::Actor)?;
            let wrapper = state
                .graphs
                .as_ref()
                .and_then(|graphs| graphs.action())
                .and_then(|action| action.controller.as_ref())
                .ok_or(E::MissingController)?;
            if !wrapper
                .generated_action()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &controller))
                || context.controller() != Some(wrapper.controller_identity())
            {
                return Err(E::ForeignController);
            }
        }
        let updated = retained.try_update(self, &mut context, dt, allocate);
        // Enter/Exit used shallow copies; generated StateUpdate changed this
        // original-context projection. Publish its reached instance on failure
        // as well, and retain the other five wrapper slots exactly.
        let written = (|| {
            let mut state = self.prefix().map_err(E::Actor)?;
            let wrapper = state
                .graphs
                .as_mut()
                .and_then(|graphs| graphs.action_mut())
                .and_then(|action| action.controller.as_mut())
                .ok_or(E::MissingController)?;
            if !wrapper
                .generated_action()
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, &controller))
                || context.controller() != Some(wrapper.controller_identity())
            {
                return Err(E::ForeignController);
            }
            wrapper.set_runtime_instance(
                context
                    .instance()
                    .map(StateGraphContextHandle::shallow_copy),
            );
            Ok(())
        })();
        match (updated, written) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(E::Update(error)),
            (Ok(()), Err(error)) => Err(error),
            (Err(update), Err(writeback)) => Err(E::UpdateAndWriteback {
                update: Box::new(update),
                writeback: Box::new(writeback),
            }),
        }
    }
}
