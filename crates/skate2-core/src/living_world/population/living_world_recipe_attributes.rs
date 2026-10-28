//! RecipeMan's attribute reads through the existing loaded Vault occurrence.
use super::{
    FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID, FORCED_VEHICLE_MODEL_ATTRIBUTE_ID,
    LIVING_WORLD_CONFIG_CLASS_ID, LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID,
    LIVING_WORLD_MODELS_CLASS_ID, LOAD_MODEL_ATTRIBUTE_ID, RecipeMappingProvider,
    runtime::{ModelCategory, RecipeModel},
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum RecipeAttributeError {
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    MissingModel(u64),
    MissingFilename(u64),
    FilenameUtf8(std::str::Utf8Error),
    Category(u32),
}
impl From<AttributeRuntimeError> for RecipeAttributeError {
    fn from(value: AttributeRuntimeError) -> Self {
        Self::Attribute(value)
    }
}
impl std::fmt::Display for RecipeAttributeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld recipe attributes: {self:?}")
    }
}

pub(crate) struct RecipeAttributes<'a> {
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
    pub(crate) cvars: &'a InputCvarReferences,
    // The existing mapping provider has an infallible bool projection. Retain a
    // host cvar error and stop at its next fallible boundary; never hide it.
    cvar_error: Option<InputCvarError>,
}
impl<'a> RecipeAttributes<'a> {
    pub(crate) fn new(
        vault: &'a Arc<AttributeRuntimeVault>,
        cvars: &'a InputCvarReferences,
    ) -> Self {
        Self {
            vault,
            cvars,
            cvar_error: None,
        }
    }
    pub(crate) fn finish(&mut self) -> Result<(), RecipeAttributeError> {
        match self.cvar_error.take() {
            Some(error) => Err(RecipeAttributeError::Cvar(error)),
            None => Ok(()),
        }
    }
    /// The GetRandomSubModelKey loaded-filter and GenerateModelAssetID read
    /// only the filename, without interpreting unrelated category enum values.
    pub(crate) fn filename(&self, key: u64) -> Result<String, RecipeAttributeError> {
        let instance = self.vault.instance(LIVING_WORLD_MODELS_CLASS_ID, key);
        if instance.is_null() {
            return Err(RecipeAttributeError::MissingModel(key));
        }
        let filename = instance
            .try_cached_layout_text(0)?
            .ok_or(RecipeAttributeError::MissingFilename(key))?;
        Ok(std::str::from_utf8(filename.bytes())
            .map_err(RecipeAttributeError::FilenameUtf8)?
            .to_owned())
    }
    /// Forced prefix shared by GetRandomSubModelKey8271aff8 and
    /// GenerateModelAssetID82712b44. Other category words fall through.
    pub(crate) fn forced_model(&self, key: u64) -> Result<Option<u64>, RecipeAttributeError> {
        if !self
            .cvars
            .try_recipe_use_forced()
            .map_err(RecipeAttributeError::Cvar)?
        {
            return Ok(None);
        }
        let category = self
            .vault
            .instance(LIVING_WORLD_MODELS_CLASS_ID, key)
            .try_scalar_word(0xf689_7cc7_b637_850c, 0)?
            .unwrap_or(0);
        let attribute = match category {
            0 => FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID,
            3 => FORCED_VEHICLE_MODEL_ATTRIBUTE_ID,
            _ => return Ok(None),
        };
        // Missing RefSpec is the zeroed DefaultDataArea, hence collection0.
        Ok(Some(
            self.vault
                .instance(
                    LIVING_WORLD_CONFIG_CLASS_ID,
                    LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID,
                )
                .try_ref_spec_collection(attribute)?
                .unwrap_or(0),
        ))
    }
    /// Info constructor82701ba8 and Load827239e0 read the same category node,
    /// filename layout+0 and originating collection key. Null category defaults0.
    pub(crate) fn model(&self, key: u64) -> Result<RecipeModel, RecipeAttributeError> {
        let instance = self.vault.instance(LIVING_WORLD_MODELS_CLASS_ID, key);
        if instance.is_null() {
            return Err(RecipeAttributeError::MissingModel(key));
        }
        let word = instance
            .try_scalar_word(0xf689_7cc7_b637_850c, 0)?
            .unwrap_or(0);
        let category =
            ModelCategory::from_word(word).ok_or(RecipeAttributeError::Category(word))?;
        let filename = self.filename(key)?;
        Ok(RecipeModel {
            key,
            filename,
            category,
        })
    }
}
impl RecipeMappingProvider for RecipeAttributes<'_> {
    type Error = RecipeAttributeError;
    fn use_forced_recipes(&mut self) -> bool {
        match self.cvars.try_recipe_use_forced() {
            Ok(value) => value,
            Err(error) => {
                self.cvar_error = Some(error);
                false
            }
        }
    }
    fn forced_model_collection(&mut self, attribute_id: u64) -> Result<Option<u64>, Self::Error> {
        self.finish()?;
        debug_assert!(matches!(
            attribute_id,
            FORCED_PEDESTRIAN_MODEL_ATTRIBUTE_ID | FORCED_VEHICLE_MODEL_ATTRIBUTE_ID
        ));
        Ok(self
            .vault
            .instance(
                LIVING_WORLD_CONFIG_CLASS_ID,
                LIVING_WORLD_FORCED_CONFIG_COLLECTION_ID,
            )
            .try_ref_spec_collection(attribute_id)?)
    }
    fn leaf_descendants_sorted(&mut self, class: u64, seed: u64) -> Result<Vec<u64>, Self::Error> {
        self.finish()?;
        Ok(self.vault.leaf_descendants_sorted(class, seed)?)
    }
    fn load_model_nonzero(&mut self, class: u64, collection: u64) -> Result<bool, Self::Error> {
        self.finish()?;
        Ok(self
            .vault
            .instance(class, collection)
            .try_get_data(LOAD_MODEL_ATTRIBUTE_ID, 0)?
            .is_some_and(|value| value.bytes().first().is_some_and(|&v| v != 0)))
    }
    fn collection_parent(&mut self, class: u64, collection: u64) -> Result<u64, Self::Error> {
        self.finish()?;
        Ok(self.vault.collection_parent(class, collection)?)
    }
}
