//! Pedestrian82e97748 attribute bindings over the retained session Vault.
use crate::attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeError, AttributeRuntimeVault};
use std::sync::Arc;

pub(crate) const ANIMATION_DATA: u64 = 0x0036_7b8f_33e7_9c43;
pub(crate) const CHASE_PARAMETERS: u64 = 0xfbc4_2525_e74d_946f;
pub(crate) const LOCOMOTION_PARAMETERS: u64 = 0x8a97_3d56_59a6_5f1b;
pub(crate) const TAKE_DOWN_DATA: u64 = 0x76ac_331f_bd58_03ff;

pub(crate) struct PedestrianResources {
    pub(crate) entity: AttributeRuntimeInstance,
    pub(crate) model: AttributeRuntimeInstance,
    pub(crate) animation: AttributeRuntimeInstance,
    pub(crate) chase: AttributeRuntimeInstance,
    pub(crate) locomotion: AttributeRuntimeInstance,
    pub(crate) take_down: AttributeRuntimeInstance,
}
#[derive(Debug)]
pub(crate) enum PedestrianResourceError {
    Attribute(AttributeRuntimeError),
    MissingEntity(u64),
    MissingReference(u64),
}
impl std::fmt::Display for PedestrianResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pedestrian resources: {self:?}")
    }
}
impl std::error::Error for PedestrianResourceError {}
impl PedestrianResources {
    pub(crate) fn load(vault: &Arc<AttributeRuntimeVault>, entity_key: u64, model_key: u64)
        -> Result<Self, PedestrianResourceError>
    {
        let entity = vault.instance(crate::living_world_grind_services::dmo_objects::ENTITIES_CLASS, entity_key);
        if entity.is_null() { return Err(PedestrianResourceError::MissingEntity(entity_key)); }
        let model = vault.instance(crate::living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID, model_key);
        let required = |key| entity.try_ref_spec_instance(key)
            .map_err(PedestrianResourceError::Attribute)?
            .ok_or(PedestrianResourceError::MissingReference(key));
        // References retain their own authored classes. No default pedestrian
        // collection or guard constants replace missing required resources.
        let animation = required(ANIMATION_DATA)?;
        let chase = required(CHASE_PARAMETERS)?;
        let locomotion = required(LOCOMOTION_PARAMETERS)?;
        let take_down = required(TAKE_DOWN_DATA)?;
        Ok(Self { entity, model, animation, chase, locomotion, take_down })
    }
}
