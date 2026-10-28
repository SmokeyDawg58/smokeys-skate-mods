//! `pegasus::tGrabData` (0x20), `tGrab` (0x50), Fixup 0x829c5798.
//! Every non-null pointer is relative to the containing tGrabData, including
//! the three pointers within each tGrab. Section-2 references name a tGrab
//! inside this object; they are resolved by the arena owner, not this parser.
use super::{
    AxisAlignedBox, PegasusError, as_usize, be_f32_array, be_u32, be_u64, counted_range,
    element_at_offset, parse_aabb, require,
};

pub const GRAB_DATA_TYPE_ID: u32 = 0x00eb_001f;
const HEADER_SIZE: usize = 0x20;
const GRAB_SIZE: usize = 0x50;

#[derive(Clone, Debug, PartialEq)]
pub struct Grab {
    pub bounds: AxisAlignedBox,
    pub attribute_class: u64,
    pub attribute_collection: u64,
    pub spline_points: Vec<[f32; 4]>,
    pub approach_points: Vec<[f32; 4]>,
    pub approach_vectors: Vec<[f32; 4]>,
    pub flags: u32,
    pub kind: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrabData {
    pub grabs_offset: u32,
    pub grabs: Vec<Grab>,
}

fn array(bytes: &[u8], field: usize, count: u32, stride: usize) -> Result<&[u8], PegasusError> {
    let offset = be_u32(bytes, field)?;
    if count != 0 && offset == 0 {
        return Err(PegasusError::MissingOffset {
            field_offset: field,
        });
    }
    counted_range(bytes, as_usize(offset)?, count, stride)
}

fn vectors(bytes: &[u8], field: usize, count: u8) -> Result<Vec<[f32; 4]>, PegasusError> {
    let data = array(bytes, field, u32::from(count), 16)?;
    (0..usize::from(count))
        .map(|i| be_f32_array::<4>(data, i * 16))
        .collect()
}

impl GrabData {
    /// Decode the unfixed, big-endian serialized object without guest pointers.
    ///
    /// # Errors
    /// Returns an error for truncated records, null nonempty arrays or overflow.
    pub fn parse(bytes: &[u8]) -> Result<Self, PegasusError> {
        require(bytes, 0, HEADER_SIZE)?;
        let count = be_u32(bytes, 0)?;
        array(bytes, 0x10, count, GRAB_SIZE)?;
        // Validate the three backing arrays, but follow each record's own
        // pointers below; no assumption of an interleaved per-grab allocation.
        for (count_field, pointer_field) in [(4, 0x14), (8, 0x18), (12, 0x1c)] {
            array(bytes, pointer_field, be_u32(bytes, count_field)?, 16)?;
        }
        let grabs_offset = be_u32(bytes, 0x10)?;
        let start = as_usize(grabs_offset)?;
        let mut grabs = Vec::with_capacity(as_usize(count)?);
        for i in 0..as_usize(count)? {
            let at = start + i * GRAB_SIZE; // range/multiplication checked above
            grabs.push(Grab {
                bounds: parse_aabb(bytes, at)?,
                attribute_class: be_u64(bytes, at + 0x20)?,
                attribute_collection: be_u64(bytes, at + 0x28)?,
                spline_points: vectors(bytes, at + 0x30, bytes[at + 0x44])?,
                approach_points: vectors(bytes, at + 0x34, bytes[at + 0x45])?,
                approach_vectors: vectors(bytes, at + 0x38, bytes[at + 0x46])?,
                flags: be_u32(bytes, at + 0x3c)?,
                kind: be_u32(bytes, at + 0x40)?,
            });
        }
        Ok(Self {
            grabs_offset,
            grabs,
        })
    }

    #[must_use]
    pub fn grab_at_object_offset(&self, offset: u32) -> Option<&Grab> {
        element_at_offset(&self.grabs, offset, self.grabs_offset, GRAB_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![0; 0xf0];
        for (at, value) in [
            (0, 2u32),
            (4, 2),
            (12, 1),
            (0x10, 0x20),
            (0x14, 0xc0),
            (0x1c, 0xe0),
            (0x50, 0xc0),
            (0x58, 0xe0),
            (0xa0, 0xd0),
            (0xa8, 0xe0),
            (0x5c, 0x3e4),
            (0xac, 0x804),
        ] {
            b[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        b[0x64] = 1;
        b[0x66] = 1;
        b[0xb4] = 1;
        b[0xb6] = 1;
        for (at, value) in [
            (0xc0, 2.0f32),
            (0xcc, 1.0),
            (0xd0, 5.0),
            (0xdc, -0.0),
            (0xe4, 1.0),
        ] {
            b[at..at + 4].copy_from_slice(&value.to_be_bytes());
        }
        b
    }
    #[test]
    fn pointers_use_container_base_and_keep_shared_vectors_and_w() {
        let data = GrabData::parse(&fixture()).unwrap();
        let a = data.grab_at_object_offset(0x20).unwrap();
        let b = data.grab_at_object_offset(0x70).unwrap();
        assert_eq!(a.spline_points[0], [2.0, 0.0, 0.0, 1.0]);
        assert_eq!(b.spline_points[0][0], 5.0);
        assert_eq!(b.spline_points[0][3].to_bits(), (-0.0f32).to_bits());
        assert_eq!(a.approach_vectors, b.approach_vectors);
        assert!(a.approach_points.is_empty());
        assert_eq!((a.flags, b.flags), (0x3e4, 0x804));
        assert!(data.grab_at_object_offset(0x21).is_none());
        assert!(data.grab_at_object_offset(0xc0).is_none());
    }
    #[test]
    fn invalid_array_pointers_fail_instead_of_reading_the_header() {
        let mut b = fixture();
        b[0xa0..0xa4].fill(0);
        assert!(matches!(
            GrabData::parse(&b),
            Err(PegasusError::MissingOffset { field_offset: 0xa0 })
        ));
        let b = fixture();
        assert!(GrabData::parse(&b[..0xef]).is_err());
    }
}
