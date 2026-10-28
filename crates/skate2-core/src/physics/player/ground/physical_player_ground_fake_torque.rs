//! Consecutive Ground control82d9a1d8..a42c, before doJump at a430.
//! Unregistered draft: no state/jump/collision branch is presumed inactive.
use super::ground_wallride::GroundBeforeFakeTorque;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        local_torque::LocalTorqueError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences, fake_torque::FakeTorqueCvars},
    physical_player_assembly::PhysicalAssemblyWorld,
    skateboard_force_runtime::fake_torque::{FakeTorqueError, FakeTorqueNumeric, try_fake_torque},
};
use std::sync::Arc;

pub(crate) struct GroundBeforeJump(pub(crate) GroundBeforeFakeTorque);
#[derive(Debug)]
pub(crate) enum GroundFakeTorqueError<N> {
    Missing,
    Foreign,
    Pair(SimulationSkeletonJumpPairError),
    Calculate(FakeTorqueError<InputCvarError, N>),
    Apply(LocalTorqueError),
}
impl<N: std::fmt::Display> std::fmt::Display for GroundFakeTorqueError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => f.write_str("Ground fake torque requires the retained skater pool"),
            Self::Foreign => f.write_str("Ground fake torque received another owner continuation"),
            Self::Pair(e) => e.fmt(f),
            Self::Calculate(e) => e.fmt(f),
            Self::Apply(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_fake_torque<N: FakeTorqueNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: GroundBeforeFakeTorque,
        references: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<GroundBeforeJump, GroundFakeTorqueError<N::Error>> {
        use GroundFakeTorqueError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing)?;
        let same = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                Arc::ptr_eq(owner, &before.owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::Foreign);
        }
        let torque =
            try_fake_torque(&mut FakeTorqueCvars(references), numeric).map_err(E::Calculate)?;
        if let Some(torque) = torque {
            pool.try_apply_board_local_torque(&mut self.backend, board, torque)
                .map_err(E::Apply)?;
        }
        Ok(GroundBeforeJump(before))
    }
}
