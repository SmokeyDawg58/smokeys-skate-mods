//! `pegasus::tVisualIndicatorData` (0x10) and `tVisualIndicator` (0x40),
//! Fixup 0x829c5f20. `m_pVIs`, `m_pSegs` and every indicator's own `m_pSegs`
//! are relative to the containing tVisualIndicatorData; an indicator's null
//! `m_pSegs` stays null.
use super::{
    AxisAlignedBox, PegasusError, as_usize, be_f32_array, be_u32, be_u64, counted_range,
    parse_aabb, require,
};

pub const VISUAL_INDICATOR_DATA_TYPE_ID: u32 = 0x00eb_0021;
const HEADER_SIZE: usize = 0x10;
const INDICATOR_SIZE: usize = 0x40;
const SEGMENT_SIZE: usize = 0x40;

/// One `tVisualIndicator`: a ribbon spline's segment frames.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualIndicator {
    pub guid: u64,
    /// `m_uiGuidLocal`, which `ribbon_spline` collections name.
    pub guid_local: u64,
    /// `m_BBox`.
    pub bounds: AxisAlignedBox,
    /// `m_uiFlags`: bit 0 is `Spline::mBillboard`.
    pub flags: u32,
    /// `m_pSegs`: one `Matrix44` per segment, rows in order.
    pub segments: Vec<[[f32; 4]; 4]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisualIndicatorData {
    /// `m_uiNumSegs`: the segments of every indicator together.
    pub segment_count: u32,
    pub indicators: Vec<VisualIndicator>,
}

fn segments(
    bytes: &[u8],
    offset: u32,
    count: u32,
    field: usize,
) -> Result<Vec<[[f32; 4]; 4]>, PegasusError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if offset == 0 {
        return Err(PegasusError::MissingOffset {
            field_offset: field,
        });
    }
    let data = counted_range(bytes, as_usize(offset)?, count, SEGMENT_SIZE)?;
    (0..as_usize(count)?)
        .map(|segment| {
            let at = segment * SEGMENT_SIZE;
            Ok([
                be_f32_array::<4>(data, at)?,
                be_f32_array::<4>(data, at + 0x10)?,
                be_f32_array::<4>(data, at + 0x20)?,
                be_f32_array::<4>(data, at + 0x30)?,
            ])
        })
        .collect()
}

impl VisualIndicatorData {
    /// Decode the unfixed, big-endian serialized object without guest pointers.
    ///
    /// # Errors
    /// Returns an error for truncated records, null nonempty arrays or overflow.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, HEADER_SIZE)?;
        let count = be_u32(bytes, 0)?;
        let segment_count = be_u32(bytes, 4)?;
        let indicators_offset = be_u32(bytes, 8)?;
        if count != 0 && indicators_offset == 0 {
            return Err(PegasusError::MissingOffset { field_offset: 8 });
        }
        let start = as_usize(indicators_offset)?;
        counted_range(bytes, start, count, INDICATOR_SIZE)?;
        let mut indicators = Vec::with_capacity(as_usize(count)?);
        for index in 0..as_usize(count)? {
            let at = start + index * INDICATOR_SIZE;
            let segment_total = be_u32(bytes, at + 0x34)?;
            indicators.push(VisualIndicator {
                guid: be_u64(bytes, at)?,
                guid_local: be_u64(bytes, at + 8)?,
                bounds: parse_aabb(bytes, at + 0x10)?,
                flags: be_u32(bytes, at + 0x30)?,
                segments: segments(bytes, be_u32(bytes, at + 0x38)?, segment_total, at + 0x38)?,
            });
        }
        Ok(Self {
            segment_count,
            indicators,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_f32(bytes: &mut [u8], offset: usize, value: f32) {
        put_u32(bytes, offset, value.to_bits());
    }

    #[test]
    fn parses_indicators_with_object_relative_segment_arrays() {
        let mut bytes = vec![0_u8; 0x10 + 2 * 0x40 + 3 * 0x40];
        put_u32(&mut bytes, 0, 2);
        put_u32(&mut bytes, 4, 3);
        put_u32(&mut bytes, 8, 0x10);
        put_u32(&mut bytes, 12, 0x90);
        bytes[0x10 + 8..0x10 + 16].copy_from_slice(&0x0001_8e2b_03e3_8707_u64.to_be_bytes());
        put_f32(&mut bytes, 0x10 + 0x10, -1.0);
        put_f32(&mut bytes, 0x10 + 0x20, 2.0);
        put_u32(&mut bytes, 0x10 + 0x30, 9);
        put_u32(&mut bytes, 0x10 + 0x34, 2);
        put_u32(&mut bytes, 0x10 + 0x38, 0x90);
        put_u32(&mut bytes, 0x50 + 0x34, 1);
        put_u32(&mut bytes, 0x50 + 0x38, 0x110);
        put_f32(&mut bytes, 0x90 + 0x30, 0.25);
        put_f32(&mut bytes, 0xd0 + 0x34, 4.0);
        put_f32(&mut bytes, 0x110 + 0x3c, 1.0);
        let data = VisualIndicatorData::parse(&bytes).unwrap();
        assert_eq!(data.segment_count, 3);
        assert_eq!(data.indicators.len(), 2);
        let first = &data.indicators[0];
        assert_eq!(first.guid_local, 0x0001_8e2b_03e3_8707);
        assert_eq!(
            (
                first.bounds.minimum[0],
                first.bounds.maximum[0],
                first.flags
            ),
            (-1.0, 2.0, 9)
        );
        assert_eq!(first.segments.len(), 2);
        assert_eq!(first.segments[0][3][0], 0.25);
        assert_eq!(first.segments[1][3][1], 4.0);
        assert_eq!(data.indicators[1].segments[0][3][3], 1.0);
    }

    #[test]
    fn a_null_segment_array_with_segments_is_rejected() {
        let mut bytes = vec![0_u8; 0x50];
        put_u32(&mut bytes, 0, 1);
        put_u32(&mut bytes, 8, 0x10);
        put_u32(&mut bytes, 0x10 + 0x34, 1);
        assert_eq!(
            VisualIndicatorData::parse(&bytes),
            Err(PegasusError::MissingOffset { field_offset: 0x48 })
        );
    }
}
