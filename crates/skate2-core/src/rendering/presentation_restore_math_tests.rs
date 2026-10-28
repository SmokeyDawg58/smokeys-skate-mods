use super::*;

#[test]
fn xna_polynomials_follow_the_functions_they_approximate() {
    for step in -40..=40 {
        let angle = step as f32 * 0.1;
        assert!((xm_sin(angle) - angle.sin()).abs() < 2e-6, "sin {angle}");
        assert!((xm_cos(angle) - angle.cos()).abs() < 2e-6, "cos {angle}");
    }
    for step in -20..=20 {
        let value = step as f32 * 0.05;
        assert!((xm_acos(value) - value.acos()).abs() < 1e-4, "acos {value}");
    }
}

#[test]
fn five_degree_threshold_is_the_native_constant() {
    assert!((xm_cos(MIN_SLERP_ANGLE) - 5.0_f32.to_radians().cos()).abs() < 1e-6);
}

fn axis_angle(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let (sine, cosine) = (angle * 0.5).sin_cos();
    [axis[0] * sine, axis[1] * sine, axis[2] * sine, cosine]
}

fn angle_between(a: [f32; 4], b: [f32; 4]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(a, b)| a * b).sum();
    2.0 * dot.abs().min(1.0).acos()
}

#[test]
fn slerp_reaches_both_ends_on_either_path() {
    for angle in [0.02, 1.5] {
        let from = axis_angle([0.0, 1.0, 0.0], 0.3);
        let to = axis_angle([0.0, 1.0, 0.0], 0.3 + angle);
        assert!(angle_between(slerp(from, to, 0.0), from) < 1e-3);
        assert!(angle_between(slerp(from, to, 1.0), to) < 1e-3);
        let middle = slerp(from, to, 0.5);
        assert!(angle_between(middle, axis_angle([0.0, 1.0, 0.0], 0.3 + angle * 0.5)) < 1e-3);
    }
}

#[test]
fn slerp_takes_the_shorter_arc() {
    let from = axis_angle([1.0, 0.0, 0.0], 0.2);
    let to = axis_angle([1.0, 0.0, 0.0], 0.4).map(|lane| -lane);
    let middle = slerp(from, to, 0.5);
    assert!(angle_between(middle, axis_angle([1.0, 0.0, 0.0], 0.3)) < 1e-3);
}

fn yaw(angle: f32, translation: [f32; 3]) -> AnimationMatrix {
    let (sine, cosine) = angle.sin_cos();
    let rows = [
        [cosine, 0.0, -sine, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sine, 0.0, cosine, 0.0],
        [translation[0], translation[1], translation[2], 1.0],
    ];
    std::array::from_fn(|index| rows[index / 4][index % 4])
}

fn heading(matrix: &AnimationMatrix) -> f32 {
    (-matrix[2]).atan2(matrix[0])
}

#[test]
fn matrix_slerp_returns_its_ends() {
    let from = yaw(0.2, [3.0, 0.0, 4.0]);
    let to = yaw(1.0, [5.0, 0.0, 4.0]);
    assert_eq!(matrix_slerp(&from, &to, 0.0).unwrap(), from);
    assert_eq!(matrix_slerp(&from, &to, 1.0).unwrap(), to);
}

#[test]
fn matrix_slerp_turns_about_the_axis_at_the_origin() {
    let from = yaw(0.2, [0.0; 3]);
    let to = yaw(1.0, [0.0; 3]);
    let middle = matrix_slerp(&from, &to, 0.5).unwrap();
    for (lane, (value, expected)) in middle.iter().zip(yaw(0.6, [0.0; 3])).enumerate() {
        assert!(
            (value - expected).abs() < 1e-4,
            "{lane}: {value} {expected}"
        );
    }
}

#[test]
fn far_from_the_origin_the_rows_lerp_and_normalize() {
    let from = yaw(0.0, [1000.0, 0.0, 1000.0]);
    let to = yaw(0.3, [1000.5, 0.0, 1000.0]);
    let middle = matrix_slerp(&from, &to, 0.5).unwrap();
    assert!(
        (heading(&middle) - 0.15).abs() < 1e-4,
        "{}",
        heading(&middle)
    );
    assert!((middle[12] - 1000.25).abs() < 1e-3);
    for row in 0..3 {
        let length: f32 = middle[row * 4..row * 4 + 3]
            .iter()
            .map(|lane| lane * lane)
            .sum();
        assert!((length - 1.0).abs() < 1e-5, "{row}");
    }
}
