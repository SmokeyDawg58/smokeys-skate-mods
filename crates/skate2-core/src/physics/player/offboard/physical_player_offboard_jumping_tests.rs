use super::*;

#[test]
fn window_bounds_move_only_for_ordered_values_strictly_inside_the_band() {
    assert_eq!(push_lower(1.0, 0.5, 2.0), 2.0);
    assert_eq!(pull_upper(1.0, 0.5, 2.0), 0.5);
    for value in [0.5, 2.0, 3.0, f32::NAN] {
        assert_eq!(push_lower(value, 0.5, 2.0).to_bits(), value.to_bits());
        assert_eq!(pull_upper(value, 0.5, 2.0).to_bits(), value.to_bits());
    }
}

#[test]
fn fsel_takes_the_first_value_for_signed_zero_and_the_second_for_nan() {
    assert_eq!(fsel(-0.0, 1.0, 2.0), 1.0);
    assert_eq!(fsel(f32::NAN, 1.0, 2.0), 2.0);
    assert_eq!(fsel(-1.0, 1.0, 2.0), 2.0);
}

#[test]
fn stored_lanes_keep_a_missing_w_as_zero() {
    let value = V {
        xyz: [1.0, 2.0, 3.0],
        w: None,
    };
    assert_eq!(lanes(value), [1.0, 2.0, 3.0, 0.0]);
    assert_eq!(lanes(up(4.0)), [0.0, 4.0, 0.0, 0.0]);
}
