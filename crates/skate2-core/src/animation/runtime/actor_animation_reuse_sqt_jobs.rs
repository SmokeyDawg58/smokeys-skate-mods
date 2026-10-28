//! RAW hand/face continuation of `GetNextLoc82a24c28` and `DispatchReuseBufferCmd`
//! 82a2fc18. Reuse executes after prior producers, before later slot writes.
use super::{
    ActorAnimationInitializeSource, ActorFaceError, AnimationEntryPoint, AnimationExecutionError,
    AnimationJobCode, AnimationJobWork, AnimationQueuedJob, Arc, BindPoseCommand, BindPoseError,
    BufferedBodyCommands, Fetch, HandFaceRuntimeError, PendingSqtDispatch, SqtDispatchWork,
    next_job, tree_commands,
};
use tree_commands::reuse::Segment;

#[cfg(test)]
mod tests {
    use super::super::{AndaleHierarchy, AndaleSqt, ProgramOutput, QueuedSqtProgram, TreeCommands};
    use super::*;
    use crate::actor_animation_construction::initialization::bind_pose::sqt_output::OwnedAnimationSqtBuffer;
    use crate::actor_animation_tree::eval::AnimationEvalCommands;

    #[test]
    fn bounded_hand_segments_keep_fixed_operands_and_extract_only_at_the_end() {
        let (mut commands, fixed) = TreeCommands::fixed_test_collector();
        commands.enable_reuse();
        fixed.lock().unwrap().fixed_frames = [2.0, 6.0, 0.0]
            .map(|x| {
                vec![AndaleSqt {
                    scale: [1.0; 4],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    translation: [x, 0.0, 0.0, 1.0],
                }]
            })
            .to_vec();
        for _ in 0..4 {
            commands.set_fixed_sqt(&fixed, 0).unwrap();
            commands.set_fixed_sqt(&fixed, 1).unwrap();
            commands.blend(0.0).unwrap();
        }
        for _ in 0..3 {
            commands.blend(0.0).unwrap();
        }
        commands.set_fixed_sqt(&fixed, 0).unwrap();
        commands.set_fixed_sqt(&fixed, 1).unwrap();
        commands.blend(1.0).unwrap();
        commands.blend(0.5).unwrap();
        let segments = commands.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(commands.dynamic_count(), 4);
        let output = OwnedAnimationSqtBuffer::allocate_unknown(1);
        output.begin_extract().unwrap();
        let work = Arc::new(SqtDispatchWork::new(vec![QueuedSqtProgram {
            commands,
            output: ProgramOutput::Sqt(output.clone()),
            hierarchy: AndaleHierarchy {
                has_trajectory: false,
                parents: vec![-1],
                bone_metadata: vec![0],
                parts: vec![],
                bone_names: vec![],
                packed_bone_names: vec![],
            }.into(),
        }]));
        let first = SegmentWork {
            work: Arc::clone(&work),
            segment: segments[0].clone(),
            final_segment: false,
        };
        first.run(AnimationJobCode::SqtBlend).unwrap();
        first.run(AnimationJobCode::ReuseBuffer).unwrap();
        assert!(output.is_unwritten().unwrap());
        let last = SegmentWork {
            work: Arc::clone(&work),
            segment: segments[1].clone(),
            final_segment: true,
        };
        last.run(AnimationJobCode::SqtBlend).unwrap();
        assert!(!output.is_unwritten().unwrap());
        assert!(output.read().is_err());
        let state = work.state.lock().unwrap();
        let x = state.sqts[0][0].as_ref().unwrap()[0].translation[0];
        assert!((3.9..4.1).contains(&x), "lost a live or fixed operand: {x}");
        drop(state);
        work.complete_sync().unwrap();
        assert_eq!(output.read().unwrap().0, 1);
    }
}

struct SegmentWork {
    work: Arc<SqtDispatchWork>,
    segment: Segment,
    final_segment: bool,
}

impl AnimationJobWork for SegmentWork {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self
            .work
            .state
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?;
        let result = (|| {
            let program = &self.work.programs[0];
            let sqts = &mut state.sqts[0];
            match code {
                AnimationJobCode::FetchRaw => {
                    for (slot, fetch) in &program.commands.fetches[self.segment.fetches.clone()] {
                        let values = match fetch {
                            Fetch::Pose(pose) => pose.raw_sqt_for(&program.hierarchy)?,
                            Fetch::Clip(_) => return Err(BindPoseError::UnsupportedSetup),
                        };
                        *sqts
                            .get_mut(usize::from(*slot))
                            .ok_or(BindPoseError::UnsupportedSetup)? = Some(values);
                    }
                }
                AnimationJobCode::SqtBlend => tree_commands::execute_operation_range(
                    &program.commands,
                    sqts,
                    &program.hierarchy,
                    self.segment.operations.clone(),
                )?,
                AnimationJobCode::ReuseBuffer => tree_commands::reuse::execute(
                    &self.segment.copies,
                    sqts,
                    program.hierarchy.parents.len(),
                )?,
                _ => return Err(BindPoseError::UnsupportedSetup),
            }
            if self.final_segment && program.extraction_job()? == Some(code) {
                program.extract(sqts)?;
            }
            Ok(())
        })();
        result.map_err(|error| {
            state.failure = Some(Arc::new(error));
            AnimationExecutionError::Work("buffered hand/face reuse segment")
        })
    }
}

impl BufferedBodyCommands {
    #[expect(
        clippy::too_many_lines,
        reason = "preserve ordered same-owner segment configuration, submission and publication"
    )]
    pub(super) fn submit_reused_sqt<S: ActorAnimationInitializeSource>(
        &mut self,
        work: Arc<SqtDispatchWork>,
        source: &mut S,
    ) -> Result<(), HandFaceRuntimeError<S::Error>> {
        // Preserve the existing single-Actor RAW ownership boundary. External
        // SetData and matrix-output streams have separate producer contracts.
        if work.programs.len() != 1 || work.has_set() || work.has_matrix_pose() {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let segments = work.programs[0].commands.segments();
        let boundaries = self
            .commands
            .iter()
            .filter_map(|command| match command {
                BindPoseCommand::ReuseBuffer { segment } => Some(*segment),
                _ => None,
            })
            .collect::<Vec<_>>();
        if boundaries != (0..segments.len() - 1).collect::<Vec<_>>() {
            return Err(BindPoseError::UnsupportedSetup.into());
        }
        let affinity =
            crate::animation_job_owners::execution::affinity::selected_category_affinity(
                self.job_category,
            )?;
        let mut stages = vec![];
        let mut shapes = vec![];
        let mut boundary_index = 0;
        for (index, segment) in segments.iter().enumerate() {
            let final_segment = index + 1 == segments.len();
            let view = Arc::new(SegmentWork {
                work: Arc::clone(&work),
                segment: segment.clone(),
                final_segment,
            });
            let fetch = !segment.fetches.is_empty();
            let blend = !segment.operations.is_empty();
            // Native empty-copy boundaries flush producers without assigning
            // ReuseBuffer code. The chain still orders the adjacent segments.
            let reuse = !segment.copies.is_empty();
            shapes.push((fetch, blend, reuse));
            for (present, code, name) in [
                (fetch, AnimationJobCode::FetchRaw, b"AnimFetch".as_slice()),
                (blend, AnimationJobCode::SqtBlend, b"ACSsqtBlend".as_slice()),
                (
                    reuse,
                    AnimationJobCode::ReuseBuffer,
                    b"ACSreuseBuffer".as_slice(),
                ),
            ] {
                if !present {
                    continue;
                }
                let job = match code {
                    AnimationJobCode::FetchRaw => next_job(
                        self.fetch_pool
                            .as_mut()
                            .ok_or(BindPoseError::BufferedOwnerRequired)?,
                    )?,
                    AnimationJobCode::SqtBlend => next_job(
                        self.blend_pool
                            .as_mut()
                            .ok_or(BindPoseError::BufferedOwnerRequired)?,
                    )?,
                    AnimationJobCode::ReuseBuffer => {
                        // Reuse the retained boundary-job storage already owned
                        // by this ACS; no new command system or SQT allocation.
                        if boundary_index == self.body_reuse_jobs.len() {
                            self.body_reuse_jobs.push(self.scheduler.create_job());
                        }
                        let job = Arc::clone(&self.body_reuse_jobs[boundary_index]);
                        boundary_index += 1;
                        job
                    }
                    _ => unreachable!(),
                };
                {
                    let mut retained = job.lock().map_err(|_| ActorFaceError::Lock)?;
                    retained.entry_point = AnimationEntryPoint::new(Some(code), Some(name));
                    retained.entry_point.affinity = affinity;
                    if code == AnimationJobCode::SqtBlend {
                        retained.parameters[2] = 16;
                    }
                }
                stages.push((job, Some(Arc::clone(&view) as Arc<dyn AnimationJobWork>)));
            }
        }
        let root = Arc::clone(
            self.null_job
                .as_ref()
                .ok_or(BindPoseError::BufferedOwnerRequired)?,
        );
        {
            let mut retained = root.lock().map_err(|_| ActorFaceError::Lock)?;
            retained.entry_point = AnimationEntryPoint::new(None, Some(b""));
            retained.entry_point.affinity = affinity;
        }
        stages.push((root, None));
        stages.reverse();
        let last = stages.len() - 1;
        let jobs = stages
            .into_iter()
            .enumerate()
            .map(|(index, (job, work))| AnimationQueuedJob {
                job,
                work,
                dependencies: if index == last {
                    vec![]
                } else {
                    vec![index + 1]
                },
            })
            .collect::<Vec<_>>();
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
        let pending = self.sqt_pending.as_mut().expect("retained SQT reuse work");
        backend.submit_sqt_segments(&pending.jobs, &shapes)?;
        pending.backend = Some(Arc::clone(backend));
        self.last_submitted.clone_from(&self.commands);
        self.nblend.end_command_set();
        self.in_command_set -= 1;
        self.initiated_commands += 1;
        Ok(())
    }
}
