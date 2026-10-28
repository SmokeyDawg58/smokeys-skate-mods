//! `QuatTransCompressQ3`, 0x827d5c58..0x827d61f8.
use super::{
    CompressionError, CompressionSource, CompressionVectors, admitted_count,
    check_translation_range, integer_low16, scaled_quaternion, select_hemisphere,
};
use crate::reckoning_numeric::ReckoningNumeric;

pub(crate) struct QuaternionTranslationDestination<'a> {
    pub(crate) quaternions: &'a mut [[u16; 3]],
    pub(crate) translations: &'a mut [[u16; 3]],
}

/// Uses the same borrowed source SQT records, positive-count admission and
/// mutable destinations as the standalone primitives. No intermediate output
/// buffer hides source partial writes on a later conversion failure.
pub(crate) fn try_compress_quaternion_translation<N: ReckoningNumeric>(
    source: CompressionSource<'_>,
    count: i32,
    destination: &mut QuaternionTranslationDestination<'_>,
    range: f32,
    vectors: CompressionVectors,
    numeric: &mut N,
) -> Result<(), CompressionError<N::Error>> {
    let count = admitted_count(
        count,
        destination
            .quaternions
            .len()
            .min(destination.translations.len()),
    )?;
    let mut q = source.quaternion(0)?; // 5c7c before translation5c94.
    let mut t = source.translation(0)?;
    // 5c98 constant 0x820229b8, 5cb0 scalar fdivs and 5cb4 rounded store.
    // This is not standalone TransCompress's estimate/refinement sequence.
    let translation_ratio = f32::from_bits(0x46ff_fe00) / range;
    for index in 0..count - 1 {
        let current_q = q;
        let current_t = t;
        q = source.quaternion(index + 1)?; // 5cf0 then 5cf8 prefetch.
        t = source.translation(index + 1)?;
        let scaled_q = scaled_quaternion(current_q, vectors);
        check_translation_range(current_t, range, index)?;
        let q = select_hemisphere(scaled_q, vectors);
        let t = current_t.map(|lane| lane * translation_ratio);
        compress_loop_record(
            q,
            t,
            &mut destination.quaternions[index],
            &mut destination.translations[index],
            numeric,
        )?;
    }
    let scaled_q = scaled_quaternion(q, vectors); // Final-element path5f84.
    check_translation_range(t, range, count - 1)?;
    let q = select_hemisphere(scaled_q, vectors);
    let t = t.map(|lane| lane * translation_ratio);
    compress_tail_record(
        q,
        t,
        &mut destination.quaternions[count - 1],
        &mut destination.translations[count - 1],
        numeric,
    )
}

fn compress_loop_record<N: ReckoningNumeric>(
    q: [f32; 4],
    t: [f32; 4],
    output_q: &mut [u16; 3],
    output_t: &mut [u16; 3],
    numeric: &mut N,
) -> Result<(), CompressionError<N::Error>> {
    // Conversion order5ef0,5f08,5f14,5f20,5f24,5f2c.
    let qz = integer_low16(q[2], numeric)?;
    let tx = integer_low16(t[0], numeric)?;
    let ty = integer_low16(t[1], numeric)?;
    let qx = integer_low16(q[0], numeric)?;
    let qy = integer_low16(q[1], numeric)?;
    let tz = integer_low16(t[2], numeric)?;
    output_q[2] = qz; // 5f48
    output_t[2] = tz; // 5f4c
    output_t[1] = ty; // 5f58
    output_t[0] = tx; // 5f5c
    output_q[1] = qy; // 5f64
    output_q[0] = qx; // 5f68
    Ok(())
}
fn compress_tail_record<N: ReckoningNumeric>(
    q: [f32; 4],
    t: [f32; 4],
    output_q: &mut [u16; 3],
    output_t: &mut [u16; 3],
    numeric: &mut N,
) -> Result<(), CompressionError<N::Error>> {
    let qy = integer_low16(q[1], numeric)?; // 6174
    let qz = integer_low16(q[2], numeric)?; // 617c
    let qx = integer_low16(q[0], numeric)?; // 618c
    output_q[1] = qy; // 61a4, before the remaining conversions.
    let ty = integer_low16(t[1], numeric)?; // 61ac
    let tz = integer_low16(t[2], numeric)?; // 61b0
    let tx = integer_low16(t[0], numeric)?; // 61c4
    output_q[0] = qx; // 61d4
    output_t[0] = tx; // 61d8
    output_q[2] = qz; // 61dc
    output_t[2] = tz; // 61e0
    output_t[1] = ty; // 61e4
    Ok(())
}
