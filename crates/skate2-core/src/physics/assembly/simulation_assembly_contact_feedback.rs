//! The post-export prefix of `Island::Step_Solver3`: reset Static RF at
//! 0x827bae4c, then sample the CURRENT freezing byte at 0x827bae5c, conditionally
//! freeze at 0x827bae6c. The authored-ground caller then invokes the actual
//! physical contact-list producer at the source 0x827bae74 continuation.
//! The retained progress is host failure bookkeeping, not a native field.

use super::SimulationAssemblyConstraintSession;
use crate::{
    simulation_contact_jacobian::contact_spy::SimulationContactSpyError,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
};
use std::fmt;

#[cfg(test)]
#[path = "simulation_assembly_contact_feedback_tests.rs"]
mod tests;

/// Current world services, sampled at the actual consuming phase. No default
/// is supplied: native `kEnableFreezing` starts at 1, not the controlled zero
/// used by existing isolated ground corpus frames. The unsigned manager Params
/// threshold is independent of the pool's current replacement cooldown.
pub(crate) trait SimulationContactFeedbackInput {
    type Error;
    fn current_enable_freezing(&mut self) -> Result<u8, Self::Error>;
    fn current_frames_before_freezing(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationContactFeedbackProgress {
    NotStarted,
    Started,
    StaticForcesReset,
    Complete { frozen_bodies: usize },
}

#[derive(Debug)]
pub(crate) enum SimulationContactFeedbackError<E> {
    MissingSpyFrame,
    AlreadyAttempted,
    Spy(SimulationContactSpyError),
    Body(SimulationRigidBodyPoolError),
    Input(E),
}

impl<E: fmt::Debug> fmt::Display for SimulationContactFeedbackError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSpyFrame => write!(f, "contact feedback requires a completed spy frame"),
            Self::AlreadyAttempted => {
                write!(f, "contact feedback already attempted for this frame")
            }
            Self::Spy(error) => write!(f, "contact feedback spy: {error}"),
            Self::Body(error) => write!(f, "contact feedback body: {error}"),
            Self::Input(error) => write!(f, "contact feedback input: {error:?}"),
        }
    }
}

impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// Requires the actual current completed/exported contact batch. Host
    /// validation precedes this phase's writes. Failure after reset preserves
    /// that prefix and refuses replay; it does not undo completed integration.
    /// Freezing preserves body occurrences and therefore the solved-spy frame,
    /// but does not make pre-freeze publication state/labels fresh.
    pub(crate) fn try_prepare_contact_feedback<I: SimulationContactFeedbackInput>(
        &mut self,
        input: &mut I,
    ) -> Result<(), SimulationContactFeedbackError<I::Error>> {
        use SimulationContactFeedbackError as Error;
        use SimulationContactFeedbackProgress as Progress;
        self.contact_spies
            .as_ref()
            .ok_or(Error::MissingSpyFrame)?
            .try_validate_current(&self.contacts, &self.backend.rigid_bodies)
            .map_err(Error::Spy)?;
        if self.contact_feedback_progress != Progress::NotStarted {
            return Err(Error::AlreadyAttempted);
        }
        self.contact_feedback_progress = Progress::Started;
        self.backend
            .rigid_bodies
            .try_reset_static_reaction_forces()
            .map_err(Error::Body)?;
        self.contact_feedback_progress = Progress::StaticForcesReset;
        let frozen_bodies = if input.current_enable_freezing().map_err(Error::Input)? != 0 {
            let threshold = input
                .current_frames_before_freezing()
                .map_err(Error::Input)?;
            self.backend
                .rigid_bodies
                .try_freeze_if_necessary(threshold)
                .map_err(Error::Body)?
        } else {
            0
        };
        if frozen_bodies != 0 {
            // Host freshness policy: native leaves publications for its next
            // update. Same-frame direct host consumers must not accept cached
            // frozen/header/active data as current. Retained island labels and
            // actual bounds/volume values are not eagerly recomputed or cleared.
            if let Some(skaters) = &mut self.skaters {
                skaters.invalidate_refresh();
            }
        }
        self.contact_feedback_progress = Progress::Complete { frozen_bodies };
        Ok(())
    }
}

/// Explicit fixture policy: no claim that the source default disables freezing.
#[cfg(test)]
pub(crate) struct ControlledNoFreezing;

#[cfg(test)]
impl SimulationContactFeedbackInput for ControlledNoFreezing {
    type Error = std::convert::Infallible;
    fn current_enable_freezing(&mut self) -> Result<u8, Self::Error> {
        Ok(0)
    }
    fn current_frames_before_freezing(&mut self) -> Result<u32, Self::Error> {
        panic!("disabled freezing must not sample the threshold")
    }
}
