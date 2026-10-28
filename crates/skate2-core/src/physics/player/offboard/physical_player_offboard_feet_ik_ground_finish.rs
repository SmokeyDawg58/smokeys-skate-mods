//! Final two calls of `OffBoardFeetIK::UpdateGround` 0x82dd1e28..1e34, after
//! `Update` (ApplyIK): `CalculateGroundCorrectionAndWipeout` 0x82dcd328, then
//! `CalculateHeightAboveGround` 0x82dd0788 stored to +0x110.
use super::OffboardFeetIk;
use crate::{
    dynamic_assembly_session::skeleton_normal::SkeletonNormalAssembly,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        normalize_from_squared_and_estimate, try_normalize_safe,
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Debug)]
pub(crate) enum OffboardGroundFinishError {
    ProcessedPoisoned,
    MissingMirrorFlag,
    MissingProcessedFlags8d0,
    MissingEffectiveAnimation,
    Cvar(InputCvarError),
    Numeric(ReckoningHostNumericError),
}

impl std::fmt::Display for OffboardGroundFinishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoardFeetIK ground finish: {self:?}")
    }
}

/// `Skeleton::GetTrajectoryWorld` 0x82b3b488: AnimToWorld X/Y/Z rotate the
/// trajectory translation (Skeleton+2e30) with two fused accumulations.
fn trajectory_world(
    animation_to_world: &SkeletonRootMatrix,
    local: [f32; 3],
) -> SkeletonRootVector {
    let columns = animation_to_world.columns();
    let first = columns[0].map(|axis| axis * local[0]);
    let second = columns[1].zip(first, |axis, sum| axis.mul_add(local[1], sum));
    columns[2].zip(second, |axis, sum| axis.mul_add(local[2], sum))
}

fn flatten(mut vector: SkeletonRootVector) -> SkeletonRootVector {
    vector.xyz[1] = 0.0;
    vector
}

impl OffboardFeetIk {
    /// Native order: ground correction and wipeout counters first, then the
    /// height reads the same frame's AnimToWorld and surface results.
    pub(crate) fn try_finish_ground_update(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        trajectory_translation: [f32; 3],
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFinishError> {
        self.try_calculate_ground_correction_and_wipeout(skeleton, trajectory_translation, cvars)?;
        let height = self.try_calculate_ground_height(skeleton, cvars)?;
        self.height_above_ground = height;
        Ok(())
    }

    /// `CalculateGroundCorrectionAndWipeout` 0x82dcd328.
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered native function with retained counter stores"
    )]
    fn try_calculate_ground_correction_and_wipeout(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        trajectory_translation: [f32; 3],
        cvars: &InputCvarReferences,
    ) -> Result<(), OffboardGroundFinishError> {
        use OffboardGroundFinishError as E;
        let left_valid = self.result_left.valid;
        let right_valid = self.result_right.valid;
        let animation_left = self.current_animation_position_left;
        let animation_right = self.current_animation_position_right;
        let mut bad_left = false;
        let mut bad_right = false;
        // cd374..380: neither surface valid counts as bad ground directly and
        // skips the feet-not-down counter.
        let both_bad = if !left_valid && !right_valid {
            true
        } else {
            let left_delta = animation_left.xyz[1] - self.result_left.position.xyz[1];
            let right_delta = animation_right.xyz[1] - self.result_right.position.xyz[1];
            let y_normal = cvars.try_y_normal_value_to_wipeout().map_err(E::Cvar)?;
            // cd3e4..3f8 / cd40c..420: ordered `ble` keeps the foot down.
            let left_not_down = !left_valid
                || !(left_delta <= cvars.try_height_above_ground_foot_test().map_err(E::Cvar)?);
            let right_not_down = !right_valid
                || !(right_delta <= cvars.try_height_above_ground_foot_test().map_err(E::Cvar)?);
            self.foot_not_down_wipeout_counter = if left_not_down && right_not_down {
                self.foot_not_down_wipeout_counter.wrapping_add(1)
            } else {
                0
            };
            // cd460..4b4: ordered `bgt` height test, then the vector
            // compare kYNormalValueToWipeout > normal.y (NaN is not bad).
            bad_left = !left_valid
                || left_delta
                    > cvars
                        .try_height_above_ground_to_wipeout()
                        .map_err(E::Cvar)?
                || y_normal > self.result_left.normal.xyz[1];
            bad_right = !right_valid
                || right_delta
                    > cvars
                        .try_height_above_ground_to_wipeout()
                        .map_err(E::Cvar)?
                || y_normal > self.result_right.normal.xyz[1];
            bad_left && bad_right
        };
        self.wipeout_counter = if both_bad {
            self.wipeout_counter.wrapping_add(1)
        } else {
            0
        };
        // cd574/590 signed compares; the second cvar is read only if needed.
        if self.wipeout_counter
            > cvars
                .try_obik_frames_to_wipeout_for_bad_ground()
                .map_err(E::Cvar)?
            || self.foot_not_down_wipeout_counter
                > cvars
                    .try_obik_frames_to_wipeout_for_feet_not_down()
                    .map_err(E::Cvar)?
        {
            self.request_wipeout = true;
        }
        let (flags, z_axis) = {
            let processed = self
                .binding
                .processed
                .lock()
                .map_err(|_| E::ProcessedPoisoned)?;
            let flags = processed
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?;
            // Processed+e0 = mEffectiveAnimTransform (+c0) Z column.
            let z_axis = processed
                .ground_position_fields()
                .effective_animation
                .ok_or(E::MissingEffectiveAnimation)?
                .columns()[2];
            (flags, z_axis)
        };
        let zero = SkeletonRootVector::known([0.0; 4]);
        if !(bad_left || bad_right) || flags & 0x0020_0000 != 0 {
            // cd868..8bc: counter, validity, correction and direction clear.
            self.bad_ground_correction_counter = 0;
            self.trajectory_direction_valid = false;
            self.bad_ground_correction = zero;
            self.last_valid_trajectory_direction = [0.0; 4];
            return Ok(());
        }
        let mut numeric = HostReckoningNumeric;
        self.trajectory_direction_valid = false;
        let trajectory = trajectory_world(
            skeleton.root_transforms().animation_to_world(),
            trajectory_translation,
        );
        let squared = numeric
            .dot_vector(trajectory, trajectory)
            .map_err(E::Numeric)?;
        let minimum = cvars.try_obik_min_traj_value().map_err(E::Cvar)?;
        // cd60c: ordered vector compare; the two-refinement finish has no
        // zero select because the gate already excludes it.
        if squared > minimum * minimum {
            let estimate = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(E::Numeric)?;
            let direction = normalize_from_squared_and_estimate(trajectory, squared, estimate);
            self.last_valid_trajectory_direction = [
                direction.xyz[0],
                direction.xyz[1],
                direction.xyz[2],
                direction.w.unwrap_or(0.0),
            ];
            self.trajectory_direction_valid = true;
            self.bad_ground_correction_counter = 0;
        }
        // cd678..6b4: away from the single bad foot, else backwards along Z.
        let away = if bad_left == bad_right {
            z_axis.map(|lane| -lane)
        } else if bad_left {
            animation_right.zip(animation_left, |right, left| right - left)
        } else {
            animation_left.zip(animation_right, |left, right| left - right)
        };
        let direction =
            try_normalize_safe(flatten(away), zero, &mut numeric).map_err(E::Numeric)?;
        let opposed = self.trajectory_direction_valid
            && 0.0
                > numeric
                    .dot_vector(
                        direction,
                        SkeletonRootVector::known(self.last_valid_trajectory_direction),
                    )
                    .map_err(E::Numeric)?;
        let push = if opposed {
            direction
        } else {
            direction.zip(z_axis, |away, forward| away - forward)
        };
        let push = try_normalize_safe(flatten(push), zero, &mut numeric).map_err(E::Numeric)?;
        let speed = cvars.try_obik_speed_away_from_bad_foot().map_err(E::Cvar)?;
        self.bad_ground_correction_counter = self.bad_ground_correction_counter.wrapping_add(1);
        let frames = cvars
            .try_obik_frames_for_ground_correction()
            .map_err(E::Cvar)?;
        if self.bad_ground_correction_counter >= frames {
            self.bad_ground_correction = zero;
            self.bad_ground_correction_counter = frames;
            return Ok(());
        }
        let correction = push.map(|lane| lane * speed);
        self.bad_ground_correction = if opposed {
            // cd7fc..82c: cancel the flattened, unnormalized trajectory.
            correction.zip(flatten(trajectory).map(|lane| -lane), |c, t| c + t)
        } else {
            correction
        };
        Ok(())
    }

    /// `CalculateHeightAboveGround` 0x82dd0788: AnimToWorld Y minus the lower
    /// slope-adjusted valid foot surface; no valid foot returns zero.
    fn try_calculate_ground_height(
        &mut self,
        skeleton: &SkeletonNormalAssembly,
        cvars: &InputCvarReferences,
    ) -> Result<f32, OffboardGroundFinishError> {
        use OffboardGroundFinishError as E;
        let animation_y = skeleton.root_transforms().animation_to_world().columns()[3].xyz[1];
        self.animation_space_y = animation_y;
        let (z, x) = {
            let processed = self
                .binding
                .processed
                .lock()
                .map_err(|_| E::ProcessedPoisoned)?;
            let z = skeleton
                .try_effective_animation_to_world(&processed)
                .map_err(|_| E::MissingMirrorFlag)?
                .columns()[2];
            let x = skeleton
                .try_effective_animation_to_world(&processed)
                .map_err(|_| E::MissingMirrorFlag)?
                .columns()[0];
            (z, x)
        };
        let mut lowest = animation_y;
        if self.result_left.valid {
            let adjustment = slope_adjustment(self.result_left.normal, x, z, cvars)?;
            self.left_slope_adjust_y = adjustment;
            lowest = adjustment + self.result_left.position.xyz[1];
        }
        if !self.result_right.valid {
            return Ok(animation_y - lowest);
        }
        let adjustment = slope_adjustment(self.result_right.normal, x, z, cvars)?;
        self.right_slope_adjust_y = adjustment;
        let right = adjustment + self.result_right.position.xyz[1];
        // 0x82dd08f4 fsel: NaN difference keeps the left/animation value.
        let selected = if lowest - right >= 0.0 { right } else { lowest };
        Ok(animation_y - selected)
    }
}

/// 0x82dd081c..878: surface normal without its AnimToWorld-X component,
/// safely normalized, projected on Z and scaled by kSlopeEffectScalar.
fn slope_adjustment(
    normal: SkeletonRootVector,
    x: SkeletonRootVector,
    z: SkeletonRootVector,
    cvars: &InputCvarReferences,
) -> Result<f32, OffboardGroundFinishError> {
    use OffboardGroundFinishError as E;
    let mut numeric = HostReckoningNumeric;
    let dot = numeric.dot_vector(x, normal).map_err(E::Numeric)?;
    let projected = normal.zip(x.map(|lane| lane * dot), |n, p| n - p);
    let normalized =
        try_normalize_safe(projected, SkeletonRootVector::known([0.0; 4]), &mut numeric)
            .map_err(E::Numeric)?;
    let along = numeric.dot_vector(normalized, z).map_err(E::Numeric)?;
    Ok(cvars.try_slope_effect_scalar().map_err(E::Cvar)? * along)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory_world_rotates_translation_by_animation_axes() {
        let matrix = SkeletonRootMatrix::from_columns([
            SkeletonRootVector::known([0.0, 0.0, -1.0, 0.0]),
            SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            SkeletonRootVector::known([1.0, 0.0, 0.0, 0.0]),
            SkeletonRootVector::known([5.0, 6.0, 7.0, 1.0]),
        ]);
        let world = trajectory_world(&matrix, [2.0, 3.0, 4.0]);
        assert_eq!(world.xyz, [4.0, 3.0, -2.0]);
    }

    #[test]
    fn flatten_zeroes_only_y() {
        let flat = flatten(SkeletonRootVector::known([1.0, -2.0, 3.0, 4.0]));
        assert_eq!(flat.xyz, [1.0, 0.0, 3.0]);
        assert_eq!(flat.w, Some(4.0));
    }
}
