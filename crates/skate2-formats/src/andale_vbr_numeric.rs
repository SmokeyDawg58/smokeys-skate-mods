//! Native VBR table and scalar reconstruction graphs.
//!
//! Horizontal reductions use explicit separately rounded host products and
//! left-associated sums. Xenon reduction rounding, denormals, NaN payloads and
//! estimate instructions still require native traces; this is not bit parity.
use std::sync::OnceLock;

// XMVectorCos824b0130, g_XMCosCoefficients0/1/2 at823cfe40/50/60.
const COSINE_COEFFICIENTS: [u32; 12] = [
    0x3f80_0000,
    0xbf00_0000,
    0x3d2a_aaab,
    0xbab6_0b61,
    0x37d0_0d01,
    0xb493_f27e,
    0x310f_76c8,
    0xad49_cba5,
    0x2957_3f9f,
    0xa534_13c3,
    0x20f2_a15d,
    0x9c86_71cb,
];

/// Scalar lane of `XMVectorCos824b0130`, used only for finite IDCT angles.
fn cosine(angle: f32) -> f32 {
    // g_XMPiConstants0@823cfea0: lane1 is 2pi, lane3 is its reciprocal.
    let turns = (angle * f32::from_bits(0x3e22_f983)).round_ties_even();
    // Native VNMSUB negates the rounded fused subtraction, including zero.
    let reduced = -f32::from_bits(0x40c9_0fdb).mul_add(turns, -angle);
    let square = reduced * reduced;
    let fourth = square * square;
    let sixth = fourth * square;
    let eighth = fourth * fourth;
    let tenth = sixth * fourth;
    let twelfth = sixth * sixth;
    let fourteenth = eighth * sixth;
    let sixteenth = eighth * eighth;
    let eighteenth = tenth * eighth;
    let twentieth = tenth * tenth;
    let twenty_second = twelfth * tenth;
    let powers = [
        square,
        fourth,
        sixth,
        eighth,
        tenth,
        twelfth,
        fourteenth,
        sixteenth,
        eighteenth,
        twentieth,
        twenty_second,
    ];
    let mut result = 1.0;
    for (coefficient, power) in COSINE_COEFFICIENTS[1..].iter().zip(powers) {
        result = f32::from_bits(*coefficient).mul_add(power, result);
    }
    result
}

/// `InitIDctTablesInv82edb3d0`: immutable table shared by all scalar callers.
/// The native table is per decompressor; its inputs are fixed build constants.
pub(super) fn inverse_dct_table() -> &'static [[f32; 8]; 8] {
    static TABLE: OnceLock<[[f32; 8]; 8]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let frequencies = [
            0x0000_0000,
            0x3ec9_0fdb,
            0x3f49_0fdb,
            0x3f96_cbe4,
            0x3fc9_0fdb,
            0x3ffb_53d2,
            0x4016_cbe4,
            0x402f_ede0,
        ]
        .map(f32::from_bits);
        let positions = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5];
        positions.map(|position| {
            let mut row = frequencies.map(|frequency| cosine(position * frequency) * 0.25);
            row[0] *= 0.5;
            row
        })
    })
}

/// `XMVectorLog` graph in `VBRDecompressor::Init82edc650..6a0`, exclusively
/// integer frequencies 2..9. The binary exponent supplies floor(VLOGEFP) for
/// these positive normals; hardware estimate edges remain a trace boundary.
fn quantization_log(frequency: f32) -> f32 {
    // C0@820f0fb0, C1@820f0fa0. Preserve the source parallel cubic grouping.
    let first = [0x3fb8_aa0e, 0xbf38_9e52, 0x3ef5_162d, 0xbeb1_d204].map(f32::from_bits);
    let second = [0x3e77_adbd, 0xbe0c_d4fb, 0x3d55_41c6, 0xbc18_8b0b].map(f32::from_bits);
    let exponent = match frequency.to_bits() >> 23 {
        128 => 1.0,
        129 => 2.0,
        130 => 3.0,
        _ => unreachable!("codec quantization frequency is in 2..=9"),
    };
    let mantissa = f32::from_bits((frequency.to_bits() & 0x007f_ffff) | 0x3f80_0000);
    let delta = mantissa - 1.0;
    let squared = delta * delta;
    let cubed = delta * squared;
    let fourth = squared * squared;
    let first_linear = delta.mul_add(first[1], first[0]);
    let second_linear = delta.mul_add(second[1], second[0]);
    let first_quadratic = squared.mul_add(first[2], first_linear);
    let second_quadratic = squared.mul_add(second[2], second_linear);
    let first_cubic = cubed.mul_add(first[3], first_quadratic);
    let second_cubic = cubed.mul_add(second[3], second_quadratic);
    let combined = fourth.mul_add(second_cubic, first_cubic);
    let logarithm_two = delta.mul_add(combined, exponent);
    // gLn2@8227e950 followed by separate 1/32768 multiplication.
    logarithm_two * f32::from_bits(0x3f31_7218)
}

pub(super) fn initial_quantization_row(table_index: u8) -> [f32; 8] {
    // VBRDecompressor::Init82edc604..730 creates all 256 rows once. The
    // coefficients are immutable build constants, shared across decompressors.
    static TABLE: OnceLock<[[f32; 8]; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let initial = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
            .map(|frequency| quantization_log(frequency) * f32::from_bits(0x3800_0000));
        let mut scale = 1.0_f32;
        std::array::from_fn(|_| {
            let row = initial.map(|value| value * scale);
            // 82edc728 recurrent FADDS; multiplication by the index differs.
            scale += f32::from_bits(0x3e4c_cccd);
            row
        })
    })[usize::from(table_index)]
}

/// Explicit host policy for each native VMSUM4FP, not Xenon reduction parity.
fn host_dot4(left: &[f32], right: &[f32]) -> f32 {
    let products: [f32; 4] = std::array::from_fn(|lane| left[lane] * right[lane]);
    ((products[0] + products[1]) + products[2]) + products[3]
}

/// FrameBlock82edbb24..70 reduces halves independently before their sum.
pub(super) fn reconstruct_sample(coefficients: &[f32; 8], weights: &[f32; 8]) -> f32 {
    let low = host_dot4(&coefficients[..4], &weights[..4]);
    let high = host_dot4(&coefficients[4..], &weights[4..]);
    high + low
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_table_polynomials_preserve_nontrivial_row_bits() {
        // Supplemental Python/UCRT-fmaf evaluation of captured source graph;
        // these are host regression oracles, not Xbox runtime traces.
        assert_eq!(
            inverse_dct_table()[7].map(f32::to_bits),
            [
                0x3e00_0000,
                0xbe7b_14bb,
                0x3e6c_835e,
                0xbe54_db30,
                0x3e35_04ef,
                0xbe0e_39de,
                0x3dc3_ef16,
                0xbd47_c5f0,
            ]
        );
        assert_eq!(
            [2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
                .map(|frequency| quantization_log(frequency).to_bits()),
            [
                0x3f31_7218,
                0x3f8c_9f54,
                0x3fb1_7218,
                0x3fce_020f,
                0x3fe5_585f,
                0x3ff9_1396,
                0x4005_1592,
                0x400c_9f54,
            ]
        );
        assert_eq!(
            super::super::vbr_quantization_row(17, -1.0, 0.5).map(f32::to_bits),
            [
                0x3901_985a,
                0x393a_af43,
                0x3963_30b4,
                0x3981_4def,
                0x398e_23ce,
                0x3998_fdf9,
                0x39a2_6487,
                0x39aa_af43,
            ]
        );
    }

    #[test]
    fn quantization_prescales_weights_before_both_coefficient_reductions() {
        let coefficients = [32767, -16384, 73, 12001, -32000, 919, -11111, 22222];
        let result =
            super::super::vbr_reconstruct_scalar_samples(coefficients, 17, -1.0, 0.5, -2.0, 3.0);
        assert_eq!(
            result.map(f32::to_bits),
            [
                0xc09a_1092,
                0x40b2_659c,
                0x410a_5b82,
                0xc165_dd32,
                0x4105_92ce,
                0x4099_3031,
                0x41be_4cef,
                0xc0f6_e67b,
            ]
        );
        // Dequantizing first would instead give sample1=40b2659a and
        // sample3=c165dd33, even with the same polynomial and dot4 policy.
    }

    #[test]
    fn reduction_keeps_two_halves_and_documented_host_rounding() {
        let coefficients = [1.0e20, 1.0, 1.0, 1.0, -1.0e20, 1.0, 1.0, 1.0];
        assert_eq!(reconstruct_sample(&coefficients, &[1.0; 8]).to_bits(), 0);
        let serial = coefficients
            .into_iter()
            .fold(0.0_f32, |sum, value| sum + value);
        assert_eq!(serial.to_bits(), 3.0_f32.to_bits());
    }
}
