//! Native Action condition bodies with concrete intent storage.
//! Callers must resolve and verify the actual Actor UCOM occurrence before
//! passing its retained map. No external boolean predicate substitutes for it.
use super::construct::{Comparison, ComparisonOp};
use crate::{actor_graph_construction::ActorGraphIntents, play_animation::FastString36};
use std::cmp::Ordering;
#[cfg(test)]
#[path = "generated_action_conditions_tests.rs"]
mod tests;

/// `EvalCompare82ceeea0`. Raw BO/BI matters for unordered operands:
/// `GreaterEqual` uses NOT LT (4098ff98), `LessEqual` uses NOT GT (4099ff7c).
/// Thus both return true for NaN; ordinary Rust >=/<= would be incorrect.
pub(crate) fn compare(comparison: Comparison, value: f32) -> bool {
    let value = if comparison.absolute {
        value.abs()
    } else {
        value
    };
    let ordering = value.partial_cmp(&comparison.reference);
    match comparison.op {
        ComparisonOp::Equal => ordering == Some(Ordering::Equal),
        ComparisonOp::NotEqual => ordering != Some(Ordering::Equal),
        ComparisonOp::Greater => ordering == Some(Ordering::Greater),
        ComparisonOp::Less => ordering == Some(Ordering::Less),
        ComparisonOp::GreaterEqual => ordering != Some(Ordering::Less),
        ComparisonOp::LessEqual => ordering != Some(Ordering::Greater),
        ComparisonOp::Nop => false,
    }
}

/// `HasAGIntent82cc1748`: actual map presence is tested first; NOP then returns
/// true even for a stored zero/NaN. Missing intent never invokes comparison.
pub(super) fn has_action_intent(
    comparison: Comparison,
    name: &FastString36,
    intents: &ActorGraphIntents,
) -> bool {
    let Some(value) = intents.get(name) else {
        return false;
    };
    comparison.op == ComparisonOp::Nop || compare(comparison, value)
}

/// `InParentStateForTime82c8c950` calls the retained controller's time query.
/// `parent` is the factory object's just-written state tag, not the state's
/// declaration parent. Missing time is the native -1 sentinel.
pub(crate) fn parent_time_condition<E>(
    times: &std::collections::BTreeMap<u16, f32>,
    state_count: usize,
    parent: Option<u16>,
    comparison: Comparison,
    invalid_state: impl FnOnce(u16) -> E,
) -> Result<bool, E> {
    let time = match parent {
        None | Some(u16::MAX) => -1.0,
        Some(index) => {
            if usize::from(index) >= state_count {
                return Err(invalid_state(index));
            }
            times.get(&index).copied().unwrap_or(-1.0)
        }
    };
    // Native BLT rejects ordered negatives; unordered proceeds to EvalCompare.
    Ok(time.partial_cmp(&0.0) != Some(std::cmp::Ordering::Less) && compare(comparison, time))
}
