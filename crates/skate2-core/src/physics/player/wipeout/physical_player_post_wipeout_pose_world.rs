//! Same-owner `UpdatePoseBuffer` entry; enclosing post-wipeout scheduling is separate.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, pose_publication::PosePoolError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use std::sync::Arc;
impl PhysicalAssemblyWorld {
    pub(crate) fn try_publish_skeleton_pose<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
        sin_cos: &mut S,
    ) -> Result<(), PosePoolError<N::Error, S::Error>> {
        self.skaters
            .as_mut()
            .ok_or(PosePoolError::Missing("skater pool"))?
            .try_publish_skeleton_pose(&self.backend, board, skeleton, owner, numeric, sin_cos)
    }
}
