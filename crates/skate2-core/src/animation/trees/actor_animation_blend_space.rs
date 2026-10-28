//! Retained `BlendSpace` constructor and timing, 82d71c30/82d6bf98..d068.
use super::time::AnimationProperties;
use super::{
    AnimationClipRef, AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool,
};
use skate2_formats::andale::AndaleBlendSpace;
use std::sync::{Arc, Mutex};
#[path = "actor_animation_blend_space_generic2.rs"]
pub(crate) mod generic2;
#[path = "actor_animation_blend_space_jobs.rs"]
pub(crate) mod jobs;
#[path = "actor_animation_blend_space_update.rs"]
pub(crate) mod update;

pub(crate) struct BlendSpaceNode {
    pub(crate) header: Option<AnimationClipRef>,
    pub(crate) data: Option<Arc<AndaleBlendSpace>>,
    pub(crate) volume_set: usize,
    pub(crate) volume: usize,
    pub(crate) time: f32,
    pub(crate) trimmed: bool,
    pub(crate) attributes: Vec<f32>,
    pub(crate) parameters: Arc<Mutex<jobs::BlendSpaceParameters>>,
    // None outer means uninitialized native allocation; inner None is SetClip(NULL).
    #[expect(
        clippy::option_option,
        reason = "unwritten native slot is distinct from explicitly stored null child"
    )]
    pub(crate) children: Vec<Option<Option<AnimationNodeHandle>>>,
    pub(crate) properties: Vec<AnimationProperties>,
    pub(crate) vector_location: usize,
}
impl BlendSpaceNode {
    fn new() -> Self {
        Self {
            header: None,
            data: None,
            volume_set: 0,
            volume: 0,
            time: 0.0,
            trimmed: false,
            attributes: vec![],
            parameters: Arc::new(Mutex::new(jobs::BlendSpaceParameters::uninitiated())),
            children: vec![],
            properties: vec![],
            vector_location: 0,
        }
    }
    pub(crate) fn active_count(&self) -> Result<usize, AnimationTreeError> {
        let data = self
            .data
            .as_ref()
            .ok_or(AnimationTreeError::MissingRecord)?;
        usize::try_from(
            data.volume_sets
                .get(self.volume_set)
                .ok_or(AnimationTreeError::MissingRecord)?
                .dimensions,
        )
        .ok()
        .and_then(|v| v.checked_add(1))
        .ok_or(AnimationTreeError::MissingRecord)
    }
    pub(crate) fn active_child(
        &self,
        index: usize,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.child_at_volume(self.volume, index)
    }
    pub(crate) fn child_at_volume(
        &self,
        volume_index: usize,
        index: usize,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        let data = self
            .data
            .as_ref()
            .ok_or(AnimationTreeError::MissingRecord)?;
        let volume = data
            .volume_sets
            .get(self.volume_set)
            .and_then(|set| set.volumes.get(volume_index))
            .ok_or(AnimationTreeError::MissingRecord)?;
        let offset = usize::try_from(volume.animation_indexes_offset)
            .ok()
            .and_then(|v| index.checked_mul(4).and_then(|i| v.checked_add(i)))
            .ok_or(AnimationTreeError::MissingRecord)?;
        let end = offset
            .checked_add(4)
            .ok_or(AnimationTreeError::MissingRecord)?;
        let bytes = volume
            .serialized_data
            .get(offset..end)
            .ok_or(AnimationTreeError::MissingRecord)?;
        let child = usize::try_from(u32::from_be_bytes(bytes.try_into().expect("four bytes")))
            .map_err(|_| AnimationTreeError::MissingRecord)?;
        self.children
            .get(child)
            .and_then(Option::as_ref)
            .and_then(Option::as_ref)
            .cloned()
            .ok_or(AnimationTreeError::MissingChild)
    }
}
impl AnimationTreePool {
    pub(crate) fn push_blend_space(&mut self) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.push(112, AnimationNodeKind::BlendSpace(BlendSpaceNode::new()))
    }
    pub(crate) fn initialize_blend_space(
        &mut self,
        handle: &AnimationNodeHandle,
        header: AnimationClipRef,
        data: &Arc<AndaleBlendSpace>,
    ) -> Result<(), AnimationTreeError> {
        let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        node.header = Some(header);
        node.data = Some(Arc::clone(data));
        node.volume_set = 0;
        node.volume = 0;
        let dimension = usize::try_from(
            data.volume_sets
                .first()
                .ok_or(AnimationTreeError::MissingRecord)?
                .dimensions,
        )
        .map_err(|_| AnimationTreeError::MissingRecord)?;
        let size = usize::try_from(data.maximum_dimensions)
            .ok()
            .and_then(|v| v.checked_add(1))
            .ok_or(AnimationTreeError::Allocation)?;
        let vector_bytes = size
            .checked_add(1)
            .and_then(|v| v.checked_mul(4))
            .ok_or(AnimationTreeError::Allocation)?;
        node.vector_location = 0;
        allocate_vector(
            &mut node.attributes,
            size,
            vector_bytes,
            &mut node.vector_location,
        )?;
        *node
            .attributes
            .get_mut(dimension)
            .ok_or(AnimationTreeError::MissingRecord)? = 1.0;
        let mut parameters = node
            .parameters
            .lock()
            .map_err(|_| AnimationTreeError::Lock)?;
        allocate_vector(
            &mut parameters.projected,
            size,
            vector_bytes,
            &mut node.vector_location,
        )?;
        *parameters
            .projected
            .get_mut(dimension)
            .ok_or(AnimationTreeError::MissingRecord)? = 1.0;
        allocate_vector(
            &mut parameters.coefficients,
            size,
            vector_bytes,
            &mut node.vector_location,
        )?;
        *parameters
            .coefficients
            .first_mut()
            .ok_or(AnimationTreeError::MissingRecord)? = 1.0;
        node.children
            .try_reserve_exact(data.animation_names.len())
            .map_err(|_| AnimationTreeError::Allocation)?;
        node.children.resize(data.animation_names.len(), None);
        node.properties
            .try_reserve_exact(dimension + 1)
            .map_err(|_| AnimationTreeError::Allocation)?;
        node.properties
            .resize(dimension + 1, AnimationProperties::default());
        Ok(())
    }
    /// `SetClip82d6cc70` stores a null result too; no Clip-type assertion.
    pub(crate) fn set_blend_space_child(
        &mut self,
        handle: &AnimationNodeHandle,
        index: usize,
        child: Option<AnimationNodeHandle>,
    ) -> Result<(), AnimationTreeError> {
        let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        *node
            .children
            .get_mut(index)
            .ok_or(AnimationTreeError::MissingChild)? = Some(child);
        Ok(())
    }
    pub(crate) fn blend_space(
        &self,
        handle: &AnimationNodeHandle,
    ) -> Result<&BlendSpaceNode, AnimationTreeError> {
        let AnimationNodeKind::BlendSpace(node) = &self.node(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        Ok(node)
    }
    /// `GetLength82d6cce8`: child length before coefficient, fused ascending sum.
    pub(crate) fn blend_space_length(
        &self,
        handle: &AnimationNodeHandle,
    ) -> Result<f32, AnimationTreeError> {
        let mut sum = 0.0;
        for i in 0..self.blend_space(handle)?.active_count()? {
            let child = self.blend_space(handle)?.active_child(i)?;
            let length = self.length(&child)?;
            let coefficient = *self
                .blend_space(handle)?
                .parameters
                .lock()
                .map_err(|_| AnimationTreeError::Lock)?
                .coefficients
                .get(i)
                .ok_or(AnimationTreeError::MissingRecord)?;
            sum = length.mul_add(coefficient, sum);
        }
        Ok(sum)
    }
    /// `SetSpeed82d6d020`: all declared children, no parent speed assertion.
    pub(crate) fn set_blend_space_speed(
        &mut self,
        handle: &AnimationNodeHandle,
        speed: f32,
    ) -> Result<(), AnimationTreeError> {
        let count = self.blend_space(handle)?.children.len();
        for i in 0..count {
            let child = self.blend_space(handle)?.children[i]
                .as_ref()
                .and_then(Option::as_ref)
                .cloned()
                .ok_or(AnimationTreeError::MissingChild)?;
            self.set_speed(&child, speed)?;
        }
        Ok(())
    }
    /// `SetTime82d6cf88`: native division and child order, no parent time store.
    pub(crate) fn set_blend_space_time(
        &mut self,
        handle: &AnimationNodeHandle,
        time: f32,
    ) -> Result<(), AnimationTreeError> {
        let count = self.blend_space(handle)?.active_count()?;
        let ratio = time / self.length(handle)?;
        for i in 0..count {
            let child = self.blend_space(handle)?.active_child(i)?;
            let child_time = self.length(&child)? * ratio;
            self.set_time(&child, child_time)?;
        }
        Ok(())
    }
}

fn allocate_vector(
    output: &mut Vec<f32>,
    size: usize,
    bytes: usize,
    location: &mut usize,
) -> Result<(), AnimationTreeError> {
    output
        .try_reserve_exact(size)
        .map_err(|_| AnimationTreeError::Allocation)?;
    output.resize(size, 0.0);
    *location = location
        .checked_add(bytes)
        .ok_or(AnimationTreeError::Allocation)?;
    Ok(())
}
impl AnimationTreePool {
    /// `AddTime82d6cd70`: reset property ZERO on every length iteration, as raw ce48..74.
    pub(crate) fn add_blend_space_time(
        &mut self,
        handle: &AnimationNodeHandle,
        dt: f32,
        output: &mut AnimationProperties,
    ) -> Result<(), AnimationTreeError> {
        let count = self.blend_space(handle)?.active_count()?;
        let bytes = (count + 1)
            .checked_mul(4)
            .ok_or(AnimationTreeError::Allocation)?;
        let mut lengths = Vec::new();
        lengths
            .try_reserve_exact(count)
            .map_err(|_| AnimationTreeError::Allocation)?;
        lengths.resize(count, 0.0);
        let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
            unreachable!()
        };
        node.vector_location = node
            .vector_location
            .checked_add(bytes)
            .ok_or(AnimationTreeError::Allocation)?;
        let mut sum = 0.0;
        for (i, length) in lengths.iter_mut().enumerate() {
            let child = self.blend_space(handle)?.active_child(i)?;
            *length = self.length(&child)?;
            let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
                unreachable!()
            };
            let coefficient = *node
                .parameters
                .lock()
                .map_err(|_| AnimationTreeError::Lock)?
                .coefficients
                .get(i)
                .ok_or(AnimationTreeError::MissingRecord)?;
            *node
                .properties
                .first_mut()
                .ok_or(AnimationTreeError::MissingRecord)? = AnimationProperties::default();
            sum = coefficient.mul_add(*length, sum);
        }
        let inverse = 1.0 / sum;
        for (i, length) in lengths.iter().enumerate() {
            let child = self.blend_space(handle)?.active_child(i)?;
            let mut property = *self
                .blend_space(handle)?
                .properties
                .get(i)
                .ok_or(AnimationTreeError::MissingRecord)?;
            let result = self.add_time(&child, (inverse * *length) * dt, &mut property);
            let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
                unreachable!()
            };
            node.properties[i] = property;
            result?;
        }
        let scale = sum / lengths[0];
        let first = self.blend_space(handle)?.properties[0];
        output.expired_or_looped = first.expired_or_looped;
        output.time_to_expire = first.time_to_expire * scale;
        output.expired_by = first.expired_by * scale;
        let child = self.blend_space(handle)?.active_child(0)?;
        let time = self.time(&child)?;
        let AnimationNodeKind::BlendSpace(node) = &mut self.node_mut(handle)?.kind else {
            unreachable!()
        };
        node.time = time * scale;
        // Debug overlay global debugLine=5 at82d6cf60 is outside this simulation projection.
        node.vector_location -= bytes;
        Ok(())
    }
}

#[cfg(test)]
#[path = "actor_animation_blend_space_tests.rs"]
mod tests;

#[path = "actor_animation_blend_space_parameterization.rs"]
pub(crate) mod parameterization;
