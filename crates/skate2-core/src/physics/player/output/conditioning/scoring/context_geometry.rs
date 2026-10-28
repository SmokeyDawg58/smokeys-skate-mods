//! Original scoring-only context tests82df3e48/3f28/deeea0/dea9b8.
//! These consume existing PhysOut probes; they do not issue collision queries.
use super::{scoring_attributes::ScoreError, scoring_external::NearbyDmo};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnderOver {
    Under,
    Over,
    Neither,
}

fn over_point(
    skater: SkeletonRootVector,
    point: SkeletonRootVector,
    min_y: f32,
    max_y: f32,
    radius_squared: f32,
) -> Result<UnderOver, ScoreError> {
    let mut delta = point.zip(skater, |a, b| a - b);
    delta.xyz[1] = 0.0;
    let distance = HostReckoningNumeric
        .dot_vector(delta, delta)
        .map_err(ScoreError::Numeric)?;
    if distance >= radius_squared {
        return Ok(UnderOver::Neither);
    }
    Ok(if skater.xyz[1] > max_y {
        UnderOver::Over
    } else if min_y > skater.xyz[1] {
        UnderOver::Under
    } else {
        UnderOver::Neither
    })
}
pub(super) fn over_points(
    skater: SkeletonRootVector,
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    radius_squared: f32,
) -> Result<UnderOver, ScoreError> {
    let min_y = if b.xyz[1] > a.xyz[1] {
        a.xyz[1]
    } else {
        b.xyz[1]
    };
    let max_y = if a.xyz[1] > b.xyz[1] {
        a.xyz[1]
    } else {
        b.xyz[1]
    };
    let first = over_point(skater, a, min_y, max_y, radius_squared)?;
    if first != UnderOver::Neither {
        Ok(first)
    } else {
        over_point(skater, b, min_y, max_y, radius_squared)
    }
}

/// Native transpose followed by MUL / MADD / MADD; no reciprocal or scaling.
pub(super) fn project_to_basis(matrix: &SkeletonRootMatrix, delta: SkeletonRootVector) -> [f32; 3] {
    std::array::from_fn(|i| {
        let basis = matrix.columns()[i].xyz;
        let xy = basis[1].mul_add(delta.xyz[1], basis[0] * delta.xyz[0]);
        basis[2].mul_add(delta.xyz[2], xy)
    })
}
fn ray_aabb(
    point: [f32; 3],
    direction: [f32; 3],
    minimum: [f32; 3],
    maximum: [f32; 3],
    near: &mut f32,
    far: &mut f32,
) -> bool {
    for i in 0..3 {
        if direction[i].abs() >= f32::from_bits(0x3586_37bd) {
            let inv = 1.0 / direction[i];
            let mut a = (minimum[i] - point[i]) * inv;
            let mut b = (maximum[i] - point[i]) * inv;
            // Raw BLE branch swaps unordered operands too.
            if !(a <= b) {
                std::mem::swap(&mut a, &mut b);
            }
            *near = if *near - a >= 0.0 { *near } else { a };
            *far = if *far - b >= 0.0 { b } else { *far };
            if *near > *far {
                return false;
            }
        } else if point[i] < minimum[i] || point[i] > maximum[i] {
            return false;
        }
    }
    true
}
pub(super) fn above_dmo(
    actor: SkeletonRootVector,
    ground: SkeletonRootVector,
    object: &NearbyDmo,
) -> UnderOver {
    let origin = object.transform.columns()[3];
    let relative = actor.zip(origin, |a, b| a - b);
    let start = relative.map(|x| x + 0.0);
    let end = relative.zip(SkeletonRootVector::known([0.0, -10.0, 0.0, 0.0]), |a, b| {
        a + b
    });
    let a = project_to_basis(&object.transform, start);
    let b = project_to_basis(&object.transform, end);
    let direction = std::array::from_fn(|i| b[i] - a[i]);
    let mut near = 0.0;
    let mut far = 1.0;
    if !ray_aabb(
        a,
        direction,
        object.minimum.xyz,
        object.maximum.xyz,
        &mut near,
        &mut far,
    ) {
        return UnderOver::Neither;
    }
    // Source compares the existing ground hit to the ray's FAR intersection,
    // reconstructed in world space. Bounds overlap alone does not score.
    let hit_y = (end.xyz[1] - start.xyz[1]).mul_add(far, origin.xyz[1] + start.xyz[1]);
    if !(ground.xyz[1] - hit_y <= f32::from_bits(0x3a83_126f)) {
        UnderOver::Over
    } else {
        UnderOver::Neither
    }
}
