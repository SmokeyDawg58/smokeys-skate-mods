use super::*;
use crate::collision_query_batches::{CollisionQueryBatchCapacity, CollisionQueryBatchConfig};

fn config(tests: u32) -> CollisionQueryBatchConfig {
    let cap = CollisionQueryBatchCapacity { batches: 1, tests };
    CollisionQueryBatchConfig {
        line: cap,
        boxes: cap,
        trajectory: cap,
        step_selector: cap,
    }
}
fn input() -> AnimationStepInput {
    AnimationStepInput {
        reference: [0.0; 4].into(),
        reference_offset: [0.0; 4].into(),
        parameter_vector: [0.0; 4].into(),
        speed_at_end: [0.0; 4].into(),
        parameter: 0.0,
        spread_angle: 0.0,
        turn_intent: 0.0,
        max_y: 0.0,
        flags: 0,
    }
}
#[test]
fn capacity_precedes_count_assertion_and_native_next_consumes_failing_slot() {
    let mut batches = CollisionQueryBatches::<()>::new(config(0));
    let lease = batches.acquire(Family::StepSelector, "full").unwrap();
    batches.begin(&lease).unwrap();
    assert!(matches!(
        batches.add_step_test(&lease, &[input(); 5]),
        Ok(false)
    ));
    let mut batches = CollisionQueryBatches::<()>::new(config(2));
    let lease = batches.acquire(Family::StepSelector, "invalid").unwrap();
    batches.begin(&lease).unwrap();
    assert!(matches!(
        batches.add_step_test(&lease, &[input(); 5]),
        Err(StepBatchError::InputCountAssertion)
    ));
    assert_eq!(batches.test_count(&lease), Ok(1));
    assert!(matches!(
        batches.end_step_tests(&lease, |_, _| Ok::<_, ()>(())),
        Err(StepBatchError::Batch(BatchError::Incomplete {
            test: 0,
            ..
        }))
    ));
}
#[test]
fn failed_worker_preserves_partial_payload_and_cannot_sync_or_release() {
    let mut batches = CollisionQueryBatches::<()>::new(config(1));
    let lease = batches.acquire(Family::StepSelector, "partial").unwrap();
    batches.begin(&lease).unwrap();
    assert!(batches.add_step_test(&lease, &[input()]).unwrap());
    assert!(matches!(
        batches.end_step_tests(&lease, |_, result| {
            result[0][0].priority = 3;
            Err(())
        }),
        Err(StepBatchError::Execute { test: 0, .. })
    ));
    assert_eq!(
        batches.batch(&lease).unwrap().steps[0].result[0][0].priority,
        3
    );
    assert!(batches.sync(&lease).is_err());
    assert!(batches.release(&lease).is_err());
}
#[test]
fn complete_miss_still_returns_result_storage_and_reset_retains_trajectory() {
    let mut batches = CollisionQueryBatches::<()>::new(config(1));
    let lease = batches.acquire(Family::StepSelector, "miss").unwrap();
    batches.begin(&lease).unwrap();
    batches.add_step_test(&lease, &[input()]).unwrap();
    batches
        .end_step_tests(&lease, |_, result| {
            result[0][0].trajectory.time = [42.0; 4];
            Ok::<_, ()>(())
        })
        .unwrap();
    batches.sync(&lease).unwrap();
    let result = batches.get_step_result(&lease, 0).unwrap();
    assert!(!result[0][0].valid);
    assert_eq!(result[0][0].collision.time, [-1.0; 4]);
    batches.begin(&lease).unwrap();
    batches.add_step_test(&lease, &[input()]).unwrap();
    assert_eq!(
        batches.batch(&lease).unwrap().steps[0].result[0][0]
            .trajectory
            .time,
        [42.0; 4]
    );
}
#[test]
fn aggregate_uses_current_priority_time_even_for_higher_priority_candidate() {
    let mut old = StepTrajectoryResult::new();
    let mut first = StepTrajectoryResult::new();
    first.priority = 1;
    first.collision.time = [2.0; 4];
    first.hit_times[1] = 2.0;
    first.surface = 10;
    consider_child(&mut old, &first);
    assert_eq!(old.surface, 10);
    let mut later = first;
    later.priority = 3;
    later.surface = 20;
    later.hit_times[1] = 1.0;
    consider_child(&mut old, &later);
    assert_eq!(old.surface, 20);
    let mut equal = later;
    equal.surface = 30;
    consider_child(&mut old, &equal);
    assert_eq!(old.surface, 20);
    equal.collision.time[3] = f32::NAN;
    equal.priority = 0;
    consider_child(&mut old, &equal);
    assert_eq!(old.surface, 20);
}
