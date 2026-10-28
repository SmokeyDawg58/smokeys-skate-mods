//! Native scoring entity attribution82dee8b0/e960/ea28/eb08.
use super::scoring_attributes::ScoreError;
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    living_world_grind_services::{dmo_models::CHARACTERISTICS_CLASS, dmo_objects::ENTITIES_CLASS},
};
use std::sync::Arc;
pub(super) const SCORING_ENTITIES_CLASS: u64 = 0xf4c9_f7ff_78b8_f653;
const SCORING_REFERENCE: u64 = 0xe356_ed00_abf1_d7f0;
const ENTITY_TYPE: u64 = 0x8b67_0c50_4a36_3a7c;

pub(super) fn scoring_type(
    vault: &Arc<AttributeRuntimeVault>,
    key: u64,
) -> Result<i32, ScoreError> {
    // Native cmpdi checks the full64-bit sentinel, despite decompiler HIDWORD.
    if key == u64::MAX {
        return Ok(-1);
    }
    let value = vault.instance(SCORING_ENTITIES_CLASS, key);
    if value.is_null() {
        return Ok(-1);
    }
    entity_type_value(&value)
}
fn entity_type_value(
    value: &crate::attribute_runtime::AttributeRuntimeInstance,
) -> Result<i32, ScoreError> {
    let Some(data) = value.try_get_data(ENTITY_TYPE, 0)? else {
        return Ok(0);
    };
    Ok(i32::from_be_bytes(
        data.bytes()
            .get(..4)
            .ok_or(ScoreError::Invalid("entity type width"))?
            .try_into()
            .unwrap(),
    ))
}
pub(super) fn dmo_key(vault: &Arc<AttributeRuntimeVault>, key: u64) -> Result<u64, ScoreError> {
    if key == u64::MAX {
        return Ok(u64::MAX);
    }
    let value = vault.instance(CHARACTERISTICS_CLASS, key);
    if value.is_null() {
        return Ok(u64::MAX);
    }
    Ok(value
        .try_ref_spec_collection(SCORING_REFERENCE)?
        .unwrap_or(0))
}
pub(super) fn pedestrian_or_vehicle_type(
    vault: &Arc<AttributeRuntimeVault>,
    key: u64,
) -> Result<i32, ScoreError> {
    if key == u64::MAX {
        return Ok(-1);
    }
    let value = vault.instance(ENTITIES_CLASS, key);
    if value.is_null() {
        return Ok(-1);
    }
    let key = value
        .try_ref_spec_collection(SCORING_REFERENCE)?
        .unwrap_or(0);
    scoring_type(vault, key)
}
pub(super) fn static_object_type(
    vault: &Arc<AttributeRuntimeVault>,
    key: u64,
) -> Result<i32, ScoreError> {
    if key == u64::MAX {
        return Ok(-1);
    }
    let object = vault.instance(0x4ef1_6277_cb3b_0f0a, key);
    if object.is_null() {
        return Ok(-1);
    }
    let data = object
        .try_cached_layout_field(32, 8)?
        .ok_or(ScoreError::Missing("static object scoring layout"))?;
    let key = u64::from_be_bytes(
        data.bytes()
            .try_into()
            .map_err(|_| ScoreError::Invalid("static scoring reference"))?,
    );
    // Unlike the pedestrian/DMO helper, native has no null-collection guard
    // after constructing scoring_entities here; lookup uses DefaultDataArea.
    entity_type_value(&vault.instance(SCORING_ENTITIES_CLASS, key))
}
