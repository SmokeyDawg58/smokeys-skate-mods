//! Shared generated TemplateHC selection, used by Action and Pedestrian.
//! Pedestrian GetNextState82934be0/Evaluate82934390 match the same template;
//! only hierarchy capacity differs (Action8, Pedestrian9). No leaf predicates
//! are shared here. Error envelope remains the existing ActionUpdateError.
use crate::generated_action_load::{NULL_INDEX, StateDeclaration, TransitionDeclaration};
use crate::generated_action_load::runtime::ActionUpdateError as Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mask {
    Pre,
    Sustain,
}

pub(crate) struct Graph<'a, const DEPTH: usize> {
    pub(crate) states: &'a [StateDeclaration],
    pub(crate) transitions: &'a [TransitionDeclaration],
    pub(crate) indices: &'a [u16],
    pub(crate) current: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Selected {
    pub(crate) state: u16,
    pub(crate) transition: u16,
}

impl<const DEPTH: usize> Graph<'_, DEPTH> {
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

    /// `SetAncestors829291d8`: `IndexSet` insertion orders numeric indices.
    pub(crate) fn ancestors(&self, mut state: u16) -> Result<Vec<u16>, Error> {
        let mut result = Vec::with_capacity(DEPTH);
        while state != NULL_INDEX {
            if result.len() == DEPTH || result.contains(&state) {
                return Err(Error::Capacity);
            }
            result.push(state);
            state = self.state(state)?.parent;
        }
        result.sort_unstable();
        Ok(result)
    }

    /// `StateHasDescendent829283b0` walks the FIRST argument's parents,
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

    /// `GetStateParentActivation<2>82929618`: child before parent, lazy.
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

    /// `Evaluate82929c08`: sustain only the current ancestry; siblings use Pre.
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

    /// `ValidTarget829296b0`: target always uses Pre, even a self transition.
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

    /// `SearchTransitions8292a020/82929e58`: full leaf-to-root search per
    /// priority, then restart from current for the next lower priority.
    pub(crate) fn search(
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

    /// `GetNextState82951138` and `EvaluateState82929d50`.
    pub(crate) fn next(
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

    /// `UpdateLists82952068`, `FindCommonAncestors829292f0` and
    /// `IndexSet<8>::intersect82928c30`: pivot is an exclusive numeric limit.
    pub(crate) fn changes(&self, selected: &Selected) -> Result<(Vec<u16>, Vec<u16>), Error> {
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

