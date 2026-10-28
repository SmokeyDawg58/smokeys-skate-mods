//! ComputeAirParams82c93948, with each native physical-state store retained.
use super::*;
use crate::{
    physical_player_phys_out::construct::skeleton_fill_suffix::plane_angle,
    reckoning_numeric::wrap_turns_to_radians, reckoning_sin_cos::try_sin_cos,
};
const GRAVITY: V = V::known([0.0, f32::from_bits(0xc11cf5c3), 0.0, 0.0]);
const HALF_GRAVITY: V = V::known([0.0, f32::from_bits(0xc09cf5c3), 0.0, 0.0]);
const DT: f32 = f32::from_bits(0x3c888889);
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
fn sqrt<N: ReckoningNumeric, S>(
    value: f32,
    numeric: &mut N,
) -> Result<f32, LocomotionParamsError<N::Error, S>> {
    let seed = numeric
        .reciprocal_sqrt_estimate(value)
        .map_err(LocomotionParamsError::Numeric)?;
    Ok(super::sqrt_from_seed(value, seed))
}
impl AnimationLocomotionTrajectory {
    pub(super) fn compute_air<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
        &mut self,
        i: usize,
        current: usize,
        start: V,
        end: V,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        sin_cos: &mut S,
    ) -> Result<(), LocomotionParamsError<N::Error, S::Error>> {
        use LocomotionParamsError as E;
        let next = (current + 1) % 4;
        if !self.input_feet[current].initialized {
            return Err(E::Assertion("current input foot initialized"));
        }
        if !self.input_feet[next].initialized {
            return Err(E::Assertion("next input foot initialized"));
        }
        let input = self.input_feet[current];
        let next_input = self.input_feet[next];
        self.physical_feet[i].states[2].vectors[2] = add(next_input.states[0].vectors[0], end);
        self.physical_feet[i].states[2].vectors[0] = add(
            add(input.states[1].vectors[0], input.states[1].vectors[2]),
            start,
        );
        self.physical_feet[i].states[2].vectors[3] = next_input.states[0].vectors[1];
        let displacement = sub(
            self.physical_feet[i].states[2].vectors[2],
            self.physical_feet[i].states[2].vectors[0],
        );
        let intended = sub(
            add(next_input.states[0].vectors[0], next_input.data.position),
            add(input.states[2].vectors[0], input.data.position),
        );
        let average = half(add(input.states[2].vectors[1], input.states[1].vectors[3]));
        let original_y = average.xyz[1];
        let angle = plane_angle(
            displacement,
            intended,
            V::known([0.0, 1.0, 0.0, 0.0]),
            numeric,
        )
        .map_err(E::Numeric)?;
        let angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22f983));
        let angle = cvars.try_loco_correction_angle_sign().map_err(E::Cvar)? * angle;
        let (sine, cosine) = try_sin_cos(angle, sin_cos).map_err(E::SinCos)?;
        // Native shuffled matrix repeats X in W; W is not silently zeroed.
        let x = V::known([cosine, 0.0, -sine, cosine]);
        let y = V::known([0.0, 1.0, 0.0, 0.0]);
        let z = V::known([sine, 0.0, cosine, sine]);
        let flags = self.physical_feet[i].states[2].debug_flags & !7;
        self.physical_feet[i].states[2].debug_flags = flags;
        let rotated = y.zip(x.map(|v| v * average.xyz[0]), |v, first| {
            v.mul_add(average.xyz[1], first)
        });
        let rotated = z.zip(rotated, |v, first| v.mul_add(average.xyz[2], first));
        let mut duration;
        if cvars
            .try_loco_enable_jump_height_adjust()
            .map_err(E::Cvar)?
            && self.physical_feet[i].states[2].action == 1
        {
            let gravity = GRAVITY.xyz[1];
            let apex = cvars.try_loco_jump_height().map_err(E::Cvar)? + start.xyz[1];
            if apex.partial_cmp(&self.physical_feet[i].states[2].vectors[0].xyz[1])
                != Some(std::cmp::Ordering::Greater)
            {
                return Err(E::Assertion("jump apex above start"));
            }
            let square =
                ((apex - self.physical_feet[i].states[2].vectors[0].xyz[1]) * gravity) * -2.0;
            let inverse = 1.0 / gravity;
            let speed = sqrt(square, numeric)?;
            self.physical_feet[i].states[2].vectors[1].xyz[1] = speed;
            let ascent = -(inverse * speed);
            let delta = apex - self.physical_feet[i].states[2].vectors[2].xyz[1];
            let descent = if delta > 0.0 {
                sqrt((inverse * delta) * -2.0, numeric)?
            } else {
                0.0
            };
            duration = descent + ascent;
        } else {
            let old_time = input.states[2].time;
            let mut z_time = f32::MAX;
            let mut x_time = f32::MAX;
            if rotated.xyz[2].abs() > f32::from_bits(0x3a83126f)
                && rotated.xyz[2] * displacement.xyz[2] > 0.0
            {
                z_time = displacement.xyz[2] / rotated.xyz[2];
            }
            if rotated.xyz[0].abs() > f32::from_bits(0x3a83126f)
                && displacement.xyz[0] * rotated.xyz[0] > 0.0
            {
                x_time = displacement.xyz[0] / rotated.xyz[0];
            }
            if (old_time - z_time).abs() < (old_time - x_time).abs() {
                x_time = z_time;
            }
            duration = x_time;
            if duration == f32::MAX {
                duration = old_time;
                self.physical_feet[i].states[2].debug_flags = flags | 4;
            } else if duration / old_time < cvars.try_loco_time_ratio().map_err(E::Cvar)? {
                let delta = self.physical_feet[i].states[2].vectors[2].xyz[1]
                    - self.physical_feet[i].states[2].vectors[0].xyz[1];
                let gravity = GRAVITY.xyz[1];
                let product = delta * gravity;
                let (first, second) = if delta < 0.0 {
                    let inverse = 1.0 / gravity;
                    self.physical_feet[i].states[2].debug_flags = flags | 2;
                    let target = cvars
                        .try_capture_loco_second_degree_ratio()
                        .map_err(E::Cvar)?;
                    let lower = fsel(-f32::MAX - original_y, -f32::MAX, original_y);
                    let downward = fsel(-lower, lower, 0.0);
                    let square = downward * downward;
                    let ratio = target.try_value().map_err(E::Cvar)?;
                    let discriminant = product.mul_add(2.0, square);
                    let root = -sqrt(discriminant, numeric)?;
                    (
                        ((root - downward) * ratio) * inverse,
                        (((-downward) - root) * ratio) * inverse,
                    )
                } else {
                    let end_y = self.physical_feet[i].states[2].vectors[3].xyz[1];
                    self.physical_feet[i].states[2].debug_flags = flags | 1;
                    let inverse = 1.0 / gravity;
                    let positive = fsel(-end_y, 0.0, end_y);
                    let target = cvars
                        .try_capture_loco_second_degree_ratio()
                        .map_err(E::Cvar)?;
                    let bounded = fsel(f32::MAX - positive, positive, f32::MAX);
                    let discriminant = bounded.mul_add(bounded, -(product * 2.0));
                    let seed = numeric
                        .reciprocal_sqrt_estimate(discriminant)
                        .map_err(E::Numeric)?;
                    //3ebc loads captured target value AFTER the estimate.
                    let ratio = target.try_value().map_err(E::Cvar)?;
                    let root = super::sqrt_from_seed(discriminant, seed);
                    (
                        ((bounded - root) * inverse) * ratio,
                        (((-root) - bounded) * inverse) * ratio,
                    )
                };
                //3fe4 only ordered LT keeps first; equality/unordered choose second.
                duration = if (old_time - first)
                    .abs()
                    .partial_cmp(&(old_time - second).abs())
                    == Some(std::cmp::Ordering::Less)
                {
                    first
                } else {
                    second
                };
            }
        }
        duration = fsel(duration - DT, duration, DT);
        let seed = numeric.reciprocal_estimate(duration).map_err(E::Numeric)?;
        let mut inverse = seed;
        for _ in 0..2 {
            let residual = -inverse.mul_add(duration, -1.0);
            inverse = inverse.mul_add(residual, inverse);
        }
        let gravity_term = HALF_GRAVITY.map(|g| (g * duration) * duration);
        let velocity = sub(displacement, gravity_term).map(|v| inverse * v);
        self.physical_feet[i].states[2].vectors[1] = velocity;
        self.physical_feet[i].states[2].vectors[3] =
            GRAVITY.zip(velocity, |g, v| g.mul_add(duration, v));
        self.physical_feet[i].states[2].time = duration;
        self.physical_feet[i].states[2].vectors[2] = sub(
            self.physical_feet[i].states[2].vectors[2],
            self.physical_feet[i].states[2].vectors[0],
        );
        self.physical_feet[i].states[2].vectors[0] =
            sub(self.physical_feet[i].states[2].vectors[0], start);
        if velocity.xyz.iter().any(|v| v.is_nan()) {
            return Err(E::Assertion("physical air velocity NaN"));
        }
        self.physical_feet[i].initialized = true;
        Ok(())
    }
}
