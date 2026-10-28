//! The inverse transforms `VP6_DecodeBlock` (`0x82f16be0`) runs inline in
//! VMX128 code. Products are single-precision floats truncated to integers
//! (`vmulfp128`, `vcfpsxws128`) and shifted right by 16 (`vsraw128`), sums
//! saturate (`vaddsws`/`vsubsws`), and the residual is packed with signed
//! saturation (`vpkswss128`). The C reference transforms in the executable
//! (`IDctSlow` `0x82f48c68`, `IDct10` `0x82f49af8`) are not what the decoder
//! calls, and they differ from it in rounding and saturation.

use super::tables::ZIGZAG;

const C1S7: i32 = 64277;
const C2S6: i32 = 60547;
const C3S5: i32 = 54491;
const C4S4: i32 = 46341;
const C5S3: i32 = 36410;
const C6S2: i32 = 25080;
const C7S1: i32 = 12785;

/// One cosine product: `vcsxwfp128`, `vmulfp128`, `vcfpsxws128`, then
/// `vsraw128` by 16.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the VMX float product"
)]
fn mul(constant: i32, x: i32) -> i32 {
    (((constant as f32) * (x as f32)) as i32) >> 16
}

/// The one-dimensional transform of both passes. The second pass adds the
/// rounding 8 to `Gd`, `Ed`, `Add` and `Fd` before the final sums.
fn transform(x: [i32; 8], round: bool) -> [i32; 8] {
    let a = mul(C1S7, x[1]).saturating_add(mul(C7S1, x[7]));
    let b = mul(C7S1, x[1]).saturating_sub(mul(C1S7, x[7]));
    let c = mul(C3S5, x[3]).saturating_add(mul(C5S3, x[5]));
    let d = mul(C3S5, x[5]).saturating_sub(mul(C5S3, x[3]));
    let ad = mul(C4S4, a.saturating_sub(c));
    let bd = mul(C4S4, b.saturating_sub(d));
    let cd = a.saturating_add(c);
    let dd = b.saturating_add(d);
    let e = mul(C4S4, x[0].saturating_add(x[4]));
    let f = mul(C4S4, x[0].saturating_sub(x[4]));
    let g = mul(C2S6, x[2]).saturating_add(mul(C6S2, x[6]));
    let h = mul(C6S2, x[2]).saturating_sub(mul(C2S6, x[6]));
    let mut ed = e.saturating_sub(g);
    let mut gd = e.saturating_add(g);
    let mut add = f.saturating_add(ad);
    let bdd = bd.saturating_sub(h);
    let mut fd = f.saturating_sub(ad);
    let hd = bd.saturating_add(h);
    if round {
        ed = ed.saturating_add(8);
        gd = gd.saturating_add(8);
        add = add.saturating_add(8);
        fd = fd.saturating_add(8);
    }
    [
        gd.saturating_add(cd),
        add.saturating_add(hd),
        add.saturating_sub(hd),
        ed.saturating_add(dd),
        ed.saturating_sub(dd),
        fd.saturating_add(bdd),
        fd.saturating_sub(bdd),
        gd.saturating_sub(cd),
    ]
}

/// Rows first, then columns with `(x + 8) >> 4`, as the native passes run.
fn transform_2d(input: &[i32; 64], rows: usize) -> [i16; 64] {
    let mut pass = [0i32; 64];
    for row in 0..rows {
        let mut x = [0i32; 8];
        x.copy_from_slice(&input[row * 8..row * 8 + 8]);
        pass[row * 8..row * 8 + 8].copy_from_slice(&transform(x, false));
    }
    let mut out = [0i16; 64];
    for column in 0..8 {
        let x = std::array::from_fn(|row| pass[row * 8 + column]);
        for (row, value) in transform(x, true).into_iter().enumerate() {
            out[row * 8 + column] = saturate16(value >> 4);
        }
    }
    out
}

#[expect(clippy::cast_possible_truncation, reason = "clamped to the i16 range")]
fn saturate16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// The dequantized coefficient: the coefficient times its quantizer, which
/// the native multiplies as exact floats.
fn dequantize(coefficient: i16, quantizer: i16) -> i32 {
    i32::from(coefficient) * i32::from(quantizer)
}

/// The DC-only path (selector at most 1): every residual is
/// `(dc * q + 15) >> 5`, as `IDct1` (`0x82f4a148`) computes it.
pub(super) fn dc_only(coefficients: &[i16; 64], quantizers: &[i16; 64]) -> [i16; 64] {
    let value = dequantize(coefficients[0], quantizers[0]).saturating_add(15) >> 5;
    [saturate16(value); 64]
}

/// The ten-coefficient path (selector 2 to 10). Its input permutes
/// (`vPermCoeff_row0_block0_1` .. `vPermCoeff_row3_block0_1`) load the
/// first four rows and columns from coefficient positions 0 to 11; raster
/// positions (2,2) and (1,3) read zero, and (3,2), (2,3) and (3,3) read the
/// top byte of dequantized position 4, repeated into a word.
pub(super) fn ten(coefficients: &[i16; 64], quantizers: &[i16; 64]) -> [i16; 64] {
    let value = |position: usize| dequantize(coefficients[position], quantizers[position]);
    let top = value(4).to_be_bytes()[0];
    let garbage = i32::from_be_bytes([top; 4]);
    let mut input = [0i32; 64];
    for (position, raster) in [
        (0, 0),
        (2, 8),
        (3, 16),
        (9, 24),
        (1, 1),
        (4, 9),
        (8, 17),
        (11, 25),
        (5, 2),
        (7, 10),
        (6, 3),
    ] {
        input[raster] = value(position);
    }
    input[26] = garbage;
    input[19] = garbage;
    input[27] = garbage;
    transform_2d(&input, 4)
}

/// The full transform (selector above 10).
pub(super) fn full(coefficients: &[i16; 64], quantizers: &[i16; 64]) -> [i16; 64] {
    let mut input = [0i32; 64];
    for (position, &raster) in ZIGZAG.iter().enumerate() {
        input[usize::from(raster)] = dequantize(coefficients[position], quantizers[position]);
    }
    transform_2d(&input, 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quantizers(dc: i16, ac: i16) -> [i16; 64] {
        let mut q = [ac; 64];
        q[0] = dc;
        q
    }

    fn coefficients(values: &[(usize, i16)]) -> [i16; 64] {
        let mut c = [0i16; 64];
        for &(position, value) in values {
            c[position] = value;
        }
        c
    }

    /// Outputs of the native VMX code, run instruction by instruction
    /// (0x82f17c30, 0x82f17ce4 and 0x82f18390).
    #[test]
    fn dc_only_matches_the_native_block() {
        let out = dc_only(&coefficients(&[(0, 37)]), &quantizers(188, 376));
        assert!(out.iter().all(|&v| v == 217));
    }

    #[test]
    fn ten_coefficient_path_reads_the_sign_of_position_four() {
        let q = quantizers(188, 120);
        let out = ten(&coefficients(&[(4, -5)]), &q);
        assert_eq!(&out[24..32], &[-7, -6, -4, -2, 2, 4, 6, 7]);
        assert_eq!(&out[40..48], &[20, 17, 12, 4, -4, -11, -17, -20]);
        let positive = ten(&coefficients(&[(4, 5)]), &q);
        let full_out = full(&coefficients(&[(4, 5)]), &q);
        assert_eq!(positive, full_out);
    }

    #[test]
    fn ten_coefficient_path_matches_the_native_block() {
        let q = quantizers(140, 192);
        let out = ten(
            &coefficients(&[(0, 2), (2, -164), (4, 26), (7, -61), (9, -45)]),
            &q,
        );
        assert_eq!(
            &out[0..8],
            &[-2037, -1694, -1229, -951, -1070, -1569, -2203, -2357]
        );
        assert_eq!(
            &out[56..64],
            &[2054, 1712, 1246, 968, 1088, 1586, 2220, 2374]
        );
    }

    #[test]
    fn full_path_matches_the_native_block() {
        let q = quantizers(168, 248);
        let out = full(
            &coefficients(&[(0, 9), (1, -4), (5, 3), (14, 2), (27, -1), (63, 1)]),
            &q,
        );
        assert_eq!(out, NATIVE_FULL);
    }

    /// The native full transform (0x82f18390) of the coefficients in
    /// `full_path_matches_the_native_block`.
    const NATIVE_FULL: [i16; 64] = [
        46, 16, -13, 25, 48, 31, 93, 131, 44, 23, -23, 36, 37, 40, 86, 133, 48, 11, -5, 15, 58, 23,
        98, 129, 43, 26, -28, 43, 30, 46, 82, 135, 49, 9, -3, 13, 60, 21, 99, 129, 43, 25, -26, 41,
        32, 44, 84, 134, 48, 13, -8, 20, 54, 26, 96, 130, 45, 20, -18, 31, 42, 36, 89, 133,
    ];
}
