//! Selected host Actor prefix composition, not the native whole-manager schedule.
//! Mount as `actor_construction::selected_frame_session`.
use super::{
    ActorConstruction,
    action_runtime_session::ActorActionRuntimeError,
    anim_out_phys_in::publication::AnimationPhysicsPrepareError,
    animation_phase1::ActorAnimationPhase1Error,
    body_session::{ActorBodyAnimationServices, BodySessionError},
    hands_face_session::{HandsFaceSelection, HandsFaceSessionError, HandsFaceSessionOutputs},
    input_generation::runtime::{ActorInputRuntimeError, ActorInputRuntimeServices},
    motion_runtime_session::neutral_lifecycle::ForcePhysicsWriter,
    update_input_flags::SelectedActorUpdateError,
};
use crate::{
    actor_animation_construction::{
        frame_phase::AnimationPhaseDispatchError, initialization::body_dispatch::BodyDispatchError,
    },
    actor_face_playback::blend_pose_stream::BlendPoseTokenPool,
    generated_motion_load::runtime::MotionUpdateError,
    input_cvar_runtime::InputCvarReferences,
    players_backend_settings::PlayersBackendSettings,
    reckoning_numeric::ReckoningNumeric,
};

/// The enclosing scheduler supplies actual LOD and phase2 wait policy.
/// `HandsFaceSelection` remains live and is read only at native dispatch branches.
#[derive(Clone, Copy)]
pub(crate) struct SelectedActorFrameRequest<'a> {
    pub(crate) dt: f32,
    pub(crate) lod: i32,
    pub(crate) wait_for_bs_job: bool,
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) settings: &'a PlayersBackendSettings,
}

/// All owners are retained by the caller across ticks. In particular, creating a
/// new token pool here would discard outstanding stream/token generations.
pub(crate) struct SelectedActorFrameServices<'a, N, S> {
    pub(crate) body: &'a mut ActorBodyAnimationServices,
    pub(crate) input: &'a mut ActorInputRuntimeServices,
    pub(crate) motion_writer: &'a mut dyn ForcePhysicsWriter,
    pub(crate) hand_tokens: &'a mut BlendPoseTokenPool,
    pub(crate) selection: &'a mut S,
    pub(crate) numeric: &'a mut N,
    /// The retained camera manager behind `Simulation::GetCameraManager`.
    pub(crate) camera: &'a dyn super::animation_phase1::SkaterOpacityCamera,
}

#[derive(Debug)]
pub(crate) enum SelectedActorFrameError<N, S> {
    GenerateIntents(Box<ActorInputRuntimeError<N>>),
    Phase1(Box<ActorAnimationPhase1Error>),
    Action(Box<ActorActionRuntimeError>),
    ClearMotionAttributes(Box<MotionUpdateError>),
    Motion(Box<MotionUpdateError>),
    Phase2(Box<AnimationPhaseDispatchError<BodySessionError>>),
    Phase3(Box<AnimationPhaseDispatchError<BodySessionError>>),
    BodySync(Box<BodyDispatchError<BodySessionError>>),
    HandsFace(Box<HandsFaceSessionError<S>>),
    SetUpPhysics(Box<AnimationPhysicsPrepareError<N>>),
    AfterPhysicsSetup(Box<SelectedActorUpdateError>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for SelectedActorFrameError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GenerateIntents(error) => {
                write!(f, "selected Actor prefix stage GenerateIntents: {error:?}")
            }
            Self::Phase1(error) => write!(f, "selected Actor prefix stage Phase1: {error:?}"),
            Self::Action(error) => write!(f, "selected Actor prefix stage Action: {error:?}"),
            Self::ClearMotionAttributes(error) => write!(
                f,
                "selected Actor prefix stage ClearMotionAttributes: {error:?}"
            ),
            Self::Motion(error) => write!(f, "selected Actor prefix stage Motion: {error:?}"),
            Self::Phase2(error) => write!(f, "selected Actor prefix stage Phase2: {error:?}"),
            Self::Phase3(error) => write!(f, "selected Actor prefix stage Phase3: {error:?}"),
            Self::BodySync(error) => write!(f, "selected Actor prefix stage BodySync: {error:?}"),
            Self::HandsFace(error) => write!(f, "selected Actor prefix stage HandsFace: {error:?}"),
            Self::SetUpPhysics(error) => {
                write!(f, "selected Actor prefix stage SetUpPhysics: {error:?}")
            }
            Self::AfterPhysicsSetup(error) => write!(
                f,
                "selected Actor prefix stage AfterPhysicsSetup: {error:?}"
            ),
        }
    }
}

impl ActorConstruction {
    /// Compose existing `GenerateActionGraphIntents825fda28`, Phase1 825f41a8,
    /// generated Action/Motion, animation phases and ACS synchronization, then
    /// `SetUpPhysics825ff6c0` and selected Actor.Update825f10f0..1140.
    ///
    /// Requires already constructed/mounted Actor, animation and physics
    /// auxiliaries. It does not construct settings or advance the physical
    /// Update1 consumer. Earlier source writes persist if a later stage fails;
    /// replaying this entire function is not a rollback/retry strategy.
    pub(crate) fn try_selected_actor_phase1<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        request: SelectedActorFrameRequest<'_>,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<(), SelectedActorFrameError<N::Error, S::Error>> {
        use SelectedActorFrameError as E;
        let SelectedActorFrameRequest { dt, cvars, .. } = request;
        self.try_generate_runtime_action_intents(dt, services.input, services.numeric)
            .map_err(|e| E::GenerateIntents(Box::new(e)))?;
        self.try_update_animation_phase1_prefix(
            dt,
            cvars,
            services.body,
            services.motion_writer,
            services.camera,
        )
        .map_err(|e| E::Phase1(Box::new(e)))?;
        self.try_update_action(dt)
            .map_err(|e| E::Action(Box::new(e)))?;
        self.try_clear_motion_attributes()
            .map_err(|e| E::ClearMotionAttributes(Box::new(e)))?;
        self.try_update_motion(dt, services.motion_writer)
            .map_err(|e| E::Motion(Box::new(e)))?;
        Ok(())
    }

    /// Continuation after Manager.Update2 in SimUpdate1 (8289f410).
    /// Splitting the existing composition keeps traffic perception before
    /// Skater.UpdateAnimation2/3 without copying any animation implementation.
    pub(crate) fn try_selected_actor_phase2_and_physics_setup<
        N: ReckoningNumeric,
        S: HandsFaceSelection,
    >(
        &self,
        request: SelectedActorFrameRequest<'_>,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<HandsFaceSessionOutputs, SelectedActorFrameError<N::Error, S::Error>> {
        self.try_selected_actor_phase2::<N, S>(request)?;
        self.try_selected_actor_phase3(request, services)?;
        let hands = self.try_selected_actor_hands(request, services)?;
        self.try_selected_actor_physics_sync(services)?;
        self.try_selected_actor_set_up_physics(request, services)?;
        // Actor.Update follows in the outer scheduler with the retained
        // LivingWorld services needed by its actual nonempty Chasee callback.
        Ok(hands)
    }

    /// `SkaterManager::UpdateAnimation2` 0x825fa310's `StartAnimUpdatePhase2` for this Actor.
    pub(crate) fn try_selected_actor_phase2<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        request: SelectedActorFrameRequest<'_>,
    ) -> Result<(), SelectedActorFrameError<N::Error, S::Error>> {
        self.try_update_animation_phase2(request.wait_for_bs_job, request.cvars)
            .map_err(|e| SelectedActorFrameError::Phase2(Box::new(e)))
    }

    /// `SkaterManager::UpdateAnimation3` 0x825fa4e0's `StartAnimUpdatePhase3` for this Actor.
    pub(crate) fn try_selected_actor_phase3<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        request: SelectedActorFrameRequest<'_>,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<(), SelectedActorFrameError<N::Error, S::Error>> {
        self.try_update_animation_phase3(request.dt, services.body, services.motion_writer)
            .map_err(|e| SelectedActorFrameError::Phase3(Box::new(e)))
    }

    /// `UpdateAnimation3`'s `DispatchHandsFaceAnim` for this Actor, inside the left and
    /// right hand ACS command sets every skater shares.
    pub(crate) fn try_selected_actor_hands<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        request: SelectedActorFrameRequest<'_>,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<HandsFaceSessionOutputs, SelectedActorFrameError<N::Error, S::Error>> {
        self.try_hands_face_session(request.lod, services.body, services.hand_tokens, services.selection)
            .map_err(|e| SelectedActorFrameError::HandsFace(Box::new(e)))
    }

    /// `SkaterManager::UpdatePhysics` 0x825ffd98's per-Actor ACS `Sync`, after every
    /// skater's `UpdateAnimation3`.
    pub(crate) fn try_selected_actor_physics_sync<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<(), SelectedActorFrameError<N::Error, S::Error>> {
        self.try_sync_body_frame(services.body)
            .map_err(|e| SelectedActorFrameError::BodySync(Box::new(e)))
    }

    /// `SkaterManager::UpdatePhysics` 0x825ffd98's `SetUpPhysics` for this Actor.
    pub(crate) fn try_selected_actor_set_up_physics<N: ReckoningNumeric, S: HandsFaceSelection>(
        &self,
        request: SelectedActorFrameRequest<'_>,
        services: &mut SelectedActorFrameServices<'_, N, S>,
    ) -> Result<(), SelectedActorFrameError<N::Error, S::Error>> {
        self.try_set_up_physics(request.dt, request.cvars, services.numeric, request.settings)
            .map_err(|e| SelectedActorFrameError::SetUpPhysics(Box::new(e)))
    }
}
