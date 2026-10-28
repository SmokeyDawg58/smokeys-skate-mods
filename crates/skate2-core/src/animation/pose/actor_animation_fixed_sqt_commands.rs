//! Source fixed-SQT operands within the existing buffered command collector.
//! Parent registration/fields are coordinated with the integration gate.
use super::{ActorFaceError, AndaleSqt, Arc, BindPoseError, Command, Mutex, TreeCommands};
use crate::actor_face_playback::FaceCommands;
use std::borrow::Cow;

/// Host typed-command operand tag. Native commands store ID and fixed bit in
/// separate fields; low12 is the native ID and this tag is never asset data.
pub(super) const FIXED_SQT_BIT: u16 = 0x8000;
impl TreeCommands {
    pub(crate) fn output_is_fixed(&self) -> Result<bool, BindPoseError> {
        Ok(self.output()? & FIXED_SQT_BIT != 0)
    }

    /// SetSQTById82a2c6a0->SetDataById preserves a fixed reference, without
    /// consuming `GetNextLoc` or copying/re-registering the original fixed pose.
    pub(crate) fn set_fixed_sqt(
        &mut self,
        owner: &Arc<Mutex<FaceCommands>>,
        id: u32,
    ) -> Result<(), BindPoseError> {
        let retained = owner.lock().map_err(|_| ActorFaceError::Lock)?;
        let actual = retained
            .buffered
            .as_ref()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        if usize::try_from(id).map_or(true, |id| id >= retained.max_frames) || id > 0x0fff {
            return Err(BindPoseError::FixedSqtId {
                id,
                maximum: retained.max_frames,
            });
        }
        if !Arc::ptr_eq(actual, &self.owner) {
            return Err(BindPoseError::UnsupportedSetup);
        }
        if self
            .fixed_owner
            .as_ref()
            .is_some_and(|previous| !Arc::ptr_eq(previous, owner))
        {
            return Err(BindPoseError::UnsupportedSetup);
        }
        self.fixed_owner = Some(Arc::clone(owner));
        let id = u16::try_from(id).map_err(|_| BindPoseError::UnsupportedSetup)?;
        self.owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .commands
            .push(Command::SetSqtById {
                id,
                sqt: id | FIXED_SQT_BIT,
            });
        self.produced_fixed(id | FIXED_SQT_BIT);
        self.stack.push(id | FIXED_SQT_BIT);
        Ok(())
    }
    /// Blend82a25d18..4c: first dynamic, else second dynamic, else new dynamic.
    pub(super) fn binary_output(&mut self, first: u16, second: u16) -> Result<u16, BindPoseError> {
        if first & FIXED_SQT_BIT == 0 {
            return Ok(first);
        }
        if second & FIXED_SQT_BIT == 0 {
            return Ok(second);
        }
        self.allocate_dynamic()
    }
    pub(super) fn allocate_dynamic(&mut self) -> Result<u16, BindPoseError> {
        self.allocate_reusing()
    }
    pub(crate) fn dynamic_count(&self) -> usize {
        self.reuse.high_water
    }
    /// Dynamic values are borrowed; fixed values come from the same retained
    /// constructor frame owner at job execution, never from a fabricated fetch.
    pub(crate) fn operand<'a>(
        &self,
        id: u16,
        dynamic: &'a [Option<Vec<AndaleSqt>>],
    ) -> Result<Cow<'a, [AndaleSqt]>, BindPoseError> {
        if id & FIXED_SQT_BIT == 0 {
            return dynamic
                .get(usize::from(id))
                .and_then(Option::as_deref)
                .map(Cow::Borrowed)
                .ok_or(BindPoseError::MissingBindPose);
        }
        let owner = self
            .fixed_owner
            .as_ref()
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        owner
            .fixed_frames
            .get(usize::from(id & 0x0fff))
            .cloned()
            .map(Cow::Owned)
            .ok_or(BindPoseError::MissingBindPose)
    }
}

#[cfg(test)]
#[path = "actor_animation_fixed_sqt_commands_tests.rs"]
mod tests;
