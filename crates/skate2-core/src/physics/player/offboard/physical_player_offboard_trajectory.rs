//! OffboardTrajectorySelector Launch82dbb5d8, Update82dc2f70, GetFrame82dc3280.
//! Uses the retained selector and shared World predictor. Does not admit state502.
use super::{OffboardLaunchInfo, OffboardTrajectoryOwner, RetainedPredictionResult};
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError, AttributeRuntimeInstance},
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_footplant::RetainedTrajectory,
    physics::PointGraph,
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, normalize_from_squared_and_estimate,
    },
    simulation_assembly_backend::constraint_session::{
        prediction::{
            PhysicalPredictionOwner,
            runtime::{PredictionRuntimeError, future::PredictionTarget},
        },
        world_line_query::SimulationWorldLineQueries,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as Vector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

const DT: f32 = f32::from_bits(0x3c88_8889);
const RADIUS: u64 = 0x620e_ea2e_27a3_71a2;
const OFFSET: u64 = 0x5623_5c4f_a1e0_fc78;
const INDEX_SHIFT: u64 = 0x1b6e_3ff7_ec72_cde1;
const BLEND: u64 = 0x8215_bf19_d6a7_4cd1;
const ZERO: Vector = Vector::known([0.0; 4]);
const UP: Vector = Vector::known([0.0, 1.0, 0.0, 0.0]);

#[derive(Debug)]
pub(crate) enum Error<E, N> {
    Source(E),
    Numeric(N),
    Attribute(AttributeRuntimeError),
    Prediction(PredictionRuntimeError<N>),
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    AttributeRange,
    CandidateCapacity,
    QueryNotStarted,
    FutureNotReady,
    Selector(super::selector_update::SelectorError<E, N>),
}
impl<E: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display for Error<E, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "offboard trajectory: {self:?}")
    }
}

/// Services sample live references at the call sites below. Debug draw is the
/// actual source collaborator, not an unconditional no-op. Plane intersection
/// is shared with the other tTrajectory consumers and retains numeric policy.
pub(crate) trait Services<N: ReckoningNumeric> {
    type Error;
    fn maximum_time(&mut self) -> Result<f32, Self::Error>;
    fn maximum_error_end(&mut self) -> Result<f32, Self::Error>;
    fn maximum_error_start(&mut self) -> Result<f32, Self::Error>;
    fn force_failure(&mut self) -> Result<bool, Self::Error>;
    fn debug_draw_enabled(&mut self) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
    fn processed_up_vector(&mut self) -> Result<Vector, Self::Error>;
    fn processed_dt(&mut self) -> Result<f32, Self::Error>;
    fn world_gravity(&mut self) -> Vector;
}

/// DebugDrawTrajectory82dc1080. This compiled body has no drawing calls.
/// Re-read the live flag, require the overlay only for a valid collision, then
/// evaluate the three getters in native order. COM and land index are the
/// source snapshots BEFORE Update shifts COM and publishes the new land index.
#[expect(clippy::cast_precision_loss, reason = "native signed frame conversion")]
fn debug_draw_trajectory<N: ReckoningNumeric, S: Services<N>>(
    result: &RetainedPredictionResult,
    winning_com: &RetainedTrajectory,
    land_index: i32,
    services: &mut S,
) -> Result<(), Error<S::Error, N::Error>> {
    if services.debug_draw_enabled().map_err(Error::Source)?
        && result.time.iter().all(|&time| time >= 0.0)
    {
        services.require_overlay().map_err(Error::Source)?;
        // GetCollisionPosition82d8a648 includes the native time assertion.
        let _ = super::selector_numeric::collision_position(result).map_err(Error::Selector)?;
        // GetContactPosition82db57a8: valid collision returns its stored position.
        let _ = result.position;
        // GetPositionNFramesFromStart82d89cd0; no estimate service is called.
        let _ = position_at(winning_com, land_index as f32 * DT);
    }
    Ok(())
}

fn data_word<E, N>(data: Option<&AttributeRuntimeData>, offset: usize) -> Result<u32, Error<E, N>> {
    let Some(data) = data else { return Ok(0) };
    let bytes = data
        .bytes()
        .get(offset..offset + 4)
        .ok_or(Error::AttributeRange)?;
    Ok(u32::from_be_bytes(bytes.try_into().expect("four bytes")))
}
fn scalar<E, N>(instance: &AttributeRuntimeInstance, key: u64) -> Result<f32, Error<E, N>> {
    Ok(f32::from_bits(
        instance
            .try_scalar_word(key, 0)
            .map_err(Error::Attribute)?
            .unwrap_or(0),
    ))
}
fn vector<E, N>(data: Option<&AttributeRuntimeData>, offset: usize) -> Result<Vector, Error<E, N>> {
    Ok(Vector::known([
        f32::from_bits(data_word(data, offset)?),
        f32::from_bits(data_word(data, offset + 4)?),
        f32::from_bits(data_word(data, offset + 8)?),
        f32::from_bits(data_word(data, offset + 12)?),
    ]))
}

/// Full-vector source tTrajectory evaluation; duration is not changed by shift.
pub(crate) fn position_at(trajectory: &RetainedTrajectory, time: f32) -> Vector {
    let linear = trajectory
        .velocity
        .zip(trajectory.position, |v, p| v.mul_add(time, p));
    let half_gravity = trajectory.gravity.map(|g| g * (1.0 * 0.5));
    let squared = time * time;
    half_gravity.zip(linear, |g, p| g.mul_add(squared, p))
}
pub(crate) fn velocity_at(trajectory: &RetainedTrajectory, time: f32) -> Vector {
    trajectory
        .gravity
        .zip(trajectory.velocity, |g, v| g.mul_add(time, v))
}
#[expect(
    clippy::cast_precision_loss,
    reason = "native signed integer to float rounding"
)]
pub(crate) fn shift_by_index(trajectory: &mut RetainedTrajectory, frames: i32) {
    let time = frames as f32 * DT;
    let position = position_at(trajectory, time);
    trajectory.velocity = velocity_at(trajectory, time);
    trajectory.position = position;
}

/// tTrajectory::_TrajectoryPlaneIntersection82db54e0 and greatest-time wrapper
/// 82db7098. In particular there is no zero-gravity/linear fallback. The wrapper
/// zeroes its second scratch time before testing a single root against zero.
pub(crate) fn greatest_plane_time<N: ReckoningNumeric>(
    trajectory: &RetainedTrajectory,
    point: Vector,
    normal: Vector,
    time_out: &mut [f32; 4],
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let plane_distance = numeric.dot_vector(point, normal)?;
    let position_distance = numeric.dot_vector(normal, trajectory.position)?;
    let half_seed = numeric.reciprocal_estimate(2.0)?;
    let constant = position_distance - plane_distance;
    let linear = numeric.dot_vector(normal, trajectory.velocity)?;
    let linear_squared = linear * linear;
    let half = refine_drive_reciprocal(2.0, half_seed);
    let half_gravity = trajectory.gravity.map(|g| half * g);
    let quadratic = numeric.dot_vector(normal, half_gravity)?;
    let discriminant = linear_squared - (4.0 * quadratic) * constant;
    if 0.0 > discriminant {
        return Ok(false);
    }
    let minus_linear = f32::from_bits(linear.to_bits() ^ 0x8000_0000);
    if discriminant > 0.0 {
        let sqrt_seed = numeric.reciprocal_sqrt_estimate(discriminant)?;
        let denominator = 2.0 * quadratic;
        let first_seed = numeric.reciprocal_estimate(denominator)?;
        let second_seed = numeric.reciprocal_estimate(denominator)?;
        let root = length_from_squared_and_estimate(discriminant, sqrt_seed);
        let first = refine_drive_reciprocal(denominator, first_seed) * (minus_linear + root);
        let second = refine_drive_reciprocal(denominator, second_seed) * (minus_linear - root);
        // Same host exceptional-selection boundary as other VMX min/max leaves.
        *time_out = [if first > second { first } else { second }; 4];
        Ok(true)
    } else {
        let denominator = 2.0 * quadratic;
        let seed = numeric.reciprocal_estimate(denominator)?;
        let root = refine_drive_reciprocal(denominator, seed) * minus_linear;
        *time_out = [root; 4];
        Ok(root > 0.0)
    }
}

impl OffboardTrajectoryOwner {
    fn gravity<E, N>(&self) -> Result<Vector, Error<E, N>> {
        let layout = self
            .physics_attributes
            .try_cached_layout(0x10)
            .map_err(Error::Attribute)?;
        vector(layout.as_ref(), 0)
    }
    /// InitTrajectoryInfo82dbb358: count is not capped to onboard's seven and
    /// cone angles are never read. Every local velocity delta is full zero.
    fn initialize<N: ReckoningNumeric, S: Services<N>>(
        &mut self,
        services: &mut S,
    ) -> Result<(), Error<S::Error, N::Error>> {
        self.num_trajectories = i32::from_ne_bytes(self.launch.num_trajectories.to_ne_bytes());
        let radius_data = self
            .attributes
            .try_get_data(RADIUS, 0)
            .map_err(Error::Attribute)?;
        let offset_data = self
            .attributes
            .try_get_data(OFFSET, 0)
            .map_err(Error::Attribute)?;
        let radius = f32::from_bits(data_word(radius_data.as_ref(), 0)?);
        self.trajectory_offset = vector(offset_data.as_ref(), 0)?.map(|lane| lane - radius);
        let position = self
            .launch
            .start_com_position
            .zip(self.trajectory_offset, |a, b| a + b);
        let shift = i32::from_ne_bytes(
            self.attributes
                .try_scalar_word(INDEX_SHIFT, 0)
                .map_err(Error::Attribute)?
                .unwrap_or(0)
                .to_ne_bytes(),
        );
        // Safe-host boundary for native fixed arrays; do not clamp authored count.
        if self.num_trajectories > 16 {
            return Err(Error::CandidateCapacity);
        }
        for index in 0..self.num_trajectories.max(0) as usize {
            self.data[index].index_shift = shift;
            let velocity = self.launch.start_velocity.map(|lane| lane + 0.0);
            let duration = services.maximum_time().map_err(Error::Source)?;
            let gravity = self.gravity()?;
            self.data[index].trajectory = RetainedTrajectory {
                position,
                velocity,
                gravity,
                time: [duration; 4],
            };
            shift_by_index(&mut self.data[index].trajectory, shift);
        }
        Ok(())
    }

    /// Source-time readiness and score writes retain completed prefixes on error.
    pub(crate) fn update<N: ReckoningNumeric, S: Services<N>>(
        &mut self,
        services: &mut S,
    ) -> Result<bool, Error<S::Error, N::Error>> {
        self.has_just_changed = false;
        if !self.just_started_query {
            return Err(Error::QueryNotStarted);
        }
        if !self.queries[0].ready {
            return Ok(self.ready);
        }
        self.just_started_query = false;
        self.best_trajectory = 0;
        self.winning_result = None;
        if self.num_trajectories > 16 {
            return Err(Error::CandidateCapacity);
        }
        for index in 0..self.num_trajectories.max(0) as usize {
            self.scores[index] = 0.0;
            let query = &mut self.queries[index];
            if !query.ready {
                return Err(Error::FutureNotReady);
            }
            let result = &mut query.result;
            let data = &mut self.data[index];
            // vcmpgefp record branch is ALL four time lanes, not just X.
            if result.time.iter().all(|&t| t >= 0.0)
                && !services.force_failure().map_err(Error::Source)?
            {
                data.landing_position =
                    super::selector_numeric::collision_position(result).map_err(Error::Selector)?;
                let normal = if result.time.iter().all(|&t| t >= 0.0) {
                    result.ground_normal
                } else {
                    UP
                };
                result.normal = normal;
                data.landing_normal = normal;
                data.landing_velocity = landing_velocity(result);
                data.collision_index = result.frame;
                data.collision_found = true;
                if result.surface & 0xf80 == 0x300 {
                    result.normal = services.processed_up_vector().map_err(Error::Source)?;
                }
            } else {
                data.landing_normal = UP;
                data.landing_velocity = ZERO;
                data.collision_index = 120;
                data.collision_found = false;
                // Landing position intentionally retained on miss.
            }
        }
        self.scores[0] += 1.0;
        let mut best_score = -100_000_000.0;
        for index in 0..self.num_trajectories.max(0) as usize {
            if self.scores[index] > best_score {
                best_score = self.scores[index];
                self.best_trajectory = index as i32;
            }
        }
        let best = self.best_trajectory as usize;
        if !self.queries[best].ready {
            return Err(Error::FutureNotReady);
        }
        self.winning_result = Some(best);
        self.winning_data = self.data[best];
        self.winning_com = self.data[best].trajectory;
        if services.debug_draw_enabled().map_err(Error::Source)? {
            debug_draw_trajectory::<N, S>(
                &self.queries[best].result,
                &self.winning_com,
                self.land_index,
                services,
            )?;
        }
        shift_by_index(
            &mut self.winning_com,
            self.data[best].index_shift.wrapping_neg(),
        );
        // Source XORs the offset sign bit then adds, preserving signed-zero behavior.
        self.winning_com.position = self
            .winning_com
            .position
            .zip(self.trajectory_offset, |position, offset| {
                position + f32::from_bits(offset.to_bits() ^ 0x8000_0000)
            });
        self.land_index = self.data[best]
            .collision_index
            .wrapping_add(self.data[best].index_shift);
        if services.debug_draw_enabled().map_err(Error::Source)? {
            services.require_overlay().map_err(Error::Source)?;
        }
        self.ready = true;
        Ok(true)
    }
}

#[expect(clippy::cast_precision_loss, reason = "native frame conversion")]
fn landing_velocity(result: &RetainedPredictionResult) -> Vector {
    if result.time.iter().all(|&t| t >= 0.0) {
        super::selector_numeric::landing_velocity(result)
    } else {
        ZERO
    }
}

/// The actual queued predictor targets the existing selector future. Drop the
/// selector lock before submission: AddQuery resets that same retained future.
#[expect(
    clippy::too_many_arguments,
    reason = "retained owner plus actual World collaborators"
)]
pub(crate) fn launch<N: ReckoningNumeric, S: Services<N>>(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    launch: OffboardLaunchInfo,
    services: &mut S,
    predictor: &PhysicalPredictionOwner,
    world: &mut SimulationWorldLineQueries,
    ground: &StaticGroundPool,
    dmos: &SimulationDmoAssemblyPool,
    numeric: &mut N,
) -> Result<(), Error<S::Error, N::Error>> {
    let processed = Arc::clone(
        owner
            .reckoning()
            .lock()
            .map_err(|_| Error::Poisoned)?
            .processed(),
    );
    {
        let mut owners = owner.trajectories().lock().map_err(|_| Error::Poisoned)?;
        let selector = owners
            .offboard
            .as_mut()
            .ok_or(Error::Missing("offboard selector"))?;
        if !selector.bindings.matches(owner, &processed) {
            return Err(Error::ForeignOwner);
        }
        selector.reset();
        selector.raw_trajectory = RetainedTrajectory {
            position: launch.start_com_position,
            velocity: launch.start_velocity,
            gravity: selector.gravity()?,
            time: [2.0; 4],
        };
        selector.best_trajectory = -1;
        selector.trajectory_blend = 0.0;
        selector.trajectory_blend_time = 0.0;
        selector.launch = launch;
        selector.ready = false;
        selector.initialize::<N, S>(services)?;
    }
    let mut slot = 0;
    loop {
        let (trajectory, attributes) = {
            let owners = owner.trajectories().lock().map_err(|_| Error::Poisoned)?;
            let selector = owners
                .offboard
                .as_ref()
                .ok_or(Error::Missing("offboard selector"))?;
            if slot >= selector.num_trajectories.max(0) as usize {
                break;
            }
            (selector.data[slot].trajectory, selector.attributes.clone())
        };
        let maximum_error_end = services.maximum_error_end().map_err(Error::Source)?;
        let maximum_error_start = services.maximum_error_start().map_err(Error::Source)?;
        let duration = services.maximum_time().map_err(Error::Source)?;
        let radius = scalar(&attributes, RADIUS)?;
        predictor
            .try_add_query(
                owner,
                PredictionTarget::Offboard(slot),
                BallisticTrajectoryInput {
                    position: trajectory.position,
                    velocity: trajectory.velocity,
                    gravity: trajectory.gravity,
                    radius,
                    duration,
                    maximum_error_start,
                    maximum_error_end,
                },
                false,
                world,
                ground,
                dmos,
                numeric,
            )
            .map_err(Error::Prediction)?;
        slot += 1;
    }
    owner
        .trajectories()
        .lock()
        .map_err(|_| Error::Poisoned)?
        .offboard
        .as_mut()
        .ok_or(Error::Missing("offboard selector"))?
        .just_started_query = true;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub(crate) com_position: Vector,
    pub(crate) com_velocity: Vector,
    pub(crate) landing_position: Vector,
    pub(crate) landing_normal: Vector,
    pub(crate) landing_velocity: Vector,
    pub(crate) time_to_land: f32,
    pub(crate) frame: i32,
    pub(crate) landing_valid: bool,
}

impl OffboardTrajectoryOwner {
    #[expect(clippy::cast_precision_loss, reason = "native signed frame conversion")]
    pub(crate) fn frame<N: ReckoningNumeric, S: Services<N>>(
        &mut self,
        frame: i32,
        height_adjust: f32,
        services: &mut S,
        numeric: &mut N,
    ) -> Result<Frame, Error<S::Error, N::Error>> {
        self.current_index = frame;
        if self.just_started_query {
            self.update::<N, S>(services)?;
        }
        if height_adjust > 0.0 {
            let y_scale = scalar(&self.attributes, 0x1e1f_10ff_af84_217e)?;
            let y_max = scalar(&self.attributes, 0x7a92_6737_7ad5_a725)?;
            let speed_threshold = scalar(&self.attributes, 0x5933_0b31_13f6_6b4d)?;
            let speed_scale = scalar(&self.attributes, 0xe69c_9413_adfa_d13b)?;
            let previous = frame.wrapping_sub(1);
            let mut velocity = velocity_at(&self.winning_com, previous as f32 * DT);
            let y = velocity.xyz[1] + y_scale * height_adjust;
            let y = if y_max - y >= 0.0 { y } else { y_max };
            let mut horizontal = velocity;
            horizontal.xyz[1] = 0.0;
            let squared = numeric
                .dot_vector(horizontal, horizontal)
                .map_err(Error::Numeric)?;
            let estimate = numeric
                .reciprocal_sqrt_estimate(squared)
                .map_err(Error::Numeric)?;
            let speed = length_from_squared_and_estimate(squared, estimate);
            if speed > speed_threshold {
                let estimate = numeric
                    .reciprocal_sqrt_estimate(squared)
                    .map_err(Error::Numeric)?;
                let normalized = normalize_from_squared_and_estimate(horizontal, squared, estimate);
                let adjusted = (speed - speed_threshold).mul_add(speed_scale, speed_threshold);
                velocity = normalized.map(|lane| lane * adjusted);
            }
            velocity.xyz[1] = y;
            shift_by_index(&mut self.winning_com, previous);
            self.winning_com.velocity = velocity;
            shift_by_index(&mut self.winning_com, 1_i32.wrapping_sub(frame));
        }
        let time = frame as f32 * DT;
        let mut output = Frame {
            com_position: ZERO,
            com_velocity: ZERO,
            landing_position: ZERO,
            landing_normal: UP,
            landing_velocity: ZERO,
            time_to_land: 0.0,
            frame,
            landing_valid: false,
        };
        if self.just_started_query {
            output.com_position = position_at(&self.raw_trajectory, time);
            output.com_velocity = velocity_at(&self.raw_trajectory, time);
        } else {
            self.trajectory_blend_time =
                services.processed_dt().map_err(Error::Source)? + self.trajectory_blend_time;
            let data = self
                .attributes
                .try_get_data(BLEND, 0)
                .map_err(Error::Attribute)?;
            let mut graph = PointGraph::<8>::default();
            for index in 0..8 {
                graph.x[index] = f32::from_bits(data_word(data.as_ref(), 0x10 + index * 4)?);
                graph.y[index] = f32::from_bits(data_word(data.as_ref(), 0x30 + index * 4)?);
            }
            self.trajectory_blend = graph.evaluate(self.trajectory_blend_time);
            let winning =
                position_at(&self.winning_com, time).map(|lane| lane * self.trajectory_blend);
            let raw = position_at(&self.raw_trajectory, time);
            output.com_position =
                raw.zip(winning, |r, w| r.mul_add(1.0 - self.trajectory_blend, w));
            output.com_velocity = velocity_at(&self.winning_com, time);
            output.landing_valid = self.winning_data.collision_found;
            if self.best_trajectory < 0 {
                return Err(Error::Missing("winning trajectory"));
            }
            output.landing_position = self.winning_data.landing_position;
            output.landing_normal = self.winning_data.landing_normal;
            output.landing_velocity = self.winning_data.landing_velocity;
        }
        if height_adjust < 0.0 {
            let trajectory = RetainedTrajectory {
                position: output.com_position,
                velocity: output.com_velocity,
                gravity: services.world_gravity(),
                time: [-1.0; 4],
            };
            let mut point = output.com_position;
            point.xyz[1] += height_adjust;
            let mut time = [0.0; 4];
            if greatest_plane_time(&trajectory, point, UP, &mut time, numeric)
                .map_err(Error::Numeric)?
            {
                output.time_to_land = time[0];
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
#[path = "physical_player_offboard_trajectory_tests.rs"]
mod tests;
