//! `SetAttributes82d79a00`, Initiate3 82d742c0 and Complete3 82d777a8.
use super::generic2::compute_parameterization2;
use super::jobs::{
    BlendSpaceJobBuffer, BlendSpaceJobError, BlendSpacePool, BlendSpaceRecord, compute_record,
};
use super::{AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool};
use crate::{
    actor_animation_tree::attributes::SettableTreeAttribute,
    input_cvar_runtime::InputCvarReferences,
};
use std::sync::{Arc, Mutex};
pub(crate) struct BlendSpaceAttributeContext<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) queue: Arc<Mutex<BlendSpaceJobBuffer>>,
    pub(crate) pool: BlendSpacePool,
}
impl AnimationTreePool {
    pub(crate) fn set_blend_space_attributes(
        &mut self,
        handle: &AnimationNodeHandle,
        input: &[SettableTreeAttribute],
        context: &mut BlendSpaceAttributeContext<'_>,
    ) -> Result<bool, BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let mut changed = false;
        for attribute in input {
            if attribute.normalized {
                return Err(E::Missing("non-normalized BlendSpace input"));
            }
            let dimensions = self
                .blend_space(handle)
                .map_err(E::Tree)?
                .active_count()
                .map_err(E::Tree)?
                - 1;
            for index in 0..dimensions {
                let node = self.blend_space(handle).map_err(E::Tree)?;
                let name = node
                    .data
                    .as_ref()
                    .and_then(|d| d.volume_sets.get(node.volume_set))
                    .and_then(|s| s.packed_dimension_names.get(index))
                    .ok_or(E::Missing("packed dimension name"))?;
                if *name != attribute.name {
                    continue;
                }
                if !(attribute.value < 1000. && attribute.value > -1000.) {
                    return Err(E::Missing("BlendSpace attribute strict range"));
                }
                let AnimationNodeKind::BlendSpace(node) =
                    &mut self.node_mut(handle).map_err(E::Tree)?.kind
                else {
                    unreachable!()
                };
                *node
                    .attributes
                    .get_mut(index)
                    .ok_or(E::Missing("attribute vector lane"))? = attribute.value;
                changed = true;
            }
        }
        if changed {
            let dimensions = self
                .blend_space(handle)
                .map_err(E::Tree)?
                .active_count()
                .map_err(E::Tree)?
                - 1;
            if dimensions != 3
                || !context
                    .cvars
                    .try_blend_space_optimizations()
                    .map_err(E::Cvar)?
            {
                if dimensions == 2 {
                    self.update_blend_space2(handle, context.cvars)?;
                } else {
                    return Err(E::Tree(AnimationTreeError::PendingBlendSpaceOperation(
                        "non-optimized parameterization",
                    )));
                }
            } else {
                self.initiate_blend_space3(handle, context)?;
                // Native caller samples Jobs again after Initiate; do not cache.
                if !context.cvars.try_blend_space_jobs().map_err(E::Cvar)? {
                    self.complete_blend_space3(handle, context.cvars)?;
                }
            }
        }
        Ok(true)
    }
    /// Two-dimensional generic `UpdateParametrisation82d76a08` executes
    /// synchronously on the retained serialized volumes and current node.
    fn update_blend_space2(
        &mut self,
        handle: &AnimationNodeHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let node = self.blend_space(handle).map_err(E::Tree)?;
        let data = Arc::clone(node.data.as_ref().ok_or(E::Missing("BlendSpace data"))?);
        let set_index = node.volume_set;
        let query: [f32; 3] = node
            .attributes
            .get(..3)
            .and_then(|lanes| lanes.try_into().ok())
            .ok_or(E::Missing("three attribute lanes"))?;
        let parameters = Arc::clone(&node.parameters);
        let set = data
            .volume_sets
            .get(set_index)
            .ok_or(E::Missing("volume set"))?;
        if set.dimensions != 2 {
            return Err(E::Missing("two dimensions"));
        }
        let result =
            compute_parameterization2(query, &set.volumes, || cvars.try_blend_space_slicing_n())
                .map_err(E::Generic2)?;
        if result.needs_projection && cvars.try_blend_space_print().map_err(E::Cvar)? {
            return Err(E::Missing("BlendSpace generic projection debug overlay"));
        }
        {
            let mut output = parameters.lock().map_err(|_| E::Lock)?;
            let state = &mut *output;
            let projected = state
                .projected
                .get_mut(..3)
                .ok_or(E::Missing("three projected lanes"))?;
            let coefficients = state
                .coefficients
                .get_mut(..3)
                .ok_or(E::Missing("three coefficient lanes"))?;
            if result.needs_projection {
                projected.copy_from_slice(&result.projected);
                coefficients.copy_from_slice(&result.coefficients);
            } else {
                coefficients.copy_from_slice(&result.coefficients);
                projected.copy_from_slice(&result.projected);
            }
            state.found = result.selected;
            state.needs_projection = result.needs_projection;
        }
        self.retime_blend_space_selected(handle, &parameters)?;
        if cvars.try_blend_space_print().map_err(E::Cvar)? {
            return Err(E::Missing("BlendSpace generic debug overlay"));
        }
        Ok(())
    }
    fn initiate_blend_space3(
        &mut self,
        handle: &AnimationNodeHandle,
        context: &mut BlendSpaceAttributeContext<'_>,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let node = self.blend_space(handle).map_err(E::Tree)?;
        let data = node.data.as_ref().ok_or(E::Missing("BlendSpace data"))?;
        let set = data
            .volume_sets
            .get(node.volume_set)
            .ok_or(E::Missing("volume set"))?;
        if set.dimensions != 3 {
            return Err(E::Missing("three dimensions"));
        }
        let mut parameters = node.parameters.lock().map_err(|_| E::Lock)?;
        parameters.ready = false;
        parameters.is_data_needed = false;
        parameters.volume_count = set.volumes.len();
        parameters.needs_projection = false;
        parameters.found = None;
        parameters.query = Some(
            node.attributes
                .get(..4)
                .and_then(|v| v.try_into().ok())
                .ok_or(E::Missing("four attribute lanes"))?,
        );
        drop(parameters);
        let record = BlendSpaceRecord {
            pool: context.pool,
            node: handle.clone(),
            parameters: Arc::clone(&node.parameters),
            data: Arc::clone(data),
            volume_set: node.volume_set,
            volume_data_size: set.serialized_size,
        };
        if context.cvars.try_blend_space_jobs().map_err(E::Cvar)? {
            context
                .queue
                .lock()
                .map_err(|_| E::Lock)?
                .register(record)?;
        } else {
            compute_record(&record, context.cvars)?;
        }
        Ok(())
    }
    pub(crate) fn complete_blend_space3(
        &mut self,
        handle: &AnimationNodeHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let parameters = Arc::clone(&self.blend_space(handle).map_err(E::Tree)?.parameters);
        let needs = parameters.lock().map_err(|_| E::Lock)?.needs_projection;
        if needs && cvars.try_blend_space_print().map_err(E::Cvar)? {
            return Err(E::Missing("BlendSpace projection debug overlay"));
        }
        self.retime_blend_space_selected(handle, &parameters)?;
        if cvars.try_blend_space_print().map_err(E::Cvar)? {
            return Err(E::Missing("BlendSpace debug overlay"));
        }
        // Native debugLine +=12 remains explicit debug-overlay accounting debt.
        Ok(())
    }

    /// Shared child retiming after a selected volume changes. Both generic
    /// `UpdateParametrisation82d76a08` and `Complete3 82d777a8` use the
    /// current old child 0 time ratio before publishing the new volume.
    pub(crate) fn retime_blend_space_selected(
        &mut self,
        handle: &AnimationNodeHandle,
        parameters: &Arc<Mutex<super::jobs::BlendSpaceParameters>>,
    ) -> Result<(), BlendSpaceJobError> {
        use BlendSpaceJobError as E;
        let found = parameters.lock().map_err(|_| E::Lock)?.found;
        let count = self
            .blend_space(handle)
            .map_err(E::Tree)?
            .active_count()
            .map_err(E::Tree)?;
        if let Some(found) = found {
            if found != self.blend_space(handle).map_err(E::Tree)?.volume {
                let first = self
                    .blend_space(handle)
                    .map_err(E::Tree)?
                    .active_child(0)
                    .map_err(E::Tree)?;
                // Nonvirtual Clip.GetTimeRatio82d6bbd8 calls both virtuals in order.
                let time = self.time(&first).map_err(E::Tree)?;
                let length = self.length(&first).map_err(E::Tree)?;
                let ratio = time / length;
                for index in 0..count {
                    let found = parameters
                        .lock()
                        .map_err(|_| E::Lock)?
                        .found
                        .ok_or(E::Missing("found volume"))?;
                    let child = self
                        .blend_space(handle)
                        .map_err(E::Tree)?
                        .child_at_volume(found, index)
                        .map_err(E::Tree)?;
                    let time = self.length(&child).map_err(E::Tree)? * ratio;
                    self.set_time(&child, time).map_err(E::Tree)?;
                }
                let found = parameters
                    .lock()
                    .map_err(|_| E::Lock)?
                    .found
                    .ok_or(E::Missing("found volume"))?;
                let AnimationNodeKind::BlendSpace(node) =
                    &mut self.node_mut(handle).map_err(E::Tree)?.kind
                else {
                    unreachable!()
                };
                node.volume = found;
            }
        }
        Ok(())
    }
}
