//! Draft child of `actor_animation_sqt_output`: same-allocation extraction views
//! and native part-wise `PoseBufferExt<SQTExt>::OverrideWith825920b0`.
//! Requires the explicitly documented parent handle range/hierarchy amendment.
use super::{ActorFaceError, Arc, OwnedAnimationSqtBuffer, SqtOutputAllocation};
use skate2_formats::andale::{AndaleHierarchy, AndaleHierarchyPart};
use std::ops::Range;

#[cfg(test)]
#[path = "actor_animation_sqt_output_ranges_tests.rs"]
mod tests;

impl OwnedAnimationSqtBuffer {
    /// Real descriptor retained from the existing playback database. No default
    /// hierarchy is manufactured for anonymous legacy/test allocations.
    pub(crate) fn allocate_for_hierarchy(hierarchy: Arc<AndaleHierarchy>) -> Self {
        let mut output = Self::allocate_unknown(hierarchy.parents.len());
        output.hierarchy = Some(hierarchy);
        output
    }

    /// Pointer-plus-length extraction view; source Restore passes a full pose
    /// pointer plus 48*faceOffset. This does not construct a new subhierarchy.
    pub(crate) fn try_subrange(&self, relative: Range<usize>) -> Result<Self, ActorFaceError> {
        let range = checked_range(&self.range, relative)?;
        Ok(Self {
            storage: Arc::clone(&self.storage),
            range,
            hierarchy: None,
        })
    }

    /// Bind a view only to an actual supplied descriptor of the matching size.
    /// Needed when a native pose-buffer object has its own hierarchy but points
    /// into an existing allocation; raw `ExtractSQT` views need no descriptor.
    pub(crate) fn try_subrange_for_hierarchy(
        &self,
        relative: Range<usize>,
        hierarchy: Arc<AndaleHierarchy>,
    ) -> Result<Self, ActorFaceError> {
        let mut output = self.try_subrange(relative)?;
        if output.range.len() != hierarchy.parents.len() {
            return Err(ActorFaceError::UnsupportedPose);
        }
        output.hierarchy = Some(hierarchy);
        Ok(output)
    }

    /// `OverrideWith825920b0`: iterate SOURCE parts, first matching destination
    /// packed name, then copy the SOURCE count. It is not a full-pose memcpy.
    pub(crate) fn try_override_from(&self, source: &Self) -> Result<(), ActorFaceError> {
        let source_hierarchy = source
            .hierarchy
            .as_ref()
            .ok_or(ActorFaceError::UnsupportedPose)?;
        let destination_hierarchy = self
            .hierarchy
            .as_ref()
            .ok_or(ActorFaceError::UnsupportedPose)?;
        if Arc::ptr_eq(&self.storage, &source.storage) {
            let mut storage = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
            require_synchronized(&storage)?;
            require_idle(&storage)?;
            let mut copied = false;
            for source_part in &source_hierarchy.parts {
                if let Some(destination_part) = matching_part(destination_hierarchy, source_part) {
                    let from = part_range(
                        &source.range,
                        source_part.bone_offset,
                        source_part.bone_count,
                    )?;
                    let to = part_range(
                        &self.range,
                        destination_part.bone_offset,
                        source_part.bone_count,
                    )?;
                    for (source_index, destination_index) in from.zip(to) {
                        // Native memcpy is forward for SQT-aligned ranges, not
                        // memmove. A preceding write may change this source.
                        let value = storage.values[source_index];
                        begin_copy(&mut storage, &mut copied)?;
                        storage.values[destination_index] = value;
                    }
                }
            }
            finish_copy(&mut storage, copied);
            return Ok(());
        }
        // Stable host allocation ordering avoids AB/BA mutex deadlock. This
        // changes no native data sampling in the selected exclusive caller.
        if Arc::as_ptr(&source.storage) < Arc::as_ptr(&self.storage) {
            let from = source.storage.lock().map_err(|_| ActorFaceError::Lock)?;
            let mut to = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
            copy_between(
                source,
                self,
                source_hierarchy,
                destination_hierarchy,
                &from,
                &mut to,
            )
        } else {
            let mut to = self.storage.lock().map_err(|_| ActorFaceError::Lock)?;
            let from = source.storage.lock().map_err(|_| ActorFaceError::Lock)?;
            copy_between(
                source,
                self,
                source_hierarchy,
                destination_hierarchy,
                &from,
                &mut to,
            )
        }
    }
}

fn copy_between(
    source: &OwnedAnimationSqtBuffer,
    destination: &OwnedAnimationSqtBuffer,
    source_hierarchy: &AndaleHierarchy,
    destination_hierarchy: &AndaleHierarchy,
    from: &SqtOutputAllocation,
    to: &mut SqtOutputAllocation,
) -> Result<(), ActorFaceError> {
    require_synchronized(from)?;
    require_idle(from)?;
    require_idle(to)?;
    let mut copied = false;
    for source_part in &source_hierarchy.parts {
        if let Some(destination_part) = matching_part(destination_hierarchy, source_part) {
            let source_range = part_range(
                &source.range,
                source_part.bone_offset,
                source_part.bone_count,
            )?;
            let destination_range = part_range(
                &destination.range,
                destination_part.bone_offset,
                source_part.bone_count,
            )?;
            for (source_index, destination_index) in source_range.zip(destination_range) {
                begin_copy(to, &mut copied)?;
                to.values[destination_index] = from.values[source_index];
            }
        }
    }
    finish_copy(to, copied);
    Ok(())
}
fn matching_part<'a>(
    hierarchy: &'a AndaleHierarchy,
    source: &AndaleHierarchyPart,
) -> Option<&'a AndaleHierarchyPart> {
    // GetPartIndex825910a0 compares all six words and returns first match.
    hierarchy
        .parts
        .iter()
        .find(|part| part.packed_name == source.packed_name)
}
fn require_synchronized(storage: &SqtOutputAllocation) -> Result<(), ActorFaceError> {
    if storage.synchronized {
        Ok(())
    } else {
        Err(ActorFaceError::UnsupportedPose)
    }
}
fn require_idle(storage: &SqtOutputAllocation) -> Result<(), ActorFaceError> {
    if storage.extraction_pending {
        Err(ActorFaceError::UnsupportedPose)
    } else {
        Ok(())
    }
}
fn begin_copy(storage: &mut SqtOutputAllocation, copied: &mut bool) -> Result<(), ActorFaceError> {
    if !*copied {
        let next = storage
            .generation
            .checked_add(1)
            .ok_or(ActorFaceError::UnsupportedTiming)?;
        storage.synchronized = false;
        storage.generation = next;
        *copied = true;
    }
    Ok(())
}
fn finish_copy(storage: &mut SqtOutputAllocation, copied: bool) {
    if copied {
        // Synchronous source memcpy needs no job-manager completion. Unknown
        // untouched storage remains unknown and cannot become a readable pose.
        storage.synchronized = storage.values.iter().all(Option::is_some);
    }
}
fn checked_range(
    parent: &Range<usize>,
    relative: Range<usize>,
) -> Result<Range<usize>, ActorFaceError> {
    if relative.start > relative.end || relative.end > parent.len() {
        return Err(ActorFaceError::UnsupportedPose);
    }
    let start = parent
        .start
        .checked_add(relative.start)
        .ok_or(ActorFaceError::UnsupportedPose)?;
    let end = parent
        .start
        .checked_add(relative.end)
        .ok_or(ActorFaceError::UnsupportedPose)?;
    Ok(start..end)
}
fn part_range(
    parent: &Range<usize>,
    offset: u32,
    count: u32,
) -> Result<Range<usize>, ActorFaceError> {
    let start = usize::try_from(offset).map_err(|_| ActorFaceError::UnsupportedPose)?;
    let count = usize::try_from(count).map_err(|_| ActorFaceError::UnsupportedPose)?;
    let end = start
        .checked_add(count)
        .ok_or(ActorFaceError::UnsupportedPose)?;
    checked_range(parent, start..end)
}
