//! LandingOnDeckManager runtime on the constructor-retained owner.
//! Query leases and collision results are the real world trajectory objects.
use super::LandingOnDeck;
use crate::{
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::remaining_states::{StateConstructorError, read_floats},
    physical_player_footplant::RetainedTrajectory,
    physical_player_landing_on_deck_math::smallest_starting_speed_time,
    physical_player_phys_out::{PhysOutAccessError, PhysicalPlayerPhysOut},
    physical_player_trajectory_owners::offboard_runtime::{
        greatest_plane_time, position_at, velocity_at,
    },
    reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate},
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineLeaseError, SimulationWorldLineQueries, trajectory::WorldTrajectoryError,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    simulation_drive_jacobian::refine_drive_reciprocal,
    skeleton_root_transforms::SkeletonRootVector as V,
    static_ground_pool::StaticGroundPool,
};

/// Math::VECTOR3_GRAVITY, initializer82f76360, including its explicit W=0.
const GRAVITY: V = V::known([0.0, f32::from_bits(0xc11c_cccd), 0.0, 0.0]);

#[cfg(test)]
#[path = "physical_player_landing_on_deck_tests.rs"]
mod tests;

#[derive(Debug)]
pub(crate) enum LandingError<N> {
    Numeric(N),
    Adjustment(
        crate::physical_player_trajectory_owners::selector_update::SelectorError<
            std::convert::Infallible,
            N,
        >,
    ),
    Cvar(InputCvarError),
    Attribute(StateConstructorError),
    Overlay(DebugOverlayError),
    Lease(SimulationWorldLineLeaseError),
    Query(WorldTrajectoryError<N>),
    Output(PhysOutAccessError),
    Poisoned,
    Missing(&'static str),
    ForeignOwner,
    QueryAlreadyActive,
    TrajectoryInvalid,
}
impl<N: std::fmt::Debug> std::fmt::Display for LandingError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LandingOnDeck: {self:?}")
    }
}

pub(crate) struct LandingWorld<'a> {
    pub(crate) queries: &'a mut SimulationWorldLineQueries,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) dmos: &'a SimulationDmoAssemblyPool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LandingResult {
    pub(crate) position: V,
    pub(crate) velocity: V,
    pub(crate) can_land: bool,
    pub(crate) time_to_land: f32,
}

impl LandingOnDeck {
    /// IsBoardUpright82dcee00: runtime Vault/default read precedes deck Y load.
    fn is_board_upright<N>(&self) -> Result<bool, LandingError<N>> {
        let minimum = read_floats::<1>(&self.attributes, 0x6793_fb9b_1167_5beb)
            .map_err(LandingError::Attribute)?[0];
        let processed = self
            .skeleton
            .processed
            .lock()
            .map_err(|_| LandingError::Poisoned)?;
        let deck = processed
            .deck_transform()
            .ok_or(LandingError::Missing("Processed deck+40"))?;
        Ok(deck.columns()[1].xyz[1] > minimum)
    }

    fn board_velocity<N>(&self) -> Result<V, LandingError<N>> {
        self.skeleton
            .processed
            .lock()
            .map_err(|_| LandingError::Poisoned)?
            .prior_output()
            .velocity()
            .map(|v| v.value)
            .ok_or(LandingError::Missing("Processed velocity+190"))
    }

    fn flags<N>(&self) -> Result<u32, LandingError<N>> {
        self.skeleton
            .processed
            .lock()
            .map_err(|_| LandingError::Poisoned)?
            .physics_state_fields()
            .flags_8d0
            .ok_or(LandingError::Missing("Processed flags+8d0"))
    }

    /// ConsiderSkateboard82dce6d8. A failed plane query only clears can_land.
    pub(crate) fn consider_skateboard<N: ReckoningNumeric>(
        &mut self,
        com_position: V,
        com_velocity: V,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        use LandingError as E;
        let (wheels, mut board_velocity) = {
            let p = self.skeleton.processed.lock().map_err(|_| E::Poisoned)?;
            (
                p.physics_state_fields()
                    .num_wheels_in_contact
                    .ok_or(E::Missing("Processed wheel count"))?,
                p.prior_output()
                    .velocity()
                    .ok_or(E::Missing("Processed velocity+190"))?
                    .value,
            )
        };
        if wheels == 0 {
            board_velocity.xyz[1] = 0.0;
        }
        let relative = RetainedTrajectory {
            position: com_position,
            velocity: com_velocity.zip(board_velocity, |a, b| a - b),
            gravity: if wheels == 0 {
                GRAVITY.map(|v| v * 0.5)
            } else {
                GRAVITY
            },
            time: [-1.0; 4],
        };
        let offset =
            read_floats::<1>(&self.attributes, 0xa2cb_5a7b_d91d_fbd6).map_err(E::Attribute)?[0];
        let (up, board_position) = {
            let p = self.skeleton.processed.lock().map_err(|_| E::Poisoned)?;
            let up = p
                .prior_output()
                .reckoning_up()
                .ok_or(E::Missing("Processed up+220"))?
                .value;
            let position = p
                .deck_transform()
                .ok_or(E::Missing("Processed deck+40"))?
                .columns()[3];
            (up, position)
        };
        let plane = up.zip(board_position, |n, p| n.mul_add(offset, p));
        let mut time = [0.0; 4];
        if !greatest_plane_time(&relative, plane, up, &mut time, numeric).map_err(E::Numeric)?
            || !(time[0] > 0.0)
        {
            self.can_land = false;
            return Ok(());
        }
        let time = time[0];
        let distance = com_position.zip(plane, |p, q| p - q);
        let max = cvars.try_landing_max_time_adjust().map_err(E::Cvar)?;
        let min = cvars.try_landing_min_time_adjust().map_err(E::Cvar)?;
        let predicted = position_at(&relative, time);
        let error = plane.zip(predicted, |a, b| a - b);
        let max_time = time * max;
        let min_time = time * min;
        let optimal = smallest_starting_speed_time(distance, numeric).map_err(E::Numeric)?;
        if cvars.try_landing_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        // Preserve the native position store before the fallible reciprocal seed.
        self.ideal_trajectory.position = com_position;
        let ideal_time = max_time.min(min_time.max(optimal));
        let delta = plane.zip(com_position, |a, b| a - b);
        let inverse = refine_drive_reciprocal(
            ideal_time,
            numeric
                .reciprocal_estimate(ideal_time)
                .map_err(E::Numeric)?,
        );
        let gravity_term = GRAVITY.map(|g| (g * 0.5) * ideal_time);
        self.ideal_trajectory.velocity = delta.map(|v| inverse * v).zip(gravity_term, |a, b| a - b);
        self.ideal_trajectory.gravity = GRAVITY;
        self.ideal_trajectory.time = [-1.0; 4];
        let raw_board_velocity = self.board_velocity()?;
        self.ideal_trajectory.velocity = self
            .ideal_trajectory
            .velocity
            .zip(raw_board_velocity, |a, b| a + b);
        self.landing_error = error;
        self.landing_error.xyz[1] = 0.0;
        self.time_to_land = time;
        self.ideal_time_to_land = ideal_time;
        Ok(())
    }

    /// InitAssistedTrajectory82dce480; it does not reset trajectory_time.
    pub(crate) fn init_assisted_trajectory<N: ReckoningNumeric>(
        &mut self,
        com_velocity: V,
        cvars: &InputCvarReferences,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<f32, LandingError<N::Error>> {
        use LandingError as E;
        let delta = com_velocity.zip(self.ideal_trajectory.velocity, |a, b| a - b);
        self.trajectory.position = self.ideal_trajectory.position;
        self.trajectory.velocity = self.ideal_trajectory.velocity;
        let squared = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
        self.trajectory.gravity = self.ideal_trajectory.gravity;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        self.trajectory.time = self.ideal_trajectory.time;
        self.trajectory_valid = true;
        self.time_to_land = self.ideal_time_to_land;
        let error = length_from_squared_and_estimate(squared, seed);
        if cvars.try_landing_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        Ok(error)
    }

    /// StartLandOnBoardQuery82dd0db8; do not clear can_land at a skipped gate.
    #[expect(
        clippy::too_many_arguments,
        reason = "source data and actual world collaborators"
    )]
    pub(crate) fn start_land_on_board_query<N: ReckoningNumeric>(
        &mut self,
        position: V,
        velocity: V,
        maximum_error: f32,
        cvars: &InputCvarReferences,
        world: &mut LandingWorld<'_>,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        use LandingError as E;
        self.tested_trajectory = false;
        if (self.landing_hippy_jump || self.flags()? & 0x40000 == 0) && self.is_board_upright()? {
            self.consider_skateboard(position, velocity, cvars, numeric, overlay)?;
            if self.time_to_land > cvars.try_landing_min_jump_time().map_err(E::Cvar)? {
                self.can_land = self.init_assisted_trajectory(velocity, cvars, numeric, overlay)?
                    < maximum_error;
            }
            if self.can_land && !self.tested_trajectory {
                self.tested_trajectory = true;
                self.start_trajectory_test(
                    position,
                    self.ideal_time_to_land,
                    cvars,
                    world,
                    numeric,
                    overlay,
                )?;
            }
        }
        Ok(())
    }

    /// StartTrajectoryTest82dcee98. Batch identity survives all later failures.
    pub(crate) fn start_trajectory_test<N: ReckoningNumeric>(
        &mut self,
        position: V,
        duration: f32,
        cvars: &InputCvarReferences,
        world: &mut LandingWorld<'_>,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        use LandingError as E;
        let (board_velocity, board_position) = {
            let p = self.skeleton.processed.lock().map_err(|_| E::Poisoned)?;
            (
                p.prior_output()
                    .velocity()
                    .ok_or(E::Missing("Processed velocity+190"))?
                    .value,
                p.deck_transform()
                    .ok_or(E::Missing("Processed deck+40"))?
                    .columns()[3],
            )
        };
        self.something_in_the_way = true;
        let seed = numeric.reciprocal_estimate(duration).map_err(E::Numeric)?;
        let offset = cvars.try_landing_trajectory_offset_y().map_err(E::Cvar)?;
        let radius = cvars.try_landing_trajectory_radius().map_err(E::Cvar)?;
        let target = board_velocity
            .zip(board_position, |v, p| v.mul_add(duration, p))
            .zip(V::known([0.0, offset + radius, 0.0, 0.0]), |a, b| a + b);
        let inverse = refine_drive_reciprocal(duration, seed);
        let delta = target.zip(position, |a, b| a - b);
        let velocity = delta
            .map(|v| inverse * v)
            .zip(GRAVITY.map(|g| (g * 0.5) * duration), |a, b| a - b);
        if cvars.try_landing_debug_trajectory().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        if self.trajectory_batch.is_some() {
            return Err(E::QueryAlreadyActive);
        }
        // Retail vector text calls825187c0 are empty; this assertion is not gated.
        overlay.require().map_err(E::Overlay)?;
        if !world.queries.belongs_to(self.world.manager) {
            return Err(E::ForeignOwner);
        }
        self.trajectory_batch = Some(
            world
                .queries
                .try_acquire_trajectory(world.dmos, "LandingOnDeckManager::StartTrajectoryTest")
                .map_err(E::Lease)?,
        );
        let batch = self
            .trajectory_batch
            .as_ref()
            .expect("published acquisition");
        world
            .queries
            .try_begin_trajectory(batch)
            .map_err(E::Lease)?;
        let radius = cvars.try_landing_trajectory_radius().map_err(E::Cvar)?;
        let error = cvars.try_landing_trajectory_error().map_err(E::Cvar)?;
        let _accepted = world
            .queries
            .try_add_ballistic_trajectory(
                batch,
                BallisticTrajectoryInput {
                    position,
                    velocity,
                    gravity: GRAVITY,
                    radius,
                    duration,
                    maximum_error_start: error,
                    maximum_error_end: error,
                },
            )
            .map_err(E::Query)?;
        world
            .queries
            .try_end_trajectory(batch, world.ground, world.dmos, numeric)
            .map_err(E::Query)?;
        // Fresh offset load82dcf19c..1b4; Hex-Rays misses it due scratch reuse.
        self.test_trajectory_end_y =
            target.xyz[1] + cvars.try_landing_trajectory_offset_y().map_err(E::Cvar)?;
        Ok(())
    }

    /// SyncTrajectoryTest82dd1250; publish obstruction before Release, clear after.
    pub(crate) fn sync_trajectory_test<N>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N>> {
        use LandingError as E;
        let batch = self
            .trajectory_batch
            .as_ref()
            .ok_or(E::Missing("trajectory batch"))?;
        if !queries.belongs_to(self.world.manager) {
            return Err(E::ForeignOwner);
        }
        queries.try_sync_trajectory(batch).map_err(E::Lease)?;
        let result = queries
            .try_get_trajectory_result(batch, 0)
            .map_err(E::Lease)?;
        let draw = cvars.try_landing_debug().map_err(E::Cvar)?;
        if draw && result.is_some() {
            overlay.require().map_err(E::Overlay)?;
        }
        let blocked =
            result.is_some_and(|result| result.position.xyz[1] > self.test_trajectory_end_y);
        if !blocked && cvars.try_landing_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        self.something_in_the_way = blocked;
        queries.try_release_trajectory(batch).map_err(E::Lease)?;
        self.trajectory_batch = None;
        Ok(())
    }

    /// UpdatePostPhysics82dd1bf8; no query is an intentional empty branch.
    pub(crate) fn update_post_physics<N>(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N>> {
        if self.trajectory_batch.is_some() {
            self.sync_trajectory_test(queries, cvars, overlay)?;
        }
        Ok(())
    }

    /// FillPhysOut82dced30; debug assertion precedes the actual output write latch.
    pub(crate) fn fill_phys_out<N>(
        &self,
        output: &mut PhysicalPlayerPhysOut,
        cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N>> {
        if cvars.try_landing_debug().map_err(LandingError::Cvar)? {
            overlay.require().map_err(LandingError::Overlay)?;
        }
        output
            .try_publish_can_land_on_skateboard(self.can_land && !self.something_in_the_way)
            .map_err(LandingError::Output)
    }

    /// Update82dd0ea8. Mandatory for the LandingOnDeck state, separate from Air502.
    pub(crate) fn update<N: ReckoningNumeric>(
        &mut self,
        result: &mut LandingResult,
        cvars: &InputCvarReferences,
        world: &mut LandingWorld<'_>,
        numeric: &mut N,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), LandingError<N::Error>> {
        use LandingError as E;
        if !self.trajectory_valid {
            return Err(E::TrajectoryInvalid);
        }
        self.trajectory_time += f32::from_bits(0x3c88_8889);
        let position = position_at(&self.trajectory, self.trajectory_time);
        let velocity = velocity_at(&self.trajectory, self.trajectory_time);
        self.can_land = true;
        self.consider_skateboard(position, velocity, cvars, numeric, overlay)?;
        if !self.tested_trajectory && self.can_land {
            self.tested_trajectory = true;
            self.start_trajectory_test(
                position,
                self.time_to_land,
                cvars,
                world,
                numeric,
                overlay,
            )?;
        }
        let ai = {
            let p = self.skeleton.processed.lock().map_err(|_| E::Poisoned)?;
            let (flags, known) = p.physics_state_fields().ai_flags;
            if known & 0x8000_0000 == 0 {
                return Err(E::Missing("Processed AI sign flag"));
            }
            flags & 0x8000_0000 != 0
        };
        let squared = numeric
            .dot_vector(self.landing_error, self.landing_error)
            .map_err(E::Numeric)?;
        let maximum = cvars.try_landing_max_error(ai).map_err(E::Cvar)?;
        let seed = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        self.can_land = maximum > length_from_squared_and_estimate(squared, seed);
        self.can_land = self.can_land
            && (self.landing_hippy_jump || self.flags()? & 0x40000 == 0)
            && self.is_board_upright()?;
        if cvars.try_landing_debug().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
            let _upright = self.is_board_upright::<N::Error>()?;
        }
        result.can_land = self.can_land && !self.something_in_the_way;
        result.velocity = velocity_at(&self.trajectory, self.time_to_land + self.trajectory_time);
        result.position = position;
        result.time_to_land = self.time_to_land;
        Ok(())
    }
}

#[path = "physical_player_landing_on_deck_followon.rs"]
mod followon;
