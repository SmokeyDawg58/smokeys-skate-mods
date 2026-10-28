//! Actor-owned animation allocation and interface registration, 825feccc..feefc.
//! Stops at the actual constructor continuation; Initialize is a later stage.
use super::{
    ActorConstruction, ActorConstructionError, ActorConstructorPrefix, ActorGraphAuthority,
    ActorGraphRegistrationError, ActorIdentity, ActorPrefixPhase, Arc, AttributeRuntimeError,
    AttributeRuntimeVault, MotionGraphEntityAuthority, Mutex,
};
use crate::{
    actor_animation_construction::{
        ActorAnimationConstruction, ActorAnimationConstructionError, ActorAnimationInputs,
        ActorAnimationInterface, ActorAnimationPhase, ActorAnimationRegistry, AnimationLifetime,
    },
    actor_face_playback::{
        FaceAnimationManagers, FaceCommandManager, FaceDatabaseManager, FacePlaybackManager,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
};

#[cfg(test)]
#[path = "actor_animation_mount_tests.rs"]
mod tests;

pub(crate) type MountedActorAnimation =
    ActorAnimationConstruction<ActorIdentity, Mutex<PhysicalAssemblyWorld>>;

/// Allocation825fecdc has no constructor writes, but actual Sim category55
/// OR100 clears its 80 bytes. Preserve that allocator provenance.
pub(crate) struct ExternalPlayerResetStorage {
    pub(crate) transform: crate::skeleton_root_transforms::SkeletonRootMatrix,
    pub(crate) on_board: bool,
}

/// Live caller queries, sampled in the native order. The implementation must
/// retain the actual skater manager and player backend, rather than infer local
/// status from the usual player index zero.
pub(crate) trait ActorAnimationSelection {
    type Error;
    fn local_player_index(&mut self) -> Result<u32, Self::Error>;
    fn is_network_game(&mut self) -> Result<bool, Self::Error>;
    fn ghost_system_enabled(&mut self) -> Result<bool, Self::Error>;
    fn marquee_id(&mut self, player: u32) -> Result<u64, Self::Error>;
    fn character_is_male(&mut self, player: u32) -> Result<bool, Self::Error>;
    fn command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error>;
    fn playback_manager(&mut self) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error>;
    fn database_manager(&mut self) -> Result<Arc<Mutex<FaceDatabaseManager>>, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum ActorAnimationMountError<E> {
    Actor(ActorConstructionError),
    Selection(E),
    Attribute(AttributeRuntimeError),
    ForeignWorld,
    WorldLock,
    AnimationLock,
    Constructor(ActorAnimationConstructionError<ActorGraphRegistrationError>),
    ConstructorIncomplete,
}
impl<E: std::fmt::Debug> std::fmt::Display for ActorAnimationMountError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Actor(error) => write!(f, "animation Actor owner: {error}"),
            Self::Selection(error) => write!(f, "animation selection: {error:?}"),
            Self::Attribute(error) => write!(f, "animation marquee attribute: {error}"),
            Self::Constructor(error) => write!(f, "animation constructor: {error:?}"),
            other => write!(f, "animation mount: {other:?}"),
        }
    }
}

struct Registry<'a> {
    authority: &'a ActorGraphAuthority<ActorIdentity>,
    state: &'a mut ActorConstructorPrefix,
    player_index: u32,
}
impl ActorAnimationRegistry<ActorIdentity> for Registry<'_> {
    type Error = ActorGraphRegistrationError;
    fn register_animation_interface(
        &mut self,
        entity: &MotionGraphEntityAuthority<ActorIdentity>,
        kind: ActorAnimationInterface,
        occurrence: &Arc<()>,
    ) -> Result<(), Self::Error> {
        if !entity.is_same_issuance(&self.authority.entity_handle()) {
            return Err(ActorGraphRegistrationError::ForeignEntity);
        }
        let name = kind.source_name();
        if self.state.interfaces.binary_search(&name).is_ok() {
            return Err(ActorGraphRegistrationError::DuplicateInterface(name));
        }
        self.state
            .register::<std::convert::Infallible>(name)
            .map_err(|_| ActorGraphRegistrationError::Allocation)?;
        let index = match kind {
            ActorAnimationInterface::Animatable => 0,
            ActorAnimationInterface::ChannelAnimatable => 1,
            ActorAnimationInterface::SkaterAnim => 2,
        };
        self.state.animation_interfaces[index] = Some(Arc::clone(occurrence));
        Ok(())
    }
    fn actor_player_index(
        &mut self,
        entity: &MotionGraphEntityAuthority<ActorIdentity>,
    ) -> Result<u32, Self::Error> {
        if !entity.is_same_issuance(&self.authority.entity_handle()) {
            return Err(ActorGraphRegistrationError::ForeignEntity);
        }
        Ok(self.player_index)
    }
}

impl ActorConstruction {
    /// The camera's native `GetSkaterAnim` assertion concerns the published pointer,
    /// not a partial constructor allocation retained for failure ownership.
    pub(crate) fn published_camera_animation(
        &self,
    ) -> Result<Option<Arc<Mutex<MountedActorAnimation>>>, ActorConstructionError> {
        let state = self.prefix()?;
        Ok(state
            .animation
            .as_ref()
            .filter(|_| state.animation_published)
            .map(Arc::clone))
    }
    pub(crate) fn retained_animation(
        &self,
    ) -> Result<Option<Arc<Mutex<MountedActorAnimation>>>, ActorConstructionError> {
        Ok(self.prefix()?.animation.as_ref().map(Arc::clone))
    }
    /// Host-success allocation path, with original caller selection and SAME
    /// world/entity ownership. Allocation-failure parity is not represented.
    /// The returned allocation is retained even when later construction fails.
    pub(crate) fn try_construct_animation_prefix<S: ActorAnimationSelection>(
        &self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: &Arc<AttributeRuntimeVault>,
        selection: &mut S,
    ) -> Result<Arc<Mutex<MountedActorAnimation>>, ActorAnimationMountError<S::Error>> {
        // Check host ownership before beginning source constructor effects.
        if world
            .lock()
            .map_err(|_| ActorAnimationMountError::WorldLock)?
            .physical_manager_authority()
            != self.authority.entity_key().manager()
        {
            return Err(ActorAnimationMountError::ForeignWorld);
        }
        let mut state = self.prefix().map_err(ActorAnimationMountError::Actor)?;
        if state.phase != ActorPrefixPhase::GraphFacadesConstructed {
            return Err(ActorAnimationMountError::Actor(
                ActorConstructionError::GraphPrefixIncomplete,
            ));
        }
        state.phase = ActorPrefixPhase::AnimationSelecting;
        state.external_player_reset = Some(ExternalPlayerResetStorage {
            transform: crate::skeleton_root_transforms::SkeletonRootMatrix::known([0.0; 16]),
            on_board: false,
        });
        let local = selection
            .local_player_index()
            .map_err(ActorAnimationMountError::Selection)?
            == self.player_index;
        let low_cull = local
            && !selection
                .is_network_game()
                .map_err(ActorAnimationMountError::Selection)?
            && !selection
                .ghost_system_enabled()
                .map_err(ActorAnimationMountError::Selection)?;
        let marquee = selection
            .marquee_id(self.player_index)
            .map_err(ActorAnimationMountError::Selection)?;
        // Wrapper is constructed even for the CAC sentinel, before its branch.
        let instance = vault.instance(0x91bc_a669_3efc_9aa7, marquee);
        let face_name: Option<Arc<[u8]>> = if marquee == u64::MAX {
            Some(Arc::from(
                if selection
                    .character_is_male(self.player_index)
                    .map_err(ActorAnimationMountError::Selection)?
                {
                    &b"CAC_male"[..]
                } else {
                    &b"CAC_female"[..]
                },
            ))
        } else if instance.is_null() {
            None
        } else {
            instance
                .try_cached_layout_text(0)
                .map_err(ActorAnimationMountError::Attribute)?
                .map(|data| Arc::from(data.bytes()))
        };
        let animation = Arc::new(Mutex::new(ActorAnimationConstruction::new(
            &self.authority,
            ActorAnimationInputs {
                physics_world: Arc::clone(world),
                life: AnimationLifetime::Transient,
                use_low_blend_cull_threshold: low_cull,
                face_name,
            },
        )));
        state.animation = Some(Arc::clone(&animation));
        state.phase = ActorPrefixPhase::AnimationConstructing;
        animation
            .lock()
            .map_err(|_| ActorAnimationMountError::AnimationLock)?
            .try_mark_sim_allocation()
            .map_err(|_| ActorAnimationMountError::ConstructorIncomplete)?;
        // Actual source captures command, playback, then face database owners
        // after allocation but before any animation constructor interfaces.
        let command = selection
            .command_manager()
            .map_err(ActorAnimationMountError::Selection)?;
        let playback = selection
            .playback_manager()
            .map_err(ActorAnimationMountError::Selection)?;
        let database = selection
            .database_manager()
            .map_err(ActorAnimationMountError::Selection)?;
        animation
            .lock()
            .map_err(|_| ActorAnimationMountError::AnimationLock)?
            .try_bind_face_managers(FaceAnimationManagers {
                command,
                playback,
                database,
            })
            .map_err(|_| ActorAnimationMountError::ConstructorIncomplete)?;
        animation
            .lock()
            .map_err(|_| ActorAnimationMountError::AnimationLock)?
            .try_construct_prefix(&mut Registry {
                authority: &self.authority,
                state: &mut state,
                player_index: self.player_index,
            })
            .map_err(ActorAnimationMountError::Constructor)?;
        Ok(animation)
    }

    /// Actual +1fc publication follows constructor return, never just prefix
    /// registration. Selected nonnull faces must complete their real setup.
    pub(crate) fn try_publish_animation_constructor(
        &self,
    ) -> Result<(), ActorAnimationMountError<std::convert::Infallible>> {
        let mut state = self.prefix().map_err(ActorAnimationMountError::Actor)?;
        if state.phase != ActorPrefixPhase::AnimationConstructing {
            return Err(ActorAnimationMountError::ConstructorIncomplete);
        }
        let animation = state
            .animation
            .as_ref()
            .ok_or(ActorAnimationMountError::ConstructorIncomplete)?;
        if animation
            .lock()
            .map_err(|_| ActorAnimationMountError::AnimationLock)?
            .phase()
            != ActorAnimationPhase::ConstructorReturned
        {
            return Err(ActorAnimationMountError::ConstructorIncomplete);
        }
        state.animation_published = true;
        state
            .fields
            .as_mut()
            .expect("Actor fields precede animation")
            .animation_null = false;
        state.phase = ActorPrefixPhase::AnimationConstructorReturned;
        Ok(())
    }
}
