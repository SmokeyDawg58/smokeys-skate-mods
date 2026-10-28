use super::{
    AnimationExecutionError, AnimationJobCode, AnimationJobWork, AnimationQueuedJob,
    AnimationSchedulerBackend, InstancePhase, Queue, Shared, affinity, execute, take_ready,
};
use crate::animation_job_owners::{AnimationEntryPoint, AnimationJobScheduler};
use std::sync::{Arc, Condvar, Mutex};

// Deterministic unit queue: same backend/instance/barrier implementation, no
// independently racing workers. Production Init still creates three ANY workers.
fn backend() -> Arc<AnimationSchedulerBackend> {
    Arc::new(AnimationSchedulerBackend {
        shared: Arc::new(Shared {
            queue: Mutex::new(Queue {
                slots: (0..512).map(|_| None).collect(),
                previous_index: 0,
                next_submission: 0,
                retained: vec![],
                failure: None,
                stopping: false,
            }),
            changed: Condvar::new(),
        }),
        workers: vec![],
        source_processors: [1, 3, 5],
        worker_affinity: 63,
        calling_thread_may_run_deferred: true,
        function_queue_size: 512,
        job_queue_size: 128,
    })
}
fn null_job(category: u32) -> Vec<AnimationQueuedJob> {
    let job = AnimationJobScheduler::new().create_job();
    job.lock().unwrap().entry_point.affinity =
        affinity::selected_category_affinity(category).unwrap();
    vec![AnimationQueuedJob {
        job,
        dependencies: vec![],
        work: None,
    }]
}
fn phase(backend: &AnimationSchedulerBackend, job: &AnimationQueuedJob) -> InstancePhase {
    let index = usize::from(job.job.lock().unwrap().handle.index);
    backend.shared.queue.lock().unwrap().slots[index]
        .as_ref()
        .unwrap()
        .phase
}

#[test]
fn admitted_categories_preserve_distinct_calling_thread_masks() {
    assert_eq!(affinity::selected_category_affinity(0), Ok(1));
    assert_eq!(affinity::selected_category_affinity(2), Ok(4));
    for category in [1, 3, u32::MAX] {
        assert_eq!(
            affinity::selected_category_affinity(category),
            Err(AnimationExecutionError::UnsupportedCategory(category))
        );
    }
}

#[test]
fn category_two_wait_runs_same_queue_without_consuming_ready_body_peer() {
    let backend = backend();
    let body = null_job(0);
    let presentation = null_job(2);
    backend.submit_sqt_streams(&body, false, false).unwrap();
    backend
        .submit_sqt_streams(&presentation, false, false)
        .unwrap();
    backend.wait_in_category(&presentation[0].job, 2).unwrap();
    assert_eq!(phase(&backend, &presentation[0]), InstancePhase::Done);
    assert_eq!(phase(&backend, &body[0]), InstancePhase::Ready);
    backend.release_selected(&presentation).unwrap();
    backend.wait(&body[0].job).unwrap();
    backend.release_selected(&body).unwrap();
}

#[test]
fn submitted_instance_retains_affinity_when_entry_point_owner_changes() {
    let backend = backend();
    let presentation = null_job(2);
    backend
        .submit_sqt_streams(&presentation, false, false)
        .unwrap();
    presentation[0].job.lock().unwrap().entry_point.affinity = 1;
    assert!(take_ready(&mut backend.shared.queue.lock().unwrap(), 1).is_none());
    backend.wait_in_category(&presentation[0].job, 2).unwrap();
    assert_eq!(phase(&backend, &presentation[0]), InstancePhase::Done);
}

#[test]
fn any_worker_mask_uses_native_affinity_priority_before_submission_order() {
    let backend = backend();
    let body = null_job(0);
    let presentation = null_job(2);
    backend.submit_sqt_streams(&body, false, false).unwrap();
    backend
        .submit_sqt_streams(&presentation, false, false)
        .unwrap();
    let selected = take_ready(
        &mut backend.shared.queue.lock().unwrap(),
        backend.worker_affinity,
    )
    .unwrap();
    assert_eq!(
        selected.index,
        usize::from(presentation[0].job.lock().unwrap().handle.index)
    );
    execute(&backend.shared, selected);
    let selected = take_ready(
        &mut backend.shared.queue.lock().unwrap(),
        backend.worker_affinity,
    )
    .unwrap();
    assert_eq!(
        selected.index,
        usize::from(body[0].job.lock().unwrap().handle.index)
    );
    execute(&backend.shared, selected);
    assert_eq!(phase(&backend, &body[0]), InstancePhase::Done);
}

#[test]
fn unmatched_affinity_does_not_claim_ready_jobs_and_ties_use_submission_low47() {
    let backend = backend();
    let first = null_job(2);
    let second = null_job(2);
    backend.submit_sqt_streams(&first, false, false).unwrap();
    backend.submit_sqt_streams(&second, false, false).unwrap();
    for mask in [0, 1, 2] {
        assert!(take_ready(&mut backend.shared.queue.lock().unwrap(), mask).is_none());
    }
    assert_eq!(phase(&backend, &first[0]), InstancePhase::Ready);
    let selected = take_ready(&mut backend.shared.queue.lock().unwrap(), 4).unwrap();
    assert_eq!(
        selected.index,
        usize::from(first[0].job.lock().unwrap().handle.index)
    );
    execute(&backend.shared, selected);
    assert_eq!(affinity::ready_order(4, 63, (1_u64 << 47) + 7), (59, 7));
}

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
fn category_two_fetch_blend_dependencies_complete_before_root_and_keep_body_ready() {
    let backend = backend();
    let body = null_job(0);
    backend.submit_sqt_streams(&body, false, false).unwrap();
    let scheduler = AnimationJobScheduler::new();
    let work = Arc::new(Work(Mutex::new(vec![])));
    let jobs: Vec<_> = [
        None,
        Some(AnimationJobCode::SqtBlend),
        Some(AnimationJobCode::FetchRaw),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, code)| {
        let job = scheduler.create_job();
        job.lock().unwrap().entry_point = AnimationEntryPoint::new(code, None);
        job.lock().unwrap().entry_point.affinity = affinity::selected_category_affinity(2).unwrap();
        AnimationQueuedJob {
            job,
            dependencies: if index == 2 { vec![] } else { vec![index + 1] },
            work: code.map(|_| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
        }
    })
    .collect();
    backend.submit_sqt_streams(&jobs, true, true).unwrap();
    backend.wait_in_category(&jobs[0].job, 2).unwrap();
    assert_eq!(
        *work.0.lock().unwrap(),
        [AnimationJobCode::FetchRaw, AnimationJobCode::SqtBlend]
    );
    assert_eq!(phase(&backend, &body[0]), InstancePhase::Ready);
    backend.release_selected(&jobs).unwrap();
    backend.wait(&body[0].job).unwrap();
}

#[test]
fn unadmitted_entry_affinity_is_rejected_before_queue_mutation() {
    let backend = backend();
    let jobs = null_job(0);
    jobs[0].job.lock().unwrap().entry_point.affinity = 2;
    assert_eq!(
        backend.submit_sqt_streams(&jobs, false, false),
        Err(AnimationExecutionError::UnsupportedEntryPoint)
    );
    assert!(backend.shared.queue.lock().unwrap().retained.is_empty());
}
