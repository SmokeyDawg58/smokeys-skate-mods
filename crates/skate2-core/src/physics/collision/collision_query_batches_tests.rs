use super::*;
use crate::{
    collision_line_collection::{CollisionLineCollectionMesh, try_collection_line},
    collision_line_numeric::HostCollisionLineNumeric,
    collision_mesh_line::{CollisionMeshOccurrence, tests::simple_triangle_mesh},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::convert::Infallible;

fn config(batches: u32, tests: u32) -> CollisionQueryBatchConfig {
    let capacity = CollisionQueryBatchCapacity { batches, tests };
    CollisionQueryBatchConfig {
        line: capacity,
        boxes: capacity,
        trajectory: capacity,
        step_selector: capacity,
    }
}

fn input(x: f32) -> CollisionLineInput {
    CollisionLineInput {
        start: SkeletonRootVector::known([x, 0.5, 1.0, 4.0]),
        end: SkeletonRootVector::known([x, 0.5, -1.0, 6.0]),
        fatness: 0.0,
    }
}

fn hit(owner: u8) -> CollisionMeshLineNearest<u8> {
    let mut result = CollisionMeshLineNearest::new();
    result.hit_count = 1;
    result.owner = Some(owner);
    result.line_parameter = 0.5;
    result
}

#[test]
fn allocated_four_families_start_idle_and_first_free_order_survives_release() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(2, 3));
    batches.sync_jobs_all().unwrap();
    for family in CollisionQueryBatchFamily::ALL {
        let first = batches.acquire(family, "").unwrap();
        let second = batches.acquire(family, "second").unwrap();
        assert_eq!((first.slot, second.slot), (0, 1));
        assert_eq!(batches.test_count(&first), Ok(0));
        assert_eq!(batches.state(&first), Ok(CollisionQueryBatchState::Idle));
        assert!(
            matches!(batches.acquire(family, "full"), Err(CollisionQueryBatchError::Exhausted { family: actual }) if actual == family)
        );
        batches.release(&first).unwrap();
        let reused = batches.acquire(family, "reused").unwrap();
        assert_eq!(reused.slot, 0);
        assert_eq!(
            batches.batch(&reused).unwrap().label.as_deref(),
            Some("reused")
        );
        assert_eq!(
            batches.state(&first),
            Err(CollisionQueryBatchError::ExpiredLease)
        );
        batches.release(&second).unwrap();
        batches.release(&reused).unwrap();
    }
}

#[test]
fn zero_batch_capacity_exhausts_and_zero_test_capacity_precedes_state_check() {
    let mut none = CollisionQueryBatches::<u8>::new(config(0, 0));
    assert!(matches!(
        none.acquire(CollisionQueryBatchFamily::Line, "x"),
        Err(CollisionQueryBatchError::Exhausted { .. })
    ));
    none.sync_jobs_all().unwrap();
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 0));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Line, "zero")
        .unwrap();
    assert_eq!(batches.add_line(&lease, input(0.5)), Ok(false));
    assert_eq!(batches.state(&lease), Ok(CollisionQueryBatchState::Idle));
}

#[test]
fn foreign_and_stale_leases_never_mutate_another_owner_or_new_acquisition() {
    let mut a = CollisionQueryBatches::<u8>::new(config(1, 2));
    let mut b = CollisionQueryBatches::<u8>::new(config(1, 2));
    let lease = a.acquire(CollisionQueryBatchFamily::Line, "a").unwrap();
    assert_eq!(b.begin(&lease), Err(CollisionQueryBatchError::WrongOwner));
    a.release(&lease).unwrap();
    let next = a.acquire(CollisionQueryBatchFamily::Line, "next").unwrap();
    assert_eq!(a.begin(&lease), Err(CollisionQueryBatchError::ExpiredLease));
    assert_eq!(a.state(&next), Ok(CollisionQueryBatchState::Idle));
}

#[test]
fn dropping_a_token_does_not_invent_native_release() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 1));
    drop(
        batches
            .acquire(CollisionQueryBatchFamily::Line, "lost")
            .unwrap(),
    );
    batches.sync_jobs_all().unwrap();
    assert!(matches!(
        batches.acquire(CollisionQueryBatchFamily::Line, "again"),
        Err(CollisionQueryBatchError::Exhausted { .. })
    ));
}

#[test]
fn source_capacity_precedes_state_and_gettest_checks_state_before_index() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 1));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Line, "line")
        .unwrap();
    assert_eq!(
        batches.add_line(&lease, input(0.5)),
        Err(CollisionQueryBatchError::State {
            actual: CollisionQueryBatchState::Idle
        })
    );
    batches.begin(&lease).unwrap();
    assert!(matches!(
        batches.get_line_result(&lease, 999),
        Err(CollisionQueryBatchError::State {
            actual: CollisionQueryBatchState::AddTests
        })
    ));
    assert_eq!(batches.add_line(&lease, input(0.5)), Ok(true));
    batches
        .end_line(&lease, |_| Ok::<_, Infallible>(hit(3)))
        .unwrap();
    assert_eq!(batches.add_line(&lease, input(4.0)), Ok(false));
    assert!(matches!(
        batches.get_line_result(&lease, 999),
        Err(CollisionQueryBatchError::State {
            actual: CollisionQueryBatchState::Running
        })
    ));
    batches.sync(&lease).unwrap();
    assert!(matches!(
        batches.get_line_result(&lease, 1),
        Err(CollisionQueryBatchError::TestIndex { index: 1, count: 1 })
    ));
    assert_eq!(
        batches.get_line_result(&lease, 0).unwrap().unwrap().owner,
        Some(3)
    );
}

#[test]
fn helper_sync_preserves_running_and_adding_states_without_execution_or_release() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(2, 2));
    let adding = batches
        .acquire(CollisionQueryBatchFamily::Line, "adding")
        .unwrap();
    let running = batches
        .acquire(CollisionQueryBatchFamily::Line, "running")
        .unwrap();
    batches.begin(&adding).unwrap();
    batches.add_line(&adding, input(0.5)).unwrap();
    batches.begin(&running).unwrap();
    batches.add_line(&running, input(0.7)).unwrap();
    batches
        .end_line(&running, |_| Ok::<_, Infallible>(hit(1)))
        .unwrap();
    batches.sync_jobs_all().unwrap();
    assert_eq!(
        batches.state(&adding),
        Ok(CollisionQueryBatchState::AddTests)
    );
    assert_eq!(
        batches.state(&running),
        Ok(CollisionQueryBatchState::Running)
    );
    assert_eq!(batches.test_count(&adding), Ok(1));
    assert_eq!(
        batches.sync(&adding),
        Err(CollisionQueryBatchError::State {
            actual: CollisionQueryBatchState::AddTests
        })
    );
    assert!(matches!(
        batches.acquire(CollisionQueryBatchFamily::Line, "still full"),
        Err(CollisionQueryBatchError::Exhausted { .. })
    ));
}

#[test]
fn release_adding_is_allowed_and_reacquire_retains_count_until_begin() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 2));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Line, "old")
        .unwrap();
    batches.begin(&lease).unwrap();
    batches.add_line(&lease, input(0.5)).unwrap();
    batches.release(&lease).unwrap();
    let next = batches
        .acquire(CollisionQueryBatchFamily::Line, "new")
        .unwrap();
    assert_eq!(batches.test_count(&next), Ok(1));
    // Source can expose this initialized AddTest miss after release/reacquire.
    assert!(batches.get_line_result(&next, 0).unwrap().is_none());
    batches.begin(&next).unwrap();
    assert_eq!(batches.test_count(&next), Ok(0));
    assert_eq!(batches.batch(&next).unwrap().lines.len(), 1);
}

#[test]
fn late_execution_refusal_retains_prior_result_and_running_lease_without_retry() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 3));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Line, "fail")
        .unwrap();
    batches.begin(&lease).unwrap();
    for x in [1.0, 2.0, 3.0] {
        batches.add_line(&lease, input(x)).unwrap();
    }
    let mut calls = Vec::new();
    let error = batches
        .end_line(&lease, |request| {
            calls.push(request.start.xyz[0]);
            if calls.len() == 2 {
                Err("actual collection refused")
            } else {
                Ok(hit(9))
            }
        })
        .unwrap_err();
    assert!(matches!(
        error,
        CollisionQueryBatchExecutionError::Line {
            test: 1,
            source: "actual collection refused"
        }
    ));
    assert_eq!(calls, [1.0, 2.0]);
    let batch = batches.batch(&lease).unwrap();
    assert_eq!(batch.lines[0].result.owner, Some(9));
    assert_eq!(batch.lines[1].result.hit_count, 0);
    assert_eq!(batch.lines[2].result.hit_count, 0);
    let expected = CollisionQueryBatchError::Incomplete {
        family: CollisionQueryBatchFamily::Line,
        slot: 0,
        test: 1,
    };
    assert_eq!(batches.sync_jobs_all(), Err(expected));
    assert_eq!(batches.sync(&lease), Err(expected));
    assert_eq!(batches.release(&lease), Err(expected));
    assert_eq!(batches.state(&lease), Ok(CollisionQueryBatchState::Running));
    assert!(matches!(
        batches.end_line(&lease, |_| -> Result<_, Infallible> {
            panic!("must not replay")
        }),
        Err(CollisionQueryBatchExecutionError::Batch(
            CollisionQueryBatchError::State {
                actual: CollisionQueryBatchState::Running
            }
        ))
    ));
}

#[test]
fn all_family_sync_order_reports_first_incomplete_slot_and_preserves_state() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(2, 2));
    for family in CollisionQueryBatchFamily::ALL {
        for batch in &mut batches.families[family.index()] {
            batch.count = 1;
            batch.incomplete = Some(0);
            batch.state = CollisionQueryBatchState::Running;
        }
    }
    for family in CollisionQueryBatchFamily::ALL {
        for slot in 0..2 {
            assert_eq!(
                batches.sync_jobs_all(),
                Err(CollisionQueryBatchError::Incomplete {
                    family,
                    slot,
                    test: 0
                })
            );
            batches.families[family.index()][slot].incomplete = None;
        }
    }
    batches.sync_jobs_all().unwrap();
    assert!(
        batches
            .families
            .iter()
            .flatten()
            .all(|batch| batch.state == CollisionQueryBatchState::Running && batch.count == 1)
    );
}

#[test]
fn unsupported_nonempty_family_refuses_after_running_and_empty_family_is_valid() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 2));
    for family in [
        CollisionQueryBatchFamily::Box,
        CollisionQueryBatchFamily::Trajectory,
        CollisionQueryBatchFamily::StepSelector,
    ] {
        let lease = batches.acquire(family, "empty").unwrap();
        batches.begin(&lease).unwrap();
        batches
            .end_line(&lease, |_| -> Result<_, Infallible> {
                panic!("not a line")
            })
            .unwrap();
        batches.sync(&lease).unwrap();
        batches.begin(&lease).unwrap();
        // No public request acceptance exists before these typed callers do.
        batches.batch_mut(&lease).unwrap().count = 1;
        assert!(
            matches!(batches.end_line(&lease, |_| -> Result<_, Infallible> { panic!("not a line") }), Err(CollisionQueryBatchExecutionError::Unsupported { family: actual }) if actual == family)
        );
        assert_eq!(batches.state(&lease), Ok(CollisionQueryBatchState::Running));
        assert!(
            matches!(batches.release(&lease), Err(CollisionQueryBatchError::Incomplete { family: actual, .. }) if actual == family)
        );
    }
}

#[test]
fn source_count_zero_skips_job_state_and_non_line_add_does_not_change_count() {
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 2));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Box, "box")
        .unwrap();
    batches.batch_mut(&lease).unwrap().incomplete = Some(17);
    batches.sync_jobs_all().unwrap();
    assert_eq!(
        batches.add_line(&lease, input(0.5)),
        Err(CollisionQueryBatchError::WrongFamily {
            actual: CollisionQueryBatchFamily::Box
        })
    );
    assert_eq!(batches.test_count(&lease), Ok(0));
}

#[test]
fn actual_collection_geometry_publishes_hit_and_miss_then_reuses_the_same_slots() {
    let (mesh, bytes) = simple_triangle_mesh(133);
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let bounds = Ok(([-20.0; 3], [20.0; 3]));
    let mut batches = CollisionQueryBatches::<u8>::new(config(1, 2));
    let lease = batches
        .acquire(CollisionQueryBatchFamily::Line, "real collection")
        .unwrap();
    batches.begin(&lease).unwrap();
    batches.add_line(&lease, input(0.5)).unwrap();
    batches.add_line(&lease, input(19.0)).unwrap();
    batches
        .end_line(&lease, |request| {
            let mut result = CollisionMeshLineNearest::new();
            let occurrence = CollisionLineCollectionMesh {
                occurrence: CollisionMeshOccurrence {
                    mesh: &mesh,
                    bytes: &bytes,
                    forward: &identity,
                    inverse: &identity,
                    owner: Some(42),
                    slot: 0,
                },
                local_bounds: &bounds,
            };
            try_collection_line(
                std::iter::once(occurrence),
                request,
                &mut result,
                &mut HostCollisionLineNumeric,
            )?;
            Ok::<
                _,
                crate::collision_line_collection::CollisionLineCollectionError<
                    crate::reckoning_numeric::ReckoningHostNumericError,
                >,
            >(result)
        })
        .unwrap();
    batches.sync_jobs_all().unwrap();
    assert_eq!(batches.state(&lease), Ok(CollisionQueryBatchState::Running));
    batches.sync(&lease).unwrap();
    let result = batches.get_line_result(&lease, 0).unwrap().unwrap();
    assert_eq!(result.owner, Some(42));
    assert_eq!(result.surface_id, 133);
    assert_eq!(result.line_parameter.to_bits(), 0.5_f32.to_bits());
    assert_eq!(
        result.position.xyz.map(f32::to_bits),
        [0.5_f32, 0.5, 0.0].map(f32::to_bits)
    );
    assert_eq!(result.position.w, Some(5.0));
    assert!(batches.get_line_result(&lease, 1).unwrap().is_none());
    batches.begin(&lease).unwrap();
    batches.add_line(&lease, input(10.0)).unwrap();
    assert_eq!(batches.batch(&lease).unwrap().lines[0].result.hit_count, 0);
    assert_eq!(batches.batch(&lease).unwrap().lines.len(), 2);
}
