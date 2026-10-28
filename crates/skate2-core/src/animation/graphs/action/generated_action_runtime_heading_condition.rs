//! Actual condition98 finishes with existing `EvalCompare`.
use super::{ActionUpdateError, ActorConstruction, ActorIdentity, BoundMotionGraphContext};
use crate::generated_action_load::construct::Comparison;

pub(super) fn evaluate_heading_condition(
    actor: &ActorConstruction,
    context: &BoundMotionGraphContext<ActorIdentity>,
    comparison: Comparison,
) -> Result<bool, ActionUpdateError> {
    let value = actor.action_runtime_heading_adjust_factor(context)?;
    Ok(super::super::conditions::compare(comparison, value))
}
