//! `Pedestrian::RemapAnim82e8d8e0` attribute selection on the retained Vault.
use super::{
    AttributeRuntimeData, AttributeRuntimeDataOrigin, AttributeRuntimeError,
    AttributeRuntimeInstance,
};

/// Full type identity assembled by `82e8d9d8..82e8d9f4`; PDB size 56.
const ANIMATION_ATTRIBUTES_TYPE: u64 = 0x5088_a439_d663_388c;

/// PDB Anim::tAnimAttributes, numeric suffix after the relocated Clip pointer.
#[derive(Clone, Copy)]
pub(crate) struct PedestrianBranchWindow {
    pub(crate) start: f32,
    pub(crate) end: f32,
    pub(crate) foot: i32,
}
#[derive(Clone, Copy)]
pub(crate) struct PedestrianAnimAttributes {
    pub(crate) foot_usage: u32,
    pub(crate) start_foot: i32,
    pub(crate) end_foot: i32,
    pub(crate) branch_count: i32,
    pub(crate) branches: [PedestrianBranchWindow; 3],
}

/// PDB tLocomotionSpeedIntentMapping; original Vault type fdff00a501b1e0f7.
pub(crate) struct PedestrianSpeedMapping {
    pub(crate) velocity: f32,
    pub(crate) major: String,
    pub(crate) minor: String,
    pub(crate) allowed_previous: [String; 5],
}

impl AttributeRuntimeInstance {
    /// Pedestrian::RegisterAnimAttributes82e79ba8: a valid attribute of another
    /// type clears the registered pointer. Index zero is used without RNG.
    pub(crate) fn try_pedestrian_anim_attributes(&self, key: u64)
        -> Result<Option<PedestrianAnimAttributes>, AttributeRuntimeError> {
        let Some((class_key, class, _, node)) = self.resolved_node(key)? else { return Ok(None); };
        if node.flags & 0x80 == 0 { return Ok(None); }
        let definition = class.definitions.get(&key).ok_or(AttributeRuntimeError::MissingDefinition {
            class: class_key, key,
        })?;
        if definition.type_id != ANIMATION_ATTRIBUTES_TYPE { return Ok(None); }
        let Some(data) = self.try_array_field_data(key, 0, ANIMATION_ATTRIBUTES_TYPE, 56, 4..56)?
            else { return Ok(None); };
        let word = |offset: usize| u32::from_be_bytes(data.bytes()[offset..offset+4].try_into().expect("numeric attribute word"));
        let branch = |offset| PedestrianBranchWindow { start: f32::from_bits(word(offset)), end: f32::from_bits(word(offset+4)), foot: word(offset+8) as i32 };
        Ok(Some(PedestrianAnimAttributes { foot_usage: word(0), start_foot: word(4) as i32,
            end_foot: word(8) as i32, branch_count: word(12) as i32,
            branches: [branch(16), branch(28), branch(40)] }))
    }
    /// Original UpdateBaseLocomotion82e8e7e0 reads authored order, not sorted
    /// thresholds. Pointer-bearing strings use the shared Vault relocation path.
    pub(crate) fn try_pedestrian_speed_mappings(&self) -> Result<Vec<PedestrianSpeedMapping>, String> {
        const KEY: u64 = 0x9c9a_4f2c_b1cc_45de;
        const TYPE: u64 = 0xfdff_00a5_01b1_e0f7;
        let count = self.try_length(KEY).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for index in 0..count {
            let bytes = self.try_array_field_data(KEY, index, TYPE, 32, 0..4)
                .map_err(|e| e.to_string())?.ok_or("pedestrian speed mapping missing")?;
            let velocity = f32::from_be_bytes(bytes.bytes().try_into()
                .map_err(|_| "pedestrian speed mapping scalar size")?);
            let text = |offset| -> Result<String, String> {
                let data = self.try_array_text_field(KEY, index, TYPE, 32, offset)
                    .map_err(|e| e.to_string())?.ok_or("pedestrian speed mapping null text")?;
                Ok(std::str::from_utf8(data.bytes()).map_err(|e| e.to_string())?.to_owned())
            };
            result.push(PedestrianSpeedMapping { velocity, major: text(4)?, minor: text(8)?,
                allowed_previous: [text(12)?, text(16)?, text(20)?, text(24)?, text(28)?] });
        }
        Ok(result)
    }
    /// Resolve the authored animation Clip after the caller's registered
    /// per-pedestrian override lookup. `None` means no valid attribute and the
    /// original semantic name must be retained. A null pointer or empty array is
    /// an error, not a second attempt using the original name.
    ///
    /// `random` must draw from the SAME living-world RNG as population/model
    /// selection. It is called exactly once for an array length greater than 1.
    pub(crate) fn try_pedestrian_animation_clip(
        &self,
        key: u64,
        mut random: impl FnMut() -> u32,
    ) -> Result<Option<AttributeRuntimeData>, AttributeRuntimeError> {
        use AttributeRuntimeError as E;
        let Some((class_key, class, collection, node)) = self.resolved_node(key)? else {
            return Ok(None);
        };
        if node.flags & 0x80 == 0 {
            return Ok(None);
        }
        let definition = class.definitions.get(&key).ok_or(E::MissingDefinition {
            class: class_key,
            key,
        })?;
        let expected_size = if definition.type_id == ANIMATION_ATTRIBUTES_TYPE {
            56
        } else {
            4
        };
        let size = usize::from(definition.size);
        if size != expected_size {
            return Err(E::PayloadSize {
                key,
                expected: expected_size,
                actual: size,
            });
        }
        let index = if definition.flags & 1 != 0 {
            let length = self.try_length(key)?;
            if length > 1 { random() % length } else { 0 }
        } else {
            0
        };
        let projection = super::text::Projection::TextField { offset: 0 };
        let result = if node.flags & 2 == 0 {
            self.node_data(class_key, class, collection, node, key, projection)?
        } else {
            if node.type_key != Some(definition.type_id) {
                return Err(E::NodeTypeMismatch {
                    key,
                    node_type: node.type_key.unwrap_or(0),
                    definition_type: definition.type_id,
                });
            }
            if node.flags & 8 != 0 || definition.flags & 0x20 != 0 {
                return Err(E::UnsupportedConstructed { key });
            }
            // Match existing typed array readers' supported stored-pointer
            // domain. Other storage forms require their original owner path.
            if node.flags & 0x70 != 0 {
                return Err(E::UnsupportedArray { key });
            }
            let header = self.array_range(class_key, key, node, 0, 8)?;
            let half = |offset| {
                u16::from_be_bytes(
                    header.bytes()[offset..offset + 2]
                        .try_into()
                        .expect("two-byte header"),
                )
            };
            if index >= u32::from(half(2)) {
                return Err(E::UnsupportedArray { key });
            }
            let stride = usize::from(half(4));
            if stride != size {
                return Err(E::PayloadSize {
                    key,
                    expected: size,
                    actual: stride,
                });
            }
            let relative = 8 + usize::from((half(6) >> 12) & 8) + index as usize * stride;
            let location = self
                .vault
                .pointer(node.stored_word.expect("explicit array pointer"))?
                .ok_or(E::UnsupportedArray { key })?;
            self.vault.project_data(
                location,
                relative,
                size,
                AttributeRuntimeDataOrigin::StoredPointer,
                projection,
            )?
        };
        result.map(Some).ok_or(E::UnsupportedArray { key })
    }
}
