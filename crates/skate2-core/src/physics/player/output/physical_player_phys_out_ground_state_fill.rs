//! Actual PhysicsGround.Fill82d8ddf0..e0e4; actual `PhysOut` construct child.
use super::super::{PhysOutComponent, PhysicalPlayerPhysOut, SkeletonRootVector};
use super::prior_input::PriorInputReadError;
#[cfg(test)]
#[path = "physical_player_phys_out_ground_state_fill_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug)]
pub(crate) enum GroundFillFlag {
    PushAccelerating,
    Pinning,
    WallRideExit,
    AntiFlipNudge,
    WorldGrab,
    GrabObject,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum GroundFillScalar {
    AbsoluteSpeed,
    TimeToSkitch,
    SkitchHeight,
}
pub(crate) trait GroundStateFillSource {
    type Error;
    type Pointer;
    fn flag(&mut self, flag: GroundFillFlag) -> Result<bool, Self::Error>;
    fn scalar(&mut self, scalar: GroundFillScalar) -> Result<f32, Self::Error>;
    fn capture(&mut self, key: u64) -> Result<Self::Pointer, Self::Error>;
    fn load(&mut self, pointer: &Self::Pointer) -> Result<f32, Self::Error>;
    fn anti_flip_torque(&mut self) -> Result<SkeletonRootVector, Self::Error>;
    fn grab_type(&mut self) -> Result<i32, Self::Error>;
    fn grab_object(&mut self) -> Result<u32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum GroundStateFillError<E> {
    Output(PriorInputReadError),
    Source(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for GroundStateFillError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(error) => write!(f, "Ground.Fill output: {error:?}"),
            Self::Source(error) => write!(f, "Ground.Fill source: {error:?}"),
        }
    }
}
impl PhysicalPlayerPhysOut {
    fn ground_fill_guard(&self, component: PhysOutComponent) -> Result<(), PriorInputReadError> {
        self.check_current_guard()
            .map_err(PriorInputReadError::Access)?;
        self.require_complete_component(component)
            .map_err(PriorInputReadError::Component)
    }
    pub(crate) fn try_fill_ground_state<S: GroundStateFillSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), GroundStateFillError<S::Error>> {
        use GroundFillFlag as F;
        use GroundFillScalar as V;
        use GroundStateFillError as E;
        self.ground_fill_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        self.concrete.motion.as_mut().unwrap().is_push_accelerating =
            source.flag(F::PushAccelerating).map_err(E::Source)?;
        let first = source.capture(0xa9b5_32eb_bed1_6f38).map_err(E::Source)?;
        let second = source.capture(0x1e76_e849_db99_f955).map_err(E::Source)?;
        let second_value = source.load(&second).map_err(E::Source)?;
        let first_value = source.load(&first).map_err(E::Source)?;
        let limit = second_value + first_value;
        let pushable = source.scalar(V::AbsoluteSpeed).map_err(E::Source)? < limit;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().is_pinning =
            source.flag(F::Pinning).map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::Motion)
            .map_err(E::Output)?;
        self.concrete.motion.as_mut().unwrap().is_at_pushable_speed = pushable;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().wall_ride_exit =
            source.flag(F::WallRideExit).map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().anti_flip_torque =
            source.anti_flip_torque().map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete
            .ground
            .as_mut()
            .unwrap()
            .anti_flip_nudge_present = source.flag(F::AntiFlipNudge).map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().time_to_skitch =
            source.scalar(V::TimeToSkitch).map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::Ground)
            .map_err(E::Output)?;
        self.concrete.ground.as_mut().unwrap().skitch_spline_height =
            source.scalar(V::SkitchHeight).map_err(E::Source)?;
        //82d8e050 samples the lock before the byte read82d8e058, but the
        // assertion branch82d8e060 follows that read.
        let guard = self
            .sample_write_guard()
            .map_err(PriorInputReadError::Access)
            .map_err(E::Output)?;
        let world_grab = source.flag(F::WorldGrab).map_err(E::Source)?;
        self.check_write_guard(&guard)
            .map_err(PriorInputReadError::Access)
            .map_err(E::Output)?;
        self.require_complete_component(PhysOutComponent::Intents)
            .map_err(PriorInputReadError::Component)
            .map_err(E::Output)?;
        self.concrete
            .intents
            .as_mut()
            .unwrap()
            .has_world_grab_intent = world_grab;
        self.ground_fill_guard(PhysOutComponent::OffBoard)
            .map_err(E::Output)?;
        self.concrete.offboard.as_mut().unwrap().is_grabbing_object =
            source.flag(F::GrabObject).map_err(E::Source)?;
        self.ground_fill_guard(PhysOutComponent::State)
            .map_err(E::Output)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .grab_spline_desc
            .object_type = source.grab_type().map_err(E::Source)?;
        self.concrete
            .state
            .as_mut()
            .unwrap()
            .grab_spline_desc
            .object_id = i32::from_ne_bytes(source.grab_object().map_err(E::Source)?.to_ne_bytes());
        Ok(())
    }
}
