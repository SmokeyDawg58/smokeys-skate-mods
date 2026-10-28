//! Pedestrian SetJumpToPosition82e701f8, SpatialAdjust JumpNow82e7d5f0.
use super::{JumpTrajectory, SpatialAdjust};
use crate::reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate, try_length};
use crate::simulation_drive_jacobian::refine_drive_reciprocal;

pub(crate) struct JumpTuning {
    pub(crate) ledge_failure_distance: f32,
    pub(crate) ledge_clearance: f32,
    pub(crate) min_horizontal_ratio: f32,
    pub(crate) max_horizontal_ratio: f32,
    pub(crate) up_min_height: f32,
    pub(crate) up_max_height: f32,
    pub(crate) down_min_height: f32,
    pub(crate) down_max_height: f32,
    pub(crate) use_ledge: bool,
    pub(crate) use_landing: bool,
}

impl SpatialAdjust {
    /// Pedestrian::SetJumpToPosition82e701f8. Caller supplies actual current
    /// GetPosition and GetFacingDirection, not the cached NavRig position.
    pub(crate) fn set_jump_to_position(&mut self, position: [f32; 4], facing: [f32; 4],
        landing_position: [f32; 4], ledge_distance: f32,
    ) {
        // Native multiply then add (not fused); initializes every jump member.
        let ledge_position = std::array::from_fn(|i| position[i] + facing[i] * ledge_distance);
        self.jump.trajectory = Some(JumpTrajectory { initial_velocity: [0.; 4],
            downward_velocity: [0.; 4], landing_position, ledge_position,
            initial_height: 0., horizontal_speed: 0. });
        self.jump.elapsed_time = 0.;
        self.jump.up_time = 0.;
        self.jump.down_time = 0.;
        self.jump.is_jumping = false;
        self.jump.downward_motion = Some(false);
    }

    /// cLivingWorldSpatialAdjust::JumpNow82e7d5f0. Entity wrapper82e80f90
    /// supplies actual GetPosition and Chaser::GetMaxChaseSpeed. Preserves authored
    /// out-of-range ratios and the solver's zero/-1 degenerate results.
    pub(crate) fn jump_now<N: ReckoningNumeric>(&mut self, initial: [f32; 4],
        horizontal_speed: f32, tuning: &JumpTuning, numeric: &mut N,
    ) -> Result<(), String> where N::Error: std::fmt::Debug {
        let trajectory = self.jump.trajectory.as_mut()
            .ok_or("JumpNow before SetJumpToPosition")?;
        let landing_delta = [trajectory.landing_position[0] - initial[0], 0.,
            trajectory.landing_position[2] - initial[2], 0.];
        let ledge_delta = [trajectory.ledge_position[0] - initial[0], 0.,
            trajectory.ledge_position[2] - initial[2], 0.];
        let landing_squared = numeric.dot3(landing_delta, landing_delta)
            .map_err(|e| format!("pedestrian jump landing distance: {e:?}"))?;
        let projection = numeric.dot3(ledge_delta, landing_delta)
            .map_err(|e| format!("pedestrian jump ledge projection: {e:?}"))?;
        // Raw ble then blt: unordered projection takes the fallback too.
        if projection <= f32::from_bits(0x3400_0000) || !(projection < landing_squared) {
            trajectory.ledge_position = std::array::from_fn(|i|
                landing_delta[i].mul_add(tuning.ledge_failure_distance, initial[i]));
        }
        let landing_y = trajectory.landing_position[1];
        trajectory.ledge_position[1] =
            (if landing_y > initial[1] { landing_y } else { initial[1] }) + tuning.ledge_clearance;
        self.jump.elapsed_time = 0.;
        self.jump.is_jumping = true;
        self.jump.downward_motion = Some(false);
        trajectory.initial_height = initial[1];
        let height = landing_y - initial[1];
        let (min_height, max_height) = if height >= 0. {
            (tuning.up_min_height, tuning.up_max_height)
        } else { (tuning.down_min_height, tuning.down_max_height) };
        let ratio = (height.abs() - min_height) / (max_height - min_height);
        trajectory.horizontal_speed = ratio.mul_add(
            tuning.max_horizontal_ratio - tuning.min_horizontal_ratio,
            tuning.min_horizontal_ratio) * horizontal_speed;
        if tuning.use_ledge {
            solve_for_initial_velocity(initial, trajectory.ledge_position,
                trajectory.horizontal_speed, &mut trajectory.initial_velocity,
                &mut self.jump.up_time, numeric)?;
            if tuning.use_landing {
                solve_for_initial_velocity(trajectory.ledge_position, trajectory.landing_position,
                    trajectory.horizontal_speed, &mut trajectory.downward_velocity,
                    &mut self.jump.down_time, numeric)?;
            } else {
                trajectory.downward_velocity = trajectory.initial_velocity;
                let delta = [trajectory.landing_position[0] - trajectory.ledge_position[0], 0.,
                    trajectory.landing_position[2] - trajectory.ledge_position[2], 0.];
                let distance = try_length(delta, numeric)
                    .map_err(|e| format!("pedestrian jump down distance: {e:?}"))?;
                self.jump.down_time = (1.0 / trajectory.horizontal_speed) * distance;
            }
        } else {
            let mut travel_time = -1.;
            solve_for_initial_velocity(initial, trajectory.landing_position,
                trajectory.horizontal_speed, &mut trajectory.initial_velocity,
                &mut travel_time, numeric)?;
            trajectory.downward_velocity = trajectory.initial_velocity;
            let y_velocity = trajectory.initial_velocity[1];
            self.jump.up_time = if y_velocity >= 0. {
                f32::from_bits(0x3dd0_fac6) * y_velocity
            } else { f32::from_bits(0x3d08_8889) };
            let remaining = travel_time - self.jump.up_time;
            self.jump.down_time = if remaining >= 0. { remaining }
                else { f32::from_bits(0x3d08_8889) };
        }
        Ok(())
    }
}

/// geotest::SolveForInitialVelocity82e56468. Same shared numeric primitives,
/// with native vector reciprocal/refinement and scalar division kept distinct.
/// Writes zero velocity/time-1 before either early-out or any numeric failure.
pub(crate) fn solve_for_initial_velocity<N: ReckoningNumeric>(initial: [f32; 4], final_position: [f32; 4],
    horizontal_speed: f32, velocity: &mut [f32; 4], travel_time: &mut f32, numeric: &mut N,
) -> Result<(), String> where N::Error: std::fmt::Debug {
    *travel_time = -1.;
    *velocity = [0.; 4];
    if !(horizontal_speed > 0.) { return Ok(()); }
    let delta: [f32; 4] = std::array::from_fn(|i| final_position[i] - initial[i]);
    let horizontal = [delta[0], 0., delta[2], 0.];
    if !horizontal[..3].iter().any(|v| v.abs() > f32::from_bits(0x3400_0000)) { return Ok(()); }
    let squared = numeric.dot3(horizontal, horizontal)
        .map_err(|e| format!("pedestrian trajectory length: {e:?}"))?;
    let speed_seed = numeric.reciprocal_estimate(horizontal_speed)
        .map_err(|e| format!("pedestrian trajectory reciprocal speed: {e:?}"))?;
    let length_seed = numeric.reciprocal_sqrt_estimate(squared)
        .map_err(|e| format!("pedestrian trajectory length estimate: {e:?}"))?;
    let inverse_speed = refine_drive_reciprocal(horizontal_speed, speed_seed);
    let length = length_from_squared_and_estimate(squared, length_seed);
    let time = inverse_speed * length;
    let inverse_length_seed = numeric.reciprocal_estimate(length)
        .map_err(|e| format!("pedestrian trajectory reciprocal length: {e:?}"))?;
    *travel_time = time;
    let inverse_time = 1.0 / time;
    let inverse_length = refine_drive_reciprocal(length, inverse_length_seed);
    let mut result = horizontal.map(|v| (inverse_length * v) * horizontal_speed);
    result[1] = inverse_time.mul_add(delta[1], time * f32::from_bits(0x409c_cccd));
    *velocity = result;
    Ok(())
}
