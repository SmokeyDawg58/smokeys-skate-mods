//! The VMX128 operation sequences the audio game code compiles its vector math to.
//!
//! `vrsqrtefp` and `vrefp` seeds are the exact reciprocal (square root), the same convention as
//! the physics port (`refine_drive_reciprocal_sqrt`); the Newton steps after them, which decide
//! the result, are the native fused sequences. `vmsum3fp` is `(x + y) + z` of rounded products.

/// `rw::math::vpu::Vector3` (the `w` lane is carried, never read).
pub type Vec3 = [f32; 4];

pub const ZERO: Vec3 = [0.0; 4];

#[must_use]
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

#[must_use]
pub fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s, a[3] * s]
}

/// `vmsum3fp128`.
#[must_use]
pub fn dot3(a: Vec3, b: Vec3) -> f32 {
    let x = a[0] * b[0];
    let y = a[1] * b[1];
    let z = a[2] * b[2];
    (x + y) + z
}

/// Two `vrsqrtefp` Newton steps: `y += (y / 2) * (1 - x * y * y)`, each fused.
#[must_use]
pub fn rsqrt(squared: f32) -> f32 {
    let mut y = 1.0 / squared.sqrt();
    for _ in 0..2 {
        let residual = (-squared).mul_add(y * y, 1.0);
        y = (y * 0.5).mul_add(residual, y);
    }
    y
}

/// Two `vrefp` Newton steps: `y += y * (1 - y * d)`, each fused.
#[must_use]
pub fn recip(d: f32) -> f32 {
    let mut y = 1.0 / d;
    for _ in 0..2 {
        let residual = (-y).mul_add(d, 1.0);
        y = y.mul_add(residual, y);
    }
    y
}

/// `x * rsqrt(x)` with the `vcmpeqfp`/`vsel` zero guard: the length for a squared length.
#[must_use]
pub fn length_from_squared(squared: f32) -> f32 {
    let length = squared * rsqrt(squared);
    if squared == 0.0 { 0.0 } else { length }
}

/// `Vector3::Magnitude` (guarded).
#[must_use]
pub fn length(a: Vec3) -> f32 {
    length_from_squared(dot3(a, a))
}

/// Unguarded normalisation (`v * rsqrt(|v|^2)`): a zero vector becomes NaN, as natively.
#[must_use]
pub fn normalize_unguarded(a: Vec3) -> Vec3 {
    scale(a, rsqrt(dot3(a, a)))
}

/// `vmaxfp`: NaN when either input is NaN.
#[must_use]
pub fn vmax(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        return f32::NAN;
    }
    if a > b || (a == b && a.is_sign_positive()) { a } else { b }
}

/// `vminfp`: NaN when either input is NaN.
#[must_use]
pub fn vmin(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        return f32::NAN;
    }
    if a < b || (a == b && a.is_sign_negative()) { a } else { b }
}

/// `XMVectorACos` 0x824b0278 (XNA Math, Cody and Waite) on one lane.
#[must_use]
pub fn xm_acos(v: f32) -> f32 {
    const ONE_PLUS_EPSILON: f32 = f32::from_bits(0x3f80_0001);
    const C0: [u32; 4] = [0xbd6d_d42d, 0xbed6_5553, 0x3e66_3246, 0x400b_1889];
    const C1: [u32; 4] = [0x3f1d_d7b6, 0x4089_80bd, 0xbf98_3f2f, 0xc0d1_360e];
    const C2: [u32; 4] = [0xbfaf_4418, 0xc08f_6ad9, 0x3fb5_8485, 0x40af_6ad8];
    const PI: f32 = f32::from_bits(0x4049_0fdb);
    let c0 = C0.map(f32::from_bits);
    let c1 = C1.map(f32::from_bits);
    let c2 = C2.map(f32::from_bits);
    let abs = f32::from_bits(v.to_bits() & 0x7FFF_FFFF);
    let v2 = v * v;
    let one_minus = ONE_PLUS_EPSILON - abs;
    let abs3 = v2 * abs;
    let rest = (-abs).mul_add(v, v);
    let p = [0, 1, 2, 3].map(|i| c0[i].mul_add(abs, c1[i]).mul_add(abs, c2[i]));
    let y0 = 1.0 / one_minus.sqrt();
    let half_x = one_minus * 0.5;
    let half_pi = PI * 0.5;
    let residual = (-half_x).mul_add(y0 * y0, 0.5);
    let upper = p[2].mul_add(abs3, p[3]);
    let lower = p[0].mul_add(abs3, p[1]);
    let y1 = y0.mul_add(residual, y0);
    let a = v * upper;
    let b = rest * lower;
    half_pi - b.mul_add(y1, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acos_matches_libm_closely() {
        for i in -100..=100 {
            let x = i as f32 / 100.0;
            assert!((xm_acos(x) - x.acos()).abs() < 2e-4, "{x}: {} vs {}", xm_acos(x), x.acos());
        }
    }

    #[test]
    fn guarded_length() {
        assert_eq!(length(ZERO), 0.0);
        assert!((length([3.0, 4.0, 0.0, 0.0]) - 5.0).abs() < 1e-6);
    }
}
