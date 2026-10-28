//! `BodyFlippingSignal` and time-intent state; actual Actor adapter is a child.
use crate::{
    actor_graph_construction::ActorGraphIntents,
    motion_graph_entity_authority::MotionGraphInstanceToken, physics::FilteredPhysicsState,
    play_animation::FastString36,
};
#[path = "actor_action_ground_runtime.rs"]
pub(crate) mod runtime;

pub(crate) trait GroundActionServices {
    type Error;
    fn action_intent_exists(&mut self, name: &FastString36) -> Result<bool, Self::Error>;
    fn require_motion(&mut self) -> Result<(), Self::Error>;
    fn filtered_state(&mut self) -> Result<FilteredPhysicsState, Self::Error>;
    fn delta_time(&mut self) -> Result<f32, Self::Error>;
    fn body_flip_extension(&mut self) -> Result<f32, Self::Error>;
    fn takeoff_extension(&mut self) -> Result<f32, Self::Error>;
    fn insert_motion(&mut self, name: &FastString36, value: f32) -> Result<(), Self::Error>;
    fn erase_motion(&mut self, name: &FastString36) -> Result<(), Self::Error>;
}
pub(crate) struct BodyFlippingSignalInstance {
    token: MotionGraphInstanceToken,
    body_flip_time: f32,
    last_flip_name: FastString36,
    time_in_air: f32,
    /// Native +24 ctor false; selected methods do not read or modify it.
    is_flipping: bool,
}
impl BodyFlippingSignalInstance {
    /// Data82caebd8, allocation2c/alignment16 in `CreateInstance82caec70`.
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            body_flip_time: 0.0,
            last_flip_name: ActorGraphIntents::native_name(b""),
            time_in_air: 0.0,
            is_flipping: false,
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
    /// Begin82c94840 stores first tuning before the second lookup assertion.
    pub(crate) fn begin<S: GroundActionServices>(&mut self, s: &mut S) -> Result<(), S::Error> {
        self.body_flip_time = s.body_flip_extension()?;
        self.time_in_air = s.takeoff_extension()?;
        Ok(())
    }
    fn body_window<S: GroundActionServices>(&mut self, s: &mut S) -> Result<bool, S::Error> {
        let front = ActorGraphIntents::native_name(b"FrontFlip");
        let back = ActorGraphIntents::native_name(b"BackFlip");
        if s.action_intent_exists(&front)? {
            self.body_flip_time = 0.0;
            self.last_flip_name = front;
        } else if s.action_intent_exists(&back)? {
            self.body_flip_time = 0.0;
            self.last_flip_name = back;
        } else {
            let limit = s.body_flip_extension()?;
            let time = s.delta_time()? + self.body_flip_time;
            self.body_flip_time = if time - limit >= 0.0 { limit } else { time }; //fsel230c
        }
        Ok(self.body_flip_time < s.body_flip_extension()?)
    }
    fn takeoff_window<S: GroundActionServices>(&mut self, s: &mut S) -> Result<bool, S::Error> {
        if s.filtered_state()? == FilteredPhysicsState::Air {
            let limit = s.takeoff_extension()?;
            let time = s.delta_time()? + self.time_in_air;
            self.time_in_air = if time - limit >= 0.0 { limit } else { time };
        } else {
            self.time_in_air = 0.0;
        }
        // Raw4099006c skips on NOT-GT, including unordered.
        if self.time_in_air.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Ok(false);
        }
        Ok(self.time_in_air < s.takeoff_extension()?)
    }
    /// Update82cd5cd8 always evaluates both windows before Motion acquisition.
    pub(crate) fn update<S: GroundActionServices>(&mut self, s: &mut S) -> Result<(), S::Error> {
        let body = self.body_window(s)?;
        let takeoff = self.takeoff_window(s)?;
        s.require_motion()?;
        if body && takeoff {
            s.insert_motion(&self.last_flip_name, 1.0)?;
        } else {
            s.erase_motion(&ActorGraphIntents::native_name(b"FrontFlip"))?;
            s.erase_motion(&ActorGraphIntents::native_name(b"BackFlip"))?;
        }
        Ok(())
    }
}
pub(crate) struct TimeIntentInstance {
    token: MotionGraphInstanceToken,
    time: f32,
}
impl TimeIntentInstance {
    /// Shared instance allocator82712148:0xc, alignment16, float+8=+0.
    pub(crate) fn new() -> Self {
        Self {
            token: MotionGraphInstanceToken::issue(),
            time: 0.0,
        }
    }
    pub(crate) fn token(&self) -> &MotionGraphInstanceToken {
        &self.token
    }
    /// Caller resolves Action then Motion before invoking this82cd5928 body.
    pub(crate) fn update<S: GroundActionServices>(
        &mut self,
        action: &FastString36,
        motion: &FastString36,
        s: &mut S,
    ) -> Result<(), S::Error> {
        s.require_motion()?;
        if s.action_intent_exists(action)? {
            let updated = s.delta_time()? + self.time;
            self.time = updated;
            s.insert_motion(motion, self.time)?;
        } else {
            self.time = 0.0;
            s.erase_motion(motion)?;
        }
        Ok(())
    }
    /// End82cc2028 erases only; instance timer is not reset.
    #[expect(
        clippy::unused_self,
        reason = "retained lifecycle method; source End does not dereference instance"
    )]
    pub(crate) fn end<S: GroundActionServices>(
        &self,
        motion: &FastString36,
        s: &mut S,
    ) -> Result<(), S::Error> {
        s.require_motion()?;
        s.erase_motion(motion)
    }
}
#[cfg(test)]
#[path = "actor_action_ground_callbacks_tests.rs"]
mod tests;
