//! HoM entity collision scoring82df1a20/1c58/1f70, retained EntityScorer owners.
use super::{
    super::scoring::EntitySet,
    scoring_attributes::ScoreError,
    scoring_entity_types,
    scoring_external::ScoringExternalInput,
    wipeout_score::{EntityScorer, WipeoutScorer},
};
use crate::attribute_runtime::AttributeRuntimeVault;
use std::sync::Arc;

impl EntityScorer {
    pub(super) fn reset_tracker(&mut self) -> Result<(), ScoreError> {
        let prefix = self
            .collided_entities
            .get_mut(..self.count as usize)
            .ok_or(ScoreError::Invalid("wipeout entity count"))?;
        // Native Reset clears only the count prefix, preserving inactive data.
        for entry in prefix {
            entry.reset();
        }
        self.count = 0;
        self.tracker_score = 0;
        self.score = 0;
        self.num_new_collisions_this_frame = 0;
        Ok(())
    }
    fn contains(&self, id: u32) -> Result<bool, ScoreError> {
        Ok(self
            .collided_entities
            .get(..self.count as usize)
            .ok_or(ScoreError::Invalid("wipeout entity count"))?
            .iter()
            .any(|entry| entry.unique_id == id))
    }
    fn add(&mut self, id: u32, kind: i32, points: u32) {
        // Capacity exhaustion is an ordinary native false return, not an error.
        if let Some(entry) = self.collided_entities.get_mut(self.count as usize) {
            entry.unique_id = id;
            entry.entity_type = kind;
            entry.score = points;
            self.count += 1;
            self.score = self.score.wrapping_add(points);
            self.num_new_collisions_this_frame = self.num_new_collisions_this_frame.wrapping_add(1);
        }
    }
    fn entity_points(&self, kind: i32, vehicle: bool) -> Result<u32, ScoreError> {
        let (list, default) = if vehicle {
            (0xff1c_ed9b_60b2_8cda, 0xfc8a_190e_55a1_0559)
        } else {
            (0x9e62_0977_a93d_37ba, 0x9627_6c72_10f1_fdd2)
        };
        for i in 0..self.attributes.try_length(list)? {
            // GetPointValueFromEntityType82decde8/82decec8 reads an
            // eight-byte (entity type, points) array record. Both authored
            // tables share type d215ad904e7f2356; scalar GetData rejects them.
            let entry = self.attributes.try_plain_array_data(
                list, i, 0xd215_ad90_4e7f_2356, 8,
            )?;
            let bytes = match entry.as_ref() {
                Some(value) => value
                    .bytes()
                    .get(..8)
                    .ok_or(ScoreError::Invalid("entity score entry"))?,
                None => &[0; 8],
            };
            if i32::from_be_bytes(bytes[..4].try_into().unwrap()) == kind {
                return Ok(u32::from_be_bytes(bytes[4..8].try_into().unwrap()));
            }
        }
        super::scoring_attributes::word(&self.attributes, default)
    }
    pub(super) fn fill_entity_output(&self, out: &mut EntitySet) -> Result<(), ScoreError> {
        let count = self.count as usize;
        if count > 8 {
            return Err(ScoreError::Invalid("wipeout entity count"));
        }
        out.count = self.count;
        out.score = self.score;
        out.num_new_collisions_this_frame = self.num_new_collisions_this_frame;
        out.collided_entities[..count].clone_from_slice(&self.collided_entities[..count]);
        Ok(())
    }
}
impl WipeoutScorer {
    pub(super) fn update_entities(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: &ScoringExternalInput,
    ) -> Result<(), ScoreError> {
        let pedestrian = self
            .pedestrian
            .as_mut()
            .ok_or(ScoreError::Missing("pedestrian scorer"))?;
        pedestrian.num_new_collisions_this_frame = 0;
        for entity in &input.pedestrians {
            if entity.colliding && !pedestrian.contains(entity.unique_id)? {
                let kind = scoring_entity_types::pedestrian_or_vehicle_type(vault, entity.kind)?;
                let points = pedestrian.entity_points(kind, false)?;
                pedestrian.add(entity.unique_id, kind, points);
            }
        }
        let vehicle = self
            .vehicle
            .as_mut()
            .ok_or(ScoreError::Missing("vehicle scorer"))?;
        vehicle.num_new_collisions_this_frame = 0;
        for entity in &input.vehicles {
            if entity.colliding && !vehicle.contains(entity.unique_id)? {
                let kind = scoring_entity_types::pedestrian_or_vehicle_type(vault, entity.kind)?;
                let points = vehicle.entity_points(kind, true)?;
                vehicle.add(entity.unique_id, kind, points);
            }
        }
        let dmo = self
            .dmo
            .published_value_mut()
            .ok_or(ScoreError::Missing("DMO scorer"))?;
        dmo.base.num_new_collisions_this_frame = 0;
        for entity in &input.dmos {
            if entity.colliding && !dmo.base.contains(entity.unique_id)? {
                let key = scoring_entity_types::dmo_key(vault, entity.kind)?;
                let kind = scoring_entity_types::scoring_type(vault, key)?;
                dmo.base.add(
                    entity.unique_id,
                    kind,
                    dmo.default_score
                        .ok_or(ScoreError::Missing("DMO scoring default"))?,
                );
            }
        }
        Ok(())
    }
}
