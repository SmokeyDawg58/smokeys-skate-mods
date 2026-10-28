//! The CRT `rand` 0x82e9fe58 the AI's profile and trick weights draw from: the per-thread
//! `_tiddata::_holdrand` (1 until `srand`), stepped as `holdrand * 214013 + 2531011` and
//! answered as bits 16..30.
use std::sync::Mutex;

static HOLDRAND: Mutex<u32> = Mutex::new(1);

/// `rand`.
pub(crate) fn rand() -> u32 {
    let mut holdrand = HOLDRAND.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    *holdrand = holdrand.wrapping_mul(214_013).wrapping_add(2_531_011);
    (*holdrand >> 16) & 0x7fff
}

/// `(float)rand() * (1 / 32767)`, the unit draw the AI's odds use.
pub(crate) fn unit() -> f32 {
    rand() as f32 * f32::from_bits(0x3800_0100)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_unit_scale_is_one_over_32767() {
        assert_eq!(f32::from_bits(0x3800_0100), 0.000030518509_f32);
    }
}
