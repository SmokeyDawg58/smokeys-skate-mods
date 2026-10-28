//! Actual hand/face buffered SQT work reusing body fetch/operation execution.
//! Native extraction is attached to its producing fetch/blend or fixed copy.
use super::super::initialization::{
    bind_pose::{BindPoseCommand, BindPoseError, BufferedBodyCommands},
    body_dispatch::tree_commands::{self, Fetch, TreeCommands},
};
use super::runtime::HandFaceRuntimeError;
use crate::{
    actor_animation_construction::initialization::ActorAnimationInitializeSource,
    actor_face_playback::ActorFaceError,
    animation_job_owners::{
        AnimationEntryPoint, AnimationJobCode,
        execution::{
            AnimationExecutionError, AnimationJobWork, AnimationQueuedJob,
            AnimationSchedulerBackend,
        },
    },
};
use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
use std::sync::{Arc, Mutex};

#[path = "actor_animation_matrix_program.rs"]
pub(crate) mod matrix;
#[path = "../runtime/actor_animation_reuse_sqt_jobs.rs"]
mod reuse_jobs;
#[path = "actor_animation_pedestrian_batch_jobs.rs"]
mod pedestrian_batches;
use matrix::ProgramOutput;

pub(crate) struct QueuedSqtProgram {
    pub(crate) commands: TreeCommands,
    pub(crate) output: ProgramOutput,
    pub(crate) hierarchy: Arc<AndaleHierarchy>,
}
impl QueuedSqtProgram {
    fn extraction_job(&self) -> Result<Option<AnimationJobCode>, BindPoseError> {
        if matches!(self.output, ProgramOutput::Matrix(_)) {
            return Ok(Some(AnimationJobCode::PoseMatMult));
        }
        if self.commands.output_is_fixed()? {
            return Ok(None);
        }
        let output = self.commands.output()?;
        if let Some(operation) = self.commands.operations.last() {
            let actual = match operation {
                BindPoseCommand::NBlend { output, .. }
                | BindPoseCommand::Blend { output, .. }
                | BindPoseCommand::ChannelBlend { output, .. }
                | BindPoseCommand::AddSqt { output, .. }
                | BindPoseCommand::ReplaceSqt { output, .. } => *output,
                _ => return Err(BindPoseError::UnsupportedSetup),
            };
            if actual == output {
                return Ok(Some(AnimationJobCode::SqtBlend));
            }
        } else if self
            .commands
            .fetches
            .iter()
            .any(|(slot, _)| *slot == output)
        {
            return Ok(Some(self.commands.fetch_code));
        }
        if self
            .commands
            .external_sqts
            .iter()
            .any(|(slot, _)| *slot == output)
        {
            return Ok(Some(AnimationJobCode::SetData));
        }
        Err(BindPoseError::UnsupportedSetup)
    }
    fn extract(&self, sqts: &[Option<Vec<AndaleSqt>>]) -> Result<(), BindPoseError> {
        let values = self.commands.operand(self.commands.output()?, sqts)?;
        self.output.extract_sqt(&values)?;
        Ok(())
    }
}
pub(crate) struct SqtDispatchWork {
    pub(crate) programs: Vec<QueuedSqtProgram>,
    state: Mutex<SqtDispatchWorkState>,
}
pub(crate) struct PendingSqtDispatch {
    work: Arc<SqtDispatchWork>,
    jobs: Vec<AnimationQueuedJob>,
    backend: Option<Arc<AnimationSchedulerBackend>>,
}
struct SqtDispatchWorkState {
    sqts: Vec<Vec<Option<Vec<AndaleSqt>>>>,
    matrices: Vec<Option<Vec<crate::animation::AnimationMatrix>>>,
    failure: Option<Arc<BindPoseError>>,
}
impl SqtDispatchWork {
    pub(crate) fn new(programs: Vec<QueuedSqtProgram>) -> Self {
        let sqts = programs
            .iter()
            .map(|program| vec![None; program.commands.dynamic_count()])
            .collect();
        let matrices = vec![None; programs.len()];
        Self {
            programs,
            state: Mutex::new(SqtDispatchWorkState {
                sqts,
                matrices,
                failure: None,
            }),
        }
    }
    pub(crate) fn failure(&self) -> Result<Option<Arc<BindPoseError>>, AnimationExecutionError> {
        Ok(self
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?
            .failure
            .clone())
    }
    pub(crate) fn has_set(&self) -> bool {
        self.programs
            .iter()
            .any(|p| !p.commands.external_sqts.is_empty()
                || matches!(&p.output, ProgramOutput::Matrix(matrix) if !matrix.fixed))
    }
    pub(crate) fn has_matrix_pose(&self) -> bool {
        self.programs
            .iter()
            .any(|p| matches!(p.output, ProgramOutput::Matrix(_)))
    }
    pub(crate) fn has_fetch(&self) -> bool {
        self.programs
            .iter()
            .any(|program| !program.commands.fetches.is_empty())
    }
    pub(crate) fn has_blend(&self) -> bool {
        self.programs
            .iter()
            .any(|program| !program.commands.operations.is_empty())
    }
    // Schedule82a32cc8..32d18 copies a fixed result directly while scheduling.
    fn extract_fixed_outputs(&self) -> Result<(), BindPoseError> {
        for program in &self.programs {
            if program.extraction_job()?.is_none() {
                program.extract(&[])?;
            }
        }
        Ok(())
    }
    pub(crate) fn complete_sync(&self) -> Result<(), BindPoseError> {
        for program in &self.programs {
            program.output.complete_sync()?;
        }
        Ok(())
    }
}
impl AnimationJobWork for SqtDispatchWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        let result = (|| {
            let SqtDispatchWorkState { sqts, matrices, .. } = &mut *state;
            for ((program, sqts), matrices) in self.programs.iter().zip(sqts).zip(matrices) {
                match code {
                    AnimationJobCode::SetData => {
                        for (slot, source) in &program.commands.external_sqts {
                            let values = source.try_read_for_set_job()?;
                            if !program.commands.accepts_external_sqt_size(*slot,values.len(),program.hierarchy.parents.len()) {
                                return Err(BindPoseError::InvalidHierarchy);
                            }
                            *sqts
                                .get_mut(usize::from(*slot))
                                .ok_or(BindPoseError::UnsupportedSetup)? = Some(values);
                        }
                        if let ProgramOutput::Matrix(output) = &program.output {
                            if output.source.len() != program.hierarchy.parents.len() {
                                return Err(BindPoseError::InvalidHierarchy);
                            }
                            *matrices = Some(output.source.to_vec());
                        }
                    }
                    AnimationJobCode::PoseMatMult => {
                        let ProgramOutput::Matrix(output) = &program.output else {
                            continue;
                        };
                        let values = program.commands.operand(program.commands.output()?, sqts)?;
                        output.execute(
                            &program.hierarchy,
                            &values,
                            if output.fixed { &output.source } else {
                                matrices.as_deref().ok_or(BindPoseError::MissingBindPose)?
                            },
                        )?;
                    }
                    AnimationJobCode::FetchRaw | AnimationJobCode::FetchVbr => {
                        for (slot, fetch) in &program.commands.fetches {
                            if program.commands.fetch_code != code {
                                return Err(BindPoseError::UnsupportedSetup);
                            }
                            let values = match (code, fetch) {
                                (AnimationJobCode::FetchRaw, Fetch::Pose(pose)) => pose.raw_sqt_for(&program.hierarchy)?,
                                (AnimationJobCode::FetchVbr, Fetch::Pose(pose)) => pose.constant_vbr_sqt_for(&program.hierarchy)?,
                                (AnimationJobCode::FetchRaw|AnimationJobCode::FetchVbr, Fetch::Clip(plan)) => tree_commands::execute_fetch(plan)?,
                                _ => return Err(BindPoseError::UnsupportedSetup),
                            };
                            *sqts
                                .get_mut(usize::from(*slot))
                                .ok_or(BindPoseError::UnsupportedSetup)? = Some(values);
                        }
                    }
                    AnimationJobCode::SqtBlend => tree_commands::execute_operations(
                        &program.commands,
                        sqts,
                        &program.hierarchy,
                    )?,
                    _ => return Err(BindPoseError::UnsupportedSetup),
                }
                // Schedule forwards the ExtractData parent destination into
                // the actual producing engine, with no separate copy job.
                if code != AnimationJobCode::PoseMatMult && program.extraction_job()? == Some(code)
                {
                    program.extract(sqts)?;
                }
            }
            Ok(())
        })();
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("buffered hand/face SQT")
        })
    }
}

impl BufferedBodyCommands {
    /// `EndCommands` retains completed stream graphs for the later `EndCommandSet`.
    pub(crate) fn retain_sqt_program(
        &mut self,
        program: QueuedSqtProgram,
    ) -> Result<(), BindPoseError> {
        if self.in_commands == 0 || self.in_command_set == 0 {
            return Err(BindPoseError::AlreadyInCommands);
        }
        // EndCommands82a289e0 preserves mCurrMax between character streams.
        // Streams begin at the prior current allocation, including after its
        // reuse boundary. The pedestrian dispatcher retains those dependencies.
        if let Some(previous) = self.sqt_programs.last() {
            if program.commands.allocation_start != previous.commands.allocation_end()
                || !Arc::ptr_eq(&previous.commands.owner, &program.commands.owner) {
                return Err(BindPoseError::UnsupportedSetup);
            }
        }
        self.in_commands -= 1;
        self.sqt_programs.push(program);
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "ordered shared pool and job dependency publication"
    )]
    pub(crate) fn submit_sqt_programs<S: ActorAnimationInitializeSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        if self.in_command_set != 1 || self.in_commands != 0 || self.sqt_pending.is_some() {
            return Err(BindPoseError::AlreadyInCommands.into());
        }
        let work = Arc::new(SqtDispatchWork::new(std::mem::take(&mut self.sqt_programs)));
        work.extract_fixed_outputs()?;
        let has_matrix_pose = work.has_matrix_pose();
        let has_blend = work.has_blend();
        let has_fetch = work.has_fetch();
        let has_set = work.has_set();
        let fetch_code = work.programs.iter().find(|p| !p.commands.fetches.is_empty())
            .or_else(|| work.programs.first())
            .map_or(AnimationJobCode::FetchRaw, |p| p.commands.fetch_code);
        if work.programs.iter().any(|p| !p.commands.fetches.is_empty() && p.commands.fetch_code != fetch_code) {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let pedestrian_batches = (fetch_code == AnimationJobCode::FetchVbr
            && work.programs.iter().all(|p| matches!(&p.output, ProgramOutput::Matrix(matrix) if matrix.fixed)))
            || (fetch_code==AnimationJobCode::FetchRaw&&self.max_dynamic_sqts==9&&self.job_category==2
                &&work.programs.iter().all(|p|matches!(p.output,ProgramOutput::Matrix(_))));
        if fetch_code==AnimationJobCode::FetchRaw&&self.job_category==2
            &&self.max_set_replace_sqts==0&&matches!(self.max_dynamic_sqts,24|40)
            &&work.programs.iter().all(|p|matches!(p.output,ProgramOutput::Sqt(_))){
            return self.submit_pedestrian_batches(work,source);
        }
        if !pedestrian_batches && work.programs.iter().any(|p| p.commands.has_reuse()) {
            return self.submit_reused_sqt(work, source);
        }
        let extracts: usize = work.programs.iter().flat_map(|p| p.commands.fetches.iter())
            .map(|(_, fetch)| match fetch { Fetch::Pose(_) => self.max_extracts_per_fetch,
                Fetch::Clip(plan) => plan.extract_count() }).sum();
        let blends: usize = work.programs.iter().map(|p| p.commands.operations.len()).sum();
        if !pedestrian_batches && (extracts > self.extract_capacity || extracts >= self.max_extracts_per_job
            || (has_blend && blends > self.blend_pool.as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?.max_work_units)) {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        // Existing dispatcher is a chain. Mixed independent fetch/set graphs
        // require their actual native dependencies before host admission.
        if !pedestrian_batches && has_set && has_fetch {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let root = Arc::clone(
            self.null_job
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        let mut selected = vec![(root, None, b"".as_slice())];
        if has_matrix_pose {
            if self.pose_matrix_pool.is_none() {
                self.entries.push(
                    super::super::initialization::bind_pose::BufferedEntryAllocation {
                        manager: None,
                        entry: None,
                    },
                );
                let entry = self.entries.last_mut().expect("retained matrix pose entry");
                entry.manager = Some(
                    source
                        .require_job_manager()
                        .map_err(HandFaceRuntimeError::Source)?,
                );
                entry.entry = Some(AnimationEntryPoint::new(
                    Some(AnimationJobCode::PoseMatMult),
                    Some(b"ACSposeMatMult"),
                ));
                self.pose_matrix_pool = Some(
                    super::super::initialization::bind_pose::BufferedJobPool::new(
                        &self.scheduler,
                        14,
                        19,
                        self.max_pose_operations / 6 + 1,
                    ),
                );
            }
            if pedestrian_batches {
                return self.submit_pedestrian_batches(work, source);
            }
            let poses = work.programs.iter().filter(|p| matches!(p.output, ProgramOutput::Matrix(_))).count();
            if poses > self.pose_matrix_pool.as_ref().expect("retained matrix pool").max_work_units {
                return Err(BindPoseError::UnsupportedSetup.into());
            }
            selected.push((
                next_job(
                    self.pose_matrix_pool
                        .as_mut()
                        .expect("retained matrix pool"),
                )?,
                Some(AnimationJobCode::PoseMatMult),
                b"ACSposeMatMult".as_slice(),
            ));
        }
        if has_blend {
            selected.push((
                next_job(
                    self.blend_pool
                        .as_mut()
                        .ok_or(BindPoseError::BufferedOwnerRequired)?,
                )?,
                Some(AnimationJobCode::SqtBlend),
                b"ACSsqtBlend",
            ));
        }
        if has_fetch {
            selected.push((
                next_job(
                    self.fetch_pool
                        .as_mut()
                        .ok_or(BindPoseError::BufferedOwnerRequired)?,
                )?,
                Some(fetch_code),
                if fetch_code == AnimationJobCode::FetchVbr { b"AnimFetchVBR".as_slice() } else { b"AnimFetch".as_slice() },
            ));
        }
        if has_set {
            selected.push((
                next_job(
                    self.set_pool
                        .as_mut()
                        .ok_or(BindPoseError::BufferedOwnerRequired)?,
                )?,
                Some(AnimationJobCode::SetData),
                b"ACSsetData",
            ));
        }
        let last = selected.len() - 1;
        let mut jobs = Vec::with_capacity(selected.len());
        for (index, (job, code, name)) in selected.into_iter().enumerate() {
            {
                let mut job = job.lock().map_err(|_| ActorFaceError::Lock)?;
                job.entry_point = AnimationEntryPoint::new(code, Some(name));
                job.entry_point.affinity =
                    crate::animation_job_owners::execution::affinity::selected_category_affinity(
                        self.job_category,
                    )?;
                if matches!(
                    code,
                    Some(
                        AnimationJobCode::SqtBlend
                            | AnimationJobCode::SetData
                            | AnimationJobCode::PoseMatMult
                    )
                ) {
                    job.parameters[2] = 16;
                }
            }
            jobs.push(AnimationQueuedJob {
                job,
                dependencies: if index == last {
                    vec![]
                } else {
                    vec![index + 1]
                },
                work: code.map(|_| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
            });
        }
        self.sqt_pending = Some(PendingSqtDispatch {
            work,
            jobs,
            backend: None,
        });
        let manager = source
            .require_job_manager()
            .map_err(HandFaceRuntimeError::Source)?;
        let backend = manager
            .scheduler
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        let pending = self.sqt_pending.as_mut().expect("retained SQT work");
        backend.submit_sqt_matrix_streams_with_fetch(
            &pending.jobs,
            has_blend,
            has_fetch,
            has_set,
            has_matrix_pose,
            fetch_code,
        )?;
        pending.backend = Some(Arc::clone(backend));
        self.last_submitted.clone_from(&self.commands);
        self.nblend.end_command_set();
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        Ok(())
    }

    pub(crate) fn sync_sqt_programs<S: ActorAnimationInitializeSource>(
        &mut self,
        source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        if self.initiated_commands == 0 {
            return Ok(());
        }
        let pending = self
            .sqt_pending
            .as_ref()
            .ok_or(BindPoseError::BufferedOwnerRequired)?;
        let backend = pending
            .backend
            .as_ref()
            .ok_or(AnimationExecutionError::MissingBackend)?;
        let manager = source
            .require_job_manager()
            .map_err(HandFaceRuntimeError::Source)?;
        if !manager
            .scheduler
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, backend))
        {
            return Err(AnimationExecutionError::MissingBackend.into());
        }
        if let Err(error) = backend.wait_in_category(&pending.jobs[0].job, self.job_category) {
            if let Some(error) = pending.work.failure()? {
                return Err(HandFaceRuntimeError::Evaluation(error));
            }
            return Err(error.into());
        }
        backend.release_selected(&pending.jobs)?;
        for job in &pending.jobs {
            job.job
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .clear_after_sync();
        }
        self.fetch_pool
            .as_mut()
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .current_job = 0;
        self.blend_pool
            .as_mut()
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .current_job = 0;
        self.set_pool
            .as_mut()
            .ok_or(BindPoseError::BufferedOwnerRequired)?
            .current_job = 0;
        if let Some(pool) = &mut self.pose_matrix_pool {
            pool.current_job = 0;
        }
        self.commands.clear();
        self.nblend.synchronized();
        self.initiated_commands -= 1;
        self.synchronized_sets += 1;
        pending.work.complete_sync()?;
        self.sqt_pending = None;
        Ok(())
    }
}

fn next_job(
    pool: &mut super::super::initialization::bind_pose::BufferedJobPool,
) -> Result<Arc<Mutex<crate::animation_job_owners::DormantAnimationJob>>, BindPoseError> {
    let job = pool
        .jobs
        .get(pool.current_job)
        .cloned()
        .ok_or(BindPoseError::UnsupportedSetup)?;
    pool.current_job += 1;
    Ok(job)
}

#[cfg(test)]
use super::super::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;
#[cfg(test)]
#[path = "actor_animation_sqt_dispatch_jobs_tests.rs"]
mod tests;
