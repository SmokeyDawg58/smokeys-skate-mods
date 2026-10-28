//! Live Andale attributes from retained nodes, before Eval advances old time.
use super::{
    AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool, ClipNode,
};
use crate::{play_animation::FastString30, simulation_animation_startup::AnimationAttributePool};
use skate2_formats::andale::{AndaleAttribute, AndaleAttributeValue, AndaleRuntimeAttribute};

use super::blend_space::{jobs::BlendSpaceJobError, update::BlendSpaceAttributeContext};

#[derive(Debug)]
pub(crate) enum TreeAttributeError {
    Tree(AnimationTreeError),
    BlendSpace(BlendSpaceJobError),
    Missing(&'static str),
    Capacity,
    InvalidName,
    InvalidLoop,
    TypeMismatch,
}
impl From<AnimationTreeError> for TreeAttributeError {
    fn from(error: AnimationTreeError) -> Self {
        Self::Tree(error)
    }
}
impl std::fmt::Display for TreeAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tree(error) => write!(f, "tree attributes: {error}"),
            Self::BlendSpace(error) => error.fmt(f),
            Self::Missing(owner) => write!(f, "tree attributes require {owner}"),
            other => write!(f, "tree attributes: {other:?}"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct SettableTreeAttribute {
    pub(crate) sequence_id: i32,
    pub(crate) name: FastString30,
    pub(crate) value: f32,
    pub(crate) normalized: bool,
}

/// `PoolStack<Attributes>::Push82d79cc0`; strict byte end and actual shared count.
pub(crate) fn push_scratch(pool: &mut AnimationAttributePool) -> Result<(), TreeAttributeError> {
    if pool.element_count >= pool.elements.len()
        || (pool.element_count + 1) * 7712 >= pool.memory.len()
    {
        return Err(TreeAttributeError::Capacity);
    }
    pool.element_count += 1;
    Ok(())
}
pub(crate) fn append_attribute(
    output: &mut Vec<AndaleRuntimeAttribute>,
    attribute: AndaleRuntimeAttribute,
) -> Result<(), TreeAttributeError> {
    if output.len() >= 120 {
        return Err(TreeAttributeError::Capacity);
    }
    output.push(attribute);
    Ok(())
}
pub(crate) fn packed_name(
    attribute: &AndaleRuntimeAttribute,
) -> Result<FastString30, TreeAttributeError> {
    FastString30::encode(&attribute.name).map_err(|_| TreeAttributeError::InvalidName)
}

impl AnimationTreePool {
    #[cfg(test)]
    pub(crate) fn set_attributes(
        &mut self,
        node: &AnimationNodeHandle,
        input: &[SettableTreeAttribute],
    ) -> Result<bool, TreeAttributeError> {
        self.set_attributes_with_blend_space(node, input, &mut None)
    }

    /// `Clip82d77c98` / `PhaseBlend82d76868`: single-attribute query uses stack locals.
    #[expect(
        clippy::too_many_lines,
        reason = "source ordered node dispatch and attribute filtering"
    )]
    pub(crate) fn get_attribute(
        &self,
        node: &AnimationNodeHandle,
        name: FastString30,
        flags: u8,
    ) -> Result<Option<AndaleRuntimeAttribute>, TreeAttributeError> {
        use AnimationNodeKind as N;
        match &self.node(node)?.kind {
            N::Sequence(_) => self.get_attribute(&self.selected_child(node)?, name, flags),
            N::Selector(n) => {
                let selected = self.selected_child(node)?;
                let child = if n.transitioning {
                    n.children
                        .get(n.next.ok_or(TreeAttributeError::Missing("Selector next"))?)
                        .ok_or(TreeAttributeError::Missing("Selector next"))?
                        .0
                        .clone()
                } else {
                    selected
                };
                self.get_attribute(&child, name, flags)
            }
            N::BlendSpace(_) => {
                Err(AnimationTreeError::PendingBlendSpaceOperation("GetAttribute").into())
            }
            N::Pose { .. } | N::FixedSqt { .. } => Ok(None),
            N::Clip(clip) => {
                let header = clip
                    .header
                    .as_ref()
                    .ok_or(TreeAttributeError::Missing("Clip header"))?;
                let database = header
                    .database
                    .lock()
                    .map_err(|_| AnimationTreeError::Lock)?;
                let source = database
                    .clip_by_record(header.record_index)
                    .ok_or(AnimationTreeError::MissingRecord)?;
                for attribute in &source.attributes {
                    if FastString30::encode(&attribute.name)
                        .map_err(|_| TreeAttributeError::InvalidName)?
                        != name
                    {
                        continue;
                    }
                    let status = clip_attribute_status(clip, attribute)?;
                    if flags & 0x1c & status == 0 || flags & 3 & status == 0 {
                        continue;
                    }
                    if matches!(attribute.value, AndaleAttributeValue::Unsupported { .. }) {
                        return Err(TreeAttributeError::TypeMismatch);
                    }
                    return Ok(Some(AndaleRuntimeAttribute {
                        sequence_id: -1,
                        name: attribute.name.clone(),
                        status,
                        value: attribute.value.clone(),
                        begin_seconds: if status & 2 != 0 {
                            -1.0
                        } else {
                            attribute.begin * clip.length
                        },
                        end_seconds: if status & 2 != 0 {
                            -1.0
                        } else {
                            attribute.end * clip.length
                        },
                    }));
                }
                Ok(None)
            }
            N::AddSqt { motion, .. } => self.get_attribute(motion, name, flags),
            N::Mirror {
                child, hierarchy, ..
            } => {
                let mut attribute = self.get_attribute(
                    child
                        .as_ref()
                        .ok_or(TreeAttributeError::Missing("Mirror child"))?,
                    name,
                    flags,
                )?;
                if let Some(attribute) = &mut attribute {
                    mirror_attribute(attribute, hierarchy.as_ref())?;
                }
                Ok(attribute)
            }
            N::PhaseBlend(phase) => {
                let first = phase
                    .child1
                    .as_ref()
                    .ok_or(TreeAttributeError::Missing("PhaseBlend child1"))?;
                let second = phase
                    .child2
                    .as_ref()
                    .ok_or(TreeAttributeError::Missing("PhaseBlend child2"))?;
                if phase.blend < phase.cull_threshold {
                    return self.get_attribute(first, name, flags);
                }
                if phase.blend > 1.0 - phase.cull_threshold {
                    return self.get_attribute(second, name, flags);
                }
                let left = self.get_attribute(first, name, flags)?;
                let right = self.get_attribute(second, name, 31)?;
                if let (Some(mut left), Some(right)) = (left, right) {
                    blend_attribute(&mut left, &right, phase.blend)?;
                    Ok(Some(left))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// `Clip82d80808`, `PhaseBlend82d7fab0`, `AddSQT82588678`, `Mirror82ca9a60`.
    pub(crate) fn get_attributes(
        &self,
        node: &AnimationNodeHandle,
        flags: u8,
        scratch: &mut AnimationAttributePool,
        output: &mut Vec<AndaleRuntimeAttribute>,
    ) -> Result<(), TreeAttributeError> {
        use AnimationNodeKind as N;
        match &self.node(node)?.kind {
            N::Sequence(_) => self.sequence_attributes(node, scratch, output),
            N::Selector(n) => {
                if n.transitioning {
                    return Err(TreeAttributeError::Missing(
                        "Selector transitioning GetAttributes assertion",
                    ));
                }
                self.get_attributes(&self.selected_child(node)?, flags, scratch, output)
            }
            N::BlendSpace(_) => self.get_blend_space_attributes(node, flags, scratch, output),
            N::Pose { .. } | N::FixedSqt { .. } => Ok(()), // Pose vtable82044b48+30 points at shared blr825187c0.
            N::Clip(clip) => Self::clip_attributes(clip, flags, output),
            N::AddSqt { motion, .. } => self.get_attributes(motion, flags, scratch, output),
            N::Mirror {
                child, hierarchy, ..
            } => {
                self.get_attributes(
                    child
                        .as_ref()
                        .ok_or(TreeAttributeError::Missing("Mirror child"))?,
                    flags,
                    scratch,
                    output,
                )?;
                for attribute in output {
                    mirror_attribute(attribute, hierarchy.as_ref())?;
                }
                Ok(())
            }
            N::PhaseBlend(phase) => {
                let first = phase
                    .child1
                    .as_ref()
                    .ok_or(TreeAttributeError::Missing("PhaseBlend child1"))?;
                let second = phase
                    .child2
                    .as_ref()
                    .ok_or(TreeAttributeError::Missing("PhaseBlend child2"))?;
                if phase.blend < phase.cull_threshold {
                    return self.get_attributes(first, flags, scratch, output);
                }
                if phase.blend > 1.0 - phase.cull_threshold {
                    return self.get_attributes(second, flags, scratch, output);
                }
                push_scratch(scratch)?;
                push_scratch(scratch)?;
                let mut left = Vec::new();
                let mut right = Vec::new();
                self.get_attributes(first, flags, scratch, &mut left)?;
                self.get_attributes(second, 31, scratch, &mut right)?;
                blend_attributes(left, &right, phase.blend, output)?;
                scratch.element_count -= 2;
                Ok(())
            }
        }
    }

    #[expect(
        clippy::float_cmp,
        reason = "source static attribute sentinel compares exactly with -1"
    )]
    fn clip_attributes(
        clip: &ClipNode,
        flags: u8,
        output: &mut Vec<AndaleRuntimeAttribute>,
    ) -> Result<(), TreeAttributeError> {
        let header = clip
            .header
            .as_ref()
            .ok_or(TreeAttributeError::Missing("Clip header"))?;
        let database = header
            .database
            .lock()
            .map_err(|_| AnimationTreeError::Lock)?;
        let source = database
            .clip_by_record(header.record_index)
            .ok_or(AnimationTreeError::MissingRecord)?;
        for attribute in &source.attributes {
            let status = clip_attribute_status(clip, attribute)?;
            // Static attributes bypass f entirely in GetAttributes (not GetAttribute).
            if attribute.begin == -1.0
                || ((flags & 0x1c & status) != 0 && (flags & 3 & status) != 0)
            {
                if matches!(attribute.value, AndaleAttributeValue::Unsupported { .. }) {
                    return Err(TreeAttributeError::TypeMismatch);
                }
                append_attribute(
                    output,
                    AndaleRuntimeAttribute {
                        sequence_id: -1,
                        name: attribute.name.clone(),
                        begin_seconds: if status & 2 != 0 {
                            -1.0
                        } else {
                            attribute.begin * clip.length
                        },
                        end_seconds: if status & 2 != 0 {
                            -1.0
                        } else {
                            attribute.end * clip.length
                        },
                        status,
                        value: attribute.value.clone(),
                    },
                )?;
            }
        }
        Ok(())
    }

    /// `PhaseBlend::SetAttributes82d79528`, including all-child refresh and stable sort.
    #[expect(
        clippy::too_many_lines,
        reason = "preserve recursive writes, cache update, child coordinates and retiming prefixes"
    )]
    pub(crate) fn set_attributes_with_blend_space(
        &mut self,
        node: &AnimationNodeHandle,
        input: &[SettableTreeAttribute],
        context: &mut Option<BlendSpaceAttributeContext<'_>>,
    ) -> Result<bool, TreeAttributeError> {
        use AnimationNodeKind as N;
        let children = match &self.node(node)?.kind {
            N::Sequence(_) => return self.set_sequence_attributes(node, input, context),
            N::Selector(_) => return self.set_selector_attributes(node, input, context),
            N::BlendSpace(_) => {
                let context = context
                    .as_mut()
                    .ok_or(TreeAttributeError::Missing("BlendSpace attribute context"))?;
                return self
                    .set_blend_space_attributes(node, input, context)
                    .map_err(TreeAttributeError::BlendSpace);
            }
            N::Clip(_) | N::Pose { .. } | N::FixedSqt { .. } => return Ok(false),
            N::AddSqt { motion, .. } => {
                return self.set_attributes_with_blend_space(&motion.clone(), input, context);
            }
            N::Mirror { child, .. } => {
                return self.set_attributes_with_blend_space(
                    &child
                        .clone()
                        .ok_or(TreeAttributeError::Missing("Mirror child"))?,
                    input,
                    context,
                );
            }
            N::PhaseBlend(phase) => phase
                .children
                .iter()
                .map(|(_, child)| child.clone())
                .collect::<Vec<_>>(),
        };
        if children.len() < 2 {
            return Err(TreeAttributeError::Missing("two PhaseBlend children"));
        }
        let mut changed = false;
        for child in &children {
            changed |= self.set_attributes_with_blend_space(child, input, context)?;
        }
        let N::PhaseBlend(phase) = &mut self.node_mut(node)?.kind else {
            unreachable!()
        };
        if let Some(attribute) = input
            .iter()
            .find(|attribute| attribute.name == phase.attribute)
        {
            let similar = attribute.normalized == phase.last_attribute_normalized
                && (attribute.value - phase.last_attribute).abs() < 0.0001;
            if !similar {
                phase.last_attribute = attribute.value;
                phase.last_attribute_normalized = attribute.normalized;
                changed = true;
            }
        }
        if !changed {
            return Ok(false);
        }
        let name = phase.attribute;
        for (index, child) in children.iter().enumerate() {
            let coordinate = self
                .get_attribute(child, name, 15)?
                .and_then(|attribute| attribute.scalar_value())
                .ok_or(TreeAttributeError::Missing("PhaseBlend child coordinate"))?;
            let N::PhaseBlend(phase) = &mut self.node_mut(node)?.kind else {
                unreachable!()
            };
            phase.children[index].0 = coordinate;
        }
        let N::PhaseBlend(phase) = &mut self.node_mut(node)?.kind else {
            unreachable!()
        };
        // Native bubble sort compares ordered less; equal and unordered pairs stay put.
        for end in (1..phase.children.len()).rev() {
            for index in 0..end {
                if phase.children[index + 1].0 < phase.children[index].0 {
                    phase.children.swap(index, index + 1);
                }
            }
        }
        let first_value = phase.children[0].0;
        let last_value = phase.children[phase.children.len() - 1].0;
        let desired = if phase.last_attribute_normalized {
            (last_value - first_value).mul_add(phase.last_attribute, first_value)
        } else {
            phase.last_attribute
        };
        let mut start = 0;
        let mut count = phase.children.len();
        while count != 0 {
            let half = count / 2;
            let middle = start + half;
            if phase.children[middle].0 < desired {
                start = middle + 1;
                count -= half + 1;
            } else {
                count = half;
            }
        }
        let index = start.min(phase.children.len() - 1).max(1);
        let (lower, first) = phase.children[index - 1].clone();
        let (upper, second) = phase.children[index].clone();
        let denominator = upper - lower;
        let denominator = if denominator == 0.0 { 1.0 } else { denominator };
        let lower_clamped = select(lower - desired, lower, desired);
        let upper_clamped = select(upper - lower_clamped, lower_clamped, upper);
        let value = (upper_clamped - lower) / denominator;
        let value = select(-value, 0.0, value);
        let blend = select(1.0 - value, value, 1.0);
        phase.blend = blend;
        let proportion = phase.time / select(-phase.length, 1.0, phase.length);
        if phase.length == 0.0 && phase.time != 0.0 {
            return Err(AnimationTreeError::ZeroLength.into());
        }
        phase.child1 = Some(first.clone());
        phase.child2 = Some(second.clone());
        if first == second {
            return Err(TreeAttributeError::Missing("distinct PhaseBlend children"));
        }
        let first_length = self.length(&first)? * (1.0 - blend);
        let length = self.length(&second)?.mul_add(blend, first_length);
        let N::PhaseBlend(phase) = &mut self.node_mut(node)?.kind else {
            unreachable!()
        };
        phase.length = length;
        self.set_time(node, length * proportion)?;
        Ok(true)
    }
}

fn select(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// `Clip::GetAttributeStatus82d6d3b0`; live time/old time and integer loop count.
#[expect(
    clippy::float_cmp,
    reason = "source static attribute sentinel compares exactly with -1"
)]
fn clip_attribute_status(
    clip: &ClipNode,
    attribute: &AndaleAttribute,
) -> Result<u8, TreeAttributeError> {
    if attribute.begin == -1.0 {
        return Ok(6);
    }
    let begin = attribute.begin * clip.length;
    let end = attribute.end * clip.length;
    // d43c is NOTGT; d468 is GT. These are not ordered <= comparisons
    // when either operand is NaN. Retain the raw CR branch predicates.
    let begins_by_current = begin.partial_cmp(&clip.time) != Some(std::cmp::Ordering::Greater);
    let crossed = if clip.loops == 0 {
        end > clip.old_time && begins_by_current
    } else {
        end > clip.old_time || begins_by_current
    };
    if !crossed {
        return Ok(17);
    }
    if clip.loops != 0
        && (clip.loops != 1
            || clip.time.partial_cmp(&clip.old_time) != Some(std::cmp::Ordering::Less))
    {
        return Err(TreeAttributeError::InvalidLoop);
    }
    Ok(if clip.time < begin || clip.time > end {
        9
    } else {
        5
    })
}

/// `Attributes::Blend82d7f3d0`: sorted intersection, left status and payload owner.
fn blend_attributes(
    left: Vec<AndaleRuntimeAttribute>,
    right: &[AndaleRuntimeAttribute],
    coefficient: f32,
    output: &mut Vec<AndaleRuntimeAttribute>,
) -> Result<(), TreeAttributeError> {
    let mut index = 0;
    for mut attribute in left {
        let name = packed_name(&attribute)?;
        while index < right.len() && packed_name(&right[index])?.words < name.words {
            index += 1;
        }
        if index == right.len() {
            break;
        }
        if packed_name(&right[index])? == name {
            blend_attribute(&mut attribute, &right[index], coefficient)?;
            append_attribute(output, attribute)?;
        }
    }
    Ok(())
}

/// `Attribute::MultBy82d6b0e8`; time stores precede payload dispatch.
fn multiply_attribute(attribute: &mut AndaleRuntimeAttribute, coefficient: f32) {
    attribute.begin_seconds = if attribute.status & 2 != 0 {
        -1.0
    } else {
        attribute.begin_seconds * coefficient
    };
    attribute.end_seconds = if attribute.status & 2 != 0 {
        -1.0
    } else {
        attribute.end_seconds * coefficient
    };
    match &mut attribute.value {
        AndaleAttributeValue::Float(value) => *value *= coefficient,
        AndaleAttributeValue::Vector(values) => {
            for value in values {
                *value *= coefficient;
            }
        }
        AndaleAttributeValue::Bytes24(bytes) => {
            let value = f32::from_be_bytes(bytes[20..24].try_into().expect("four bytes"));
            bytes[20..24].copy_from_slice(&(coefficient * value).to_be_bytes());
        }
        AndaleAttributeValue::Unsupported { .. } => (),
    }
}
/// `Attribute::MultAdd82d6b198`; separate time product/add stores precede type assertion.
#[expect(
    clippy::assign_op_pattern,
    reason = "retain native contribution plus old operand order"
)]
fn multiply_add_attribute(
    left: &mut AndaleRuntimeAttribute,
    right: &AndaleRuntimeAttribute,
    coefficient: f32,
) -> Result<(), TreeAttributeError> {
    let begin = if left.status & 2 != 0 {
        0.0
    } else {
        right.begin_seconds * coefficient
    };
    left.begin_seconds = begin + left.begin_seconds;
    let end = if left.status & 2 != 0 {
        0.0
    } else {
        right.end_seconds * coefficient
    };
    left.end_seconds = end + left.end_seconds;
    match (&mut left.value, &right.value) {
        (AndaleAttributeValue::Float(left), AndaleAttributeValue::Float(right)) => {
            *left = right.mul_add(coefficient, *left);
        }
        (AndaleAttributeValue::Vector(left), AndaleAttributeValue::Vector(right)) => {
            for (left, right) in left.iter_mut().zip(right) {
                *left = right.mul_add(coefficient, *left);
            }
        }
        (AndaleAttributeValue::Bytes24(left), AndaleAttributeValue::Bytes24(right)) => {
            let a = f32::from_be_bytes(left[20..24].try_into().expect("four bytes"));
            let b = f32::from_be_bytes(right[20..24].try_into().expect("four bytes"));
            left[20..24].copy_from_slice(&b.mul_add(coefficient, a).to_be_bytes());
        }
        (
            AndaleAttributeValue::Unsupported {
                attribute_type: a, ..
            },
            AndaleAttributeValue::Unsupported {
                attribute_type: b, ..
            },
        ) if a == b => (),
        _ => return Err(TreeAttributeError::TypeMismatch),
    }
    Ok(())
}
/// Existing two-way caller retains its original `MultBy` then `MultAdd` ordering.
fn blend_attribute(
    left: &mut AndaleRuntimeAttribute,
    right: &AndaleRuntimeAttribute,
    coefficient: f32,
) -> Result<(), TreeAttributeError> {
    multiply_attribute(left, 1.0 - coefficient);
    multiply_add_attribute(left, right, coefficient)
}

/// `Attribute::Mirror82ca9938`: sign-XOR vector X or replace only packed bone.
fn mirror_attribute(
    attribute: &mut AndaleRuntimeAttribute,
    hierarchy: Option<&std::sync::Arc<std::sync::Mutex<crate::actor_face_playback::FaceDatabase>>>,
) -> Result<(), TreeAttributeError> {
    match &mut attribute.value {
        AndaleAttributeValue::Float(_) => (),
        AndaleAttributeValue::Vector(value) => {
            value[0] = f32::from_bits(value[0].to_bits() ^ 0x8000_0000);
        }
        AndaleAttributeValue::Bytes24(bytes) => {
            let name = FastString30 {
                words: std::array::from_fn(|index| {
                    u32::from_be_bytes(
                        bytes[index * 4..index * 4 + 4]
                            .try_into()
                            .expect("four bytes"),
                    )
                }),
            };
            let owner = hierarchy
                .ok_or(TreeAttributeError::Missing("Mirror hierarchy"))?
                .lock()
                .map_err(|_| AnimationTreeError::Lock)?;
            let hierarchy = &owner
                .parsed
                .as_ref()
                .ok_or(AnimationTreeError::MissingRecord)?
                .hierarchy;
            let bone = hierarchy
                .packed_bone_names
                .iter()
                .position(|candidate| *candidate == name)
                .ok_or(TreeAttributeError::Missing("attribute bone"))?;
            let partner = usize::try_from(hierarchy.bone_metadata[bone])
                .map_err(|_| TreeAttributeError::Missing("mirror bone"))?;
            let mirrored = hierarchy
                .packed_bone_names
                .get(partner)
                .ok_or(TreeAttributeError::Missing("mirror bone"))?;
            for (word, output) in mirrored.words.iter().zip(bytes[..20].chunks_exact_mut(4)) {
                output.copy_from_slice(&word.to_be_bytes());
            }
        }
        AndaleAttributeValue::Unsupported { .. } => return Err(TreeAttributeError::TypeMismatch),
    }
    Ok(())
}

#[cfg(test)]
#[path = "actor_animation_tree_attributes_tests.rs"]
mod tests;

#[path = "actor_animation_blend_space_attributes.rs"]
mod blend_space_attributes;
