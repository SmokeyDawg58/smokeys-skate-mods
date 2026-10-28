//! Update5's same-owner continuation after Board.Fill, before Skeleton Part19.
use super::board_fill_final::Update5AfterBoardFill;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, skeleton_fill_prefix::SkeletonFillError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update5SkeletonPrefixError<N> {
    MissingPool,
    Fill(SkeletonFillError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for Update5SkeletonPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Skeleton Fill requires actual skater pool"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}

pub(crate) struct Update5BeforeSkeletonPart19 {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) captured_com_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
}
impl PhysicalAssemblyWorld {
    /// Board call82e04ef8 precedes Skeleton call82e04f18. The selected prefix
    /// stops at82b58614; it does not imply the complete Skeleton callback returned.
    pub(crate) fn try_continue_update5_skeleton_prefix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        after_board: Update5AfterBoardFill,
        numeric: &mut N,
    ) -> Result<Update5BeforeSkeletonPart19, Update5SkeletonPrefixError<N::Error>> {
        let captured_com_velocity = self
            .skaters
            .as_mut()
            .ok_or(Update5SkeletonPrefixError::MissingPool)?
            .try_fill_paired_skeleton_output_prefix(
                &self.backend,
                board,
                skeleton,
                &after_board.owner,
                numeric,
            )
            .map_err(Update5SkeletonPrefixError::Fill)?;
        Ok(Update5BeforeSkeletonPart19 {
            owner: after_board.owner,
            captured_com_velocity,
        })
    }
}
