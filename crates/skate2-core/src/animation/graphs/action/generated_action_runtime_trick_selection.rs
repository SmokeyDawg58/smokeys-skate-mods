//! Actual Trick children: Pre828ff990 and Sustain82906048 expressions91..93.
use super::{ActionUpdateError as Error, selection::Mask};
use crate::generated_action_load::ids::{condition as cond, expression as expr};

pub(super) fn expression(
    mask: Mask,
    index: u16,
    mut condition: impl FnMut(u16) -> Result<bool, Error>,
) -> Result<bool, Error> {
    let gesture = match index {
        expr::TRICK_FROM_SQUARE_52 => cond::HAS_GESTURE_INTENT_SQUARE,
        expr::TRICK_FROM_TAIL_53 => cond::HAS_GESTURE_INTENT_TAIL,
        expr::TRICK_FROM_NOSE_54 => cond::HAS_GESTURE_INTENT_NOSE,
        _ => {
            return Err(match mask {
                Mask::Pre => Error::PendingExpression(index),
                Mask::Sustain => Error::PendingMaskedExpression {
                    mask,
                    expression: index,
                },
            });
        }
    };
    Ok(condition(gesture)?
        || (condition(cond::HAS_AG_INTENT_HOLD_PATTERN)?
            && (condition(cond::IN_PARENT_STATE_FOR_TIME_LESS_0_4)?
                || condition(cond::PHYS_FILTERED_STATE_AIR)?)))
}

#[cfg(test)]
#[path = "generated_action_runtime_trick_selection_tests.rs"]
mod tests;
