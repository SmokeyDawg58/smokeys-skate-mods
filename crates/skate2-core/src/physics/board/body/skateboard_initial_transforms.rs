//! Initial local Part placement (0x82b2bc10) and self-culling (0x82b34d98).
//!
//! This is not full `SkateboardBody::Init`, live `Part::SetTransform`, island
//! admission, or the later spawn transform. The writer path performs each
//! source-ordered detached Part write as its inputs become available. The legacy
//! value-returning adapter stages into scratch and publishes no partial result.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "authored initial placement awaits live skater integration"
    )
)]

use crate::{
    skateboard_attributes::{SkateboardAttributeClass, SkateboardAttributeRead},
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
};
use skate2_formats::collision::BitTable;
use std::fmt;

#[cfg(test)]
#[path = "skateboard_initial_transforms_tests.rs"]
mod tests;

const DECK_LENGTH: u64 = 0xc94d_6509_1dcb_828b;
const FRONT_OFFSET: u64 = 0xe02d_cbd0_7830_d3a5;
const REAR_OFFSET: u64 = 0xa69c_ca4e_15db_a369;
const VERTICAL_OFFSET: u64 = 0x2be0_8dd7_a920_7622;
const LATERAL_OFFSET: u64 = 0x5a84_c435_dc34_0af8;

/// The single native `XMVectorSinCos` call at 0x82b2c28c. Returned values are
/// sine, cosine. The native caller broadcasts the scalar angle to all lanes;
/// the stored columns consume the corresponding sine/cosine values.
pub(crate) trait SkateboardInitialTransformNumeric {
    type Error;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error>;
}

/// Host libm trig is an explicit approximation, not `XMVector` result bits.
pub(crate) struct HostSkateboardInitialTransformNumeric;

impl SkateboardInitialTransformNumeric for HostSkateboardInitialTransformNumeric {
    type Error = core::convert::Infallible;
    fn sine_cosine(&mut self, angle: f32) -> Result<(f32, f32), Self::Error> {
        Ok(angle.sin_cos())
    }
}

#[derive(Debug)]
pub(crate) enum SkateboardInitialTransformError<A, N, W = core::convert::Infallible> {
    Attribute(A),
    Numeric(N),
    Write(W),
}

impl<A: fmt::Display, N: fmt::Display, W: fmt::Display> fmt::Display
    for SkateboardInitialTransformError<A, N, W>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Attribute(error) => write!(f, "initial skateboard placement attribute: {error}"),
            Self::Numeric(error) => write!(f, "initial skateboard placement numeric: {error}"),
            Self::Write(error) => write!(f, "initial skateboard Part write: {error}"),
        }
    }
}

/// Only the source constructor can produce this seven-update sequence.
/// Matrix words are retained without affine-W or finite-value normalization.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SkateboardInitialTransforms {
    ordered_updates: [(usize, [f32; 16]); 7],
}

impl SkateboardInitialTransforms {
    pub(crate) const fn ordered_updates(&self) -> &[(usize, [f32; 16]); 7] {
        &self.ordered_updates
    }

    /// Definition order for the existing assembly attachment consumer.
    pub(crate) fn by_part(&self) -> [[f32; 16]; 7] {
        let mut result = [[0.0; 16]; 7];
        for &(part, transform) in &self.ordered_updates {
            result[part] = transform;
        }
        result
    }
}

/// `SkateboardBody::InitializeTransforms`, 0x82b2bc10..0x82b2c38c.
/// All 14 lookups go through the shared runtime fetch/default provider.
/// Repeated reads and delayed pointer dereferences are never snapshotted.
pub(crate) fn try_initialize_skateboard_transforms<
    A: SkateboardAttributeRead,
    N: SkateboardInitialTransformNumeric,
>(
    attributes: &mut A,
    numeric: &mut N,
) -> Result<SkateboardInitialTransforms, SkateboardInitialTransformError<A::Error, N::Error>> {
    let mut updates = [(6, FRESH_VOLUME_TRANSFORM); 7];
    let mut count = 0;
    try_initialize_skateboard_transforms_with_writer(attributes, numeric, |part, transform| {
        updates[count] = (part, transform);
        count += 1;
        Ok::<_, core::convert::Infallible>(())
    })?;
    Ok(SkateboardInitialTransforms {
        ordered_updates: updates,
    })
}

/// Same source numeric/read graph, writing actual detached Parts in order
/// 6,0,1,2,3,4,5. A failure preserves completed writes; no later read or write
/// runs. The caller supplies the already-created Assembly's checked setter.
pub(crate) fn try_initialize_skateboard_transforms_with_writer<
    A: SkateboardAttributeRead,
    N: SkateboardInitialTransformNumeric,
    W,
>(
    attributes: &mut A,
    numeric: &mut N,
    mut write: impl FnMut(usize, [f32; 16]) -> Result<(), W>,
) -> Result<(), SkateboardInitialTransformError<A::Error, N::Error, W>> {
    use SkateboardAttributeClass::{Deck, Trucks, Wheels};
    write(6, FRESH_VOLUME_TRANSFORM).map_err(SkateboardInitialTransformError::Write)?;
    let length_pointer = attributes
        .lookup(Deck, DECK_LENGTH)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let front_pointer = attributes
        .lookup(Trucks, FRONT_OFFSET)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let length = attributes
        .float(&length_pointer)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let front_offset = attributes
        .float(&front_pointer)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let front = length.mul_add(0.5, front_offset);

    let rear_pointer = attributes
        .lookup(Trucks, REAR_OFFSET)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let length_pointer = attributes
        .lookup(Deck, DECK_LENGTH)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let rear_offset = attributes
        .float(&rear_pointer)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    let negated_offset = -rear_offset;
    let length = attributes
        .float(&length_pointer)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    // fneg(offset), then fnmsubs(length,half,negated_offset), not a separate
    // half-length product and addition. Keep both source sign operations.
    let rear = -length.mul_add(0.5, -negated_offset);

    for wheel in 0..4 {
        let vertical_pointer = attributes
            .lookup(Trucks, VERTICAL_OFFSET)
            .map_err(SkateboardInitialTransformError::Attribute)?;
        let lateral_pointer = attributes
            .lookup(Wheels, LATERAL_OFFSET)
            .map_err(SkateboardInitialTransformError::Attribute)?;
        let lateral = attributes
            .float(&lateral_pointer)
            .map_err(SkateboardInitialTransformError::Attribute)?;
        let vertical = attributes
            .float(&vertical_pointer)
            .map_err(SkateboardInitialTransformError::Attribute)?;
        let lateral = if wheel % 2 == 0 { -lateral } else { lateral };
        let longitudinal = if wheel < 2 { front } else { rear };
        write(
            wheel,
            translated_identity([lateral, vertical, longitudinal, 0.0]),
        )
        .map_err(SkateboardInitialTransformError::Write)?;
    }

    let vertical = attributes
        .read_float(Trucks, VERTICAL_OFFSET)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    write(4, translated_identity([0.0, vertical, front, 0.0]))
        .map_err(SkateboardInitialTransformError::Write)?;
    let (sine, cosine) = numeric
        .sine_cosine(f32::from_bits(0x3fc9_0fdb))
        .map_err(SkateboardInitialTransformError::Numeric)?;
    let mut rear_transform = [
        cosine, 0.0, -sine, cosine, 0.0, 1.0, 0.0, 0.0, sine, 0.0, cosine, sine, 0.0, 0.0, 0.0, 0.0,
    ];
    let vertical = attributes
        .read_float(Trucks, VERTICAL_OFFSET)
        .map_err(SkateboardInitialTransformError::Attribute)?;
    rear_transform[12..].copy_from_slice(&[0.0, vertical, rear, 0.0]);
    write(5, rear_transform).map_err(SkateboardInitialTransformError::Write)
}

fn translated_identity(translation: [f32; 4]) -> [f32; 16] {
    let mut transform = FRESH_VOLUME_TRANSFORM;
    transform[12..].copy_from_slice(&translation);
    transform
}

/// `InitializeCullingTable` (0x82b34d98) composed with `BitTable::Initialize`
/// (0x82bc3308). All 49 directed bits are set; padding remains initialized zero.
/// A retained nonnull table still permits the collision job's self-pair append
/// before it suppresses contacts. This is not a request to omit that job path.
pub(crate) fn initialize_skateboard_culling_table() -> BitTable {
    let mut table = BitTable {
        rows: 7,
        columns: 7,
        array_size: (7 * 7 + 31) >> 5,
        words: vec![0; 2],
    };
    for row in 0..7 {
        for column in 0..7 {
            let bit = table.columns * row + column;
            table.words[(bit >> 5) as usize] |= 1 << (bit & 31);
        }
    }
    table
}
