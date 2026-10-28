//! Late reads from the same processed owner in grounded Reckoning.
use super::ReckoningGroundInput;
use crate::skeleton_root_transforms::SkeletonRootVector;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundScalar {
    BodySpin,
    DeckSpeed,
    AbsoluteBodySpeed,
    Balance,
    TimeDelta,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundVector {
    ComToDeck,
    DeckY,
    DeckZ,
    EffectiveZ,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GroundInputError {
    Missing(&'static str),
}
impl std::fmt::Display for GroundInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "Reckoning processed field missing: {v}"),
        }
    }
}
/// The source retains the processed pointer; it does not snapshot every field at
/// entry. A missing late field leaves all earlier Reckoning stores intact.
pub(crate) trait GroundInputRead {
    fn scalar(&self, field: GroundScalar) -> Result<f32, GroundInputError>;
    fn vector(&self, field: GroundVector) -> Result<SkeletonRootVector, GroundInputError>;
    fn wheels(&self) -> Result<i32, GroundInputError>;
    fn flag8c4(&self, mask: u32) -> Result<bool, GroundInputError>;
    fn flag8c8(&self, mask: u32) -> Result<bool, GroundInputError>;
}
/// Explicit immutable controlled sample compatibility for existing leaf tests.
impl GroundInputRead for ReckoningGroundInput {
    fn scalar(&self, field: GroundScalar) -> Result<f32, GroundInputError> {
        Ok(match field {
            GroundScalar::BodySpin => self.body_spin,
            GroundScalar::DeckSpeed => self.deck_speed,
            GroundScalar::AbsoluteBodySpeed => self.absolute_body_speed,
            GroundScalar::Balance => self.balance,
            GroundScalar::TimeDelta => self.time_delta,
        })
    }
    fn vector(&self, field: GroundVector) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(SkeletonRootVector::known(match field {
            GroundVector::ComToDeck => self.com_to_deck,
            GroundVector::DeckY => self.deck_y,
            GroundVector::DeckZ => self.deck_z,
            GroundVector::EffectiveZ => self.effective_z,
        }))
    }
    fn wheels(&self) -> Result<i32, GroundInputError> {
        Ok(self.wheels_in_contact)
    }
    fn flag8c4(&self, mask: u32) -> Result<bool, GroundInputError> {
        Ok(self.flags_8c4 & mask != 0)
    }
    fn flag8c8(&self, mask: u32) -> Result<bool, GroundInputError> {
        Ok(self.flags_8c8 & mask != 0)
    }
}

/// Full vectors consumed by the wrapper's stores; ground.Y is independently read
/// before the later full ground-normal filter input.
pub(crate) trait GroundDirectionsRead {
    fn dynamic_up(&self) -> Result<SkeletonRootVector, GroundInputError>;
    fn heading(&self) -> Result<SkeletonRootVector, GroundInputError>;
    fn ground_y(&self) -> Result<f32, GroundInputError>;
    fn ground(&self) -> Result<SkeletonRootVector, GroundInputError>;
}
pub(super) struct CapturedGroundDirections {
    pub(super) ground: SkeletonRootVector,
    pub(super) dynamic: SkeletonRootVector,
    pub(super) heading: SkeletonRootVector,
}
impl GroundDirectionsRead for CapturedGroundDirections {
    fn dynamic_up(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(self.dynamic)
    }
    fn heading(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(self.heading)
    }
    fn ground_y(&self) -> Result<f32, GroundInputError> {
        Ok(self.ground.xyz[1])
    }
    fn ground(&self) -> Result<SkeletonRootVector, GroundInputError> {
        Ok(self.ground)
    }
}
