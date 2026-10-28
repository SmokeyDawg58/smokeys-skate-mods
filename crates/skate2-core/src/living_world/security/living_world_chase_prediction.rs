//! geotest::PredictIntersectAtMaxSpeed82e54ca8, preserving the two native
//! coordinate branches rather than replacing them with a textbook time solver.
//! Raw VMX VA=((word>>16)&31)|(((word>>5)&1)<<5)|(((word>>10)&1)<<6).
//! IDA's displayed VA32..63 is misleading; scalar VMADD uses A*C+B.
//! Shared host estimate/refinement boundaries do not claim Xenon bit parity.
use crate::{
    reckoning_numeric::ReckoningNumeric,
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Interception {
    pub(crate) time: f32,
    pub(crate) point: [f32; 3],
}
#[derive(Clone, Debug)]
pub(crate) struct Intersections {
    pub(crate) line_start1: [f32; 3],
    pub(crate) line_start2: [f32; 3],
    pub(crate) line_velocity1: [f32; 3],
    pub(crate) points: Vec<Interception>,
}
fn reciprocal<N: ReckoningNumeric>(value: f32, n: &mut N) -> Result<f32, N::Error> {
    Ok(refine_drive_reciprocal(
        value,
        n.reciprocal_estimate(value)?,
    ))
}
pub(crate) fn predict_intersect_at_max_speed<N: ReckoningNumeric>(
    position1: [f32; 3],
    velocity1: [f32; 3],
    position2: [f32; 3],
    speed2: f32,
    n: &mut N,
) -> Result<Intersections, N::Error> {
    let mut output = Intersections {
        line_start1: position1,
        line_start2: [position2[0], position1[1], position2[2]],
        line_velocity1: [velocity1[0], 0.0, velocity1[2]],
        points: Vec::with_capacity(2),
    };
    let dx = position1[0] - position2[0];
    let dz = position1[2] - position2[2];
    let epsilon = f32::from_bits(0x3727_c5ac);
    // 82e54d94/da8 and 82e55038/4c use STRICT dominance. Equal absolute
    // coordinate differences take neither branch, including coincident points.
    let axis = if dx.abs() > dz.abs() && dx.abs() > epsilon {
        Some((0, 2))
    } else if dz.abs() > dx.abs() && dz.abs() > epsilon {
        Some((2, 0))
    } else {
        None
    };
    let mut times = [-f32::MAX; 2];
    if let Some((dominant, other)) = axis {
        let difference = position2[dominant] - position1[dominant];
        let other_difference = position2[other] - position1[other];
        let inverse_speed = reciprocal(speed2, n)?;
        let inverse_difference = reciprocal(difference, n)?;
        let ratio = inverse_difference * other_difference;
        let scaled_velocity = velocity1[dominant] * ratio;
        let a = ratio.mul_add(ratio, 1.0);
        let twice_ratio = 2.0 * ratio;
        let scaled = inverse_speed * scaled_velocity;
        // Raw standard VMADD82e54eb0 /82e55154 is A*C+B. Preserve the
        // authored arithmetic graph, including this addition's sign.
        let c_base = inverse_speed.mul_add(velocity1[other], scaled);
        let four_a = 4.0 * a;
        let square = c_base * c_base;
        let b = twice_ratio * c_base;
        let c = square - 1.0;
        let discriminant = b * b - four_a * c;
        if discriminant >= 0.0 {
            let seed = n.reciprocal_sqrt_estimate(discriminant)?;
            let refined = refine_drive_reciprocal_sqrt(discriminant, seed);
            let square_root = if discriminant == 0.0 {
                0.0
            } else {
                discriminant * refined
            };
            let denominator = 2.0 * a;
            // Native computes two reciprocal lanes separately.
            let inverse_plus = reciprocal(denominator, n)?;
            let inverse_minus = reciprocal(denominator, n)?;
            let root_plus = inverse_plus * (-b + square_root);
            let root_minus = inverse_minus * (-b - square_root);
            let denominator_plus = velocity1[dominant] - speed2 * root_plus;
            let denominator_minus = velocity1[dominant] - speed2 * root_minus;
            if denominator_plus.abs() > epsilon {
                times[0] = reciprocal(denominator_plus, n)? * difference;
            }
            if denominator_minus.abs() > epsilon {
                times[1] = reciprocal(denominator_minus, n)? * difference;
            }
        }
    }
    // 82e55324..55438 admits nonnegative times and sorts the admitted pair.
    let ordered = if times[1] > times[0] {
        times
    } else {
        [times[1], times[0]]
    };
    for time in ordered {
        if time >= 0.0 {
            let velocity = output.line_velocity1;
            output.points.push(Interception {
                time,
                point: [
                    velocity[0].mul_add(time, position1[0]),
                    velocity[1].mul_add(time, position1[1]),
                    velocity[2].mul_add(time, position1[2]),
                ],
            });
        }
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PedestrianInterception {
    pub(crate) possible: bool,
    pub(crate) point: [f32; 3],
    pub(crate) time: f32,
}
/// Pedestrian::InterceptTarget82e7ab78. The angular gate can deliberately
/// choose the current target point while still reporting interception possible.
pub(crate) fn intercept_target<N: ReckoningNumeric>(
    position1: [f32; 3],
    velocity1: [f32; 3],
    max_chase_speed1: f32,
    position2: [f32; 3],
    max_chase_speed2: f32,
    max_time: f32,
    minimum_behind_angle_degrees: f32,
    n: &mut N,
) -> Result<PedestrianInterception, N::Error> {
    use crate::reckoning_numeric::{try_length, try_unsigned_angle};
    let mut result = PedestrianInterception {
        possible: false,
        point: position1,
        time: f32::MAX,
    };
    let angle_limit = minimum_behind_angle_degrees * f32::from_bits(0x3c8e_fa35);
    let predictions =
        predict_intersect_at_max_speed(position1, velocity1, position2, max_chase_speed2, n)?;
    let Some(first) = predictions.points.first() else {
        return Ok(result);
    };
    if max_time > first.time {
        result.possible = true;
        let delta = [
            position2[0] - position1[0],
            position2[1] - position1[1],
            position2[2] - position1[2],
            0.0,
        ];
        let velocity = [velocity1[0], velocity1[1], velocity1[2], 0.0];
        let angle = try_unsigned_angle(delta, velocity, n)?;
        if angle > angle_limit && std::f32::consts::PI - angle_limit > angle {
            result.time = first.time;
            result.point = [
                velocity1[0].mul_add(first.time, position1[0]),
                velocity1[1].mul_add(first.time, position1[1]),
                velocity1[2].mul_add(first.time, position1[2]),
            ];
        } else {
            let length = try_length(delta, n)?;
            result.time = reciprocal(max_chase_speed1, n)? * length;
        }
    }
    Ok(result)
}
