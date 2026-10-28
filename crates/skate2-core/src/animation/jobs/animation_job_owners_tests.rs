use super::*;

#[test]
fn create_job_keeps_unknown_event_payloads_and_dependency_triggers_dormant() {
    let scheduler = AnimationJobScheduler::new();
    let job = scheduler.create_job();
    let job = job.lock().unwrap();
    assert_eq!(job.parameters, [0; 4]);
    assert_eq!(job.handle.submission_id, 0);
    assert!(job.handle.backend.is_none());
    assert_eq!(job.dependency_count, 0);
    assert!(
        job.dependencies
            .iter()
            .all(|dependency| dependency.job.is_none()
                && dependency.handle.submission_id == 0
                && dependency.trigger.is_none())
    );
    assert!(
        job.events
            .iter()
            .flatten()
            .all(|event| event.kind == 0 && event.payload == [None; 3])
    );
    assert_eq!(job.entry_point.code, None);
    assert!(job.entry_point.name.is_empty());
}

#[test]
fn entry_point_constructor_and_name_keep_native_defaults_and_fifteen_byte_limit() {
    let entry =
        AnimationEntryPoint::new(Some(AnimationJobCode::BlendSpace), Some(b"AnimBlendSpace"));
    assert_eq!(entry.name, b"AnimBlendSpace");
    assert_eq!(
        (
            entry.priority,
            entry.affinity,
            entry.environment,
            entry.code_recycle
        ),
        (128, 63, 0, 1)
    );
    assert_eq!(
        AnimationEntryPoint::new(None, Some(b"0123456789abcdefg")).name,
        b"0123456789abcde"
    );
    assert_eq!(
        AnimationEntryPoint::new(None, Some(b"abc\0ignored")).name,
        b"abc"
    );
    assert!(AnimationJobManager::new().scheduler.is_none());
}
