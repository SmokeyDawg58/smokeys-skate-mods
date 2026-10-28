//! Retained `PoseBufferExt<SQTExt>` allocation and actual `ExtractSQT` publication.
use crate::actor_face_playback::ActorFaceError;
use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
use std::ops::Range;
use std::sync::{Arc, Mutex};

#[path = "actor_animation_sqt_output_mutation.rs"]
pub(crate) mod mutation;
#[path = "actor_animation_sqt_output_ranges.rs"]
mod ranges;

#[derive(Clone)]
pub(crate) struct OwnedAnimationSqtBuffer {
    storage: Arc<Mutex<SqtOutputAllocation>>,
    range: Range<usize>,
    hierarchy: Option<Arc<AndaleHierarchy>>,
}
struct SqtOutputAllocation {
    values: Vec<Option<AndaleSqt>>,
    generation: u64,
    synchronized: bool,
    extraction_pending: bool,
}
impl OwnedAnimationSqtBuffer {
    pub(crate) fn new() -> Self {
        Self::allocate_unknown(0)
    }
    pub(crate) fn allocate_unknown(bones: usize) -> Self {
        Self {
            storage: Arc::new(Mutex::new(SqtOutputAllocation {
                values: vec![None; bones],
                generation: 0,
                synchronized: false,
                extraction_pending: false,
            })),
            range: 0..bones,
            hierarchy: None,
        }
    }
    /// Host admission identity; no data read or physical sampling occurs.
    pub(crate) fn is_same_view(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storage, &other.storage) && self.range == other.range
    }
    pub(crate) fn len(&self) -> Result<usize, ActorFaceError> {
        Ok(self
            .storage
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .values
            .get(self.range.clone())
            .ok_or(ActorFaceError::UnsupportedPose)?
            .len())
    }
    pub(crate) fn is_empty(&self) -> Result<bool, ActorFaceError> {
        Ok(self.len()? == 0)
    }
    pub(crate) fn is_unwritten(&self) -> Result<bool, ActorFaceError> {
        Ok(self
            .storage
            .lock()
            .map_err(|_| ActorFaceError::Lock)?
            .values
            .get(self.range.clone())
            .ok_or(ActorFaceError::UnsupportedPose)?
            .iter()
            .all(Option::is_none))
    }
    pub(crate) fn begin_extract(&self) -> Result<(), ActorFaceError> {
        let mut storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if storage.extraction_pending {
            return Err(ActorFaceError::UnsupportedPose);
        }
        storage.extraction_pending = true;
        storage.synchronized = false;
        Ok(())
    }
    /// AnimMan's callback2 intentionally names the same ExtractSQTs scratch
    /// allocation for several character streams within one command set.
    pub(crate) fn begin_shared_extract(&self) -> Result<(), ActorFaceError> {
        let mut storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        storage.extraction_pending = true;
        storage.synchronized = false;
        Ok(())
    }
    /// Called by the producing fetch/blend job or native fixed-copy scheduling,
    /// never at command emission.
    pub(crate) fn extract(&self, values: &[AndaleSqt]) -> Result<(), ActorFaceError> {
        let mut storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        let destination = storage
            .values
            .get_mut(self.range.clone())
            .ok_or(ActorFaceError::UnsupportedPose)?;
        if values.len() != destination.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        for (destination, value) in destination.iter_mut().zip(values) {
            *destination = Some(*value);
        }
        storage.generation = storage
            .generation
            .checked_add(1)
            .ok_or(ActorFaceError::UnsupportedTiming)?;
        Ok(())
    }
    pub(crate) fn complete_sync(&self) -> Result<(), ActorFaceError> {
        let mut storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if storage.values.get(self.range.clone()).ok_or(ActorFaceError::UnsupportedPose)?
            .iter().any(Option::is_none) {
            return Err(ActorFaceError::UnsupportedPose);
        }
        storage.extraction_pending = false;
        storage.synchronized = true;
        Ok(())
    }
    pub(crate) fn read(&self) -> Result<(u64, Vec<AndaleSqt>), ActorFaceError> {
        let storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
        if !storage.synchronized {
            return Err(ActorFaceError::UnsupportedPose);
        }
        let values = storage
            .values
            .get(self.range.clone())
            .ok_or(ActorFaceError::UnsupportedPose)?
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or(ActorFaceError::UnsupportedPose)?;
        Ok((storage.generation, values))
    }
}

#[cfg(test)]
#[path = "actor_animation_sqt_output_tests.rs"]
mod tests;
