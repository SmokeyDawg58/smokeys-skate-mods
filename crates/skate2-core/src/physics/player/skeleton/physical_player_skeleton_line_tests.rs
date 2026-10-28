//! Source Start82dfe850 / End82dfeb70 / Fill82dfeca8 on the same physical pair.
//! Update1 launches new batches but copies the old results; Update4 collects.
use crate::physical_player_assembly::{PhysicalAssemblyWorld, hilod_tail::SkeletonLineResult};
use crate::{
    collision_query_batches::trajectory::SkeletonTrajectoryInput,
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError},
        skeleton_normal::part_transform_query::SkeletonPartTransformQueryError,
    },
    input_cvar_runtime::InputCvarError,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineLeaseError, trajectory::WorldTrajectoryError,
    },
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
#[cfg(test)]
#[path = "physical_player_skeleton_line_tests_tests.rs"]
pub(crate) mod tests;

#[derive(Debug)]
pub(crate) enum SkeletonLineTestsError<N> {
    MissingPool,
    MissingWorld,
    MissingDmo,
    MissingRecords,
    Poisoned,
    UnfinishedBatch { index: usize },
    MissingBatch { index: usize },
    Pair(SimulationSkeletonJumpPairError),
    Transform(SkeletonPartTransformQueryError),
    Cvar(InputCvarError),
    Lease(SimulationWorldLineLeaseError),
    World(WorldTrajectoryError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for SkeletonLineTestsError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("skeleton lines require the skater pool"),
            Self::MissingWorld => f.write_str("skeleton lines require the world's query owner"),
            Self::MissingDmo => {
                f.write_str("skeleton lines require the actual DMO collection owner")
            }
            Self::MissingRecords => f.write_str("skeleton line records have not been constructed"),
            Self::Poisoned => f.write_str("skeleton line owner lock is poisoned"),
            Self::UnfinishedBatch { index } => {
                write!(f, "skeleton line {index} still owns an unreleased batch")
            }
            Self::MissingBatch { index } => {
                write!(f, "skeleton line {index} has no batch to collect")
            }
            Self::Pair(error) => write!(f, "skeleton line physical pair: {error:?}"),
            Self::Transform(error) => write!(f, "skeleton line start transform: {error}"),
            Self::Cvar(error) => write!(f, "skeleton line radius reference: {error:?}"),
            Self::Lease(error) => write!(f, "skeleton line query lease: {error:?}"),
            Self::World(error) => write!(f, "skeleton line world query: {error}"),
        }
    }
}
fn shifted(value: SkeletonRootVector, y: f32) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: [value.xyz[0] + 0.0, value.xyz[1] + y, value.xyz[2] + 0.0],
        w: value.w.map(|w| w + 0.0),
    }
}
fn result_or_clear(
    result: Option<crate::collision_query_batches::trajectory::TrajectoryResult>,
) -> SkeletonLineResult {
    match result {
        Some(result) => SkeletonLineResult {
            position: result.position,
            normal: result.ground_normal,
            surface_id: i32::from_ne_bytes(result.surface.to_ne_bytes()),
            valid: result.valid(),
        },
        None => SkeletonLineResult {
            position: SkeletonRootVector::known([0.0; 4]),
            normal: SkeletonRootVector::known([0.0; 4]),
            surface_id: 0,
            valid: false,
        },
    }
}
impl PhysicalAssemblyWorld {
    /// Source reads three Skeleton query transforms before acquiring any batch.
    /// Each request retains its source start store and live radius sample.
    pub(crate) fn try_start_skeleton_line_tests<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<(), SkeletonLineTestsError<N::Error>> {
        use SkeletonLineTestsError as E;
        let queries = self.world_line_queries.as_mut().ok_or(E::MissingWorld)?;
        let dmos = self.dmos.as_ref().ok_or(E::MissingDmo)?;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, owner| {
                let hips = *normal
                    .try_query_part_transform(23)
                    .map_err(E::Transform)?
                    .columns()
                    .last()
                    .unwrap();
                let left = *normal
                    .try_query_part_transform(15)
                    .map_err(E::Transform)?
                    .columns()
                    .last()
                    .unwrap();
                let right = *normal
                    .try_query_part_transform(19)
                    .map_err(E::Transform)?
                    .columns()
                    .last()
                    .unwrap();
                let mut tail = owner.tail().lock().map_err(|_| E::Poisoned)?;
                let records = tail.line_records.as_mut().ok_or(E::MissingRecords)?;
                for (index, (position, velocity)) in [
                    (hips, SkeletonRootVector::known([0.0, -100.0, 0.0, 0.0])),
                    (
                        shifted(left, f32::from_bits(0x3ea8_f5c3)),
                        SkeletonRootVector::known([
                            0.0,
                            -1.5 - f32::from_bits(0x3ea8_f5c3),
                            0.0,
                            0.0,
                        ]),
                    ),
                    (
                        shifted(right, f32::from_bits(0x3ea8_f5c3)),
                        SkeletonRootVector::known([
                            0.0,
                            -1.5 - f32::from_bits(0x3ea8_f5c3),
                            0.0,
                            0.0,
                        ]),
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let record = &mut records[index];
                    // Host retry guard prevents replacing an unreleased source lease.
                    if record.trajectory_batch.is_some() {
                        return Err(E::UnfinishedBatch { index });
                    }
                    record.trajectory_batch = Some(
                        queries
                            .try_acquire_skeleton_trajectory(
                                dmos,
                                "PhysicalPlayer::StartSkeletonLineTests",
                            )
                            .map_err(E::Lease)?,
                    );
                    let batch = record
                        .trajectory_batch
                        .as_ref()
                        .expect("published acquisition");
                    queries.try_begin_trajectory(batch).map_err(E::Lease)?;
                    record.start_position = position;
                    let radius = owner
                        .inputs()
                        .cvars
                        .try_feet_line_test_radius()
                        .map_err(E::Cvar)?;
                    let _ = queries
                        .try_add_skeleton_trajectory(
                            batch,
                            SkeletonTrajectoryInput {
                                position,
                                velocity,
                                radius,
                            },
                        )
                        .map_err(E::World)?;
                    queries
                        .try_end_trajectory(batch, ground, dmos, numeric)
                        .map_err(E::World)?;
                }
                Ok(())
            })
            .map_err(E::Pair)?
    }
    /// Update4 call82e01c94: Sync/GetResult then copy/clear, Release, null pointer.
    pub(crate) fn try_end_skeleton_line_tests(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkeletonLineTestsError<std::convert::Infallible>> {
        use SkeletonLineTestsError as E;
        let queries = self.world_line_queries.as_mut().ok_or(E::MissingWorld)?;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                let mut tail = owner.tail().lock().map_err(|_| E::Poisoned)?;
                let records = tail.line_records.as_mut().ok_or(E::MissingRecords)?;
                for (index, record) in records.iter_mut().enumerate() {
                    let batch = record
                        .trajectory_batch
                        .as_ref()
                        .ok_or(E::MissingBatch { index })?;
                    queries.try_sync_trajectory(batch).map_err(E::Lease)?;
                    let result = queries
                        .try_get_trajectory_result(batch, 0)
                        .map_err(E::Lease)?;
                    record.result = result_or_clear(result);
                    queries.try_release_trajectory(batch).map_err(E::Lease)?;
                    record.trajectory_batch = None;
                }
                Ok(())
            })
            .map_err(E::Pair)?
    }
    /// Update1 call36d4 plus conditional ID stores immediately before `ProcessData`.
    /// No Sync, current-result getter or batch release occurs here.
    pub(crate) fn try_fill_skeleton_line_surfaces(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkeletonLineTestsError<std::convert::Infallible>> {
        use SkeletonLineTestsError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                let processed = std::sync::Arc::clone(
                    owner
                        .reckoning()
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .processed(),
                );
                let tail = owner.tail().lock().map_err(|_| E::Poisoned)?;
                let records = tail.line_records.as_ref().ok_or(E::MissingRecords)?;
                let results = std::array::from_fn(|i| records[i].result);
                processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .fill_skeleton_line_surfaces(&results);
                Ok(())
            })
            .map_err(E::Pair)?
    }
}
