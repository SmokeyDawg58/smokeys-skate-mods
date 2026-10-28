//! World/Island AcquireTrajectory827ab958/827b8998 and actual child jobs.
//! Selected skeleton queries retain leases until the later Update4 Sync/Release.
use super::{
    AcquiredChild, CollisionQueryBatchFamily, ConstructorCollection, ReckoningNumeric,
    SimulationDmoAssemblyPool, SimulationWorldLineBatch, SimulationWorldLineChild,
    SimulationWorldLineLeaseError, SimulationWorldLineQueries, StaticGroundPool,
};
use crate::{
    collision_query_batches::trajectory::{
        BallisticTrajectoryInput, SkeletonTrajectoryInput, TrajectoryBatchError, TrajectoryInput,
        TrajectoryResult, linear_collision_time, skeleton_segment,
    },
    physics::{
        BallisticTrajectory, TrajectoryCollisionWorld, TrajectoryGroundTriangle,
        TrajectoryPredictionError, TrajectoryPredictionQuery, TrajectorySweepHit,
        calculate_trajectory_ground_normal, predict_ballistic_trajectory_with_numeric,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::DmoMeshGeometry,
    skeleton_root_transforms::SkeletonRootVector,
};
use skate2_formats::collision::CollisionError;
use std::{cell::Cell, sync::Arc};

#[derive(Debug)]
pub(crate) struct SimulationWorldTrajectoryBatch(SimulationWorldLineBatch);
#[derive(Debug)]
pub(crate) enum WorldTrajectoryError<N> {
    Lease(SimulationWorldLineLeaseError),
    Add {
        child: SimulationWorldLineChild,
        source: TrajectoryBatchError<std::convert::Infallible>,
    },
    Job {
        child: SimulationWorldLineChild,
        source: TrajectoryBatchError<TrajectoryJobError<N>>,
    },
}
impl<N: std::fmt::Debug> std::fmt::Display for WorldTrajectoryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lease(error) => write!(f, "world trajectory lease: {error:?}"),
            Self::Add { child, source } => write!(f, "world trajectory {child:?} add: {source}"),
            Self::Job { child, source } => write!(f, "world trajectory {child:?} job: {source}"),
        }
    }
}
impl<N> From<SimulationWorldLineLeaseError> for WorldTrajectoryError<N> {
    fn from(value: SimulationWorldLineLeaseError) -> Self {
        Self::Lease(value)
    }
}
#[derive(Debug)]
pub(crate) enum TrajectoryJobError<N> {
    Numeric(N),
    Collision(CollisionError),
    UnpublishedMembership { count: usize },
}
impl<N: std::fmt::Debug> std::fmt::Display for TrajectoryJobError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "trajectory numeric operation: {error:?}"),
            Self::Collision(error) => write!(f, "trajectory terrain query: {error}"),
            Self::UnpublishedMembership { count } => write!(
                f,
                "trajectory collection has {count} members without published geometry"
            ),
        }
    }
}

/// The actual published `StaticGroundPool` is borrowed throughout the job.
struct GroundGeometry<'a>(&'a StaticGroundPool);
impl TrajectoryCollisionWorld for GroundGeometry<'_> {
    type Error = CollisionError;
    fn sweep_sphere_segment(
        &mut self,
        start: [f32; 3],
        end: [f32; 3],
        radius: f32,
    ) -> Result<Option<TrajectorySweepHit>, Self::Error> {
        self.0.trajectory_sweep_sphere_segment(start, end, radius)
    }
    fn triangles_overlapping_aabb(
        &mut self,
        minimum: [f32; 3],
        maximum: [f32; 3],
        _: usize,
    ) -> Result<Vec<TrajectoryGroundTriangle>, Self::Error> {
        self.0
            .trajectory_ground_triangles_overlapping_aabb(minimum, maximum)
    }
}

fn run<N: ReckoningNumeric>(
    input: SkeletonTrajectoryInput,
    geometry: &mut impl TrajectoryCollisionWorld<Error = CollisionError>,
    numeric: &mut N,
) -> Result<TrajectoryResult, TrajectoryJobError<N::Error>> {
    let mut result = TrajectoryResult::initialized();
    let Some((start, end)) =
        skeleton_segment(input, numeric).map_err(TrajectoryJobError::Numeric)?
    else {
        return Ok(result);
    };
    let Some(hit) = geometry
        .sweep_sphere_segment(start, end, input.radius)
        .map_err(TrajectoryJobError::Collision)?
    else {
        return Ok(result);
    };
    let (time, frame) =
        linear_collision_time(input, hit.position, numeric).map_err(TrajectoryJobError::Numeric)?;
    let minimum = hit.position.map(|v| v - input.radius);
    let maximum = hit.position.map(|v| v + input.radius);
    let triangles = geometry
        .triangles_overlapping_aabb(minimum, maximum, 64)
        .map_err(TrajectoryJobError::Collision)?;
    // Source827c1d88..1da8 forms g*(frame*DT)+v0, even though this caller's g=0.
    #[expect(
        clippy::cast_precision_loss,
        reason = "native integer frame conversion to float"
    )]
    let frame_time = frame as f32 * f32::from_bits(0x3c88_8889);
    let velocity = input.velocity.xyz.map(|v| 0.0_f32.mul_add(frame_time, v));
    let normal = calculate_trajectory_ground_normal(velocity, &triangles);
    // Existing authored geometry APIs expose XYZ only. Preserve unknown W;
    // no successful trace/canonical fourth lane is manufactured by this bridge.
    result.position = SkeletonRootVector {
        xyz: hit.position,
        w: None,
    };
    result.normal = SkeletonRootVector {
        xyz: hit.normal,
        w: None,
    };
    result.ground_normal = SkeletonRootVector {
        xyz: normal,
        w: None,
    };
    result.time = [time; 4];
    result.frame = frame;
    result.surface = hit.surface_id;
    Ok(result)
}

/// Convert only at the geometry leaf. Stored requests retain all optional W lanes.
fn run_ballistic<N: ReckoningNumeric>(
    input: BallisticTrajectoryInput,
    geometry: &mut impl TrajectoryCollisionWorld<Error = CollisionError>,
    numeric: &mut N,
) -> Result<TrajectoryResult, TrajectoryJobError<N::Error>> {
    let output = predict_ballistic_trajectory_with_numeric(
        TrajectoryPredictionQuery {
            trajectory: BallisticTrajectory {
                initial_position: input.position.xyz,
                initial_velocity: input.velocity.xyz,
                gravity: input.gravity.xyz,
                duration: input.duration,
            },
            radius: input.radius,
            maximum_error_start: input.maximum_error_start,
            maximum_error_end: input.maximum_error_end,
            calculate_ground_normal: true, // Local827c2270 passes true at 22e0.
        },
        geometry,
        numeric,
    )
    .map_err(|error| match error {
        TrajectoryPredictionError::Geometry(error) => TrajectoryJobError::Collision(error),
        TrajectoryPredictionError::Numeric(error) => TrajectoryJobError::Numeric(error),
    })?;
    // A native hit payload is copied even if its computed time is negative.
    if std::env::var_os("SKATE2_TRACE_TRAJECTORY").is_some() {
        eprintln!(
            "ballistic query: p={:?} v={:?} g={:?} radius={} duration={} errors=({}, {}) result={:?}",
            input.position.xyz,
            input.velocity.xyz,
            input.gravity.xyz,
            input.radius,
            input.duration,
            input.maximum_error_start,
            input.maximum_error_end,
            output
        );
    }
    // Validity filtering belongs to GetResult, not Local job execution.
    let Some(output) = output else {
        return Ok(TrajectoryResult::initialized());
    };
    Ok(TrajectoryResult {
        position: SkeletonRootVector {
            xyz: output.position,
            w: None,
        },
        normal: SkeletonRootVector {
            xyz: output.normal,
            w: None,
        },
        ground_normal: SkeletonRootVector {
            xyz: output.ground_normal,
            w: None,
        },
        time: [output.collision_time; 4],
        frame: output.collision_frame,
        surface: output.surface_id,
    })
}
fn run_request<N: ReckoningNumeric>(
    input: TrajectoryInput,
    geometry: &mut impl TrajectoryCollisionWorld<Error = CollisionError>,
    numeric: &mut N,
) -> Result<TrajectoryResult, TrajectoryJobError<N::Error>> {
    match input {
        TrajectoryInput::Skeleton(input) => run(input, geometry, numeric),
        TrajectoryInput::Ballistic(input) => run_ballistic(input, geometry, numeric),
    }
}

/// Used only after checking the actual constructor-owned collection is empty.
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

/// `ConstructorCollection` is demonstrably empty and has no publication API.
/// Its actual slot count and matching live membership are checked at each job.
fn run_constructor_collection<N: ReckoningNumeric>(
    collection: &ConstructorCollection,
    membership: usize,
    input: TrajectoryInput,
    numeric: &mut N,
) -> Result<TrajectoryResult, TrajectoryJobError<N::Error>> {
    if membership != 0 || !collection.slots.is_empty() {
        return Err(TrajectoryJobError::UnpublishedMembership {
            count: membership.max(collection.slots.len()),
        });
    }
    // The same walk executes against the verified zero-entry collection.
    run_request(input, &mut CheckedEmptyGeometry, numeric)
}

impl SimulationWorldLineQueries {
    pub(crate) fn try_acquire_skeleton_trajectory(
        &mut self,
        dmos: &SimulationDmoAssemblyPool,
        label: &str,
    ) -> Result<SimulationWorldTrajectoryBatch, SimulationWorldLineLeaseError> {
        self.try_acquire_trajectory(dmos, label)
    }
    pub(crate) fn try_acquire_trajectory(
        &mut self,
        dmos: &SimulationDmoAssemblyPool,
        label: &str,
    ) -> Result<SimulationWorldTrajectoryBatch, SimulationWorldLineLeaseError> {
        self.try_acquire_trajectory_with_flags(dmos, label, 7)
    }
    /// Island::AcquireTrajectoryBatch827b8998: explicit Ground/DMO/Vehicle bits.
    pub(crate) fn try_acquire_trajectory_with_flags(
        &mut self,
        dmos: &SimulationDmoAssemblyPool,
        label: &str,
        flags: u32,
    ) -> Result<SimulationWorldTrajectoryBatch, SimulationWorldLineLeaseError> {
        if flags == 0 {
            return Err(SimulationWorldLineLeaseError::ZeroFlags);
        }
        self.require_dmo_owner(dmos)?;
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
                .acquire(CollisionQueryBatchFamily::Trajectory, label)
            {
                Ok(lease) => children.push(AcquiredChild { child, lease }),
                Err(source) => {
                    self.retained_acquisitions.extend(children);
                    return Err(SimulationWorldLineLeaseError::Batch { child, source });
                }
            }
        }
        Ok(SimulationWorldTrajectoryBatch(SimulationWorldLineBatch {
            owner: Arc::clone(&self.occurrence),
            released: Cell::new(false),
            children,
        }))
    }
    pub(crate) fn try_begin_trajectory(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_begin(&batch.0)
    }
    pub(crate) fn try_add_skeleton_trajectory<N>(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
        input: SkeletonTrajectoryInput,
    ) -> Result<bool, WorldTrajectoryError<N>> {
        self.require_batch_owner(&batch.0)?;
        let mut accepted = true;
        for entry in &batch.0.children {
            accepted &= self
                .batches_mut(entry.child)
                .add_skeleton_trajectory(&entry.lease, input)
                .map_err(|source| WorldTrajectoryError::Add {
                    child: entry.child,
                    source,
                })?;
        }
        Ok(accepted)
    }
    pub(crate) fn try_add_ballistic_trajectory<N>(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
        input: BallisticTrajectoryInput,
    ) -> Result<bool, WorldTrajectoryError<N>> {
        self.require_batch_owner(&batch.0)?;
        let mut accepted = true;
        for entry in &batch.0.children {
            accepted &= self
                .batches_mut(entry.child)
                .add_ballistic_trajectory(&entry.lease, input)
                .map_err(|source| WorldTrajectoryError::Add {
                    child: entry.child,
                    source,
                })?;
        }
        Ok(accepted)
    }
    pub(crate) fn try_end_trajectory<N: ReckoningNumeric>(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
        ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool,
        numeric: &mut N,
    ) -> Result<(), WorldTrajectoryError<N::Error>> {
        self.require_batch_owner(&batch.0)?;
        self.require_dmo_owner(dmos)?;
        for entry in &batch.0.children {
            let result = match entry.child {
                SimulationWorldLineChild::Ground => {
                    self.ground_batches.end_trajectory(&entry.lease, |input| {
                        run_request(input, &mut GroundGeometry(ground), numeric)
                    })
                }
                // ClusteredMeshTrajectoryBatch over the DynamicPool meshes.
                SimulationWorldLineChild::Dmo => {
                    self.dmo_batches.end_trajectory(&entry.lease, |input| {
                        run_request(input, &mut DmoMeshGeometry(dmos), numeric)
                    })
                }
                SimulationWorldLineChild::Vehicle => {
                    self.vehicle_batches.end_trajectory(&entry.lease, |input| {
                        run_request(input, &mut DmoMeshGeometry(&self.vehicle), numeric)
                    })
                }
            };
            result.map_err(|source| WorldTrajectoryError::Job {
                child: entry.child,
                source,
            })?;
        }
        Ok(())
    }
    pub(crate) fn try_sync_trajectory(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_sync(&batch.0)
    }
    pub(crate) fn try_get_trajectory_result(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
        index: u32,
    ) -> Result<Option<TrajectoryResult>, SimulationWorldLineLeaseError> {
        self.require_batch_owner(&batch.0)?;
        let mut nearest: Option<TrajectoryResult> = None;
        for entry in &batch.0.children {
            let batches = self.batches_mut(entry.child);
            // Aggregate count gate; a sole direct child still asserts its index.
            if batch.0.children.len() != 1
                && index
                    >= batches.test_count(&entry.lease).map_err(|source| {
                        SimulationWorldLineLeaseError::Batch {
                            child: entry.child,
                            source,
                        }
                    })?
            {
                continue;
            }
            if let Some(candidate) =
                batches
                    .get_trajectory_result(&entry.lease, index)
                    .map_err(|source| SimulationWorldLineLeaseError::Batch {
                        child: entry.child,
                        source,
                    })?
                && nearest
                    .as_ref()
                    .is_none_or(|old| old.time.iter().zip(candidate.time).all(|(a, b)| *a > b))
            {
                nearest = Some(candidate);
            }
        }
        Ok(nearest)
    }
    pub(crate) fn try_release_trajectory(
        &mut self,
        batch: &SimulationWorldTrajectoryBatch,
    ) -> Result<(), SimulationWorldLineLeaseError> {
        self.try_release(&batch.0)
    }
}

#[cfg(test)]
#[path = "../collision/ballistic_trajectory_request_tests.rs"]
mod ballistic_request_tests;
