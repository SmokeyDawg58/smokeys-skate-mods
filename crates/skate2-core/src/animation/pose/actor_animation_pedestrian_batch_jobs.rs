//! Native bounded engine batches: Fetch82a30458, SQT82a2b0d0,
//! PoseWithMatMult82a2add0, Schedule82a32800. No separate pedestrian scheduler.
use super::{ActorAnimationInitializeSource, ActorFaceError, AnimationEntryPoint,
    AnimationExecutionError, AnimationJobCode, AnimationJobWork, AnimationQueuedJob,
    Arc, BindPoseError, BufferedBodyCommands, Fetch, HandFaceRuntimeError,
    PendingSqtDispatch, ProgramOutput, SqtDispatchWork, next_job, tree_commands};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
enum Operation { Set(usize, usize), SetMatrix(usize), Fetch(usize, usize), Blend(usize, usize), Pose(usize), Reuse(usize, usize) }
impl Operation {
    fn program(self) -> usize { match self { Self::Set(p, _) | Self::SetMatrix(p) | Self::Fetch(p, _) | Self::Blend(p, _) | Self::Pose(p) | Self::Reuse(p, _) => p } }
}
struct BatchWork { work: Arc<SqtDispatchWork>, operations: Vec<Operation> }
impl AnimationJobWork for BatchWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self.work.state.lock().map_err(|_| AnimationExecutionError::Lock)?;
        let result = (|| {
            for operation in &self.operations {
                let index = operation.program();
                let program = &self.work.programs[index];
                let super::SqtDispatchWorkState{sqts,matrices,..}=&mut *state;
                let sqts = &mut sqts[index];
                match (*operation, code) {
                    (Operation::SetMatrix(_),AnimationJobCode::SetData)=>{
                        let ProgramOutput::Matrix(matrix)=&program.output else{return Err(BindPoseError::UnsupportedSetup);};
                        matrices[index]=Some(matrix.source.to_vec());
                    }
                    (Operation::Reuse(_, segment), AnimationJobCode::ReuseBuffer) => {
                        tree_commands::reuse::execute(&program.commands.segments()[segment].copies,
                            sqts, program.hierarchy.parents.len())?;
                    }
                    (Operation::Set(_, source), AnimationJobCode::SetData) => {
                        let (slot, source) = &program.commands.external_sqts[source];
                        let values = source.try_read_for_set_job()?;
                        if !program.commands.accepts_external_sqt_size(*slot,values.len(),program.hierarchy.parents.len()) { return Err(BindPoseError::InvalidHierarchy); }
                        *sqts.get_mut(usize::from(*slot)).ok_or(BindPoseError::UnsupportedSetup)? = Some(values);
                    }
                    (Operation::Fetch(_, fetch), AnimationJobCode::FetchVbr|AnimationJobCode::FetchRaw) => {
                        let (slot, fetch) = &program.commands.fetches[fetch];
                        let values = match fetch {
                            Fetch::Pose(pose) => if code==AnimationJobCode::FetchRaw{pose.raw_sqt_for(&program.hierarchy)?}
                                else{pose.constant_vbr_sqt_for(&program.hierarchy)?},
                            Fetch::Clip(plan) => tree_commands::execute_fetch(plan)?,
                        };
                        *sqts.get_mut(usize::from(*slot)).ok_or(BindPoseError::UnsupportedSetup)? = Some(values);
                        if matches!(program.output,ProgramOutput::Sqt(_))&&program.commands.operations.is_empty()
                            &&*slot==program.commands.output()?{program.extract(sqts)?;}
                    }
                    (Operation::Blend(_, operation), AnimationJobCode::SqtBlend) => {
                        tree_commands::execute_operation_range(&program.commands, sqts,
                            &program.hierarchy, operation..operation + 1)?;
                        if operation + 1 == program.commands.operations.len() {
                            if matches!(program.output,ProgramOutput::Sqt(_)){program.extract(sqts)?;}
                            if let ProgramOutput::Matrix(matrix) = &program.output {
                                if let Some(output) = &matrix.sqt_extract {
                                    let values = program.commands.operand(program.commands.output()?, sqts)?;
                                    output.extract(&values)?;
                                }
                            }
                        }
                    }
                    (Operation::Pose(_), AnimationJobCode::PoseMatMult) => {
                        let ProgramOutput::Matrix(matrix) = &program.output else { return Err(BindPoseError::UnsupportedSetup); };
                        let values = program.commands.operand(program.commands.output()?, sqts)?;
                        matrix.execute(&program.hierarchy, &values, if matrix.fixed{&matrix.source}else{
                            matrices[index].as_deref().ok_or(BindPoseError::MissingBindPose)?})?;
                    }
                    _ => return Err(BindPoseError::UnsupportedSetup),
                }
            }
            Ok(())
        })();
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("pedestrian ACS engine batch")
        })
    }
}

struct Batch { code: AnimationJobCode, operations: Vec<Operation>, dependencies: BTreeSet<usize> }
struct Piece { program: usize, segment: tree_commands::reuse::Segment, final_segment: bool }
struct Epoch { pieces: Vec<Piece>, boundary: Option<(usize, usize)> }
impl BufferedBodyCommands {
    pub(super) fn submit_pedestrian_batches<S: ActorAnimationInitializeSource>(
        &mut self, work: Arc<SqtDispatchWork>, source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        let blend_limit = self.blend_pool.as_ref().ok_or(BindPoseError::BufferedOwnerRequired)?.max_work_units;
        let has_pose=work.has_matrix_pose();
        let fetch_code=work.programs.first().map_or(AnimationJobCode::FetchVbr,|p|p.commands.fetch_code);
        let pose_limit = self.pose_matrix_pool.as_ref().map_or(0,|pool|pool.max_work_units);
        let set_limit = self.set_pool.as_ref().ok_or(BindPoseError::BufferedOwnerRequired)?.max_work_units;
        if blend_limit == 0 || (has_pose&&pose_limit == 0) || (work.has_set()&&set_limit == 0) || self.max_extracts_per_fetch > self.max_extracts_per_job {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        // GetNextLoc's seven command-set boundaries partition SAME-owner
        // allocations. Every prior producer/extractor finishes before reuse.
        let mut epochs = Vec::new();
        let mut pieces = Vec::new();
        for (program, retained) in work.programs.iter().enumerate() {
            let segments = retained.commands.segments();
            let count = segments.len();
            for (index, segment) in segments.into_iter().enumerate() {
                let final_segment = index + 1 == count;
                pieces.push(Piece { program, segment, final_segment });
                if !final_segment {
                    epochs.push(Epoch { pieces: std::mem::take(&mut pieces), boundary: Some((program, index)) });
                }
            }
        }
        epochs.push(Epoch { pieces, boundary: None });
        let mut batches: Vec<Batch> = Vec::new();
        let mut gate = None;
        let mut pose_batches = BTreeSet::new();
        let mut extracts = 0;
        for epoch in epochs {
            let epoch_start = batches.len();
            let mut set_dependencies = vec![BTreeSet::new(); work.programs.len()];
            let mut fetch_dependencies = vec![BTreeSet::new(); work.programs.len()];
            let mut latest_blend = vec![None; work.programs.len()];
            let mut stage_start = batches.len();
            for piece in &epoch.pieces {
                for source in piece.segment.external_sqts.clone() {
                    if batches.len() == stage_start || batches.last().is_some_and(|b| b.operations.len() == set_limit) {
                        batches.push(Batch { code: AnimationJobCode::SetData, operations: Vec::new(),
                            dependencies: gate.into_iter().collect() });
                    }
                    let batch = batches.len() - 1;
                    batches[batch].operations.push(Operation::Set(piece.program, source));
                    set_dependencies[piece.program].insert(batch);
                }
                if piece.final_segment&&matches!(&work.programs[piece.program].output,ProgramOutput::Matrix(matrix) if !matrix.fixed){
                    if batches.len()==stage_start||batches.last().is_some_and(|b|b.operations.len()==set_limit){
                        batches.push(Batch{code:AnimationJobCode::SetData,operations:Vec::new(),dependencies:gate.into_iter().collect()});
                    }
                    let batch=batches.len()-1;batches[batch].operations.push(Operation::SetMatrix(piece.program));
                    set_dependencies[piece.program].insert(batch);
                }
            }
            stage_start = batches.len();
            let mut batch_extracts = 0;
            for piece in &epoch.pieces {
                let program = &work.programs[piece.program];
                for fetch in piece.segment.fetches.clone() {
                    let count = match &program.commands.fetches[fetch].1 {
                        Fetch::Pose(_) => self.max_extracts_per_fetch, Fetch::Clip(plan) => plan.extract_count(),
                    };
                    if count > self.max_extracts_per_fetch { return Err(BindPoseError::UnsupportedSetup.into()); }
                    if batches.len() == stage_start || batch_extracts + self.max_extracts_per_fetch > self.max_extracts_per_job {
                        batches.push(Batch { code: fetch_code, operations: Vec::new(),
                            dependencies: gate.into_iter().collect() });
                        batch_extracts = 0;
                    }
                    let batch = batches.len() - 1;
                    batches[batch].operations.push(Operation::Fetch(piece.program, fetch));
                    fetch_dependencies[piece.program].insert(batch);
                    batch_extracts += count;
                    extracts += count;
                }
            }
            if extracts > self.extract_capacity { return Err(BindPoseError::UnsupportedSetup.into()); }
            stage_start = batches.len();
            for piece in &epoch.pieces {
                for operation in piece.segment.operations.clone() {
                    if batches.len() == stage_start || batches.last().is_some_and(|b| b.operations.len() == blend_limit) {
                        batches.push(Batch { code: AnimationJobCode::SqtBlend, operations: Vec::new(),
                            dependencies: gate.into_iter().collect() });
                    }
                    let index = batches.len() - 1;
                    let batch = &mut batches[index];
                    batch.operations.push(Operation::Blend(piece.program, operation));
                    batch.dependencies.extend(fetch_dependencies[piece.program].iter().copied());
                    batch.dependencies.extend(set_dependencies[piece.program].iter().copied());
                    if let Some(previous) = latest_blend[piece.program] {
                        if previous != index { batch.dependencies.insert(previous); }
                    }
                    latest_blend[piece.program] = Some(index);
                }
            }
            stage_start = batches.len();
            for piece in &epoch.pieces {
                if !piece.final_segment { continue; }
                if !matches!(work.programs[piece.program].output,ProgramOutput::Matrix(_)){
                    if let Some(previous)=latest_blend[piece.program]{pose_batches.insert(previous);}
                    else{pose_batches.extend(fetch_dependencies[piece.program].iter().copied());}
                    continue;
                }
                if batches.len() == stage_start || batches.last().is_some_and(|b| b.operations.len() == pose_limit) {
                    pose_batches.insert(batches.len());
                    batches.push(Batch { code: AnimationJobCode::PoseMatMult, operations: Vec::new(),
                        dependencies: gate.into_iter().collect() });
                }
                let batch = batches.last_mut().expect("pose batch");
                batch.operations.push(Operation::Pose(piece.program));
                batch.dependencies.extend(set_dependencies[piece.program].iter().copied());
                if let Some(previous) = latest_blend[piece.program] { batch.dependencies.insert(previous); }
                else {
                    batch.dependencies.extend(fetch_dependencies[piece.program].iter().copied());
                    batch.dependencies.extend(set_dependencies[piece.program].iter().copied());
                }
            }
            if let Some((program, segment)) = epoch.boundary {
                let mut dependencies = (epoch_start..batches.len()).collect::<BTreeSet<_>>();
                dependencies.extend(gate);
                gate = Some(batches.len());
                batches.push(Batch { code: AnimationJobCode::ReuseBuffer,
                    operations: vec![Operation::Reuse(program, segment)], dependencies });
            }
        }
        let affinity = crate::animation_job_owners::execution::affinity::selected_category_affinity(self.job_category)?;
        let total = batches.len();
        let mut jobs = Vec::with_capacity(total + 1);
        let root = Arc::clone(self.null_job.as_ref().ok_or(BindPoseError::BufferedOwnerRequired)?);
        {
            let mut retained = root.lock().map_err(|_| ActorFaceError::Lock)?;
            retained.entry_point = AnimationEntryPoint::new(None, Some(b""));
            retained.entry_point.affinity = affinity;
        }
        jobs.push(AnimationQueuedJob { job: root, work: None,
            dependencies: pose_batches.into_iter().map(|index| total - index).collect() });
        let mut boundary_index = 0;
        for batch in batches.into_iter().rev() {
            let (job, code, name) = if batch.code == AnimationJobCode::ReuseBuffer {
                if boundary_index == self.body_reuse_jobs.len() { self.body_reuse_jobs.push(self.scheduler.create_job()); }
                let job = Arc::clone(&self.body_reuse_jobs[boundary_index]);
                boundary_index += 1;
                let Operation::Reuse(program, segment) = batch.operations[0] else { unreachable!() };
                // DispatchReuseBufferCmd uses a NULL barrier for empty copies.
                let copy = !work.programs[program].commands.segments()[segment].copies.is_empty();
                (job, copy.then_some(AnimationJobCode::ReuseBuffer), if copy { b"ACSreuseBuffer".as_slice() } else { b"".as_slice() })
            } else {
                let (pool, name) = match batch.code {
                    AnimationJobCode::FetchVbr => (self.fetch_pool.as_mut(), b"AnimFetchVBR".as_slice()),
                    AnimationJobCode::FetchRaw => (self.fetch_pool.as_mut(), b"AnimFetch".as_slice()),
                    AnimationJobCode::SetData => (self.set_pool.as_mut(), b"ACSsetData".as_slice()),
                    AnimationJobCode::SqtBlend => (self.blend_pool.as_mut(), b"ACSsqtBlend".as_slice()),
                    AnimationJobCode::PoseMatMult => (self.pose_matrix_pool.as_mut(), b"ACSposeMatMult".as_slice()),
                    _ => return Err(BindPoseError::UnsupportedSetup.into()),
                };
                let pool = pool.ok_or(BindPoseError::BufferedOwnerRequired)?;
                if batch.dependencies.len() > pool.max_dependencies { return Err(BindPoseError::UnsupportedSetup.into()); }
                (next_job(pool)?, Some(batch.code), name)
            };
            {
                let mut retained = job.lock().map_err(|_| ActorFaceError::Lock)?;
                retained.entry_point = AnimationEntryPoint::new(code, Some(name));
                retained.entry_point.affinity = affinity;
                if matches!(code, Some(AnimationJobCode::SetData | AnimationJobCode::SqtBlend | AnimationJobCode::PoseMatMult)) {
                    retained.parameters[2] = 16;
                }
            }
            jobs.push(AnimationQueuedJob { job,
                dependencies: batch.dependencies.into_iter().map(|index| total - index).collect(),
                work: code.map(|_| Arc::new(BatchWork { work: Arc::clone(&work), operations: batch.operations }) as Arc<dyn AnimationJobWork>) });
        }
        self.sqt_pending = Some(PendingSqtDispatch { work, jobs, backend: None });
        let manager = source.require_job_manager().map_err(HandFaceRuntimeError::Source)?;
        let backend = manager.scheduler.as_ref().ok_or(AnimationExecutionError::MissingBackend)?;
        let pending = self.sqt_pending.as_mut().expect("retained pedestrian jobs");
        backend.submit_sqt_job_graph(&pending.jobs)?;
        pending.backend = Some(Arc::clone(backend));
        self.last_submitted.clone_from(&self.commands);
        self.nblend.end_command_set();
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        Ok(())
    }
}
