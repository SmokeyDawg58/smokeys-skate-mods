//! Nursary inline UpdateScoreThreshold (ground82df0480..04e0).
use super::{super::scoring::Thresholds, scoring_attributes::ScoreError};
use crate::attribute_runtime::AttributeRuntimeInstance;

pub(super) fn update(
    general: &AttributeRuntimeInstance,
    thresholds: &mut Thresholds,
    index: usize,
    value: f32,
) -> Result<(), ScoreError> {
    let data = general
        .try_cached_layout_field(index * 12, 12)?
        .ok_or(ScoreError::Missing("big trick threshold layout"))?;
    let bytes = data.bytes();
    let source_id = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
    if source_id as usize != index {
        return Err(ScoreError::Invalid("big trick threshold ID"));
    }
    let low = f32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let high = f32::from_be_bytes(bytes[8..12].try_into().unwrap());
    let out = thresholds
        .score_thresholds
        .get_mut(index)
        .ok_or(ScoreError::Invalid("big trick threshold index"))?;
    // Achieved flags are recomputed from THIS value, not the retained maximum.
    if !(out.value - value >= 0.0) {
        out.value = value;
    }
    out.was_achieved = [value > low, value > high];
    Ok(())
}
