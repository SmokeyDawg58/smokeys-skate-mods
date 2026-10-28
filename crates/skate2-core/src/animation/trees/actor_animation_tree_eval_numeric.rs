//! Native ACS SQT arithmetic. Host horizontal sums/rsqrt seeds are explicit
//! numerical boundaries; the source fused graph and two refinements are kept.
use skate2_formats::andale::AndaleSqt;

pub(crate) fn blend(first: AndaleSqt, second: AndaleSqt, coefficient: f32) -> AndaleSqt {
    let positive = dot4(first.rotation, second.rotation) > 0.0;
    let rotation = std::array::from_fn(|lane| {
        if positive {
            (second.rotation[lane] - first.rotation[lane])
                .mul_add(coefficient, first.rotation[lane])
        } else {
            -((second.rotation[lane] + first.rotation[lane])
                .mul_add(coefficient, -first.rotation[lane]))
        }
    });
    AndaleSqt {
        scale: lerp(first.scale, second.scale, coefficient),
        rotation: normalize(rotation),
        translation: lerp(first.translation, second.translation, coefficient),
    }
}
fn lerp(first: [f32; 4], second: [f32; 4], coefficient: f32) -> [f32; 4] {
    std::array::from_fn(|lane| (second[lane] - first[lane]).mul_add(coefficient, first[lane]))
}
use crate::reckoning_numeric::host_dot4 as dot4;
fn dot3(first: [f32; 4], second: [f32; 4]) -> f32 {
    first[0].mul_add(second[0], first[1].mul_add(second[1], first[2] * second[2]))
}
pub(super) fn normalize(value: [f32; 4]) -> [f32; 4] {
    let squared = dot4(value, value);
    let mut estimate = squared.sqrt().recip();
    for _ in 0..2 {
        let square = estimate * estimate;
        let half = estimate * 0.5;
        let residual = -(squared.mul_add(square, -1.0));
        estimate = half.mul_add(residual, estimate);
    }
    value.map(|lane| lane * estimate)
}

/// Cyclic products use the native multiply followed by negative fused multiply
/// add; writing an ordinary cross product changes that rounding graph.
fn cross(first: [f32; 4], second: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|lane| {
        let next = (lane + 1) % 3;
        let last = (lane + 2) % 3;
        if lane == 3 {
            -(first[3].mul_add(second[3], -(first[3] * second[3])))
        } else {
            -(first[last].mul_add(second[next], -(first[next] * second[last])))
        }
    })
}
pub(crate) fn compose(first: [f32; 4], second: [f32; 4]) -> [f32; 4] {
    let crossed = cross(first, second);
    let mut result = std::array::from_fn(|lane| {
        first[lane].mul_add(second[3], second[lane].mul_add(first[3], crossed[lane]))
    });
    result[3] = first[3] * second[3] - dot3(first, second);
    result
}
fn rotate(quaternion: [f32; 4], vector: [f32; 4]) -> [f32; 4] {
    let crossed = cross(quaternion, vector);
    let inner = std::array::from_fn(|lane| quaternion[3].mul_add(vector[lane], crossed[lane]));
    let outer = cross(quaternion, inner);
    std::array::from_fn(|lane| 2.0_f32.mul_add(outer[lane], vector[lane]))
}

/// `TrajectoryCalculation82d6e2c8`; previous pose is a separate bone0 fetch.
pub(crate) fn trajectory(
    current: &mut AndaleSqt,
    previous: AndaleSqt,
    delta_rotation: [f32; 4],
    delta_translation: [f32; 3],
    channel_weight: Option<f32>,
) {
    let inverse_previous = std::array::from_fn(|lane| {
        if lane == 3 {
            previous.rotation[lane]
        } else {
            f32::from_bits(previous.rotation[lane].to_bits() ^ 0x8000_0000)
        }
    });
    let delta = [
        delta_translation[0],
        delta_translation[1],
        delta_translation[2],
        0.0,
    ];
    let rotated_delta = rotate(current.rotation, delta);
    let displacement = std::array::from_fn(|lane| {
        (rotated_delta[lane] + current.translation[lane]) - previous.translation[lane]
    });
    current.translation = rotate(inverse_previous, displacement);
    current.translation[3] = channel_weight.unwrap_or(1.0);
    current.rotation = compose(compose(delta_rotation, current.rotation), inverse_previous);
}

/// `ACSsqtBlend_job` case8, after `AddSQT` has ordered the command operands.
pub(crate) fn add_sqt(first: AndaleSqt, second: AndaleSqt, channel_from_first: bool) -> AndaleSqt {
    let translated = rotate(second.rotation, first.translation);
    let mut translation = std::array::from_fn(|lane| translated[lane] + second.translation[lane]);
    translation[3] = if channel_from_first {
        first.translation[3]
    } else {
        second.translation[3]
    };
    AndaleSqt {
        scale: std::array::from_fn(|lane| first.scale[lane] * second.scale[lane]),
        rotation: compose(second.rotation, first.rotation),
        translation,
    }
}

/// `ACSsqtBlend_job` case3. Mirror pairs are visited once, in bone order.
pub(super) fn mirror(
    pose: &mut [AndaleSqt],
    parents: &[i32],
    mirrors: &[i32],
    trajectory_use: u32,
) -> Result<(), ()> {
    if parents.len() != pose.len() || mirrors.len() != pose.len() {
        return Err(());
    }
    for index in 0..pose.len() {
        if mirrors[index] < 0 {
            continue;
        }
        let partner = usize::try_from(mirrors[index]).map_err(|_| ())?;
        if partner >= pose.len() {
            return Err(());
        }
        if partner < index {
            continue;
        }
        if trajectory_use == 1 && (index == 0 || parents[index] == 0) {
            if parents[index] == 0 {
                pose[index].rotation = compose(pose[index].rotation, [0.0, 1.0, 0.0, 0.0]);
            }
            pose[index].rotation[1] *= -1.0;
            pose[index].rotation[2] *= -1.0;
            pose[index].translation[0] *= -1.0;
        } else {
            mirror_regular(&mut pose[index]);
        }
        if partner > index {
            pose.swap(index, partner);
            mirror_regular(&mut pose[index]);
        }
    }
    Ok(())
}
fn mirror_regular(pose: &mut AndaleSqt) {
    pose.rotation[0] *= -1.0;
    pose.rotation[1] *= -1.0;
    pose.translation[2] *= -1.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pose(rotation: [f32; 4], translation: [f32; 4]) -> AndaleSqt {
        AndaleSqt {
            scale: [1.0; 4],
            rotation,
            translation,
        }
    }
    #[test]
    fn orthogonal_quaternions_take_native_negative_branch() {
        let result = blend(
            pose([1.0, 0.0, 0.0, 0.0], [0.0; 4]),
            pose([0.0, 1.0, 0.0, 0.0], [0.0; 4]),
            0.5,
        );
        assert!(result.rotation[0] > 0.0 && result.rotation[1] < 0.0);
        assert_eq!(
            result.rotation[0].to_bits(),
            (-result.rotation[1]).to_bits()
        );
    }
    #[test]
    fn trajectory_uses_previous_frame_and_preserves_authored_channel_weight() {
        let mut current = pose([0.0, 0.0, 0.0, 1.0], [7.0, 8.0, 9.0, 1.0]);
        let previous = pose([0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 3.0, 1.0]);
        trajectory(
            &mut current,
            previous,
            [0.0, 0.0, 0.0, 1.0],
            [2.0, 3.0, 4.0],
            Some(0.25),
        );
        assert_eq!(
            current.translation.map(f32::to_bits),
            [8.0_f32, 9.0, 10.0, 0.25].map(f32::to_bits)
        );
    }
    #[test]
    fn mirror_swaps_each_pair_once_and_preserves_scale_and_weight() {
        let mut values = [
            pose([1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 0.25]),
            pose([8.0, 9.0, 10.0, 11.0], [12.0, 13.0, 14.0, 0.5]),
        ];
        mirror(&mut values, &[-1, -1], &[1, 0], 0).unwrap();
        assert_eq!(
            values[0].rotation.map(f32::to_bits),
            [-8.0_f32, -9.0, 10.0, 11.0].map(f32::to_bits)
        );
        assert_eq!(
            values[1].translation.map(f32::to_bits),
            [5.0_f32, 6.0, -7.0, 0.25].map(f32::to_bits)
        );
        assert_eq!(
            values[0].scale.map(f32::to_bits),
            [1.0_f32; 4].map(f32::to_bits)
        );
    }
}
