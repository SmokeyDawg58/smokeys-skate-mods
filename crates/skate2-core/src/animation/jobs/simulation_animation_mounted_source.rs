//! Host-owned mounted services for `Simulation::InitAnimation` at 0x828a5640.
//! The existing startup owner controls sampling and publication order.
use super::AnimationStartupSource;
use crate::{
    animation_job_owners::AnimationJobManager,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use skate2_formats::mount::{MountError, MountSet};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum MountedAnimationSourceError<T> {
    Thread(T),
    Cvar(InputCvarError),
    Mount(MountError),
}
impl<T: std::fmt::Debug> std::fmt::Display for MountedAnimationSourceError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Thread(error) => write!(f, "animation host thread: {error:?}"),
            Self::Cvar(error) => write!(f, "animation startup cvar: {error:?}"),
            Self::Mount(error) => write!(f, "animation mounted data: {error}"),
        }
    }
}

/// Retains the supplied owners without mounting archives, initializing jobs or
/// selecting a thread identity. The callback is sampled at the native call site.
pub(crate) struct MountedAnimationSource<T> {
    mounts: Arc<MountSet>,
    jobs: Arc<AnimationJobManager>,
    cvars: InputCvarReferences,
    current_thread: T,
}
impl<T> MountedAnimationSource<T> {
    pub(crate) fn new(
        mounts: Arc<MountSet>,
        jobs: Arc<AnimationJobManager>,
        cvars: InputCvarReferences,
        current_thread: T,
    ) -> Self {
        Self {
            mounts,
            jobs,
            cvars,
            current_thread,
        }
    }
}
impl<T, E> AnimationStartupSource for MountedAnimationSource<T>
where
    T: FnMut() -> Result<u64, E>,
{
    type Error = MountedAnimationSourceError<E>;

    fn current_thread_id(&mut self) -> Result<u64, Self::Error> {
        (self.current_thread)().map_err(MountedAnimationSourceError::Thread)
    }
    fn read_use_vbr(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_use_vbr()
            .map_err(MountedAnimationSourceError::Cvar)
    }
    fn read_animation(&mut self, source_path: &str) -> Result<Box<[u8]>, Self::Error> {
        self.mounts
            .read_path(source_path)
            .map(Vec::into_boxed_slice)
            .map_err(MountedAnimationSourceError::Mount)
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        Ok(Arc::clone(&self.jobs))
    }
}

#[cfg(test)]
#[path = "simulation_animation_mounted_source_tests.rs"]
mod tests;
