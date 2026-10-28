//! Generic pointer-field projection through the existing native node search.
use super::{
    AttributePointerLocation, AttributeRuntimeData, AttributeRuntimeDataOrigin,
    AttributeRuntimeError, AttributeRuntimeVault, PointerSlot,
};

#[derive(Clone, Copy)]
pub(super) enum Projection {
    Plain,
    PlainField { offset: usize, size: usize },
    Text,
    TextField { offset: usize },
}
impl AttributeRuntimeVault {
    pub(super) fn project_data(
        &self,
        location: AttributePointerLocation,
        relative: usize,
        size: usize,
        origin: AttributeRuntimeDataOrigin,
        projection: Projection,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        if let Projection::PlainField { offset, size: field_size } = projection {
            let end = offset.checked_add(field_size).ok_or(AttributeRuntimeError::ScalarSize { size })?;
            if end > size { return Err(AttributeRuntimeError::ScalarSize { size }); }
            let relative = relative.checked_add(offset).ok_or(AttributeRuntimeError::ByteRange {
                dependency: location.dependency_index, offset: usize::MAX, size: field_size,
            })?;
            return self.range(location, relative, field_size, origin).map(Some);
        }
        if matches!(projection, Projection::Plain) {
            return self.range(location, relative, size, origin).map(Some);
        }
        let field_offset = if let Projection::TextField { offset } = projection {
            if offset.checked_add(4).is_none_or(|end| end > size) {
                return Err(AttributeRuntimeError::ScalarSize { size });
            }
            offset
        } else if size != 4 {
            return Err(AttributeRuntimeError::ScalarSize { size });
        } else { 0 };
        let offset = usize::try_from(location.offset)
            .ok()
            .and_then(|base| base.checked_add(relative))
            .and_then(|base| base.checked_add(field_offset))
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(AttributeRuntimeError::ByteRange {
                dependency: location.dependency_index,
                offset: usize::MAX,
                size,
            })?;
        self.text_at(AttributePointerLocation { offset, ..location })
    }
    pub(super) fn project_relative(
        &self,
        base: PointerSlot,
        offset: usize,
        size: usize,
        origin: AttributeRuntimeDataOrigin,
        projection: Projection,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        match self.pointer(base)? {
            Some(location) => self.project_data(location, offset, size, origin, projection),
            None if offset == 0 => Ok(None),
            None => Err(AttributeRuntimeError::NullRelativeBase { origin, offset }),
        }
    }
    pub(super) fn text_at(
        &self,
        slot: AttributePointerLocation,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        let start = slot.offset as usize;
        let error = || AttributeRuntimeError::ByteRange {
            dependency: slot.dependency_index,
            offset: start,
            size: 4,
        };
        let bytes = self
            .buffers
            .get(usize::from(slot.dependency_index))
            .and_then(|buffer| buffer.get(start..start.saturating_add(4)))
            .ok_or_else(error)?;
        let lower = AttributePointerLocation {
            offset: slot.offset.saturating_sub(3),
            ..slot
        };
        let upper = AttributePointerLocation {
            offset: slot.offset.saturating_add(3),
            ..slot
        };
        if let Some((&overlap, _)) = self
            .pointers
            .range(lower..=upper)
            .find(|(key, _)| **key != slot)
        {
            return Err(AttributeRuntimeError::RelocatedPayload { slot: overlap });
        }
        // Only this exact pointer slot is interpreted; adjacent fixups remain
        // checked by the existing range reader for the resulting text.
        let Some(text) = self.pointer(PointerSlot {
            location: slot,
            raw_word: u32::from_be_bytes(bytes.try_into().expect("four-byte pointer")),
        })?
        else {
            return Ok(None);
        };
        let start = text.offset as usize;
        let remaining = self
            .buffers
            .get(usize::from(text.dependency_index))
            .and_then(|buffer| buffer.get(start..))
            .ok_or(AttributeRuntimeError::ByteRange {
                dependency: text.dependency_index,
                offset: start,
                size: 1,
            })?;
        let len = remaining.iter().position(|byte| *byte == 0).ok_or(
            AttributeRuntimeError::UnterminatedText {
                dependency: text.dependency_index,
                offset: start,
            },
        )?;
        let mut result = self.range(text, 0, len + 1, AttributeRuntimeDataOrigin::StoredPointer)?;
        result.range.end -= 1;
        Ok(Some(result))
    }
}
