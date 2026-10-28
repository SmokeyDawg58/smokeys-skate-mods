//! `BlendSpace::GetAttributes82d7fc18`: local scratch, native sorted scan.
use super::{
    AndaleRuntimeAttribute, AnimationAttributePool, AnimationNodeHandle, AnimationTreeError,
    AnimationTreePool, TreeAttributeError, append_attribute, multiply_add_attribute,
    multiply_attribute, packed_name,
};
impl AnimationTreePool {
    pub(super) fn get_blend_space_attributes(
        &self,
        handle: &AnimationNodeHandle,
        flags: u8,
        scratch: &mut AnimationAttributePool,
        output: &mut Vec<AndaleRuntimeAttribute>,
    ) -> Result<(), TreeAttributeError> {
        let node = self.blend_space(handle)?;
        let count = node.active_count()?;
        let first = node.active_child(0)?;
        // Native uses stack-local Attributes here, not two PoolStack pushes.
        let mut left = Vec::new();
        self.get_attributes(&first, flags, scratch, &mut left)?;
        for attribute in &mut left {
            multiply_attribute(attribute, self.blend_space_coefficient(handle, 0)?);
        }
        // None represents native mType=255 without inventing a valid payload.
        let mut left: Vec<_> = left.into_iter().map(Some).collect();
        let mut right = Vec::new();
        for index in 1..count {
            right.clear();
            let child = self.blend_space(handle)?.active_child(index)?;
            self.get_attributes(&child, flags, scratch, &mut right)?;
            intersect_volume_attributes(&mut left, &right, || {
                self.blend_space_coefficient(handle, index)
            })?;
        }
        for attribute in left.into_iter().flatten() {
            append_attribute(output, attribute)?;
        }
        Ok(())
    }
    fn blend_space_coefficient(
        &self,
        handle: &AnimationNodeHandle,
        index: usize,
    ) -> Result<f32, TreeAttributeError> {
        self.blend_space(handle)?
            .parameters
            .lock()
            .map_err(|_| AnimationTreeError::Lock)?
            .coefficients
            .get(index)
            .copied()
            .ok_or(TreeAttributeError::Missing("BlendSpace coefficient"))
    }
}
/// Preserve native cursor exhaustion. An empty/right-end read is unknown
/// stack storage in the source, and remains a checked admission error here.
fn intersect_volume_attributes(
    left: &mut [Option<AndaleRuntimeAttribute>],
    right: &[AndaleRuntimeAttribute],
    mut coefficient: impl FnMut() -> Result<f32, TreeAttributeError>,
) -> Result<(), TreeAttributeError> {
    let mut cursor = 0;
    for slot in left {
        let Some(attribute) = slot.as_mut() else {
            continue;
        };
        let name = packed_name(attribute)?;
        let mut candidate = right.get(cursor).ok_or(TreeAttributeError::Missing(
            "BlendSpace unwritten attribute scratch",
        ))?;
        while packed_name(candidate)?.words < name.words {
            cursor += 1;
            if cursor == right.len() {
                // Raw fe40/fe44 compares the old last candidate once more;
                // it remains less, so this slot becomes invalid. Next slot
                // reads end storage and must not silently complete.
                break;
            }
            candidate = &right[cursor];
        }
        if packed_name(candidate)? == name {
            multiply_add_attribute(attribute, candidate, coefficient()?)?;
        } else {
            *slot = None;
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "actor_animation_blend_space_attributes_tests.rs"]
mod tests;
