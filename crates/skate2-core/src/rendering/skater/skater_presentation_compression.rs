//! Original presentation Q3 and translation compression, not pose publication.
//! Native `fctiwz` and reciprocal estimates use the existing numeric service.
//! Host float division/FMA, exceptional FP state and estimate policy remain
//! explicit numerical boundaries. No destination allocation is introduced.
use crate::reckoning_numeric::ReckoningNumeric;
use skate2_formats::andale::AndaleSqt;
use std::{cmp::Ordering, fmt};

#[path = "skater_presentation_compression_combined.rs"]
mod combined;
pub(crate) use combined::{QuaternionTranslationDestination, try_compress_quaternion_translation};
#[cfg(test)]
#[path = "skater_presentation_compression_tests.rs"]
mod tests;

/// Borrowed original SQT records, or direct vector arrays for controlled callers.
/// Access remains at the source load sites; no eagerly gathered pose is created.
#[derive(Clone, Copy)]
pub(crate) enum CompressionSource<'a> {
    Sqts(&'a [AndaleSqt]),
    Vectors {
        quaternions: &'a [[f32; 4]],
        translations: &'a [[f32; 4]],
    },
}
impl CompressionSource<'_> {
    fn quaternion<E>(self, index: usize) -> Result<[f32; 4], CompressionError<E>> {
        match self {
            Self::Sqts(values) => values.get(index).map(|value| value.rotation),
            Self::Vectors { quaternions, .. } => quaternions.get(index).copied(),
        }
        .ok_or(CompressionError::SourceBounds {
            translation: false,
            index,
        })
    }
    fn translation<E>(self, index: usize) -> Result<[f32; 4], CompressionError<E>> {
        match self {
            Self::Sqts(values) => values.get(index).map(|value| value.translation),
            Self::Vectors { translations, .. } => translations.get(index).copied(),
        }
        .ok_or(CompressionError::SourceBounds {
            translation: true,
            index,
        })
    }
}

/// Actual caller vectors. The native `vOne` parameter is unused in these Q3
/// primitives and is deliberately not sampled by this arithmetic interface.
#[derive(Clone, Copy)]
pub(crate) struct CompressionVectors {
    pub(crate) ratio: [f32; 4],
    pub(crate) zero: [f32; 4],
    pub(crate) minus_one: [f32; 4],
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CompressionError<E> {
    Numeric(E),
    NonPositiveCount(i32),
    DestinationBounds {
        required: usize,
        available: usize,
    },
    SourceBounds {
        translation: bool,
        index: usize,
    },
    RangeAssertion {
        index: usize,
        axis: usize,
        value_bits: u32,
        range_bits: u32,
    },
}
impl<E: fmt::Display> fmt::Display for CompressionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "presentation compression numeric service: {error}"),
            Self::NonPositiveCount(count) => {
                write!(f, "presentation compression invalid native count {count}")
            }
            Self::DestinationBounds {
                required,
                available,
            } => write!(
                f,
                "presentation compression needs {required} destination records, has {available}"
            ),
            Self::SourceBounds { translation, index } => write!(
                f,
                "presentation compression missing {} record {index}",
                if *translation {
                    "translation"
                } else {
                    "quaternion"
                }
            ),
            Self::RangeAssertion {
                index,
                axis,
                value_bits,
                range_bits,
            } => write!(
                f,
                "presentation translation {index} axis {axis} fails strict range ({value_bits:08x}, range {range_bits:08x})"
            ),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CompressionError<E> {}

/// Explicit host admission guard: the native invalid/nonpositive-count call
/// still loads its first input and executes a final-element tail. It is not an
/// empty operation. Destination capacity is a host memory-safety prerequisite.
fn admitted_count<E>(count: i32, available: usize) -> Result<usize, CompressionError<E>> {
    let positive = usize::try_from(count)
        .ok()
        .filter(|count| *count != 0)
        .ok_or(CompressionError::NonPositiveCount(count))?;
    if positive > available {
        return Err(CompressionError::DestinationBounds {
            required: positive,
            available,
        });
    }
    Ok(positive)
}

fn scaled_quaternion(quaternion: [f32; 4], vectors: CompressionVectors) -> [f32; 4] {
    std::array::from_fn(|lane| quaternion[lane] * vectors.ratio[lane])
}
fn select_hemisphere(scaled: [f32; 4], vectors: CompressionVectors) -> [f32; 4] {
    // VMX CR6 LT is all-true. Supplied zero can be nonuniform; retain that
    // contract instead of reducing it to a hardcoded scalar zero comparison.
    if vectors.zero.iter().all(|zero| *zero > scaled[3]) {
        std::array::from_fn(|lane| scaled[lane] * vectors.minus_one[lane])
    } else {
        scaled
    }
}
fn integer_low16<N: ReckoningNumeric>(
    value: f32,
    numeric: &mut N,
) -> Result<u16, CompressionError<N::Error>> {
    let bytes = numeric
        .truncate_i32(value)
        .map_err(CompressionError::Numeric)?
        .to_be_bytes();
    Ok(u16::from_be_bytes([bytes[2], bytes[3]]))
}

/// `QuatCompressQuantize3`, 0x827d5af0..0x827d5c54. Destination entries retain
/// logical X/Y/Z halfwords; the eventual stream writer must emit big endian.
#[expect(
    clippy::needless_range_loop,
    reason = "source next-record prefetch precedes current-record stores and final tail"
)]
pub(crate) fn try_compress_q3<N: ReckoningNumeric>(
    source: CompressionSource<'_>,
    count: i32,
    destination: &mut [[u16; 3]],
    vectors: CompressionVectors,
    numeric: &mut N,
) -> Result<(), CompressionError<N::Error>> {
    let count = admitted_count(count, destination.len())?;
    let mut current = source.quaternion(0)?; // 5afc, before the loop gate.
    for index in 0..count - 1 {
        let previous = current;
        current = source.quaternion(index + 1)?; // 5b24 prefetch before arithmetic.
        let q = select_hemisphere(scaled_quaternion(previous, vectors), vectors);
        let y = integer_low16(q[1], numeric)?;
        let x = integer_low16(q[0], numeric)?;
        let z = integer_low16(q[2], numeric)?;
        destination[index][1] = y; // 5ba8, all three conversions already completed.
        destination[index][0] = x;
        destination[index][2] = z;
    }
    let q = select_hemisphere(scaled_quaternion(current, vectors), vectors);
    let y = integer_low16(q[1], numeric)?;
    let x = integer_low16(q[0], numeric)?;
    destination[count - 1][1] = y; // 5c34 precedes final Z conversion.
    let z = integer_low16(q[2], numeric)?;
    destination[count - 1][0] = x;
    destination[count - 1][2] = z;
    Ok(())
}

fn check_translation_range<E>(
    value: [f32; 4],
    range: f32,
    index: usize,
) -> Result<(), CompressionError<E>> {
    for (axis, value) in value.into_iter().take(3).enumerate() {
        // Raw compare operands use the current axis splat, not full source W.
        // Preserve upper-then-lower short circuit and unordered assertion.
        if range.partial_cmp(&value) != Some(Ordering::Greater)
            || value.partial_cmp(&-range) != Some(Ordering::Greater)
        {
            return Err(CompressionError::RangeAssertion {
                index,
                axis,
                value_bits: value.to_bits(),
                range_bits: range.to_bits(),
            });
        }
    }
    Ok(())
}

/// `TransCompress`, 0x827d6200..0x827d646c. The combined Q3/T compressor below
/// uses scalar division instead; these numerical graphs must not be merged.
pub(crate) fn try_compress_translation<N: ReckoningNumeric>(
    translation: [f32; 4],
    range: f32,
    ratio: [f32; 4],
    destination: &mut [u16; 3],
    numeric: &mut N,
) -> Result<(), CompressionError<N::Error>> {
    check_translation_range(translation, range, 0)?;
    let seed = numeric
        .reciprocal_estimate(range)
        .map_err(CompressionError::Numeric)?;
    // 63fc/6404 VNMSUB negates the rounded fused result, including exact zero.
    let residual = -(seed.mul_add(range, -1.0));
    let first = seed.mul_add(residual, seed);
    let residual = -(first.mul_add(range, -1.0));
    let inverse = first.mul_add(residual, first);
    let scaled: [f32; 4] = std::array::from_fn(|lane| translation[lane] * (inverse * ratio[lane]));
    let z = integer_low16(scaled[2], numeric)?;
    let x = integer_low16(scaled[0], numeric)?;
    let y = integer_low16(scaled[1], numeric)?;
    destination[0] = x; // 6444, then 644c/6450.
    destination[2] = z;
    destination[1] = y;
    Ok(())
}
