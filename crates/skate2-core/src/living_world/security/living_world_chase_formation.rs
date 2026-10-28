//! Chasee::GenerateIdealPositions82de2830: original ring candidates and greedy
//! assignment in radial-distance order. Host numeric boundaries are shared.
use super::{ChaseeState, TargetableId};
use crate::{
    reckoning_numeric::{ReckoningNumeric, try_length, try_signed_angle},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos},
    simulation_drive_jacobian::refine_drive_reciprocal,
};
#[derive(Debug)]
pub(crate) enum FormationError<E> {
    NoChasers,
    MissingChaser(TargetableId),
    Numeric(E),
}
impl<E: std::fmt::Display> std::fmt::Display for FormationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoChasers => f.write_str("formation requires chasers"),
            Self::MissingChaser(id) => write!(f, "missing formation chaser {id:?}"),
            Self::Numeric(e) => e.fmt(f),
        }
    }
}
fn vector(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}
fn signed_arc<N: ReckoningNumeric>(
    a: [f32; 3],
    forward: [f32; 3],
    radius: f32,
    n: &mut N,
) -> Result<f32, N::Error> {
    let angle = try_signed_angle(vector(a), vector(forward), [0.0, 1.0, 0.0, 0.0], n)?;
    let angle = if angle > std::f32::consts::PI {
        angle - std::f32::consts::TAU
    } else {
        angle
    };
    Ok(angle * radius)
}
impl ChaseeState {
    pub(crate) fn generate_ideal_positions<N: ReckoningNumeric>(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
        facing: [f32; 3],
        chaser_positions: &[(TargetableId, [f32; 3])],
        numeric: &mut N,
    ) -> Result<(), FormationError<N::Error>> {
        use FormationError as E;
        let count = self.chasers.len();
        if count == 0 {
            return Err(E::NoChasers);
        }
        let flat_velocity = [velocity[0], 0.0, velocity[2]];
        let speed = try_length(vector(flat_velocity), numeric).map_err(E::Numeric)?;
        let (heading, divisor, wanted_radius) = if speed < 1.0 {
            (facing, 1.0, self.formation_min_radius)
        } else {
            (flat_velocity, speed, speed * self.formation_velocity_scalar)
        };
        let inverse = refine_drive_reciprocal(
            divisor,
            numeric.reciprocal_estimate(divisor).map_err(E::Numeric)?,
        );
        let forward = heading.map(|v| v * inverse);
        let radius = wanted_radius
            .max(self.formation_min_radius)
            .min(self.formation_max_radius);
        if radius > self.block_radius {
            self.block_radius = radius;
        }
        let candidate_count = count.max(2 * count - 2);
        let divisor = candidate_count as f32;
        let inverse = refine_drive_reciprocal(
            divisor,
            numeric.reciprocal_estimate(divisor).map_err(E::Numeric)?,
        );
        let (sin, cos) = try_sin_cos(
            std::f32::consts::TAU * inverse,
            &mut HostReckoningSinCosNumeric,
        )
        .unwrap_or_else(|never| match never {});
        let mut directions = Vec::with_capacity(candidate_count);
        let mut offsets = Vec::with_capacity(candidate_count);
        let mut polar = Vec::with_capacity(candidate_count);
        let mut direction = forward;
        for index in 0..candidate_count {
            if index != 0 {
                direction = [
                    direction[0].mul_add(cos, direction[2] * sin),
                    direction[1],
                    (-direction[0]).mul_add(sin, direction[2] * cos),
                ];
            }
            let offset = direction.map(|v| v * radius);
            let arc = if index == 0 {
                0.0
            } else {
                signed_arc(offset, forward, radius, numeric).map_err(E::Numeric)?
            };
            directions.push(direction);
            offsets.push(offset);
            polar.push([arc, radius]);
        }
        let mut chaser_polar = Vec::with_capacity(count);
        for chaser in &self.chasers {
            let point = chaser_positions
                .iter()
                .find(|(id, _)| *id == chaser.id)
                .ok_or(E::MissingChaser(chaser.id))?
                .1;
            let delta = [point[0] - position[0], 0.0, point[2] - position[2]];
            chaser_polar.push([
                signed_arc(delta, forward, radius, numeric).map_err(E::Numeric)?,
                try_length(vector(delta), numeric).map_err(E::Numeric)?,
            ]);
        }
        // Native backward bubble passes retain equal-distance order.
        let mut order: Vec<usize> = (0..count).collect();
        for i in 0..count {
            for j in (i + 1..count).rev() {
                if chaser_polar[order[j - 1]][1] > chaser_polar[order[j]][1] {
                    order.swap(j - 1, j);
                }
            }
        }
        let mut available: Vec<usize> = (0..candidate_count).collect();
        let half_circumference = radius * std::f32::consts::PI;
        for chaser_index in order {
            let mut best = 0;
            let mut best_cost = f32::MAX;
            for (available_index, candidate) in available.iter().copied().enumerate() {
                let mut arc = polar[candidate][0] - chaser_polar[chaser_index][0];
                if arc > half_circumference {
                    arc += half_circumference * (-2.0);
                } else if arc < -half_circumference {
                    arc += half_circumference * 2.0;
                }
                let radial = polar[candidate][1] - chaser_polar[chaser_index][1];
                let cost = arc * arc + radial * radial;
                if cost < best_cost {
                    best_cost = cost;
                    best = available_index;
                }
            }
            let chosen = available.swap_remove(best);
            let chaser = &mut self.chasers[chaser_index];
            chaser.ideal_position = offsets[chosen];
            chaser.intercept_direction = Some(directions[chosen]);
        }
        Ok(())
    }
    /// EvaluateFlankOrders82de2730 uses protector aggression distance if present.
    pub(crate) fn evaluate_flank_orders(&mut self, protector_aggression_distance: Option<f32>) {
        self.block_radius = protector_aggression_distance.unwrap_or(self.formation_min_radius);
    }
    pub(crate) fn update_chasee_timers(&mut self, dt: f32) {
        if self.chasers.is_empty() {
            return;
        }
        for timer in [&mut self.scared_timer, &mut self.nearby_event_timer] {
            if *timer > 0.0 {
                *timer -= dt;
                if *timer < 0.0 {
                    *timer = 0.0;
                }
            }
        }
    }
}
