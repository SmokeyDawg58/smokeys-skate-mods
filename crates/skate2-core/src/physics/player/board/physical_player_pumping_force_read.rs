//! Read-only child of the SAME Pumping owner; no second state/history allocation.
use super::{PhysicalPlayerPumpingOwner, PumpingError};
impl PhysicalPlayerPumpingOwner {
    pub(crate) fn try_force_absorption(&self) -> Result<f32, PumpingError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(PumpingError::NotConstructed)?
            .absorption)
    }
    pub(crate) fn try_force_acceleration(&self) -> Result<f32, PumpingError> {
        Ok(self
            .value
            .as_ref()
            .ok_or(PumpingError::NotConstructed)?
            .acceleration)
    }
}

#[cfg(test)]
mod tests {
    use super::{PhysicalPlayerPumpingOwner, PumpingError};
    #[test]
    fn reads_existing_constructed_fields_without_reset_or_parallel_history() {
        let mut owner = PhysicalPlayerPumpingOwner::new();
        assert_eq!(
            owner.try_force_absorption(),
            Err(PumpingError::NotConstructed)
        );
        owner.try_construct(true).unwrap();
        assert_eq!(owner.try_force_absorption().unwrap().to_bits(), 0);
        assert_eq!(owner.try_force_acceleration().unwrap().to_bits(), 0);
        let value = owner.value.as_mut().unwrap();
        value.acceleration = 3.0;
        value.absorption = -0.0;
        assert_eq!(
            owner.try_force_acceleration().unwrap().to_bits(),
            3.0_f32.to_bits()
        );
        assert_eq!(
            owner.try_force_absorption().unwrap().to_bits(),
            (-0.0_f32).to_bits()
        );
        owner.try_reset().unwrap();
        assert_eq!(owner.try_force_acceleration().unwrap().to_bits(), 0);
    }
}
