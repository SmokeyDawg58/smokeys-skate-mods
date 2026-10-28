//! VMX operations the AI code composes, on the port's host numeric policy: dot products are
//! `host_dot3`/`host_dot4`, reciprocal square roots take a host seed and the two fused Newton
//! rounds of the native graph.
use crate::reckoning_numeric::{host_dot3, host_dot4, length_from_squared_and_estimate};
use crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt;

pub(crate) type V4 = [f32; 4];

pub(crate) const fn v3(v: [f32; 3]) -> V4 {
    [v[0], v[1], v[2], 0.0]
}

pub(crate) const fn xyz(v: V4) -> [f32; 3] {
    [v[0], v[1], v[2]]
}

pub(crate) fn add(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] + b[i])
}

pub(crate) fn sub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] - b[i])
}

pub(crate) fn mul(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i] * b[i])
}

pub(crate) fn scale(a: V4, s: f32) -> V4 {
    a.map(|v| v * s)
}

/// `vmaddfp`: `a * c + b`, one rounding.
pub(crate) fn madd(a: V4, c: V4, b: V4) -> V4 {
    std::array::from_fn(|i| a[i].mul_add(c[i], b[i]))
}

/// `vnmsubfp`: `b - a * c`, one rounding.
pub(crate) fn nmsub(a: V4, c: V4, b: V4) -> V4 {
    std::array::from_fn(|i| (-a[i]).mul_add(c[i], b[i]))
}

/// `vmsum3fp128`.
pub(crate) fn dot3(a: V4, b: V4) -> f32 {
    host_dot3(a, b)
}

/// `vmsum4fp128`.
pub(crate) fn dot4(a: V4, b: V4) -> f32 {
    host_dot4(a, b)
}

/// `vrsqrtefp` and two Newton rounds.
pub(crate) fn rsqrt(x: f32) -> f32 {
    refine_drive_reciprocal_sqrt(x, 1.0 / x.sqrt())
}

/// `vrefp` and two Newton rounds.
pub(crate) fn reciprocal(x: f32) -> f32 {
    crate::simulation_drive_jacobian::refine_drive_reciprocal(x, 1.0 / x)
}

/// `x * rsqrt(x)`, selected to zero for an exactly zero input.
pub(crate) fn sqrt_guarded(x: f32) -> f32 {
    length_from_squared_and_estimate(x, 1.0 / x.sqrt())
}

/// `Math::NormalizeSafe` 0x824b18e0: the default within `kBadMagnitude`.
pub(crate) fn normalize_safe(vector: V4, default: V4) -> V4 {
    const BAD_MAGNITUDE: f32 = f32::from_bits(0x3586_37bd);
    let squared = dot3(vector, vector);
    if !(squared - BAD_MAGNITUDE * BAD_MAGNITUDE > 0.0) {
        return default;
    }
    scale(vector, rsqrt(squared))
}

/// The guarded length of the XYZ lanes.
pub(crate) fn length3(v: V4) -> f32 {
    sqrt_guarded(dot3(v, v))
}

/// Scalar-splat `XMVectorSin` 0x824b0060.
pub(crate) fn sin(x: f32) -> f32 {
    let mut rounding = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
    crate::reckoning_sin_cos::sine::try_sine(x, &mut rounding).unwrap_or_else(|never| match never {})
}

/// Scalar-splat `XMVectorCos` 0x824b0130.
pub(crate) fn cos(x: f32) -> f32 {
    let mut rounding = crate::reckoning_sin_cos::HostReckoningSinCosNumeric;
    crate::reckoning_sin_cos::try_cosine(x, &mut rounding).unwrap_or_else(|never| match never {})
}

/// Scalar-splat `XMVectorACos` 0x824b0278; its host estimates cannot fail.
pub(crate) fn acos(x: f32) -> f32 {
    crate::reckoning_numeric::try_acos(x, &mut crate::reckoning_numeric::HostReckoningNumeric).unwrap_or(f32::NAN)
}

/// `vminfp`: the first NaN operand, quieted; -0 under +0.
pub(crate) fn vminfp(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        f32::from_bits(a.to_bits() | 0x0040_0000)
    } else if b.is_nan() {
        f32::from_bits(b.to_bits() | 0x0040_0000)
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// `vpermwi128`: lane `i` takes source lane `(imm >> (6 - 2i)) & 3`.
pub(crate) fn permwi(v: V4, imm: u8) -> V4 {
    std::array::from_fn(|i| v[usize::from((imm >> (6 - 2 * i)) & 3)])
}

/// `vspltw`.
pub(crate) fn splat(v: V4, lane: usize) -> V4 {
    [v[lane]; 4]
}

/// `rw::math::vpu` `Cross`: `a.yzx * b.zxy - a.zxy * b.yzx` as the `vpermwi 0x63`,
/// multiply and `vnmsubfp` sequence.
pub(crate) fn cross(a: V4, b: V4) -> V4 {
    let a_zxy = permwi(a, 0x87);
    let b_yzx = permwi(b, 0x63);
    let a_yzx = permwi(a, 0x63);
    let b_zxy = permwi(b, 0x87);
    nmsub(a_zxy, b_yzx, mul(a_yzx, b_zxy))
}

/// `rw::math::vpu::Quaternion` to a rotation basis through `gSqrt2s`, the expansion
/// `Node::GetBoardOrientationAsMatrix` 0x824b61c0 inlines: rows X, Y, Z. The rows come
/// from `vperm` with `VectorPermuteConstant<1,4,0,0>`, `<0,6,0,0>` and `<2,5,0,0>` and a
/// `vrlimi128` Z lane, so their W lanes keep a diagonal, difference and sum term.
pub(crate) fn basis_from_quaternion(q: V4) -> [V4; 3] {
    let s = scale(q, f32::from_bits(0x3fb5_04f3));
    let negative_squares = nmsub(s, s, [0.5; 4]);
    let products = mul(s, permwi(s, 0x60));
    let w_products = mul(splat(s, 3), permwi(s, 0x84));
    let sum = add(products, w_products);
    let difference = sub(products, w_products);
    let diagonal = add(negative_squares, permwi(negative_squares, 0x60));
    [
        [diagonal[1], sum[0], difference[2], diagonal[0]],
        [difference[0], diagonal[2], sum[1], difference[0]],
        [sum[2], difference[1], diagonal[0], sum[0]],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permwi_reads_lane_pairs_from_the_top() {
        let v = [0., 1., 2., 3.];
        assert_eq!(permwi(v, 0x60), [1., 2., 0., 0.]);
        assert_eq!(permwi(v, 0x84), [2., 0., 1., 0.]);
        assert_eq!(permwi(v, 0x63), [1., 2., 0., 3.]);
    }

    #[test]
    fn cross_of_the_axes() {
        assert_eq!(cross([1., 0., 0., 0.], [0., 1., 0., 0.])[..3], [0., 0., 1.]);
        assert_eq!(cross([0., 1., 0., 0.], [0., 0., 1., 0.])[..3], [1., 0., 0.]);
    }

    #[test]
    fn the_identity_quaternion_gives_the_identity_basis() {
        let basis = basis_from_quaternion([0., 0., 0., 1.]);
        let close = |a: V4, b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6);
        assert!(close(basis[0], [1., 0., 0.]));
        assert!(close(basis[1], [0., 1., 0.]));
        assert!(close(basis[2], [0., 0., 1.]));
    }

    #[test]
    fn guarded_square_root_of_zero_is_zero() {
        assert_eq!(sqrt_guarded(0.0).to_bits(), 0);
        assert!((sqrt_guarded(4.0) - 2.0).abs() < 1e-6);
    }
}
