//! HandPlantPositionSelector82dc3bc8: authored viable window and coping chain.
use super::geometry::{self as geo, Segment};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    grind_data_runtime::GrindDataPrimitive,
    physical_player_footplant::RetainedTrajectory,
    physical_player_grind_manager::{
        math::{GrindNum, V4, madd, neg1, scale, sub},
        view::{GrindError, error, lanes, root},
    },
    physical_player_trajectory_owners::offboard_runtime::position_at,
    physics::PointGraph,
};
type E = GrindError;

pub(crate) fn scalar(a: &AttributeRuntimeInstance, key: u64) -> Result<f32, E> {
    Ok(f32::from_bits(
        a.try_scalar_word(key, 0).map_err(error)?.unwrap_or(0),
    ))
}

pub(crate) fn graph4(a: &AttributeRuntimeInstance, offset: usize, input: f32) -> Result<f32, E> {
    let data = a
        .try_cached_layout_field(offset + 16, 32)
        .map_err(error)?
        .ok_or_else(|| error("HandPlant position graph layout absent"))?;
    let f = |i| {
        f32::from_be_bytes(
            data.bytes()[i * 4..i * 4 + 4]
                .try_into()
                .expect("graph word"),
        )
    };
    Ok(PointGraph::<4> {
        x: std::array::from_fn(f),
        y: std::array::from_fn(|i| f(i + 4)),
    }
    .evaluate(input))
}

/// ComputeViableHandPlantExtents82dbddf0. Seven authored four-point graphs
/// bound horizontal velocity and alter the rising velocity; four ballistic
/// apices, minus hand-to-apex distance, form the selection quadrilateral.
pub(crate) fn extents(
    a: &AttributeRuntimeInstance,
    p: V4,
    v: V4,
    n: &mut GrindNum<'_>,
) -> Result<Option<[V4; 4]>, E> {
    if !(v[1] > 0.) {
        return Err(error("HandPlant rising velocity assertion"));
    }
    let minimum_horizontal = graph4(a, 0x30, v[1])?;
    let maximum_horizontal = graph4(a, 0x120, v[1])?;
    let minimum_vertical = graph4(a, 0, v[1])?;
    let subtract_vertical = graph4(a, 0x90, v[1])?;
    let add_vertical = graph4(a, 0x60, v[1])?;
    let subtract_horizontal = graph4(a, 0xf0, v[1])?;
    let add_horizontal = graph4(a, 0xc0, v[1])?;
    let lo = minimum_horizontal.max(v[0].abs() - subtract_horizontal);
    let hi = maximum_horizontal.min(v[0].abs() + add_horizontal);
    let low_y = minimum_vertical.max(v[1] - subtract_vertical);
    let high_y = v[1] + add_vertical;
    let sign = |f| if v[0] > 0. { f } else { neg1(f) };
    let distance = scalar(a, 0xa046_d051_6c0b_d62b)?;
    let gravity = [0., f32::from_bits(0xc11c_cccd), 0., 0.];
    let mut out = [[0.; 4]; 4];
    for (target, (vx, vy)) in
        out.iter_mut()
            .zip([(lo, low_y), (hi, low_y), (lo, high_y), (hi, high_y)])
    {
        // _GetHeighestPosition82d89d60; same retained trajectory evaluator.
        let time = if 0. > vy {
            0.
        } else {
            n.reciprocal(gravity[1] * -1.)? * vy
        };
        let x = sign(vx);
        let trajectory = RetainedTrajectory {
            position: root(p),
            velocity: root([x, vy, 0., x]),
            gravity: root(gravity),
            time: [-1.; 4],
        };
        *target = sub(
            lanes(position_at(&trajectory, time)),
            [0., distance, 0., 0.],
        );
    }
    Ok((hi > lo).then_some(out))
}

/// FindBestHandPlantPos82dc1ea0. Select highest chain (Z breaks height ties),
/// then project the window centreline at that height onto the entire chain.
pub(crate) fn best_position(
    segments: &mut [Segment],
    q: [V4; 4],
    depth: f32,
    n: &mut GrindNum<'_>,
) -> Result<Option<(usize, V4)>, E> {
    let [a, b, c, d] = q;
    // Source orders these by the TOP edge X comparison; negative approach
    // velocity reverses the authored min/max X labels.
    let (bottom_left, bottom_right, top_left, top_right) = if d[0] > c[0] {
        (a, b, c, d)
    } else {
        (b, a, d, c)
    };
    for s in segments.iter_mut() {
        geo::clip_z(s, neg1(depth), true, n)?;
        geo::clip_z(s, 0., false, n)?;
    }
    for s in segments.iter_mut() {
        for (a, b) in [
            (top_left, top_right),
            (top_right, bottom_right),
            (bottom_right, bottom_left),
            (bottom_left, top_left),
        ] {
            geo::clip_xy(s, a, b, n)?;
        }
    }
    let mut selected = None;
    let mut height = -f32::MAX;
    let mut z = -f32::MAX;
    for (i, s) in segments.iter().enumerate().filter(|(_, s)| s.valid) {
        let p = if s.start[1] > s.end[1] {
            s.start
        } else {
            s.end
        };
        if p[1] > height + 0.1 || (p[1] > height - 0.1 && p[2] > z) {
            selected = Some(i);
            height = p[1];
            z = p[2];
        }
    }
    let Some(index) = selected else {
        return Ok(None);
    };
    let first = segments[index];
    let mut included = vec![false; segments.len()];
    included[index] = true;
    geo::find_chain(first.start, first.end, segments, &mut included, n)?;
    geo::find_chain(first.end, first.start, segments, &mut included, n)?;
    let bottom = madd(bottom_left, 0.5, scale(bottom_right, 0.5));
    let top = madd(top_left, 0.5, scale(top_right, 0.5));
    let direction = sub(top, bottom);
    if direction[1] == 0. {
        return Err(error("HandPlant centreline height assertion"));
    }
    let t = (n.reciprocal(direction[1])? * (height - bottom[1]))
        .min(1.)
        .max(0.);
    let ideal = madd(direction, t, bottom);
    // Source discards the quadrilateral's irrelevant Z/W lanes here.
    let ideal = [ideal[0], ideal[1], 0., ideal[0]];
    let mut nearest = f32::MAX;
    let mut point = [0.; 4];
    for (s, included) in segments.iter_mut().zip(included) {
        s.valid = included;
        if !included {
            continue;
        }
        let direction = sub(s.end, s.start);
        let squared = n.dot(direction, direction)?;
        let mut candidate = s.start;
        if squared > f32::from_bits(0x3780_0000) {
            let along = n.dot(sub(ideal, s.start), direction)?;
            let t = n.reciprocal(squared)? * along;
            candidate = if 0. > t {
                s.start
            } else if t > 1. {
                s.end
            } else {
                madd(direction, t, s.start)
            };
        }
        let delta = sub(ideal, candidate);
        let squared = n.dot(delta, delta)?;
        if nearest > squared {
            nearest = squared;
            point = candidate;
        }
    }
    // Source returns the initially selected primitive, even when the nearest
    // point lies on another segment in its chain. Preserve that provenance.
    Ok(Some((index, point)))
}

pub(crate) struct PositionSelection {
    pub(crate) point: V4,
    pub(crate) direction: i32,
    pub(crate) primitive: GrindDataPrimitive,
}

pub(crate) fn select(
    a: &AttributeRuntimeInstance,
    primitives: &[GrindDataPrimitive],
    p: V4,
    v: V4,
    normal: V4,
    clamp_direction: i32,
    minimum_dot: f32,
    n: &mut GrindNum<'_>,
) -> Result<Option<PositionSelection>, E> {
    if primitives.is_empty() {
        return Ok(None);
    }
    let to_world = geo::vertical_plane(normal, p, n)?;
    let from_world = geo::inverse(to_world);
    let mut filtered = Vec::new();
    let mut segments = Vec::new();
    for primitive in primitives {
        if let Some(s) =
            geo::transformed_segment(from_world, primitive.start, primitive.end, minimum_dot, n)?
        {
            if segments.len() < 40 {
                segments.push(s);
                filtered.push(primitive);
            }
        }
    }
    if segments.is_empty() {
        return Ok(None);
    }
    let p = geo::transform_point(from_world, p);
    let velocity = geo::transform_vector(from_world, v);
    let speed = n.length([velocity[0], velocity[1], 0., 0.])?;
    let mut velocity = [velocity[0], speed, 0., velocity[0]];
    let direction = |x| if x > 0. { 1 } else { 2 };
    if clamp_direction != 0 && clamp_direction != direction(velocity[0]) {
        velocity[0] = if clamp_direction == 1 {
            f32::from_bits(0x3780_0000)
        } else {
            -f32::from_bits(0x3780_0000)
        };
    }
    let Some(q) = extents(a, p, velocity, n)? else {
        return Ok(None);
    };
    let depth = scalar(a, 0x3d91_38a0_4c6d_43f6)?;
    let Some((index, point)) = best_position(&mut segments, q, depth, n)? else {
        return Ok(None);
    };
    Ok(Some(PositionSelection {
        point: geo::transform_point(to_world, point),
        direction: direction(velocity[0]),
        primitive: filtered[index].clone(),
    }))
}
