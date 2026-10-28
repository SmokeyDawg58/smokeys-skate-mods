//! HandPlantPositionSelector geometry, reusing the existing VMX-order numerics.
use crate::{
    physical_player_grind_manager::{
        math::{GrindNum, J, V4, ZERO, cross, madd, neg1, scale, sub},
        view::{GrindError, error},
    },
    reckoning_numeric::{try_normalize_angle, try_normalize_safe, try_unsigned_angle},
};

type E = GrindError;
const SMALL: f32 = f32::from_bits(0x3780_0000); // SMALL_FLOAT_165,82351270

#[derive(Clone, Copy, Debug)]
pub(crate) struct Segment {
    pub(crate) start: V4,
    pub(crate) end: V4,
    pub(crate) valid: bool,
}

/// GetVerticalPlaneToWorld82db86a8: X=normalize(up x normal), Y=up,
/// Z=X x up, W=the actual launch COM. Flat normals are a native assertion.
pub(crate) fn vertical_plane(normal: V4, origin: V4, n: &mut GrindNum<'_>) -> Result<[V4; 4], E> {
    if normal[1].abs() > 0.95 {
        return Err(error("HandPlant vertical-plane flat normal assertion"));
    }
    let x = n.normalize_unsafe(cross(J, normal))?;
    Ok([x, J, cross(x, J), origin])
}

/// Source affine inverse uses transposed axes, then three multiply-adds.
pub(crate) fn inverse(m: [V4; 4]) -> [V4; 4] {
    let mut out = [[0.; 4]; 4];
    for row in 0..3 {
        for col in 0..3 {
            out[row][col] = m[col][row];
        }
    }
    let t = m[3].map(neg1);
    out[3] = madd(out[0], t[0], madd(out[1], t[1], scale(out[2], t[2])));
    out
}

pub(crate) fn transform_point(m: [V4; 4], p: V4) -> V4 {
    madd(m[2], p[2], madd(m[1], p[1], madd(m[0], p[0], m[3])))
}
pub(crate) fn transform_vector(m: [V4; 4], p: V4) -> V4 {
    madd(m[2], p[2], madd(m[1], p[1], scale(m[0], p[0])))
}

/// TransformGrindPrimitives82db87c0 filters on absolute local X direction.
pub(crate) fn transformed_segment(
    m: [V4; 4],
    start: V4,
    end: V4,
    minimum_dot: f32,
    n: &mut GrindNum<'_>,
) -> Result<Option<Segment>, E> {
    let start = transform_point(m, start);
    let end = transform_point(m, end);
    let direction = try_normalize_safe(sub(end, start), ZERO, n.n).map_err(error)?;
    Ok((direction[0].abs() > minimum_dot).then_some(Segment {
        start,
        end,
        valid: true,
    }))
}

/// ClipToZPlane82db89f8. POS retains z>plane; NEG retains z<plane.
/// Both endpoints exactly on the plane invalidate the segment.
pub(crate) fn clip_z(
    s: &mut Segment,
    plane: f32,
    positive: bool,
    n: &mut GrindNum<'_>,
) -> Result<(), E> {
    if !s.valid {
        return Ok(());
    }
    let rejected = |z| if positive { plane >= z } else { z >= plane };
    if rejected(s.start[2]) && rejected(s.end[2]) {
        s.valid = false;
        return Ok(());
    }
    let dz = s.end[2] - s.start[2];
    if SMALL > dz.abs() {
        return Ok(());
    }
    let t = n.reciprocal(dz)? * (plane - s.start[2]);
    if t > 0.0 && 1.0 > t {
        let point = madd(sub(s.end, s.start), t, s.start);
        if rejected(s.end[2]) {
            s.end = point;
        } else {
            s.start = point;
        }
    }
    Ok(())
}

/// ClipOnXY82db8ba0: normalized edge normal (dy,-dx), retain distance>=0.
pub(crate) fn clip_xy(s: &mut Segment, a: V4, b: V4, n: &mut GrindNum<'_>) -> Result<(), E> {
    if !s.valid {
        return Ok(());
    }
    let edge = sub(b, a);
    if edge[0] == 0. && edge[1] == 0. {
        return Err(error("HandPlant clipping edge assertion"));
    }
    let normal = n.normalize_unsafe([edge[1], neg1(edge[0]), 0., 0.])?;
    let distance = |p: V4| {
        let d = sub(p, a);
        let x = d[0] * normal[0];
        let y = d[1] * normal[1];
        x + y
    };
    let d0 = distance(s.start);
    let d1 = distance(s.end);
    if 0. > d0 && 0. > d1 {
        s.valid = false;
    } else if !(d0 >= 0. && d1 >= 0.) {
        let difference = d0 - d1;
        if difference == 0. {
            return Err(error("HandPlant clipping distance assertion"));
        }
        let t = n.reciprocal(difference)? * d0;
        let point = madd(sub(s.end, s.start), t, s.start);
        if 0. > d0 {
            s.start = point;
        } else {
            s.end = point;
        }
    }
    Ok(())
}

/// SimilarDirection82db8e50 and its inline copy in FindChain82dbec08.
fn similar(a: V4, b: V4, n: &mut GrindNum<'_>) -> Result<bool, E> {
    let angle = try_unsigned_angle(a, b, n.n).map_err(error)?;
    let angle = try_normalize_angle(angle, n.n).map_err(error)?;
    Ok(f32::from_bits(0x3f06_0a92) > angle.abs())
}

/// FindChain82dbec08: choose the first eligible continuation in source order,
/// restart scanning after each acceptance, preserving the opposite-end call.
pub(crate) fn find_chain(
    mut start: V4,
    mut end: V4,
    segments: &[Segment],
    included: &mut [bool],
    n: &mut GrindNum<'_>,
) -> Result<(), E> {
    loop {
        let mut found = false;
        for (i, s) in segments.iter().enumerate() {
            if !s.valid || included[i] {
                continue;
            }
            let direction = sub(end, start);
            let delta = sub(end, s.start);
            let reverse = if 0.0025 > n.dot(delta, delta)? {
                if !similar(direction, sub(s.end, s.start), n)? {
                    continue;
                }
                false
            } else {
                let delta = sub(end, s.end);
                if !(0.0025 > n.dot(delta, delta)?) || !similar(direction, sub(s.start, s.end), n)?
                {
                    continue;
                }
                true
            };
            (start, end) = if reverse {
                (s.end, s.start)
            } else {
                (s.start, s.end)
            };
            included[i] = true;
            found = true;
            break;
        }
        if !found {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        reckoning_numeric::HostReckoningNumeric, reckoning_sin_cos::HostReckoningSinCosNumeric,
    };
    #[test]
    fn clipping_preserves_crossing_point_and_rejects_plane_only_segments() {
        let mut host = HostReckoningNumeric;
        let mut trig = HostReckoningSinCosNumeric;
        let mut n = GrindNum {
            n: &mut host,
            s: &mut trig,
        };
        let mut segment = Segment {
            start: [-2., 1., -2., 0.],
            end: [2., 1., 2., 0.],
            valid: true,
        };
        clip_z(&mut segment, 0., false, &mut n).unwrap();
        assert!(segment.valid);
        assert!(
            segment.end[..3]
                .iter()
                .zip([0., 1., 0.])
                .all(|(a, b)| (a - b).abs() < 1e-5)
        );
        let mut planar = Segment {
            start: [0., 0., 0., 0.],
            end: [1., 0., 0., 0.],
            valid: true,
        };
        clip_z(&mut planar, 0., false, &mut n).unwrap();
        assert!(!planar.valid);
    }
    #[test]
    fn coping_chain_does_not_cross_a_right_angle_or_a_gap() {
        let mut host = HostReckoningNumeric;
        let mut trig = HostReckoningSinCosNumeric;
        let mut n = GrindNum {
            n: &mut host,
            s: &mut trig,
        };
        let segments = [
            Segment {
                start: [1., 0., 0., 0.],
                end: [2., 0., 0., 0.],
                valid: true,
            },
            Segment {
                start: [2., 0., 0., 0.],
                end: [2., 1., 0., 0.],
                valid: true,
            },
            Segment {
                start: [2.2, 0., 0., 0.],
                end: [3., 0., 0., 0.],
                valid: true,
            },
        ];
        let mut included = [false; 3];
        find_chain([0.; 4], [1., 0., 0., 0.], &segments, &mut included, &mut n).unwrap();
        assert_eq!(included, [true, false, false]);
    }
}
