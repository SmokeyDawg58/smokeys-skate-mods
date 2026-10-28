//! World/Island StepSelector acquisition827ab998/827b8ad0 on actual owners.
use super::{
    AcquiredChild, CollisionQueryBatchFamily, ConstructorCollection, SimulationDmoAssemblyPool,
    SimulationWorldLineBatch, SimulationWorldLineChild, SimulationWorldLineLeaseError,
    SimulationWorldLineQueries, StaticGroundPool,
};
use crate::{
    actor_animation_construction::AnimationStepInput,
    collision_query_batches::step::{StepBatchError, StepResult, consider_child},
    physics::{TrajectoryCollisionWorld, TrajectoryGroundTriangle, TrajectorySweepHit},
    reckoning_numeric::ReckoningNumeric,
    simulation_dmo_assembly_part_session::dynamic_pool::DmoMeshGeometry,
    walking_step_query::{
        StepGround, StepQueryError, StepQueryTuning, StepTrajectoryResult, run_step_test,
    },
};
use skate2_formats::collision::CollisionError;
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

pub(crate) struct SimulationWorldStepBatch {
    lease: SimulationWorldLineBatch,
    aggregate: RefCell<StepResult>,
}
#[derive(Debug)]
pub(crate) enum WorldStepError<N, T> {
    Lease(SimulationWorldLineLeaseError),
    Add {
        child: SimulationWorldLineChild,
        source: StepBatchError<std::convert::Infallible>,
    },
    Job {
        child: SimulationWorldLineChild,
        source: StepBatchError<StepJobError<N, T>>,
    },
}
impl<N, T> From<SimulationWorldLineLeaseError> for WorldStepError<N, T> {
    fn from(error: SimulationWorldLineLeaseError) -> Self {
        Self::Lease(error)
    }
}
#[derive(Debug)]
pub(crate) enum StepJobError<N, T> {
    UnpublishedMembership { count: usize },
    Worker(StepQueryError<CollisionError, N, T>),
}
struct CheckedEmptyGeometry;
impl TrajectoryCollisionWorld for CheckedEmptyGeometry {
    type Error = CollisionError;
    fn sweep_sphere_segment(
        &mut self,
        _: [f32; 3],
        _: [f32; 3],
        _: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
        Ok(None)
    }
    fn triangles_overlapping_aabb(
        &mut self,
        _: [f32; 3],
        _: [f32; 3],
        _: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
        Ok(Vec::new())
    }
}
fn run_empty<N: ReckoningNumeric, T: StepQueryTuning>(
    collection: &ConstructorCollection,
    membership: usize,
    inputs: &[AnimationStepInput],
    result: &mut StepResult,
    numeric: &mut N,
    tuning: &mut T,
) -> Result<(), StepJobError<N::Error, T::Error>> {
    if membership != 0 || !collection.slots.is_empty() {
        return Err(StepJobError::UnpublishedMembership {
            count: membership.max(collection.slots.len()),
        });
    }
    // Existing constructor-owned empty collections execute the full worker,
    // including its numerical operations and source-timed cvar reads.
    run_step_test(inputs, result, &mut CheckedEmptyGeometry, numeric, tuning)
        .map_err(StepJobError::Worker)
}
impl SimulationWorldLineQueries {
    /// Caller supplies World flags (1 for world-only, otherwise7). This selected
    /// World has no visual island; the existing owner exposes Island flags1/3.
    pub(crate) fn try_acquire_step(
        &mut self,
        dmos: &SimulationDmoAssemblyPool,
        label: &str,
        flags: u32,
    ) -> Result<SimulationWorldStepBatch, SimulationWorldLineLeaseError> {
        self.require_dmo_owner(dmos)?;
        if flags == 0 {
            return Err(SimulationWorldLineLeaseError::ZeroFlags);
        }
        let mut children = Vec::with_capacity(3);
        for (child, bit) in [
            (SimulationWorldLineChild::Ground, 1),
            (SimulationWorldLineChild::Dmo, 2),
            (SimulationWorldLineChild::Vehicle, 4),
        ] {
            if flags & bit == 0
                || (child == SimulationWorldLineChild::Vehicle && self.island_flags != 3)
            {
                continue;
            }
            match self
                .batches_mut(child)
                .acquire(CollisionQueryBatchFamily::StepSelector, label)
            {
                Ok(lease) => children.push(AcquiredChild { child, lease }),
                Err(source) => {
                    self.retained_acquisitions.extend(children);
                    return Err(SimulationWorldLineLeaseError::Batch { child, source });
                }
            }
        }
        Ok(SimulationWorldStepBatch {
            lease: SimulationWorldLineBatch {
                owner: Arc::clone(&self.occurrence),
                released: Cell::new(false),
                children,
            },
            aggregate: RefCell::new([[StepTrajectoryResult::new(); 4]; 5]),
        })
    }
    pub(crate) fn try_begin_step(
        &mut self,
        batch: &SimulationWorldStepBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_begin(&batch.lease)
    }
    pub(crate) fn try_add_step<N, T>(
        &mut self,
        batch: &SimulationWorldStepBatch,
        inputs: &[AnimationStepInput],
    ) -> Result<bool, WorldStepError<N, T>> {
        self.require_batch_owner(&batch.lease)?;
        let mut accepted = true;
        for entry in &batch.lease.children {
            accepted &= self
                .batches_mut(entry.child)
                .add_step_test(&entry.lease, inputs)
                .map_err(|source| WorldStepError::Add {
                    child: entry.child,
                    source,
                })?;
        }
        Ok(accepted)
    }
    pub(crate) fn try_end_step<N: ReckoningNumeric, T: StepQueryTuning>(
        &mut self,
        batch: &SimulationWorldStepBatch,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Result<(), WorldStepError<N::Error, T::Error>> {
        self.require_batch_owner(&batch.lease)?;
        self.require_dmo_owner(dmos)?;
        for entry in &batch.lease.children {
            let result = match entry.child {
                SimulationWorldLineChild::Ground => {
                    self.ground_batches
                        .end_step_tests(&entry.lease, |inputs, result| {
                            run_step_test(inputs, result, &mut StepGround(ground), numeric, tuning)
                                .map_err(StepJobError::Worker)
                        })
                }
                // ClusteredMeshStepSelectorBatch over the DynamicPool meshes.
                SimulationWorldLineChild::Dmo => {
                    self.dmo_batches
                        .end_step_tests(&entry.lease, |inputs, result| {
                            run_step_test(
                                inputs,
                                result,
                                &mut DmoMeshGeometry(dmos),
                                numeric,
                                tuning,
                            )
                            .map_err(StepJobError::Worker)
                        })
                }
                SimulationWorldLineChild::Vehicle => {
                    self.vehicle_batches
                        .end_step_tests(&entry.lease, |inputs, result| {
                            run_step_test(
                                inputs,
                                result,
                                &mut DmoMeshGeometry(&self.vehicle),
                                numeric,
                                tuning,
                            )
                            .map_err(StepJobError::Worker)
                        })
                }
            };
            result.map_err(|source| WorldStepError::Job {
                child: entry.child,
                source,
            })?;
        }
        Ok(())
    }
    pub(crate) fn try_sync_step(
        &mut self,
        batch: &SimulationWorldStepBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_sync(&batch.lease)
    }
    pub(crate) fn try_get_step_result(
        &mut self,
        batch: &SimulationWorldStepBatch,
        index: u32,
    ) -> Result<StepResult, SimulationWorldLineLeaseError> {
        self.require_batch_owner(&batch.lease)?;
        if batch.lease.children.len() == 1 {
            let entry = &batch.lease.children[0];
            return self
                .batches_mut(entry.child)
                .get_step_result(&entry.lease, index)
                .map_err(|source| SimulationWorldLineLeaseError::Batch {
                    child: entry.child,
                    source,
                });
        }
        let mut aggregate = batch.aggregate.borrow_mut();
        for row in &mut *aggregate {
            for value in row {
                value.reset();
            }
        }
        // Native advances to and checks the NEXT reset candidate after each
        // reduction. Thus all twenty entries execute, even when one is valid.
        for row in 0..5 {
            for candidate in 0..4 {
                for entry in &batch.lease.children {
                    let batches = self.batches_mut(entry.child);
                    let count = batches.test_count(&entry.lease).map_err(|source| {
                        SimulationWorldLineLeaseError::Batch {
                            child: entry.child,
                            source,
                        }
                    })?;
                    if index >= count {
                        continue;
                    }
                    let result =
                        batches
                            .get_step_result(&entry.lease, index)
                            .map_err(|source| SimulationWorldLineLeaseError::Batch {
                                child: entry.child,
                                source,
                            })?;
                    consider_child(&mut aggregate[row][candidate], &result[row][candidate]);
                }
            }
        }
        Ok(*aggregate)
    }
    pub(crate) fn try_release_step(
        &mut self,
        batch: &SimulationWorldStepBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_release(&batch.lease)
    }
}
