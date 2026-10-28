//! OffboardLocationTest82dd48e8/4cc8/1fc0 on the existing retained world owner.
use super::OffboardLocation;
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    reckoning_numeric::{ReckoningNumeric, cross_typed},
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineError, SimulationWorldLineLeaseError, SimulationWorldLineQueries,
        trajectory::WorldTrajectoryError,
    },
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};
pub(crate) trait LocationTuning {
    type Error;
    fn line_length(&mut self) -> Result<f32, Self::Error>;
    fn start_height(&mut self) -> Result<f32, Self::Error>;
    fn line_count(&mut self) -> Result<i32, Self::Error>;
    fn test_radius(&mut self) -> Result<f32, Self::Error>;
    fn trajectory_start_height(&mut self) -> Result<f32, Self::Error>;
    fn trajectory_end_length(&mut self) -> Result<f32, Self::Error>;
    fn trajectory_end_height(&mut self) -> Result<f32, Self::Error>;
    fn minimum_y_normal(&mut self) -> Result<f32, Self::Error>;
    fn minimum_normal_dot(&mut self) -> Result<f32, Self::Error>;
    fn turn_speed(&mut self) -> Result<f32, Self::Error>;
    fn step_normal_y_threshold(&mut self) -> Result<f32, Self::Error>;
}
pub(crate) struct LocationWorld<'a> {
    /// World+10 constructor snapshot; flagged World query overloads override to7.
    pub(crate) visual_island_enabled: bool,
    pub(crate) queries: &'a mut SimulationWorldLineQueries,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) dmos: &'a SimulationDmoAssemblyPool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocationInput {
    pub(crate) ground: V,
    pub(crate) look: V,
    pub(crate) jump_velocity: V,
    pub(crate) jumping: bool,
    pub(crate) world_only: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocationResult {
    pub(crate) landing_position: V,
    pub(crate) turn: f32,
    pub(crate) landing_position_valid: bool,
    pub(crate) blocked: bool,
}
impl LocationResult {
    pub(crate) fn constructed() -> Self {
        Self {
            landing_position: V::known([0.0; 4]),
            turn: 0.0,
            landing_position_valid: false,
            blocked: false,
        }
    }
}
#[derive(Debug)]
pub(crate) enum LocationError<N, T> {
    ActiveLine,
    ActiveTrajectory,
    NotReady,
    MissingLine,
    MissingTrajectory,
    Lease(SimulationWorldLineLeaseError),
    Line(SimulationWorldLineError<N>),
    Trajectory(WorldTrajectoryError<N>),
    Numeric(N),
    Tuning(T),
}

impl OffboardLocation {
    pub(crate) fn start_query<N: ReckoningNumeric + AssemblyGroundNumeric, T: LocationTuning>(
        &mut self,
        input: LocationInput,
        world: &mut LocationWorld<'_>,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), LocationError<N::Error, T::Error>> {
        use LocationError as E;
        if self.line_batch.is_some() {
            return Err(E::ActiveLine);
        }
        if self.trajectory_batch.is_some() {
            return Err(E::ActiveTrajectory);
        }
        self.ground_position = input.ground;
        self.look_direction = input.look;
        self.jump_velocity = input.jump_velocity;
        self.jumping = input.jumping;
        self.collide_with_world_only = input.world_only;
        // World827ab918/827ab970 override both families using the same snapshot.
        let flags = if input.world_only && !world.visual_island_enabled {
            1
        } else {
            7
        };
        self.line_batch = Some(
            world
                .queries
                .try_acquire_line_batch(world.dmos, "OffboardLocationTest::StartQuery", flags)
                .map_err(E::Lease)?,
        );
        let lines = self.line_batch.as_ref().expect("published");
        world.queries.try_begin(lines).map_err(E::Lease)?;
        let length = tuning.line_length().map_err(E::Tuning)?;
        let height = tuning.start_height().map_err(E::Tuning)?;
        let radius = tuning.test_radius().map_err(E::Tuning)?;
        let count = tuning.line_count().map_err(E::Tuning)?;
        let spacing = radius * 2.0;
        let displacement = input.look.map(|v| v * length);
        let base = input
            .ground
            .zip(V::known([0.0, height, 0.0, 0.0]), |a, b| a + b);
        if count > 0 {
            let increment = V::known([0.0, spacing, 0.0, 0.0]);
            let mut index = 0i32;
            loop {
                let start = increment.zip(base, |a, b| a.mul_add(index as f32, b));
                let end = start.zip(displacement, |a, b| a + b);
                let radius = tuning.test_radius().map_err(E::Tuning)?;
                let _accepted = world
                    .queries
                    .try_add_line(
                        lines,
                        CollisionLineInput {
                            start,
                            end,
                            fatness: radius,
                        },
                    )
                    .map_err(E::Lease)?;
                index = index.wrapping_add(1);
                if index >= tuning.line_count().map_err(E::Tuning)? {
                    break;
                }
            }
        }
        world
            .queries
            .try_end(lines, world.ground, world.dmos, numeric)
            .map_err(E::Line)?;
        let height = tuning.trajectory_start_height().map_err(E::Tuning)?;
        let position = input
            .ground
            .zip(V::known([0.0, height, 0.0, 0.0]), |a, b| a + b);
        let length = tuning.trajectory_end_length().map_err(E::Tuning)?;
        let height = tuning.trajectory_end_height().map_err(E::Tuning)?;
        let forward = input.look.zip(input.ground, |v, p| v.mul_add(length, p));
        let end = forward.zip(V::known([0.0, height, 0.0, 0.0]), |a, b| a + b);
        let velocity = end.zip(position, |a, b| a - b);
        self.trajectory_batch = Some(
            world
                .queries
                .try_acquire_trajectory_with_flags(
                    world.dmos,
                    "OffboardLocationTest::StartQuery",
                    flags,
                )
                .map_err(E::Lease)?,
        );
        let trajectory = self.trajectory_batch.as_ref().expect("published");
        world
            .queries
            .try_begin_trajectory(trajectory)
            .map_err(E::Lease)?;
        let radius = tuning.test_radius().map_err(E::Tuning)?;
        let _accepted = world
            .queries
            .try_add_ballistic_trajectory::<N::Error>(
                trajectory,
                BallisticTrajectoryInput {
                    position,
                    velocity,
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
            .try_end_trajectory(trajectory, world.ground, world.dmos, numeric)
            .map_err(E::Trajectory)?;
        self.sync_allowed = false;
        Ok(())
    }
    pub(crate) fn get_results<N: ReckoningNumeric, T: LocationTuning>(
        &mut self,
        result: &mut LocationResult,
        queries: &mut SimulationWorldLineQueries,
        tuning: &mut T,
        numeric: &mut N,
    ) -> Result<(), LocationError<N::Error, T::Error>> {
        use LocationError as E;
        if !self.sync_allowed {
            return Err(E::NotReady);
        }
        let lines = self.line_batch.as_ref().ok_or(E::MissingLine)?;
        queries.try_sync(lines).map_err(E::Lease)?;
        result.blocked = false;
        let mut index = 0i32;
        if tuning.line_count().map_err(E::Tuning)? > 0 {
            loop {
                if let Some(hit) = queries
                    .try_get_result(lines, index as u32)
                    .map_err(E::Lease)?
                {
                    process_line_hit(result, hit.normal, self.look_direction, tuning, numeric)?;
                }
                index = index.wrapping_add(1);
                if index >= tuning.line_count().map_err(E::Tuning)? {
                    break;
                }
            }
        }
        queries.try_release(lines).map_err(E::Lease)?;
        self.line_batch = None;
        let trajectory = self.trajectory_batch.as_ref().ok_or(E::MissingTrajectory)?;
        queries.try_sync_trajectory(trajectory).map_err(E::Lease)?;
        if let Some(hit) = queries
            .try_get_trajectory_result(trajectory, 0)
            .map_err(E::Lease)?
        {
            let threshold = tuning.step_normal_y_threshold().map_err(E::Tuning)?;
            // Native TrajectoryResult+20 is groundNormal, not the contact normal.
            if threshold > hit.ground_normal.xyz[1] {
                result.blocked = true;
            }
        }
        queries
            .try_release_trajectory(trajectory)
            .map_err(E::Lease)?;
        self.trajectory_batch = None;
        self.sync_allowed = false;
        Ok(())
    }
    pub(crate) fn clear_data(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.sync_allowed = false;
        if let Some(batch) = &self.line_batch {
            queries.try_sync(batch)?;
            queries.try_release(batch)?;
            self.line_batch = None;
        }
        if let Some(batch) = &self.trajectory_batch {
            queries.try_sync_trajectory(batch)?;
            queries.try_release_trajectory(batch)?;
            self.trajectory_batch = None;
        }
        Ok(())
    }
}
pub(crate) fn process_line_hit<N: ReckoningNumeric, T: LocationTuning>(
    result: &mut LocationResult,
    mut normal: V,
    look: V,
    tuning: &mut T,
    numeric: &mut N,
) -> Result<(), LocationError<N::Error, T::Error>> {
    use LocationError as E;
    let minimum = tuning.minimum_y_normal().map_err(E::Tuning)?;
    if minimum > normal.xyz[1].abs() {
        normal.xyz[1] = 0.0;
        let squared = numeric.dot_vector(normal, normal).map_err(E::Numeric)?;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let minimum_dot = tuning.minimum_normal_dot().map_err(E::Tuning)?;
        result.blocked = true;
        normal = crate::skateboard_controller::righting::normalize_finish(normal, squared, seed);
        let dot = numeric.dot_vector(normal, look).map_err(E::Numeric)?;
        // BO4 !GT and !LT reject unordered; both acceptance gates are strict.
        if dot > minimum_dot && dot < 0.0 {
            let orientation = numeric
                .dot_vector(V::known([0.0, 1.0, 0.0, 0.0]), cross_typed(normal, look))
                .map_err(E::Numeric)?;
            let speed = tuning.turn_speed().map_err(E::Tuning)?;
            result.turn = if orientation > 0.0 {
                f32::from_bits(speed.to_bits() ^ 0x80000000)
            } else {
                speed
            };
        }
    } else {
        result.blocked = true;
    }
    Ok(())
}

#[cfg(test)]
#[path = "physical_player_offboard_location_tests.rs"]
mod tests;
