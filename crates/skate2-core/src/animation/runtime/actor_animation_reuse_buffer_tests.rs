use super::*;
use crate::animation_job_owners::{
    AnimationEntryPoint, AnimationJobCode, AnimationJobManager, AnimationJobScheduler,
    execution::{AnimationExecutionError, AnimationJobWork, AnimationQueuedJob},
};
use std::sync::{Arc, Mutex};
fn pose(x: f32) -> Vec<AndaleSqt> {
    vec![AndaleSqt {
        scale: [1.0; 4],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [x, 0.0, 0.0, 1.0],
    }]
}
#[test]
fn sequential_alias_copy_observes_earlier_destination_store() {
    let mut values = vec![Some(pose(10.0)), Some(pose(20.0)), Some(pose(30.0))];
    execute(&[(0, 1), (2, 0)], &mut values, 1).unwrap();
    assert_eq!(
        values[2].as_ref().unwrap()[0].translation[0].to_bits(),
        20.0_f32.to_bits()
    );
    execute(&[(1, 1)], &mut values, 1).unwrap();
}
#[test]
fn capacity_reuses_graph_live_outputs_in_producer_order() {
    let (mut tree, _) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    // A later command consumes slots0/2 and writes slot0, making it newer than slot3.
    tree.stack = vec![1, 3, 0];
    tree.produced(&[0, 2], 0);
    let next = tree.allocate_dynamic().unwrap();
    assert_eq!(next, 3);
    assert_eq!(tree.reuse.segments[0].copies, [(0, 1), (1, 3), (2, 0)]);
    assert_eq!(tree.stack, [0, 1, 2]);
    assert_eq!(tree.dynamic_count(), 4);
    assert_eq!(tree.owner.lock().unwrap().max_dynamic_sqts, 4);
    // The prior reuse producer is an old-segment root: advance its destination
    // ordinal without copying it. Only the newer producer emits a copy pair.
    tree.produced(&[0, 1], 0);
    tree.stack = vec![0, 2];
    assert_eq!(tree.allocate_dynamic().unwrap(), 2);
    assert_eq!(tree.reuse.segments.len(), 2);
    assert_eq!(tree.reuse.segments[1].copies, [(1, 0)]);
    assert_eq!(tree.next_sqt, 3);
    assert_eq!(tree.stack, [1, 0]);
    assert_eq!(tree.dynamic_count(), 4);
}

#[test]
fn consumed_fixed_operands_retain_their_owner_across_dynamic_compaction() {
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    tree.produced(&[0, 1, 2, super::super::fixed::FIXED_SQT_BIT | 1], 2);
    tree.stack = vec![3, 2];
    assert_eq!(tree.allocate_dynamic().unwrap(), 2);
    assert_eq!(tree.reuse.segments[0].copies, [(0, 3), (1, 2)]);
    assert_eq!(tree.stack, [0, 1]);
    assert!(Arc::ptr_eq(tree.fixed_owner.as_ref().unwrap(), &fixed));
    assert_eq!(tree.dynamic_count(), 4);
}

#[test]
fn identity_copy_never_reads_unknown_storage() {
    let mut values = vec![None];
    execute(&[(0, 0)], &mut values, 999).unwrap();
    execute(&[], &mut values, 999).unwrap();
    assert!(values[0].is_none());
}
type ProbeState = (usize, Vec<Option<Vec<AndaleSqt>>>, Vec<AnimationJobCode>);
struct SegmentProbe(Mutex<ProbeState>);
impl AnimationJobWork for SegmentProbe {
    fn run(&self, code: AnimationJobCode) -> Result<(), AnimationExecutionError> {
        let mut state = self.0.lock().unwrap();
        match state.0 {
            0 => {
                assert_eq!(code, AnimationJobCode::FetchVbr);
                state.1[1] = Some(pose(3.0));
            }
            1 => {
                assert_eq!(code, AnimationJobCode::SqtBlend);
                state.1[1].as_mut().unwrap()[0].translation[0] = 7.0;
            }
            2 => {
                assert_eq!(code, AnimationJobCode::ReuseBuffer);
                execute(&[(0, 1)], &mut state.1, 1).unwrap();
            }
            3 => {
                assert_eq!(code, AnimationJobCode::FetchVbr);
                state.1[1] = Some(pose(100.0));
            }
            4 => {
                assert_eq!(code, AnimationJobCode::SqtBlend);
                assert_eq!(
                    state.1[0].as_ref().unwrap()[0].translation[0].to_bits(),
                    7.0_f32.to_bits()
                );
            }
            _ => {}
        }
        state.0 += 1;
        state.2.push(code);
        Ok(())
    }
}
#[test]
fn retained_scheduler_copies_live_result_before_next_segment_fetch() {
    let mut manager = AnimationJobManager::new();
    manager.try_initialize_local_backend().unwrap();
    let backend = manager.scheduler.as_ref().unwrap();
    let scheduler = AnimationJobScheduler::new();
    let work = Arc::new(SegmentProbe(Mutex::new((0, vec![None, None], vec![]))));
    let execution = [
        AnimationJobCode::FetchVbr,
        AnimationJobCode::SqtBlend,
        AnimationJobCode::ReuseBuffer,
        AnimationJobCode::FetchVbr,
        AnimationJobCode::SqtBlend,
        AnimationJobCode::SqtToLocal,
        AnimationJobCode::LocalToGlobal,
    ];
    let codes = std::iter::once(None)
        .chain(execution.into_iter().rev().map(Some))
        .collect::<Vec<_>>();
    let jobs = codes
        .iter()
        .enumerate()
        .map(|(index, &code)| {
            let job = scheduler.create_job();
            job.lock().unwrap().entry_point = AnimationEntryPoint::new(code, None);
            job.lock().unwrap().entry_point.affinity = 1;
            AnimationQueuedJob {
                job,
                dependencies: if index + 1 == codes.len() {
                    vec![]
                } else {
                    vec![index + 1]
                },
                work: code.map(|_| Arc::clone(&work) as Arc<dyn AnimationJobWork>),
            }
        })
        .collect::<Vec<_>>();
    backend
        .submit_body_segments(&jobs, &[(true, true, true), (true, true, false)])
        .unwrap();
    backend.wait(&jobs[0].job).unwrap();
    assert_eq!(work.0.lock().unwrap().2, execution);
    backend.release_completed().unwrap();
}

#[test]
fn live_fixed_root_advances_reuse_copy_cursor_but_not_reader_slots() {
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    tree.produced(&[2, 3], 2);
    tree.stack.truncate(4);
    // DispatchReuseBufferCmd counts the older fixed root; GetNextLoc does not.
    assert_eq!(tree.allocate_dynamic().unwrap(), 3);
    assert_eq!(tree.reuse.segments[0].copies, [(1, 0), (2, 1), (3, 2)]);
    let fixed_bit = super::super::fixed::FIXED_SQT_BIT;
    assert_eq!(tree.stack, [fixed_bit | 1, 0, 1, 2]);
}

#[test]
fn fixed_stack_top_becomes_the_fixed_reuse_root() {
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    tree.produced(&[2, 3], 2);
    tree.stack.truncate(3);
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    assert_eq!(tree.allocate_dynamic().unwrap(), 3);
    assert_eq!(tree.reuse.segments[0].copies, [(0, 0), (1, 1), (2, 2)]);
    let fixed_bit = super::super::fixed::FIXED_SQT_BIT;
    assert_eq!(tree.stack, [0, 1, 2, fixed_bit | 1]);
    let [root] = &tree.reuse.fixed[..] else {
        panic!("one fixed reuse root");
    };
    assert_eq!(
        (root.id, root.order, root.modeled),
        (fixed_bit | 1, 6, true)
    );
    tree.produced(&[fixed_bit | 1], 3);
    assert!(tree.reuse.fixed.is_empty());
}

#[test]
fn fixed_mirror_allocates_after_compaction_then_dynamic_mirror_stays_in_place() {
    use crate::actor_animation_tree::eval::AnimationEvalCommands;
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    let mut original = pose(2.0);
    original[0].translation[2] = 7.0;
    fixed.lock().unwrap().fixed_frames = vec![pose(0.0), original.clone()];
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    tree.produced(&[2, 3], 2);
    tree.stack.truncate(3);
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    tree.mirror(0).unwrap();
    assert_eq!(tree.stack, [0, 1, 2, 3]);
    assert_eq!(tree.dynamic_count(), 4);
    assert_eq!(tree.reuse.segments.len(), 1);
    assert!(tree.reuse.fixed.is_empty());
    let mut sqts = vec![Some(pose(0.0)), Some(pose(1.0)), Some(pose(2.0)), None];
    let hierarchy = super::super::AndaleHierarchy {
        has_trajectory: false,
        parents: vec![-1],
        bone_metadata: vec![0],
        parts: vec![],
        bone_names: vec![],
        packed_bone_names: vec![],
    };
    super::super::execute_operation_range(&tree, &mut sqts, &hierarchy, 0..1).unwrap();
    assert_eq!(sqts[3].as_ref().unwrap()[0].translation[2], -7.0);
    assert_eq!(fixed.lock().unwrap().fixed_frames[1][0].translation[2], 7.0);
    tree.mirror(0).unwrap();
    assert_eq!(tree.dynamic_count(), 4);
    assert_eq!(tree.reuse.segments.len(), 1);
    super::super::execute_operation_range(&tree, &mut sqts, &hierarchy, 1..2).unwrap();
    assert_eq!(sqts[3].as_ref().unwrap()[0].translation[2], 7.0);
}

#[test]
fn reuse_top_fixed_occurrence_may_share_id_with_a_popped_operand() {
    let (mut tree, fixed) = TreeCommands::fixed_test_collector();
    tree.enable_reuse();
    for _ in 0..4 {
        let id = tree.allocate_dynamic().unwrap();
        tree.produced(&[], id);
        tree.stack.push(id);
    }
    tree.produced(&[2, 3], 2);
    tree.stack.truncate(3);
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    tree.set_fixed_sqt(&fixed, 1).unwrap();
    let input = tree.stack.pop().unwrap();
    assert_eq!(tree.allocate_dynamic().unwrap(), 3);
    // Both are graph occurrences, even though their fixed frame IDs coincide.
    assert_eq!(tree.reuse.fixed.len(), 2);
    tree.produced(&[input], 3);
    assert_eq!(tree.reuse.fixed.len(), 1);
    assert_eq!(
        tree.reuse.fixed[0].order,
        tree.reuse.previous_boundary.unwrap()
    );
}
