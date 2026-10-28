//! Floating-point helpers with the Xbox 360's rounding.

/// `fmadds` / `vmaddfp`: `a * b + c` rounded once to f32. The f32 product is exact in f64; the
/// f64 sum is rounded to odd (its last bit set when inexact), which keeps the final rounding to f32
/// correct, so the result is the fused one.
#[inline]
pub fn fmadd(a: f32, b: f32, c: f32) -> f32 {
    let product = f64::from(a) * f64::from(b);
    let addend = f64::from(c);
    let sum = product + addend;
    let back = sum - product;
    let error = (product - (sum - back)) + (addend - back);
    if error == 0.0 || sum.to_bits() & 1 == 1 || !sum.is_finite() {
        return sum as f32;
    }
    let toward_larger_magnitude = (error > 0.0) == (sum > 0.0);
    let bits = if toward_larger_magnitude { sum.to_bits() + 1 } else { sum.to_bits() - 1 };
    f64::from_bits(bits) as f32
}

/// `fnmsubs` / `vnmsubfp`: `-(a * b - c)` rounded once.
#[inline]
pub fn fnmsub(a: f32, b: f32, c: f32) -> f32 {
    fmadd(-a, b, c)
}

/// The compiler's float-to-int round used throughout RWAC: `(int)(x + 0.5)` for x >= 0,
/// `(int)(x - 0.5)` otherwise, computed in f32 and truncated.
#[inline]
pub fn round_half_away(x: f32) -> i32 {
    if x < 0.0 { (x - 0.5) as i32 } else { (x + 0.5) as i32 }
}

/// The MSVC CRT `rand()` the Xbox 360 libraries call: `seed = seed * 214013 + 2531011`,
/// returning bits 16..30. The game shares one generator across every caller, so the sequence a
/// given sound sees depends on everything else that ran before it.
#[derive(Debug, Clone)]
pub struct CrtRand {
    seed: u32,
}

impl Default for CrtRand {
    fn default() -> Self {
        Self { seed: 1 }
    }
}

impl CrtRand {
    pub fn rand(&mut self) -> i32 {
        self.seed = self.seed.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((self.seed >> 16) & 0x7FFF) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmadd_is_single_rounded() {
        let mut state = 0x1234_5678_9abc_def0u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..2_000_000 {
            let bits = next();
            let a = f32::from_bits((bits as u32 & 0x807F_FFFF) | 0x3F00_0000);
            let b = f32::from_bits(((bits >> 32) as u32 & 0x807F_FFFF) | 0x3E80_0000);
            let c = f32::from_bits((next() as u32 & 0x807F_FFFF) | 0x3E00_0000);
            assert_eq!(fmadd(a, b, c).to_bits(), a.mul_add(b, c).to_bits(), "{a} {b} {c}");
            assert_eq!(fnmsub(a, b, c).to_bits(), (-a).mul_add(b, c).to_bits(), "{a} {b} {c}");
        }
        let halfway = [(1.0 + f32::EPSILON, 1.0 - f32::EPSILON / 2.0, -1.0), (3.0, 1.0 / 3.0, -1.0)];
        for (a, b, c) in halfway {
            assert_eq!(fmadd(a, b, c).to_bits(), a.mul_add(b, c).to_bits());
        }
    }

    #[test]
    fn crt_rand_sequence() {
        let mut rand = CrtRand::default();
        let first: Vec<i32> = (0..5).map(|_| rand.rand()).collect();
        assert_eq!(first, [41, 18467, 6334, 26500, 19169]);
    }
}
