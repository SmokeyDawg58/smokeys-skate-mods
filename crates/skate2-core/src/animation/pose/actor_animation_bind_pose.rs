//! Buffered `GetBindpose82d6e548`, selected constant VBR fetch and `ACSpose` DAG.
//! Command emission and job submission remain separate from local execution.
use super::{ActorAnimationConstruction, AnimationInitializeStage};
use crate::{
    actor_face_playback::{
        ActorFaceError, AnimationCommandSetup, AnimationPoseRef, FaceDatabase, FaceDatabaseManager,
    },
    animation::{AnimationMatrix, AnimationSkeleton, LocalPose},
    animation_job_owners::{
        AnimationEntryPoint, AnimationJobCode, AnimationJobManager, AnimationJobScheduler,
        DormantAnimationJob,
        execution::{AnimationExecutionError, AnimationJobWork, AnimationQueuedJob},
    },
    simulation_animation_startup::AnimationStartupSource,
};
use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
use std::{
    mem::MaybeUninit,
    sync::{Arc, Mutex},
};

#[cfg(test)]
#[path = "actor_animation_bind_pose_tests.rs"]
mod tests;

#[path = "actor_animation_pedestrian_command_construction.rs"]
mod pedestrian_command_construction;
#[path = "actor_animation_raw_command_construction.rs"]
mod raw_command_construction;
#[path = "actor_animation_sqt_output.rs"]
pub(crate) mod sqt_output;

#[derive(Debug, PartialEq)]
pub(crate) enum BindPoseError {
    InitializationRequired,
    BufferedOwnerRequired,
    MissingBindPose,
    UnsupportedSetup,
    FixedSqtId { id: u32, maximum: usize },
    ReuseLimitReached,
    ReuseFixedRootsPending,
    DynamicSqtCapacity { next: u16, maximum: usize },
    TreeStackUnderflow(&'static str),
    TreeCoefficientBits(u32),
    AlreadyInCommands,
    Allocation,
    NBlend(u32),
    NBlendListBounds,
    Animation(ActorFaceError),
    Execution(AnimationExecutionError),
    InvalidHierarchy,
}
impl From<ActorFaceError> for BindPoseError {
    fn from(error: ActorFaceError) -> Self {
        Self::Animation(error)
    }
}
impl From<AnimationExecutionError> for BindPoseError {
    fn from(error: AnimationExecutionError) -> Self {
        Self::Execution(error)
    }
}

pub(crate) struct BufferedEntryAllocation {
    pub(crate) manager: Option<Arc<AnimationJobManager>>,
    pub(crate) entry: Option<AnimationEntryPoint>,
}
pub(crate) struct BufferedJobPool {
    pub(crate) jobs: Vec<Arc<Mutex<DormantAnimationJob>>>,
    pub(crate) max_dependencies: usize,
    pub(crate) max_work_units: usize,
    pub(crate) current_job: usize,
}
impl BufferedJobPool {
    pub(crate) fn new(
        scheduler: &AnimationJobScheduler,
        count: usize,
        dependencies: usize,
        work: usize,
    ) -> Self {
        Self {
            jobs: (0..count).map(|_| scheduler.create_job()).collect(),
            max_dependencies: dependencies,
            max_work_units: work,
            current_job: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BindPoseCommand {
    ReuseBuffer {
        segment: usize,
    },
    SetExternalMatrix {
        matrix: u16,
    },
    SetFixedMatrix {
        id: u32,
    },
    PoseMatMult {
        sqt: u16,
        matrix: u16,
    },
    SetExternalSqt {
        sqt: u16,
    },
    ReplaceSqt { first:u16, second:u16, output:u16, offset:u8, count:u8 },
    FetchPose {
        guid: u16,
        sqt: u16,
    },
    SetSqtById {
        id: u16,
        sqt: u16,
    },
    ExtractSqt {
        sqt: u16,
    },
    FetchClip {
        guid: u16,
        sqt: u16,
        timing: crate::actor_face_playback::tree_database::clip_fetch::timing::BufferedFetchTiming,
    },
    NBlend {
        list: u16,
        count: u16,
        output: u16,
    },
    Blend {
        first: u16,
        second: u16,
        output: u16,
        coefficient: u16,
    },
    ChannelBlend {
        channel_from_first: bool,
        first: u16,
        second: u16,
        output: u16,
        coefficient: u16,
    },
    AddSqt {
        first: u16,
        second: u16,
        output: u16,
        channel_from_first: bool,
    },
    Mirror {
        input: u16,
        output: u16,
        trajectory_use: u32,
    },
    UniformScale {
        input: u16, output: u16, coefficient: u16, scale_trajectory: bool, root_bone: u8,
    },
    Pose {
        sqt: u16,
        matrix: u16,
    },
    SqtToLocal {
        sqt: u16,
        matrix: u16,
    },
    LocalToGlobal {
        matrix: u16,
    },
    ExtractMatrix {
        matrix: u16,
    },
}

pub(crate) struct BufferedBodyCommands {
    pub(crate) nblend: super::body_dispatch::tree_commands::nblend::NBlendState,
    pub(crate) hierarchy_owner: Arc<Mutex<FaceDatabase>>,
    pub(crate) database_manager: Arc<Mutex<FaceDatabaseManager>>,
    pub(crate) scheduler: Arc<AnimationJobScheduler>,
    pub(crate) entries: Vec<BufferedEntryAllocation>,
    pub(crate) null_job: Option<Arc<Mutex<DormantAnimationJob>>>,
    pub(crate) body_reuse_jobs: Vec<Arc<Mutex<DormantAnimationJob>>>,
    pub(crate) fetch_pool: Option<BufferedJobPool>,
    pub(crate) blend_pool: Option<BufferedJobPool>,
    pub(crate) set_pool: Option<BufferedJobPool>,
    pub(crate) pose_pool: Option<BufferedJobPool>,
    pub(crate) pose_matrix_pool: Option<BufferedJobPool>,
    pub(crate) max_dynamic_matrices: usize,
    pub(crate) local_pool: Option<BufferedJobPool>,
    pub(crate) global_pool: Option<BufferedJobPool>,
    pub(crate) body_pending: Option<super::body_dispatch::PendingBodyDispatch>,
    pub(crate) sqt_programs:
        Vec<crate::actor_animation_construction::hands_face::sqt_jobs::QueuedSqtProgram>,
    pub(crate) sqt_pending:
        Option<crate::actor_animation_construction::hands_face::sqt_jobs::PendingSqtDispatch>,
    pub(crate) last_body_cull_threshold: Option<f32>,
    pub(crate) extract_memory: Vec<MaybeUninit<u8>>,
    pub(crate) extract_capacity: usize,
    pub(crate) max_extracts_per_job: usize,
    pub(crate) max_extracts_per_fetch: usize,
    pub(crate) max_dynamic_sqts: usize,
    pub(crate) max_pose_operations: usize,
    pub(crate) job_category: u32,
    pub(crate) max_set_replace_sqts: usize,
    pub(crate) commands: Vec<BindPoseCommand>,
    pub(crate) in_command_set: u32,
    pub(crate) in_commands: u32,
    pub(crate) initiated_commands: u32,
    pub(crate) synchronized_sets: u32,
    pub(crate) last_submitted: Vec<BindPoseCommand>,
    pub(crate) pending: Option<Arc<BindPoseWork>>,
    pub(crate) constructed: bool,
}
impl BufferedBodyCommands {
    pub(crate) fn new(
        hierarchy_owner: Arc<Mutex<FaceDatabase>>,
        database_manager: Arc<Mutex<FaceDatabaseManager>>,
        scheduler: Arc<AnimationJobScheduler>,
    ) -> Self {
        Self {
            nblend: super::body_dispatch::tree_commands::nblend::NBlendState::default(),
            hierarchy_owner,
            database_manager,
            scheduler,
            entries: Vec::new(),
            null_job: None,
            body_reuse_jobs: vec![],
            fetch_pool: None,
            blend_pool: None,
            set_pool: None,
            pose_pool: None,
            pose_matrix_pool: None,
            max_dynamic_matrices: 0,
            local_pool: None,
            global_pool: None,
            body_pending: None,
            sqt_programs: Vec::new(),
            sqt_pending: None,
            last_body_cull_threshold: None,
            extract_memory: Vec::new(),
            extract_capacity: 0,
            max_extracts_per_job: 0,
            max_extracts_per_fetch: 0,
            max_dynamic_sqts: 0,
            max_pose_operations: 0,
            job_category: 0,
            max_set_replace_sqts: 0,
            commands: Vec::new(),
            in_command_set: 0,
            in_commands: 0,
            initiated_commands: 0,
            synchronized_sets: 0,
            last_submitted: Vec::new(),
            pending: None,
            constructed: false,
        }
    }

    /// Each native allocation is retained before its following singleton require.
    pub(crate) fn try_construct<S: AnimationStartupSource>(
        &mut self,
        setup: &AnimationCommandSetup,
        source: &mut S,
    ) -> Result<(), crate::simulation_animation_startup::AnimationStartupError<S::Error>> {
        use crate::simulation_animation_startup::AnimationStartupError as Error;
        if setup.characters == 40 {
            return self.try_construct_pedestrian_stream(setup, source);
        }
        if setup.codec == 0 && matches!(setup.characters,3|9|12|20) {
            return self.try_construct_raw_stream(setup, source);
        }
        if setup.codec != 1
            || setup.mode != 1
            || setup.fetch_mode != 1
            || setup.category != 0
            || setup.characters != 1
            || setup.capacities != [200, 100, 0, 0, 2]
        {
            return Err(Error::BindPose(BindPoseError::UnsupportedSetup));
        }
        self.nblend.configure(setup);
        self.load_entry(source, AnimationJobCode::FetchVbr, b"AnimFetchVBR")?;
        let hierarchy = self
            .hierarchy_owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?;
        let parts = hierarchy
            .parsed
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?
            .hierarchy
            .parts
            .len();
        drop(hierarchy);
        self.null_job = Some(self.scheduler.create_job());
        self.load_entry(source, AnimationJobCode::ReuseBuffer, b"ACSreuseBuffer")?;
        self.extract_capacity = 100 * parts;
        self.max_extracts_per_job = 256;
        self.max_extracts_per_fetch = parts;
        // ACS ctor82a321f4..32384: one character, AverageFetch+AverageSetReplace,
        // minimum2, converted to unsigned integer for Data's SQTList capacity.
        let average = setup.averages[1] + setup.averages[2];
        let average = if average < 2.0 { 2.0 } else { average };
        if !average.is_finite() || average >= 4_294_967_296.0 {
            return Err(Error::BindPose(BindPoseError::UnsupportedSetup));
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "native positive bounded fctidz capacity conversion"
        )]
        let dynamic_sqts = average as u32;
        self.max_dynamic_sqts = usize::try_from(dynamic_sqts).map_err(|_| Error::Allocation)?;
        let count = self.extract_capacity.div_ceil(256) + 9;
        self.fetch_pool = Some(BufferedJobPool::new(
            &self.scheduler,
            count,
            1,
            self.extract_capacity,
        ));
        let bytes = (self.extract_capacity * 64 + count * 128).div_ceil(128) * 128 + 256;
        self.extract_memory
            .try_reserve_exact(bytes)
            .map_err(|_| Error::Allocation)?;
        self.extract_memory.resize(bytes, MaybeUninit::uninit());
        self.load_entry(source, AnimationJobCode::SqtBlend, b"ACSsqtBlend")?;
        self.blend_pool = Some(BufferedJobPool::new(&self.scheduler, 18, count + 1, 100));
        self.load_entry(source, AnimationJobCode::SetData, b"ACSsetData")?;
        self.set_pool = Some(BufferedJobPool::new(&self.scheduler, 12, 1, 0));
        self.max_pose_operations = 2;
        self.constructed = true;
        Ok(())
    }
    fn load_entry<S: AnimationStartupSource>(
        &mut self,
        source: &mut S,
        code: AnimationJobCode,
        name: &[u8],
    ) -> Result<(), crate::simulation_animation_startup::AnimationStartupError<S::Error>> {
        self.entries.push(BufferedEntryAllocation {
            manager: None,
            entry: None,
        });
        let allocation = self.entries.last_mut().expect("just retained allocation");
        allocation.manager = Some(
            source
                .require_job_manager()
                .map_err(crate::simulation_animation_startup::AnimationStartupError::Source)?,
        );
        allocation.entry = Some(AnimationEntryPoint::new(Some(code), Some(name)));
        Ok(())
    }
}

/// Allocation becomes observable before `BeginCommandSet`; values become readable
/// after the actual extraction job and Sync, never by interpreting unknown bytes.
pub(crate) struct OwnedAnimationPoseBuffer {
    pub(crate) hierarchy: Option<Arc<Mutex<FaceDatabase>>>,
    storage: Option<Arc<Mutex<PoseOutputAllocation>>>,
    /// Retained fetch evidence, not an additional native `GetBindpose` output.
    pub(crate) local_pose: Option<Vec<AndaleSqt>>,
}
pub(crate) struct PoseOutputAllocation {
    /// None is unknown memory, not a zero or identity matrix.
    pub(crate) values: Vec<Option<AnimationMatrix>>,
    pub(crate) synchronized: bool,
}
impl OwnedAnimationPoseBuffer {
    pub(crate) fn allocate_unknown(
        hierarchy: Arc<Mutex<FaceDatabase>>,
        bones: usize,
    ) -> Result<Self, BindPoseError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(bones)
            .map_err(|_| BindPoseError::Allocation)?;
        values.resize(bones, None);
        Ok(Self {
            hierarchy: Some(hierarchy),
            storage: Some(Arc::new(Mutex::new(PoseOutputAllocation {
                values,
                synchronized: false,
            }))),
            local_pose: None,
        })
    }
    pub(crate) fn shared_storage(&self) -> Result<Arc<Mutex<PoseOutputAllocation>>, BindPoseError> {
        self.storage
            .as_ref()
            .map(Arc::clone)
            .ok_or(BindPoseError::InitializationRequired)
    }
    pub(crate) fn new() -> Self {
        Self {
            hierarchy: None,
            storage: None,
            local_pose: None,
        }
    }
    pub(crate) fn allocation_len(&self) -> Result<usize, BindPoseError> {
        self.storage.as_ref().map_or(Ok(0), |storage| {
            Ok(storage
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .values
                .len())
        })
    }
    /// Reads the actual destination written by the queued pose job. Values are
    /// exposed only after root completion and command-system Sync.
    pub(crate) fn matrices(&self) -> Result<Option<Vec<AnimationMatrix>>, BindPoseError> {
        let Some(storage) = &self.storage else {
            return Ok(None);
        };
        let storage = storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if !storage.synchronized {
            return Ok(None);
        }
        storage
            .values
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .map(Some)
            .ok_or(BindPoseError::MissingBindPose)
    }

    /// `SkaterAnim::GetPhysUpdateData82c97148..725c` modifies the actual
    /// synchronized root matrix in place before publishing either pose buffer.
    pub(crate) fn try_update_root_matrix(
        &self,
        update: impl FnOnce(&mut AnimationMatrix),
    ) -> Result<(), BindPoseError> {
        let storage = self
            .storage
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?;
        let mut storage = storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if !storage.synchronized {
            return Err(BindPoseError::InitializationRequired);
        }
        let root = storage
            .values
            .first_mut()
            .and_then(Option::as_mut)
            .ok_or(BindPoseError::MissingBindPose)?;
        update(root);
        Ok(())
    }
}
pub(crate) struct BindPoseWork {
    pose: AnimationPoseRef,
    hierarchy: AndaleHierarchy,
    output: Arc<Mutex<PoseOutputAllocation>>,
    state: Mutex<BindPoseWorkState>,
}
struct BindPoseWorkState {
    sqt: Option<Vec<AndaleSqt>>,
    internal_matrix: Option<Vec<AnimationMatrix>>,
    failure: Option<Arc<BindPoseError>>,
}
impl AnimationJobWork for BindPoseWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        let result = match code {
            AnimationJobCode::FetchVbr => self
                .pose
                .constant_vbr_sqt_for(&self.hierarchy)
                .map(|pose| state.sqt = Some(pose))
                .map_err(BindPoseError::Animation),
            AnimationJobCode::Pose => {
                let pose = state
                    .sqt
                    .as_deref()
                    .map(LocalPose::from_andale)
                    .ok_or(BindPoseError::MissingBindPose);
                pose.and_then(|pose| {
                    let mut skeleton = AnimationSkeleton::from_andale(&self.hierarchy)
                        .map_err(|_| BindPoseError::InvalidHierarchy)?;
                    // NewACS constructs Skeleton(numBones, trajectory=0, parents).
                    skeleton.trajectory_bone = Some(0);
                    let matrices = skeleton
                        .global_matrices(&pose)
                        .map_err(|_| BindPoseError::InvalidHierarchy)?;
                    // ACSpose extraOutput points at the SAME allocation retained
                    // before BeginCommandSet. Sync changes only readability.
                    let mut output = self.output.lock().map_err(|_| ActorFaceError::Lock)?;
                    if output.values.len() != matrices.len() {
                        return Err(BindPoseError::InvalidHierarchy);
                    }
                    for (destination, matrix) in output.values.iter_mut().zip(&matrices) {
                        *destination = Some(*matrix);
                    }
                    state.internal_matrix = Some(matrices);
                    Ok(())
                })
            }
            _ => Err(BindPoseError::UnsupportedSetup),
        };
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("buffered bind pose")
        })
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ActorBindPoseError<E> {
    Source(E),
    BindPose(BindPoseError),
    Evaluation(Arc<BindPoseError>),
}
impl<E> From<BindPoseError> for ActorBindPoseError<E> {
    fn from(error: BindPoseError) -> Self {
        Self::BindPose(error)
    }
}
impl<E> From<ActorFaceError> for ActorBindPoseError<E> {
    fn from(error: ActorFaceError) -> Self {
        Self::BindPose(error.into())
    }
}
impl<E> From<AnimationExecutionError> for ActorBindPoseError<E> {
    fn from(error: AnimationExecutionError) -> Self {
        Self::BindPose(error.into())
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn try_get_bind_pose<S: super::ActorAnimationInitializeSource>(
        &mut self,
        output: &mut OwnedAnimationPoseBuffer,
        source: &mut S,
    ) -> Result<(), ActorBindPoseError<S::Error>> {
        let initialized = self
            .initialization
            .as_ref()
            .ok_or(BindPoseError::InitializationRequired)?;
        if initialized.stage != AnimationInitializeStage::BodyReturned {
            return Err(BindPoseError::InitializationRequired.into());
        }
        let Some(playback) = &initialized.playback else {
            return Ok(());
        };
        let commands = Arc::clone(&playback.lock().map_err(|_| ActorFaceError::Lock)?.commands);
        let buffered = commands
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .buffered
            .clone()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let pose = initialized
            .bind_pose
            .clone()
            .ok_or(BindPoseError::MissingBindPose)?;
        let mut buffered = buffered.lock().map_err(|_| ActorFaceError::Lock)?;
        let evaluated = &mut self
            .base
            .as_mut()
            .ok_or(BindPoseError::InitializationRequired)?
            .bind_pose_evaluated;
        buffered.try_evaluate(&pose, output, evaluated, source)
    }
}

impl BufferedBodyCommands {
    #[expect(
        clippy::too_many_lines,
        reason = "Keep allocation, command submission, Sync and partial failure state in native order"
    )]
    fn try_evaluate<S: super::ActorAnimationInitializeSource>(
        &mut self,
        pose: &AnimationPoseRef,
        output: &mut OwnedAnimationPoseBuffer,
        evaluated: &mut bool,
        source: &mut S,
    ) -> Result<(), ActorBindPoseError<S::Error>> {
        if !self.constructed {
            return Err(BindPoseError::BufferedOwnerRequired.into());
        }
        if self.in_command_set != 0 || self.in_commands != 0 || self.initiated_commands != 0 {
            return Err(BindPoseError::AlreadyInCommands.into());
        }
        let hierarchy = self
            .hierarchy_owner
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .parsed
            .as_ref()
            .ok_or(ActorFaceError::MissingRecord)?
            .hierarchy
            .clone();
        output.hierarchy = Some(Arc::clone(&self.hierarchy_owner));
        output.local_pose = None;
        let storage = Arc::new(Mutex::new(PoseOutputAllocation {
            values: Vec::new(),
            synchronized: false,
        }));
        output.storage = Some(Arc::clone(&storage));
        {
            let mut allocation = storage.lock().map_err(|_| ActorFaceError::Lock)?;
            allocation
                .values
                .try_reserve_exact(hierarchy.bone_names.len())
                .map_err(|_| BindPoseError::Allocation)?;
            allocation.values.resize(hierarchy.bone_names.len(), None);
        }
        self.in_command_set += 1; // BeginCommandSet.
        self.in_commands += 1; // BeginCommands.
        let guid = pose.guid()?;
        self.commands
            .push(BindPoseCommand::FetchPose { guid, sqt: 0 });
        *evaluated = true; // Pose::Eval writes only after FetchPose returns.
        self.commands
            .push(BindPoseCommand::Pose { sqt: 0, matrix: 0 });
        self.commands
            .push(BindPoseCommand::ExtractMatrix { matrix: 0 });
        self.in_commands -= 1; // EndCommands; command stack now empty.
        // EndCommandSet constructs the terminal root and evaluates the dependency
        // graph. FetchSys resolves the GUID through the actual registered owner.
        let resolved = self
            .database_manager
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .pose_by_guid(guid)?;
        if hierarchy.parts.len() != self.max_extracts_per_fetch || hierarchy.parts.len() >= 256 {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let work = Arc::new(BindPoseWork {
            pose: resolved,
            hierarchy,
            output: Arc::clone(&storage),
            state: Mutex::new(BindPoseWorkState {
                sqt: None,
                internal_matrix: None,
                failure: None,
            }),
        });
        self.pending = Some(Arc::clone(&work));
        if self.pose_pool.is_none() {
            self.entries.push(BufferedEntryAllocation {
                manager: None,
                entry: None,
            });
            let allocation = self
                .entries
                .last_mut()
                .expect("pose entry allocation retained");
            allocation.manager = Some(
                source
                    .require_job_manager()
                    .map_err(ActorBindPoseError::Source)?,
            );
            allocation.entry = Some(AnimationEntryPoint::new(
                Some(AnimationJobCode::Pose),
                Some(b"ACSpose"),
            ));
            self.pose_pool = Some(BufferedJobPool::new(
                &self.scheduler,
                14,
                19,
                self.max_pose_operations / 6 + 1,
            ));
        }
        let root = Arc::clone(
            self.null_job
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        let fetch = self
            .fetch_pool
            .as_mut()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let fetch_job = Arc::clone(&fetch.jobs[fetch.current_job]);
        fetch.current_job += 1;
        let pose_pool = self
            .pose_pool
            .as_mut()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let pose_job = Arc::clone(&pose_pool.jobs[pose_pool.current_job]);
        pose_pool.current_job += 1;
        // Flush configures each current job, but submission occurs only at the
        // terminal null job's ScheduleTree. Extraction is fused into ACSpose.
        for (job, code, name) in [
            (&root, None, b"".as_slice()),
            (
                &pose_job,
                Some(AnimationJobCode::Pose),
                b"ACSpose".as_slice(),
            ),
            (
                &fetch_job,
                Some(AnimationJobCode::FetchVbr),
                b"AnimFetchVBR".as_slice(),
            ),
        ] {
            let mut job = job.lock().map_err(|_| ActorFaceError::Lock)?;
            job.entry_point = AnimationEntryPoint::new(code, Some(name));
            job.entry_point.affinity = 1; // SetCategoryForJob(category0).
        }
        let queue = [
            AnimationQueuedJob {
                job: Arc::clone(&root),
                dependencies: vec![1],
                work: None,
            },
            AnimationQueuedJob {
                job: pose_job,
                dependencies: vec![2],
                work: Some(Arc::clone(&work) as Arc<dyn AnimationJobWork>),
            },
            AnimationQueuedJob {
                job: fetch_job,
                dependencies: Vec::new(),
                work: Some(Arc::clone(&work) as Arc<dyn AnimationJobWork>),
            },
        ];
        let manager = source
            .require_job_manager()
            .map_err(ActorBindPoseError::Source)?;
        let backend = manager
            .scheduler
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        backend.submit_bind_pose(&queue)?;
        self.last_submitted.clone_from(&self.commands);
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        // Sync checks the live manager too; an unrelated replacement backend is
        // outside the supported waiting path and cannot drain this queue.
        let wait_manager = source
            .require_job_manager()
            .map_err(ActorBindPoseError::Source)?;
        if !wait_manager
            .scheduler
            .as_ref()
            .is_some_and(|waiting| Arc::ptr_eq(waiting, backend))
        {
            return Err(AnimationExecutionError::MissingBackend.into());
        }
        if let Err(error) = backend.wait(&root) {
            if let Some(failure) = work
                .state
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .failure
                .clone()
            {
                return Err(ActorBindPoseError::Evaluation(failure));
            }
            return Err(error.into());
        }
        let mut state = work.state.lock().map_err(|_| ActorFaceError::Lock)?;
        output.local_pose = state.sqt.take();
        drop(state);
        backend.release_completed()?;
        // ReInitForNewCommandSet clears jobs/commands only after completion.
        for queued in &queue {
            queued
                .job
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .clear_after_sync();
        }
        self.fetch_pool
            .as_mut()
            .expect("constructed fetch")
            .current_job = 0;
        self.pose_pool
            .as_mut()
            .expect("constructed pose")
            .current_job = 0;
        self.commands.clear();
        self.pending = None;
        self.initiated_commands -= 1;
        self.synchronized_sets += 1;
        storage
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .synchronized = true;
        Ok(())
    }
}
