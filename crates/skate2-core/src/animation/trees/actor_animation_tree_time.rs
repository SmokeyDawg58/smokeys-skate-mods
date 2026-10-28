//! Native tree `AddTime`; animation properties follow the motion operand.
use super::{AnimationNodeHandle, AnimationNodeKind, AnimationTreeError, AnimationTreePool};

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimationProperties {
    pub(crate) is_playing: bool,
    pub(crate) expired_or_looped: bool,
    pub(crate) expired_by: f32,
    pub(crate) time_to_expire: f32,
}
impl Default for AnimationProperties {
    fn default() -> Self {
        Self {
            is_playing: false,
            expired_or_looped: false,
            expired_by: -1.0,
            time_to_expire: -1.0,
        }
    }
}

impl AnimationTreePool {
    pub(crate) fn time(&self, node: &AnimationNodeHandle) -> Result<f32, AnimationTreeError> {
        match &self.node(node)?.kind {
            AnimationNodeKind::Sequence(_) | AnimationNodeKind::Selector(_) => {
                self.time(&self.selected_child(node)?)
            }
            AnimationNodeKind::Clip(clip) => Ok(clip.time),
            AnimationNodeKind::BlendSpace(node) => Ok(node.time),
            AnimationNodeKind::PhaseBlend(phase) => {
                if phase.child1.is_none() || phase.child2.is_none() {
                    return Err(AnimationTreeError::MissingChild);
                }
                Ok(phase.time)
            }
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. } => Ok(0.0),
            AnimationNodeKind::AddSqt { motion, .. } => self.time(motion),
            AnimationNodeKind::Mirror { child, .. } => {
                self.time(child.as_ref().ok_or(AnimationTreeError::MissingChild)?)
            }
        }
    }
    /// `Clip82d6d238`, `PhaseBlend82d6bd48`, `AddSQT82588500`, `Mirror82c92790`.
    pub(crate) fn add_time(
        &mut self,
        node: &AnimationNodeHandle,
        dt: f32,
        properties: &mut AnimationProperties,
    ) -> Result<(), AnimationTreeError> {
        if matches!(self.node(node)?.kind, AnimationNodeKind::Sequence(_)) {
            return self.add_sequence_time(node, dt, properties);
        }
        if matches!(self.node(node)?.kind, AnimationNodeKind::Selector(_)) {
            return self.add_selector_time(node, dt, properties);
        }
        if matches!(self.node(node)?.kind, AnimationNodeKind::BlendSpace(_)) {
            return self.add_blend_space_time(node, dt, properties);
        }
        // Pose AddTime is the empty vtable slot, including for unusual dt.
        if matches!(
            self.node(node)?.kind,
            AnimationNodeKind::Pose { .. } | AnimationNodeKind::FixedSqt { .. }
        ) {
            return Ok(());
        }
        if let AnimationNodeKind::PhaseBlend(phase) = &self.node(node)?.kind {
            if phase.child1.is_none() || phase.child2.is_none() {
                return Err(AnimationTreeError::MissingChild);
            }
        }
        if dt < 0.0 {
            return Err(AnimationTreeError::NegativeTime);
        }
        match &mut self.node_mut(node)?.kind {
            AnimationNodeKind::Clip(clip) => {
                if clip.header.is_none() {
                    return Err(AnimationTreeError::MissingRecord);
                }
                clip.time += dt;
                let length = clip.length;
                properties.time_to_expire = length - clip.time;
                if clip.time > length {
                    if clip.looping {
                        while clip.time > length {
                            properties.expired_or_looped = true;
                            properties.expired_by = clip.time - length;
                            let previous = clip.time;
                            clip.time -= length;
                            clip.loops = clip.loops.wrapping_add(1);
                            if clip.time.to_bits() == previous.to_bits() {
                                return Err(AnimationTreeError::NonProgressingLoop);
                            }
                        }
                    } else {
                        properties.expired_or_looped = true;
                        properties.expired_by = clip.time - length;
                    }
                } else {
                    properties.expired_or_looped = false;
                }
                Ok(())
            }
            AnimationNodeKind::PhaseBlend(_) => self.add_phase_time(node, dt, properties),
            AnimationNodeKind::AddSqt { motion, .. } => {
                let motion = motion.clone();
                self.add_time(&motion, dt, properties)
            }
            AnimationNodeKind::Mirror { child, .. } => {
                let child = child.clone().ok_or(AnimationTreeError::MissingChild)?;
                self.add_time(&child, dt, properties)
            }
            AnimationNodeKind::Pose { .. }
            | AnimationNodeKind::FixedSqt { .. }
            | AnimationNodeKind::BlendSpace(_)
            | AnimationNodeKind::Sequence(_)
            | AnimationNodeKind::Selector(_) => unreachable!(),
        }
    }
    fn add_phase_time(
        &mut self,
        node: &AnimationNodeHandle,
        dt: f32,
        properties: &mut AnimationProperties,
    ) -> Result<(), AnimationTreeError> {
        let AnimationNodeKind::PhaseBlend(phase) = &self.node(node)?.kind else {
            unreachable!()
        };
        let first = phase
            .child1
            .clone()
            .ok_or(AnimationTreeError::MissingChild)?;
        let second = phase
            .child2
            .clone()
            .ok_or(AnimationTreeError::MissingChild)?;
        let length1 = self.length(&first)?;
        let length2 = self.length(&second)?;
        let length = self.length(node)?;
        if length == 0.0 {
            return Err(AnimationTreeError::ZeroLength);
        }
        let proportion = dt / length;
        let mut discarded = AnimationProperties::default();
        self.add_time(&first, proportion * length1, properties)?;
        self.add_time(&second, proportion * length2, &mut discarded)?;
        let inverse_first = 1.0 / length1;
        let time = (self.time(&first)? * inverse_first) * length;
        let AnimationNodeKind::PhaseBlend(phase) = &mut self.node_mut(node)?.kind else {
            unreachable!()
        };
        phase.time = time;
        properties.time_to_expire = (properties.time_to_expire * inverse_first) * length;
        let expired_by = (inverse_first * length) * properties.expired_by;
        properties.expired_by = expired_by;
        let check = if properties.expired_or_looped {
            properties.expired_by
        } else {
            properties.time_to_expire
        };
        // Raw bge accepts unordered too (tests LT false), matching native assert.
        if check < 0.0 {
            return Err(AnimationTreeError::NegativeTime);
        }
        Ok(())
    }
}
