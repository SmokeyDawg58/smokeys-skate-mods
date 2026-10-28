//! KnownAir trajectory consumers, separate from PhysicsAir's COM advance helper.
use super::KnownAir;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    physical_player_trajectory_owners::{OnboardTrajectoryOwner, offboard_runtime::position_at},
    reckoning_numeric::{ReckoningNumeric, try_length, try_normalize_safe},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
};
const DT: f32 = f32::from_bits(0x3c88_8889);

#[derive(Debug)]
pub(crate) enum KnownAirTrajectoryError<N> {
    Missing(&'static str),
    Attribute(AttributeRuntimeError),
    Input(crate::reckoning::grounded_input::GroundInputError),
    Cvar(crate::input_cvar_runtime::InputCvarError),
    Numeric(N),
    InvalidCollisionTime(u32),
    ClosestPointIterationLimit,
}
impl<N: std::fmt::Debug> std::fmt::Display for KnownAirTrajectoryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KnownAir trajectory: {self:?}")
    }
}

impl OnboardTrajectoryOwner {
    /// GetClosestTrajectoryIndexToCOM82db9b30 -> _CalcClosestPoint82db99f0.
    /// The orientation argument is unused by the original wrapper. Stop on the
    /// first increasing distance; ties continue, and frame100 is an assertion.
    pub(crate) fn try_closest_com_index<N: ReckoningNumeric>(
        &self,
        com: V,
        n: &mut N,
    ) -> Result<(i32, V), KnownAirTrajectoryError<N::Error>> {
        let mut previous = 100_000_000.0;
        let mut offset = V::known([0.0; 4]);
        for index in 0..100 {
            let point = position_at(&self.winning_com, index as f32 * DT);
            let difference = com.zip(point, |a, b| a - b);
            let squared = n
                .dot_vector(difference, difference)
                .map_err(KnownAirTrajectoryError::Numeric)?;
            if squared > previous {
                return Ok((index - 1, offset));
            }
            offset = difference;
            previous = squared;
        }
        Err(KnownAirTrajectoryError::ClosestPointIterationLimit)
    }

    /// GetComPos82db9d40, fixed60Hz sample time independent of current dt.
    pub(crate) fn known_air_com_position(&self, index: i32) -> V {
        position_at(&self.winning_com, index as f32 * DT)
    }
}

impl KnownAir {
    /// InitTrajectoryInfo82d992a0. Caller supplies the freshly fetched global
    /// Reckoning flip coefficient (key978ce73b0efe76c8); own attributes remain live.
    pub(crate) fn try_initialize_trajectory<N: ReckoningNumeric>(
        &mut self,
        selector: &OnboardTrajectoryOwner,
        flip_coefficient: f32,
        n: &mut N,
    ) -> Result<(), KnownAirTrajectoryError<N::Error>> {
        use KnownAirTrajectoryError as E;
        if selector.best_trajectory < 0 || selector.winning_result.is_none() {
            return Err(E::Missing("selected winning trajectory"));
        }
        let normal = selector.winning_data.landing_normal;
        self.landing_normal = Some(normal);
        let result = &selector.winning_buffer;
        let trajectory = &result.trajectory;
        // _GetHeighestPosition: reject upward/nonvertical gravity rather than
        // inventing an apex for an unsupported trajectory.
        let gravity = trajectory.gravity.xyz;
        if gravity[0] != 0.0 || gravity[2] != 0.0 || !(gravity[1] <= 0.0) {
            return Err(E::Missing("vertical downward trajectory gravity"));
        }
        let vy = trajectory.velocity.xyz[1];
        let downward = gravity[1] * -1.0;
        let apex_time = if 0.0 > vy || 0.0 >= downward {
            0.0
        } else {
            let estimate = n.reciprocal_estimate(downward).map_err(E::Numeric)?;
            refine_drive_reciprocal(downward, estimate) * vy
        };
        self.time_to_apex = Some(apex_time);
        self.apex_pos = Some(position_at(trajectory, apex_time));
        let time = result.time[0];
        if time.abs() >= 100.0 {
            return Err(E::InvalidCollisionTime(time.to_bits()));
        }
        self.coll_pos = Some(position_at(trajectory, time));
        self.coll_time = Some(time);
        self.body_flip_speed_target = Some((flip_coefficient / time) * f32::from_bits(0xc0c9_0fdb));
        self.start_pos = Some(selector.start_board_position);
        // Native samples frame-1 and frame-2 and divides their difference by dt;
        // using the analytic velocity at collision time changes this value.
        let velocity = if result.time.iter().all(|v| *v >= 0.0) {
            let before = position_at(trajectory, (result.frame - 2) as f32 * DT);
            let after = position_at(trajectory, (result.frame - 1) as f32 * DT);
            let inverse =
                refine_drive_reciprocal(DT, n.reciprocal_estimate(DT).map_err(E::Numeric)?);
            after.zip(before, |a, b| (a - b) * inverse)
        } else {
            V::known([0.0; 4])
        };
        let speed = n.dot_vector(normal, velocity).map_err(E::Numeric)?;
        self.coll_speed = Some(speed);
        let tangent = velocity.zip(normal.map(|v| v * speed), |v, normal| v - normal);
        let minimum = self
            .attributes
            .as_ref()
            .ok_or(E::Missing("KnownAir attributes"))?
            .try_scalar_word(0x44ff_62c3_637c_2b43, 0)
            .map_err(E::Attribute)?
            .map_or(0.0, f32::from_bits);
        if minimum > try_length(tangent, n).map_err(E::Numeric)? {
            self.landing_heading = Some(V::known([0.0; 4]));
        } else {
            self.landing_heading =
                Some(try_normalize_safe(tangent, [0.0; 4], n).map_err(E::Numeric)?);
            self.landing_heading_valid = Some(true);
        }
        let heading = self.landing_heading.ok_or(E::Missing("landing heading"))?;
        if 0.1 > n.dot_vector(heading, heading).map_err(E::Numeric)? {
            self.landing_heading_valid = Some(false);
        }
        Ok(())
    }

    /// UpdateTrajectoryFollow82d8c078: increment first, then fade the entry
    /// error against pre-increment state time and follow the selected COM curve.
    pub(crate) fn try_follow_trajectory<N>(
        &mut self,
        selector: &OnboardTrajectoryOwner,
        correction_duration: f32,
    ) -> Result<(), KnownAirTrajectoryError<N>> {
        use KnownAirTrajectoryError::Missing;
        let index = self
            .traj_index
            .ok_or(Missing("trajectory index"))?
            .wrapping_add(1);
        self.traj_index = Some(index);
        let time = self.time_in_state.ok_or(Missing("time in state"))?;
        let ratio = (correction_duration - time) / correction_duration;
        // Scalar fsel pair: preserve zero/NaN selection, not f32::clamp.
        let nonnegative = if -ratio >= 0.0 { 0.0 } else { ratio };
        let blend = if 1.0 - nonnegative >= 0.0 {
            nonnegative
        } else {
            1.0
        };
        let error = self
            .trajectory_error
            .ok_or(Missing("trajectory error"))?
            .map(|v| v * blend);
        self.target_com_pos = Some(
            selector
                .known_air_com_position(index)
                .zip(error, |a, b| a + b),
        );
        Ok(())
    }
}
