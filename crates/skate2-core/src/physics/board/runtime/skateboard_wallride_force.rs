//! Complete source branch graph for `Toolkit_CalcWallRideForce82b29300`.
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_typed},
    skateboard_foot_forces::SkateboardFootForce,
    skeleton_ik_line_frame::normalize_safe,
    skeleton_root_transforms::SkeletonRootVector,
};

const ZERO: SkeletonRootVector = SkeletonRootVector::known([0.0; 4]);
#[cfg(test)]
#[path = "skateboard_wallride_force_tests.rs"]
mod tests;
#[derive(Clone, Copy)]
pub(crate) enum WallRideVector {
    GroundNormal,
    Up,
    Velocity,
    DeckPosition,
}
#[derive(Clone, Copy)]
pub(crate) enum WallRideScalar {
    Mass,
    Gravity,
}
pub(crate) struct WallRideHit {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
}
/// Implementations must borrow the actual owners, including the retained feet
/// binding and same World query. No default hit or fixed tuning belongs here.
pub(crate) trait WallRideSource {
    type Error;
    type Scalar;
    type TrajectoryLayout;
    type DebugTarget;
    fn capture_feet(&mut self, key: u64) -> Result<Self::Scalar, Self::Error>;
    fn load_feet(&mut self, value: &Self::Scalar) -> Result<f32, Self::Error>;
    fn feet_graph(&mut self, time: f32) -> Result<f32, Self::Error>;
    fn capture_trajectory_layout(&mut self) -> Result<Self::TrajectoryLayout, Self::Error>;
    fn trajectory_height(&mut self, layout: &Self::TrajectoryLayout) -> Result<f32, Self::Error>;
    fn vector(&mut self, field: WallRideVector) -> Result<SkeletonRootVector, Self::Error>;
    fn scalar(&mut self, field: WallRideScalar) -> Result<f32, Self::Error>;
    fn wheels(&mut self) -> Result<i32, Self::Error>;
    fn capture_debug(&mut self) -> Result<Self::DebugTarget, Self::Error>;
    fn debug_value(&mut self, target: &Self::DebugTarget) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
    fn line(
        &mut self,
        start: SkeletonRootVector,
        end: SkeletonRootVector,
    ) -> Result<Option<WallRideHit>, Self::Error>;
    fn clear_jump_flag(&mut self) -> Result<(), Self::Error>;
    fn clear_jump_height(&mut self) -> Result<(), Self::Error>;
    fn publish_jump_flag(&mut self) -> Result<(), Self::Error>;
    fn publish_jump_velocity(&mut self, velocity: SkeletonRootVector) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum WallRideError<S, N> {
    Source(S),
    Numeric(N),
}
impl<S: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for WallRideError<S, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(e) => write!(f, "wallride source: {e:?}"),
            Self::Numeric(e) => write!(f, "wallride numeric: {e:?}"),
        }
    }
}
fn feet<S: WallRideSource>(source: &mut S, key: u64) -> Result<f32, S::Error> {
    let handle = source.capture_feet(key)?;
    source.load_feet(&handle)
}
fn debug<S: WallRideSource>(source: &mut S) -> Result<(), S::Error> {
    let target = source.capture_debug()?;
    if source.debug_value(&target)? {
        source.require_overlay()?;
    }
    Ok(())
}
fn scale(v: SkeletonRootVector, s: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: v.xyz.map(|x| x * s),
        w: v.w.map(|x| x * s),
    }
}
fn add(a: SkeletonRootVector, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i] + b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a + b),
    }
}
fn fused(a: SkeletonRootVector, s: f32, b: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|i| a.xyz[i].mul_add(s, b.xyz[i])),
        w: a.w.zip(b.w).map(|(a, b)| a.mul_add(s, b)),
    }
}

/// Caller supplies its source-initialized output record. A degenerate active
/// direction returns false BEFORE common output stores and preserves that record.
pub(crate) fn try_calculate<S: WallRideSource, N: ReckoningNumeric>(
    source: &mut S,
    numeric: &mut N,
    ground_time: f32,
    wall_time: f32,
    output: &mut SkateboardFootForce,
) -> Result<bool, WallRideError<S::Error, N::Error>> {
    use WallRideError as E;
    let max_y = feet(source, 0xbaba_9160_6102_af9a).map_err(E::Source)?;
    let min_up_dot = feet(source, 0x19c8_04c3_e3e2_b566).map_err(E::Source)?;
    let hit_dot_limit = feet(source, 0x90c4_e79c_bfba_ed5e).map_err(E::Source)?;
    source.clear_jump_flag().map_err(E::Source)?;
    source.clear_jump_height().map_err(E::Source)?;
    let normal_for_dot = source
        .vector(WallRideVector::GroundNormal)
        .map_err(E::Source)?;
    let up = source.vector(WallRideVector::Up).map_err(E::Source)?;
    let up_dot = numeric.dot_vector(up, normal_for_dot).map_err(E::Numeric)?;
    let normal_for_y = source
        .vector(WallRideVector::GroundNormal)
        .map_err(E::Source)?;
    let active = max_y > normal_for_y.xyz[1].abs() && up_dot > min_up_dot;
    let mut force = ZERO;
    if active {
        let normal = source
            .vector(WallRideVector::GroundNormal)
            .map_err(E::Source)?;
        let velocity = source.vector(WallRideVector::Velocity).map_err(E::Source)?;
        let force_time = feet(source, 0xd91a_09c7_e923_7430).map_err(E::Source)?;
        let graph = source.feet_graph(ground_time).map_err(E::Source)?;
        let mass = source.scalar(WallRideScalar::Mass).map_err(E::Source)?;
        let gravity = source.scalar(WallRideScalar::Gravity).map_err(E::Source)?;
        let weight = mass * gravity;
        let target = source.capture_debug().map_err(E::Source)?;
        let draw = source.debug_value(&target).map_err(E::Source)?;
        force = SkeletonRootVector::known([0.0, -(weight * graph), 0.0, 0.0]);
        if draw {
            source.require_overlay().map_err(E::Source)?;
        }
        let wheels_allow = if ground_time > f32::from_bits(0x3c23_d70a) {
            source.wheels().map_err(E::Source)? > 0
        } else {
            false
        };
        if wheels_allow || ground_time > force_time {
            let trajectory = source.capture_trajectory_layout().map_err(E::Source)?;
            let first_cross = cross_typed(SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]), normal);
            let position = source
                .vector(WallRideVector::DeckPosition)
                .map_err(E::Source)?;
            let offset = source.trajectory_height(&trajectory).map_err(E::Source)?;
            let second_cross = cross_typed(first_cross, normal);
            let start = fused(normal, offset, position);
            let direction = normalize_safe(second_cross, numeric).map_err(E::Numeric)?;
            let displacement = scale(direction, 4.0);
            let squared = numeric
                .dot_vector(displacement, displacement)
                .map_err(E::Numeric)?;
            if f32::from_bits(0x3780_0000) > squared {
                return Ok(false);
            }
            let end = add(start, displacement);
            let hit = source.line(start, end).map_err(E::Source)?;
            debug(source).map_err(E::Source)?;
            if let Some(hit) = hit {
                let height = feet(source, 0x843a_276a_c05b_4a8b).map_err(E::Source)?;
                let max_time = source
                    .capture_feet(0x569d_f113_0ffd_dbb6)
                    .map_err(E::Source)?;
                let hit_y = hit.position.xyz[1];
                let start_y = start.xyz[1];
                let max_time = source.load_feet(&max_time).map_err(E::Source)?;
                let vel_time = feet(source, 0xc2bc_e9a7_d58b_e7bb).map_err(E::Source)?;
                let delta_y = hit_y - start_y;
                let target = source.capture_debug().map_err(E::Source)?;
                let alignment = numeric.dot_vector(hit.normal, normal).map_err(E::Numeric)?;
                let draw = source.debug_value(&target).map_err(E::Source)?;
                let predicted_height = velocity.xyz[1].mul_add(vel_time, delta_y.abs());
                if draw {
                    source.require_overlay().map_err(E::Source)?;
                }
                if alignment < hit_dot_limit && (predicted_height < height || wall_time > max_time)
                {
                    let along = numeric.dot_vector(normal, velocity).map_err(E::Numeric)?;
                    source.publish_jump_flag().map_err(E::Source)?;
                    let projection = scale(normal, along);
                    let mut tangent = SkeletonRootVector {
                        xyz: std::array::from_fn(|i| velocity.xyz[i] - projection.xyz[i]),
                        w: velocity.w.zip(projection.w).map(|(v, p)| v - p),
                    };
                    if 0.0 > tangent.xyz[1] {
                        let down = feet(source, 0x7291_4fa6_24d2_947b).map_err(E::Source)?;
                        tangent.xyz[1] *= down;
                    }
                    let jump = feet(source, 0xfd00_fa38_6f0a_c466).map_err(E::Source)?;
                    source
                        .publish_jump_velocity(fused(normal, jump, tangent))
                        .map_err(E::Source)?;
                    debug(source).map_err(E::Source)?;
                }
            }
        }
    }
    output.world_space_force = force;
    output.local_space_position = ZERO;
    Ok(active)
}
