//! Native mutable animation nodes; references retain their actual source owners.
//! Pool pushes preserve `PoolStack` strict end checks and original stack order.
use crate::{
    actor_face_playback::{AnimationPoseRef, FaceDatabase},
    play_animation::FastString30,
};
use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
};

#[path = "actor_animation_blend_space.rs"]
pub(crate) mod blend_space;

#[path = "actor_animation_tree_time.rs"]
pub(crate) mod time;

#[path = "actor_animation_tree_eval.rs"]
pub(crate) mod eval;

#[path = "actor_animation_tree_attributes.rs"]
pub(crate) mod attributes;
#[path = "actor_animation_sequence_node.rs"]
pub(crate) mod sequence;
#[path = "actor_animation_sequence_attributes.rs"]
mod sequence_attributes;
#[path = "actor_animation_sequence_eval.rs"]
mod sequence_eval;

#[derive(Debug, Clone)]
pub(crate) struct AnimationNodeHandle {
    pool: Arc<()>,
    index: usize,
}
impl PartialEq for AnimationNodeHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.pool, &other.pool) && self.index == other.index
    }
}
impl Eq for AnimationNodeHandle {}
#[derive(Debug)]
pub(crate) enum AnimationTreeError {
    ElementCapacity,
    ByteCapacity,
    Allocation,
    ForeignNode,
    NodeKind,
    MissingRecord,
    MissingChild,
    TooManyChildren,
    InvalidSpeed,
    NegativeTime,
    TimeOutsideClip,
    ZeroLength,
    ZeroSpeed,
    Lock,
    UnsupportedRecord(u32),
    ControllerAssertion(&'static str),
    PendingBlendSpaceOperation(&'static str),
    NonProgressingLoop,
}
impl std::fmt::Display for AnimationTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingBlendSpaceOperation(operation) => {
                write!(f, "BlendSpace requires {operation}")
            }
            Self::UnsupportedRecord(kind) => {
                write!(f, "animation tree record family is not implemented: {kind}")
            }
            other => write!(f, "animation tree: {other:?}"),
        }
    }
}

/// The source header pointer is a database occurrence and record, not a copy of
/// decoded SQTs or a caller-supplied pose pretending to be a clip.
#[derive(Clone)]
pub(crate) struct AnimationClipRef {
    pub(crate) database: Arc<Mutex<FaceDatabase>>,
    pub(crate) record_index: usize,
}
#[expect(
    dead_code,
    reason = "original Clip debug and trim fields await native evaluation consumers"
)]
pub(crate) struct ClipNode {
    pub(crate) header: Option<AnimationClipRef>,
    pub(crate) debug_old_time: f32,
    pub(crate) debug_num_loops: i32,
    pub(crate) speed: f32,
    pub(crate) length: f32,
    pub(crate) time: f32,
    pub(crate) old_time: f32,
    pub(crate) loops: i32,
    pub(crate) frames: f32,
    pub(crate) looping: bool,
    pub(crate) fps: f32,
    pub(crate) base_speed: f32,
    pub(crate) trimmed: bool,
}
impl ClipNode {
    /// Inline constructor in `Push<Clip>828087a8`.
    fn new() -> Self {
        Self {
            header: None,
            debug_old_time: 0.0,
            debug_num_loops: 0,
            speed: 0.0,
            length: 0.0,
            time: 0.0,
            old_time: 0.0,
            loops: 0,
            frames: 0.0,
            looping: false,
            fps: 30.0,
            base_speed: 1.0,
            trimmed: false,
        }
    }
    /// `FrameToTime82805ed0` uses two ordered f32 multiplies and one division.
    fn frame_to_time(&self, frame: f32) -> Result<f32, AnimationTreeError> {
        if self.speed == 0.0 || self.base_speed == 0.0 {
            return Err(AnimationTreeError::ZeroSpeed);
        }
        Ok(frame / ((self.base_speed * self.fps) * self.speed))
    }
    /// `Clip::SetSpeed82807108`; source bgt requires ordered positive speed.
    fn set_speed(&mut self, speed: f32) -> Result<(), AnimationTreeError> {
        require_speed(speed)?;
        let previous_speed = self.speed * self.base_speed;
        let time = previous_speed * self.time;
        let old_time = previous_speed * self.old_time;
        self.speed = speed;
        let inverse = 1.0 / (self.speed * self.base_speed);
        self.time = inverse * time;
        self.old_time = inverse * old_time;
        self.length = self.frame_to_time(self.frames - 1.0)?;
        Ok(())
    }
    /// `SetTime828071f0`: native not-LT branch accepts unordered time.
    fn set_time(&mut self, time: f32) -> Result<(), AnimationTreeError> {
        if time < 0.0 {
            return Err(AnimationTreeError::NegativeTime);
        }
        self.time = time;
        self.old_time = time;
        self.loops = 0;
        Ok(())
    }
}

pub(crate) struct PhaseBlendNode {
    pub(crate) attribute: FastString30,
    pub(crate) blend: f32,
    pub(crate) last_attribute: f32,
    pub(crate) time: f32,
    pub(crate) length: f32,
    pub(crate) cull_threshold: f32,
    pub(crate) child1: Option<AnimationNodeHandle>,
    pub(crate) child2: Option<AnimationNodeHandle>,
    pub(crate) last_attribute_normalized: bool,
    pub(crate) children: Vec<(f32, AnimationNodeHandle)>,
}
impl PhaseBlendNode {
    /// `PhaseBlend::PhaseBlend82d75810`; low31 padding bits are not fabricated.
    fn new() -> Self {
        Self {
            attribute: FastString30 { words: [0; 5] },
            blend: 0.0,
            last_attribute: f32::MAX,
            time: 0.0,
            length: 0.0,
            cull_threshold: 0.0,
            child1: None,
            child2: None,
            last_attribute_normalized: true,
            children: Vec::new(),
        }
    }
}
pub(crate) enum AnimationNodeKind {
    Sequence(sequence::SequenceNode),
    Selector(sequence::SelectorNode),
    Clip(ClipNode),
    PhaseBlend(PhaseBlendNode),
    BlendSpace(blend_space::BlendSpaceNode),
    Pose {
        header: Option<AnimationPoseRef>,
    },
    FixedSqt {
        id: u32,
    },
    AddSqt {
        a: AnimationNodeHandle,
        b: AnimationNodeHandle,
        motion: AnimationNodeHandle,
    },
    Mirror {
        hierarchy: Option<Arc<Mutex<FaceDatabase>>>,
        child: Option<AnimationNodeHandle>,
        trajectory_use: Option<u32>,
    },
}
pub(crate) struct AnimationNode {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "original node evaluation bit awaits native evaluator; corpus checks constructor value"
        )
    )]
    pub(crate) evaluated: bool,
    pub(crate) kind: AnimationNodeKind,
    start: usize,
}
pub(crate) struct AnimationTreePool {
    identity: Arc<()>,
    maximum_bytes: usize,
    maximum_elements: usize,
    location: usize,
    nodes: Vec<AnimationNode>,
}
impl AnimationTreePool {
    /// `PoolStack::Clear82c8ef98`: destroy elements in allocation order.
    pub(crate) fn clear(&mut self) {
        for node in self.nodes.drain(..) {
            drop(node);
        }
        self.location = 0;
    }
    pub(crate) fn new(maximum_bytes: usize, maximum_elements: usize) -> Self {
        Self {
            identity: Arc::new(()),
            maximum_bytes,
            maximum_elements,
            location: 0,
            nodes: Vec::new(),
        }
    }
    fn push(
        &mut self,
        bytes: usize,
        kind: AnimationNodeKind,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        if self.nodes.len() >= self.maximum_elements {
            return Err(AnimationTreeError::ElementCapacity);
        }
        let end = self
            .location
            .checked_add(bytes)
            .filter(|&end| end < self.maximum_bytes)
            .ok_or(AnimationTreeError::ByteCapacity)?;
        self.nodes
            .try_reserve(1)
            .map_err(|_| AnimationTreeError::Allocation)?;
        let handle = AnimationNodeHandle {
            pool: Arc::clone(&self.identity),
            index: self.nodes.len(),
        };
        self.nodes.push(AnimationNode {
            evaluated: false,
            kind,
            start: self.location,
        });
        self.location = end;
        Ok(handle)
    }
    pub(crate) fn node(
        &self,
        handle: &AnimationNodeHandle,
    ) -> Result<&AnimationNode, AnimationTreeError> {
        if !Arc::ptr_eq(&self.identity, &handle.pool) {
            return Err(AnimationTreeError::ForeignNode);
        }
        self.nodes
            .get(handle.index)
            .ok_or(AnimationTreeError::ForeignNode)
    }
    fn node_mut(
        &mut self,
        handle: &AnimationNodeHandle,
    ) -> Result<&mut AnimationNode, AnimationTreeError> {
        if !Arc::ptr_eq(&self.identity, &handle.pool) {
            return Err(AnimationTreeError::ForeignNode);
        }
        self.nodes
            .get_mut(handle.index)
            .ok_or(AnimationTreeError::ForeignNode)
    }
    /// Source Pop removes the current top; it does not roll back a recursive tree.
    pub(crate) fn pop(&mut self) -> Result<(), AnimationTreeError> {
        let node = self.nodes.pop().ok_or(AnimationTreeError::MissingChild)?;
        self.location = node.start;
        Ok(())
    }
    pub(crate) fn push_clip(&mut self) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.push(64, AnimationNodeKind::Clip(ClipNode::new()))
    }
    /// `Clip::Init82807260`: preserve writes before either later assertion.
    pub(crate) fn initialize_clip(
        &mut self,
        handle: &AnimationNodeHandle,
        header: AnimationClipRef,
        speed: f32,
        time: f32,
    ) -> Result<(), AnimationTreeError> {
        require_speed(speed)?;
        let (frames, looping, fps, base_speed) = {
            let database = header
                .database
                .lock()
                .map_err(|_| AnimationTreeError::Lock)?;
            let clip = database
                .clip_by_record(header.record_index)
                .ok_or(AnimationTreeError::MissingRecord)?;
            (
                clip.frame_count,
                clip.looping(),
                clip.frames_per_second,
                clip.base_speed,
            )
        };
        let AnimationNodeKind::Clip(clip) = &mut self.node_mut(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        clip.header = Some(header);
        clip.frames = frames;
        clip.looping = looping;
        clip.fps = fps;
        clip.loops = 0;
        clip.base_speed = base_speed;
        clip.old_time = time;
        clip.set_speed(speed)?;
        clip.set_time(time)?;
        clip.length = clip.frame_to_time(clip.frames - 1.0)?;
        if time < 0.0 || time > clip.length {
            return Err(AnimationTreeError::TimeOutsideClip);
        }
        Ok(())
    }
    pub(crate) fn push_phase_blend(
        &mut self,
        attribute: FastString30,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        let handle = self.push(128, AnimationNodeKind::PhaseBlend(PhaseBlendNode::new()))?;
        let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(&handle)?.kind else {
            unreachable!()
        };
        phase.attribute = attribute;
        phase.time = 0.0;
        Ok(handle)
    }
    /// `PhaseBlend::AddChild82d7af60`, including publication before length query.
    pub(crate) fn add_phase_child(
        &mut self,
        parent: &AnimationNodeHandle,
        child: &AnimationNodeHandle,
    ) -> Result<(), AnimationTreeError> {
        self.node(child)?;
        let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(parent)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        if phase.children.len() >= 5 {
            return Err(AnimationTreeError::TooManyChildren);
        }
        phase
            .children
            .try_reserve(1)
            .map_err(|_| AnimationTreeError::Allocation)?;
        phase.children.push((0.0, child.clone()));
        let first = phase.children.len() == 1;
        if first {
            phase.child1 = Some(child.clone());
        } else if phase.children.len() == 2 {
            phase.child2 = Some(child.clone());
        }
        if first {
            let length = self.length(child)?;
            let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(parent)?.kind else {
                unreachable!()
            };
            phase.length = length;
        }
        Ok(())
    }
    pub(crate) fn length(&self, handle: &AnimationNodeHandle) -> Result<f32, AnimationTreeError> {
        match &self.node(handle)?.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                self.length(&self.selected_child(handle)?)
            }
            AnimationNodeKind::Clip(clip) => Ok(clip.length),
            AnimationNodeKind::BlendSpace(_) => self.blend_space_length(handle),
            AnimationNodeKind::PhaseBlend(phase) => {
                if phase.child1.is_none() || phase.child2.is_none() {
                    return Err(AnimationTreeError::MissingChild);
                }
                Ok(phase.length)
            }
            AnimationNodeKind::AddSqt { motion, .. } => self.length(motion),
            AnimationNodeKind::Mirror { child, .. } => {
                self.length(child.as_ref().ok_or(AnimationTreeError::MissingChild)?)
            }
            // Pose::GetLength82587bd0 asserts; it is not a zero-duration clip.
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => {
                Err(AnimationTreeError::NodeKind)
            }
        }
    }
    /// `PhaseBlend::SetSpeed82d75a80`, source child order and fused length blend.
    pub(crate) fn set_speed(
        &mut self,
        handle: &AnimationNodeHandle,
        speed: f32,
    ) -> Result<(), AnimationTreeError> {
        match &self.node(handle)?.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                return self.set_controller_speed(handle, speed);
            }
            AnimationNodeKind::BlendSpace(_) => return self.set_blend_space_speed(handle, speed),
            // Pose vtable82044b48 slots7/8 point to the empty825187c0.
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => return Ok(()),
            AnimationNodeKind::AddSqt { a, b, .. } => {
                let (a, b) = (a.clone(), b.clone());
                self.set_speed(&a, speed)?;
                return self.set_speed(&b, speed);
            }
            AnimationNodeKind::Mirror { child, .. } => {
                let child = child.clone().ok_or(AnimationTreeError::MissingChild)?;
                return self.set_speed(&child, speed);
            }
            AnimationNodeKind::Clip(_) | AnimationNodeKind::PhaseBlend(_) => {}
        }
        require_speed(speed)?;
        if let AnimationNodeKind::Clip(clip) = &mut self.node_mut(handle)?.kind {
            return clip.set_speed(speed);
        }
        let AnimationNodeKind::PhaseBlend(phase) = &self.node(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        let denominator = if -phase.length >= 0.0 {
            1.0
        } else {
            phase.length
        };
        let proportion = phase.time / denominator;
        let children: Vec<_> = phase
            .children
            .iter()
            .map(|(_, child)| child.clone())
            .collect();
        for child in children {
            self.set_speed(&child, speed)?;
        }
        let AnimationNodeKind::PhaseBlend(phase) = &self.node(handle)?.kind else {
            unreachable!()
        };
        let length1 = self.length(
            phase
                .child1
                .as_ref()
                .ok_or(AnimationTreeError::MissingChild)?,
        )?;
        let length2 = self.length(
            phase
                .child2
                .as_ref()
                .ok_or(AnimationTreeError::MissingChild)?,
        )?;
        let length = length2.mul_add(phase.blend, length1 * (1.0 - phase.blend));
        let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(handle)?.kind else {
            unreachable!()
        };
        phase.length = length;
        self.set_time(handle, length * proportion)
    }
    /// `PhaseBlend::SetTime82d75970` updates just the selected two children.
    pub(crate) fn set_time(
        &mut self,
        handle: &AnimationNodeHandle,
        time: f32,
    ) -> Result<(), AnimationTreeError> {
        match &self.node(handle)?.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                return self.set_controller_time(handle, time);
            }
            AnimationNodeKind::BlendSpace(_) => return self.set_blend_space_time(handle, time),
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => return Ok(()),
            AnimationNodeKind::AddSqt { a, b, .. } => {
                let (a, b) = (a.clone(), b.clone());
                self.set_time(&a, time)?;
                return self.set_time(&b, time);
            }
            AnimationNodeKind::Mirror { child, .. } => {
                let child = child.clone().ok_or(AnimationTreeError::MissingChild)?;
                return self.set_time(&child, time);
            }
            AnimationNodeKind::Clip(_) | AnimationNodeKind::PhaseBlend(_) => {}
        }
        if time < 0.0 {
            return Err(AnimationTreeError::NegativeTime);
        }
        if let AnimationNodeKind::Clip(clip) = &mut self.node_mut(handle)?.kind {
            return clip.set_time(time);
        }
        let AnimationNodeKind::PhaseBlend(phase) = &self.node(handle)?.kind else {
            return Err(AnimationTreeError::NodeKind);
        };
        if phase.length == 0.0 {
            return Err(AnimationTreeError::ZeroLength);
        }
        let proportion = time / phase.length;
        let first = phase
            .child1
            .clone()
            .ok_or(AnimationTreeError::MissingChild)?;
        self.set_time(&first, self.length(&first)? * proportion)?;
        let AnimationNodeKind::PhaseBlend(phase) = &self.node(handle)?.kind else {
            unreachable!()
        };
        let second = phase
            .child2
            .clone()
            .ok_or(AnimationTreeError::MissingChild)?;
        self.set_time(&second, self.length(&second)? * proportion)?;
        let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(handle)?.kind else {
            unreachable!()
        };
        phase.time = time;
        Ok(())
    }
    pub(crate) fn push_pose(
        &mut self,
        header: Option<AnimationPoseRef>,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        let handle = self.push(16, AnimationNodeKind::Pose { header: None })?;
        let header = header.ok_or(AnimationTreeError::MissingRecord)?;
        let AnimationNodeKind::Pose {
            header: destination,
        } = &mut self.node_mut(&handle)?.kind
        else {
            unreachable!()
        };
        *destination = Some(header);
        Ok(handle)
    }
    /// `SetSQTById` node used by cPedestrianAnim::AddBindPose82e7f460.
    pub(crate) fn push_fixed_sqt(
        &mut self,
        id: u32,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.push(16, AnimationNodeKind::FixedSqt { id })
    }
    pub(crate) fn push_add_sqt(
        &mut self,
        a: AnimationNodeHandle,
        b: AnimationNodeHandle,
        motion: AnimationNodeHandle,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.node(&a)?;
        self.node(&b)?;
        self.node(&motion)?;
        self.push(32, AnimationNodeKind::AddSqt { a, b, motion })
    }
    pub(crate) fn push_mirror(
        &mut self,
        hierarchy: Option<Arc<Mutex<FaceDatabase>>>,
        child: AnimationNodeHandle,
        trajectory_use: u32,
    ) -> Result<AnimationNodeHandle, AnimationTreeError> {
        self.node(&child)?;
        let handle = self.push(
            32,
            AnimationNodeKind::Mirror {
                hierarchy: None,
                child: None,
                trajectory_use: None,
            },
        )?;
        let hierarchy = hierarchy.ok_or(AnimationTreeError::MissingRecord)?;
        let AnimationNodeKind::Mirror {
            hierarchy: destination,
            child: target,
            trajectory_use: usage,
        } = &mut self.node_mut(&handle)?.kind
        else {
            unreachable!()
        };
        *target = Some(child);
        *destination = Some(hierarchy);
        *usage = Some(trajectory_use);
        Ok(handle)
    }
}
fn require_speed(speed: f32) -> Result<(), AnimationTreeError> {
    if speed.partial_cmp(&0.0) == Some(Ordering::Greater) {
        Ok(())
    } else {
        Err(AnimationTreeError::InvalidSpeed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_exact_end_rejects_and_missing_pose_keeps_source_push() {
        let mut exact = AnimationTreePool::new(128, 20);
        assert!(matches!(
            exact.push_phase_blend(FastString30 { words: [0; 5] }),
            Err(AnimationTreeError::ByteCapacity)
        ));
        assert!(exact.nodes.is_empty());
        let mut pool = AnimationTreePool::new(145, 20);
        assert!(matches!(
            pool.push_pose(None),
            Err(AnimationTreeError::MissingRecord)
        ));
        assert_eq!(pool.location, 16);
        assert_eq!(pool.nodes.len(), 1);
        pool.push_phase_blend(FastString30 { words: [0; 5] })
            .unwrap();
        assert_eq!(pool.location, 144);
    }

    #[test]
    fn pool_identity_is_not_equal_indices_or_equal_empty_arc_payloads() {
        let mut first = AnimationTreePool::new(1024, 20);
        let mut second = AnimationTreePool::new(1024, 20);
        let a = first.push_clip().unwrap();
        let b = second.push_clip().unwrap();
        assert_ne!(a, b);
        assert!(matches!(
            first.node(&b),
            Err(AnimationTreeError::ForeignNode)
        ));
    }
}
