//! Inverse of the recovered CharacterEntity::Pack v7 stream.
use super::{CharacterRecipe, CharacterRecipeError as Error};
pub(super) fn u32_value(out: &mut Vec<u8>, value: u32) { out.extend_from_slice(&value.to_be_bytes()); }
pub(super) fn u64_value(out: &mut Vec<u8>, value: u64) { out.extend_from_slice(&value.to_be_bytes()); }
pub(super) fn count(out: &mut Vec<u8>, value: usize) -> Result<(), Error> {
    u32_value(out, u32::try_from(value).map_err(|_| Error::CountOutsideInput { field: "serialized length", count: u32::MAX })?);
    Ok(())
}
pub(super) fn string(out: &mut Vec<u8>, value: &str) -> Result<(), Error> {
    count(out, value.len())?; out.extend_from_slice(value.as_bytes()); Ok(())
}
impl CharacterRecipe {
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        if self.version != super::CHARACTER_RECIPE_VERSION { return Err(Error::UnsupportedVersion(self.version)); }
        if self.has_customizations != self.customizations.is_some() { return Err(Error::UnsupportedCustomizations); }
        let mut out = Vec::new();
        u32_value(&mut out, self.version); string(&mut out, &self.schema_name)?;
        for v in [self.category, self.memory_category, self.clone_type] { u32_value(&mut out, v); }
        count(&mut out, self.components.len())?;
        for part in &self.components {
            string(&mut out, &part.name)?; count(&mut out, part.models.len())?;
            for model in &part.models {
                u64_value(&mut out, model.asset_id); count(&mut out, model.lods.len())?;
                for lod in &model.lods {
                    out.push(lod.level); u64_value(&mut out, lod.asset_id); u64_value(&mut out, lod.arena_asset_id);
                    count(&mut out, lod.material_instances.len())?;
                    for instance in &lod.material_instances {
                        count(&mut out, instance.materials.len())?;
                        for material in &instance.materials {
                            u64_value(&mut out, material.asset_id); count(&mut out, material.shader_parameters.len())?;
                            for param in &material.shader_parameters {
                                string(&mut out, &param.channel_name)?; u64_value(&mut out, param.asset_id);
                            }
                        }
                    }
                }
            }
        }
        out.push(u8::from(self.customizations.is_some()));
        if let Some(custom) = &self.customizations { out.extend(custom.encode()?); }
        while out.len() % 4 != 0 { out.push(0); }
        let size = out.len(); count(&mut out, size)?;
        Ok(out)
    }
}
