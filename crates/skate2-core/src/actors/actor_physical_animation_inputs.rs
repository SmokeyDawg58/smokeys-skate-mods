//! Constructor-local animation pose and parameters for the actual physical pair.
use super::{ActorConstruction, ActorConstructionError};
use crate::{
    actor_animation_construction::initialization::bind_pose::{
        ActorBindPoseError, BindPoseError, OwnedAnimationPoseBuffer,
    },
    actor_construction::{
        anim_out_phys_in_mount::ActorAnimOutError,
        animation_start::ActorAnimationStartError,
        body_session::{ActorBodyAnimationServices, BodySessionError},
        input_generation::runtime::ActorInputRuntimeServices,
        player_animation::{ActorAnimationPlayerServices, PlayerAnimationError},
    },
    actor_face_playback::{ActorFaceError, AnimationPhysicsRef, FaceDatabase},
    physical_player_assembly::PhysicalPlayerAssemblyInputs,
};
use std::sync::{Arc, Mutex};

pub(crate) struct ActorPhysicalAnimationInputs {
    pub(crate) constructor_flags:
        crate::actor_construction::player_animation::PhysicalConstructorFlags,
    pub(crate) input_services: ActorInputRuntimeServices,
    pub(crate) body_services: ActorBodyAnimationServices,
    pub(crate) hierarchy: Arc<Mutex<FaceDatabase>>,
    pub(crate) parameters: AnimationPhysicsRef,
    /// Native constructor temporary: release after the physical constructor.
    pub(crate) pose: OwnedAnimationPoseBuffer,
}
#[derive(Debug)]
pub(crate) enum PhysicalAnimationInputsError {
    Actor(ActorConstructionError),
    Initialize(ActorAnimationStartError<PlayerAnimationError>),
    PhysicsInput(ActorAnimOutError),
    BindPose(ActorBindPoseError<PlayerAnimationError>),
    Pose(BindPoseError),
    Animation(ActorFaceError),
    Body(BodySessionError),
    ConstructorFlags(PlayerAnimationError),
    Settings(crate::players_backend_settings::PlayersSettingsError),
    Missing(&'static str),
    Lock,
    ForeignHierarchy,
}
impl std::fmt::Display for PhysicalAnimationInputsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(e) => write!(f, "physical-animation Actor: {e}"),
            Self::Initialize(e) => write!(f, "physical-animation initialize: {e}"),
            Self::PhysicsInput(e) => write!(f, "animation physics input: {e}"),
            Self::BindPose(e) => write!(f, "constructor bind pose: {e:?}"),
            Self::Pose(e) => write!(f, "constructor pose storage: {e:?}"),
            Self::Animation(e) => write!(f, "constructor animation data: {e:?}"),
            Self::Body(e) => write!(f, "constructor body services: {e}"),
            Self::Settings(e) => write!(f, "physical constructor settings: {e:?}"),
            Self::ConstructorFlags(e) => write!(f, "physical constructor flags: {e}"),
            Self::Missing(name) => write!(f, "missing constructor {name}"),
            Self::Lock => f.write_str("constructor animation lock poisoned"),
            Self::ForeignHierarchy => f.write_str("constructor pose has a different hierarchy"),
        }
    }
}
impl ActorPhysicalAnimationInputs {
    /// Project the retained constructor animation without placeholder assembly assets.
    /// The caller supplies the actual world attributes, selected spawn and hat.
    pub(crate) fn try_assembly_inputs(
        &self,
        cvars: crate::input_cvar_runtime::InputCvarReferences,
        vault: Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        spawn: [f32; 16],
        hat_type: impl FnOnce() -> Result<u32, PhysicalAnimationInputsError>,
    ) -> Result<PhysicalPlayerAssemblyInputs, PhysicalAnimationInputsError> {
        let hierarchy = self.try_constructor_hierarchy()?;
        let parameters = self
            .parameters
            .snapshot()
            .map_err(PhysicalAnimationInputsError::Animation)?;
        let pose_hierarchy = Arc::clone(&hierarchy);
        let pose_matrices = self.try_constructor_pose()?;
        // Native Actor ff2c0 reads hat only after the produced bind pose.
        let hat_type = hat_type()?;
        Ok(PhysicalPlayerAssemblyInputs {
            cvars,
            vault,
            hierarchy,
            parameters,
            pose_hierarchy,
            pose_matrices,
            spawn,
            hat_type,
            bouncy_ragdoll: self.constructor_flags.bouncy_ragdoll,
            is_human: self.constructor_flags.is_human,
        })
    }

    pub(crate) fn try_apply_to(
        &self,
        inputs: &mut PhysicalPlayerAssemblyInputs,
    ) -> Result<(), PhysicalAnimationInputsError> {
        // Retain the existing caller's partial writes at each fallible boundary.
        inputs.hierarchy = self.try_constructor_hierarchy()?;
        inputs.parameters = self
            .parameters
            .snapshot()
            .map_err(PhysicalAnimationInputsError::Animation)?;
        inputs.pose_hierarchy = Arc::clone(&inputs.hierarchy);
        inputs.pose_matrices = self.try_constructor_pose()?;
        inputs.bouncy_ragdoll = self.constructor_flags.bouncy_ragdoll;
        inputs.is_human = self.constructor_flags.is_human;
        Ok(())
    }

    fn try_constructor_hierarchy(
        &self,
    ) -> Result<Arc<skate2_formats::andale::AndaleHierarchy>, PhysicalAnimationInputsError> {
        use PhysicalAnimationInputsError as E;
        if !self
            .pose
            .hierarchy
            .as_ref()
            .is_some_and(|owner| Arc::ptr_eq(owner, &self.hierarchy))
        {
            return Err(E::ForeignHierarchy);
        }
        let hierarchy = self
            .hierarchy
            .lock()
            .map_err(|_| E::Lock)?
            .parsed
            .as_ref()
            .ok_or(E::Missing("parsed hierarchy"))?
            .hierarchy
            .clone();
        Ok(Arc::new(hierarchy))
    }

    fn try_constructor_pose(
        &self,
    ) -> Result<Arc<[crate::animation::AnimationMatrix]>, PhysicalAnimationInputsError> {
        use PhysicalAnimationInputsError as E;
        Ok(Arc::from(
            self.pose
                .matrices()
                .map_err(E::Pose)?
                .ok_or(E::Missing("evaluated bind pose"))?,
        ))
    }
}
impl ActorConstruction {
    /// `Actor825fef00..ff16c`: existing initialization, physics input, actual
    /// `phys_Tpose` lookup and bind-pose evaluation; no diagnostic frame dispatch.
    pub(crate) fn try_initialize_physical_animation_inputs(
        &self,
        source: &mut ActorAnimationPlayerServices<'_>,
        input_services: ActorInputRuntimeServices,
    ) -> Result<ActorPhysicalAnimationInputs, PhysicalAnimationInputsError> {
        use PhysicalAnimationInputsError as E;
        let attributes = source.startup.attributes.as_ref().map(Arc::clone);
        self.try_initialize_animation(source, attributes)
            .map_err(E::Initialize)?;
        self.try_construct_animation_physics_input()
            .map_err(E::PhysicsInput)?;
        let retained = self
            .retained_animation()
            .map_err(E::Actor)?
            .ok_or(E::Missing("animation"))?;
        let mut animation = retained.lock().map_err(|_| E::Lock)?;
        let initialized = animation
            .initialization()
            .ok_or(E::Missing("initialization"))?;
        let hierarchy = Arc::clone(
            initialized
                .hierarchy
                .as_ref()
                .ok_or(E::Missing("hierarchy"))?,
        );
        let playback = initialized
            .playback
            .as_ref()
            .ok_or(E::Missing("playback"))?;
        let parameters = playback
            .lock()
            .map_err(|_| E::Lock)?
            .get_physics_parameters(
                crate::actor_graph_construction::ActorGraphIntents::native_name(b"phys_Tpose"),
            )
            .map_err(E::Animation)?
            .ok_or(E::Missing("phys_Tpose parameters"))?;
        // Native flags are sampled after physics parameters, before GetBindpose.
        let player = self.player_index;
        let constructor_flags = source
            .try_physical_constructor_flags(player)
            .map_err(E::ConstructorFlags)?;
        let mut pose = OwnedAnimationPoseBuffer::new();
        animation
            .try_get_bind_pose(&mut pose, source)
            .map_err(E::BindPose)?;
        drop(animation);
        let body_services =
            ActorBodyAnimationServices::try_from_startup(source.startup, source.cvars)
                .map_err(E::Body)?;
        Ok(ActorPhysicalAnimationInputs {
            constructor_flags,
            input_services,
            body_services,
            hierarchy,
            parameters,
            pose,
        })
    }
}
