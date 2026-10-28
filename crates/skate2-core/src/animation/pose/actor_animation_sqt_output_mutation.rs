//! Synchronous channel edits over retained SQT allocations, including aliasing.
use super::{ActorFaceError, OwnedAnimationSqtBuffer, SqtOutputAllocation};
use skate2_formats::andale::AndaleSqt;
use std::{ops::Range, sync::Arc};

#[derive(Debug)]
pub(crate) enum SqtMutationError<E> {
    Owner(ActorFaceError),
    Operation(E),
}
impl<E: std::fmt::Display> std::fmt::Display for SqtMutationError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "SQT edit owner: {e:?}"),
            Self::Operation(e) => e.fmt(f),
        }
    }
}
/// Reads resolve against the same mutable storage when source and destination
/// alias; this preserves native read-after-write behavior without pose copies.
pub(crate) struct SqtChannelEdit<'a> {
    destination: &'a mut SqtOutputAllocation,
    source: Option<&'a SqtOutputAllocation>,
    source_range: Range<usize>,
    destination_range: Range<usize>,
    written: bool,
}
impl SqtChannelEdit<'_> {
    pub(crate) fn source(&self, index: usize) -> Result<AndaleSqt, ActorFaceError> {
        let storage = self.source.unwrap_or(self.destination);
        storage
            .values
            .get(self.source_range.clone())
            .and_then(|v| v.get(index))
            .copied()
            .flatten()
            .ok_or(ActorFaceError::UnsupportedPose)
    }
    pub(crate) fn destination(&self, index: usize) -> Result<AndaleSqt, ActorFaceError> {
        self.destination
            .values
            .get(self.destination_range.clone())
            .and_then(|v| v.get(index))
            .copied()
            .flatten()
            .ok_or(ActorFaceError::UnsupportedPose)
    }
    fn write(&mut self, index: usize, value: AndaleSqt) -> Result<(), ActorFaceError> {
        if index >= self.destination_range.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        if !self.written {
            self.destination.generation = self
                .destination
                .generation
                .checked_add(1)
                .ok_or(ActorFaceError::UnsupportedTiming)?;
            self.destination.synchronized = false;
            self.written = true;
        }
        self.destination.values[self.destination_range.start + index] = Some(value);
        Ok(())
    }
    pub(crate) fn set_scale(
        &mut self,
        index: usize,
        scale: [f32; 4],
    ) -> Result<(), ActorFaceError> {
        let mut value = self.destination(index)?;
        value.scale = scale;
        self.write(index, value)
    }
    pub(crate) fn set_rotation(
        &mut self,
        index: usize,
        rotation: [f32; 4],
    ) -> Result<(), ActorFaceError> {
        let mut value = self.destination(index)?;
        value.rotation = rotation;
        self.write(index, value)
    }
    pub(crate) fn set_translation(
        &mut self,
        index: usize,
        translation: [f32; 4],
    ) -> Result<(), ActorFaceError> {
        let mut value = self.destination(index)?;
        value.translation = translation;
        self.write(index, value)
    }
}
fn require(storage: &SqtOutputAllocation) -> Result<(), ActorFaceError> {
    if storage.synchronized && !storage.extraction_pending {
        Ok(())
    } else {
        Err(ActorFaceError::UnsupportedPose)
    }
}
impl OwnedAnimationSqtBuffer {
    /// Native `SetData` reads an external pointer at job execution. An extraction
    /// into that same known allocation may already be queued; this is not a
    /// public synchronized-pose read and does not complete that extraction.
    pub(crate) fn try_read_for_set_job(&self) -> Result<Vec<AndaleSqt>, ActorFaceError> {
        let storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if !storage.synchronized && !storage.extraction_pending {
            return Err(ActorFaceError::UnsupportedPose);
        }
        storage
            .values
            .get(self.range.clone())
            .ok_or(ActorFaceError::UnsupportedPose)?
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or(ActorFaceError::UnsupportedPose)
    }
    pub(crate) fn try_edit_channels<T, E>(
        &self,
        source: &Self,
        edit: impl FnOnce(&mut SqtChannelEdit<'_>) -> Result<T, E>,
    ) -> Result<T, SqtMutationError<E>> {
        use SqtMutationError::Owner;
        if Arc::ptr_eq(&self.storage, &source.storage) {
            let mut destination = self
                .storage
                .lock()
                .map_err(|_| Owner(ActorFaceError::Lock))?;
            return run_edit(
                &mut destination,
                None,
                self.range.clone(),
                source.range.clone(),
                edit,
            );
        }
        if Arc::as_ptr(&source.storage) < Arc::as_ptr(&self.storage) {
            let source_storage = source
                .storage
                .lock()
                .map_err(|_| Owner(ActorFaceError::Lock))?;
            let mut destination = self
                .storage
                .lock()
                .map_err(|_| Owner(ActorFaceError::Lock))?;
            run_edit(
                &mut destination,
                Some(&source_storage),
                self.range.clone(),
                source.range.clone(),
                edit,
            )
        } else {
            let mut destination = self
                .storage
                .lock()
                .map_err(|_| Owner(ActorFaceError::Lock))?;
            let source_storage = source
                .storage
                .lock()
                .map_err(|_| Owner(ActorFaceError::Lock))?;
            run_edit(
                &mut destination,
                Some(&source_storage),
                self.range.clone(),
                source.range.clone(),
                edit,
            )
        }
    }
}
fn run_edit<T, E>(
    destination: &mut SqtOutputAllocation,
    source: Option<&SqtOutputAllocation>,
    destination_range: Range<usize>,
    source_range: Range<usize>,
    edit: impl FnOnce(&mut SqtChannelEdit<'_>) -> Result<T, E>,
) -> Result<T, SqtMutationError<E>> {
    require(destination).map_err(SqtMutationError::Owner)?;
    if let Some(source) = source {
        require(source).map_err(SqtMutationError::Owner)?;
    }
    let mut access = SqtChannelEdit {
        destination,
        source,
        source_range,
        destination_range,
        written: false,
    };
    let result = edit(&mut access).map_err(SqtMutationError::Operation);
    if result.is_ok() && access.written {
        access.destination.synchronized = true;
    }
    result
}
#[cfg(test)]
#[expect(
    clippy::float_cmp,
    reason = "exact copied fixture values prove alias visibility and retained stores"
)]
mod tests {
    use super::*;
    #[test]
    fn later_failure_retains_store_without_publishing_success() {
        let buffer = OwnedAnimationSqtBuffer::allocate_unknown(1);
        let value = AndaleSqt {
            scale: [1.0; 4],
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [0.0; 4],
        };
        buffer.extract(&[value]).unwrap();
        buffer.complete_sync().unwrap();
        let result = buffer.try_edit_channels(&buffer, |edit| {
            edit.set_translation(0, [9.0; 4]).unwrap();
            assert_eq!(edit.source(0).unwrap().translation, [9.0; 4]);
            Err::<(), _>("later native service failed")
        });
        assert!(matches!(result, Err(SqtMutationError::Operation(_))));
        assert!(buffer.read().is_err());
        assert_eq!(
            buffer.storage.lock().unwrap().values[0]
                .unwrap()
                .translation,
            [9.0; 4]
        );
    }
}
