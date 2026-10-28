//! BoxVolume::LineSegIntersect82ad7170 for VolumeLineBatch's identity-local box.
//! Six-step face/edge/corner region traversal, not the formats preview slab test.
use crate::{
    collision_sphere_line::{self, SphereLineIntersection},
    collision_volume_line::{VolumeBoxHit, VolumeData},
    physical_player_grab_spline::geometry::{inverse_transform_point, transform_point},
    reckoning_numeric::{ReckoningNumeric, cross_full4, try_normalize_unsafe},
    simulation_drive_jacobian::refine_drive_reciprocal,
};
type V = [f32; 4];
#[derive(Clone, Copy)]
struct Fraction { num: f32, den: f32 }
impl Fraction {
    const ZERO: Self = Self { num: 0., den: 1. };
    fn value(self) -> f32 { self.num / self.den }
    /// Native comparison preserves later boundary on a tie.
    fn no_later_than(self, other: Self) -> bool {
        !(self.den * other.num < self.num * other.den)
    }
}
fn sub(a: V, b: V) -> V { std::array::from_fn(|i| a[i] - b[i]) }
fn advance(p: V, d: V, t: f32) -> V { std::array::from_fn(|i| d[i].mul_add(t, p[i])) }
fn axis_normal(axis: usize, sign: f32) -> V {
    let mut v = [0.; 4]; v[axis] = sign; v
}
/// Boundary plane admission used by corner/edge/face transitions.
fn boundary(distance: f32, velocity: f32) -> Option<Fraction> {
    if !(distance > 0.) { return Some(Fraction::ZERO); }
    let minimum_reciprocal = f32::from_bits(0x00200000);
    if velocity < minimum_reciprocal || velocity < distance * minimum_reciprocal
        || !(distance < velocity) { return None; }
    Some(Fraction { num: distance, den: velocity })
}
fn scaled_normal<N: ReckoningNumeric>(mut offset: V, edge: Option<usize>,
    radius: f32, overlap: bool, numeric: &mut N,
) -> Result<V, N::Error> {
    if let Some(axis) = edge { offset[axis] = 0.; }
    if overlap { return try_normalize_unsafe(offset, numeric); }
    let estimate = numeric.reciprocal_estimate(radius)?;
    let inverse = refine_drive_reciprocal(radius, estimate);
    Ok(offset.map(|v| v * inverse))
}

pub(crate) fn intersect<N: ReckoningNumeric>(volume: &VolumeData, start: V,
    end: V, fatness: f32, numeric: &mut N,
) -> Result<Option<VolumeBoxHit>, N::Error> {
    let rows = std::array::from_fn(|r| std::array::from_fn(|c| volume.transform[r * 4 + c]));
    let origin = inverse_transform_point(&rows, start);
    let end = inverse_transform_point(&rows, end);
    let Some((point, normal, parameter)) = local_intersect(origin, end,
        volume.dimensions, fatness, numeric)? else { return Ok(None); };
    let position = transform_point(&rows, point);
    let normal: V = std::array::from_fn(|i| rows[2][i].mul_add(normal[2],
        rows[1][i].mul_add(normal[1], rows[0][i] * normal[0])));
    //82ad7e58..60: move the fat-line centre back to the actual surface.
    Ok(Some(VolumeBoxHit { position: std::array::from_fn(|i| position[i] - normal[i] * fatness),
        normal, parameter }))
}

fn local_intersect<N: ReckoningNumeric>(mut point: V, end: V, half: V,
    radius: f32, numeric: &mut N,
) -> Result<Option<(V, V, f32)>, N::Error> {
    let mut delta = sub(end, point);
    let direction_sign: V = delta.map(|v| if v >= 0. { 1. } else { -1. });
    let mut side = [0.; 4];
    let mut inside = 0;
    let mut axis = 0;
    let mut nearest = -f32::MAX;
    let mut nearest_axis = 0;
    let mut nearest_sign = 1.;
    for i in 0..3 {
        for (distance, sign) in [(-half[i] - point[i], -1.), (point[i] - half[i], 1.)] {
            if distance > nearest { nearest = distance; nearest_axis = i; nearest_sign = sign; }
        }
        if point[i] < -half[i] { side[i] = -1.; }
        else if point[i] > half[i] { side[i] = 1.; }
        else {
            inside += 1;
            if inside == 1 { axis = i; }
            else if inside == 2 { axis = 3 - i - axis; }
        }
    }
    let mut parameter = 0.;
    // Original decrements from six only when moving to another region.
    for _ in 0..6 {
        if inside == 3 {
            return Ok(Some((point, axis_normal(nearest_axis, nearest_sign), parameter)));
        }
        let fraction;
        if inside == 2 {
            let distance = point[axis].mul_add(side[axis], -(half[axis] + radius));
            if !(distance > 0.) {
                return Ok(Some((point, axis_normal(axis, side[axis]), parameter)));
            }
            let velocity = -(delta[axis] * side[axis]);
            let minimum = f32::from_bits(0x00200000);
            if velocity < minimum || velocity < distance * minimum { return Ok(None); }
            let face = Fraction { num: distance, den: velocity };
            let mut candidate = (distance < velocity).then_some((axis, face));
            let mut i = (1 << axis) & 3;
            while i != axis {
                if direction_sign[i] != 0. {
                    let sign = -direction_sign[i];
                    if let Some(next) = boundary(point[i].mul_add(sign, half[i]), -(delta[i] * sign)) {
                        if candidate.is_none_or(|(_, old)| next.no_later_than(old)) { candidate = Some((i, next)); }
                    }
                }
                i = (1 << i) & 3;
            }
            let Some((next_axis, next)) = candidate else { return Ok(None); };
            if next_axis == axis {
                let t = next.value();
                return Ok(Some((advance(point, delta, t), axis_normal(axis, side[axis]),
                    (1. - parameter).mul_add(t, parameter))));
            }
            side[next_axis] = direction_sign[next_axis];
            axis = 3 - next_axis - axis;
            inside = 1;
            fraction = next;
        } else {
            let centre: V = [side[0] * half[0], side[1] * half[1], side[2] * half[2], 0.];
            if inside == 0 {
                let sphere = collision_sphere_line::intersect(point, delta, centre, radius,
                    |a, b| numeric.dot3(a, b))?;
                if matches!(sphere, SphereLineIntersection::NotApproaching) { return Ok(None); }
                let sphere = if radius > 0. { sphere.fraction().map(|[num, den]| Fraction { num, den }) } else { None };
                if sphere.is_some_and(|f| f.num == 0.) {
                    let normal = scaled_normal(sub(point, centre), None, radius, true, numeric)?;
                    return Ok(Some((point, normal, parameter)));
                }
                let mut candidate = sphere.map(|f| (None, f));
                for i in 0..3 {
                    if direction_sign[i] * side[i] < 0. {
                        if let Some(next) = boundary(point[i].mul_add(side[i], -half[i]), -(delta[i] * side[i])) {
                            if candidate.is_none_or(|(_, old)| next.no_later_than(old)) { candidate = Some((Some(i), next)); }
                        }
                    }
                }
                let Some((next_axis, next)) = candidate else { return Ok(None); };
                if let Some(i) = next_axis {
                    axis = i; side[i] = 0.; inside = 1; fraction = next;
                } else {
                    let t = next.value();
                    point = advance(point, delta, t);
                    let normal = scaled_normal(sub(point, centre), None, radius, false, numeric)?;
                    return Ok(Some((point, normal, (1. - parameter).mul_add(t, parameter))));
                }
            } else {
                // Cylinder around the edge; the axis is the one inside slab.
                let unit = axis_normal(axis, 1.);
                let offset = cross_full4(sub(centre, point), unit);
                let offset_squared = numeric.dot3(offset, offset)?;
                let mut cylinder = if offset_squared < radius * radius { Some(Fraction::ZERO) } else {
                    let travel = cross_full4(delta, unit);
                    let projection = numeric.dot3(offset, travel)?;
                    if !(projection > 0.) { return Ok(None); }
                    let length_squared = numeric.dot3(travel, travel)?;
                    let discriminant = length_squared.mul_add(radius * radius,
                        projection.mul_add(projection, -(length_squared * offset_squared)));
                    let beyond = projection - length_squared;
                    if discriminant < 0. || !(beyond < 0. || beyond * beyond < discriminant) { None }
                    else { Some(Fraction { num: projection - discriminant.sqrt(), den: length_squared }) }
                };
                if !(radius > 0.) { cylinder = None; }
                if cylinder.is_some_and(|f| f.num == 0.) {
                    let normal = scaled_normal(sub(point, centre), Some(axis), radius, true, numeric)?;
                    return Ok(Some((point, normal, parameter)));
                }
                let sign = if delta[axis] > 0. { 1. } else { -1. };
                let mut velocity = delta[axis] * sign;
                let distance = -point[axis].mul_add(sign, -half[axis]);
                if velocity < f32::MIN_POSITIVE { velocity = 1.; }
                let mut boundary_hit = (!(distance > velocity)).then_some((axis, Fraction { num: distance, den: velocity }));
                let mut i = (1 << axis) & 3;
                while i != axis {
                    if direction_sign[i] * side[i] < 0. {
                        if let Some(next) = boundary(point[i].mul_add(side[i], -half[i]), -(delta[i] * side[i])) {
                            if boundary_hit.is_none_or(|(_, old)| next.no_later_than(old)) { boundary_hit = Some((i, next)); }
                        }
                    }
                    i = (1 << i) & 3;
                }
                if let Some(hit) = cylinder {
                    if boundary_hit.is_none_or(|(_, b)| !(b.num * hit.den < b.den * hit.num)) {
                        let t = hit.value(); point = advance(point, delta, t);
                        let normal = scaled_normal(sub(point, centre), Some(axis), radius, false, numeric)?;
                        return Ok(Some((point, normal, (1. - parameter).mul_add(t, parameter))));
                    }
                }
                let Some((next_axis, next)) = boundary_hit else { return Ok(None); };
                if next_axis == axis { inside = 0; side[axis] = direction_sign[axis]; }
                else { inside = 2; axis = 3 - next_axis - axis; side[next_axis] = 0.; }
                fraction = next;
            }
        }
        let t = fraction.value();
        parameter = (1. - parameter).mul_add(t, parameter);
        point = advance(point, delta, t);
        delta = sub(end, point);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    #[test]
    fn face_hit_returns_surface_instead_of_fat_line_centre() {
        let volume = VolumeData { transform: [1.,0.,0.,0., 0.,1.,0.,0., 0.,0.,1.,0., 0.,0.,0.,1.],
            dimensions: [1.,1.,1.,0.], mask: 0 };
        let hit = intersect(&volume, [3.,0.,0.,1.], [0.,0.,0.,1.], 0.5,
            &mut HostReckoningNumeric).unwrap().unwrap();
        assert_eq!(hit.parameter, 0.5);
        assert_eq!(&hit.position[..3], &[1.,0.,0.]);
        assert_eq!(&hit.normal[..3], &[1.,0.,0.]);
    }
    #[test]
    fn rounded_corner_rejects_inflated_slab_false_positive() {
        let result = local_intersect([1.4,1.4,3.,0.], [1.4,1.4,-3.,0.],
            [1.,1.,1.,0.], 0.5, &mut HostReckoningNumeric).unwrap();
        assert!(result.is_none());
    }
    #[test]
    fn initial_interior_uses_nearest_face_with_authored_axis_ties() {
        let (_, normal, t) = local_intersect([0.;4], [0.,0.,2.,0.],
            [1.,1.,1.,0.], 0.3, &mut HostReckoningNumeric).unwrap().unwrap();
        assert_eq!(t, 0.);
        assert_eq!(normal, [-1.,0.,0.,0.]);
    }
}
