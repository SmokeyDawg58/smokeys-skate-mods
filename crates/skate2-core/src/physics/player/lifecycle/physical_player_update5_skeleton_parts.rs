//! Continue the same Skeleton Fill invocation through the hips velocity store.
use super::update5_skeleton_prefix::Update5BeforeSkeletonPart19;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, skeleton_part_fill::PairedSkeletonPartFillError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update5SkeletonPartsError {
    MissingPool,
    Fill(PairedSkeletonPartFillError),
}
impl std::fmt::Display for Update5SkeletonPartsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Skeleton part Fill requires actual pool"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}
pub(crate) struct Update5BeforeSkeletonAngularVelocity {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) captured_com_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_skeleton_parts(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeSkeletonPart19,
    ) -> Result<Update5BeforeSkeletonAngularVelocity, Update5SkeletonPartsError> {
        self.skaters
            .as_mut()
            .ok_or(Update5SkeletonPartsError::MissingPool)?
            .try_fill_paired_skeleton_parts(&self.backend, board, skeleton, &before.owner)
            .map_err(Update5SkeletonPartsError::Fill)?;
        Ok(Update5BeforeSkeletonAngularVelocity {
            owner: before.owner,
            captured_com_velocity: before.captured_com_velocity,
        })
    }
}
