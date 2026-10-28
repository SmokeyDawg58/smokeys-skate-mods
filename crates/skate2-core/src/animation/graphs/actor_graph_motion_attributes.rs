//! Actual `AddMgAttribute`825fe7a0 appends; duplicate names are not replaced.
use super::{ActorMgAttribute, ActorSkaterGraphState};
use crate::{generated_motion_load::runtime::MotionUpdateError, play_animation::FastString30};

impl ActorSkaterGraphState {
    pub(crate) fn try_append_motion_attribute(
        &mut self,
        name: FastString30,
        value: f32,
    ) -> Result<(), MotionUpdateError> {
        if self.attributes.len() == self.attribute_maximum() {
            return Err(MotionUpdateError::Capacity);
        }
        self.attributes.push(ActorMgAttribute { name, value });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_names_append_and_nan_is_not_rejected_by_motion_store() {
        let mut state = ActorSkaterGraphState::try_new::<std::convert::Infallible>().unwrap();
        let name = FastString30::encode("Trick").unwrap();
        state.try_append_motion_attribute(name, -0.0).unwrap();
        state
            .try_append_motion_attribute(name, f32::from_bits(0x7fc0_4321))
            .unwrap();
        assert_eq!(state.attributes().len(), 2);
        assert_eq!(state.attributes()[0].value.to_bits(), 0x8000_0000);
        assert_eq!(state.attributes()[1].value.to_bits(), 0x7fc0_4321);
    }
}
