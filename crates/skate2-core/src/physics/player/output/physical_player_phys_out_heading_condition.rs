//! Proposed construct child: direct const condition load, not a write accessor.
use super::{ConcretePhysOutError, PhysOutComponent, PhysicalPlayerPhysOut};
impl PhysicalPlayerPhysOut {
    /// `PhysicsSpeedAndSlopeCompare`82c997f0/f4 reads Motion+c0 directly.
    pub(crate) fn try_action_heading_adjust_factor(
        &self,
    ) -> Result<f32, ConcretePhysOutError<core::convert::Infallible>> {
        self.require_complete_component(PhysOutComponent::Motion)?;
        Ok(self
            .concrete
            .motion
            .as_deref()
            .expect("checked component")
            .heading_adjust_factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_player_phys_out::construct::PhysOutAllocation;
    #[test]
    fn direct_condition_read_keeps_current_bits_and_ignores_write_latch() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete::<core::convert::Infallible>(|_| Ok(PhysOutAllocation::Host))
            .unwrap();
        output.locked = Some(true);
        let bits = 0x7fc1_2345;
        output
            .concrete
            .motion
            .as_mut()
            .unwrap()
            .heading_adjust_factor = f32::from_bits(bits);
        assert_eq!(
            output.try_action_heading_adjust_factor().unwrap().to_bits(),
            bits
        );
    }
    #[test]
    fn absent_component_remains_an_error() {
        assert!(
            PhysicalPlayerPhysOut::new()
                .try_action_heading_adjust_factor()
                .is_err()
        );
    }
}
