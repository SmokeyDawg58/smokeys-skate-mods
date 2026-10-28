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
    physical_player_assembly::PhysicalAssemblyWorld,
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
                mounts,
                vault,
                cvars,
                skaters,
                animation_managers: FaceAnimationManagers::new(),
                juice: crate::juice_trick_input::JuiceTrickInput::new_static_initialized(),
                pause: crate::selected_pause_manager::SelectedPauseManager::construct(),
            },
        );
        selected
            .try_initialize_mounted(
                world,
                marquee,
                request,
                &mut source,
                &mut HostReckoningNumeric,
            )
            .map_err(Error::Mounted)?;
        Ok(Self { selected, source })
    }
}
