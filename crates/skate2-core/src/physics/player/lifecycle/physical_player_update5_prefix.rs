//! Actual Update5 entry82e04ea8 through hips-line output, before board Fill.
//! This entry neither publishes physical state nor runs output conditioners.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::update5_line_output::Update5LineOutputError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update5PrefixError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    PoisonedOutput,
    Output(Update5LineOutputError),
}
impl std::fmt::Display for Update5PrefixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Update5 requires the actual skater pool"),
            Self::Pair(e) => write!(f, "Update5 pair: {e:?}"),
            Self::PoisonedOutput => f.write_str("Update5 output lock poisoned"),
            Self::Output(e) => e.fmt(f),
        }
    }
}
/// A successful prefix; board Fill82b2f3e8 at call82e04ef8 remains pending.
pub(crate) struct Update5BeforeBoardFill {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_begin_player_update5(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<Update5BeforeBoardFill, Update5PrefixError> {
        use Update5PrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PoisonedOutput)?
            .try_begin_update5_output(owner.tail())
            .map_err(E::Output)?;
        Ok(Update5BeforeBoardFill { owner })
    }
}
