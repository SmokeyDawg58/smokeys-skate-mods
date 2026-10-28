//! Bind selected character materials to the shared native material shader paths.
use crate::gpu_renderer::{
    CameraRelativeClip, GpuObject, GpuRenderer, RenderResult,
    character::{CharacterGhostState, CharacterShaderFamily},
    character_palette::{CharacterMeshAdmission, SelectedCharacterGpuBinding},
};
use skate2_core::character::{
    material_admission::CharacterMaterialAdmission, palette::CharacterPalette,
};
use skate2_formats::mount::MountSet;
use std::collections::BTreeSet;

/// Builds real GPU objects; caller retains returned bindings beside stable object
/// indices in its existing window-owned object vector.
pub(crate) fn create_objects(
    renderer: &GpuRenderer,
    mounts: &MountSet,
    palette: &CharacterPalette,
    materials: &CharacterMaterialAdmission,
    camera: &CameraRelativeClip,
    ghost: CharacterGhostState,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
) -> RenderResult<Vec<(SelectedCharacterGpuBinding, GpuObject)>> {
    create_objects_filtered(renderer, mounts, palette, materials, camera, ghost, environment, frame, irradiance, |_| true)
}

/// `create_objects` for only the parts `keep` accepts (the PC Object Dropper's copy of the
/// player's board draws just the board's parts).
pub(crate) fn create_objects_filtered(
    renderer: &GpuRenderer,
    mounts: &MountSet,
    palette: &CharacterPalette,
    materials: &CharacterMaterialAdmission,
    camera: &CameraRelativeClip,
    ghost: CharacterGhostState,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
    keep: impl Fn(usize) -> bool,
) -> RenderResult<Vec<(SelectedCharacterGpuBinding, GpuObject)>> {
    let meshes: Vec<_> = materials.meshes().filter(|&(part, _)| keep(part)).collect();
    // `CreateEffectListFromMaterialKey` 0x82e203f8: each material's
    // `DrawOrder` RefSpec GUID is the first `InstanceSortKey` byte.
    let keys = meshes
        .iter()
        .map(|&(part, mesh)| Ok(materials.view(palette, part, mesh)?.material_key))
        .collect::<RenderResult<BTreeSet<_>>>()?;
    let bindings = crate::load_material_attribute_bindings(mounts, &keys)?;
    let mut output = Vec::new();
    for (part, mesh) in meshes {
        let view = materials.view(palette, part, mesh)?;
        let family = family(view.effect)?;
        let order = bindings
            .get(&view.material_key)
            .ok_or("character material has no attribute binding")?
            .draw_order;
        let draw_order = u8::try_from(order)
            .map_err(|_| format!("character draw order {order} is not a sort-key byte"))?;
        let texture = renderer.upload_native_texture(view.texture)?;
        let alpha = view
            .alpha
            .map(|alpha| renderer.upload_native_texture(alpha))
            .transpose()?;
        let object = renderer.create_selected_character_object(
            palette,
            materials,
            CharacterMeshAdmission {
                part,
                mesh,
                family,
                draw_order,
            },
            camera,
            &texture,
            if family.is_hair() {
                alpha.as_ref()
            } else {
                None
            },
            ghost,
            living_world_constants(&view, environment, frame)?,
        )?;
        let (binding, mut object) = object.into_parts();
        if family == CharacterShaderFamily::LivingWorld {
            renderer.bind_living_world_lighting(&mut object, &texture, &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
        } else if let Some(mode) = crate::gpu_renderer::character::cas::mode(view.effect) {
            let normal =
                renderer.upload_native_texture(view.normal.ok_or("CAS normal channel absent")?)?;
            let decal = view
                .decal
                .map(|t| renderer.upload_native_texture(t))
                .transpose()?;
            let decal2 = view
                .decal2
                .map(|t| renderer.upload_native_texture(t))
                .transpose()?;
            let inputs = cas_inputs(&view, environment, frame, irradiance, camera)?;
            renderer.bind_cas_surface(
                &mut object,
                &view.geometry.geometry,
                &normal,
                alpha.as_ref(),
                decal.as_ref(),
                decal2.as_ref(),
                &inputs,
                cas_custom(&view, mode),
            )?;
        } else {
            let normal=renderer.upload_native_texture(view.normal.ok_or("character normal channel absent")?)?;
            renderer.bind_character_surface(&mut object, &view.geometry.geometry, &normal, alpha.as_ref(),
                &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
        }
        renderer.set_character_fog(&object, frame)?;
        output.push((binding, object));
    }
    Ok(output)
}
pub(crate) fn update_object(
    renderer: &GpuRenderer,
    binding: &SelectedCharacterGpuBinding,
    object: &mut GpuObject,
    palette: &CharacterPalette,
    materials: &CharacterMaterialAdmission,
    camera: &CameraRelativeClip,
    ghost: CharacterGhostState,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
) -> RenderResult<()> {
    let (part, mesh) = binding.part_mesh();
    let view = materials.view(palette, part, mesh)?;
    renderer.update_selected_character_binding(
        binding,
        object,
        palette,
        materials,
        camera,
        ghost,
        living_world_constants(&view, environment, frame)?,
    )?;
    if let Some(mode) = crate::gpu_renderer::character::cas::mode(view.effect) {
        renderer.update_character_cas_constants(
            object,
            &cas_inputs(&view, environment, frame, irradiance, camera)?,
            cas_custom(&view, mode),
        )?;
    } else {
        renderer.update_character_stamp(object,
            &cas_inputs(&view, environment, frame, irradiance, camera)?)?;
    }
    renderer.set_character_fog(object, frame)?;
    Ok(())
}
fn family(effect: &str) -> RenderResult<CharacterShaderFamily> {
    match effect {
        "defaultcharacter" | "cacstamp_cloth" | "cacstamp_skin" | "cacstamp_face"
        | "cacstamp_shift" => Ok(CharacterShaderFamily::Character),
        "defaulthair" | "cac_hair" | "cac_hair_ropa" => Ok(CharacterShaderFamily::Hair),
        "defaulthair_ropa" => Ok(CharacterShaderFamily::HairRopa),
        "defaultcharacter_ropa" | "cacstamp_cloth_ropa" => Ok(CharacterShaderFamily::Ropa),
        "defaultlivingworld" => Ok(CharacterShaderFamily::LivingWorld),
        other => Err(format!("unsupported actual character material effect {other:?}").into()),
    }
}

/// `i_colorize_red`/`i_colorize_blue` of a skater mesh with the pedestrian effect: a plain
/// `SkaterPresEntity` binds neither (`BindConstants` 0x827dcf50) and `cScope::DispatchParamsHelper`
/// 0x82e14aa0 uploads only bound params, so the native register keeps the previous draw's
/// value. The port uses the `ColorizedSkaterPresEntity` constructor's colour (0x827e5e20).
const UNBOUND_COLORIZE: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

/// The `defaultlivingworld` constants of a skater mesh: the colorize pair, the fog, then the
/// material gain, the exposure and the `m_params[5].w`/`[7].w` factors.
fn living_world_constants(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
) -> RenderResult<Option<[[f32; 4]; 5]>> {
    if view.effect != "defaultlivingworld" {
        return Ok(None);
    }
    let rows = environment
        .material_parameter_array(view.material_key.0, view.material_key.1)
        .and_then(|rows| rows.get(..8))
        .ok_or("livingworld eight material rows absent")?;
    let exposure = environment.frame_character_attributes(frame)?[2][2];
    Ok(Some([
        UNBOUND_COLORIZE,
        UNBOUND_COLORIZE,
        frame.fog_k1,
        frame.fog_colour,
        [rows[0][1], exposure, rows[5][3], rows[7][3]],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn effect_mapping_has_no_substitution() {
        assert_eq!(
            family("defaultcharacter").unwrap(),
            CharacterShaderFamily::Character
        );
        assert_eq!(family("defaulthair").unwrap(), CharacterShaderFamily::Hair);
        assert_eq!(
            family("defaultcharacter_ropa").unwrap(),
            CharacterShaderFamily::Ropa
        );
        assert!(family("default").is_err());
        assert!(family("marquee_cloth").is_err());
    }
}

fn cas_custom(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    mode: f32,
) -> [[f32; 4]; 5] {
    [
        view.colours[0],
        view.colours[1],
        view.tattoos[0],
        view.tattoos[1],
        [mode, 0., 0., 0.],
    ]
}
fn cas_inputs(
    view: &skate2_core::character::material_admission::CharacterMaterialView<'_>,
    environment: &crate::render_environment::EnvironmentLighting,
    frame: crate::gpu_renderer::FrameEnvironment,
    irradiance: skate2_formats::irradiance::Coefficients,
    camera: &CameraRelativeClip,
) -> RenderResult<crate::gpu_renderer::character::stamp::Inputs> {
    Ok(crate::gpu_renderer::character::stamp::Inputs {
        material: environment
            .material_parameter_array(view.material_key.0, view.material_key.1)
            .and_then(|rows| rows.get(..7))
            .ok_or("CAS character seven material rows absent")?
            .try_into()?,
        irradiance,
        attributes: environment.frame_character_attributes(frame)?,
        light: frame.light_direction,
        csm: frame.shadows.globals(),
        camera: camera.camera,
        material_lod: 0.,
    })
}
