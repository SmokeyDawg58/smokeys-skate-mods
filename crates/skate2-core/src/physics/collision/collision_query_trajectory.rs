//! Actual trajectory-family lease storage for skeleton and predictor requests.
//! Add827bdcc0, Local827c2270, Walk827c1e58, Get827bdf68.
use super::{
    CollisionQueryBatchError as BatchError, CollisionQueryBatchFamily as Family,
    CollisionQueryBatchLease, CollisionQueryBatchState as State, CollisionQueryBatches,
};
use crate::{reckoning_numeric::ReckoningNumeric, skeleton_root_transforms::SkeletonRootVector};

/// The source skeleton caller supplies g=0, T=1, errorStart=errorEnd=0.
/// This request type makes that selected route explicit, retaining every input lane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonTrajectoryInput {
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) radius: f32,
}
/// Native Add827bdcc0 retains p0, v0, g, T, radius and both error ratios.
/// Unknown fourth lanes survive storage; geometry consumes only XYZ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BallisticTrajectoryInput {
    pub(crate) position: SkeletonRootVector,
    pub(crate) velocity: SkeletonRootVector,
    pub(crate) gravity: SkeletonRootVector,
    pub(crate) duration: f32,
    pub(crate) radius: f32,
    pub(crate) maximum_error_start: f32,
    pub(crate) maximum_error_end: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TrajectoryInput {
    Skeleton(SkeletonTrajectoryInput),
    Ballistic(BallisticTrajectoryInput),
}
impl TrajectoryInput {
    fn has_nan_xyz(self) -> bool {
        let (position, velocity, gravity) = match self {
            Self::Skeleton(input) => (input.position.xyz, input.velocity.xyz, [0.0; 3]),
            Self::Ballistic(input) => (input.position.xyz, input.velocity.xyz, input.gravity.xyz),
        };
        position
            .iter()
            .chain(&velocity)
            .chain(&gravity)
            .any(|v| v.is_nan())
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrajectoryResult {
    pub(crate) position: SkeletonRootVector,
    pub(crate) normal: SkeletonRootVector,
    pub(crate) ground_normal: SkeletonRootVector,
    pub(crate) time: [f32; 4],
    pub(crate) frame: i32,
    pub(crate) surface: u32,
}
impl TrajectoryResult {
    pub(crate) fn initialized() -> Self {
        Self {
            position: SkeletonRootVector::known([0.0; 4]),
            normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            ground_normal: SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]),
            time: [-1.0; 4],
            frame: -1,
            surface: 0,
        }
    }
    pub(crate) fn valid(&self) -> bool {
        self.time.iter().all(|lane| *lane >= 0.0)
    }
}
pub(super) struct TrajectoryTest {
    input: TrajectoryInput,
    result: TrajectoryResult,
}
#[derive(Debug)]
pub(crate) enum TrajectoryBatchError<E> {
    Batch(BatchError),
    InputNaN,
    WrongInputKind,
    Execute { test: u32, source: E },
}
impl<E: std::fmt::Debug> std::fmt::Display for TrajectoryBatchError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Batch(error) => write!(f, "trajectory batch: {error}"),
            Self::InputNaN => f.write_str("trajectory position, velocity or gravity contains NaN"),
            Self::WrongInputKind => f.write_str("ballistic input passed to skeleton-only executor"),
            Self::Execute { test, source } => {
                write!(f, "trajectory test {test} failed: {source:?}")
            }
        }
    }
}
impl<O> CollisionQueryBatches<O> {
    pub(crate) fn add_skeleton_trajectory(
        &mut self,
        lease: &CollisionQueryBatchLease,
        input: SkeletonTrajectoryInput,
    ) -> Result<bool, TrajectoryBatchError<std::convert::Infallible>> {
        self.add_trajectory(lease, TrajectoryInput::Skeleton(input))
    }
    pub(crate) fn add_ballistic_trajectory(
        &mut self,
        lease: &CollisionQueryBatchLease,
        input: BallisticTrajectoryInput,
    ) -> Result<bool, TrajectoryBatchError<std::convert::Infallible>> {
        self.add_trajectory(lease, TrajectoryInput::Ballistic(input))
    }
    fn add_trajectory(
        &mut self,
        lease: &CollisionQueryBatchLease,
        input: TrajectoryInput,
    ) -> Result<bool, TrajectoryBatchError<std::convert::Infallible>> {
        let batch = self.batch_mut(lease).map_err(TrajectoryBatchError::Batch)?;
        if lease.family != Family::Trajectory {
            return Err(TrajectoryBatchError::Batch(BatchError::WrongFamily {
                actual: lease.family,
            }));
        }
        // Native Add checks XYZ self equality before the capacity test.
        if input.has_nan_xyz() {
            return Err(TrajectoryBatchError::InputNaN);
        }
        if batch.count == batch.capacity {
            return Ok(false);
        }
        if batch.state != State::AddTests {
            return Err(TrajectoryBatchError::Batch(BatchError::State {
                actual: batch.state,
            }));
        }
        let index = batch.count as usize;
        batch.count += 1;
        let test = TrajectoryTest {
            input,
            result: TrajectoryResult::initialized(),
        };
        if index == batch.trajectories.len() {
            batch.trajectories.push(test);
        } else {
            batch.trajectories[index] = test;
        }
        Ok(true)
    }
    pub(crate) fn end_skeleton_trajectory<E>(
        &mut self,
        lease: &CollisionQueryBatchLease,
        mut execute: impl FnMut(SkeletonTrajectoryInput) -> Result<TrajectoryResult, E>,
    ) -> Result<(), TrajectoryBatchError<E>> {
        self.finish_trajectory(lease, |index, input| match input {
            TrajectoryInput::Skeleton(input) => {
                execute(input).map_err(|source| TrajectoryBatchError::Execute {
                    test: index,
                    source,
                })
            }
            TrajectoryInput::Ballistic(_) => Err(TrajectoryBatchError::WrongInputKind),
        })
    }
    pub(crate) fn end_trajectory<E>(
        &mut self,
        lease: &CollisionQueryBatchLease,
        mut execute: impl FnMut(TrajectoryInput) -> Result<TrajectoryResult, E>,
    ) -> Result<(), TrajectoryBatchError<E>> {
        self.finish_trajectory(lease, |index, input| {
            execute(input).map_err(|source| TrajectoryBatchError::Execute {
                test: index,
                source,
            })
        })
    }
    fn finish_trajectory<E>(
        &mut self,
        lease: &CollisionQueryBatchLease,
        mut execute: impl FnMut(
            u32,
            TrajectoryInput,
        ) -> Result<TrajectoryResult, TrajectoryBatchError<E>>,
    ) -> Result<(), TrajectoryBatchError<E>> {
        let batch = self.batch_mut(lease).map_err(TrajectoryBatchError::Batch)?;
        if lease.family != Family::Trajectory {
            return Err(TrajectoryBatchError::Batch(BatchError::WrongFamily {
                actual: lease.family,
            }));
        }
        if batch.state != State::AddTests {
            return Err(TrajectoryBatchError::Batch(BatchError::State {
                actual: batch.state,
            }));
        }
        batch.state = State::Running;
        for index in 0..batch.count {
            batch.incomplete = Some(index);
            let result = execute(index, batch.trajectories[index as usize].input)?;
            batch.trajectories[index as usize].result = result;
        }
        batch.incomplete = None;
        Ok(())
    }
    pub(crate) fn get_trajectory_result(
        &self,
        lease: &CollisionQueryBatchLease,
        index: u32,
    ) -> Result<Option<TrajectoryResult>, BatchError> {
        let batch = self.batch(lease)?;
        if lease.family != Family::Trajectory {
            return Err(BatchError::WrongFamily {
                actual: lease.family,
            });
        }
        if batch.state != State::Idle {
            return Err(BatchError::State {
                actual: batch.state,
            });
        }
        if index >= batch.count {
            return Err(BatchError::TestIndex {
                index,
                count: batch.count,
            });
        }
        let result = batch.trajectories[index as usize].result;
        Ok(result.valid().then_some(result))
    }
}

fn scratch(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}
fn length<N: ReckoningNumeric>(v: [f32; 3], numeric: &mut N) -> Result<f32, N::Error> {
    let squared = numeric.dot3(scratch(v), scratch(v))?;
    let mut inverse = numeric.reciprocal_sqrt_estimate(squared)?;
    for _ in 0..2 {
        let square = inverse * inverse;
        let half = inverse * 0.5;
        let residual = -squared.mul_add(square, -1.0);
        inverse = half.mul_add(residual, inverse);
    }
    let value = squared * inverse;
    Ok(if squared == 0.0 { 0.0 } else { value })
}

/// Selected g=0 branch of `CalculateTimeOfCollision827bce08`. Native time uses
/// the unrounded ratio at d080, while only frame uses floor at d064/d068.
pub(crate) fn linear_collision_time<N: ReckoningNumeric>(
    input: SkeletonTrajectoryInput,
    position: [f32; 3],
    numeric: &mut N,
) -> Result<(f32, i32), N::Error> {
    let delta = std::array::from_fn(|i| position[i] - input.position.xyz[i]);
    let distance = length(delta, numeric)?;
    let epsilon = f32::from_bits(0x38d1_b717);
    // CalculateTime827bcf54 returns before loading/reducing velocity when the
    // distance test fails. Preserve that lazy numerical failure boundary.
    if distance.abs().partial_cmp(&epsilon) != Some(std::cmp::Ordering::Greater) {
        return Ok((0.0, 0));
    }
    let speed = length(input.velocity.xyz, numeric)?;
    if speed.abs().partial_cmp(&epsilon) != Some(std::cmp::Ordering::Greater) {
        return Ok((0.0, 0));
    }
    let mut inverse = numeric.reciprocal_estimate(speed)?;
    for _ in 0..2 {
        let residual = -inverse.mul_add(speed, -1.0);
        inverse = inverse.mul_add(residual, inverse);
    }
    let ratio = inverse * distance;
    let frame = numeric.truncate_i32(ratio.floor())?;
    Ok((ratio * f32::from_bits(0x3c88_8889), frame))
}

/// The native zero-gravity, duration-one walk has one segment. Threshold is
/// (errorStart*radius)*errorStart*radius, hence source zero for this caller.
pub(crate) type SkeletonTrajectorySegment = ([f32; 3], [f32; 3]);
pub(crate) fn skeleton_segment<N: ReckoningNumeric>(
    input: SkeletonTrajectoryInput,
    numeric: &mut N,
) -> Result<Option<SkeletonTrajectorySegment>, N::Error> {
    let start = std::array::from_fn(|i| {
        0.0_f32.mul_add(
            0.0,
            input.velocity.xyz[i].mul_add(0.0, input.position.xyz[i]),
        )
    });
    let end = std::array::from_fn(|i| {
        0.0_f32.mul_add(
            1.0,
            input.velocity.xyz[i].mul_add(1.0, input.position.xyz[i]),
        )
    });
    let delta = std::array::from_fn(|i| end[i] - start[i]);
    let squared = numeric.dot3(scratch(delta), scratch(delta))?;
    let threshold = ((0.0 * input.radius) * 0.0) * input.radius;
    Ok((squared > threshold).then_some((start, end)))
}
