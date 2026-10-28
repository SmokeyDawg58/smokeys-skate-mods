//! Grind vector arithmetic in the native VMX operation order.
//!
//! Cross products use the shared YZXW fused graph, dot products the three-lane
//! reduction of the supplied numeric, and lengths, reciprocals and
//! normalisations the two-refinement estimate chains the grind code repeats
//! inline. Debug-build `FailAssert` sites continue as the shipped build does.
use super::view::{GrindError, error};
use crate::{
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningNumeric, cross_full4, length_from_squared_and_estimate,
        try_acos,
    },
    reckoning_sin_cos::{
        HostReckoningSinCosNumeric, ReckoningSinCosNumeric, try_cosine, try_sin_cos,
    },
    simulation_drive_jacobian::{refine_drive_reciprocal, refine_drive_reciprocal_sqrt},
};

pub(crate) type V4 = [f32; 4];
pub(crate) const ZERO: V4 = [0.0; 4];
pub(crate) const I: V4 = [1.0, 0.0, 0.0, 0.0];
pub(crate) const J: V4 = [0.0, 1.0, 0.0, 0.0];
pub(crate) const K: V4 = [0.0, 0.0, 1.0, 0.0];
/// `__real_3c8efa35` / `DEGREES_TO_RADIANS`, the grind degrees-to-radians factor.
pub(crate) const DEG_TO_RAD: f32 = f32::from_bits(0x3c8e_fa35);
/// `HALF_PI`, `PI` and `TWO_PI` of the grind geometry code.
pub(crate) const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
pub(crate) const PI: f32 = f32::from_bits(0x4049_0fdb);
pub(crate) const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

pub(crate) fn add(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(crate) fn sub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(crate) fn scale(a: V4, s: f32) -> V4 {
    a.map(|v| v * s)
}
pub(crate) fn mul(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] * b[i])
}
/// `vmaddfp` against a splat: `a * s + c` with one rounding per lane.
pub(crate) fn madd(a: V4, s: f32, c: V4) -> V4 {
    std::array::from_fn(|i| a[i].mul_add(s, c[i]))
}
/// Sign-mask `vxor` on every lane, zeros and NaN payloads included.
pub(crate) fn neg(a: V4) -> V4 {
    a.map(neg1)
}
pub(crate) fn neg1(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}
/// `vandc` with the sign mask.
pub(crate) fn abs1(v: f32) -> f32 {
    f32::from_bits(v.to_bits() & 0x7fff_ffff)
}
/// `a*b.yzx - a.yzx*b` with the second product fused, permuted back to XYZW:
/// `cross(a, b)` where `a` is the operand multiplied by the permuted other.
pub(crate) fn cross(a: V4, b: V4) -> V4 {
    cross_full4(a, b)
}
/// `fsel d, a, c, b`: `a >= 0 ? c : b`; an unordered `a` selects `b`.
pub(crate) fn fsel(a: f32, c: f32, b: f32) -> f32 {
    if a >= 0.0 { c } else { b }
}
/// `vmaxfp`/`vminfp` clamp of a dot product to [-1, 1] before `XMVectorACos`.
pub(crate) fn clamp_unit(v: f32) -> f32 {
    let lower = if -1.0 > v { -1.0 } else { v };
    if 1.0 < lower { 1.0 } else { lower }
}

/// Length with the `vsel` zero guard: an exactly zero square returns zero.
pub(crate) fn length<N: ReckoningNumeric>(v: V4, n: &mut N) -> Result<f32, N::Error> {
    let squared = n.dot3(v, v)?;
    let estimate = n.reciprocal_sqrt_estimate(squared)?;
    Ok(length_from_squared_and_estimate(squared, estimate))
}
/// `vrefp` then two fused Newton refinements.
pub(crate) fn reciprocal<N: ReckoningNumeric>(value: f32, n: &mut N) -> Result<f32, N::Error> {
    let estimate = n.reciprocal_estimate(value)?;
    Ok(refine_drive_reciprocal(value, estimate))
}
/// `vrsqrtefp` chain without a zero guard, every lane scaled.
pub(crate) fn normalize_unsafe<N: ReckoningNumeric>(v: V4, n: &mut N) -> Result<V4, N::Error> {
    let squared = n.dot3(v, v)?;
    let estimate = n.reciprocal_sqrt_estimate(squared)?;
    Ok(scale(v, refine_drive_reciprocal_sqrt(squared, estimate)))
}
/// Guarded length then refined reciprocal of that length.
pub(crate) fn normalize_by_length<N: ReckoningNumeric>(v: V4, n: &mut N) -> Result<V4, N::Error> {
    let length = length(v, n)?;
    Ok(scale(v, reciprocal(length, n)?))
}

/// `GrindUtils::CalcNormalizedGrindDir` 0x82d84548.
pub(crate) fn normalized_grind_dir<N: ReckoningNumeric>(
    start: V4,
    end: V4,
    n: &mut N,
) -> Result<V4, N::Error> {
    normalize_by_length(sub(end, start), n)
}

/// `GrindUtils::CalcUpmostNormal` 0x82d8cd88: the upward normal of `dir`,
/// flipped to non-negative Y; a zero result returns I.
pub(crate) fn upmost_normal<N: ReckoningNumeric>(dir: V4, n: &mut N) -> Result<V4, N::Error> {
    let side = cross(J, dir);
    let up = cross(side, dir);
    let length = length(up, n)?;
    if length <= 0.0 {
        return Ok(I);
    }
    let signed = if 0.0 > up[1] { neg1(length) } else { length };
    Ok(scale(up, reciprocal(signed, n)?))
}

fn perpendicular_slope<N: ReckoningNumeric>(
    tangent: V4,
    velocity: V4,
    threshold: f32,
    n: &mut N,
) -> Result<f32, N::Error> {
    let along = n.dot3(velocity, tangent)?;
    let perpendicular = sub(velocity, scale(tangent, along));
    let length = length(perpendicular, n)?;
    if length <= threshold {
        return Ok(0.0);
    }
    Ok(reciprocal(length, n)? * perpendicular[1])
}
/// `GrindUtils::CalcApproachSlopeSine` 0x82dda1a8.
pub(crate) fn approach_slope_sine<N: ReckoningNumeric>(
    tangent: V4,
    velocity: V4,
    n: &mut N,
) -> Result<f32, N::Error> {
    perpendicular_slope(tangent, velocity, f32::from_bits(0x3c23_d70a), n)
}
/// `GrindUtils::CalcVerticalApproach` 0x82dda0c0.
pub(crate) fn vertical_approach<N: ReckoningNumeric>(
    tangent: V4,
    velocity: V4,
    n: &mut N,
) -> Result<f32, N::Error> {
    perpendicular_slope(tangent, velocity, f32::from_bits(0x3a83_126f), n)
}

/// Quaternion from a unit axis and an angle, `q = (axis * sin(a/2), cos(a/2))`.
pub(crate) fn axis_quaternion<S: ReckoningSinCosNumeric>(
    axis: V4,
    angle: f32,
    s: &mut S,
) -> Result<V4, S::Error> {
    let (sine, cosine) = try_sin_cos(angle * 0.5, s)?;
    let mut q = scale(axis, sine);
    q[3] = cosine;
    Ok(q)
}
/// `v + 2 q x (q x v + w v)`, the shared quaternion rotation graph.
pub(crate) fn quaternion_rotate(q: V4, v: V4) -> V4 {
    let first = cross(q, v);
    let t: V4 = std::array::from_fn(|i| q[3].mul_add(v[i], first[i]));
    let second = cross(q, t);
    std::array::from_fn(|i| 2.0_f32.mul_add(second[i], v[i]))
}
/// `PhysicsUtility::RotateVector` 0x826fc598.
pub(crate) fn rotate_vector<S: ReckoningSinCosNumeric>(
    axis: V4,
    angle: f32,
    v: V4,
    s: &mut S,
) -> Result<V4, S::Error> {
    Ok(quaternion_rotate(axis_quaternion(axis, angle, s)?, v))
}

/// `CustomCollision::LineSegment_Triangle_Test` 0x82cf8fa0: the point where
/// segment `s`..`t` crosses triangle `p0 p1 p2`, if it does.
pub(crate) fn segment_triangle<N: ReckoningNumeric>(
    s: V4,
    t: V4,
    p0: V4,
    p1: V4,
    p2: V4,
    n: &mut N,
) -> Result<Option<V4>, N::Error> {
    let e2 = sub(p2, p0);
    let e1 = sub(p1, p0);
    let d = sub(t, s);
    let normal = cross(e1, e2);
    let denominator = neg1(n.dot3(d, normal)?);
    if denominator == 0.0 {
        return Ok(None);
    }
    let w = sub(s, p0);
    let inverse = 1.0 / denominator;
    let parameter = inverse * n.dot3(w, normal)?;
    if parameter < 0.0 || parameter > 1.0 {
        return Ok(None);
    }
    let other = 1.0 / neg1(denominator);
    let c = cross(w, d);
    let u = inverse * n.dot3(e2, c)?;
    let v = other * n.dot3(e1, c)?;
    if v + u > 1.0 || u < 0.0 || v < 0.0 {
        return Ok(None);
    }
    Ok(Some(madd(d, parameter, s)))
}

/// `GrindUtils::AdjustControlForDeadZone` 0x82dd20b8.
pub(crate) fn adjust_for_dead_zone(control: f32, dead_zone: f32) -> f32 {
    if control > dead_zone {
        (control - dead_zone) / (1.0 - dead_zone)
    } else if control < neg1(dead_zone) {
        (control + dead_zone) / (1.0 - dead_zone)
    } else {
        0.0
    }
}

/// The host numerics every grind routine threads through, with their errors
/// folded into [`GrindError`].
pub(crate) struct GrindNum<'a> {
    pub(crate) n: &'a mut HostReckoningNumeric,
    pub(crate) s: &'a mut HostReckoningSinCosNumeric,
}
impl GrindNum<'_> {
    pub(crate) fn dot(&mut self, a: V4, b: V4) -> Result<f32, GrindError> {
        self.n.dot3(a, b).map_err(error)
    }
    pub(crate) fn length(&mut self, v: V4) -> Result<f32, GrindError> {
        length(v, self.n).map_err(error)
    }
    pub(crate) fn reciprocal(&mut self, value: f32) -> Result<f32, GrindError> {
        reciprocal(value, self.n).map_err(error)
    }
    pub(crate) fn normalize_unsafe(&mut self, v: V4) -> Result<V4, GrindError> {
        normalize_unsafe(v, self.n).map_err(error)
    }
    pub(crate) fn normalize_by_length(&mut self, v: V4) -> Result<V4, GrindError> {
        normalize_by_length(v, self.n).map_err(error)
    }
    pub(crate) fn grind_dir(&mut self, start: V4, end: V4) -> Result<V4, GrindError> {
        normalized_grind_dir(start, end, self.n).map_err(error)
    }
    pub(crate) fn upmost(&mut self, dir: V4) -> Result<V4, GrindError> {
        upmost_normal(dir, self.n).map_err(error)
    }
    pub(crate) fn approach_slope_sine(&mut self, t: V4, v: V4) -> Result<f32, GrindError> {
        approach_slope_sine(t, v, self.n).map_err(error)
    }
    pub(crate) fn vertical_approach(&mut self, t: V4, v: V4) -> Result<f32, GrindError> {
        vertical_approach(t, v, self.n).map_err(error)
    }
    pub(crate) fn triangle(&mut self, s: V4, t: V4, p: [V4; 3]) -> Result<Option<V4>, GrindError> {
        segment_triangle(s, t, p[0], p[1], p[2], self.n).map_err(error)
    }
    pub(crate) fn sin_cos(&mut self, angle: f32) -> Result<(f32, f32), GrindError> {
        try_sin_cos(angle, self.s).map_err(error)
    }
    pub(crate) fn cosine(&mut self, angle: f32) -> Result<f32, GrindError> {
        try_cosine(angle, self.s).map_err(error)
    }
    pub(crate) fn acos(&mut self, value: f32) -> Result<f32, GrindError> {
        try_acos(value, self.n).map_err(error)
    }
    pub(crate) fn quaternion(&mut self, axis: V4, angle: f32) -> Result<V4, GrindError> {
        axis_quaternion(axis, angle, self.s).map_err(error)
    }
    pub(crate) fn rotate(&mut self, axis: V4, angle: f32, v: V4) -> Result<V4, GrindError> {
        rotate_vector(axis, angle, v, self.s).map_err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;

    #[test]
    fn segment_crosses_a_unit_triangle_at_its_plane() {
        let mut n = HostReckoningNumeric;
        let hit = segment_triangle(
            [0.25, 1.0, 0.25, 0.0],
            [0.25, -1.0, 0.25, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            &mut n,
        )
        .unwrap()
        .unwrap();
        assert_eq!(hit[1], 0.0);
        assert!((hit[0] - 0.25).abs() < 1e-6 && (hit[2] - 0.25).abs() < 1e-6);
        let miss = segment_triangle(
            [2.0, 1.0, 2.0, 0.0],
            [2.0, -1.0, 2.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            &mut n,
        )
        .unwrap();
        assert!(miss.is_none());
    }

    #[test]
    fn upmost_normal_points_up_and_dead_zone_rescales() {
        let mut n = HostReckoningNumeric;
        let up = upmost_normal([1.0, 0.0, 0.0, 0.0], &mut n).unwrap();
        assert!((up[1] - 1.0).abs() < 1e-6);
        assert_eq!(upmost_normal(J, &mut n).unwrap(), I);
        assert_eq!(adjust_for_dead_zone(0.1, 0.25), 0.0);
        assert!((adjust_for_dead_zone(1.0, 0.25) - 1.0).abs() < 1e-6);
        assert!((adjust_for_dead_zone(-1.0, 0.25) + 1.0).abs() < 1e-6);
    }
}
