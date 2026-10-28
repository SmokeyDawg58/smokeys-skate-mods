//! RotationTracker82deb830/82df0c50/82df0cf8. Reuse the native numeric graphs.
use super::{collector_base::RotationTracker, scoring_attributes::ScoreError};
use crate::{
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, ReckoningVector, cross_full4, try_normalize_unsafe,
        try_signed_angle,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
impl RotationTracker {
    fn calculate_rotation(&self, current: &SkeletonRootMatrix) -> Result<f32, ScoreError> {
        let old = self
            .last_frame_matrix
            .as_ref()
            .ok_or(ScoreError::Missing("rotation tracker reset"))?;
        let (axis_index, reference_index) = match self.rotation_type {
            0 => (1, 2), // yaw
            1 => (2, 1), // roll
            2 => (0, 1), // pitch
            _ => return Err(ScoreError::Invalid("scoring rotation type")),
        };
        let axis = old.columns()[axis_index].operand();
        let old_reference = old.columns()[reference_index].operand();
        let new_reference = current.columns()[reference_index].operand();
        let mut numeric = HostReckoningNumeric;
        // Native two unchecked normalizations, not a host epsilon/fallback.
        let side = try_normalize_unsafe(cross_full4(axis, new_reference), &mut numeric)
            .map_err(ScoreError::Numeric)?;
        let projected = try_normalize_unsafe(cross_full4(side, axis), &mut numeric)
            .map_err(ScoreError::Numeric)?;
        let dot = numeric
            .dot3(projected, old_reference)
            .map_err(ScoreError::Numeric)?;
        if 0.0 > dot {
            return Ok(0.0);
        }
        try_signed_angle(projected, old_reference, axis, &mut numeric).map_err(ScoreError::Numeric)
    }
    pub(super) fn update(&mut self, current: SkeletonRootMatrix) -> Result<(), ScoreError> {
        let mut delta = self.calculate_rotation(&current)?;
        let pi = f32::from_bits(0x4049_0fdb);
        let tau = f32::from_bits(0x40c9_0fdb);
        if delta > pi {
            delta -= tau;
        } else if delta < -pi {
            delta += tau;
        }
        self.accumulated_angle_radians = delta + self.accumulated_angle_radians;
        self.last_frame_matrix = Some(current);
        Ok(())
    }
    pub(super) fn update_with_banking(
        &mut self,
        current: SkeletonRootMatrix,
        minimum_degrees: f32,
    ) -> Result<f32, ScoreError> {
        let before = self.accumulated_angle_radians;
        let degrees = (before * f32::from_bits(0x4265_2ee1)).abs();
        self.update(current)?;
        if (self.accumulated_angle_radians * f32::from_bits(0x4265_2ee1)).abs() >= degrees
            || degrees <= minimum_degrees
        {
            return Ok(0.0);
        }
        self.accumulated_angle_radians -= before;
        Ok(degrees)
    }
}
