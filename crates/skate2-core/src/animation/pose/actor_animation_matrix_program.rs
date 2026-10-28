//! Selected buffered `SetMatrix` / `PoseWithMatMult` using the existing ACS jobs.
use super::{ActorFaceError, BindPoseError, QueuedSqtProgram, TreeCommands};
use crate::{
    actor_animation_construction::initialization::bind_pose::{
        BindPoseCommand, OwnedAnimationPoseBuffer, PoseOutputAllocation,
        sqt_output::OwnedAnimationSqtBuffer,
    },
    actor_face_playback::FaceCommands,
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose},
};
use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
use std::sync::{Arc, Mutex};

impl FaceCommands {
    /// RegisterFixedMatrix82a28968 copies exactly mNumExtBones matrices into
    /// the next slot of THIS ACS fixed allocation and returns that slot ID.
    pub(crate) fn register_fixed_matrix(&mut self, matrices: &[AnimationMatrix])
        -> Result<u32, BindPoseError> {
        let maximum = self.setup.as_ref().ok_or(BindPoseError::InitializationRequired)?.fixed_matrix as usize;
        if self.fixed_matrices.len() >= maximum {
            return Err(BindPoseError::UnsupportedSetup);
        }
        let database = self.database.lock().map_err(|_| ActorFaceError::Lock)?;
        let bones = database.parsed.as_ref().ok_or(ActorFaceError::MissingRecord)?.hierarchy.parents.len();
        let input = matrices.get(..bones).ok_or(BindPoseError::InvalidHierarchy)?;
        let id = u32::try_from(self.fixed_matrices.len()).map_err(|_| BindPoseError::Allocation)?;
        self.fixed_matrices.push(Arc::from(input));
        Ok(id)
    }
}

/// A native external matrix pointer retained until the actual `SetData` job.
/// The supplied allocation is the model's authored inverse-bind owner.
pub(crate) struct MatrixProgram {
    pub(crate) source: Arc<[AnimationMatrix]>,
    pub(crate) fixed: bool,
    pub(crate) sqt_extract: Option<OwnedAnimationSqtBuffer>,
    output: Arc<Mutex<PoseOutputAllocation>>,
}

pub(crate) enum ProgramOutput {
    Sqt(OwnedAnimationSqtBuffer),
    Matrix(MatrixProgram),
}
impl ProgramOutput {
    pub(crate) fn complete_sync(&self) -> Result<(), BindPoseError> {
        match self {
            Self::Sqt(output) => output.complete_sync().map_err(Into::into),
            Self::Matrix(program) => {
                let mut output = program.output.lock().map_err(|_| ActorFaceError::Lock)?;
                // Native callback pointers can name the prefix of a larger
                // Marquee allocation; this command only publishes its bones.
                if output.values.get(..program.source.len()).ok_or(BindPoseError::InvalidHierarchy)?
                    .iter().any(Option::is_none) {
                    return Err(BindPoseError::MissingBindPose);
                }
                output.synchronized = true;
                if let Some(sqt) = &program.sqt_extract { sqt.complete_sync()?; }
                Ok(())
            }
        }
    }
    pub(crate) fn extract_sqt(&self, values: &[AndaleSqt]) -> Result<(), BindPoseError> {
        let Self::Sqt(output) = self else {
            return Err(BindPoseError::UnsupportedSetup);
        };
        output.extract(values).map_err(Into::into)
    }
    #[cfg(test)]
    pub(crate) fn sqt(&self) -> &OwnedAnimationSqtBuffer {
        let Self::Sqt(output) = self else {
            panic!("expected SQT test output")
        };
        output
    }
}

impl MatrixProgram {
    /// Retain the existing matrix allocation until the queued pose job finishes.
    pub(crate) fn new(source: Arc<[AnimationMatrix]>, destination: &OwnedAnimationPoseBuffer)
        -> Result<Self, BindPoseError> {
        let output = destination.shared_storage()?;
        output.lock().map_err(|_| ActorFaceError::Lock)?.synchronized = false;
        Ok(Self { source, output, fixed: false, sqt_extract: None })
    }
    /// SetDataById82a24890 retains a registered matrix operand; it does not
    /// emit the external-pointer SetData producer.
    pub(crate) fn new_fixed(source: Arc<[AnimationMatrix]>, destination: &OwnedAnimationPoseBuffer)
        -> Result<Self, BindPoseError> {
        let mut result = Self::new(source, destination)?;
        result.fixed = true;
        Ok(result)
    }
    /// `ACSposeMatMult_job82a23e78` extracts only after the complete native
    /// hierarchy and affine inverse-bind multiplication have finished.
    pub(crate) fn execute(
        &self,
        hierarchy: &AndaleHierarchy,
        sqts: &[AndaleSqt],
        matrices: &[AnimationMatrix],
    ) -> Result<(), BindPoseError> {
        let mut skeleton = AnimationSkeleton::from_andale(hierarchy)
            .map_err(|_| BindPoseError::InvalidHierarchy)?;
        skeleton.trajectory_bone = Some(0); // Actual presentation ACS descriptor.
        let global = skeleton
            .global_matrices(&LocalPose::from_andale(sqts))
            .map_err(|_| BindPoseError::InvalidHierarchy)?;
        if global.len() != matrices.len() {
            return Err(BindPoseError::InvalidHierarchy);
        }
        let posed = global
            .into_iter()
            .zip(matrices)
            .map(|(global, inverse)| {
                // `PoseWithMatMult82a23a5c..afc` uses the same affine graph as
                // hierarchy composition: P starts with the parent's translation.
                crate::animation::compose_pose_matrix(global, *inverse)
            })
            .collect::<Vec<_>>();
        let mut output = self.output.lock().map_err(|_| ActorFaceError::Lock)?;
        if output.values.len() < posed.len() {
            return Err(BindPoseError::InvalidHierarchy);
        }
        for (slot, matrix) in output.values.iter_mut().zip(posed) {
            *slot = Some(matrix);
        }
        Ok(())
    }
}

/// The selected `UpdateBodies827deb70..ba8` four-command sequence. The host
/// supplies the admitted model's actual inverse-bind allocation; this does not
/// synthesize native model membership or own another presentation pose.
pub(crate) fn queue_matrix_pose(
    commands: &Arc<Mutex<FaceCommands>>,
    sqt: &OwnedAnimationSqtBuffer,
    inverse_bind: Arc<[AnimationMatrix]>,
    destination: &OwnedAnimationPoseBuffer,
) -> Result<(), BindPoseError> {
    let retained = commands.lock().map_err(|_| ActorFaceError::Lock)?;
    let owner = retained
        .buffered
        .clone()
        .ok_or(BindPoseError::BufferedOwnerRequired)?;
    let hierarchy = retained
        .database
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .hierarchy_snapshot()
        .ok_or(ActorFaceError::MissingRecord)?;
    drop(retained);
    {
        let mut buffered = owner.lock().map_err(|_| ActorFaceError::Lock)?;
        if !buffered.constructed || buffered.in_command_set == 0 || buffered.in_commands != 0 {
            return Err(BindPoseError::AlreadyInCommands);
        }
        buffered.in_commands += 1;
    }
    let mut tree = TreeCommands::new(Arc::clone(&owner));
    tree.set_external_sqt(sqt)?;
    // Selected graph has one dynamic matrix. Native `SetData82a2c84c`
    // allocates it before usage checks and retains the pointer without copying.
    {
        let mut buffered = owner.lock().map_err(|_| ActorFaceError::Lock)?;
        if buffered.max_dynamic_matrices == 0 {
            return Err(BindPoseError::UnsupportedSetup);
        }
        buffered
            .commands
            .push(BindPoseCommand::SetExternalMatrix { matrix: 0 });
        // One pose op is emitted by this selected graph. The matrix result
        // reuses the dynamic SetMatrix slot, without a second allocation.
        if buffered.max_pose_operations == 0 {
            return Err(BindPoseError::UnsupportedSetup);
        }
        buffered.commands.push(BindPoseCommand::PoseMatMult {
            sqt: tree.output()?,
            matrix: 0,
        });
        buffered
            .commands
            .push(BindPoseCommand::ExtractMatrix { matrix: 0 });
    }
    let program = MatrixProgram::new(inverse_bind, destination)?;
    owner
        .lock()
        .map_err(|_| ActorFaceError::Lock)?
        .retain_sqt_program(QueuedSqtProgram {
            commands: tree,
            output: ProgramOutput::Matrix(program),
            hierarchy,
        })
}

#[cfg(test)]
#[path = "actor_animation_matrix_program_tests.rs"]
mod tests;
