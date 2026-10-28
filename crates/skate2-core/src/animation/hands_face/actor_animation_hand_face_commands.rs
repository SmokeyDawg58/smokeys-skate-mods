//! `SetUpBlendPoseStream` calls into the SAME buffered ACS collector and jobs.
use super::sqt_jobs::QueuedSqtProgram;
use super::{AnimationPointerState, HandFaceStreamCommands};
use crate::{
    actor_animation_construction::initialization::{
        bind_pose::{BindPoseCommand, BindPoseError, sqt_output::OwnedAnimationSqtBuffer},
        body_dispatch::tree_commands::TreeCommands,
    },
    actor_animation_tree::eval::AnimationEvalCommands,
    actor_face_playback::{
        ActorFaceError, AnimationPoseRef, FaceCommands, tree_database::clip_fetch::ClipFetchPlan,
    },
};
use skate2_formats::andale::AndaleHierarchy;
use std::sync::{Arc, Mutex};

pub(crate) struct StreamCommands {
    commands: Option<TreeCommands>,
    fixed: Arc<Mutex<FaceCommands>>,
    output: OwnedAnimationSqtBuffer,
    hierarchy: Arc<AndaleHierarchy>,
    clip: AnimationPointerState,
    /// ACS `mErrorCode` for this BeginCommands/EndCommands block. Once set,
    /// every later command is skipped and EndCommands82a289e0 rolls the block back.
    discarded: bool,
    /// Owner command count at BeginCommands, the EndCommands rollback point.
    begin_length: usize,
}
impl StreamCommands {
    pub(crate) fn new(
        fixed: Arc<Mutex<FaceCommands>>,
        output: OwnedAnimationSqtBuffer,
        clip: AnimationPointerState,
    ) -> Result<Self, BindPoseError> {
        let retained = fixed.lock().map_err(|_| ActorFaceError::Lock)?;
        let owner = Arc::clone(
            retained
                .buffered
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        let hierarchy = retained
            .database
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .hierarchy_snapshot()
            .ok_or(ActorFaceError::MissingRecord)?;
        drop(retained);
        let mut commands = TreeCommands::new(owner);
        commands.enable_reuse();
        Ok(Self {
            commands: Some(commands),
            fixed,
            output,
            hierarchy,
            clip,
            discarded: false,
            begin_length: 0,
        })
    }
    pub(crate) fn commands(&mut self) -> Result<&mut TreeCommands, BindPoseError> {
        self.commands
            .as_mut()
            .ok_or(BindPoseError::AlreadyInCommands)
    }
}
impl AnimationEvalCommands for StreamCommands {
    type Error = BindPoseError;
    fn fixed_sqt(
        &mut self,
        owner: &Arc<Mutex<crate::actor_face_playback::FaceCommands>>,
        id: u32,
    ) -> Option<Result<(), Self::Error>> {
        if self.discarded {
            return Some(Ok(()));
        }
        Some(
            self.commands()
                .and_then(|commands| commands.set_fixed_sqt(owner, id)),
        )
    }
    fn fetch(&mut self, plan: ClipFetchPlan) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.fetch(plan)
    }
    fn fetch_pose(&mut self, pose: &AnimationPoseRef) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.fetch_pose(pose)
    }
    fn n_blend(&mut self, values: &[f32]) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.emit_n_blend(values)
    }
    fn blend(&mut self, value: f32) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.blend(value)
    }
    fn channel_blend(&mut self, value: f32, channel_from_first: bool) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.channel_blend(value, channel_from_first)
    }
    fn add_sqt(&mut self, motion_a: bool) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.add_sqt(motion_a)
    }
    fn mirror(&mut self, value: u32) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        self.commands()?.mirror(value)
    }
}
impl HandFaceStreamCommands for StreamCommands {
    fn begin_stream(&mut self) -> Result<(), Self::Error> {
        let commands = self.commands()?;
        let mut owner = commands.owner.lock().map_err(|_| ActorFaceError::Lock)?;
        if !owner.constructed || owner.in_command_set == 0 || owner.in_commands != 0 {
            return Err(BindPoseError::AlreadyInCommands);
        }
        owner.in_commands += 1;
        owner.nblend.begin_commands();
        let length = owner.commands.len();
        drop(owner);
        self.begin_length = length;
        Ok(())
    }
    fn pose_frame(&mut self, pose: u32) -> Result<(), Self::Error> {
        if self.clip != AnimationPointerState::Null {
            return Err(BindPoseError::UnsupportedSetup);
        }
        if self.discarded {
            return Ok(());
        }
        // SetSQTById82a2c6a0: an ID at or past mFixedSQTs->mMax_Num sets
        // error 4 instead of calling SetDataById.
        let maximum = self
            .fixed
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .max_frames;
        if usize::try_from(pose).map_or(true, |pose| pose >= maximum) {
            self.discarded = true;
            return Ok(());
        }
        let fixed = Arc::clone(&self.fixed);
        self.commands()?.set_fixed_sqt(&fixed, pose)
    }

    fn extract_sqt(&mut self) -> Result<(), Self::Error> {
        if self.discarded {
            return Ok(());
        }
        let commands = self.commands()?;
        let sqt = commands.select_stream_extraction_root()?;
        commands
            .owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(BindPoseCommand::ExtractSqt { sqt });
        self.output.begin_extract()?;
        Ok(())
    }
    fn end_stream(&mut self) -> Result<(), Self::Error> {
        let commands = self
            .commands
            .take()
            .ok_or(BindPoseError::AlreadyInCommands)?;
        let owner = Arc::clone(&commands.owner);
        if self.discarded {
            // EndCommands82a289e0 with an error: the block's commands are
            // dropped and nothing is extracted, so the pose buffer keeps its
            // previous values.
            let mut owner = owner.lock().map_err(|_| ActorFaceError::Lock)?;
            if owner.in_commands == 0 || owner.in_command_set == 0 {
                return Err(BindPoseError::AlreadyInCommands);
            }
            owner.commands.truncate(self.begin_length);
            owner.in_commands -= 1;
            return Ok(());
        }
        owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .retain_sqt_program(QueuedSqtProgram {
                commands,
                output: super::sqt_jobs::matrix::ProgramOutput::Sqt(self.output.clone()),
                hierarchy: Arc::clone(&self.hierarchy),
            })
    }
}
