use super::{
    AnimationExecutionError, AnimationJobCode, AnimationJobManager, AnimationJobWork,
    AnimationQueuedJob,
};
use crate::animation_job_owners::{AnimationEntryPoint, AnimationJobScheduler};
use std::sync::{Arc, Mutex};

struct Work(Mutex<Vec<AnimationJobCode>>);
impl AnimationJobWork for Work {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        self.0
            .lock()
            .map_err(|_| AnimationExecutionError::Lock)?
            .push(code);
        Ok(())
    }
}

#[test]
fn complete_dependency_barriers_release_fetch_then_pose_then_root() {
    let mut manager = AnimationJobManager::new();
    manager.try_initialize_local_backend().unwrap();
    let backend = manager.scheduler.as_ref().unwrap();
    let scheduler = AnimationJobScheduler::new();
    let work = Arc::new(Work(Mutex::new(Vec::new())));
    let mut queued = Vec::new();
    for (index, code) in [
        None,
        Some(AnimationJobCode::Pose),
        Some(AnimationJobCode::FetchVbr),
    ]
    .into_iter()
    .enumerate()
    {
        let job = scheduler.create_job();
        job.lock().unwrap().entry_point = AnimationEntryPoint::new(code, None);
        job.lock().unwrap().entry_point.affinity = 1;
        queued.push(AnimationQueuedJob {
            job,
            dependencies: if index == 2 {
                Vec::new()
            } else {
                vec![index + 1]
            },
            work: code.map(|_| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
        });
    }
    backend.submit_bind_pose(&queued).unwrap();
    backend.wait(&queued[0].job).unwrap();
    assert_eq!(
        *work.0.lock().unwrap(),
        [AnimationJobCode::FetchVbr, AnimationJobCode::Pose]
    );
    assert_eq!(
        backend.submit_bind_pose(&queued),
        Err(AnimationExecutionError::SubmissionStillRetained)
    );
    backend.release_completed().unwrap();
    backend.submit_bind_pose(&queued).unwrap();
    backend.wait(&queued[0].job).unwrap();
    backend.release_completed().unwrap();
}

#[test]
fn body_sqt_job_finishes_before_local_and_global_extraction() {
    let mut manager = AnimationJobManager::new();
    manager.try_initialize_local_backend().unwrap();
    let backend = manager.scheduler.as_ref().unwrap();
    let scheduler = AnimationJobScheduler::new();
    let work = Arc::new(Work(Mutex::new(Vec::new())));
    let mut queued = Vec::new();
    for (index, code) in [
        None,
        Some(AnimationJobCode::LocalToGlobal),
        Some(AnimationJobCode::SqtToLocal),
        Some(AnimationJobCode::SqtBlend),
        Some(AnimationJobCode::FetchVbr),
    ]
    .into_iter()
    .enumerate()
    {
        let job = scheduler.create_job();
        job.lock().unwrap().entry_point = AnimationEntryPoint::new(code, None);
        job.lock().unwrap().entry_point.affinity = 1;
        queued.push(AnimationQueuedJob {
            job,
            dependencies: if index == 4 { vec![] } else { vec![index + 1] },
            work: code.map(|_| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
        });
    }
    assert_eq!(
        backend.submit_body_tree(&queued, false),
        Err(AnimationExecutionError::UnsupportedGraph)
    );
    backend.submit_body_tree(&queued, true).unwrap();
    backend.wait(&queued[0].job).unwrap();
    assert_eq!(
        *work.0.lock().unwrap(),
        [
            AnimationJobCode::FetchVbr,
            AnimationJobCode::SqtBlend,
            AnimationJobCode::SqtToLocal,
            AnimationJobCode::LocalToGlobal
        ]
    );
    backend.release_completed().unwrap();
}

#[test]
fn separate_acs_submission_and_sync_preserve_peer_handles() {
    let mut manager = AnimationJobManager::new();
    manager.try_initialize_local_backend().unwrap();
    let backend = manager.scheduler.as_ref().unwrap();
    let scheduler = AnimationJobScheduler::new();
    let first = scheduler.create_job();
    let second = scheduler.create_job();
    first.lock().unwrap().entry_point.affinity = 1;
    second.lock().unwrap().entry_point.affinity = 1;
    let first = vec![AnimationQueuedJob {
        job: first,
        dependencies: vec![],
        work: None,
    }];
    let second = vec![AnimationQueuedJob {
        job: second,
        dependencies: vec![],
        work: None,
    }];
    backend.submit_sqt_streams(&first, false, false).unwrap();
    backend.submit_sqt_streams(&second, false, false).unwrap();
    backend.wait(&first[0].job).unwrap();
    backend.release_selected(&first).unwrap();
    assert_eq!(
        backend.wait(&first[0].job),
        Err(AnimationExecutionError::InvalidHandle)
    );
    backend.wait(&second[0].job).unwrap();
    backend.release_selected(&second).unwrap();
}
