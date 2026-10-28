//! EA APT assets used by Skate's Flash frontend. This reads original data;
//! it does not execute ActionScript or invent missing timeline behavior.
//!
//! Layouts: `AptCharacterAnimation::Fixup`82a1beb0, `AptMovie::resolve`
//! 82a00a58, `AptLoader::CompleteLoad`82a1cbe0 and `AptGeometryInfo`82a6fc90.

mod font;
mod geometry;
mod movie;
pub use font::{BitmapFont, FontError, Glyph};
pub use geometry::{Geometry, GeometryElement, GeometryKind, GeometryShape};
pub use movie::*;

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AptError {
    Magic(&'static str),
    Bounds {
        offset: usize,
        size: usize,
    },
    Overflow,
    NullTable,
    UnterminatedString(usize),
    InvalidUtf8(usize),
    Invalid {
        offset: usize,
        field: &'static str,
        value: u32,
    },
    Unsupported {
        offset: usize,
        kind: &'static str,
        value: u32,
    },
}

impl fmt::Display for AptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid APT asset: {self:?}")
    }
}
impl std::error::Error for AptError {}

/// Serialized `AptMatrix` field order (24 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy)]
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn bytes(self, offset: usize, size: usize) -> Result<&'a [u8], AptError> {
        let end = offset.checked_add(size).ok_or(AptError::Overflow)?;
        self.0
            .get(offset..end)
            .ok_or(AptError::Bounds { offset, size })
    }
    fn u32(self, offset: usize) -> Result<u32, AptError> {
        Ok(u32::from_be_bytes(
            self.bytes(offset, 4)?.try_into().unwrap(),
        ))
    }
    fn offset(self, offset: usize) -> Result<usize, AptError> {
        usize::try_from(self.u32(offset)?).map_err(|_| AptError::Overflow)
    }
    fn f32(self, offset: usize) -> Result<f32, AptError> {
        Ok(f32::from_bits(self.u32(offset)?))
    }
    fn array(
        self,
        offset: usize,
        count: usize,
        stride: usize,
    ) -> Result<std::ops::Range<usize>, AptError> {
        if count != 0 && offset == 0 {
            return Err(AptError::NullTable);
        }
        self.bytes(offset, count.checked_mul(stride).ok_or(AptError::Overflow)?)?;
        Ok(0..count)
    }
    fn string(self, offset: usize) -> Result<&'a str, AptError> {
        if offset == 0 {
            return Ok("");
        }
        let tail = self
            .0
            .get(offset..)
            .ok_or(AptError::Bounds { offset, size: 1 })?;
        let end = tail
            .iter()
            .position(|&b| b == 0)
            .ok_or(AptError::UnterminatedString(offset))?;
        std::str::from_utf8(&tail[..end]).map_err(|_| AptError::InvalidUtf8(offset))
    }
    fn pointed_string(self, offset: usize) -> Result<&'a str, AptError> {
        self.string(self.offset(offset)?)
    }
    fn matrix(self, o: usize) -> Result<Matrix, AptError> {
        Ok(Matrix {
            a: self.f32(o)?,
            b: self.f32(o + 4)?,
            c: self.f32(o + 8)?,
            d: self.f32(o + 12)?,
            tx: self.f32(o + 16)?,
            ty: self.f32(o + 20)?,
        })
    }
    fn rect(self, o: usize) -> Result<Rect, AptError> {
        Ok(Rect {
            left: self.f32(o)?,
            top: self.f32(o + 4)?,
            right: self.f32(o + 8)?,
            bottom: self.f32(o + 12)?,
        })
    }
}

#[cfg(test)]
mod tests;
