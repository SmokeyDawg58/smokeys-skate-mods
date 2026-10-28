//! `ISkaterMotionGraph` `HasTiltToLargeForPreland825f48d0` / `IsTimeToPrepareLand825f4b60`.
use crate::physics::PointGraph;
#[cfg(test)]
#[path = "motion_prepare_land_query_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flag {
    TargetingWallRide,
    CanLandOnBoard,
    AirTrajectoryValid,
    OffboardTrajectoryReady,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    LandingNormalY,
    LastTiltX,
    VelocityY,
    OffboardTime,
    ComToDeck,
    CollisionTime,
    TrajectoryNormalY,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tuning {
    FeetNormalLimit,
    TiltXLimit,
    TiltVelocityLimit,
    FinalDistance,
    DistanceVelocity,
    LandingOnDeckDistance,
}
pub(crate) trait PrepareLandSource {
    type Error;
    /// Native resolves cached `IPhysical` against the UCOM list on each call.
    fn resolve_physical(&mut self) -> Result<bool, Self::Error>;
    fn flag(&mut self, field: Flag) -> Result<bool, Self::Error>;
    fn value(&mut self, field: Value) -> Result<f32, Self::Error>;
    fn tuning(&mut self, field: Tuning) -> Result<f32, Self::Error>;
    fn landing_graph(&mut self) -> Result<PointGraph<8>, Self::Error>;
    fn offboard_preland_time(&mut self) -> Result<f32, Self::Error>;
}
/// Virtual+1c of actual `ISkaterMotionGraph` vtable82069688. It is NOT the
/// `GetNextAnimTransOveride` method suggested by the decompiler's wrong this type.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub(crate) fn has_tilt_too_large<S: PrepareLandSource>(s: &mut S) -> Result<bool, S::Error> {
    if !s.resolve_physical()? {
        return Ok(false);
    }
    if s.flag(Flag::TargetingWallRide)? {
        return Ok(true);
    }
    let normal_y = s.value(Value::LandingNormalY)?;
    let normal_limit = s.tuning(Tuning::FeetNormalLimit)?;
    // VMX compare splats each operand; CR6 all-true bit tests ordered GT.
    if normal_y > normal_limit {
        return Ok(false);
    }
    let tilt = f32::from_bits(s.value(Value::LastTiltX)?.to_bits() & 0x7fff_ffff);
    let tilt_limit = s.tuning(Tuning::TiltXLimit)?;
    if !(tilt > tilt_limit) {
        return Ok(false);
    }
    let velocity_y = s.value(Value::VelocityY)?;
    Ok(velocity_y > s.tuning(Tuning::TiltVelocityLimit)?)
}
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub(crate) fn time_to_prepare_land<S: PrepareLandSource>(s: &mut S) -> Result<bool, S::Error> {
    if has_tilt_too_large(s)? {
        return Ok(false);
    }
    if !s.resolve_physical()? {
        return Ok(false);
    }
    if !(s.value(Value::VelocityY)? < 0.0) {
        return Ok(false);
    }
    let final_distance = s.tuning(Tuning::FinalDistance)?;
    let distance_velocity = s.tuning(Tuning::DistanceVelocity)?;
    if s.flag(Flag::CanLandOnBoard)? {
        let distance = s.tuning(Tuning::LandingOnDeckDistance)?;
        let threshold = (final_distance - distance) / distance_velocity;
        return Ok(s.value(Value::OffboardTime)? < threshold);
    }
    if s.flag(Flag::AirTrajectoryValid)? {
        let distance = s.value(Value::ComToDeck)?;
        let mut threshold = (final_distance - distance) / distance_velocity;
        let collision_time = s.value(Value::CollisionTime)?;
        // Deliberately resample this source flag rather than reusing the branch.
        if s.flag(Flag::AirTrajectoryValid)? {
            let normal_y = s.value(Value::TrajectoryNormalY)?;
            let lower = if -normal_y >= 0.0 { 0.0 } else { normal_y };
            let graph = s.landing_graph()?;
            let input = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
            #[expect(
                clippy::assign_op_pattern,
                reason = "retain source multiplication operand order"
            )]
            {
                threshold = graph.evaluate(input) * threshold;
            }
        }
        return Ok(collision_time < threshold);
    }
    if !s.flag(Flag::OffboardTrajectoryReady)? {
        return Ok(false);
    }
    let time = s.value(Value::OffboardTime)?;
    Ok(time < s.offboard_preland_time()?)
}
