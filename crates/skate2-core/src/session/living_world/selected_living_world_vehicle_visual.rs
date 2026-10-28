//! Completed simulation publication for CreatePresVehicle82810d40.
//! Uses the already finalized RecipeMan assets and actual VehicleMan members.
use crate::{
    actor_construction::ActorConstruction,
    attribute_runtime::AttributeRuntimeVault,
    character::CharacterScene,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::vehicle_entity::frame::{VehicleFrameError, error},
    physical_player_assembly::PhysicalAssemblyWorld,
    presentation_restore::living_world::{VehicleReplay, VehicleWheels},
    selected_living_world_recipes::PresentationRecipeModels,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SelectedVehicleVisual {
    pub entity_id: u32,
    pub model_key: u64,
    pub recipe_id: u64,
    pub scene: Arc<CharacterScene>,
    pub pose_name: String,
    pub transform: [f32; 16],
    /// The wheels and lights of the vehicle's own record, as a presentation entity
    /// created with it takes them.
    pub wheels: VehicleWheels,
    pub colours: [[f32; 4]; 2],
    pub irradiance: [[f32; 4]; 9],
    pub opacity_parameters: crate::LivingWorldOpacityParameters,
    pub opacity_settings: crate::LivingWorldOpacitySettings,
    pub simulation_center_velocity: [f32; 4],
}

/// SHLightingMan::UpdateDefaultLightProbe82827860 validates nine float3s;
/// cPresVehicle::BindConstants8280b8f8 copies this same default probe per car.
pub(crate) fn vehicle_default_irradiance(
    vault: &Arc<AttributeRuntimeVault>,
) -> Result<[[f32; 4]; 9], VehicleFrameError> {
    let location = vault.instance(
        skate2_formats::attrib::string_hash64("render_locations"),
        0xec76_3034_105b_866b,
    );
    let key = 0xf45d_fcab_6ec1_95cd;
    if location.try_length(key).map_err(error)? != 9 {
        return Err(error(
            "vehicle default light probe requires nine coefficients",
        ));
    }
    let mut irradiance = [[0.; 4]; 9];
    for (i, entry) in irradiance.iter_mut().enumerate() {
        let Some(data) = location
            .try_vector_array_data(key, i as u32, 3)
            .map_err(error)?
        else {
            // Native GetAttributePointer fallback: DefaultDataArea(12).
            continue;
        };
        for (lane, word) in entry[..3].iter_mut().zip(data.bytes().chunks_exact(4)) {
            *lane = f32::from_be_bytes(word.try_into().expect("four-byte lane"));
        }
    }
    Ok(irradiance)
}

pub(crate) fn vehicle_visuals(
    world: &PhysicalAssemblyWorld,
    actor: &ActorConstruction,
    vault: &Arc<AttributeRuntimeVault>,
    cvars: &InputCvarReferences,
    recipes: &PresentationRecipeModels,
) -> Result<Vec<SelectedVehicleVisual>, VehicleFrameError> {
    let (_, velocity) = crate::simulation_assembly_backend::constraint_session::dmo_living_world::local_skater_census_centre(actor)
        .map_err(error)?;
    let owner = world.grind_services.living_world().lock().map_err(error)?;
    let Some(vehicles) = owner.grind_vehicle_entities.as_deref() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut default_irradiance = None;
    for id in vehicles.ids() {
        let v = vehicles
            .get(id)
            .ok_or_else(|| error("vehicle publication membership"))?;
        let Some(assets) = recipes.get(v.core.recipe_id) else {
            // CreatePresVehicle returns null until this recipe is published.
            continue;
        };
        let model = vault.instance(
            crate::living_world_recipe_manager::LIVING_WORLD_MODELS_CLASS_ID,
            v.core.model_key,
        );
        let name = model
            .try_cached_layout_text(0)
            .map_err(error)?
            .ok_or_else(|| error("vehicle bind-pose name"))?;
        let read = |instance: &crate::attribute_runtime::AttributeRuntimeInstance,
                    key,
                    index,
                    count|
         -> Result<[f32; 4], VehicleFrameError> {
            let Some(data) = instance
                .try_vector_array_data(key, index, count)
                .map_err(error)?
            else {
                return Ok([0.; 4]);
            };
            if data.bytes().len() < count * 4 {
                return Err(error("vehicle float vector payload"));
            }
            let mut value = [0.; 4];
            for (lane, word) in value[..count].iter_mut().zip(data.bytes().chunks_exact(4)) {
                *lane = f32::from_be_bytes(word.try_into().expect("four-byte lane"));
            }
            Ok(value)
        };
        let mut colours = [[0.; 4]; 2];
        for (i, key) in [0xdf76_d7d7_7385_7edb, 0x1202_6e2e_ed18_cc8d]
            .into_iter()
            .enumerate()
        {
            colours[i] = read(&model, key, u32::from(v.core.colour_indices[i]), 4)?;
        }
        let irradiance = match default_irradiance {
            Some(probe) => probe,
            None => {
                let probe = vehicle_default_irradiance(vault)?;
                default_irradiance = Some(probe);
                probe
            }
        };
        let mut transform = std::array::from_fn(|i| v.core.transform[i / 4][i % 4]);
        transform[15] = 1.;
        let wheels = VehicleReplay::new(
            id,
            transform,
            v.wheels.spin_angle,
            v.wheels.turn_angle,
            v.route.lights,
        )
        .wheels();
        out.push(SelectedVehicleVisual {
            entity_id: id,
            model_key: v.core.model_key,
            recipe_id: v.core.recipe_id,
            scene: Arc::clone(assets.finalized.scene().map_err(error)?),
            pose_name: std::str::from_utf8(name.bytes()).map_err(error)?.to_owned(),
            transform,
            wheels,
            colours,
            irradiance,
            opacity_parameters: crate::LivingWorldOpacityParameters::try_read(&model)
                .map_err(error)?,
            opacity_settings: cvars.try_living_world_opacity_settings().map_err(error)?,
            simulation_center_velocity: velocity,
        });
    }
    Ok(out)
}
