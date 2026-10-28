//! PhysState_Skitching::Enter82d94678, called by the shared state transition.
use super::{GrabDescriptor, Skitching, StateConstructorError};

#[derive(Debug)]
pub(crate) enum SkitchEntryError<E> {
    Reset(StateConstructorError),
    DisableBoard(E),
    Lock,
    Missing(&'static str),
    GrabSplineNotReady,
}

impl Skitching {
    /// The outer SetPhysicsState owns Ground.Exit, controller changes and
    /// current-state publication. Do not repeat any of those in this callback.
    pub(crate) fn try_enter<E>(
        &mut self,
        disable_board_animation: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), SkitchEntryError<E>> {
        //82d94690 then82d94698, before testing the processed ready bit.
        self.reset().map_err(SkitchEntryError::Reset)?;
        disable_board_animation().map_err(SkitchEntryError::DisableBoard)?;
        let processed = self
            .base
            .processed
            .lock()
            .map_err(|_| SkitchEntryError::Lock)?;
        let flags = processed
            .physics_state_fields()
            .flags_8d0
            .ok_or(SkitchEntryError::Missing("Processed flags8d0"))?;
        // Native lbz2256 tests bit1 of the first big-endian byte, not bit1
        // of the host u32. Failure is an assert in the debug reference.
        require_ready(flags)?;
        let spline = processed
            .grab_spline(false)
            .ok_or(SkitchEntryError::Missing("Processed primary GrabSpline"))?;
        self.grab_spline = Some(spline.clone());
        //82d9471c preserves every other bit (decompiler incorrectly uses0).
        self.force_exit = Some(false);
        self.grab_spline_desc = Some(GrabDescriptor {
            object_type: spline.descriptor_type,
            object_id: spline.descriptor_object,
        });
        //82d94740 LogEventCoordinate is the existing disabled telemetry path.
        Ok(())
    }
}

fn require_ready<E>(flags: u32) -> Result<(), SkitchEntryError<E>> {
    if flags & 0x0100_0000 != 0 {
        Ok(())
    } else {
        Err(SkitchEntryError::GrabSplineNotReady)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entry_requires_the_big_endian_primary_spline_ready_bit() {
        assert!(require_ready::<()>(0x0100_0000).is_ok());
        assert!(require_ready::<()>(0xffff_ffff).is_ok());
        assert!(matches!(
            require_ready::<()>(1),
            Err(SkitchEntryError::GrabSplineNotReady)
        ));
        assert!(matches!(
            require_ready::<()>(0xfeff_ffff),
            Err(SkitchEntryError::GrabSplineNotReady)
        ));
    }
}
