//! `CalculateScores82dc5e00` and `AddWallRideScores82db96f8` on retained results.
use super::{
    OnboardTrajectoryOwner,
    selector_numeric::{adjust, collision_position, landing_velocity},
    selector_update::{SelectorError, SelectorGrindPolicy, SelectorSource, count, ready},
};
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed, try_length, try_normalize_safe},
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
};
const ZERO: V = V::known([0.0; 4]);
const UP: V = V::known([0.0, 1.0, 0.0, 0.0]);

#[expect(
    clippy::too_many_lines,
    clippy::neg_multiply,
    reason = "source-ordered scoring with lazy reads; preserve native gravity multiply by minus one"
)]
pub(super) fn calculate_scores<S: SelectorSource, N: ReckoningNumeric>(
    s: &mut OnboardTrajectoryOwner,
    src: &mut S,
    n: &mut N,
    policy: SelectorGrindPolicy,
) -> Result<(), SelectorError<S::Error, N::Error>> {
    use SelectorError::{Numeric, Source};
    let frames = src.minimum_frames().map_err(Source)?;
    let layout = src.layout().map_err(Source)?;
    let middle = src.scalar(&layout, 0x1d4).map_err(Source)?;
    let direction_factor = src.scalar(&layout, 0x1dc).map_err(Source)?;
    let force_factor = src.scalar(&layout, 0x1d8).map_err(Source)?;
    let apex_min = src.scalar(&layout, 0x1ec).map_err(Source)?;
    let transition_factor = src.scalar(&layout, 0x1c8).map_err(Source)?;
    for i in 0..count(s)? {
        s.wallride_scores[i] = 0.0;
        s.scores[i] = 0.0;
        s.is_wallride[i] = false;
        let r = ready(s, i)?;
        if r.frame < frames || !r.time.iter().all(|t| *t >= 0.0) {
            continue;
        }
        let central = i == 0 && s.pass == 1;
        let mut middle = if central && !s.on_vert { middle } else { 0.0 };
        if src.central().map_err(Source)? && i == 0 {
            middle = 1_000_000.0;
        }
        s.data[i].landing_position = collision_position(&r)?;
        let normal = if r.time.iter().all(|t| *t >= 0.0) {
            r.ground_normal
        } else {
            UP
        };
        s.queries[i].result.normal = normal;
        s.data[i].landing_normal = normal;
        let velocity = landing_velocity(&r);
        s.data[i].landing_velocity = velocity;
        let unit = try_normalize_safe(velocity, ZERO, n).map_err(Numeric)?;
        let dot = n.dot_vector(normal, velocity).map_err(Numeric)?;
        let cross = cross_typed(normal, unit);
        let squared = n.dot_vector(cross, cross).map_err(Numeric)?;
        let seed = n.reciprocal_sqrt_estimate(squared).map_err(Numeric)?;
        let length = crate::reckoning_numeric::length_from_squared_and_estimate(squared, seed);
        let factor = src.graph(&layout, 0xf0, 8, dot.abs()).map_err(Source)?;
        let force = (dot * force_factor) * factor;
        let direction = (length * direction_factor) * factor;
        // The parked host policy keeps the grind term zero and never reaches
        // the grind score or penalty; native scoring runs both.
        let mut grind = 0.0;
        if policy == SelectorGrindPolicy::Native {
            if central && src.grind_scoring_allowed().map_err(Source)? {
                grind = src.grind_score(s, i, velocity).map_err(Source)?;
            }
            if src.avoid_grinds().map_err(Source)? || grind == 0.0 {
                grind = src.grind_penalty(s, i).map_err(Source)?;
            }
        }
        // Only an ordered positive grind score skips the wall probe.
        if !(grind > 0.0) {
            let wall = wall_score(s, i, src, n, grind < 0.0)?;
            s.wallride_scores[i] = wall;
            if wall > 0.0 {
                s.is_wallride[i] = true;
            } else if wall < 0.0 {
                s.queries[i].result.normal = UP;
            }
        }
        let surface = (s.queries[i].result.surface >> 7) & 31;
        let surface_score = match surface {
            6..=8 => {
                let layout = src.layout().map_err(Source)?;
                let up = if surface == 8 {
                    None
                } else {
                    Some(src.up().map_err(Source)?)
                };
                let score = src
                    .scalar(&layout, if surface == 6 { 0x1cc } else { 0x1d0 })
                    .map_err(Source)?;
                if let Some(up) = up {
                    s.queries[i].result.normal = up;
                }
                score
            }
            _ => 0.0,
        };
        s.data[i].landing_normal = s.queries[i].result.normal;
        let transition = if s.on_vert {
            let sign = if src.transition().map_err(Source)? < 0.5 {
                -1.0
            } else {
                1.0
            };
            // Reversing on vert forfeits the grind bonus for the layout+0xa0
            // penalty at zero distance.
            if grind > 0.0 && sign < 0.0 {
                s.grind_bonus_awarded = false;
                let layout = src.layout().map_err(Source)?;
                grind = src.graph(&layout, 0xb0, 8, 0.0).map_err(Source)?;
            }
            let ground = src.ground_normal().map_err(Source)?;
            let mut velocity = src.ground_velocity().map_err(Source)?;
            velocity.xyz[1] = 0.0;
            let horizontal = try_normalize_safe(velocity, ZERO, n).map_err(Numeric)?;
            let r = s.queries[i].result;
            let time = r.time[0];
            if time.abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
                return Err(SelectorError::CollisionTime(time.to_bits()));
            }
            let g = r.trajectory.gravity.xyz;
            if g[0] != 0.0
                || g[2] != 0.0
                || matches!(
                    g[1].partial_cmp(&0.0),
                    None | Some(std::cmp::Ordering::Greater)
                )
            {
                return Err(SelectorError::Missing("gravity pointing down"));
            }
            let vy = r.trajectory.velocity.xyz[1];
            let neg_g = g[1] * -1.0;
            let apex = if 0.0 > vy || 0.0 >= neg_g {
                0.0
            } else {
                let seed = n.reciprocal_estimate(neg_g).map_err(Numeric)?;
                refine_drive_reciprocal(neg_g, seed) * vy
            };
            let penalty = if (time - apex).abs() < apex_min {
                -500.0
            } else {
                0.0
            };
            let alignment = n
                .dot_vector(s.data[i].landing_normal, horizontal)
                .map_err(Numeric)?;
            alignment * ((1.0 - ground.xyz[1].abs()) * sign * transition_factor) + penalty
        } else {
            0.0
        };
        let time = s.queries[i].result.time[0];
        if time.abs().partial_cmp(&100.0) != Some(std::cmp::Ordering::Less) {
            return Err(SelectorError::CollisionTime(time.to_bits()));
        }
        let time_bonus = src.graph(&layout, 0x10, 8, time).map_err(Source)?;
        s.scores[i] = (time_bonus
            + (((((direction + 1.0) + force) + middle) + grind) + transition))
            + surface_score;
    }
    Ok(())
}

/// Both feet getters execute before the wall-normal gate. Probe starts from the
/// stored collision point, while later approach/height comparisons use ballistic
/// `GetCollisionPosition`; these must not be collapsed into a single point.
fn wall_score<S: SelectorSource, N: ReckoningNumeric>(
    selector: &mut OnboardTrajectoryOwner,
    index: usize,
    src: &mut S,
    numeric: &mut N,
    near_grind: bool,
) -> Result<f32, SelectorError<S::Error, N::Error>> {
    use SelectorError::{Numeric, Source};
    let max_y = src.feet(0xbaba_9160_6102_af9a).map_err(Source)?;
    let max_dot = src.feet(0x90c4_e79c_bfba_ed5e).map_err(Source)?;
    let result = selector.queries[index].result;
    if result.normal.xyz[1].abs().partial_cmp(&max_y) != Some(std::cmp::Ordering::Less) {
        if src.wall_debug().map_err(Source)? {
            src.overlay().map_err(Source)?;
        }
        return Ok(0.0);
    }
    let layout = src.layout().map_err(Source)?;
    let distance = src.scalar(&layout, 0x1a0).map_err(Source)?;
    let p = if result.time.iter().all(|t| *t >= 0.0) {
        result.position
    } else {
        ZERO
    };
    let start = p.zip(result.normal.map(|v| v * distance), |p, d| p + d);
    let end = start.zip(V::known([0.0, -10.0, 0.0, 0.0]), |a, b| a + b);
    let hit = src.line(start, end).map_err(Source)?;
    let mut score = 0.0;
    if let Some((position, normal)) = hit {
        let layout = src.layout().map_err(Source)?;
        let dot = numeric.dot_vector(normal, result.normal).map_err(Numeric)?;
        let per_height = src.scalar(&layout, 0x1c4).map_err(Source)?;
        let height = (position.xyz[1] - start.xyz[1]).abs();
        if max_dot > dot {
            if near_grind {
                score = height * per_height - 1000.0;
            } else {
                let delta = result
                    .trajectory
                    .position
                    .zip(collision_position(&result)?, |a, b| a - b);
                let mut horizontal = delta;
                horizontal.xyz[1] = 0.0;
                let length = try_length(horizontal, numeric).map_err(Numeric)?;
                let slope = if length > 0.01 {
                    let seed = numeric.reciprocal_estimate(length).map_err(Numeric)?;
                    refine_drive_reciprocal(length, seed) * delta.xyz[1]
                } else {
                    delta.xyz[1]
                };
                let layout = src.layout().map_err(Source)?;
                let max_slope = src.scalar(&layout, 0x1a8).map_err(Source)?;
                if matches!(
                    slope.partial_cmp(&max_slope),
                    None | Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                ) {
                    let layout = src.layout().map_err(Source)?;
                    let min_height = src.scalar(&layout, 0x1a4).map_err(Source)?;
                    if matches!(
                        height.partial_cmp(&min_height),
                        None | Some(std::cmp::Ordering::Equal | std::cmp::Ordering::Greater)
                    ) {
                        let layout = src.layout().map_err(Source)?;
                        let boost = src.graph(&layout, 0x180, 4, height).map_err(Source)?;
                        score = height * per_height;
                        let p = collision_position(&selector.queries[index].result)?;
                        if p.xyz[1] > selector.highest_wallride_position {
                            selector.highest_wallride_position =
                                collision_position(&selector.queries[index].result)?.xyz[1];
                        }
                        let layout = src.layout().map_err(Source)?;
                        let maximum = src.scalar(&layout, 0x1f4).map_err(Source)?;
                        adjust(
                            &mut selector.queries[index].result.trajectory,
                            result.frame,
                            V::known([0.0, boost, 0.0, 0.0]),
                            maximum,
                            numeric,
                        )?;
                    } else {
                        score = height - 1000.0;
                    }
                } else {
                    score = height * per_height - 1000.0;
                }
            }
        }
        if src.wall_debug().map_err(Source)? {
            for _ in 0..3 {
                src.overlay().map_err(Source)?;
            }
        }
    } else if src.wall_debug().map_err(Source)? {
        for _ in 0..2 {
            src.overlay().map_err(Source)?;
        }
    }
    // Final source cvar is re-read even after a successful wall probe.
    let _ = src.wall_debug().map_err(Source)?;
    Ok(score)
}

pub(super) fn add_wall_scores<S: SelectorSource, N>(
    s: &mut OnboardTrajectoryOwner,
    src: &mut S,
) -> Result<(), SelectorError<S::Error, N>> {
    use SelectorError::Source;
    let frames = src.minimum_frames().map_err(Source)?;
    let highest = s.highest_wallride_position;
    for i in 0..count(s)? {
        let r = ready(s, i)?;
        if r.frame < frames || !r.time.iter().all(|t| *t >= 0.0) {
            continue;
        }
        let p = collision_position(&r)?;
        if s.wallride_scores[i] == 0.0 && p.xyz[1] > highest {
            s.wallride_scores[i] = 2000.0;
        }
        let combined = s.wallride_scores[i] + s.scores[i];
        s.scores[i] = combined;
    }
    Ok(())
}
#[cfg(test)]
#[path = "physical_player_selector_scoring_tests.rs"]
mod tests;
