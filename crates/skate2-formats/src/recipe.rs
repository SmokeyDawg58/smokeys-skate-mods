//! Version-7 character recipes used by the native character loader.
//!
//! This is the byte-stream written by `Character::CharacterEntity::Pack` and
//! read by `Character::CharacterEntity::Unpack`. Integers are big-endian and
//! strings are a 32-bit byte count followed immediately by UTF-8 bytes; they
//! are not individually aligned.

use std::fmt;

use crate::hash::rw_hash64_string;

pub const CHARACTER_RECIPE_VERSION: u32 = 7;
pub mod customizations;
mod encode;
pub use customizations::CharacterCustomizations;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterRecipe {
    pub version: u32,
    pub schema_name: String,
    pub category: u32,
    pub memory_category: u32,
    pub clone_type: u32,
    pub components: Vec<CharacterComponent>,
    pub has_customizations: bool,
    pub customizations: Option<CharacterCustomizations>,
    /// The size recorded immediately after the aligned recipe body.
    pub packed_size: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterComponent {
    pub name: String,
    pub models: Vec<CharacterModel>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterModel {
    pub asset_id: u64,
    pub lods: Vec<CharacterLod>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterLod {
    pub level: u8,
    pub asset_id: u64,
    pub arena_asset_id: u64,
    pub material_instances: Vec<CharacterMaterialInstance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterMaterialInstance {
    pub materials: Vec<CharacterMaterial>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterMaterial {
    pub asset_id: u64,
    pub shader_parameters: Vec<CharacterShaderParameter>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterShaderParameter {
    pub channel_name: String,
    pub asset_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CharacterAssetKind {
    Geometry,
    Texture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterAssetRequest {
    pub kind: CharacterAssetKind,
    pub source_asset_id: u64,
    pub loaded_asset_id: u64,
    pub entry_path: String,
    pub compound_path: String,
}

impl CharacterRecipe {
    /// Parses one complete binary character recipe.
    ///
    /// Includes the version-2 Character::Customizations payload when present.
    ///
    /// # Errors
    ///
    /// Returns [`CharacterRecipeError`] for unsupported versions or
    /// customizations, truncated data, malformed strings/counts, invalid
    /// alignment padding, a bad size footer, or trailing bytes.
    pub fn parse(bytes: &[u8]) -> Result<Self, CharacterRecipeError> {
        let (recipe, consumed) = Self::parse_prefix(bytes)?;
        if consumed != bytes.len() {
            return Err(CharacterRecipeError::TrailingBytes { count: bytes.len() - consumed });
        }
        Ok(recipe)
    }

    /// CACPartDB embeds a complete recipe before its aligned string/part tables.
    pub fn parse_prefix(bytes: &[u8]) -> Result<(Self, usize), CharacterRecipeError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u32()?;
        if version != CHARACTER_RECIPE_VERSION {
            return Err(CharacterRecipeError::UnsupportedVersion(version));
        }
        let schema_name = reader.string()?;
        let category = reader.u32()?;
        let memory_category = reader.u32()?;
        let clone_type = reader.u32()?;
        let component_count = reader.count(8, "components")?;
        let mut components = Vec::with_capacity(component_count);
        for _ in 0..component_count {
            let name = reader.string()?;
            let model_count = reader.count(12, "component models")?;
            let mut models = Vec::with_capacity(model_count);
            for _ in 0..model_count {
                let asset_id = reader.u64()?;
                let lod_count = reader.count(21, "model LODs")?;
                let mut lods = Vec::with_capacity(lod_count);
                for _ in 0..lod_count {
                    let level = reader.u8()?;
                    let asset_id = reader.u64()?;
                    let arena_asset_id = reader.u64()?;
                    let instance_count = reader.count(4, "material instances")?;
                    let mut material_instances = Vec::with_capacity(instance_count);
                    for _ in 0..instance_count {
                        let material_count = reader.count(12, "materials")?;
                        let mut materials = Vec::with_capacity(material_count);
                        for _ in 0..material_count {
                            let asset_id = reader.u64()?;
                            let parameter_count = reader.count(12, "shader parameters")?;
                            let mut shader_parameters = Vec::with_capacity(parameter_count);
                            for _ in 0..parameter_count {
                                shader_parameters.push(CharacterShaderParameter {
                                    channel_name: reader.string()?,
                                    asset_id: reader.u64()?,
                                });
                            }
                            materials.push(CharacterMaterial {
                                asset_id,
                                shader_parameters,
                            });
                        }
                        material_instances.push(CharacterMaterialInstance { materials });
                    }
                    lods.push(CharacterLod {
                        level,
                        asset_id,
                        arena_asset_id,
                        material_instances,
                    });
                }
                models.push(CharacterModel { asset_id, lods });
            }
            components.push(CharacterComponent { name, models });
        }

        let customizations = if reader.u8()? != 0 {
            Some(CharacterCustomizations::read(&mut reader)?)
        } else { None };
        reader.align_four()?;
        let body_size = reader.position;
        let packed_size = reader.u32()?;
        if usize::try_from(packed_size).ok() != Some(body_size) {
            return Err(CharacterRecipeError::SizeMismatch {
                recorded: packed_size,
                actual: body_size,
            });
        }
        Ok((Self {
            version,
            schema_name,
            category,
            memory_category,
            clone_type,
            components,
            has_customizations: customizations.is_some(),
            customizations,
            packed_size,
        }, reader.position))
    }

    #[must_use]
    pub fn model_count(&self) -> usize {
        self.components.iter().map(|part| part.models.len()).sum()
    }

    #[must_use]
    pub fn lod_count(&self) -> usize {
        self.components
            .iter()
            .flat_map(|part| &part.models)
            .map(|model| model.lods.len())
            .sum()
    }

    #[must_use]
    pub fn material_count(&self) -> usize {
        self.components
            .iter()
            .flat_map(|part| &part.models)
            .flat_map(|model| &model.lods)
            .flat_map(|lod| &lod.material_instances)
            .map(|instance| instance.materials.len())
            .sum()
    }

    #[must_use]
    pub fn material_instance_count(&self) -> usize {
        self.components
            .iter()
            .flat_map(|part| &part.models)
            .flat_map(|model| &model.lods)
            .map(|lod| lod.material_instances.len())
            .sum()
    }

    #[must_use]
    pub fn shader_parameter_count(&self) -> usize {
        self.components
            .iter()
            .flat_map(|part| &part.models)
            .flat_map(|model| &model.lods)
            .flat_map(|lod| &lod.material_instances)
            .flat_map(|instance| &instance.materials)
            .map(|material| material.shader_parameters.len())
            .sum()
    }

    #[must_use]
    pub fn referenced_asset_ids(&self) -> Vec<u64> {
        let mut result = Vec::new();
        for model in self.components.iter().flat_map(|part| &part.models) {
            result.push(model.asset_id);
            for lod in &model.lods {
                result.push(lod.asset_id);
                result.push(lod.arena_asset_id);
                for material in lod
                    .material_instances
                    .iter()
                    .flat_map(|instance| &instance.materials)
                {
                    result.push(material.asset_id);
                    result.extend(
                        material
                            .shader_parameters
                            .iter()
                            .map(|parameter| parameter.asset_id),
                    );
                }
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    /// Reconstructs the arena requests produced by
    /// `CharacterEntity::ExtractAssetList` for this recipe.
    #[must_use]
    pub fn asset_requests(&self, category: &str) -> Vec<CharacterAssetRequest> {
        let mut result = Vec::new();
        for component in &self.components {
            for model in &component.models {
                for lod in &model.lods {
                    let entry_path = format!(
                        "data\\content\\{category}\\model\\{}\\{}\\0x{:016x}.rx2",
                        self.schema_name, component.name, lod.arena_asset_id
                    );
                    let compound_path = character_compound_path(category, &entry_path);
                    result.push(CharacterAssetRequest {
                        kind: CharacterAssetKind::Geometry,
                        source_asset_id: lod.arena_asset_id,
                        loaded_asset_id: rw_hash64_string(&compound_path),
                        entry_path,
                        compound_path,
                    });

                    for parameter in lod
                        .material_instances
                        .iter()
                        .flat_map(|instance| &instance.materials)
                        .flat_map(|material| &material.shader_parameters)
                    {
                        let basename = format!("0x{:016x}.rx2", parameter.asset_id);
                        let entry_path = format!("data\\content\\{category}\\texture\\{basename}");
                        result.push(CharacterAssetRequest {
                            kind: CharacterAssetKind::Texture,
                            source_asset_id: parameter.asset_id,
                            loaded_asset_id: rw_hash64_string(&basename),
                            compound_path: character_compound_path(category, &entry_path),
                            entry_path,
                        });
                    }
                }
            }
        }
        result
    }
}

fn character_compound_path(category: &str, entry_path: &str) -> String {
    // CharacterEntity::ExtractAssetList82d0c58c..5bc (and texture suffix):
    // vehicle keeps its entry namespace but shares the LivingWorld archive.
    let archive = if category == "vehicle" {
        "livingworld"
    } else {
        category
    };
    format!("data\\content/{archive}.big|{entry_path}")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterRecipeError {
    Truncated { offset: usize, needed: usize },
    InvalidUtf8 { offset: usize, length: usize },
    CountOutsideInput { field: &'static str, count: u32 },
    UnsupportedVersion(u32),
    UnsupportedCustomizations,
    NonzeroPadding { offset: usize, value: u8 },
    SizeMismatch { recorded: u32, actual: usize },
    TrailingBytes { count: usize },
}

impl fmt::Display for CharacterRecipeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { offset, needed } => {
                write!(
                    formatter,
                    "recipe is truncated at {offset}, requiring {needed} bytes"
                )
            }
            Self::InvalidUtf8 { offset, length } => {
                write!(
                    formatter,
                    "recipe string at {offset} has {length} invalid UTF-8 bytes"
                )
            }
            Self::CountOutsideInput { field, count } => {
                write!(
                    formatter,
                    "recipe {field} count {count} cannot fit in the input"
                )
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported character recipe version {version}")
            }
            Self::UnsupportedCustomizations => {
                formatter.write_str("invalid or unsupported character customization payload")
            }
            Self::NonzeroPadding { offset, value } => {
                write!(
                    formatter,
                    "recipe alignment byte at {offset} is {value:#04x}"
                )
            }
            Self::SizeMismatch { recorded, actual } => {
                write!(
                    formatter,
                    "recipe records {recorded} bytes but consumed {actual}"
                )
            }
            Self::TrailingBytes { count } => write!(formatter, "recipe has {count} trailing bytes"),
        }
    }
}

impl std::error::Error for CharacterRecipeError {}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], CharacterRecipeError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(CharacterRecipeError::Truncated {
                offset: self.position,
                needed: length,
            })?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(CharacterRecipeError::Truncated {
                offset: self.position,
                needed: length,
            })?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, CharacterRecipeError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, CharacterRecipeError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }

    fn u64(&mut self) -> Result<u64, CharacterRecipeError> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }

    fn string(&mut self) -> Result<String, CharacterRecipeError> {
        let length = usize::try_from(self.u32()?).map_err(|_| CharacterRecipeError::Truncated {
            offset: self.position,
            needed: usize::MAX,
        })?;
        let offset = self.position;
        let bytes = self.take(length)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| CharacterRecipeError::InvalidUtf8 { offset, length })
    }

    fn count(
        &mut self,
        minimum_item_size: usize,
        field: &'static str,
    ) -> Result<usize, CharacterRecipeError> {
        let count = self.u32()?;
        let remaining = self.bytes.len().saturating_sub(self.position);
        let value = usize::try_from(count)
            .ok()
            .filter(|value| value.saturating_mul(minimum_item_size) <= remaining)
            .ok_or(CharacterRecipeError::CountOutsideInput { field, count })?;
        Ok(value)
    }

    fn align_four(&mut self) -> Result<(), CharacterRecipeError> {
        while self.position % 4 != 0 {
            let offset = self.position;
            let value = self.u8()?;
            if value != 0 {
                return Err(CharacterRecipeError::NonzeroPadding { offset, value });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_keeps_entry_namespace_but_uses_livingworld_archive() {
        let entry = "data\\content/character/vehicle/taxi.rx2";
        assert_eq!(
            character_compound_path("vehicle", entry),
            format!("data\\content/livingworld.big|{entry}")
        );
        assert_eq!(
            character_compound_path("marquee", "skater.rx2"),
            "data\\content/marquee.big|skater.rx2"
        );
    }

    fn u32_bytes(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_be_bytes());
    }

    fn u64_bytes(output: &mut Vec<u8>, value: u64) {
        output.extend_from_slice(&value.to_be_bytes());
    }

    fn string_bytes(output: &mut Vec<u8>, value: &str) {
        u32_bytes(output, u32::try_from(value.len()).unwrap());
        output.extend_from_slice(value.as_bytes());
    }

    fn fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        u32_bytes(&mut bytes, CHARACTER_RECIPE_VERSION);
        string_bytes(&mut bytes, "test_skater");
        u32_bytes(&mut bytes, 0);
        u32_bytes(&mut bytes, 15);
        u32_bytes(&mut bytes, 0);
        u32_bytes(&mut bytes, 1);
        string_bytes(&mut bytes, "Board");
        u32_bytes(&mut bytes, 1);
        u64_bytes(&mut bytes, 0x10);
        u32_bytes(&mut bytes, 1);
        bytes.push(2);
        u64_bytes(&mut bytes, 0x20);
        u64_bytes(&mut bytes, 0x30);
        u32_bytes(&mut bytes, 1);
        u32_bytes(&mut bytes, 1);
        u64_bytes(&mut bytes, 0x40);
        u32_bytes(&mut bytes, 1);
        string_bytes(&mut bytes, "DiffuseMap");
        u64_bytes(&mut bytes, 0x50);
        bytes.push(0);
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        let size = u32::try_from(bytes.len()).unwrap();
        u32_bytes(&mut bytes, size);
        bytes
    }

    #[test]
    fn parses_exact_nested_graph_and_unaligned_strings() {
        let recipe = CharacterRecipe::parse(&fixture()).unwrap();
        assert_eq!(recipe.schema_name, "test_skater");
        assert_eq!(recipe.memory_category, 15);
        assert_eq!(recipe.components[0].name, "Board");
        assert_eq!(recipe.components[0].models[0].asset_id, 0x10);
        assert_eq!(recipe.components[0].models[0].lods[0].level, 2);
        assert_eq!(recipe.model_count(), 1);
        assert_eq!(recipe.lod_count(), 1);
        assert_eq!(recipe.material_instance_count(), 1);
        assert_eq!(recipe.material_count(), 1);
        assert_eq!(recipe.shader_parameter_count(), 1);
        assert_eq!(
            recipe.referenced_asset_ids(),
            [0x10, 0x20, 0x30, 0x40, 0x50]
        );
        assert_eq!(
            recipe.asset_requests("marquee"),
            [
                CharacterAssetRequest {
                    kind: CharacterAssetKind::Geometry,
                    source_asset_id: 0x30,
                    loaded_asset_id: rw_hash64_string(
                        "data\\content/marquee.big|data\\content\\marquee\\model\\test_skater\\Board\\0x0000000000000030.rx2"
                    ),
                    entry_path: "data\\content\\marquee\\model\\test_skater\\Board\\0x0000000000000030.rx2".to_owned(),
                    compound_path: "data\\content/marquee.big|data\\content\\marquee\\model\\test_skater\\Board\\0x0000000000000030.rx2".to_owned(),
                },
                CharacterAssetRequest {
                    kind: CharacterAssetKind::Texture,
                    source_asset_id: 0x50,
                    loaded_asset_id: rw_hash64_string("0x0000000000000050.rx2"),
                    entry_path: "data\\content\\marquee\\texture\\0x0000000000000050.rx2".to_owned(),
                    compound_path: "data\\content/marquee.big|data\\content\\marquee\\texture\\0x0000000000000050.rx2".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn rejects_footer_disagreement() {
        let mut bytes = fixture();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert!(matches!(
            CharacterRecipe::parse(&bytes),
            Err(CharacterRecipeError::SizeMismatch { .. })
        ));
    }

    #[test]
    fn customized_recipe_round_trip_preserves_float_bits_and_asset_identity() {
        let mut recipe = CharacterRecipe::parse(&fixture()).unwrap();
        recipe.has_customizations = true;
        recipe.customizations = Some(CharacterCustomizations {
            is_male: false,
            colour_capacity: 12,
            colours: vec![customizations::Colour { key: 4,
                base_rgb_bits: [0x80000000, 0x3f400000, 0x7fc12345],
                highlight_rgb_bits: [0x3f800000; 3], model_id: 0xabcdef0123456789,
                material_id: 0x1020304050607080 }],
            stamp_capacity: 3,
            stamps: vec![customizations::TextureStamp { path: "native/stamp".into(),
                model_id: 7, material_id: 8, stamp_material_id: 9, user_generated: false, key: 2 }],
            embedded_capacity: 6, embedded: vec![[0,8],[2,3]],
            morph_bits: vec![0x3f000000; 21],
        });
        let encoded = recipe.encode().unwrap();
        let decoded = CharacterRecipe::parse(&encoded).unwrap();
        assert_eq!(decoded.customizations, recipe.customizations);
        assert_eq!(decoded.components, recipe.components);
        assert_eq!(decoded.encode().unwrap(), encoded);
    }
}
