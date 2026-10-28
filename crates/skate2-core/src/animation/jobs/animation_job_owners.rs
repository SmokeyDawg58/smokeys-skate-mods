//! Dormant native animation job and entry-point construction.
//! References: command manager824d56b8, job manager826e33f8, `CreateJob`824d4cf0,
//! entry point829ad7a8, Job829ae658/Clear829ad7e0. No scheduler execution here.
#![expect(
    dead_code,
    reason = "retained job construction precedes animation scheduling integration"
)]
use std::sync::{Arc, Mutex, Weak};

#[cfg(test)]
#[path = "animation_job_owners_tests.rs"]
mod tests;

#[path = "animation_job_execution.rs"]
pub(crate) mod execution;
pub(crate) use execution::AnimationSchedulerBackend;
pub(crate) struct AnimationJobManager {
    pub(crate) scheduler: Option<Arc<AnimationSchedulerBackend>>,
}
impl AnimationJobManager {
    pub(crate) fn new() -> Self {
        Self { scheduler: None }
    }
}

/// Native `AnimJobScheduler` is a vtable-only implementation of `IJobScheduler`.
pub(crate) struct AnimationJobScheduler;
impl AnimationJobScheduler {
    pub(crate) fn new() -> Self {
        Self
    }
    /// Allocates and constructs a new dormant Job; no submission is performed.
    /// Reference: 824d4cf0 then829ae658/829ad7e0.
    #[expect(
        clippy::unused_self,
        reason = "source virtual allocator method operates through the retained scheduler identity"
    )]
    pub(crate) fn create_job(&self) -> Arc<Mutex<DormantAnimationJob>> {
        Arc::new(Mutex::new(DormantAnimationJob::new()))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimationJobCode {
    BlendSpace,
    FetchRaw,
    FetchVbr,
    Pose,
    PoseMatMult,
    SqtToLocal,
    LocalToGlobal,
    ReuseBuffer,
    SqtBlend,
    SetData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnimationEntryPoint {
    /// Bytes after the C string terminator are not observed or initialized here.
    pub(crate) name: Vec<u8>,
    pub(crate) priority: u32,
    pub(crate) affinity: u32,
    pub(crate) environment: u32,
    pub(crate) kernel_swap: u32,
    pub(crate) code_recycle: u32,
    pub(crate) break_on_entry: bool,
    pub(crate) allow_sleep: bool,
    pub(crate) code: Option<AnimationJobCode>,
}
impl AnimationEntryPoint {
    pub(crate) fn new(code: Option<AnimationJobCode>, name: Option<&[u8]>) -> Self {
        let name = name.unwrap_or_default();
        let end = name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(name.len())
            .min(15);
        Self {
            name: name[..end].to_vec(),
            priority: 128,
            affinity: 63,
            environment: 0,
            kernel_swap: 0,
            code_recycle: 1,
            break_on_entry: false,
            allow_sleep: false,
            code,
        }
    }
}

pub(crate) struct AnimationJobHandle {
    pub(crate) submission_id: u64,
    pub(crate) backend: Option<Arc<AnimationSchedulerBackend>>,
    pub(crate) index: u16,
    pub(crate) padding: u16,
}
impl AnimationJobHandle {
    fn new() -> Self {
        Self {
            submission_id: 0,
            backend: None,
            index: 0,
            padding: 0,
        }
    }
}
pub(crate) struct AnimationJobDependency {
    pub(crate) job: Option<Arc<Mutex<DormantAnimationJob>>>,
    pub(crate) handle: AnimationJobHandle,
    pub(crate) trigger: Option<u32>,
}
pub(crate) struct AnimationJobEvent {
    pub(crate) kind: u32,
    /// Event's unions and decrementer pointer remain unwritten when kind=NONE.
    pub(crate) payload: [Option<u32>; 3],
}
impl AnimationJobEvent {
    fn new() -> Self {
        Self {
            kind: 0,
            payload: [None; 3],
        }
    }
}
pub(crate) struct DormantAnimationJob {
    pub(crate) seen: bool,
    pub(crate) entry_point: AnimationEntryPoint,
    pub(crate) parameters: [u32; 4],
    pub(crate) handle: AnimationJobHandle,
    pub(crate) has_job_dependency: bool,
    pub(crate) dependencies: [AnimationJobDependency; 10],
    pub(crate) dependency_count: usize,
    pub(crate) dependency_extensions: Vec<Vec<AnimationJobDependency>>,
    /// None represents unwritten storage. Populated native back pointers are
    /// non-owning; Weak preserves their lifetime without making an Arc cycle.
    pub(crate) dependents: [Option<Weak<Mutex<DormantAnimationJob>>>; 6],
    pub(crate) dependent_count: usize,
    pub(crate) dependent_extensions: Vec<Vec<Weak<Mutex<DormantAnimationJob>>>>,
    pub(crate) events: [[AnimationJobEvent; 10]; 2],
    pub(crate) event_counts: [usize; 2],
    pub(crate) event_extensions: [Vec<Vec<AnimationJobEvent>>; 2],
    pub(crate) start_event: AnimationJobEvent,
}
impl DormantAnimationJob {
    /// Clear829ad7e0 resets the entry/parameters/counts, but retains the last
    /// instance handle and start event. Inactive pointer slots are not read.
    pub(crate) fn clear_after_sync(&mut self) {
        self.entry_point = AnimationEntryPoint::new(None, None);
        self.parameters = [0; 4];
        self.seen = false;
        self.has_job_dependency = false;
        self.dependency_count = 0;
        self.dependent_count = 0;
        self.event_counts = [0; 2];
        for dependency in &mut self.dependencies {
            dependency.job = None;
        }
        self.dependents.fill(None);
        self.dependency_extensions.clear();
        self.dependent_extensions.clear();
        for events in &mut self.event_extensions {
            events.clear();
        }
    }
    pub(crate) fn new() -> Self {
        Self {
            seen: false,
            entry_point: AnimationEntryPoint::new(None, None),
            parameters: [0; 4],
            handle: AnimationJobHandle::new(),
            has_job_dependency: false,
            dependencies: std::array::from_fn(|_| AnimationJobDependency {
                job: None,
                handle: AnimationJobHandle::new(),
                trigger: None,
            }),
            dependency_count: 0,
            dependency_extensions: Vec::new(),
            dependents: std::array::from_fn(|_| None),
            dependent_count: 0,
            dependent_extensions: Vec::new(),
            events: std::array::from_fn(|_| std::array::from_fn(|_| AnimationJobEvent::new())),
            event_counts: [0; 2],
            event_extensions: std::array::from_fn(|_| Vec::new()),
            start_event: AnimationJobEvent::new(),
        }
    }
}
