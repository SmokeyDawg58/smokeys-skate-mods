//! CreateLocomotionTrajectory action/prediction suffix82cade10..82cae164.
use super::locomotion_create_mg::CreateMotionActions;
use super::locomotion_producer::{FootPredictionTuning, IntentDataError, TrajectoryProducerError};
use super::{ActorAnimationConstruction, AnimationLocomotionTrajectory};
use std::convert::Infallible;

pub(super) trait CreatePredictionTuning: FootPredictionTuning {
    fn failure_frame_count(&mut self) -> Result<i32, Self::Error>;
    fn minimum_spread_degrees(&mut self) -> Result<f32, Self::Error>;
    fn maximum_spread_degrees(&mut self) -> Result<f32, Self::Error>;
    fn maximum_step_height(&mut self, jump: bool) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(super) enum CreateFinishError<T, D> {
    Missing(&'static str),
    UnknownFlags(u32),
    Trajectory(TrajectoryProducerError<Infallible>),
    Intent(IntentDataError<D>),
    Prediction(TrajectoryProducerError<T>),
    Tuning(T),
    IntegerDivision,
    PhysicalAction,
}
pub(crate) struct StepLaunchContinuation {
    pub(crate) first: usize,
    pub(crate) count: usize,
    pub(crate) world_only: bool,
}
impl<K, W> ActorAnimationConstruction<K, W> {
    /// Called only after the source per-row ingestion, foot-world-position
    /// update and collision-trajectory correction. Returns the required Launch
    /// continuation; the Actor caller executes it with this owner's lock freed.
    pub(super) fn try_finish_create_locomotion<T: CreatePredictionTuning, D>(
        &mut self,
        actions: CreateMotionActions,
        previous_action: i32,
        tuning: &mut T,
        debug_draw: impl FnOnce(&AnimationLocomotionTrajectory, usize) -> Result<(), D>,
    ) -> Result<StepLaunchContinuation, CreateFinishError<T::Error, D>> {
        use CreateFinishError as E;
        let player = self.player.as_mut().ok_or(E::Missing("player"))?;
        let fields = self
            .before_sprint
            .as_mut()
            .ok_or(E::Missing("locomotion fields"))?;
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        let skater = self.skater.as_mut().ok_or(E::Missing("trajectory owner"))?;
        let trajectory = &mut skater.trajectory;
        if player.flags_known_mask & 0x1000 == 0 {
            return Err(E::UnknownFlags(0x1000));
        }
        if player.flags_bits & 0x1000 != 0 {
            trajectory
                .try_set_action(1, 0, 1, 3)
                .map_err(E::Trajectory)?;
            player.flags_bits &= !0x1000;
        }
        if player.flags_known_mask & 0x10000 == 0 {
            return Err(E::UnknownFlags(0x10000));
        }
        if player.flags_bits & 0x10000 != 0 {
            player.flags_bits |= 0x200;
            player.flags_known_mask |= 0x200;
        }
        if actions.consumed_20000 {
            trajectory
                .try_set_action(12, 0, 0, 6)
                .map_err(E::Trajectory)?;
        } else if actions.consumed_8000 {
            trajectory
                .try_set_action(13, 0, 0, 3)
                .map_err(E::Trajectory)?;
            trajectory
                .try_set_action(14, 0, 3, 3)
                .map_err(E::Trajectory)?;
        } else if actions.consumed_4000 {
            trajectory
                .try_set_action(15, 0, 0, 6)
                .map_err(E::Trajectory)?;
        } else if actions.consumed_2000 {
            trajectory
                .try_set_action(16, 0, 0, 6)
                .map_err(E::Trajectory)?;
        }
        if player.flags_known_mask & 0x80000 == 0 {
            return Err(E::UnknownFlags(0x80000));
        }
        if player.flags_bits & 0x80000 != 0 {
            fields.last_foot_down = fields.foot_down;
        }
        trajectory
            .try_update_intent_data(debug_draw)
            .map_err(E::Intent)?;
        skater.step_inputs[0].flags = 0;
        trajectory
            .try_foot_prediction(fields.foot_down, &mut skater.step_inputs[0], tuning)
            .map_err(E::Prediction)?;
        let turn = suffix.turn;
        let frames = tuning.failure_frame_count().map_err(E::Tuning)?;
        let minimum = tuning.minimum_spread_degrees().map_err(E::Tuning)?;
        let failures = fields.step_failure_counts[0];
        if frames == 0 {
            return Err(E::IntegerDivision);
        }
        let maximum = tuning.maximum_spread_degrees().map_err(E::Tuning)?;
        let remainder = failures.wrapping_rem(frames);
        let radians = f32::from_bits(0x3c8efa35);
        let minimum = minimum * radians;
        let maximum = maximum * radians;
        skater.step_inputs[0].turn_intent = turn;
        if frames == -1 && failures == i32::MIN {
            return Err(E::IntegerDivision);
        }
        skater.step_inputs[0].spread_angle =
            ((remainder as f32) * (maximum - minimum)) / (frames as f32) + minimum;
        if fields.second_flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::UnknownFlags(0x4000_0000));
        }
        let sprint = fields.second_flags_bits & 0x4000_0000 != 0
            && matches!(previous_action, 0 | 9 | 10 | 3 | 1);
        skater.step_inputs[0].flags =
            (skater.step_inputs[0].flags & !0x10) | (u8::from(sprint) << 4);
        let input = &skater.step_inputs[0];
        let offset = input.reference.zip(input.reference_offset, |a, b| a + b);
        suffix.queried_feet[0] = offset.zip(input.parameter_vector, |a, b| a + b);
        // Source re-reads the physical action after publishing the queried foot.
        let jump = trajectory
            .physics_action_probe()
            .map_err(|_| E::PhysicalAction)?
            == 1;
        if player.flags_known_mask & 1 == 0 {
            return Err(E::UnknownFlags(1));
        }
        let world_only = player.flags_bits & 1 != 0;
        let height = tuning.maximum_step_height(jump).map_err(E::Tuning)?;
        skater.step_inputs[0].max_y = height + skater.step_inputs[0].reference.xyz[1];
        Ok(StepLaunchContinuation {
            first: 0,
            count: 1,
            world_only,
        })
    }
}
