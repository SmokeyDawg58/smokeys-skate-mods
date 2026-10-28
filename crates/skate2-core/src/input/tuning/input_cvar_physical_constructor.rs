//! Native constructor target loads: `SetTransform82b34ac0` and drives82b50a30..74.
use super::{InputCvarError, InputCvarReferences};
use crate::skeleton_drives::SkeletonDriveCvars;

impl InputCvarReferences {
    pub(crate) fn try_orthogonal_tolerance(&self) -> Result<f32, InputCvarError> {
        self.try_current_float(266)
    }
}

/// Each method resolves the current target; neither defaults nor captured values
/// replace the four distinct native displacement/strength samples.
pub(crate) struct ConstructorDriveCvars<'a>(pub(crate) &'a InputCvarReferences);
impl SkeletonDriveCvars for ConstructorDriveCvars<'_> {
    type Error = InputCvarError;
    fn hard_drive_displacement(&mut self) -> Result<f32, Self::Error> {
        self.0.try_current_float(267)
    }
    fn hard_drive_strength(&mut self) -> Result<f32, Self::Error> {
        self.0.try_current_float(268)
    }
}
