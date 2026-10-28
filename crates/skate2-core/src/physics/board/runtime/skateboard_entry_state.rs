//! Selected outer Skateboard constructor and `DisableBoardAnimation` stores.
use crate::simulation_drive_pool::SimulationDriveDynamics;
use std::sync::{Arc, RwLock};
#[path = "skateboard_entry_animation_enable.rs"]
pub(crate) mod animation_enable;
#[path = "skateboard_entry_collision_response.rs"]
mod collision_response;

pub(crate) trait SkateboardSpeedModelSource {
    type Error;
    fn effective_transform(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error>;
    fn deck_velocity(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ResetSpeedModelError<S, N> {
    Source(S),
    Numeric(N),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DisableBoardAnimationError<E> {
    Body(E),
    LinearPoisoned,
    AngularPoisoned,
}

pub(crate) struct SkateboardEntryState {
    animated: bool,
    wants_respawn: bool,
    abort_push: Option<bool>,
    reset_target_after_collision: bool,
    target_speed: f32,
    manual_error_integral: f32,
    manual_last_target_angle: f32,
    manual_last_actual_angle: f32,
    manual_torque: f32,
    manual_time_since_reset: f32,
    /// Native +fc mManualSpinAngle; independent of +f4 mManualTorque.
    manual_spin_angle: f32,
}

impl SkateboardEntryState {
    /// Outer+101, sampled by `FillPhysOut82b30714`.
    pub(crate) fn wants_respawn(&self) -> bool {
        self.wants_respawn
    }
    /// `FillPhysOut82b3074c` clears this only after publishing the request.
    pub(crate) fn clear_wants_respawn(&mut self) {
        self.wants_respawn = false;
    }
    /// Push abort flag store at 82b27380 on the same entry.
    pub(crate) fn request_abort_push(&mut self) {
        self.abort_push = Some(true);
    }
    /// Update3 82dfe7ac clears Skateboard+530/40000000, independently of collisions.
    pub(crate) fn clear_abort_push(&mut self) {
        self.abort_push = Some(false);
    }
    pub(crate) fn abort_push(&self) -> Option<bool> {
        self.abort_push
    }

    /// 82b2a1f0/200 captures current effective transform before fresh velocity;
    /// dot3 at214, fabs220, and only then the actual targetSpeed+e4 store224.
    pub(crate) fn try_reset_speed_model<
        S: SkateboardSpeedModelSource,
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        source: &mut S,
        numeric: &mut N,
    ) -> Result<(), ResetSpeedModelError<S::Error, N::Error>> {
        use ResetSpeedModelError as E;
        let transform = source.effective_transform().map_err(E::Source)?;
        let velocity = source.deck_velocity().map_err(E::Source)?;
        let axis = transform.columns()[2];
        // W never participates in the native dot3. Scratch zeros do not publish
        // or establish a missing physical vector lane.
        let value = numeric
            .dot3(
                [velocity.xyz[0], velocity.xyz[1], velocity.xyz[2], 0.0],
                [axis.xyz[0], axis.xyz[1], axis.xyz[2], 0.0],
            )
            .map_err(E::Numeric)?;
        self.target_speed = value.abs();
        Ok(())
    }
    /// Selected outer ctor stores 0x82b38b10/14, before Body construction.
    pub(crate) fn new() -> Self {
        Self {
            animated: false,
            // Native outer constructor82b38b0c, not a PostPhysics default.
            wants_respawn: false,
            // Actual late constructor clears at82b38c18 and82b38b58.
            abort_push: Some(false),
            reset_target_after_collision: false,
            target_speed: 0.0,
            // Source outer ctor 82b38b18/20/28/30/38, after targetSpeed.
            manual_error_integral: 0.0,
            manual_last_target_angle: 0.0,
            manual_last_actual_angle: 0.0,
            manual_torque: 0.0,
            manual_time_since_reset: 0.0,
            manual_spin_angle: 0.0, //82b38b3c; ResetManual preserves this field.
        }
    }

    /// 82b23ef8..3f08, independent of targetSpeed and board animation.
    pub(crate) fn reset_manual(&mut self) {
        self.manual_error_integral = 0.0;
        self.manual_last_target_angle = 0.0;
        self.manual_last_actual_angle = 0.0;
        self.manual_torque = 0.0;
        self.manual_time_since_reset = 0.0;
    }

    /// `ScaleManualForce` 82b23ed0: load torque then integral, multiply separately,
    /// commit torque at3ee0 before integral at3ee4. Other histories are untouched.
    pub(crate) fn scale_manual_force(&mut self, scalar: f32) {
        let torque = self.manual_torque;
        let integral = self.manual_error_integral;
        let torque = scalar * torque;
        let integral = scalar * integral;
        self.manual_torque = torque;
        self.manual_error_integral = integral;
    }

    pub(crate) fn animated(&self) -> bool {
        self.animated
    }
    pub(crate) fn target_speed(&self) -> f32 {
        self.target_speed
    }
    pub(crate) fn reset_teleport_animated(&mut self) {
        self.animated = false;
    }
    pub(crate) fn reset_teleport_target_speed(&mut self) {
        self.target_speed = 0.0;
    }

    /// Repeated outer ctor store 0x82b38c14, after the actual deck binding.
    pub(crate) fn finish_body_constructor(&mut self) {
        self.animated = false;
    }

    /// 0x82b2a100. Body captures are distinct and precede their respective
    /// stores; no lock spans both groups. Host failures retain native prefixes.
    pub(crate) fn try_disable_board_animation<E>(
        &mut self,
        mut capture_body: impl FnMut() -> Result<Arc<RwLock<SimulationDriveDynamics>>, E>,
    ) -> Result<(), DisableBoardAnimationError<E>> {
        use DisableBoardAnimationError as Error;
        let body = capture_body().map_err(Error::Body)?;
        self.animated = false;
        {
            let mut dynamics = body.write().map_err(|_| Error::LinearPoisoned)?;
            dynamics.linear.raw_type = 2;
            dynamics.linear.spring = 0.0;
            dynamics.linear.damping = 0.0;
            dynamics.linear.strength = 0.0;
        }
        let body = capture_body().map_err(Error::Body)?;
        {
            let mut dynamics = body.write().map_err(|_| Error::AngularPoisoned)?;
            dynamics.angular.spring = 0.0;
            dynamics.angular.raw_type = 2;
            dynamics.angular.damping = 0.0;
            dynamics.angular.strength = 0.0;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "skateboard_entry_state_tests.rs"]
mod tests;

#[path = "../control/skateboard_speed_model_prefix.rs"]
pub(crate) mod speed_model_prefix;

#[path = "../control/skateboard_speed_model_update.rs"]
pub(crate) mod speed_model_update;

#[path = "../control/skateboard_heading_torque.rs"]
pub(crate) mod heading_torque;

#[path = "../control/skateboard_manual_effect.rs"]
pub(crate) mod manual_effect;

#[path = "skateboard_entry_postphysics.rs"]
pub(crate) mod postphysics;
