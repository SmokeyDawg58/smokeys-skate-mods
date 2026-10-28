//! Predictor `AddQuery`82e4b868 / `UpdateQuery`82e49ae0 / Update82e4ac68.
//! Child of the existing world predictor; submission does not publish readiness.
use super::PhysicalPredictionOwner;
use crate::{
    camera_prediction::CameraPrediction,
    collision_query_batches::trajectory::BallisticTrajectoryInput,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_footplant::RetainedTrajectory,
    physical_player_trajectory_owners::RetainedPredictionResult,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineQueries,
        trajectory::{SimulationWorldTrajectoryBatch, WorldTrajectoryError},
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skeleton_root_transforms::SkeletonRootVector as Vector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
#[path = "physical_player_prediction_future.rs"]
pub(crate) mod future;
use future::{PredictionDestination, PredictionFutureMut, PredictionTarget};

#[cfg(test)]
#[path = "physical_player_predictor_runtime_tests.rs"]
mod tests;

/// Identity of the existing selector slot, not another future/result owner.
pub(super) struct PendingPredictionQuery {
    destination: PredictionDestination,
    batch: Option<SimulationWorldTrajectoryBatch>,
}

#[derive(Debug)]
pub(crate) enum PredictionRuntimeError<N> {
    ForeignManager,
    Missing(&'static str),
    Poisoned,
    InputNaN,
    RadiusTooSmall,
    QueueFull,
    AlreadyReady,
    Overlay(DebugOverlayError),
    World(WorldTrajectoryError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for PredictionRuntimeError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "trajectory predictor: {self:?}")
    }
}

/// Source full stores, including known fourth lanes on reset and unknown lanes
/// on the input copies. A later capacity/admission failure retains these writes.
fn reset_future(future: &mut PredictionFutureMut<'_>, input: BallisticTrajectoryInput, draw: bool) {
    future.set_ready(false);
    future.reset_result(RetainedPredictionResult {
        position: Vector::known([0.0; 4]),
        normal: Vector::known([0.0, 1.0, 0.0, 0.0]),
        ground_normal: Vector::known([0.0, 1.0, 0.0, 0.0]),
        time: [-1.0; 4],
        frame: -1,
        surface: 0,
        trajectory: RetainedTrajectory {
            position: input.position,
            velocity: input.velocity,
            gravity: input.gravity,
            time: [input.duration; 4],
        },
        draw_true: draw,
        radius: input.radius,
        max_error_start: input.maximum_error_start,
        max_error_end: input.maximum_error_end,
    });
}

impl PhysicalPredictionOwner {
    /// Actual World batch acquisition and job submission. The future stays
    /// unready even when this host executes the collision work synchronously.
    #[expect(
        clippy::too_many_arguments,
        reason = "same retained predictor/future and actual world query collaborators"
    )]
    pub(crate) fn try_add_onboard_query<N: ReckoningNumeric>(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        slot: usize,
        input: BallisticTrajectoryInput,
        draw: bool,
        world: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), PredictionRuntimeError<N::Error>> {
        self.try_add_query(
            owner,
            PredictionTarget::Onboard(slot),
            input,
            draw,
            world,
            ground,
            dmos,
            numeric,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "actual predictor target and World collaborators"
    )]
    pub(crate) fn try_add_query<N: ReckoningNumeric>(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        target: PredictionTarget,
        input: BallisticTrajectoryInput,
        draw: bool,
        world: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), PredictionRuntimeError<N::Error>> {
        use PredictionRuntimeError as E;
        if owner.manager() != self.manager {
            return Err(E::ForeignManager);
        }
        self.try_add_owned_query(
            PredictionDestination::Player {
                owner: Arc::downgrade(owner),
                target,
            },
            input,
            draw,
            world,
            ground,
            dmos,
            numeric,
        )
    }

    /// `TrajectoryPredictor::StartQuery82e32ac0`: the camera's retained future
    /// is submitted to the same World predictor as onboard and wipeout queries.
    /// Airborne queries use the predictor gravity; grounded look-around queries
    /// use an all-zero gravity vector. Both error ratios are exactly 1.0.
    #[expect(
        clippy::too_many_arguments,
        reason = "native camera submission and existing World query collaborators"
    )]
    pub(crate) fn try_add_camera_query<N: ReckoningNumeric>(
        &self,
        camera: &Arc<Mutex<CameraPrediction>>,
        position: Vector,
        velocity: Vector,
        radius: f32,
        lookahead: f32,
        in_air: bool,
        world: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), PredictionRuntimeError<N::Error>> {
        {
            let camera_owner = camera
                .lock()
                .map_err(|_| PredictionRuntimeError::Poisoned)?;
            if !std::ptr::eq(camera_owner.world_predictor.as_ref(), self) {
                return Err(PredictionRuntimeError::ForeignManager);
            }
        }
        let input = BallisticTrajectoryInput {
            position,
            velocity,
            gravity: Vector::known(if in_air { self.gravity } else { [0.0; 4] }),
            radius,
            duration: lookahead,
            maximum_error_start: 1.0,
            maximum_error_end: 1.0,
        };
        self.try_add_owned_query(
            PredictionDestination::Camera(Arc::downgrade(camera)),
            input,
            false,
            world,
            ground,
            dmos,
            numeric,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one source AddQuery path shared by existing future owners"
    )]
    fn try_add_owned_query<N: ReckoningNumeric>(
        &self,
        destination: PredictionDestination,
        input: BallisticTrajectoryInput,
        draw: bool,
        world: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), PredictionRuntimeError<N::Error>> {
        use PredictionRuntimeError as E;
        if !world.belongs_to(self.manager) {
            return Err(E::ForeignManager);
        }
        // Native self-equality asserts reject NaN XYZ; infinity and unknown W
        // are not silently replaced or subjected to additional finite checks.
        if input
            .position
            .xyz
            .iter()
            .chain(&input.velocity.xyz)
            .any(|v| v.is_nan())
        {
            return Err(E::InputNaN);
        }
        if input.radius < f32::from_bits(0x3c23_d70a) {
            return Err(E::RadiusTooSmall);
        }
        destination.with_future(|mut future| {
            reset_future(&mut future, input, draw);
            Ok(())
        })?;
        let index = {
            let mut queue = self.queue.lock().map_err(|_| E::Poisoned)?;
            if queue.length == queue.slots.len() {
                return Err(E::QueueFull);
            }
            let index = queue.length;
            queue.slots[index] = Some(PendingPredictionQuery {
                destination,
                batch: None,
            });
            queue.length += 1;
            index
        };
        // Acquisition failure leaves the actual queued request with null batch,
        // matching the source append-before-acquire prefix.
        let batch = world
            .try_acquire_trajectory(dmos, "Predictor::AddQuery")
            .map_err(|e| E::World(e.into()))?;
        let mut queue = self.queue.lock().map_err(|_| E::Poisoned)?;
        let request = queue.slots[index]
            .as_mut()
            .ok_or(E::Missing("queued request"))?;
        request.batch = Some(batch);
        let batch = request.batch.as_ref().expect("just stored batch");
        world
            .try_begin_trajectory(batch)
            .map_err(|e| E::World(e.into()))?;
        // Source calls AddTest without branching on its return value.
        let _ = world
            .try_add_ballistic_trajectory(batch, input)
            .map_err(E::World)?;
        world
            .try_end_trajectory(batch, ground, dmos, numeric)
            .map_err(E::World)
    }

    /// Call at actual Predictor.Update schedule points, not immediately after
    /// Launch. Earlier result copies/releases survive a later query failure.
    pub(crate) fn try_update_queries<N>(
        &self,
        world: &mut SimulationWorldLineQueries,
        overlay: &DebugOverlaySlot,
    ) -> Result<(), PredictionRuntimeError<N>> {
        use PredictionRuntimeError as E;
        if !world.belongs_to(self.manager) {
            return Err(E::ForeignManager);
        }
        let mut queue = self.queue.lock().map_err(|_| E::Poisoned)?;
        if queue.length == 0 {
            return Ok(());
        }
        overlay.require().map_err(E::Overlay)?;
        for index in 0..queue.length {
            let request = queue.slots[index]
                .as_mut()
                .ok_or(E::Missing("queued request"))?;
            match &request.destination {
                PredictionDestination::Player { owner, .. } => {
                    let owner = owner.upgrade().ok_or(E::Missing("future owner"))?;
                    if owner.manager() != self.manager {
                        return Err(E::ForeignManager);
                    }
                }
                PredictionDestination::Camera(owner) => {
                    let owner = owner.upgrade().ok_or(E::Missing("camera future owner"))?;
                    if !std::ptr::eq(
                        owner
                            .lock()
                            .map_err(|_| E::Poisoned)?
                            .world_predictor
                            .as_ref(),
                        self,
                    ) {
                        return Err(E::ForeignManager);
                    }
                }
            }
            request.destination.with_future(|mut future| {
                if future.ready() {
                    return Err(E::AlreadyReady);
                }
                let batch = request.batch.as_ref().ok_or(E::Missing("queued batch"))?;
                world
                    .try_sync_trajectory(batch)
                    .map_err(|e| E::World(e.into()))?;
                if let Some(collision) = world
                    .try_get_trajectory_result(batch, 0)
                    .map_err(|e| E::World(e.into()))?
                {
                    future.copy_collision(collision);
                }
                world
                    .try_release_trajectory(batch)
                    .map_err(|e| E::World(e.into()))?;
                request.batch = None;
                future.set_ready(true);
                Ok(())
            })?;
        }
        for slot in &mut queue.slots {
            *slot = None;
        }
        queue.length = 0;
        Ok(())
    }
}
