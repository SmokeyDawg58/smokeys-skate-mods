//! Retained Locomotion::Trajectory producer helpers; unapplied continuation.
use super::{AnimationLocomotionTrajectory, AnimationStepInput};
use crate::skeleton_root_transforms::SkeletonRootVector as V;

#[derive(Debug, PartialEq)]
pub(super) enum TrajectoryProducerError<E> {
    Foot(i32),
    State(i32),
    Time,
    TimeRatio,
    FootLock,
    Vector(&'static str),
    Tuning(E),
}

pub(super) trait FootPredictionTuning {
    type Error;
    fn start_y_offset(&mut self) -> Result<f32, Self::Error>;
    fn jump_height(&mut self) -> Result<f32, Self::Error>;
    fn minimum_apex_height(&mut self) -> Result<f32, Self::Error>;
}

fn foot<E>(value: i32) -> Result<usize, TrajectoryProducerError<E>> {
    usize::try_from(value)
        .ok()
        .filter(|v| *v < 4)
        .ok_or(TrajectoryProducerError::Foot(value))
}
fn state<E>(value: i32) -> Result<usize, TrajectoryProducerError<E>> {
    usize::try_from(value)
        .ok()
        .filter(|v| *v < 3)
        .ok_or(TrajectoryProducerError::State(value))
}
type Error = TrajectoryProducerError<std::convert::Infallible>;

#[derive(Debug)]
pub(super) enum IntentDataError<D> {
    Trajectory(Error),
    Debug(D),
}

impl AnimationLocomotionTrajectory {
    /// UpdateIntentData82c98878. The callback is the real DebugDraw continuation,
    /// after every retained mutation, not a substitute trajectory computation.
    pub(super) fn try_update_intent_data<D>(
        &mut self,
        debug_draw: impl FnOnce(&Self, usize) -> Result<(), D>,
    ) -> Result<(), IntentDataError<D>> {
        let locked = foot(self.foot_locked).map_err(IntentDataError::Trajectory)?;
        let half_gravity = V::known([0.0, f32::from_bits(0xc09c_f5c3), 0.0, 0.0]);
        for offset in 0..2 {
            let current = (locked + offset) % 4;
            let next = (current + 1) % 4;
            let air = self.input_feet[current].states[2];
            let first =
                air.vectors[1].zip(air.vectors[0], |speed, base| speed.mul_add(air.time, base));
            let predicted = half_gravity
                .map(|g| g * air.time)
                .zip(first, |g, first| g.mul_add(air.time, first));
            let delta = predicted.zip(
                self.input_feet[next].states[0].vectors[0],
                |predicted, base| predicted - base,
            );
            self.input_feet[next].data.position = delta
                .zip(self.input_feet[current].data.position, |delta, current| {
                    delta + current
                });
            self.input_feet[next].initialized = true;
        }
        let next = (locked + 1) % 4;
        let after_next = (next + 1) % 4;
        self.input_feet[next].stride = self.input_feet[next]
            .data
            .position
            .zip(self.input_feet[locked].data.position, |next, current| {
                next - current
            });
        self.input_feet[after_next].stride = self.input_feet[after_next]
            .data
            .position
            .zip(self.input_feet[next].data.position, |next, current| {
                next - current
            });
        if self.first_update {
            self.input_feet[locked].stride = V::known([0.0, 0.0, 1.0, 0.0]);
            let physical = foot(self.physical_foot_index).map_err(IntentDataError::Trajectory)?;
            self.physical_feet[physical].stride = V::known([0.0, 0.0, 1.0, 0.0]);
            let previous = (physical + 3) % 4;
            // FootControl::operator=82c943e8 does not copy trailing padding.
            let trailing = self.physical_feet[previous].trailing_action_probe_word;
            self.physical_feet[previous] = self.input_feet[(locked + 3) % 4];
            self.physical_feet[previous].trailing_action_probe_word = trailing;
            self.physical_feet[previous].initialized = false;
        }
        debug_draw(self, locked).map_err(IntentDataError::Debug)
    }

    /// SetState82c8a828: clear old actions before changing the locked foot.
    pub(super) fn try_set_state(&mut self, new_foot: i32, new_state: i32) -> Result<bool, Error> {
        if new_foot == self.foot_locked && new_state == self.current_state {
            return Ok(false);
        }
        if self.current_state != -1 {
            let old_state = state(self.current_state)?;
            self.input_feet[foot(self.foot_locked)?].states[old_state].action = -1;
            self.physical_feet[foot(self.physical_foot_index)?].states[old_state].action = -1;
        }
        if new_foot != self.foot_locked {
            self.input_feet[foot(self.foot_locked)?].initialized = false;
            self.physical_foot_index = (self.physical_foot_index + 1) % 4;
            self.foot_locked = new_foot;
        }
        self.current_state = new_state;
        Ok(true)
    }

    /// SetAction82c8a9a0: matching starting state opens a gate that stays open.
    pub(super) fn try_set_action(
        &mut self,
        action: i32,
        starting_state: i32,
        after_states: u32,
        mut for_states: u32,
    ) -> Result<(), Error> {
        let mut current = self.current_state;
        let mut gate = false;
        let mut foot_offset = 0;
        let mut visited = 0;
        while for_states != 0 && foot_offset < 4 {
            if current == starting_state && visited >= after_states {
                gate = true;
            }
            if gate {
                for_states -= 1;
                let s = state(current)?;
                self.input_feet[foot((self.foot_locked + foot_offset) % 4)?].states[s].action =
                    action;
                self.physical_feet[foot((self.physical_foot_index + foot_offset) % 4)?].states[s]
                    .action = action;
            }
            current += 1;
            if current == 3 {
                current = 0;
                foot_offset += 1;
            }
            visited += 1;
        }
        Ok(())
    }

    /// SetSpeedCOMEnd82c8a708. XYZ equality assertion deliberately ignores W.
    pub(super) fn try_set_speed_com_end(
        &mut self,
        vector: V,
        selected_state: i32,
        selected_foot: i32,
    ) -> Result<(), Error> {
        if vector.xyz.iter().any(|v| v.is_nan()) {
            return Err(Error::Vector("speed COM end"));
        }
        self.input_feet[foot(selected_foot)?].states[state(selected_state)?].vectors[3] = vector;
        Ok(())
    }

    /// AddTime82c8a788; dt must be ordered positive, retained time not negative.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub(super) fn try_add_time(&mut self, dt: f32) -> Result<(), Error> {
        if !(dt > 0.0) || self.time < 0.0 {
            return Err(Error::Time);
        }
        self.time += dt;
        Ok(())
    }

    /// CalculateTimeToNextFoot82c8a8e8, source valid state range 0..3.
    pub(super) fn try_time_to_next_foot(&self) -> Result<f32, Error> {
        let current = state(self.current_state)?;
        let states = &self.physical_feet[foot(self.physical_foot_index)?].states;
        let mut time = states[current].time - self.time;
        for next in &states[current + 1..] {
            time = next.time + time;
        }
        Ok(time)
    }

    /// GetCurrentTimeRatio82c8b8e8, native FSEL clamp followed by equality assertion.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub(super) fn try_current_time_ratio(&self) -> Result<f32, Error> {
        if self.first_update {
            return Ok(1.0);
        }
        let s = state(self.current_state)?;
        let physical = self.physical_feet[foot(self.physical_foot_index)?].states[s].time;
        if !(physical > 0.0) {
            return Err(Error::Time);
        }
        let ratio = self.input_feet[foot(self.foot_locked)?].states[s].time / physical;
        let lower = f32::from_bits(0x3a83_126f);
        let bounded = if lower - ratio >= 0.0 { lower } else { ratio };
        let bounded = if 1000.0 - bounded >= 0.0 {
            bounded
        } else {
            1000.0
        };
        if bounded != ratio {
            return Err(Error::TimeRatio);
        }
        Ok(bounded)
    }

    /// SetFootWorldPos82c92ab0. After first update the supplied position is unused.
    pub(super) fn try_set_foot_world_position(
        &mut self,
        selected_foot: i32,
        position: V,
    ) -> Result<(), Error> {
        let f = foot(selected_foot)?;
        if self.first_update {
            self.input_feet[f].data.position = position;
            self.input_feet[f].initialized = true;
            self.input_feet[f].data.normal = V::known([0.0, 1.0, 0.0, 0.0]);
            self.input_feet[f].data.surface = 0;
        } else {
            if selected_foot != self.foot_locked {
                return Err(Error::FootLock);
            }
            self.input_feet[f].data = self.physical_feet[foot(self.physical_foot_index)?].data;
        }
        Ok(())
    }

    /// GetFootPrediction82ca86c0: publication precedes live reads and final assertions.
    pub(super) fn try_foot_prediction<T: FootPredictionTuning>(
        &self,
        selected_foot: i32,
        input: &mut AnimationStepInput,
        tuning: &mut T,
    ) -> Result<(), TrajectoryProducerError<T::Error>> {
        use TrajectoryProducerError as E;
        if selected_foot < 0 {
            return Err(E::Foot(selected_foot));
        }
        let f = foot(selected_foot % 4)?;
        input.reference = self.input_feet[f].data.position;
        let offset = tuning.start_y_offset().map_err(E::Tuning)?;
        input.reference_offset = V::known([0.0, offset, 0.0, 0.0]);
        input.parameter_vector = self.input_feet[(f + 1) % 4]
            .data
            .position
            .zip(input.reference, |next, current| next - current)
            .zip(input.reference_offset, |difference, offset| {
                difference - offset
            });
        let state = state(self.current_state)?;
        let extra = if self.input_feet[f].states[state].action == 1 {
            (tuning.jump_height().map_err(E::Tuning)? - 0.5) - input.reference_offset.xyz[1]
        } else {
            input.parameter_vector.xyz[1] - input.reference_offset.xyz[1]
        };
        let minimum = tuning.minimum_apex_height().map_err(E::Tuning)?;
        input.parameter = if extra >= 0.0 {
            minimum + extra
        } else {
            minimum
        };
        input.speed_at_end = self.input_feet[f].states[2].vectors[3];
        input.flags = (input.flags & !2) | 4 | 8;
        for (name, vector) in [
            ("reference", &input.reference),
            ("reference offset", &input.reference_offset),
            ("parameter vector", &input.parameter_vector),
            ("speed at end", &input.speed_at_end),
        ] {
            if vector.xyz.iter().any(|v| v.is_nan()) {
                return Err(E::Vector(name));
            }
        }
        if input.parameter.is_nan() {
            return Err(E::Vector("parameter"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_prediction_uses_first_published_foot_and_preserves_copy_padding_on_debug_failure() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.input_feet[0].data.position = V {
            xyz: [10.0, 0.0, 0.0],
            w: None,
        };
        trajectory.input_feet[0].states[2].time = 1.0;
        trajectory.input_feet[0].states[2].vectors[1] = V::known([2.0, 0.0, 0.0, 0.0]);
        trajectory.input_feet[1].states[0].vectors[0] = V::known([3.0, 0.0, 0.0, 0.0]);
        trajectory.input_feet[1].states[2].time = 1.0;
        trajectory.input_feet[1].states[2].vectors[1] = V::known([4.0, 0.0, 0.0, 0.0]);
        trajectory.input_feet[2].states[0].vectors[0] = V::known([5.0, 0.0, 0.0, 0.0]);
        trajectory.input_feet[3].initialized = true;
        trajectory.input_feet[3].data.surface = 91;
        trajectory.input_feet[3].trailing_action_probe_word = Some(5);
        trajectory.physical_feet[3].trailing_action_probe_word = Some(7);
        let result = trajectory.try_update_intent_data(|current, index| {
            assert_eq!(index, 0);
            assert_eq!(current.input_feet[1].data.position.xyz[0], 9.0);
            assert_eq!(current.input_feet[2].data.position.xyz[0], 8.0);
            Err("debug failure")
        });
        assert!(matches!(
            result,
            Err(IntentDataError::Debug("debug failure"))
        ));
        let g = f32::from_bits(0xc09c_f5c3);
        assert_eq!(trajectory.input_feet[1].stride.xyz, [-1.0, g, 0.0]);
        assert_eq!(trajectory.input_feet[2].data.position.xyz[1], g + g);
        assert_eq!(trajectory.input_feet[2].data.position.w, None);
        assert_eq!(trajectory.physical_feet[3].data.surface, 91);
        assert!(!trajectory.physical_feet[3].initialized);
        assert_eq!(
            trajectory.physical_feet[3].trailing_action_probe_word,
            Some(7)
        );
    }
    #[test]
    fn state_change_clears_old_pair_then_rotates_physical_foot() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.foot_locked = 2;
        trajectory.physical_foot_index = 3;
        trajectory.current_state = 1;
        trajectory.input_feet[2].initialized = true;
        trajectory.input_feet[2].states[1].action = 8;
        trajectory.physical_feet[3].states[1].action = 9;
        assert!(!trajectory.try_set_state(2, 1).unwrap());
        assert!(trajectory.input_feet[2].initialized);
        assert!(trajectory.try_set_state(3, 2).unwrap());
        assert_eq!(trajectory.physical_foot_index, 0);
        assert_eq!(trajectory.input_feet[2].states[1].action, -1);
        assert_eq!(trajectory.physical_feet[3].states[1].action, -1);
        assert!(!trajectory.input_feet[2].initialized);
        assert_eq!(trajectory.foot_locked, 3);
        assert_eq!(trajectory.current_state, 2);
    }
    #[test]
    fn action_offset_counts_visited_states_before_start_gate_and_wraps_both_owners() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.current_state = 2;
        trajectory.foot_locked = 3;
        trajectory.physical_foot_index = 1;
        trajectory.try_set_action(7, 0, 2, 4).unwrap();
        // Air visit0, next Contact visit1 is too early, next Contact visit4 opens.
        for (input, physical, state) in [(1, 3, 0), (1, 3, 1), (1, 3, 2), (2, 0, 0)] {
            assert_eq!(trajectory.input_feet[input].states[state].action, 7);
            assert_eq!(trajectory.physical_feet[physical].states[state].action, 7);
        }
        assert_eq!(trajectory.input_feet[0].states[0].action, -1);
    }
    #[test]
    fn time_ratio_uses_distinct_input_and_physical_indices_and_rejects_unordered() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        assert_eq!(trajectory.try_current_time_ratio().unwrap(), 1.0);
        trajectory.first_update = false;
        trajectory.current_state = 1;
        trajectory.foot_locked = 3;
        trajectory.physical_foot_index = 2;
        trajectory.input_feet[3].states[1].time = 2.0;
        trajectory.physical_feet[2].states[1].time = 4.0;
        trajectory.physical_feet[2].states[2].time = 3.0;
        trajectory.time = 1.0;
        assert_eq!(trajectory.try_current_time_ratio().unwrap(), 0.5);
        assert_eq!(trajectory.try_time_to_next_foot().unwrap(), 6.0);
        assert_eq!(trajectory.try_add_time(f32::NAN), Err(Error::Time));
        assert_eq!(trajectory.time, 1.0);
        trajectory.input_feet[3].states[1].time = f32::NAN;
        assert_eq!(trajectory.try_current_time_ratio(), Err(Error::TimeRatio));
        trajectory.time = f32::NAN;
        assert_eq!(trajectory.try_add_time(0.25), Ok(()));
        assert!(trajectory.time.is_nan());
    }
}
