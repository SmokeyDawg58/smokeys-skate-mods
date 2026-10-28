//! Native RAW packed expansion at827d1220/1300/13b8; supplied estimate/reduction.
use crate::reckoning_numeric::ReckoningNumeric;
pub(crate) trait PresentationDecodeNumeric: ReckoningNumeric {
    fn dot4(&mut self, first: [f32; 4], second: [f32; 4]) -> Result<f32, Self::Error>;
}
#[derive(Debug)]
pub(crate) enum DecodeError<E> {
    Bounds { offset: usize, available: usize },
    Numeric(E),
}
impl<E: std::fmt::Display> std::fmt::Display for DecodeError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bounds { offset, available } => write!(
                f,
                "RAW decode needs8 bytes at{offset}, backing size{available}"
            ),
            Self::Numeric(e) => e.fmt(f),
        }
    }
}
#[expect(
    clippy::cast_precision_loss,
    reason = "native signed word to float conversion before power-of-two scaling"
)]
fn expanded<E>(bytes: &[u8], offset: usize) -> Result<[f32; 4], DecodeError<E>> {
    let source = bytes
        .get(offset..)
        .and_then(|v| v.get(..8))
        .ok_or(DecodeError::Bounds {
            offset,
            available: bytes.len(),
        })?;
    let halves: [u16; 4] =
        std::array::from_fn(|i| u16::from_be_bytes([source[2 * i], source[2 * i + 1]]));
    Ok(std::array::from_fn(|i| {
        let neighbor = if i & 1 == 0 {
            u32::from(halves[i + 1] >> 15)
        } else {
            0
        };
        let bits = (u32::from(halves[i]) << 16) | ((u32::from(halves[i]) << 1) & 0xffff) | neighbor;
        let signed = i32::from_be_bytes(bits.to_be_bytes());
        let converted = (signed as f32) * f32::from_bits(0x3000_0000);
        converted * f32::from_bits(0x4700_0000)
    }))
}
fn refined(value: f32, mut estimate: f32) -> f32 {
    for _ in 0..2 {
        let square = estimate * estimate;
        let half = estimate * 0.5;
        let residual = -value.mul_add(square, -1.0);
        estimate = half.mul_add(residual, estimate);
    }
    estimate
}
/// Source Q4 normalization has no zero-norm shortcut.
pub(crate) fn quaternion4<N: PresentationDecodeNumeric>(
    bytes: &[u8],
    offset: usize,
    numeric: &mut N,
) -> Result<[f32; 4], DecodeError<N::Error>> {
    let value = expanded(bytes, offset)?.map(|v| v * f32::from_bits(0x3800_0100));
    let norm = numeric.dot4(value, value).map_err(DecodeError::Numeric)?;
    let seed = numeric
        .reciprocal_sqrt_estimate(norm)
        .map_err(DecodeError::Numeric)?;
    let inverse = refined(norm, seed);
    Ok(value.map(|v| v * inverse))
}
/// Source evaluates rsqrt even for a zero/negative radicand, then selects+0.
pub(crate) fn quaternion3<N: PresentationDecodeNumeric>(
    bytes: &[u8],
    offset: usize,
    numeric: &mut N,
) -> Result<[f32; 4], DecodeError<N::Error>> {
    let mut value = expanded(bytes, offset)?;
    value[3] = 0.0;
    let mut value = value.map(|v| v * f32::from_bits(0x3800_0100));
    let norm = numeric.dot4(value, value).map_err(DecodeError::Numeric)?;
    let radicand = 1.0 - norm;
    let seed = numeric
        .reciprocal_sqrt_estimate(radicand)
        .map_err(DecodeError::Numeric)?;
    let root = radicand * refined(radicand, seed);
    value[3] = if radicand <= 0.0 { 0.0 } else { root };
    Ok(value)
}
pub(crate) fn translation<E>(
    bytes: &[u8],
    offset: usize,
    range: f32,
) -> Result<[f32; 4], DecodeError<E>> {
    let factor = range * f32::from_bits(0x3800_0100);
    let mut value = expanded(bytes, offset)?.map(|v| v * factor);
    value[3] = 1.0;
    Ok(value)
}
