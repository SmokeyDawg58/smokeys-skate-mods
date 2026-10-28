//! OffboardJumpingTest StartQuery82dd5078, CalculateResults82dd5660, Tick82dd5f98,
//! the finalizer82dd5fd0 called by OffBoard.Exit and Reset82dda070, on the
//! existing retained world owner. Vector steps follow the raw VX128 operand order.
use super::OffboardJumping;
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_query_batches::trajectory::{BallisticTrajectoryInput, TrajectoryResult},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineError, SimulationWorldLineLeaseError, SimulationWorldLineQueries,
        trajectory::WorldTrajectoryError,
    },
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};

/// IWorld vtable+0x30/+0x3c single-argument overloads pass BF_Default.
const DEFAULT_FLAGS: u32 = 7;
const LABEL: &str = "SkateboardBody::StartWheelLineTest";

pub(crate) trait JumpingTuning {
    type Error;
    fn max_jump_distance(&mut self) -> Result<f32, Self::Error>;
    fn min_jump_distance(&mut self) -> Result<f32, Self::Error>;
    fn test_radius(&mut self) -> Result<f32, Self::Error>;
    fn max_jump_height(&mut self) -> Result<f32, Self::Error>;
    fn absolute_max_jump_distance(&mut self) -> Result<f32, Self::Error>;
    fn max_jump_down(&mut self) -> Result<f32, Self::Error>;
    fn minimum_distance_from_wall(&mut self) -> Result<f32, Self::Error>;
    fn minimum_distance_from_top_of_wall(&mut self) -> Result<f32, Self::Error>;
    fn minimum_ground_normal_y(&mut self) -> Result<f32, Self::Error>;
    fn debug_draw(&mut self) -> Result<bool, Self::Error>;
    fn force_bad_trajectory(&mut self) -> Result<bool, Self::Error>;
}
pub(crate) struct JumpingWorld<'a> {
    pub(crate) queries: &'a mut SimulationWorldLineQueries,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) dmos: &'a SimulationDmoAssemblyPool,
}
/// PDB JumpingTestInput: groundPos, jumpDirection, jumpDist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct JumpingInput {
    pub(crate) ground: V,
    pub(crate) direction: V,
    pub(crate) distance: f32,
}
#[derive(Debug)]
pub(crate) enum JumpingError<N, T> {
    ActiveLine,
    ActiveTrajectory,
    MissingLine,
    MissingTrajectory,
    /// kOBJT_DebugDraw asserts on a DebugOverlay singleton the port does not own.
    PendingDebugDraw,
    Lease(SimulationWorldLineLeaseError),
    Line(SimulationWorldLineError<N>),
    Trajectory(WorldTrajectoryError<N>),
    Numeric(N),
    Tuning(T),
}

fn up(value: f32) -> V {
    V::known([0.0, value, 0.0, 0.0])
}
fn add(a: V, b: V) -> V {
    a.zip(b, |a, b| a + b)
}
fn sub(a: V, b: V) -> V {
    a.zip(b, |a, b| a - b)
}
fn lanes(value: V) -> [f32; 4] {
    [
        value.xyz[0],
        value.xyz[1],
        value.xyz[2],
        value.w.unwrap_or(0.0),
    ]
}
fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}
/// `fcmpu/ble` then `fcmpu/bge`: only an ordered value strictly inside moves.
fn push_lower(value: f32, near: f32, far: f32) -> f32 {
    if value > near && value < far {
        far
    } else {
        value
    }
}
fn pull_upper(value: f32, near: f32, far: f32) -> f32 {
    if value > near && value < far {
        near
    } else {
        value
    }
}

impl OffboardJumping {
    pub(crate) fn start_query<N: ReckoningNumeric + AssemblyGroundNumeric, T: JumpingTuning>(
        &mut self,
        input: JumpingInput,
        world: &mut JumpingWorld<'_>,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), JumpingError<N::Error, T::Error>> {
        use JumpingError as E;
        // 82dd50b4 clears the counter before the three batch assertions.
        self.result_valid_counter = 0;
        if self.line_batch.is_some() {
            return Err(E::ActiveLine);
        }
        if self.trajectory_batch_one.is_some() || self.trajectory_batch_two.is_some() {
            return Err(E::ActiveTrajectory);
        }
        self.ground_position = lanes(input.ground);
        self.jump_direction = lanes(input.direction);
        self.jump_distance = input.distance;
        self.line_batch = Some(
            world
                .queries
                .try_acquire_line_batch(world.dmos, LABEL, DEFAULT_FLAGS)
                .map_err(E::Lease)?,
        );
        let lines = self.line_batch.as_ref().expect("published");
        world.queries.try_begin(lines).map_err(E::Lease)?;
        let ground = input.ground;
        let direction = input.direction;
        let radius = tuning.test_radius().map_err(E::Tuning)?;
        let height = tuning.max_jump_height().map_err(E::Tuning)?;
        let reach = tuning.absolute_max_jump_distance().map_err(E::Tuning)?;
        let doubled = radius * 2.0;
        let displacement = direction.map(|v| v * reach);
        let low = add(ground, up(height - radius));
        let high = add(ground, up(height + radius));
        let mut line = |start: V, end: V| {
            world
                .queries
                .try_add_line(
                    lines,
                    CollisionLineInput {
                        start,
                        end,
                        fatness: radius,
                    },
                )
                .map_err(E::Lease)
        };
        let _ = line(low, add(low, displacement))?;
        let lower = sub(low, up(doubled));
        let _ = line(lower, add(lower, displacement))?;
        let high_end = add(high, displacement);
        let _ = line(high, high_end)?;
        let _ = line(add(high, up(doubled)), add(high_end, up(doubled)))?;
        let floor_end = sub(add(ground, displacement), up(radius));
        let _ = line(sub(floor_end, displacement), floor_end)?;
        world
            .queries
            .try_end(lines, world.ground, world.dmos, numeric)
            .map_err(E::Line)?;
        let down = tuning.max_jump_down().map_err(E::Tuning)?;
        let base = high.zip(direction, |h, d| d.mul_add(input.distance, h));
        let position = add(base, up(radius + height));
        let back = direction.map(|v| v * (radius * -2.0));
        let back_position = add(position, back);
        let target = sub(position, up(down + height));
        self.trajectory_batch_one = Some(
            world
                .queries
                .try_acquire_trajectory_with_flags(world.dmos, LABEL, DEFAULT_FLAGS)
                .map_err(E::Lease)?,
        );
        let first = self.trajectory_batch_one.as_ref().expect("published");
        world
            .queries
            .try_begin_trajectory(first)
            .map_err(E::Lease)?;
        let _accepted = world
            .queries
            .try_add_ballistic_trajectory::<N::Error>(
                first,
                BallisticTrajectoryInput {
                    position,
                    velocity: sub(target, position),
                    gravity: V::known([0.0; 4]),
                    duration: 1.0,
                    radius,
                    maximum_error_start: 0.0,
                    maximum_error_end: 0.0,
                },
            )
            .map_err(E::Trajectory)?;
        world
            .queries
            .try_end_trajectory(first, world.ground, world.dmos, numeric)
            .map_err(E::Trajectory)?;
        self.trajectory_batch_two = Some(
            world
                .queries
                .try_acquire_trajectory_with_flags(world.dmos, LABEL, DEFAULT_FLAGS)
                .map_err(E::Lease)?,
        );
        let second = self.trajectory_batch_two.as_ref().expect("published");
        world
            .queries
            .try_begin_trajectory(second)
            .map_err(E::Lease)?;
        let _accepted = world
            .queries
            .try_add_ballistic_trajectory::<N::Error>(
                second,
                BallisticTrajectoryInput {
                    position: back_position,
                    velocity: sub(add(target, back), back_position),
                    gravity: V::known([0.0; 4]),
                    duration: 1.0,
                    radius,
                    maximum_error_start: 0.0,
                    maximum_error_end: 0.0,
                },
            )
            .map_err(E::Trajectory)?;
        world
            .queries
            .try_end_trajectory(second, world.ground, world.dmos, numeric)
            .map_err(E::Trajectory)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one native result pass: five lines, two trajectories and the ordered window clamps"
    )]
    pub(crate) fn calculate_results<N: ReckoningNumeric, T: JumpingTuning>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), JumpingError<N::Error, T::Error>> {
        use JumpingError as E;
        self.landing_position_valid = false;
        self.leap_of_faith = false;
        self.landing_position = [0.0; 4];
        self.result_valid_counter = 30;
        let lines = self.line_batch.as_ref().ok_or(E::MissingLine)?;
        queries.try_sync(lines).map_err(E::Lease)?;
        let ground = V::known(self.ground_position);
        let direction = V::known(self.jump_direction);
        let debug = |tuning: &mut T| -> Result<(), JumpingError<N::Error, T::Error>> {
            if tuning.debug_draw().map_err(E::Tuning)? {
                return Err(E::PendingDebugDraw);
            }
            Ok(())
        };
        let first = queries.try_get_result(lines, 0).map_err(E::Lease)?;
        let second = queries.try_get_result(lines, 1).map_err(E::Lease)?;
        let (mut near_hit, mut near) = (false, 0.0);
        if let Some(hit) = first {
            near_hit = true;
            near = numeric
                .dot_vector(direction, sub(hit.position, ground))
                .map_err(E::Numeric)?;
            debug(tuning)?;
        }
        let (mut low_hit, mut low) = (false, 0.0);
        if let Some(hit) = second {
            low_hit = true;
            low = numeric
                .dot_vector(direction, sub(hit.position, ground))
                .map_err(E::Numeric)?;
            debug(tuning)?;
        }
        let third = queries.try_get_result(lines, 2).map_err(E::Lease)?;
        let fourth = queries.try_get_result(lines, 3).map_err(E::Lease)?;
        let (mut wall_hit, mut wall) = (false, 0.0);
        if let Some(hit) = third {
            let margin = tuning.minimum_distance_from_wall().map_err(E::Tuning)?;
            wall_hit = true;
            wall = numeric
                .dot_vector(direction, sub(hit.position, ground))
                .map_err(E::Numeric)?
                - margin;
            debug(tuning)?;
        }
        if let Some(hit) = fourth {
            let margin = tuning.minimum_distance_from_wall().map_err(E::Tuning)?;
            let along = numeric
                .dot_vector(direction, sub(hit.position, ground))
                .map_err(E::Numeric)?;
            if wall_hit {
                let candidate = along - margin;
                wall = fsel(wall - candidate, candidate, wall);
            } else {
                wall_hit = true;
                wall = along - margin;
            }
            debug(tuning)?;
        }
        let (mut floor_hit, mut floor) = (false, 0.0);
        if let Some(hit) = queries.try_get_result(lines, 4).map_err(E::Lease)? {
            floor_hit = true;
            floor = numeric
                .dot_vector(direction, sub(hit.position, ground))
                .map_err(E::Numeric)?;
        }
        queries.try_release(lines).map_err(E::Lease)?;
        self.line_batch = None;
        let first = self
            .trajectory_batch_one
            .as_ref()
            .ok_or(E::MissingTrajectory)?;
        if self.trajectory_batch_two.is_none() {
            return Err(E::MissingTrajectory);
        }
        queries.try_sync_trajectory(first).map_err(E::Lease)?;
        let landing = queries
            .try_get_trajectory_result(first, 0)
            .map_err(E::Lease)?;
        queries.try_release_trajectory(first).map_err(E::Lease)?;
        self.trajectory_batch_one = None;
        let second = self
            .trajectory_batch_two
            .as_ref()
            .ok_or(E::MissingTrajectory)?;
        queries.try_sync_trajectory(second).map_err(E::Lease)?;
        let back_landing = queries
            .try_get_trajectory_result(second, 0)
            .map_err(E::Lease)?;
        queries.try_release_trajectory(second).map_err(E::Lease)?;
        self.trajectory_batch_two = None;
        let classify = |result: Option<TrajectoryResult>,
                        tuning: &mut T|
         -> Result<(f32, bool, bool), JumpingError<N::Error, T::Error>> {
            let Some(result) = result else {
                return Ok((0.0, false, true));
            };
            debug(tuning)?;
            let minimum = tuning.minimum_ground_normal_y().map_err(E::Tuning)?;
            // vcmpgtfp128. over splatted Y lanes; unordered is not greater.
            if result.normal.xyz[1] > minimum {
                Ok((result.position.xyz[1] - ground.xyz[1], false, false))
            } else {
                Ok((0.0, true, false))
            }
        };
        let (landing_height, landing_steep, landing_missing) = classify(landing, tuning)?;
        let (back_height, back_steep, back_missing) = classify(back_landing, tuning)?;
        let jump = self.jump_distance;
        let radius = tuning.test_radius().map_err(E::Tuning)?;
        let wall_margin = tuning.minimum_distance_from_wall().map_err(E::Tuning)?;
        let top_margin = tuning
            .minimum_distance_from_top_of_wall()
            .map_err(E::Tuning)?;
        let back_jump = (-radius).mul_add(2.0, jump);
        let mut front_low = jump - radius;
        let mut front_high = radius + jump;
        let mut back_low = back_jump - radius;
        let mut back_high = radius + back_jump;
        for (hit, along) in [(near_hit, near), (low_hit, low)] {
            if hit {
                let closest = along - wall_margin;
                let furthest = top_margin + along;
                front_low = push_lower(front_low, closest, furthest);
                front_high = pull_upper(front_high, closest, furthest);
                back_low = push_lower(back_low, closest, furthest);
                back_high = pull_upper(back_high, closest, furthest);
            }
        }
        if wall_hit {
            let limit = wall - wall_margin;
            front_high = fsel(front_high - limit, limit, front_high);
            back_high = fsel(back_high - limit, limit, back_high);
        }
        let drop = f32::from_bits(0xbdcc_cccd);
        if landing_height < drop && floor_hit {
            let closest = floor - top_margin;
            let furthest = wall_margin + floor;
            front_low = push_lower(front_low, closest, furthest);
            front_high = pull_upper(front_high, closest, furthest);
        }
        if back_height < drop && floor_hit {
            let closest = floor - top_margin;
            let furthest = wall_margin + floor;
            back_low = push_lower(back_low, closest, furthest);
            back_high = pull_upper(back_high, closest, furthest);
        }
        let mut distance = 0.0;
        let mut height = 0.0;
        let mut leap = false;
        let mut found = false;
        let mut back_allowed = true;
        if front_low < front_high && !landing_steep {
            height = landing_height;
            leap = landing_missing;
            let lower = fsel(front_low - jump, front_low, jump);
            distance = fsel(front_high - lower, lower, front_high);
            back_allowed = landing_missing && !back_missing;
            found = true;
        }
        if back_low < back_high && !back_steep && back_allowed {
            leap = back_missing;
            height = back_height;
            found = true;
            let lower = fsel(back_low - back_jump, back_low, back_jump);
            distance = fsel(back_high - lower, lower, back_high);
        }
        if !found {
            distance = 0.0;
            height = 0.0;
        }
        if tuning.force_bad_trajectory().map_err(E::Tuning)? {
            distance = jump;
            height = 0.0;
        }
        // 82dd5f64 stores the retained leap register last; the zero stores on
        // the not-found and forced-bad paths are overwritten by it.
        self.leap_of_faith = leap;
        let landing = direction.zip(ground, |d, g| d.mul_add(distance, g));
        self.landing_position = lanes(add(landing, up(height)));
        self.landing_position_valid = true;
        Ok(())
    }

    /// Tick82dd5f98: signed decrement and clamp, then a pending batch completes.
    pub(crate) fn tick<N: ReckoningNumeric, T: JumpingTuning>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), JumpingError<N::Error, T::Error>> {
        self.result_valid_counter = self.result_valid_counter.wrapping_sub(1).max(0);
        if self.line_batch.is_some() {
            self.calculate_results(queries, tuning, numeric)?;
        }
        Ok(())
    }

    /// ~OffboardJumpingTest82dd5fd0, reached from OffBoard.Exit82d86ce8.
    pub(crate) fn finish_pending<N: ReckoningNumeric, T: JumpingTuning>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), JumpingError<N::Error, T::Error>> {
        if self.line_batch.is_some() {
            self.calculate_results(queries, tuning, numeric)?;
        }
        Ok(())
    }

    /// Reset82dda070 from ResetSystems.
    pub(crate) fn reset<N: ReckoningNumeric, T: JumpingTuning>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), JumpingError<N::Error, T::Error>> {
        self.finish_pending(queries, tuning, numeric)?;
        self.result_valid_counter = 0;
        Ok(())
    }
}

#[cfg(test)]
#[path = "physical_player_offboard_jumping_tests.rs"]
mod tests;
