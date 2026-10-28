//! GlobalMetrics original nearby-context consumers82deebd8/df3fe0/def040.
use super::{
    super::{scoring::Scoring2, simple::GroundRemainder},
    scoring_attributes::ScoreError,
    scoring_context_geometry::{UnderOver, above_dmo, over_points, project_to_basis},
    scoring_entity_types,
    scoring_external::ScoringExternalInput,
    sequence::Sequence,
};
use crate::{
    attribute_runtime::AttributeRuntimeVault, physics::FilteredPhysicsState as F,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

pub(super) struct ContextFrame<'a> {
    pub ground: &'a GroundRemainder,
    pub filtered: F,
    pub damped_com: SkeletonRootVector,
    pub pedestrian_radius: f32,
    pub skater_radius: f32,
    pub score_invalid_dmos: bool,
}
fn character_context(
    f: &ContextFrame<'_>,
    a: SkeletonRootVector,
    b: SkeletonRootVector,
    radius: f32,
) -> Result<UnderOver, ScoreError> {
    let on_ground = matches!(f.filtered, F::Ground | F::Grind);
    let mut result = UnderOver::Neither;
    if !on_ground {
        result = over_points(
            f.ground.below_hips_ground_data_start_pos,
            a,
            b,
            radius * radius,
        )?;
    }
    if (on_ground || f.filtered == F::Air) && result == UnderOver::Neither {
        result = over_points(
            f.ground.below_deck_ground_data_start_pos,
            a,
            b,
            radius * radius,
        )?;
    }
    Ok(result)
}
fn slot(result: UnderOver) -> Option<usize> {
    match result {
        UnderOver::Over => Some(0),
        UnderOver::Under => Some(1),
        UnderOver::Neither => None,
    }
}

pub(super) fn collect(
    vault: &Arc<AttributeRuntimeVault>,
    input: &ScoringExternalInput,
    f: &ContextFrame<'_>,
    sequence: &mut Sequence,
    out: &mut Scoring2,
) -> Result<(), ScoreError> {
    // Native caller resets all context counts before this ordered population.
    for vehicle in &input.vehicles {
        let columns = vehicle.transform.columns();
        let center = columns[1].zip(columns[3], |up, pos| {
            up.mul_add(f32::from_bits(0x3f02_8f5c), pos)
        });
        let local = project_to_basis(&vehicle.transform, f.damped_com.zip(center, |a, b| a - b));
        let over = local[1] > 0.0;
        let margin = if over {
            0.0
        } else {
            f32::from_bits(0x3e99_999a)
        };
        if (vehicle.length * 0.5) - margin > local[2].abs()
            && (vehicle.width * 0.5) - margin > local[0].abs()
        {
            let kind = scoring_entity_types::pedestrian_or_vehicle_type(vault, vehicle.kind)?;
            out.current_context_data.context_sensitive_data[1].uid_data[if over { 0 } else { 1 }]
                .add_uid(vehicle.unique_id, kind)?;
        }
    }
    for ped in &input.pedestrians {
        // Native supplies legs first, torso second; endpoint order matters when
        // both horizontal cylinders overlap the actor at different heights.
        if let Some(index) = slot(character_context(
            f,
            ped.legs,
            ped.torso,
            f.pedestrian_radius,
        )?) {
            let kind = scoring_entity_types::pedestrian_or_vehicle_type(vault, ped.kind)?;
            out.current_context_data.context_sensitive_data[0].uid_data[index]
                .add_uid(ped.unique_id, kind)?;
        }
    }
    for skater in &input.skaters {
        if let Some(index) = slot(character_context(
            f,
            skater.upper_body,
            skater.lower_body,
            f.skater_radius,
        )?) {
            out.current_context_data.context_sensitive_data[2].uid_data[index]
                .add_uid(skater.unique_id, 8)?;
        }
    }
    let on_ground = matches!(f.filtered, F::Ground | F::Grind);
    let deck_valid =
        f.ground.below_deck_ground_data_is_valid && (on_ground || f.filtered == F::Air);
    let hips_valid = f.ground.below_hips_ground_data_is_valid && !on_ground;
    for object in &input.dmos {
        let scoring_key = scoring_entity_types::dmo_key(vault, object.kind)?;
        let kind = scoring_entity_types::scoring_type(vault, scoring_key)?;
        if !f.score_invalid_dmos && kind == -1 {
            continue;
        }
        let mut result = UnderOver::Neither;
        if hips_valid {
            result = above_dmo(
                f.ground.below_hips_ground_data_start_pos,
                f.ground.below_hips_ground_data_position,
                object,
            );
        }
        if result == UnderOver::Neither && deck_valid {
            result = above_dmo(
                f.ground.below_deck_ground_data_start_pos,
                f.ground.below_deck_ground_data_position,
                object,
            );
        }
        if result == UnderOver::Neither && !object.skater_is_grinding {
            continue;
        }
        let index = if object.skater_is_grinding {
            Some(0)
        } else {
            slot(result)
        };
        if let Some(index) = index {
            out.current_context_data.context_sensitive_data[3].uid_data[index]
                .add_uid(object.unique_id, kind)?;
        }
        if object.skater_is_grinding {
            let info = &mut out.current_frame_grind_object_info;
            info.entity_key = scoring_key;
            info.entity_type = kind;
            info.unique_id = object.unique_id;
            info.is_valid = true;
        }
        if (14..=30).contains(&kind) {
            sequence.dynamic_objects_used[(kind - 14) as usize] = true;
        }
    }
    for object in &input.static_objects {
        if !object.valid {
            continue;
        }
        let kind = scoring_entity_types::static_object_type(vault, object.kind)?;
        let unique_id = kind.wrapping_add(2) as u32;
        let list = &mut out.current_context_data.context_sensitive_data[4].uid_data[0];
        let active = list
            .uids
            .get(..list.count as usize)
            .ok_or(ScoreError::Invalid("static context count"))?;
        if !active.iter().any(|v| v.unique_id == unique_id) {
            list.add_uid(unique_id, kind)?;
        }
    }
    Ok(())
}
