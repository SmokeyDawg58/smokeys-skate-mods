//! Unregistered world continuation: Skeleton.Fill returns before Pumping.Fill.
use super::update5_skeleton_parts::Update5BeforeSkeletonAngularVelocity;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, skeleton_fill_suffix::PairedSkeletonSuffixError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update5SkeletonSuffixError<N> {
    MissingPool,
    Fill(PairedSkeletonSuffixError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for Update5SkeletonSuffixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Skeleton suffix requires actual skater pool"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}
/// The actual Skeleton callback returned. No Pumping output or final lock yet.
pub(crate) struct Update5BeforePumping {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_skeleton_suffix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeSkeletonAngularVelocity,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<Update5BeforePumping, Update5SkeletonSuffixError<N::Error>> {
        let overlay = self.debug_overlay.clone();
        self.skaters
            .as_mut()
            .ok_or(Update5SkeletonSuffixError::MissingPool)?
            .try_fill_paired_skeleton_suffix(
                &self.backend,
                board,
                skeleton,
                &before.owner,
                before.captured_com_velocity,
                references,
                &overlay,
                numeric,
            )
            .map_err(Update5SkeletonSuffixError::Fill)?;
        Ok(Update5BeforePumping {
            owner: before.owner,
        })
    }
}
