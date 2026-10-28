//! Public Launch82dc4098..4118 on the retained trajectory owner.
use super::{OnboardTrajectoryOwner, RetainedTrajectoryLaunchInfo};

/// This is a source continuation, never a completed query or Launch result.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OnboardLaunchPrefix {
    AlreadyStartedReturn,
    BeforeAdjustStartingVelocity,
}
impl OnboardTrajectoryOwner {
    pub(crate) fn begin_launch(
        &mut self,
        launch: &RetainedTrajectoryLaunchInfo,
    ) -> OnboardLaunchPrefix {
        if self.just_started_query {
            return OnboardLaunchPrefix::AlreadyStartedReturn;
        }
        self.most_vertical_grind_normal_valid = false; // 40e4
        self.pass = 0; // 40ec, signed halfword
        self.best_trajectory = -1; // 40f8
        self.most_vertical_grind_normal = [0.0, 1.0, 0.0, 0.0]; // 4104
        self.launch = *launch; // 4108: native operator=, same retained launch record
        self.ready = false; // 410c
        self.grind_bonus_awarded = false; // 4110
        OnboardLaunchPrefix::BeforeAdjustStartingVelocity
    }
}
