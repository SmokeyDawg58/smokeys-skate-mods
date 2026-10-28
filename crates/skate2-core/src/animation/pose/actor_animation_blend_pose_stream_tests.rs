use super::{
    BlendPoseError, BlendPoseTokenPool, ClipStream, eval_clip_streams, phase_stream_coefficients,
};
use crate::actor_graph_construction::ActorGraphIntents;

#[test]
fn authored_key_uses_previous_pose_and_truncated_twenty_bit_weight() {
    let name = ActorGraphIntents::native_name(b"LeftHand");
    let stream = ClipStream {
        name,
        mirror: name,
        keys: vec![(0, 3), (8191, 9)],
    };
    let mut pool = BlendPoseTokenPool::new();
    let mut buffer = pool.buffer(0).unwrap();
    eval_clip_streams(&[stream], 0.5, 1.0, &mut buffer, &mut pool).unwrap();
    assert_eq!(
        pool.tokens(&buffer, 0).unwrap(),
        &[3 << 23, 9 << 23, 0x4007_ffff, 0x8020_0000]
    );
    assert_eq!(pool.tokens(&buffer, 1).unwrap(), &[0]);
    assert_eq!(pool.tokens(&buffer, 2).unwrap(), &[0]);
}

#[test]
fn unordered_time_selects_first_key_and_does_not_require_numeric_conversion() {
    let name = ActorGraphIntents::native_name(b"Face");
    let stream = ClipStream {
        name,
        mirror: name,
        keys: vec![(0, 7), (8191, 8)],
    };
    let mut pool = BlendPoseTokenPool::new();
    let mut buffer = pool.buffer(0).unwrap();
    eval_clip_streams(&[stream], f32::NAN, 1.0, &mut buffer, &mut pool).unwrap();
    assert_eq!(pool.tokens(&buffer, 0).unwrap(), &[7 << 23]);
}

#[test]
fn native_duplicate_scan_is_backward_from_top_minus_key_count() {
    let mut pool = BlendPoseTokenPool::new();
    let mut buffer = pool.buffer(0).unwrap();
    // This is deliberately not a generic equal-pose simplifier: source scans
    // tokens at top-2 and top-3 and then removes the LAST two tokens.
    for pose in [4, 4, 9, 10] {
        pool.push_pose(&mut buffer, 0, pose).unwrap();
    }
    pool.push_coefficient(&mut buffer, 0, 0.5).unwrap();
    pool.push_blend(&mut buffer, 0, 2).unwrap();
    assert_eq!(pool.tokens(&buffer, 0).unwrap().len(), 6);
    // Source scan starts at index2 and1, so these matching values collapse.
    let mut other = pool.buffer(1).unwrap();
    for pose in [1, 9, 9, 2] {
        pool.push_pose(&mut other, 0, pose).unwrap();
    }
    pool.push_coefficient(&mut other, 0, 0.5).unwrap();
    pool.push_blend(&mut other, 0, 2).unwrap();
    assert_eq!(
        pool.tokens(&other, 0).unwrap(),
        &[1 << 23, 9 << 23, 9 << 23]
    );
}

#[test]
fn token_conversion_failure_preserves_the_source_written_prefix() {
    let mut pool = BlendPoseTokenPool::new();
    let mut buffer = pool.buffer(0).unwrap();
    pool.push_pose(&mut buffer, 0, 1).unwrap();
    pool.push_pose(&mut buffer, 0, 2).unwrap();
    assert!(matches!(
        pool.push_coefficient(&mut buffer, 0, f32::NAN),
        Err(BlendPoseError::Numeric(_))
    ));
    assert_eq!(pool.tokens(&buffer, 0).unwrap(), &[1 << 23, 2 << 23, 0]);
    assert_eq!(pool.tops[0][0], 2);
}

#[test]
fn phase_gate_smoothing_is_not_the_coefficient_passed_to_stream_blend() {
    let (coefficient, gate) = phase_stream_coefficients(0.4, true, false);
    assert_ne!(coefficient.to_bits(), gate.to_bits());
    assert_eq!(
        coefficient.to_bits(),
        ((0.4 - f32::from_bits(0x3e99_999a)) * f32::from_bits(0x4020_0001)).to_bits()
    );
}

#[test]
fn reset_reuses_pool_storage_and_clears_only_global_allocation_tops() {
    let mut pool = BlendPoseTokenPool::new();
    let mut buffer = pool.buffer(0).unwrap();
    pool.push_pose(&mut buffer, 0, 3).unwrap();
    let allocation = pool.lanes[0][0].as_ptr();
    pool.reset();
    assert_eq!(allocation, pool.lanes[0][0].as_ptr());
    assert_eq!(pool.lanes[0][0][0], 3 << 23);
    assert!(pool.tokens(&pool.buffer(0).unwrap(), 0).unwrap().is_empty());
}
