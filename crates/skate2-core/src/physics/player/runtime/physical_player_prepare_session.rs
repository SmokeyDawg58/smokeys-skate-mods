//! Selected same-pair `Update1` prepared history continuation. Caller scheduling
//! is explicit; this does not execute omitted `ProcessData`/Actor phases.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_prepare_velocity::PhysicalPlayerPrepareVelocityError,
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::prepare::{
        SkeletonProcessedPrepareError, SkeletonProcessedPrepareSource,
    },
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum PhysicalPlayerPrepareSessionError<N> {
    MissingSkaterPool,
    Pair(SimulationSkeletonJumpPairError),
    PoisonedReckoning,
    PoisonedProcessed,
    PoisonedHistory,
    Update(PhysicalPlayerPrepareVelocityError<SkeletonProcessedPrepareError, N>),
}
impl<N: fmt::Debug> fmt::Display for PhysicalPlayerPrepareSessionError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSkaterPool => f.write_str("prepared history skater pool unavailable"),
            Self::Pair(e) => write!(f, "prepared history pair: {e}"),
            Self::PoisonedReckoning => {
                f.write_str("prepared history processed occurrence access poisoned")
            }
            Self::PoisonedProcessed => f.write_str("prepared history processed state poisoned"),
            Self::PoisonedHistory => f.write_str("prepared history owner poisoned"),
            Self::Update(e) => write!(f, "prepared history continuation: {e}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_prepare_velocity<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerPrepareSessionError<N::Error>> {
        use PhysicalPlayerPrepareSessionError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingSkaterPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                let processed = Arc::clone(
                    owner
                        .reckoning()
                        .lock()
                        .map_err(|_| E::PoisonedReckoning)?
                        .processed(),
                );
                let mut processed = processed.lock().map_err(|_| E::PoisonedProcessed)?;
                let mut history = owner
                    .prepare_velocity()
                    .lock()
                    .map_err(|_| E::PoisonedHistory)?;
                history
                    .try_update(
                        &mut SkeletonProcessedPrepareSource::new(&mut processed),
                        numeric,
                    )
                    .map_err(E::Update)
            })
            .map_err(E::Pair)?
    }
}
