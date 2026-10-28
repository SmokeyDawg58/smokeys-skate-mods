//! Authored SequenceNode9 and SelectorNode10, distinct from inline transitions.
use super::time::AnimationProperties;
use super::{
    AnimationNodeHandle as H, AnimationNodeKind as Kind, AnimationTreeError as E, AnimationTreePool,
};
use crate::play_animation::FastString36;

pub(crate) struct SequenceChild {
    pub(crate) animation: H,
    pub(crate) enabled: bool,
    /// SequenceChild82d7b040 receives but never reads this authored argument.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained authored LST is ignored by native SequenceChild"
        )
    )]
    pub(crate) authored_stage: u32,
}
pub(crate) struct SequenceNode {
    pub(crate) children: Vec<SequenceChild>,
    pub(crate) selected: Option<usize>,
    pub(crate) next: Option<usize>,
    pub(crate) speed: f32,
    pub(crate) initialized: bool,
    pub(crate) transitioning: bool,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "native stores authored bit but MoveNext unconditionally wraps"
        )
    )]
    pub(crate) looping: bool,
    pub(crate) window: usize,
}
pub(crate) struct SelectorNode {
    pub(crate) children: Vec<(H, FastString36)>,
    pub(crate) selected: Option<usize>,
    /// None is the actual children.end sentinel.
    pub(crate) next: Option<usize>,
    pub(crate) transitioning: bool,
    pub(crate) transition_time: f32,
    pub(crate) transition_length: f32,
}

pub(crate) const FRAME: f32 = f32::from_bits(0x3d08_8889);

#[cfg(test)]
#[path = "actor_animation_sequence_tests.rs"]
pub(crate) mod tests;

impl AnimationTreePool {
    pub(crate) fn sequence(&self, h: &H) -> Result<&SequenceNode, E> {
        if let Kind::Sequence(n) = &self.node(h)?.kind {
            Ok(n)
        } else {
            Err(E::NodeKind)
        }
    }
    pub(crate) fn sequence_mut(&mut self, h: &H) -> Result<&mut SequenceNode, E> {
        if let Kind::Sequence(n) = &mut self.node_mut(h)?.kind {
            Ok(n)
        } else {
            Err(E::NodeKind)
        }
    }
    pub(crate) fn selector(&self, h: &H) -> Result<&SelectorNode, E> {
        if let Kind::Selector(n) = &self.node(h)?.kind {
            Ok(n)
        } else {
            Err(E::NodeKind)
        }
    }
    pub(crate) fn selector_mut(&mut self, h: &H) -> Result<&mut SelectorNode, E> {
        if let Kind::Selector(n) = &mut self.node_mut(h)?.kind {
            Ok(n)
        } else {
            Err(E::NodeKind)
        }
    }
    /// Push82d783f0: strict pool allocation 160 bytes, then authored fields.
    pub(crate) fn push_sequence(&mut self, looping: bool, window: u8) -> Result<H, E> {
        self.push(
            160,
            Kind::Sequence(SequenceNode {
                children: vec![],
                selected: None,
                next: None,
                speed: 1.0,
                initialized: false,
                transitioning: false,
                looping,
                window: usize::from(window),
            }),
        )
    }
    /// Push82d78520: fixed-vector overflow is enabled; authored14 is legal.
    pub(crate) fn push_selector(&mut self) -> Result<H, E> {
        self.push(
            416,
            Kind::Selector(SelectorNode {
                children: vec![],
                selected: None,
                next: None,
                transitioning: false,
                transition_time: 0.0,
                transition_length: 0.0,
            }),
        )
    }
    pub(crate) fn add_sequence_child(&mut self, h: &H, animation: H, stage: u32) -> Result<(), E> {
        let n = self.sequence_mut(h)?;
        n.children.try_reserve(1).map_err(|_| E::Allocation)?;
        n.children.push(SequenceChild {
            animation,
            enabled: true,
            authored_stage: stage,
        });
        if n.children.len() == 1 {
            n.selected = Some(0);
        } else if n.children.len() == 2 {
            self.sequence_move_next(h)?;
        }
        Ok(())
    }
    pub(crate) fn add_selector_child(
        &mut self,
        h: &H,
        animation: H,
        name: FastString36,
    ) -> Result<(), E> {
        let n = self.selector_mut(h)?;
        n.children.try_reserve(1).map_err(|_| E::Allocation)?;
        n.children.push((animation, name));
        if n.children.len() == 1 {
            n.selected = Some(0);
        }
        n.next = None;
        Ok(())
    }
    /// MoveNext82d6ed98, skipping disabled children, wrapping irrespective looping.
    pub(crate) fn sequence_move_next(&mut self, h: &H) -> Result<(), E> {
        let n = self.sequence_mut(h)?;
        let selected = n.selected.ok_or(E::MissingChild)?;
        if selected >= n.children.len() {
            return Err(E::MissingChild);
        }
        n.next = Some(selected);
        loop {
            let next = (n.next.expect("published") + 1) % n.children.len();
            n.next = Some(next);
            if n.children[next].enabled || next == selected {
                return Ok(());
            }
        }
    }
    pub(crate) fn selected_child(&self, h: &H) -> Result<H, E> {
        match &self.node(h)?.kind {
            Kind::Sequence(n) => n
                .children
                .get(n.selected.ok_or(E::MissingChild)?)
                .map(|c| c.animation.clone())
                .ok_or(E::MissingChild),
            Kind::Selector(n) => n
                .children
                .get(n.selected.ok_or(E::MissingChild)?)
                .map(|c| c.0.clone())
                .ok_or(E::MissingChild),
            _ => Err(E::NodeKind),
        }
    }
    pub(crate) fn set_selector_selected(&mut self, h: &H, name: FastString36) -> Result<(), E> {
        let n = self.selector_mut(h)?;
        if let Some(i) = n.children.iter().position(|c| c.1 == name) {
            n.selected = Some(i);
        }
        Ok(())
    }
    pub(crate) fn set_sequence_start(&mut self, h: &H, index: usize) -> Result<(), E> {
        self.sequence_mut(h)?.selected = Some(index);
        self.sequence_move_next(h)
    }
    pub(crate) fn set_controller_speed(&mut self, h: &H, speed: f32) -> Result<(), E> {
        let children = match &mut self.node_mut(h)?.kind {
            Kind::Sequence(n) => {
                //75dfc BO12/BI25 requires ordered GT, including rejecting NaN.
                if speed.partial_cmp(&f32::from_bits(0x38d1_b717))
                    != Some(std::cmp::Ordering::Greater)
                {
                    return Err(E::InvalidSpeed);
                }
                n.speed = speed;
                n.children
                    .iter()
                    .map(|c| c.animation.clone())
                    .collect::<Vec<_>>()
            }
            Kind::Selector(n) => n.children.iter().map(|c| c.0.clone()).collect(),
            _ => return Err(E::NodeKind),
        };
        for child in children {
            self.set_speed(&child, speed)?;
        }
        Ok(())
    }
    pub(crate) fn set_controller_time(&mut self, h: &H, time: f32) -> Result<(), E> {
        if self.length(h)? == 0.0 {
            return Err(E::ZeroLength);
        }
        let child = self.selected_child(h)?;
        self.set_time(&child, time)
    }
    /// SequenceNode.AddTime82d73598, with child property aliasing preserved.
    pub(crate) fn add_sequence_time(
        &mut self,
        h: &H,
        dt: f32,
        p: &mut AnimationProperties,
    ) -> Result<(), E> {
        let n = self.sequence(h)?;
        let frame = FRAME / n.speed;
        if n.selected == n.next {
            return Err(E::ControllerAssertion("Sequence distinct selected/next"));
        }
        let selected = self.selected_child(h)?;
        if !n.initialized {
            self.set_time(&selected, frame)?;
            self.sequence_mut(h)?.initialized = true;
        }
        self.add_time(&selected, dt, p)?;
        if p.time_to_expire < frame {
            let n = self.sequence(h)?;
            if let Some(next) = n.next {
                let child = n
                    .children
                    .get(next)
                    .ok_or(E::MissingChild)?
                    .animation
                    .clone();
                if n.transitioning {
                    self.add_time(&child, dt, &mut AnimationProperties::default())?;
                } else {
                    self.set_time(&child, frame - p.time_to_expire)?;
                    self.sequence_mut(h)?.transitioning = true;
                }
            }
        }
        if p.expired_or_looped {
            let n = self.sequence_mut(h)?;
            let selected = n.selected.ok_or(E::MissingChild)?;
            n.children.get_mut(selected).ok_or(E::MissingChild)?.enabled = true;
            let child = n.children[selected].animation.clone();
            self.set_time(&child, 0.0)?;
            let next = self.sequence(h)?.next;
            self.sequence_mut(h)?.selected = next;
            self.sequence_move_next(h)?;
            let n = self.sequence(h)?;
            let child = n
                .children
                .get(n.next.ok_or(E::MissingChild)?)
                .ok_or(E::MissingChild)?
                .animation
                .clone();
            self.set_time(&child, 0.0)?;
            self.sequence_mut(h)?.transitioning = false;
        }
        let n = self.sequence(h)?;
        if n.selected == n.next {
            return Err(E::ControllerAssertion("Sequence distinct selected/next"));
        }
        Ok(())
    }
    /// SelectorNode.AddTime82d6ee28: next/end assertion is real even during a
    /// positive transition; do not bypass it to manufacture transition support.
    pub(crate) fn add_selector_time(
        &mut self,
        h: &H,
        dt: f32,
        p: &mut AnimationProperties,
    ) -> Result<(), E> {
        let n = self.selector(h)?;
        if n.next.is_some() {
            return Err(E::ControllerAssertion("Selector next must equal end"));
        }
        if n.transitioning {
            return Err(E::MissingChild);
        }
        let child = self.selected_child(h)?;
        self.add_time(&child, dt, p)
    }
}
