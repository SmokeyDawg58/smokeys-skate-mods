//! Vehicle vision queries82d217c8/82d24018; shared geotest82e4e510/82e55578.
//! Source deliberately tests three sides, not a generic four-sided polygon.
use crate::{
    reckoning_numeric::{ReckoningNumeric, cross_full4},
    simulation_drive_jacobian::refine_drive_reciprocal,
};

#[derive(Debug)]
pub(crate) enum VisionGeometryError<E> {
    Numeric(E),
    DegenerateCircleLine,
}

fn sub(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn flat(p: [f32; 4]) -> [f32; 4] {
    [p[0], 0., p[2], 0.]
}

fn same_side<N: ReckoningNumeric>(
    p: [f32; 4],
    other: [f32; 4],
    a: [f32; 4],
    b: [f32; 4],
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let edge = sub(b, a);
    let first = cross_full4(edge, sub(p, a));
    let second = cross_full4(edge, sub(other, a));
    Ok(numeric.dot3(first, second)? >= 0.)
}

pub(crate) fn point_in_quad<N: ReckoningNumeric>(
    point: [f32; 4],
    quad: &[[f32; 4]; 4],
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let p = flat(point);
    Ok(same_side(p, quad[3], quad[2], quad[0], numeric)?
        && same_side(p, quad[2], quad[0], quad[1], numeric)?
        && same_side(p, quad[0], quad[1], quad[3], numeric)?)
}

/// CircleLineIntersection with BOTH output pointers null, the actual vehicle
/// caller. It rejects projections outside [0,1]; endpoint overlap alone does
/// not pass. Tangency also fails (strict distanceSquared - radiusSquared < 0).
fn circle_line<N: ReckoningNumeric>(
    center: [f32; 4],
    radius: f32,
    a: [f32; 4],
    b: [f32; 4],
    numeric: &mut N,
) -> Result<bool, VisionGeometryError<N::Error>> {
    use VisionGeometryError::{DegenerateCircleLine, Numeric};
    let center = flat(center);
    let a = flat(a);
    let edge = sub(flat(b), a);
    let from_a = sub(center, a);
    let squared = numeric.dot3(edge, edge).map_err(Numeric)?;
    if !(squared > 0.) {
        return Err(DegenerateCircleLine);
    }
    let seed = numeric.reciprocal_estimate(squared).map_err(Numeric)?;
    let projected = numeric.dot3(from_a, edge).map_err(Numeric)?;
    let t = refine_drive_reciprocal(squared, seed) * projected;
    if 0. > t || t > 1. {
        return Ok(false);
    }
    let nearest = std::array::from_fn(|i| edge[i].mul_add(t, a[i]));
    let delta = flat(sub(center, nearest));
    let radius_squared = radius * radius;
    let distance_squared = numeric.dot3(delta, delta).map_err(Numeric)?;
    Ok(0. > distance_squared - radius_squared)
}

pub(crate) fn cylinder_in_region<N: ReckoningNumeric>(
    point: [f32; 4],
    radius: f32,
    quad: &[[f32; 4]; 4],
    numeric: &mut N,
) -> Result<bool, VisionGeometryError<N::Error>> {
    for (a, b) in [(0, 2), (2, 3), (3, 1)] {
        if circle_line(point, radius, quad[a], quad[b], numeric)? {
            return Ok(true);
        }
    }
    point_in_quad(point, quad, numeric).map_err(VisionGeometryError::Numeric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[test]
    fn source_three_sides_leave_far_end_open_and_include_side_boundary() {
        let quad = [
            [-1., 0., 0., 0.],
            [1., 0., 0., 0.],
            [-1., 0., 2., 0.],
            [1., 0., 2., 0.],
        ];
        let mut n = HostReckoningNumeric;
        assert!(point_in_quad([0., 500., 100., 8.], &quad, &mut n).unwrap());
        assert!(point_in_quad([1., 0., 1., 0.], &quad, &mut n).unwrap());
        assert!(!point_in_quad([0., 0., -1., 0.], &quad, &mut n).unwrap());
    }
    #[test]
    fn circle_line_rejects_tangency_and_beyond_endpoint_without_clamping() {
        let a = [0.; 4];
        let b = [0., 0., 4., 0.];
        let mut n = HostReckoningNumeric;
        assert!(!circle_line([1., 0., 2., 0.], 1., a, b, &mut n).unwrap());
        assert!(circle_line([0.5, 100., 2., 0.], 1., a, b, &mut n).unwrap());
        assert!(!circle_line([0., 0., -0.5, 0.], 1., a, b, &mut n).unwrap());
        assert!(matches!(
            circle_line(a, 1., a, a, &mut n),
            Err(VisionGeometryError::DegenerateCircleLine)
        ));
    }
}
