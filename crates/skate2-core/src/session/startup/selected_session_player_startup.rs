//! Selected mounted player/Actor constructor over the existing owners.
//! Presentation remains a separate continuation after the caller's registration work.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits selected session caller")
)]

use crate::{
    actor_construction::selected_startup::{
        SelectedPlayerConstructionRequest, SelectedPlayerStartup,
        mounted::{MountedSelectedError, MountedSelectedOwners},
    },
    actor_face_playback::FaceAnimationManagers,
    animation_job_owners::{AnimationJobManager, execution::AnimationExecutionError},
    attribute_runtime::AttributeRuntimeVault,
    input_cvar_runtime::InputCvarReferences,
    input_map::SelectedInputMap,
    juice_trick_input::JuiceTrickInput,
    physical_player_assembly::PhysicalAssemblyWorld,
    player::CharacterRegistrationRequest,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    simulation_animation_startup::mounted_source::{
        MountedAnimationSource, MountedAnimationSourceError,
    },
    skater_manager_local_identity::LocalIdentityError,
};
use skate2_formats::mount::MountSet;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum SelectedSessionPlayerStartupError<E> {
    WorldLock,
    MissingLocalIdentity,
    MissingRecipeRuntime,
    MissingGraphRuntime,
    LocalIdentity(LocalIdentityError),
    InvalidLocalIndex(i32),
    AnimationJobs(AnimationExecutionError),
    Mounted(MountedSelectedError<MountedAnimationSourceError<E>, ReckoningHostNumericError>),
}

/// Owns the same mounted source across Actor construction and the later
/// presentation continuation. No archive, Vault, or world is reconstructed.
pub(crate) struct SelectedSessionPlayerStartup<T> {
    pub(crate) selected: SelectedPlayerStartup,
    pub(crate) source: MountedAnimationSource<T>,
    pub(crate) registration_request: CharacterRegistrationRequest,
}

impl<T> SelectedSessionPlayerStartup<T> {
    /// The host selects its mount, world, spawn, marquee, flags and thread
    /// answer. Failure ends this boot attempt; partial native owner writes are
    /// not replayed or presented as a completed session.
    pub(crate) fn try_new<E>(
        mounts: Arc<MountSet>,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        marquee: u64,
        request: SelectedPlayerConstructionRequest,
        current_thread: T,
    ) -> Result<Self, SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        Self::try_new_inner(
            mounts,
            world,
            vault,
            cvars,
            None,
            JuiceTrickInput::new_static_initialized(),
            marquee,
            request,
            current_thread,
        )
    }

    /// Production selected input passes the map loaded during `InputManager` Init
    /// and the same Juice occurrence installed as the device remap.
    #[expect(
        clippy::too_many_arguments,
        reason = "one selected constructor forwards distinct retained source owners and host choices"
    )]
    pub(crate) fn try_new_with_input_owner<E>(
        mounts: Arc<MountSet>,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        map: Arc<SelectedInputMap>,
        juice: JuiceTrickInput,
        marquee: u64,
        request: SelectedPlayerConstructionRequest,
        current_thread: T,
    ) -> Result<Self, SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        Self::try_new_inner(
            mounts,
            world,
            vault,
            cvars,
            Some(map),
            juice,
            marquee,
            request,
            current_thread,
        )
    }

    /// Stop after the mounted Register request and global Simulation animation
    /// startup. The selected facade can construct its shared recovery graph
    /// from these retained owners before it constructs Actor.
    #[expect(
        clippy::too_many_arguments,
        reason = "one selected constructor forwards distinct retained source owners and host choices"
    )]
    pub(crate) fn try_prepare_with_input_owner<E>(
        mounts: Arc<MountSet>,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        map: Arc<SelectedInputMap>,
        juice: JuiceTrickInput,
        marquee: u64,
        current_thread: T,
    ) -> Result<Self, SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        Self::try_prepare_inner(
            mounts,
            world,
            vault,
            cvars,
            Some(map),
            juice,
            marquee,
            current_thread,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "shared selected constructor retains distinct source owners and host choices"
    )]
    fn try_new_inner<E>(
        mounts: Arc<MountSet>,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        map: Option<Arc<SelectedInputMap>>,
        juice: JuiceTrickInput,
        marquee: u64,
        request: SelectedPlayerConstructionRequest,
        current_thread: T,
    ) -> Result<Self, SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let mut prepared = Self::try_prepare_inner(
            mounts,
            world,
            vault,
            cvars,
            map,
            juice,
            marquee,
            current_thread,
        )?;
        prepared.try_finish_actor(world, request)?;
        Ok(prepared)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "shared selected constructor retains distinct source owners and host choices"
    )]
    fn try_prepare_inner<E>(
        mounts: Arc<MountSet>,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        map: Option<Arc<SelectedInputMap>>,
        juice: JuiceTrickInput,
        marquee: u64,
        current_thread: T,
    ) -> Result<Self, SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        use SelectedSessionPlayerStartupError as Error;
        let skaters = Arc::clone(
            world
                .lock()
                .map_err(|_| Error::WorldLock)?
                .local_skater_identity
                .as_ref()
                .ok_or(Error::MissingLocalIdentity)?,
        );
        let local_index = skaters.try_local_index().map_err(Error::LocalIdentity)?;
        let player_index =
            u32::try_from(local_index).map_err(|_| Error::InvalidLocalIndex(local_index))?;
        let mut jobs = AnimationJobManager::new();
        jobs.try_initialize_local_backend()
            .map_err(Error::AnimationJobs)?;
        let mut source = MountedAnimationSource::new(
            Arc::clone(&mounts),
            Arc::new(jobs),
            cvars.clone(),
            current_thread,
        );
        let mut selected = SelectedPlayerStartup::new_mounted(
            player_index,
            MountedSelectedOwners {
                graphs: {
                    let guard=world.lock().map_err(|_|Error::WorldLock)?;
                    let living=guard.grind_services.living_world().lock().map_err(|_|Error::WorldLock)?;
                    Arc::clone(living.graph_runtime.as_ref().ok_or(Error::MissingGraphRuntime)?)
                },
                living_world_recipes: {
                    let guard=world.lock().map_err(|_|Error::WorldLock)?;
                    let living=guard.grind_services.living_world().lock().map_err(|_|Error::WorldLock)?;
                    Arc::clone(living.recipe_runtime.as_ref().ok_or(Error::MissingRecipeRuntime)?)
                },
                mounts,
                vault,
                cvars,
                skaters,
                animation_managers: FaceAnimationManagers::new(),
                juice,
                pause: crate::selected_pause_manager::SelectedPauseManager::construct(),
            },
        );
        selected.owners.map = map;
        let registration_request = selected
            .try_prepare_mounted_registration::<_, HostReckoningNumeric>(
                world,
                marquee,
                &mut source,
            )
            .map_err(Error::Mounted)?;
        Ok(Self {
            selected,
            source,
            registration_request,
        })
    }

    /// Finish the existing Actor path exactly once, using the prepared source
    /// and animation owners. Failed Actor state stays in this owner.
    pub(crate) fn try_finish_actor<E>(
        &mut self,
        world: &Arc<Mutex<PhysicalAssemblyWorld>>,
        request: SelectedPlayerConstructionRequest,
    ) -> Result<(), SelectedSessionPlayerStartupError<E>>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let selected_mounts = Arc::clone(&self.selected.owners.mounts);
        self.selected
            .try_initialize(
                world,
                &selected_mounts,
                request,
                &mut self.source,
                &mut HostReckoningNumeric,
            )
            .map_err(|error| {
                SelectedSessionPlayerStartupError::Mounted(MountedSelectedError::Selected(
                    Box::new(error),
                ))
            })
    }
}
