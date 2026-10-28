//! Selected admitted model's final ACS pass over the same replay/output owners.
use super::{ActorFaceError, PresentationAnimation, runtime};
use crate::{
    actor_animation_construction::{
        hands_face::{runtime::HandFaceRuntimeError, sqt_jobs::matrix::queue_matrix_pose},
        initialization::{
            ActorAnimationInitializeSource,
            bind_pose::{BindPoseError, sqt_output::OwnedAnimationSqtBuffer},
        },
    },
    actor_face_playback::FaceCommands,
    animation::AnimationMatrix,
};
use std::sync::{Arc, Mutex};

/// The actual current ACS resolved by the first read pass. This prevents the
/// second pass from substituting the constructor's cached playback manager.
pub(crate) struct DecodedPresentationPass {
    commands: Arc<Mutex<FaceCommands>>,
    replay: OwnedAnimationSqtBuffer,
    origin: Arc<Mutex<[Option<AnimationMatrix>; 9]>>,
}
#[derive(Debug)]
pub(crate) enum PresentationFinalPoseError<J> {
    Missing(&'static str),
    Animation(ActorFaceError),
    Commands(BindPoseError),
    Dispatch(HandFaceRuntimeError<J>),
    ForeignPresentation,
    WrongReplayRow(usize),
}
impl<J: std::fmt::Debug> std::fmt::Display for PresentationFinalPoseError<J> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "selected final pose missing {s}"),
            Self::Animation(e) => write!(f, "selected final pose owner: {e:?}"),
            Self::Commands(e) => write!(f, "selected final pose command: {e:?}"),
            Self::Dispatch(e) => write!(f, "selected final pose job: {e:?}"),
            Self::ForeignPresentation => {
                f.write_str("selected final pose belongs to another presentation owner")
            }
            Self::WrongReplayRow(row) => {
                write!(f, "selected final pose replay does not belong to row {row}")
            }
        }
    }
}
impl DecodedPresentationPass {
    pub(crate) fn from_read(
        animation: &PresentationAnimation,
        commands: Arc<Mutex<FaceCommands>>,
        replay: OwnedAnimationSqtBuffer,
    ) -> Self {
        Self {
            commands,
            replay,
            origin: Arc::clone(&animation.hips_transforms),
        }
    }
    fn validate<J>(
        &self,
        animation: &PresentationAnimation,
        row: usize,
    ) -> Result<(), PresentationFinalPoseError<J>> {
        if !Arc::ptr_eq(&self.origin, &animation.hips_transforms) {
            return Err(PresentationFinalPoseError::ForeignPresentation);
        }
        if !row
            .checked_mul(2)
            .and_then(|i| animation.replay.get(i))
            .is_some_and(|replay| replay.is_same_view(&self.replay))
        {
            return Err(PresentationFinalPoseError::WrongReplayRow(row));
        }
        Ok(())
    }
    /// Selected one-model `UpdateBodies827deb34..dec30`: second command set,
    /// SetSQT/SetMatrix/PoseWithMatMult/Extract, Sync, then saved hips overwrite.
    /// Host supplies the already admitted model and row; no membership is faked.
    pub(crate) fn try_finalize<J: ActorAnimationInitializeSource>(
        &self,
        animation: &PresentationAnimation,
        row: usize,
        inverse_bind: Arc<[AnimationMatrix]>,
        jobs: &mut J,
    ) -> Result<(), PresentationFinalPoseError<J::Error>> {
        use PresentationFinalPoseError as E;
        self.validate(animation, row)?;
        runtime::begin_command_set(&self.commands).map_err(E::Commands)?;
        let output = animation
            .interpolated
            .get(row)
            .ok_or(E::Missing("interpolated row"))?;
        queue_matrix_pose(&self.commands, &self.replay, inverse_bind, output)
            .map_err(E::Commands)?;
        runtime::end_command_set(&self.commands, jobs).map_err(E::Dispatch)?;
        runtime::sync_command_set(&self.commands, jobs).map_err(E::Dispatch)?;
        publish_saved_hips(animation, row)
    }
}
fn publish_saved_hips<J>(
    animation: &PresentationAnimation,
    row: usize,
) -> Result<(), PresentationFinalPoseError<J>> {
    use PresentationFinalPoseError as E;
    let hips = animation
        .hips_transforms
        .lock()
        .map_err(|_| E::Animation(ActorFaceError::Lock))?
        .get(row)
        .copied()
        .flatten()
        .ok_or(E::Missing("saved hips transform"))?;
    let output = animation
        .interpolated
        .get(row)
        .ok_or(E::Missing("interpolated row"))?;
    output
        .try_update_root_matrix(|root| *root = hips)
        .map_err(E::Commands)
}

#[cfg(test)]
#[path = "skater_presentation_final_pose_tests.rs"]
mod tests;
