//! Ready branch of `PhysState_Teleporting::FillPhysOut` 0x82d869f0.
use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_PHYSICS_GROUND};
use super::{PhysOutAccessError, PhysicalPlayerPhysOut};
use crate::skeleton_root_transforms::SkeletonRootMatrix;

impl PhysicalPlayerPhysOut {
    /// Source order is four separate write-latch samples, each followed by its
    /// store. The matrix is the same retained Teleporting state's four columns.
    pub(crate) fn try_publish_teleporting_ready(
        &mut self,
        teleport_to_on_board: bool,
        reset_transform: SkeletonRootMatrix,
    ) -> Result<(), PhysOutAccessError> {
        let exit_state = if teleport_to_on_board { PHYSICS_STATE_PHYSICS_GROUND } else { PHYSICS_STATE_OFFBOARD };

        let guard = self.sample_write_guard()?; // 82d86a18
        self.check_write_guard(&guard)?;
        self.concrete
            .state
            .as_mut()
            .ok_or(PhysOutAccessError::NotConstructed)?
            .is_teleport_finished = true; // 82d86a5c

        let guard = self.sample_write_guard()?; // 82d86a60
        self.check_write_guard(&guard)?;
        self.concrete
            .state
            .as_mut()
            .ok_or(PhysOutAccessError::NotConstructed)?
            .teleport_exit_state = exit_state; // 82d86a84

        let guard = self.sample_write_guard()?; // 82d86a88
        self.check_write_guard(&guard)?;
        self.concrete
            .motion
            .as_mut()
            .ok_or(PhysOutAccessError::NotConstructed)?
            .did_teleport = true; // 82d86aac

        let guard = self.sample_write_guard()?; // 82d86ab0
        self.check_write_guard(&guard)?;
        self.concrete
            .motion
            .as_mut()
            .ok_or(PhysOutAccessError::NotConstructed)?
            .teleport_transform = reset_transform; // 82d86ae4..b00, all 4 lanes per column
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_phys_out_teleporting_fill_tests.rs"]
mod tests;
