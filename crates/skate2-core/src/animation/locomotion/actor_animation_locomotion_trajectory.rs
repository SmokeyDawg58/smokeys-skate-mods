//! Retained `Trajectory::GetCurrentComPos82c933a8` and `Bezier482c93070`.
//! The numerical collaborator supplies only primitive estimate/reduction
//! operations; it does not replace trajectory evaluation.
use super::{AnimationLocomotionTrajectory, AnimationPhysicsDataError};
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{input_cvar_runtime::InputCvarReferences, reckoning_numeric::ReckoningNumeric};

#[derive(Debug)]
pub(crate) enum AnimationLocomotionError<E> {
    Physics(AnimationPhysicsDataError),
    Numeric(E),
    InvalidTime,
    InvalidVector(&'static str),
    FootDownRequired,
}
impl<E> From<AnimationPhysicsDataError> for AnimationLocomotionError<E> {
    fn from(error: AnimationPhysicsDataError) -> Self {
        Self::Physics(error)
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for AnimationLocomotionError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Physics(error) => write!(f, "{error}"),
            Self::Numeric(error) => write!(f, "animation locomotion numeric: {error:?}"),
            Self::InvalidTime => write!(f, "animation locomotion duration assertion"),
            Self::InvalidVector(name) => write!(f, "animation locomotion {name} has NaN"),
            Self::FootDownRequired => write!(f, "animation locomotion foot-down assertion"),
        }
    }
}

pub(super) const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);

pub(crate) fn bezier4(
    start: [f32; 4],
    start_speed: [f32; 4],
    mut end: [f32; 4],
    end_speed: [f32; 4],
    t: f32,
    influence: f32,
) -> [f32; 4] {
    let duration = end_speed[3] - start[3];
    end[3] = end_speed[3];
    let span = duration * influence;
    let mut first: [f32; 4] =
        std::array::from_fn(|lane| start_speed[lane].mul_add(span, start[lane]));
    let mut second: [f32; 4] = std::array::from_fn(|lane| end[lane] - end_speed[lane] * span);
    first[3] = start[3] + span;
    second[3] = end[3] - span;
    let remaining = 1.0 - t;
    let remaining_squared = remaining * remaining;
    let time_squared = t * t;
    std::array::from_fn(|lane| {
        // 93258/32c0/32e8 produce the first handle's separately rounded term.
        let first_term = ((first[lane] * remaining_squared) * t) * 3.0;
        let start_term = (start[lane] * remaining).mul_add(remaining_squared, first_term);
        let second_term = ((second[lane] * remaining) * time_squared).mul_add(3.0, start_term);
        (end[lane] * t).mul_add(time_squared, second_term)
    })
}

impl AnimationLocomotionTrajectory {
    /// `82c933a8`: clamps the same owner's time before evaluating the state,
    /// writes the destination before the final optional world-position add.
    pub(crate) fn try_current_com<N: ReckoningNumeric>(
        &mut self,
        destination: &mut WalkVector,
        world: bool,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AnimationLocomotionError<N::Error>> {
        use AnimationLocomotionError as E;
        let index = usize::try_from(self.physical_foot_index)
            .ok()
            .filter(|index| *index < self.physical_feet.len())
            .ok_or(AnimationPhysicsDataError::FootIndex(
                self.physical_foot_index,
            ))?;
        let state_index = usize::try_from(self.current_state)
            .ok()
            .filter(|index| *index < 3)
            .ok_or(AnimationPhysicsDataError::StateIndex(self.current_state))?;
        let state = self.physical_feet[index].states[state_index];
        // FSEL tests >=0 (unordered chooses its final operand). Avoid min/max,
        // which would change NaN and signed-zero selection.
        let nonnegative = if -self.time >= 0.0 { 0.0 } else { self.time };
        self.time = if state.time - nonnegative >= 0.0 {
            nonnegative
        } else {
            state.time
        };
        if state_index == 2 {
            // Raw4099 at93454 skips the assertion whenever GT is clear,
            // including unordered operands.
            if self.time > state.time {
                return Err(E::InvalidTime);
            }
            *destination = state.vectors[1].zip(state.vectors[0], |speed, base| {
                speed.mul_add(self.time, base)
            });
            // HalfG initializer82f810b8: [0,c09cf5c3,0,0]. Multiply by time
            // separately before the second fused operation, including W.
            let half_gravity = [0.0, f32::from_bits(0xc09c_f5c3), 0.0, 0.0];
            *destination = WalkVector::known(half_gravity).zip(*destination, |gravity, value| {
                (gravity * self.time).mul_add(self.time, value)
            });
        } else {
            if super::xyz_nan(&state.vectors[0].xyz) {
                return Err(E::InvalidVector("COM base"));
            }
            // Native replaces these W lanes before Bezier reads them. Only
            // these local operands become fully known; the base W is retained.
            let start_speed = [
                state.vectors[1].xyz[0],
                state.vectors[1].xyz[1],
                state.vectors[1].xyz[2],
                UPDATE_TIME,
            ];
            let end = [
                state.vectors[2].xyz[0],
                state.vectors[2].xyz[1],
                state.vectors[2].xyz[2],
                state.time - UPDATE_TIME,
            ];
            let end_speed = [
                state.vectors[3].xyz[0],
                state.vectors[3].xyz[1],
                state.vectors[3].xyz[2],
                state.time,
            ];
            let parameter = if state.time > 0.0 {
                self.time / state.time
            } else {
                self.time
            };
            let influence = cvars
                .try_bezier_influence_percentage()
                .map_err(AnimationPhysicsDataError::Cvar)?;
            let mut evaluated =
                bezier4([0.0; 4], start_speed, end, end_speed, parameter, influence);
            if evaluated.iter().any(|value| value.is_nan()) {
                return Err(E::InvalidVector("Bezier result"));
            }
            if evaluated[3] != 0.0 {
                let divisor = evaluated[3];
                let seed = numeric.reciprocal_estimate(divisor).map_err(E::Numeric)?;
                let residual = -seed.mul_add(divisor, -1.0);
                let refined = seed.mul_add(residual, seed);
                let residual = -refined.mul_add(divisor, -1.0);
                let inverse = refined.mul_add(residual, refined);
                for value in &mut evaluated {
                    *value = inverse * (*value * self.time);
                }
            }
            if evaluated.iter().any(|value| value.is_nan()) {
                return Err(E::InvalidVector("scaled Bezier result"));
            }
            evaluated[3] = 0.0;
            *destination =
                WalkVector::known(evaluated).zip(state.vectors[0], |value, base| value + base);
        }
        if world {
            let position = self.physical_feet[index].data.position;
            if state_index != 2 && super::xyz_nan(&position.xyz) {
                return Err(E::InvalidVector("world foot position"));
            }
            *destination = destination.zip(position, |value, position| value + position);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_cvar_runtime::InputCvarManager;
    use crate::reckoning_numeric::HostReckoningNumeric;

    #[test]
    fn air_clamps_retained_time_then_applies_two_fused_terms_and_world_offset() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.current_state = 2;
        trajectory.time = 8.0;
        trajectory.physical_feet[0].states[2].time = 2.0;
        trajectory.physical_feet[0].states[2].vectors[0] = WalkVector::known([1.0, 2.0, 3.0, 4.0]);
        trajectory.physical_feet[0].states[2].vectors[1] = WalkVector::known([3.0, 5.0, 7.0, 9.0]);
        trajectory.physical_feet[0].data.position = WalkVector::known([10.0, 20.0, 30.0, 40.0]);
        let cvars = InputCvarReferences::new_static_initialized();
        let mut output = WalkVector::known([0.0; 4]);
        trajectory
            .try_current_com(&mut output, true, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(trajectory.time.to_bits(), 2.0_f32.to_bits());
        let gravity = f32::from_bits(0xc09c_f5c3);
        let expected = [17.0, (gravity * 2.0).mul_add(2.0, 12.0) + 20.0, 47.0, 62.0];
        assert_eq!(
            output.known_lanes().unwrap().map(f32::to_bits),
            expected.map(f32::to_bits)
        );
        trajectory.time = -0.0;
        trajectory
            .try_current_com(&mut output, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(trajectory.time.to_bits(), 0);
        assert_eq!(
            output.known_lanes().unwrap().map(f32::to_bits),
            [1.0_f32, 2.0, 3.0, 4.0].map(f32::to_bits)
        );
    }

    #[test]
    fn bezier_uses_live_influence_and_retains_original_base_w() {
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.current_state = 0;
        trajectory.time = 0.4;
        let state = &mut trajectory.physical_feet[0].states[0];
        state.time = 1.0;
        state.vectors = [
            [1.0, 2.0, 3.0, 7.0],
            [4.0, 1.0, 2.0, 99.0],
            [10.0, 4.0, 8.0, 88.0],
            [2.0, 3.0, 5.0, 77.0],
        ]
        .map(WalkVector::known);
        let cvars = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&cvars).unwrap();
        let mut original = WalkVector::known([0.0; 4]);
        trajectory
            .try_current_com(&mut original, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        manager
            .try_deserialize_startup(Some(b"[Config]\nkBezierInfluencePercentage=0.1\n"))
            .unwrap();
        let mut changed = WalkVector::known([0.0; 4]);
        trajectory
            .try_current_com(&mut changed, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_ne!(
            original.known_lanes().unwrap().map(f32::to_bits),
            changed.known_lanes().unwrap().map(f32::to_bits)
        );
        assert_eq!(changed.w.unwrap().to_bits(), 7.0_f32.to_bits());
    }

    #[test]
    fn unknown_w_survives_air_and_bezier_base_but_overwritten_handle_w_is_known() {
        let cvars = InputCvarReferences::new_static_initialized();
        let mut manager = InputCvarManager::new();
        manager.try_link(&cvars).unwrap();
        let mut trajectory = AnimationLocomotionTrajectory::new();
        trajectory.current_state = 2;
        trajectory.time = 0.5;
        trajectory.physical_feet[0].states[2].time = 1.0;
        trajectory.physical_feet[0].states[2].vectors[1].w = None;
        let mut output = WalkVector::known([9.0; 4]);
        trajectory
            .try_current_com(&mut output, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(output.w, None);
        assert!(output.xyz.iter().all(|v| v.is_finite()));

        trajectory.current_state = 0;
        trajectory.physical_feet[0].states[0].time = 1.0;
        for vector in &mut trajectory.physical_feet[0].states[0].vectors[1..] {
            vector.w = None;
        }
        trajectory
            .try_current_com(&mut output, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(output.w, Some(0.0));
        trajectory.physical_feet[0].states[0].vectors[0].w = None;
        trajectory
            .try_current_com(&mut output, false, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(output.w, None);
        trajectory.physical_feet[0].states[0].vectors[0].w = Some(7.0);
        trajectory.physical_feet[0].data.position.w = None;
        trajectory
            .try_current_com(&mut output, true, &cvars, &mut HostReckoningNumeric)
            .unwrap();
        assert_eq!(output.w, None);
    }
}
