//! Retained `ShakeEffect.Load82e43ce0` fields for the actual camera constructor.
use crate::{skeleton_root_transforms::SkeletonRootVector, token_file_decimal::finite_decimal};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ShakeLoadError {
    Encoding,
    MissingToken,
    UnsupportedNumber(String),
    Allocation,
}
impl std::fmt::Display for ShakeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "camera shake load: {self:?}")
    }
}
/// Allocation and frame publication prefixes survive a failed source read.
pub(crate) struct ShakeKeyframes {
    pub(crate) count: u32,
    pub(crate) positions: Option<Vec<Option<SkeletonRootVector>>>,
    pub(crate) rotations: Option<Vec<Option<SkeletonRootVector>>>,
}
impl ShakeKeyframes {
    /// `ShakeEffect` ctor82e45bd8 sets count0; array pointers are not initialized.
    /// None here means unwritten allocation ownership, not a claimed native NULL.
    pub(crate) const fn constructed() -> Self {
        Self {
            count: 0,
            positions: None,
            rotations: None,
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "native signed fcfid then frsp")]
    pub(crate) fn load(&mut self, bytes: &[u8]) -> Result<(), ShakeLoadError> {
        if !bytes.is_ascii()
            || bytes
                .iter()
                .any(|b| b.is_ascii_control() && !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            return Err(ShakeLoadError::Encoding);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| ShakeLoadError::Encoding)?;
        let mut tokens = text.split_ascii_whitespace();
        let token = tokens.next().ok_or(ShakeLoadError::MissingToken)?;
        let count = token
            .parse::<i32>()
            .map_err(|_| ShakeLoadError::UnsupportedNumber(token.to_owned()))?;
        self.count = u32::from_ne_bytes(count.to_ne_bytes());
        // Native saturates the requested byte size to ffffffff above this count.
        // The host stops at that allocation boundary, after publishing count.
        if self.count > 0x0fff_ffff {
            return Err(ShakeLoadError::Allocation);
        }
        self.positions = Some(allocate(self.count as usize)?);
        self.rotations = Some(allocate(self.count as usize)?);
        let positions = self.positions.as_mut().unwrap();
        let rotations = self.rotations.as_mut().unwrap();
        let mut rotation_sum = [0.0; 3];
        let mut position_sum = [0.0; 3];
        for index in 0..self.count as usize {
            let rotation = [
                number(&mut tokens)?,
                number(&mut tokens)?,
                number(&mut tokens)?,
            ];
            number(&mut tokens)?;
            let position = [
                number(&mut tokens)?,
                number(&mut tokens)?,
                number(&mut tokens)?,
            ];
            number(&mut tokens)?;
            // All eight conversions precede the first native axis insertion.
            rotations[index] = Some(SkeletonRootVector {
                xyz: rotation,
                w: None,
            });
            positions[index] = Some(SkeletonRootVector {
                xyz: position,
                w: None,
            });
            for lane in 0..3 {
                rotation_sum[lane] += rotation[lane];
            }
            for lane in 0..3 {
                position_sum[lane] += position[lane];
            }
        }
        // The native reciprocal executes even for zero keyframes.
        let reciprocal = 1.0 / (count as f32);
        let rotation_mean = rotation_sum.map(|value| value * reciprocal);
        let position_mean = position_sum.map(|value| value * reciprocal);
        for index in 0..self.count as usize {
            let position = positions[index].as_mut().unwrap();
            for (value, mean) in position.xyz.iter_mut().zip(position_mean) {
                *value -= mean;
            }
            let rotation = rotations[index].as_mut().unwrap();
            for (value, mean) in rotation.xyz.iter_mut().zip(rotation_mean) {
                *value -= mean;
            }
        }
        Ok(())
    }
}
fn allocate(count: usize) -> Result<Vec<Option<SkeletonRootVector>>, ShakeLoadError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| ShakeLoadError::Allocation)?;
    values.resize(count, None);
    Ok(values)
}
fn number(tokens: &mut std::str::SplitAsciiWhitespace<'_>) -> Result<f32, ShakeLoadError> {
    let token = tokens.next().ok_or(ShakeLoadError::MissingToken)?;
    finite_decimal(token).ok_or_else(|| ShakeLoadError::UnsupportedNumber(token.to_owned()))
}
#[cfg(test)]
#[path = "camera_shake_tests.rs"]
mod tests;
