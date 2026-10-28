//! Shared rwcSphereLineSegIntersect82acdef0 decision/fraction graph.
use crate::reckoning_numeric::cross_full4;

/// Keep the native -1 (not approaching) distinct from 0 (miss).
pub(crate) enum SphereLineIntersection {
    NotApproaching,
    Miss,
    Inside,
    Entry { projection: f32, length_squared: f32, discriminant: f32 },
}
impl SphereLineIntersection {
    pub(crate) fn intersects(&self) -> bool {
        matches!(self, Self::Inside | Self::Entry { .. })
    }
    /// Native Fraction is numerator/denominator, not an eagerly divided t.
    /// Boolean-only grab queries need not evaluate the otherwise unused sqrt.
    pub(crate) fn fraction(&self) -> Option<[f32; 2]> {
        match *self {
            Self::Inside => Some([0.0, 1.0]),
            Self::Entry { projection, length_squared, discriminant } =>
                Some([projection - discriminant.sqrt(), length_squared]),
            _ => None,
        }
    }
}

pub(crate) fn intersect<E>(
    origin: [f32; 4], segment: [f32; 4], centre: [f32; 4], radius: f32,
    mut dot: impl FnMut([f32; 4], [f32; 4]) -> Result<f32, E>,
) -> Result<SphereLineIntersection, E> {
    use SphereLineIntersection::*;
    let to_centre = std::array::from_fn(|i| centre[i] - origin[i]);
    let radius_squared = radius * radius;
    // Raw bge at82acdf18 tests LT clear (including unordered).
    if dot(to_centre, to_centre)? < radius_squared { return Ok(Inside); }
    let projection = dot(to_centre, segment)?;
    if !(projection > 0.0) { return Ok(NotApproaching); }
    let length_squared = dot(segment, segment)?;
    let perpendicular = cross_full4(to_centre, segment);
    let discriminant = length_squared.mul_add(radius_squared, -dot(perpendicular, perpendicular)?);
    if discriminant < 0.0 { return Ok(Miss); }
    let beyond = projection - length_squared;
    if beyond > 0.0 && beyond * beyond > discriminant { return Ok(Miss); }
    Ok(Entry { projection, length_squared, discriminant })
}
