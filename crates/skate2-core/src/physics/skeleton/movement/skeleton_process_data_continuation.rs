//! Live `ProcessData` continuation after trajectory inverse82b5efcc.
//! Processed fields are acquired only when their consuming branch executes.
use crate::physical_player_grind_air_adjust::update::{GrindAirAdjustOut, GrindAirProcessed};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnimationInputError {
    Missing(&'static str),
    UnknownFlags { offset: u32, mask: u32 },
    GrindAirAdjust(String),
}
impl std::fmt::Display for AnimationInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "ProcessData requires retained {field}"),
            Self::UnknownFlags { offset, mask } => {
                write!(f, "ProcessData flags {offset:x}/{mask:x} unknown")
            }
            Self::GrindAirAdjust(e) => f.write_str(e),
        }
    }
}

/// Getter results are operands of the particular source operation. XYZ reads
/// never require a retained vector's unused W lane to be known.
pub(crate) trait AnimationProcessedRead {
    fn flag(&self, offset: u32, mask: u32) -> Result<bool, AnimationInputError> {
        self.flags(offset, mask).map(|value| value != 0)
    }
    fn com_velocity_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        self.com_velocity_xyz().map(dot_operand)
    }
    fn up_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        self.up_xyz().map(dot_operand)
    }
    fn category(&self) -> Result<i32, AnimationInputError> {
        Err(AnimationInputError::Missing("category"))
    }
    fn state(&self) -> Result<i32, AnimationInputError> {
        Err(AnimationInputError::Missing("state"))
    }
    fn filtered_state(&self) -> Result<i32, AnimationInputError> {
        Err(AnimationInputError::Missing("filtered_state"))
    }
    fn flags(&self, _offset: u32, _mask: u32) -> Result<u32, AnimationInputError> {
        Err(AnimationInputError::Missing("flag"))
    }
    fn balance(&self) -> Result<f32, AnimationInputError> {
        Err(AnimationInputError::Missing("balance"))
    }
    fn com_velocity_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Err(AnimationInputError::Missing("com_velocity_xyz"))
    }
    fn up_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Err(AnimationInputError::Missing("up_xyz"))
    }
    fn deck_y(&self) -> Result<f32, AnimationInputError> {
        Err(AnimationInputError::Missing("deck_y"))
    }
    fn deck_matrix(&self) -> Result<crate::animation::AnimationMatrix, AnimationInputError> {
        Err(AnimationInputError::Missing("deck_matrix"))
    }
    fn landing_adjust_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Err(AnimationInputError::Missing("landing_adjust_xyz"))
    }
    fn time_to_land(&self) -> Result<f32, AnimationInputError> {
        Err(AnimationInputError::Missing("time_to_land"))
    }
    /// The processed fields the active `UpdateGrindAdjust` reads.
    fn grind_air_processed(&self) -> Result<GrindAirProcessed, AnimationInputError> {
        Err(AnimationInputError::Missing("grind air processed input"))
    }
}

/// Only the native dot3 arithmetic operand gets a fourth zero lane. This does
/// not publish or establish W in any retained source vector.
pub(crate) fn dot_operand(xyz: [f32; 3]) -> [f32; 4] {
    [xyz[0], xyz[1], xyz[2], 0.0]
}

#[cfg(test)]
impl AnimationProcessedRead for super::SkeletonAdjustmentInput {
    fn com_velocity_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        Ok(self.centre_of_mass_velocity)
    }
    fn up_operand(&self) -> Result<[f32; 4], AnimationInputError> {
        Ok(self.up)
    }
    fn category(&self) -> Result<i32, AnimationInputError> {
        Ok(self.category)
    }
    fn state(&self) -> Result<i32, AnimationInputError> {
        Ok(self.state)
    }
    fn filtered_state(&self) -> Result<i32, AnimationInputError> {
        Ok(self.filtered_state)
    }
    fn flags(&self, offset: u32, mask: u32) -> Result<u32, AnimationInputError> {
        let word = match offset {
            0x8c4 => self.flags_8c4,
            0x8c8 => self.flags_8c8,
            0x8cc => self.flags_8cc,
            0x8d0 => self.landing_on_board.flags_8d0,
            _ => return Err(AnimationInputError::UnknownFlags { offset, mask }),
        };
        Ok(word & mask)
    }
    fn balance(&self) -> Result<f32, AnimationInputError> {
        Ok(self.balance)
    }
    fn com_velocity_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Ok([
            self.centre_of_mass_velocity[0],
            self.centre_of_mass_velocity[1],
            self.centre_of_mass_velocity[2],
        ])
    }
    fn up_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        Ok([self.up[0], self.up[1], self.up[2]])
    }
    fn deck_y(&self) -> Result<f32, AnimationInputError> {
        Ok(self.landing_on_board.deck_transform[13])
    }
    fn deck_matrix(&self) -> Result<crate::animation::AnimationMatrix, AnimationInputError> {
        Ok(self.landing_on_board.deck_transform)
    }
    fn landing_adjust_xyz(&self) -> Result<[f32; 3], AnimationInputError> {
        self.landing_on_board.landing_adjust_xyz()
    }
    fn time_to_land(&self) -> Result<f32, AnimationInputError> {
        Ok(self.landing_on_board.time_to_land)
    }
}

pub(crate) trait GrindAdjustRead {
    /// `GrindAirAdjust::UpdateGrindAdjust` 0x82dd1518 on the retained
    /// collaborator; its `adjusting` byte (+282) takes the result's flag.
    fn update<I: AnimationProcessedRead>(
        &mut self,
        input: &I,
    ) -> Result<GrindAirAdjustOut, AnimationInputError>;
}
/// An active controlled collaborator reports a zero adjustment about J.
#[cfg(test)]
pub(super) struct ControlledGrind {
    pub(super) activated: bool,
    pub(super) adjusting: bool,
}
#[cfg(test)]
impl GrindAdjustRead for ControlledGrind {
    fn update<I: AnimationProcessedRead>(
        &mut self,
        _: &I,
    ) -> Result<GrindAirAdjustOut, AnimationInputError> {
        let out = GrindAirAdjustOut {
            grind_direction: self.activated.then_some([0.0, 1.0, 0.0, 0.0]),
            offset: self.activated.then_some([0.0; 4]),
            orientation_euler: self.activated.then_some([0.0; 4]),
            adjustment_required: self.activated,
        };
        self.adjusting = out.adjustment_required;
        Ok(out)
    }
}

pub(crate) trait AnimationDeckDeltaWrite {
    fn write_delta(&mut self, value: f32);
}
impl AnimationDeckDeltaWrite for Option<f32> {
    fn write_delta(&mut self, value: f32) {
        *self = Some(value);
    }
}
#[cfg(test)]
impl AnimationDeckDeltaWrite for f32 {
    fn write_delta(&mut self, value: f32) {
        *self = value;
    }
}
