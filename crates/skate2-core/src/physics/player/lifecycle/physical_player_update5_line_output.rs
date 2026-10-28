//! Update5 Reset82e04ed0 and actual hips-line output callback82dfed78.
use super::super::{PhysOutAccessError, PhysOutComponent, PhysicalPlayerPhysOut};
use super::ConcretePhysOutError;
use crate::physical_player_assembly::hilod_tail::PhysicalHiLodTail;
use std::{convert::Infallible, sync::Mutex};

#[derive(Debug)]
pub(crate) enum Update5LineOutputError {
    Reset(ConcretePhysOutError<Infallible>),
    Access(PhysOutAccessError),
    Component(ConcretePhysOutError<Infallible>),
    PoisonedLineRecords,
    MissingLineRecords,
}
impl std::fmt::Display for Update5LineOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reset(e) => write!(f, "Update5 reset: {e:?}"),
            Self::Access(e) => write!(f, "Update5 line output guard: {e:?}"),
            Self::Component(e) => write!(f, "Update5 line output component: {e:?}"),
            Self::PoisonedLineRecords => f.write_str("Update5 retained line records lock poisoned"),
            Self::MissingLineRecords => {
                f.write_str("Update5 retained line records not constructed")
            }
        }
    }
}
impl PhysicalPlayerPhysOut {
    /// Resets this same output before consuming the actual owner's stored line.
    pub(crate) fn try_begin_update5_output(
        &mut self,
        tail: &Mutex<PhysicalHiLodTail>,
    ) -> Result<(), Update5LineOutputError> {
        self.try_reset_complete()
            .map_err(Update5LineOutputError::Reset)?;
        self.try_fill_update5_hips_line(tail)
    }
    fn guard_update5_ground(&self) -> Result<(), Update5LineOutputError> {
        self.check_current_guard()
            .map_err(Update5LineOutputError::Access)?;
        self.require_complete_component(PhysOutComponent::Ground)
            .map_err(Update5LineOutputError::Component)
    }
    /// Every output getter is independently guarded. Copies retain all W knowledge.
    fn try_fill_update5_hips_line(
        &mut self,
        tail: &Mutex<PhysicalHiLodTail>,
    ) -> Result<(), Update5LineOutputError> {
        use Update5LineOutputError as E;
        self.guard_update5_ground()?;
        let tail = tail.lock().map_err(|_| E::PoisonedLineRecords)?;
        let record = &tail.line_records.as_ref().ok_or(E::MissingLineRecords)?[0];
        self.concrete
            .ground
            .as_mut()
            .expect("guarded Ground")
            .below_hips_ground_data_is_valid = record.result.valid;
        self.guard_update5_ground()?;
        self.concrete
            .ground
            .as_mut()
            .expect("guarded Ground")
            .below_hips_ground_data_start_pos = record.start_position;
        if record.result.valid {
            self.guard_update5_ground()?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded Ground")
                .below_hips_ground_data_position = record.result.position;
            self.guard_update5_ground()?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded Ground")
                .below_hips_ground_data_normal = record.result.normal;
            //82dfee74 loads the source ID before the final output getter.
            let surface_id = record.result.surface_id;
            self.guard_update5_ground()?;
            self.concrete
                .ground
                .as_mut()
                .expect("guarded Ground")
                .below_hips_ground_data_surface_id = u32::from_ne_bytes(surface_id.to_ne_bytes());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_update5_line_output_tests.rs"]
mod tests;
