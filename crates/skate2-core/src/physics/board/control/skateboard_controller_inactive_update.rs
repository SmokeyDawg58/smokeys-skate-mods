//! Controller.Update82dd1d68: actual inactive return before `UpdateState`.
use super::{SkateboardController, SkateboardControllerPhase};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ControllerUpdateError {
    NotConstructed(SkateboardControllerPhase),
    MissingSystemOn,
    PendingUpdateState,
    PendingPostGripStateGate(bool),
}
impl std::fmt::Display for ControllerUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConstructed(phase) => write!(f, "controller Update construction: {phase:?}"),
            Self::MissingSystemOn => f.write_str("controller Update missing actual system flag"),
            Self::PendingUpdateState => f.write_str("controller UpdateState82dd18e0 pending"),
            Self::PendingPostGripStateGate(body_flag) => write!(
                f,
                "controller UpdateState requires post-UpdateGrip Processed gates (prior Body+364 bit27={body_flag})"
            ),
        }
    }
}
impl<D> SkateboardController<D> {
    ///82dd1d7c/1d84: inactive returns without inspecting state, hand or drives.
    pub(crate) fn try_update_inactive(&mut self) -> Result<(), ControllerUpdateError> {
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(ControllerUpdateError::NotConstructed(self.phase));
        }
        if self
            .system_on
            .ok_or(ControllerUpdateError::MissingSystemOn)?
        {
            return Err(ControllerUpdateError::PendingUpdateState);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_reads_no_unrelated_fields_and_changes_nothing() {
        let mut controller = SkateboardController::<()>::new();
        // Deliberate field-isolation fixture: production constructor coverage
        // lives in the parent tests and actual pair corpus.
        controller.phase = SkateboardControllerPhase::Complete;
        controller.system_on = Some(false);
        controller.try_update_inactive().unwrap();
        assert_eq!(controller.system_on, Some(false));
        assert!(controller.state.is_none());
        assert!(controller.hand.is_none());
        assert!(controller.retrieval.is_none());
        assert!(controller.drives.iter().all(Option::is_none));
    }

    #[test]
    fn enabled_reaches_pending_update_state_without_reading_dispatch_state() {
        let mut controller = SkateboardController::<()>::new();
        controller.phase = SkateboardControllerPhase::Complete;
        controller.system_on = Some(true);
        assert_eq!(
            controller.try_update_inactive(),
            Err(ControllerUpdateError::PendingUpdateState)
        );
        assert_eq!(controller.system_on, Some(true));
        assert!(controller.state.is_none());
    }

    #[test]
    fn allocation_or_missing_flag_cannot_claim_native_inactive_return() {
        let mut controller = SkateboardController::<()>::new();
        assert_eq!(
            controller.try_update_inactive(),
            Err(ControllerUpdateError::NotConstructed(
                SkateboardControllerPhase::Fresh
            ))
        );
        controller.phase = SkateboardControllerPhase::Complete;
        assert_eq!(
            controller.try_update_inactive(),
            Err(ControllerUpdateError::MissingSystemOn)
        );
    }
}
