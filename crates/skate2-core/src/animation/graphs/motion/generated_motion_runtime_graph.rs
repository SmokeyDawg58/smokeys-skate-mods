//! Source Motion selection, independent of callback storage but used by the
//! retained controller with its actual Actor conditions and shared cache.
use super::MotionUpdateError as Error;
use crate::generated_action_load::{NULL_INDEX, StateDeclaration, TransitionDeclaration};
#[cfg(test)]
#[path = "generated_motion_runtime_graph_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mask {
    Pre,
    Sustain,
}

pub(super) struct Graph<'a> {
    pub(super) states: &'a [StateDeclaration],
    pub(super) transitions: &'a [TransitionDeclaration],
    pub(super) indices: &'a [u16],
    pub(super) current: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Selected {
    pub(super) state: u16,
    pub(super) transition: u16,
}

impl Graph<'_> {
    fn state(&self, state: u16) -> Result<&StateDeclaration, Error> {
        self.states
            .get(usize::from(state))
            .ok_or(Error::InvalidState(state))
    }

    fn transition(&self, index: u16) -> Result<&TransitionDeclaration, Error> {
        self.transitions
            .get(usize::from(index))
            .ok_or(Error::InvalidTransition(index))
    }

    /// `SetAncestors8292b528`: `IndexSet` insertion orders numeric indices.
    pub(super) fn ancestors(&self, mut state: u16) -> Result<Vec<u16>, Error> {
        let mut result = Vec::with_capacity(11);
        while state != NULL_INDEX {
            if result.len() == 11 || result.contains(&state) {
                return Err(Error::Capacity);
            }
            result.push(state);
            state = self.state(state)?.parent;
        }
        result.sort_unstable();
        Ok(result)
    }

    /// `StateHasDescendent8292a680` walks the FIRST argument's parents,
    /// including itself. Native argument names are misleading.
    fn descends_from(&self, mut leaf: u16, ancestor: u16) -> Result<bool, Error> {
        while leaf != NULL_INDEX {
            if leaf == ancestor {
                return Ok(true);
            }
            leaf = self.state(leaf)?.parent;
        }
        Ok(false)
    }

    fn active(
        &self,
        state: u16,
        mask: Mask,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let index = self.state(state)?.expression;
        if index == NULL_INDEX {
            Ok(true)
        } else {
            expression(mask, index, state)
        }
    }

    /// `GetStateParentActivation<2>8292b968`: child before parent, lazy.
    fn sustain_parents(
        &self,
        mut state: u16,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        loop {
            if !self.active(state, Mask::Sustain, expression)? {
                return Ok(false);
            }
            state = self.state(state)?.parent;
            if state == NULL_INDEX {
                return Ok(true);
            }
        }
    }

    /// `Evaluate8292bf58`: sustain only the current ancestry; siblings use Pre.
    fn evaluate(
        &self,
        state: u16,
        mask: Mask,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<u16, Error> {
        let actual_mask = if mask == Mask::Sustain && self.descends_from(self.current, state)? {
            Mask::Sustain
        } else {
            Mask::Pre
        };
        if !self.active(state, actual_mask, expression)? {
            return Ok(NULL_INDEX);
        }
        let children = self.state(state)?.children;
        if children.count == 0 {
            return Ok(state);
        }
        for child in children.index..children.index + children.count {
            if self.state(child)?.active {
                let selected = self.evaluate(child, mask, expression)?;
                if selected != NULL_INDEX {
                    return Ok(selected);
                }
            }
        }
        Ok(NULL_INDEX)
    }

    /// `ValidTarget8292ba00`: target always uses Pre, even a self transition.
    fn valid_target(
        &self,
        target: u16,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let current = self.ancestors(self.current)?;
        let mut state = target;
        while state != NULL_INDEX {
            if state != target && current.contains(&state) {
                return self.sustain_parents(state, expression);
            }
            if !self.active(state, Mask::Pre, expression)? {
                return Ok(false);
            }
            state = self.state(state)?.parent;
        }
        Ok(true)
    }

    /// `SearchTransitions8292c370/8292c1a8`: full leaf-to-root search per
    /// priority, then restart from current for the next lower priority.
    fn search(
        &self,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<Option<Selected>, Error> {
        for priority in (0..=3).rev() {
            let mut state = self.current;
            while state != NULL_INDEX {
                let section = self.state(state)?.transitions;
                for position in section.index..section.index + section.count {
                    let index = *self
                        .indices
                        .get(usize::from(position))
                        .ok_or(Error::InvalidActiveSet)?;
                    let transition = self.transition(index)?;
                    if transition.priority != priority {
                        continue;
                    }
                    if transition.expression != NULL_INDEX
                        && !expression(Mask::Pre, transition.expression, state)?
                    {
                        continue;
                    }
                    if !self.valid_target(transition.target, expression)? {
                        continue;
                    }
                    let selected = self.evaluate(transition.target, Mask::Pre, expression)?;
                    if selected != NULL_INDEX {
                        return Ok(Some(Selected {
                            state: selected,
                            transition: index,
                        }));
                    }
                }
                state = self.state(state)?.parent;
            }
        }
        Ok(None)
    }

    /// `GetNextState829511b0` and `EvaluateState8292c0a0`.
    pub(super) fn next(
        &self,
        expression: &mut impl FnMut(Mask, u16, u16) -> Result<bool, Error>,
    ) -> Result<Selected, Error> {
        if self.current == NULL_INDEX {
            return Ok(Selected {
                state: self.evaluate(0, Mask::Pre, expression)?,
                transition: NULL_INDEX,
            });
        }
        if let Some(selected) = self.search(expression)? {
            return Ok(selected);
        }
        let current = self.state(self.current)?;
        match current.interrupt_type {
            0 => {
                if self.sustain_parents(self.current, expression)? {
                    return Ok(Selected {
                        state: self.current,
                        transition: NULL_INDEX,
                    });
                }
            }
            1 => (),
            2 => {
                if self.sustain_parents(current.interrupt_target, expression)? {
                    let selected =
                        self.evaluate(current.interrupt_target, Mask::Sustain, expression)?;
                    if selected != NULL_INDEX {
                        return Ok(Selected {
                            state: selected,
                            transition: NULL_INDEX,
                        });
                    }
                }
            }
            other => return Err(Error::InvalidInterruptType(other)),
        }
        Ok(Selected {
            state: self.evaluate(0, Mask::Sustain, expression)?,
            transition: NULL_INDEX,
        })
    }

    /// `UpdateLists829521e8`, `FindCommonAncestors8292b640` and
    /// `IndexSet<11>::intersect8292b008`: pivot is an exclusive numeric limit.
    pub(super) fn changes(&self, selected: &Selected) -> Result<(Vec<u16>, Vec<u16>), Error> {
        let target = if selected.transition == NULL_INDEX {
            NULL_INDEX
        } else {
            self.transition(selected.transition)?.target
        };
        let pivot = if target != NULL_INDEX && self.descends_from(target, self.current)? {
            self.state(target)?.parent
        } else {
            target
        };
        let old = self.ancestors(self.current)?;
        let new = self.ancestors(selected.state)?;
        let common = old
            .iter()
            .zip(&new)
            .take_while(|(a, b)| a == b && **a < pivot)
            .count();
        Ok((old[common..].to_vec(), new[common..].to_vec()))
    }
}
