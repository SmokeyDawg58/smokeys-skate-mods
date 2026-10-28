//! Region walk `FatTriangleLineSegIntersect` 0x82adeb48; source scratch and
//! published result remain distinct. Numeric ports preserve refusal prefixes.
use super::{
    CollisionTriangleLineInput, CollisionTriangleLineOutput, ReckoningNumeric, SkeletonRootVector,
    cross, scratch, subtract,
};
use crate::simulation_drive_jacobian::refine_drive_reciprocal;
#[cfg(test)]
#[path = "collision_fat_triangle_line_tests.rs"]
mod tests;
const SMALL: f32 = f32::from_bits(0x0020_0000);
type V = SkeletonRootVector;

#[derive(Clone, Copy)]
struct Fraction {
    num: f32,
    den: f32,
}
impl Fraction {
    fn value(self) -> f32 {
        self.num / self.den
    }
}

pub(super) fn scale(v: V, s: f32) -> V {
    V {
        xyz: v.xyz.map(|x| x * s),
        w: v.w.map(|x| x * s),
    }
}
fn add(a: V, b: V) -> V {
    V {
        xyz: std::array::from_fn(|i| a.xyz[i] + b.xyz[i]),
        w: a.w.zip(b.w).map(|(a, b)| a + b),
    }
}
fn advance(a: V, d: V, t: f32) -> V {
    V {
        xyz: std::array::from_fn(|i| d.xyz[i].mul_add(t, a.xyz[i])),
        w: d.w.zip(a.w).map(|(d, a)| d.mul_add(t, a)),
    }
}
fn dot<N: ReckoningNumeric>(numeric: &mut N, a: V, b: V) -> Result<f32, N::Error> {
    numeric.dot3(scratch(a), scratch(b))
}
fn gt(a: f32, b: f32) -> bool {
    a > b
}
fn lt(a: f32, b: f32) -> bool {
    a < b
}
fn abs_sum(v: V) -> f32 {
    (v.xyz[1].abs() + v.xyz[2].abs()) + v.xyz[0].abs()
}

/// Source 0x82ad9a40; route selection precedes final region/barycentric choice.
#[expect(
    clippy::many_single_char_names,
    reason = "a through e are the nearest-triangle quadratic coefficients; u/v are barycentric coordinates"
)]
fn nearest<N: ReckoningNumeric>(
    point: V,
    vertices: [V; 3],
    numeric: &mut N,
) -> Result<(V, f32, f32, u32), N::Error> {
    let [origin, bv, cv] = vertices;
    let e1 = subtract(bv, origin);
    let e2 = subtract(cv, origin);
    let offset = subtract(origin, point);
    let a = dot(numeric, e1, e1)?;
    let c = dot(numeric, e2, e2)?;
    let d = dot(numeric, e1, offset)?;
    let e = dot(numeric, e2, offset)?;
    let b = dot(numeric, e1, e2)?;
    let det = c.mul_add(a, -(b * b));
    let un = e.mul_add(b, -(d * c));
    let vn = d.mul_add(b, -(e * a));
    let opposite = || -(b.mul_add(2.0, -a)) + c;
    let route = if lt(det, SMALL) {
        let length = opposite();
        if gt(a, c) {
            if gt(a, length) { 0 } else { 2 }
        } else if gt(c, length) {
            1
        } else {
            2
        }
    } else if gt(vn + un, det) {
        if lt(un, 0.0) {
            if lt(e + c, d + b) { 1 } else { 2 }
        } else if lt(vn, 0.0) && lt(d + a, e + b) {
            0
        } else {
            2
        }
    } else if lt(un, 0.0) {
        i32::from(gt(-e, 0.0))
    } else if lt(vn, 0.0) {
        i32::from(!gt(-d, 0.0))
    } else {
        3
    };
    let (u, v, region) = match route {
        0 => {
            if !lt(d, 0.0) {
                (0.0, 0.0, 0)
            } else if lt(-d, a) {
                (-(d / a), 0.0, 3)
            } else {
                (1.0, 0.0, 1)
            }
        }
        1 => {
            if !lt(e, 0.0) {
                (0.0, 0.0, 0)
            } else if lt(-e, c) {
                (0.0, -(e / c), 4)
            } else {
                (0.0, 1.0, 2)
            }
        }
        2 => {
            let numerator = ((e + c) - b) - d;
            if gt(numerator, 0.0) {
                let length = opposite();
                if lt(numerator, length) {
                    let u = numerator / length;
                    (u, 1.0 - u, 5)
                } else {
                    (1.0, 0.0, 1)
                }
            } else {
                (0.0, 1.0, 2)
            }
        }
        _ => {
            let inverse = 1.0 / det;
            (inverse * un, inverse * vn, 6)
        }
    };
    Ok((advance(advance(origin, e1, u), e2, v), u, v, region))
}

/// Source 0x82acdef0. -1 and 0 have different region-walk meanings.
fn sphere<N: ReckoningNumeric>(
    fraction: &mut Fraction,
    origin: V,
    delta: V,
    center: V,
    radius: f32,
    numeric: &mut N,
) -> Result<i32, N::Error> {
    let c = subtract(center, origin);
    let radius2 = radius * radius;
    let distance2 = dot(numeric, c, c)?;
    if lt(distance2, radius2) {
        *fraction = Fraction { num: 0.0, den: 1.0 };
        return Ok(1);
    }
    let approach = dot(numeric, c, delta)?;
    if !gt(approach, 0.0) {
        return Ok(-1);
    }
    let length2 = dot(numeric, delta, delta)?;
    let cross_offset = cross(c, delta);
    let cross2 = dot(numeric, cross_offset, cross_offset)?;
    let discriminant = length2.mul_add(radius2, -cross2);
    if lt(discriminant, 0.0) {
        return Ok(0);
    }
    let beyond = approach - length2;
    if gt(beyond, 0.0) && gt(beyond * beyond, discriminant) {
        return Ok(0);
    }
    let root = discriminant.sqrt();
    fraction.den = length2;
    fraction.num = approach - root;
    Ok(1)
}

fn boundary(fraction: &mut Fraction, has_candidate: bool, num: f32, den: f32) -> bool {
    if !gt(den, SMALL) || !gt(num, 0.0) {
        return false;
    }
    if has_candidate && lt(den * fraction.num, num * fraction.den) {
        return false;
    }
    *fraction = Fraction { num, den };
    true
}

#[expect(
    clippy::too_many_lines,
    reason = "one source region walk shares fraction, point, and published failure prefixes across vertex/edge transitions"
)]
pub(super) fn intersect<N: ReckoningNumeric>(
    input: &CollisionTriangleLineInput,
    radius: f32,
    out: &mut CollisionTriangleLineOutput,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    out.line_parameter = Some(0.0);
    let [vertex_a, vertex_b, vertex_c] = input.vertices;
    let e1 = subtract(vertex_b, vertex_a);
    let e2 = subtract(vertex_c, vertex_a);
    let mut delta = input.delta;
    let mut offset = subtract(input.start, vertex_a);
    let perpendicular_u = cross(delta, e2);
    let mut determinant = dot(numeric, e1, perpendicular_u)?;
    let sign = if lt(determinant, 0.0) {
        determinant = -determinant;
        -1.0
    } else {
        1.0
    };
    let normal = out
        .normal
        .expect("wrapper publishes normal before fat helper");
    if lt(dot(numeric, normal, offset)? * sign, -radius) {
        return Ok(false);
    }
    let u = dot(numeric, offset, perpendicular_u)? * sign;
    let slack_u = abs_sum(perpendicular_u) * radius;
    if lt(u, -slack_u) || gt(u, slack_u + determinant) {
        return Ok(false);
    }
    let perpendicular_v = cross(e1, delta);
    let v = dot(numeric, offset, perpendicular_v)? * sign;
    let slack_v = abs_sum(perpendicular_v) * radius;
    if lt(v, -slack_v)
        || gt(v, slack_v + determinant)
        || gt(v + u, (slack_u + slack_v) + determinant)
    {
        return Ok(false);
    }
    if gt(determinant, SMALL) {
        let shifted = subtract(offset, scale(normal, sign * radius));
        let numerator = -(dot(numeric, shifted, cross(e2, e1))? * sign);
        if gt(numerator, determinant) {
            return Ok(false);
        }
        if !lt(numerator, 0.0) {
            // ed78 reduction precedes ed88's parameter publication; scaling
            // its captured result by the orientation sign happens afterward.
            let shifted_u = dot(numeric, shifted, perpendicular_u)?;
            let reciprocal = 1.0 / determinant;
            let t = reciprocal * numerator;
            out.line_parameter = Some(t);
            let u = shifted_u * sign;
            if !lt(u, 0.0) && !gt(u, determinant) {
                let v = dot(numeric, shifted, perpendicular_v)? * sign;
                if !lt(v, 0.0) && !gt(v + u, determinant) {
                    out.position = Some(advance(input.start, delta, t));
                    out.normal = Some(scale(normal, sign));
                    out.volume_parameter = Some([reciprocal * u, reciprocal * v, 0.0, 0.0]);
                    return Ok(true);
                }
            }
            offset = advance(offset, delta, t);
            delta = scale(delta, 1.0 - t);
        }
    }
    let mut start = add(offset, vertex_a);
    let (mut closest, u, v, mut region) = nearest(start, input.vertices, numeric)?;
    let diff = subtract(start, closest);
    let distance2 = dot(numeric, diff, diff)?;
    let overlap = radius.mul_add(radius, -distance2);
    if gt(overlap, 0.0) {
        let squared = dot(numeric, diff, diff)?;
        out.position = Some(start);
        let seed = numeric.reciprocal_sqrt_estimate(squared)?;
        let square = seed * seed;
        let residual = (-squared).mul_add(square, 1.0);
        let inverse = (seed * 0.5).mul_add(residual, seed);
        out.normal = Some(if gt(squared, 0.0) {
            scale(diff, inverse)
        } else {
            normal
        });
        out.volume_parameter = Some([u, v, overlap, 0.0]);
        return Ok(true);
    }
    if region == 6 {
        out.position = Some(start);
        // ef74 retains f2: the sign-adjusted determinant, not stack50 sign.
        if lt(determinant, 0.0) {
            out.normal = Some(scale(normal, -1.0));
        }
        out.volume_parameter = Some([u, v, 0.0, 0.0]);
        return Ok(true);
    }
    let mut fraction = Fraction {
        num: sign,
        den: 0.0,
    };
    for remaining in (0..5).rev() {
        if region <= 2 {
            let vertex = region;
            let mut selected = sphere(&mut fraction, start, delta, closest, radius, numeric)?;
            if selected < 0 {
                return Ok(false);
            }
            let (edge_a, edge_b) = match vertex {
                0 => (subtract(vertex_b, vertex_a), subtract(vertex_c, vertex_a)),
                1 => (subtract(vertex_a, vertex_b), subtract(vertex_c, vertex_b)),
                _ => (subtract(vertex_a, vertex_c), subtract(vertex_b, vertex_c)),
            };
            for (edge, addition) in [(edge_a, 6), (edge_b, 9)] {
                let den = dot(numeric, delta, edge)?;
                if gt(den, SMALL) {
                    let num = dot(numeric, subtract(closest, start), edge)?;
                    if boundary(&mut fraction, selected != 0, num, den) {
                        selected = i32::try_from(vertex.midpoint(addition)).expect("region <=5");
                    }
                }
            }
            if selected == 0 {
                return Ok(false);
            }
            if selected <= 2 {
                let t = fraction.value();
                let seed = numeric.reciprocal_estimate(radius)?;
                let old = out.line_parameter.expect("entry parameter");
                let position = advance(start, delta, t);
                out.line_parameter = Some((1.0 - old).mul_add(t, old));
                out.position = Some(position);
                let raw = subtract(position, closest);
                out.normal = Some(raw);
                out.normal = Some(scale(raw, refine_drive_reciprocal(radius, seed)));
                out.volume_parameter = Some([
                    if vertex == 1 { 1.0 } else { 0.0 },
                    if vertex == 2 { 1.0 } else { 0.0 },
                    0.0,
                    0.0,
                ]);
                return Ok(true);
            }
            region = u32::try_from(selected).expect("positive region");
        } else {
            let (edge_origin, edge) = match region {
                3 => (vertex_a, subtract(vertex_b, vertex_a)),
                4 => (vertex_a, subtract(vertex_c, vertex_a)),
                _ => (vertex_b, subtract(vertex_c, vertex_b)),
            };
            closest = edge_origin;
            let from_start = subtract(edge_origin, start);
            let length2 = dot(numeric, edge, edge)?;
            let cross_offset = cross(from_start, edge);
            let distance2 = dot(numeric, cross_offset, cross_offset)?;
            let radius_term = (length2 * radius) * radius;
            let mut candidate;
            if lt(distance2, radius_term) {
                fraction = Fraction { num: 0.0, den: 1.0 };
                candidate = true;
            } else {
                let direction_cross = cross(delta, edge);
                let approach = dot(numeric, cross_offset, direction_cross)?;
                if !gt(approach, 0.0) {
                    return Ok(false);
                }
                let quadratic = dot(numeric, direction_cross, direction_cross)?;
                let inner = approach.mul_add(approach, -(quadratic * distance2));
                let disc = quadratic.mul_add(radius_term, inner);
                candidate = false;
                if !lt(disc, 0.0) {
                    let beyond = approach - quadratic;
                    if lt(beyond, 0.0) || lt(beyond * beyond, disc) {
                        let root = disc.sqrt();
                        fraction.den = quadratic;
                        fraction.num = approach - root;
                        candidate = true;
                    }
                }
            }
            let den = dot(numeric, delta, edge)?;
            let mut transitioned = false;
            if gt(den, SMALL) {
                let endpoint = add(edge_origin, edge);
                let num = dot(numeric, subtract(endpoint, start), edge)?;
                if boundary(&mut fraction, candidate, num, den) {
                    region /= 2;
                    closest = endpoint;
                    transitioned = true;
                }
            }
            if !transitioned {
                #[expect(
                    clippy::neg_multiply,
                    reason = "source fmul uses negative one, preserving the numeric operation boundary"
                )]
                let reverse = den * (-1.0);
                if gt(reverse, SMALL) {
                    let num = dot(numeric, subtract(start, edge_origin), edge)?;
                    if boundary(&mut fraction, candidate, num, reverse) {
                        region = (region - 3) / 2;
                        transitioned = true;
                    }
                }
                if !transitioned && !candidate {
                    return Ok(false);
                }
            }
            if region > 2 {
                let t = fraction.value();
                let squared = dot(numeric, edge, edge)?;
                let seed = numeric.reciprocal_estimate(radius)?;
                let old = out.line_parameter.expect("entry parameter");
                let position = advance(start, delta, t);
                let inverse_edge = 1.0 / squared;
                out.line_parameter = Some((1.0 - old).mul_add(t, old));
                let edge_fraction =
                    dot(numeric, subtract(position, edge_origin), edge)? * inverse_edge;
                let raw = subtract(position, advance(edge_origin, edge, edge_fraction));
                out.normal = Some(raw);
                out.normal = Some(scale(raw, refine_drive_reciprocal(radius, seed)));
                out.position = Some(position);
                out.volume_parameter = Some(match region {
                    3 => [edge_fraction, 0.0, 0.0, 0.0],
                    4 => [0.0, edge_fraction, 0.0, 0.0],
                    _ => [1.0 - edge_fraction, edge_fraction, 0.0, 0.0],
                });
                return Ok(true);
            }
        }
        let t = fraction.value();
        let old = out.line_parameter.expect("entry parameter");
        out.line_parameter = Some((1.0 - old).mul_add(t, old));
        start = advance(start, delta, t);
        delta = scale(delta, 1.0 - t);
        if gt(t, 1.0) {
            return Ok(false);
        }
        if remaining == 0 {
            return Ok(true);
        }
    }
    unreachable!("five source transitions end in return")
}
