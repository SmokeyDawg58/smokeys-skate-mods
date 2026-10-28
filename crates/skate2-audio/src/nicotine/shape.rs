//! `NFSMixShape` (0x829ca4d0-0x829cae08) and the millibel/Q15 conversions the mixer inlines at every
//! use: `Q15todB` is a `cntlzw` switch over 15 bands of `g_Q15todBXForm` (anything outside
//! 1..=0x7FFF gives -10000), `dBToQ15` a `divw` switch over 16 octaves of `g_dBToQ15XForm`.

use super::tables::{COS_TABLE, DB_TO_Q15, PITCH_CENT, PITCH_SEMITONE, Q15_TO_DB};

/// `fctiwz`: truncate toward zero, saturating; NaN converts to 0x80000000.
pub(crate) fn fctiwz(value: f32) -> i32 {
    if value.is_nan() { i32::MIN } else { value as i32 }
}

/// The inlined Q15 to millibel conversion (default -10000).
pub fn q15_to_db(q15: i32) -> i32 {
    let value = q15 as u32;
    let zeros = value.leading_zeros();
    let index = match zeros {
        17..=22 => (value - (1 << (31 - zeros))) >> (22 - zeros),
        23..=31 => {
            let shift = zeros - 22;
            (value << shift) + (1 << shift) - 1 - 512
        }
        _ => return -10000,
    };
    Q15_TO_DB[index as usize] - 602 * (zeros as i32 - 16)
}

/// The inlined millibel to Q15 conversion (`g_dBToQ15XForm[601 + dB % 602] >> (dB / -602)`, 0 past
/// 15 octaves). Callers only pass values at or below 0.
pub fn db_to_q15(db: i32) -> i32 {
    let octaves = db / -602;
    if !(0..=15).contains(&octaves) {
        return 0;
    }
    DB_TO_Q15.get((601 + db % 602) as usize).map_or(0, |value| value >> octaves)
}

fn cos_at(index: i32) -> i32 {
    usize::try_from(index).ok().and_then(|index| COS_TABLE.get(index)).copied().unwrap_or(0)
}

fn fraction(q15: i32) -> i32 {
    (q15.wrapping_shl(9) & 0x3C00) | 0x3FF
}

/// `GetCurveOutput` 0x829cab60. Odd tables mirror the even one below them.
pub fn curve_output(table: i32, q15: i32) -> i32 {
    let (mut table, mut q15) = (table, q15);
    loop {
        match table {
            0 => {
                let index = q15 >> 6;
                if !(0..511).contains(&index) {
                    return 0;
                }
                let low = cos_at(index);
                if low == 0 {
                    return 0;
                }
                return low + ((cos_at(index + 1) - low).wrapping_mul(fraction(q15)) >> 15);
            }
            2 => {
                let value = curve_output(0, q15);
                return value.wrapping_mul(value) >> 15;
            }
            4 => {
                let index = 511 - (q15 >> 6);
                let low = 0x7FFF - cos_at(index);
                let high = 0x7FFF - cos_at(index + 1);
                return ((high - low).wrapping_mul(fraction(q15)) >> 15) + low;
            }
            6 => {
                let value = curve_output(4, q15);
                return value.wrapping_mul(value) >> 15;
            }
            8 => return 0x7FFFi32.wrapping_sub(q15),
            1 | 3 | 5 | 7 | 9 => {
                table -= 1;
                q15 = 0x7FFFi32.wrapping_sub(q15);
            }
            _ => return 0,
        }
    }
}

/// `GetAzimShapeOutput` 0x829cad00: both distances go through `etable1`; the second table id is
/// passed but never read.
pub fn azim_shape_output(table: i32, distances: [i32; 2], mix_ratio: i32) -> i32 {
    let near = curve_output(table, distances[0]);
    let far = if mix_ratio == 0 { 0x7FFF } else { curve_output(table, distances[1]) };
    let twice = mix_ratio.wrapping_shl(1);
    (0x7FFFi32.wrapping_sub(twice).wrapping_mul(near) >> 15) + (twice.wrapping_mul(far) >> 15)
}

/// `GetFloatCurveOutput` 0x829cadb0.
pub fn float_curve_output(table: i32, t: f32) -> f32 {
    curve_output(table, fctiwz(t * 32767.0)) as f32 * f32::from_bits(0x3800_0100)
}

/// `GetPitchMultFromCents` 0x829caa30.
pub fn pitch_mult_from_cents(cents: i32) -> f32 {
    let negative = cents < 0;
    let mut cents = cents;
    let mut octaves = 1.0f32;
    if cents >= 1200 {
        let count = cents as u32 / 1200;
        cents = cents.wrapping_sub(count.wrapping_mul(1200) as i32);
        for _ in 0..count {
            octaves *= 2.0;
        }
    }
    if cents <= -1200 {
        let count = (-1200i32).wrapping_sub(cents) as u32 / 1200 + 1;
        cents = cents.wrapping_add(count.wrapping_mul(1200) as i32);
        for _ in 0..count {
            octaves *= 2.0;
        }
    }
    if negative {
        let cents = cents.wrapping_neg();
        let semitone = 1.0 / PITCH_SEMITONE[(cents / 100) as usize];
        semitone / octaves * (1.0 / PITCH_CENT[(cents % 100) as usize])
    } else {
        PITCH_CENT[(cents % 100) as usize] * PITCH_SEMITONE[(cents / 100) as usize] * octaves
    }
}

/// `GetIntPitchMultFromCents` 0x829cad68: 4.12 fixed point of a 16-bit cents value.
pub fn int_pitch_mult_from_cents(cents: u16) -> i32 {
    fctiwz(pitch_mult_from_cents(i32::from(cents as i16)) * 4096.0)
}

/// `GetCentsFromPitchMult` 0x829ca4d0.
pub fn cents_from_pitch_mult(ratio: f32) -> i32 {
    if ratio > 1.0 {
        let db = q15_to_db(fctiwz(32767.0 / ratio));
        fctiwz(db as f32 * f32::from_bits(0xbfff_1fc4))
    } else {
        let db = q15_to_db(fctiwz(ratio * 32767.0));
        fctiwz(db as f32 * f32::from_bits(0x3fff_1fc4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q15_db_round_trip_ends() {
        assert_eq!(q15_to_db(0), -10000);
        assert_eq!(q15_to_db(0x8000), -10000);
        assert_eq!(q15_to_db(0x7FFF), Q15_TO_DB[511] - 602);
        assert_eq!(q15_to_db(1), Q15_TO_DB[511] - 9030);
        assert_eq!(db_to_q15(0), DB_TO_Q15[601]);
        assert_eq!(db_to_q15(-602), DB_TO_Q15[601] >> 1);
        assert_eq!(db_to_q15(-10000), 0);
    }

    #[test]
    fn mirrored_curves() {
        for q in [0, 1, 100, 0x3FFF, 0x4000, 0x7FFE, 0x7FFF] {
            assert_eq!(curve_output(9, q), q);
            assert_eq!(curve_output(1, q), curve_output(0, 0x7FFF - q));
            assert_eq!(curve_output(5, q), curve_output(4, 0x7FFF - q));
        }
        assert_eq!(curve_output(0, 0x7FFF), 0);
        assert_eq!(curve_output(10, 5), 0);
    }

    #[test]
    fn pitch_octaves() {
        assert_eq!(pitch_mult_from_cents(0), 1.0);
        assert_eq!(pitch_mult_from_cents(1200), 2.0);
        assert_eq!(pitch_mult_from_cents(-1200), 0.5);
        assert_eq!(int_pitch_mult_from_cents(0), 4096);
    }
}
