//! Reshape82c954e8. Host estimate/rounding boundaries reuse existing math services.
use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric, length_from_squared_and_estimate},
    reckoning_sin_cos::{HostReckoningSinCosNumeric, sine::try_sine, try_cosine},
    skeleton_query_rotation::try_atan,
};
pub(crate) trait ReshapeSource {
    type Error;
    fn graph(&mut self, offset: usize, input: f32) -> Result<f32, Self::Error>;
    fn scalar(&mut self, key: u64) -> Result<f32, Self::Error>;
    fn reshape_cvar(&mut self, index: usize) -> Result<bool, Self::Error>;
}
pub(crate) fn select(test: f32, yes: f32, no: f32) -> f32 {
    if test >= -0.0 { yes } else { no }
}
fn clamp(value: f32) -> f32 {
    let low = select(-1.0 - value, -1.0, value);
    select(1.0 - low, low, 1.0)
}
const PI: f32 = f32::from_bits(0x4049_0fdb);
/// VMX non-Java operand semantics, including the vcmpeqfp128 zero-axis
/// selects at82c95614/82c95898. Preserve the sign used by the quadrant mask.
/// A host subnormal must not enter reciprocal refinement as a nonzero axis:
/// its reciprocal can overflow, creating NaN and then saturated lean values.
/// This is the IEEE exponent boundary, not an input dead zone.
fn vmx_operand(value: f32) -> f32 {
    let bits = value.to_bits();
    if bits & 0x7f80_0000 == 0 {
        f32::from_bits(bits & 0x8000_0000)
    } else {
        value
    }
}
fn polar(input: [f32; 2]) -> (f32, f32) {
    let [x, y] = input.map(vmx_operand);
    let squared = x * x + y * y;
    let mut numeric = HostReckoningNumeric;
    let seed = numeric
        .reciprocal_sqrt_estimate(squared)
        .expect("host reciprocal square root has no integer conversion");
    let length = length_from_squared_and_estimate(squared, seed);
    // Raw55a0/5828 BO4 BI25 branch on !GT, including unordered.
    if length.partial_cmp(&f32::from_bits(0x3780_0000)) != Some(std::cmp::Ordering::Greater) {
        return (length, 0.0);
    }
    let seed = numeric
        .reciprocal_estimate(x)
        .expect("host reciprocal has no integer conversion");
    let residual = -seed.mul_add(x, -1.0);
    let inverse = seed.mul_add(residual, seed);
    let angle = try_atan(y.mul_add(inverse, 0.0), &mut numeric)
        .expect("host atan uses reciprocal arithmetic only");
    let sign = y.to_bits() & 0x8000_0000;
    let angle = if 0.0 > x {
        f32::from_bits(PI.to_bits() | sign) + angle
    } else {
        angle
    };
    let angle = if 0.0 == x {
        f32::from_bits(0x3fc9_0fdb | sign)
    } else {
        angle
    };
    (length, angle)
}
fn cartesian(length: f32, angle: f32) -> [f32; 2] {
    let mut numeric = HostReckoningSinCosNumeric;
    let x = try_cosine(angle, &mut numeric).unwrap_or_else(|e| match e {});
    let x = clamp(x * length);
    let y = try_sine(angle, &mut numeric).unwrap_or_else(|e| match e {});
    [x, clamp(y * length)]
}
pub(crate) fn reshape<S: ReshapeSource>(
    source: &mut S,
    input: [f32; 2],
    slow: bool,
) -> Result<[f32; 2], S::Error> {
    let (length, angle) = polar(input);
    let positive = select(angle, angle, angle + PI);
    let magnitude = source.graph(if slow { 0 } else { 0x120 }, positive)?;
    let magnitude = if source.reshape_cvar(234)? {
        magnitude
    } else {
        1.0
    };
    let clamped = cartesian(length * magnitude, angle);
    let (length, angle) = polar(clamped);
    let positive = select(angle, angle, angle + PI);
    let mapped = source.graph(if slow { 0x90 } else { 0x1b0 }, positive)?;
    let mapped = if source.reshape_cvar(235)? {
        mapped
    } else {
        positive
    };
    let mapped = select(angle, mapped, mapped + PI);
    let offset = source.scalar(if slow {
        0x1166_45a0_4f17_e2e7
    } else {
        0x1994_feea_33b8_6bdb
    })?;
    let mut mapped = offset + mapped;
    if !source.reshape_cvar(236)? {
        mapped -= offset;
    }
    Ok(cartesian(length, mapped))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct IdentityMaps;
    impl ReshapeSource for IdentityMaps {
        type Error = std::convert::Infallible;
        fn graph(&mut self, _: usize, input: f32) -> Result<f32, Self::Error> {
            Ok(input)
        }
        fn scalar(&mut self, _: u64) -> Result<f32, Self::Error> {
            Ok(0.)
        }
        fn reshape_cvar(&mut self, _: usize) -> Result<bool, Self::Error> {
            Ok(false)
        }
    }

    #[test]
    fn captured_decaying_turn_does_not_become_full_lean() {
        // lean-trace-19086-23931.csv ticks10823..10826. Previously these
        // subnormal X values saturated the reshaped vector for four ticks.
        for (bits, direction) in [
            (0x0004_1c32, 0.030407459),
            (0x0000_1b30, 0.033368602),
            (0x0000_009c, 0.035930257),
            (0x0000_0003, 0.037871692),
        ] {
            for sign in [0, 0x8000_0000] {
                for direction in [direction, -direction] {
                    for slow in [false, true] {
                        let actual = reshape(
                            &mut IdentityMaps,
                            [f32::from_bits(bits | sign), direction],
                            slow,
                        )
                        .unwrap();
                        let zero =
                            reshape(&mut IdentityMaps, [f32::from_bits(sign), direction], slow)
                                .unwrap();
                        assert_eq!(actual.map(f32::to_bits), zero.map(f32::to_bits));
                        assert!(actual[0].abs() < 0.001);
                        assert!((actual[1] - direction).abs() < 0.001);
                    }
                }
            }
        }
    }

    #[test]
    fn vmx_zero_axis_preserves_quadrant_and_normal_boundary() {
        for bits in [0, 1, 0x007f_ffff, 0x8000_0000, 0x8000_0001, 0x807f_ffff] {
            assert_eq!(
                vmx_operand(f32::from_bits(bits)).to_bits(),
                bits & 0x8000_0000
            );
            for (y, expected) in [(0.03, 0x3fc9_0fdb), (-0.03, 0xbfc9_0fdb)] {
                assert_eq!(polar([f32::from_bits(bits), y]).1.to_bits(), expected);
            }
        }
        for bits in [
            0x0080_0000,
            0x8080_0000,
            0x3f80_0000,
            0x7f80_0000,
            0x7fc0_1234,
        ] {
            assert_eq!(vmx_operand(f32::from_bits(bits)).to_bits(), bits);
        }
    }
}
