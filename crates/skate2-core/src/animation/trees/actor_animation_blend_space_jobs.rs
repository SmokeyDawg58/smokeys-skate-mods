//! Retained `BlendSpace` worker parameters and the Animatable's eight-entry queue.
//! Source Register82d6b948, worker82d73520: repeated node entries are NOT deduped.
use super::parameterization::{
    ParameterizationError, ParameterizationOutput, compute_parameterization3,
};
use crate::{
    actor_animation_tree::{AnimationNodeHandle, AnimationTreeError},
    animation_job_owners::{
        AnimationJobCode,
        execution::{AnimationExecutionError, AnimationJobWork},
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
};
use skate2_formats::andale::AndaleBlendSpace;
use std::sync::{Arc, Mutex};

/// The allocations initialized by BlendSpace.Init are shared with its worker.
/// Query=None distinguishes constructor storage from an initiated parameter block.
pub(crate) struct BlendSpaceParameters {
    pub(crate) projected: Vec<f32>,
    pub(crate) coefficients: Vec<f32>,
    pub(crate) query: Option<[f32; 4]>,
    pub(crate) volume_count: usize,
    pub(crate) found: Option<usize>,
    pub(crate) ready: bool,
    pub(crate) needs_projection: bool,
    pub(crate) is_data_needed: bool,
}
impl BlendSpaceParameters {
    pub(crate) fn uninitiated() -> Self {
        Self {
            projected: vec![],
            coefficients: vec![],
            query: None,
            volume_count: 0,
            found: None,
            ready: false,
            needs_projection: false,
            is_data_needed: false,
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum BlendSpacePool {
    Body(usize),
    Channel(usize),
}
#[derive(Clone)]
pub(crate) struct BlendSpaceRecord {
    pub(crate) pool: BlendSpacePool,
    pub(crate) node: AnimationNodeHandle,
    pub(crate) parameters: Arc<Mutex<BlendSpaceParameters>>,
    pub(crate) data: Arc<AndaleBlendSpace>,
    pub(crate) volume_set: usize,
    #[expect(
        dead_code,
        reason = "native descriptor retains DMA byte size; local CPU worker needs no DMA transfer"
    )]
    pub(crate) volume_data_size: u32,
}
/// Actual Init count0 owner. Inactive native array storage has no observable value.
pub(crate) struct BlendSpaceJobBuffer {
    pub(crate) records: Vec<BlendSpaceRecord>,
    pub(crate) failure: Option<Arc<BlendSpaceJobError>>,
}
impl BlendSpaceJobBuffer {
    pub(crate) fn initialized() -> Self {
        Self {
            records: Vec::new(),
            failure: None,
        }
    }
    pub(crate) fn register(&mut self, record: BlendSpaceRecord) -> Result<(), BlendSpaceJobError> {
        if self.records.len() >= 8 {
            return Err(BlendSpaceJobError::Capacity);
        }
        // Capacity assertion precedes this store; every call retains an entry.
        record
            .parameters
            .lock()
            .map_err(|_| BlendSpaceJobError::Lock)?
            .is_data_needed = true;
        self.records.push(record);
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) enum BlendSpaceJobError {
    Lock,
    Capacity,
    Missing(&'static str),
    Tree(AnimationTreeError),
    Cvar(InputCvarError),
    LiveNumeric(ParameterizationError<LiveNumericError>),
    Generic2(super::generic2::Generic2Error<InputCvarError>),
    Scheduler(AnimationExecutionError),
    Worker(Arc<BlendSpaceJobError>),
}
impl std::fmt::Display for BlendSpaceJobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "BlendSpace requires {name}"),
            Self::Tree(error) => error.fmt(f),
            Self::Cvar(error) => write!(f, "BlendSpace cvar: {error:?}"),
            Self::LiveNumeric(ParameterizationError::Numeric(error)) => error.fmt(f),
            Self::LiveNumeric(error) => write!(f, "BlendSpace numeric: {error:?}"),
            Self::Generic2(error) => write!(f, "BlendSpace generic 2D: {error:?}"),
            Self::Worker(error) => error.fmt(f),
            Self::Scheduler(error) => write!(f, "BlendSpace scheduler: {error:?}"),
            other => write!(f, "BlendSpace: {other:?}"),
        }
    }
}
/// Same live cvar target owner, sampled once for each outside volume.
/// Cvar access failures retain native result prefixes and the worker-needed flag.
pub(crate) fn compute_record(
    record: &BlendSpaceRecord,
    cvars: &InputCvarReferences,
) -> Result<(), BlendSpaceJobError> {
    let mut parameters = record
        .parameters
        .lock()
        .map_err(|_| BlendSpaceJobError::Lock)?;
    let query = parameters
        .query
        .ok_or(BlendSpaceJobError::Missing("initiated parameter block"))?;
    let set = record
        .data
        .volume_sets
        .get(record.volume_set)
        .ok_or(BlendSpaceJobError::Missing("volume set"))?;
    let count = parameters.volume_count;
    let volumes = set
        .volumes
        .get(..count)
        .ok_or(BlendSpaceJobError::Missing("volume count"))?;
    let BlendSpaceParameters {
        projected,
        coefficients,
        found,
        ready,
        needs_projection,
        ..
    } = &mut *parameters;
    let projected = projected
        .get_mut(..4)
        .and_then(|v| v.try_into().ok())
        .ok_or(BlendSpaceJobError::Missing("four projected lanes"))?;
    let coefficients = coefficients
        .get_mut(..4)
        .and_then(|v| v.try_into().ok())
        .ok_or(BlendSpaceJobError::Missing("four coefficient lanes"))?;
    // Adapter records cvar errors without mapping them to a guessed boolean.
    let mut numeric = BlendSpaceLiveNumeric {
        host: HostReckoningNumeric,
    };
    compute_parameterization3(
        query,
        volumes,
        ParameterizationOutput {
            projected,
            coefficients,
            found,
            needs_projection,
            ready,
        },
        &mut numeric,
        || {
            cvars
                .try_blend_space_slicing3()
                .map_err(LiveNumericError::Cvar)
        },
    )
    .map_err(|error| {
        #[cfg(test)]
        if matches!(error, ParameterizationError::VolumeBounds) {
            eprintln!(
                "BlendSpace bounds: {} set {} volumes {:?}",
                record.data.name,
                record.volume_set,
                volumes
                    .iter()
                    .map(|volume| (
                        volume.serialized_data.len(),
                        volume.normals_offset,
                        volume.heights_offset,
                        volume.animation_indexes_offset,
                        volume.matrix_coordinates.len()
                    ))
                    .collect::<Vec<_>>()
            );
        }
        BlendSpaceJobError::LiveNumeric(error)
    })
}
#[derive(Debug)]
pub(crate) enum LiveNumericError {
    Host(ReckoningHostNumericError),
    Cvar(InputCvarError),
}
impl std::fmt::Display for LiveNumericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host(error) => write!(f, "BlendSpace host numeric: {error:?}"),
            Self::Cvar(error) => write!(f, "BlendSpace cvar: {error:?}"),
        }
    }
}
struct BlendSpaceLiveNumeric {
    host: HostReckoningNumeric,
}
impl crate::reckoning_numeric::ReckoningNumeric for BlendSpaceLiveNumeric {
    type Error = LiveNumericError;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.host.dot3(a, b).map_err(LiveNumericError::Host)
    }
    fn reciprocal_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.host
            .reciprocal_estimate(v)
            .map_err(LiveNumericError::Host)
    }
    fn reciprocal_sqrt_estimate(&mut self, v: f32) -> Result<f32, Self::Error> {
        self.host
            .reciprocal_sqrt_estimate(v)
            .map_err(LiveNumericError::Host)
    }
    fn truncate_i32(&mut self, v: f32) -> Result<i32, Self::Error> {
        self.host.truncate_i32(v).map_err(LiveNumericError::Host)
    }
}
impl crate::actor_face_playback::presentation::read::decode::PresentationDecodeNumeric
    for BlendSpaceLiveNumeric
{
    fn dot4(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        self.host.dot4(a, b).map_err(LiveNumericError::Host)
    }
}
impl super::parameterization::BlendSpaceNumeric for BlendSpaceLiveNumeric {
    fn maximum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        self.host.maximum(a, b).map_err(LiveNumericError::Host)
    }
    fn minimum(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<[f32; 4], Self::Error> {
        self.host.minimum(a, b).map_err(LiveNumericError::Host)
    }
}
pub(crate) struct BlendSpaceWork {
    pub(crate) buffer: Arc<Mutex<BlendSpaceJobBuffer>>,
    pub(crate) cvars: InputCvarReferences,
}
impl AnimationJobWork for BlendSpaceWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        if code != AnimationJobCode::BlendSpace {
            return Err(AnimationExecutionError::UnsupportedEntryPoint);
        }
        match self.run_records() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.buffer
                    .lock()
                    .map_err(|_| AnimationExecutionError::Lock)?
                    .failure = Some(Arc::new(error));
                Err(AnimationExecutionError::Work(
                    "BlendSpace worker; detailed failure retained",
                ))
            }
        }
    }
}
impl BlendSpaceWork {
    fn run_records(&self) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let count = self.buffer.lock().map_err(|_| E::Lock)?.records.len();
        for index in 0..count {
            let record = self
                .buffer
                .lock()
                .map_err(|_| E::Lock)?
                .records
                .get(index)
                .cloned()
                .ok_or(E::Missing("retained worker record"))?;
            if !record
                .parameters
                .lock()
                .map_err(|_| E::Lock)?
                .is_data_needed
            {
                return Err(E::Missing("BlendSpace isDataNeeded assertion"));
            }
            compute_record(&record, &self.cvars)?;
            // Native reloads the record's parameter pointer after the kernel.
            let parameters = Arc::clone(
                &self
                    .buffer
                    .lock()
                    .map_err(|_| E::Lock)?
                    .records
                    .get(index)
                    .ok_or(E::Missing("retained worker record"))?
                    .parameters,
            );
            parameters.lock().map_err(|_| E::Lock)?.is_data_needed = false;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "actor_animation_blend_space_jobs_tests.rs"]
mod tests;
