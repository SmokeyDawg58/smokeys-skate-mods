use super::*;

#[test]
fn mean_removal_preserves_lane_ownership_and_ignores_fourth_tokens() {
    let mut value = ShakeKeyframes::constructed();
    value
        .load(b"2 1 2 3 7 10 20 30 8 3 6 9 99 14 28 42 100")
        .unwrap();
    let rotations = value.rotations.as_ref().unwrap();
    let positions = value.positions.as_ref().unwrap();
    assert_eq!(
        rotations[0].unwrap().xyz.map(f32::to_bits),
        [-1.0_f32, -2.0, -3.0].map(f32::to_bits)
    );
    assert_eq!(
        positions[1].unwrap().xyz.map(f32::to_bits),
        [2.0_f32, 4.0, 6.0].map(f32::to_bits)
    );
    assert!(
        rotations
            .iter()
            .chain(positions)
            .all(|v| v.unwrap().w.is_none())
    );
}
#[test]
fn incomplete_frame_keeps_allocations_and_prior_raw_frame_without_mean_removal() {
    let mut value = ShakeKeyframes::constructed();
    assert_eq!(
        value.load(b"2 1 2 3 4 5 6 7 8 9 10"),
        Err(ShakeLoadError::MissingToken)
    );
    assert_eq!(value.count, 2);
    let rotations = value.rotations.as_ref().unwrap();
    assert_eq!(
        rotations[0].unwrap().xyz.map(f32::to_bits),
        [1.0_f32, 2.0, 3.0].map(f32::to_bits)
    );
    assert!(rotations[1].is_none());
    assert!(value.positions.as_ref().unwrap()[1].is_none());
}
#[test]
fn source_negative_count_bits_publish_before_host_allocation_boundary() {
    let mut value = ShakeKeyframes::constructed();
    assert_eq!(value.load(b"-1"), Err(ShakeLoadError::Allocation));
    assert_eq!(value.count, u32::MAX);
    assert!(value.positions.is_none() && value.rotations.is_none());
}
#[test]
fn zero_count_is_not_replaced_with_a_synthetic_frame() {
    let mut value = ShakeKeyframes::constructed();
    value.load(b"0").unwrap();
    assert!(value.positions.as_ref().unwrap().is_empty());
    assert!(value.rotations.as_ref().unwrap().is_empty());
}
