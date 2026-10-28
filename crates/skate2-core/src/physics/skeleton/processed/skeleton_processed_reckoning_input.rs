//! Actual retained processed reads for Ground.Update -> Reckoning.
use super::SkeletonProcessedContact;
use crate::{
    reckoning::grounded_input::{
        GroundDirectionsRead, GroundInputError, GroundInputRead, GroundScalar, GroundVector,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
fn need<T>(v: Option<T>, name: &'static str) -> Result<T, GroundInputError> {
    v.ok_or(GroundInputError::Missing(name))
}
impl GroundInputRead for SkeletonProcessedContact {
    fn scalar(&self, field: GroundScalar) -> Result<f32, GroundInputError> {
        match field {
            GroundScalar::BodySpin => need(
                self.anim_attributes.physics_body_spin,
                "physics body spin+9f8",
            ),
            GroundScalar::DeckSpeed => {
                need(self.prior_output.deck_speed(), "deck speed+968").map(|v| v.value)
            }
            GroundScalar::AbsoluteBodySpeed => {
                need(self.toolkit.abs_body_speed(), "absolute body speed+948")
            }
            GroundScalar::Balance => need(self.physics_state.balance, "balance+9a0"),
            GroundScalar::TimeDelta => need(self.physics_state.time_delta, "DT+93c"),
        }
    }
    fn vector(&self, field: GroundVector) -> Result<SkeletonRootVector, GroundInputError> {
        match field {
            GroundVector::ComToDeck => {
                need(self.process_data_output.com_to_deck, "COM to deck+2f0")
            }
            GroundVector::DeckY => {
                need(self.deck.as_ref(), "deck transform+40").map(|m| m.columns()[1])
            }
            GroundVector::DeckZ => {
                need(self.deck.as_ref(), "deck transform+40").map(|m| m.columns()[2])
            }
            GroundVector::EffectiveZ => {
                need(self.effective.as_ref(), "effective transform+80").map(|m| m.columns()[2])
            }
        }
    }
    fn wheels(&self) -> Result<i32, GroundInputError> {
        need(self.physics_state.num_wheels_in_contact, "wheels+90c")
    }
    fn flag8c4(&self, mask: u32) -> Result<bool, GroundInputError> {
        let (value, known) = self.known_flags_8c4();
        if known & mask != mask {
            return Err(GroundInputError::Missing("selected flag8c4"));
        }
        Ok(value & mask != 0)
    }
    fn flag8c8(&self, mask: u32) -> Result<bool, GroundInputError> {
        Ok(need(self.flags_8c8, "flag8c8")? & mask != 0)
    }
}
impl GroundDirectionsRead for SkeletonProcessedContact {
    fn dynamic_up(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(need(self.prior_output.dynamic_up(), "dynamic up+210")?.value)
    }
    fn heading(&self) -> Result<SkeletonRootVector, GroundInputError> {
        self.vector(GroundVector::DeckZ)
    }
    fn ground_y(&self) -> Result<f32, GroundInputError> {
        Ok(
            need(self.prior_output.ground_normal(), "ground normal+1d0")?
                .value
                .xyz[1],
        )
    }
    fn ground(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(need(self.prior_output.ground_normal(), "ground normal+1d0")?.value)
    }
}
impl SkeletonProcessedContact {
    pub(crate) fn ground_spawn_coordinate(&self, axis: usize) -> Result<f32, GroundInputError> {
        Ok(need(self.deck.as_ref(), "spawn deck translation")?.columns()[3].xyz[axis])
    }
}
