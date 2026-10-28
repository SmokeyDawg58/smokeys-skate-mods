//! Same-palette original mesh binding for the existing character GPU consumer.
use super::character::{CharacterGhostState, CharacterPoseRows, CharacterShaderFamily};
use super::{CameraRelativeClip, GpuObject, GpuRenderer, GpuTexture, RenderResult};
use skate2_core::character::palette::{CharacterPalette, CharacterPaletteIdentity};
#[cfg(test)]
use skate2_core::presentation::PresentationMesh;

/// Material admission is supplied by the actual host material/texture owners.
/// Ropa's CPU branch requires the existing prepared vertex/root-row adapter.
#[derive(Clone, Copy)]
pub(crate) struct CharacterMeshAdmission {
    pub(crate) part: usize,
    pub(crate) mesh: usize,
    pub(crate) family: CharacterShaderFamily,
    /// The material's `DrawOrder` GUID, the first `InstanceSortKey` byte.
    pub(crate) draw_order: u8,
}
pub(crate) struct SelectedCharacterGpuObject {
    pub(crate) object: GpuObject,
    binding: SelectedCharacterGpuBinding,
}
pub(crate) struct SelectedCharacterGpuBinding {
    object_identity: std::sync::Arc<()>,
    palette: CharacterPaletteIdentity,
    admission: CharacterMeshAdmission,
}

#[cfg(test)]
fn geometry(
    palette: &CharacterPalette,
    part: usize,
    mesh: usize,
) -> RenderResult<&PresentationMesh> {
    let part = palette
        .skinning()
        .parts
        .get(part)
        .ok_or("selected GPU part is absent")?;
    let scene = palette.scene();
    let lod = scene
        .components
        .get(part.component_index)
        .and_then(|c| c.models.get(part.model_index))
        .and_then(|m| m.lods.get(part.lod_index))
        .ok_or("selected GPU LOD lost its scene owner")?;
    let model = lod
        .arena
        .instances
        .iter()
        .find_map(|i| i.render_model)
        .and_then(|i| lod.arena.models.get(i))
        .ok_or("selected GPU model is absent")?;
    let mesh = model
        .meshes
        .get(mesh)
        .and_then(|m| lod.arena.meshes.get(m.mesh_index))
        .ok_or("selected GPU mesh is absent")?;
    Ok(mesh)
}

fn pose_rows<'a>(
    palette: &'a CharacterPalette,
    admission: CharacterMeshAdmission,
    camera: &'a CameraRelativeClip,
    ropa_vertex_params_x: Option<f32>,
    cpu_rows:Option<&'a [[f32;4]]>,
    ghost: CharacterGhostState,
    living_world: Option<[[f32; 4]; 5]>,
) -> RenderResult<CharacterPoseRows<'a>> {
    let published = palette.mesh_rows(admission.part, admission.mesh)?;
    let cpu_branch=ropa_vertex_params_x.is_some_and(|x|x.partial_cmp(&0.)!=Some(std::cmp::Ordering::Greater));
    if cpu_branch!=cpu_rows.is_some(){return Err("native CPU cloth branch requires its prepared CPU rows".into());}
    if cpu_rows.is_some_and(|rows|rows.len()!=published.rows.len()) {return Err("CPU cloth root palette row count mismatch".into());}
    Ok(CharacterPoseRows {
        living_world,
        rows: cpu_rows.unwrap_or(published.rows),
        world_to_clip: &camera.clip_from_camera,
        camera_origin: camera.camera,
        source_generation: published.generation,
        // This override is internal to the verified rigid stream adapter;
        // the retained native +910 parameter remains zero in the material view.
        ropa_vertex_params_x:if cpu_branch {Some(1.)}else{ropa_vertex_params_x},
        ghost,
    })
}
fn validate_palette(
    identity: &CharacterPaletteIdentity,
    palette: &CharacterPalette,
) -> RenderResult<()> {
    if *identity != palette.identity() {
        return Err("character GPU update came from a different palette owner".into());
    }
    Ok(())
}

impl GpuRenderer {
    pub(crate) fn create_selected_character_object(
        &self,
        palette: &CharacterPalette,
        materials: &skate2_core::character::material_admission::CharacterMaterialAdmission,
        admission: CharacterMeshAdmission,
        camera: &CameraRelativeClip,
        diffuse: &GpuTexture,
        alpha: Option<&GpuTexture>,
        ghost: CharacterGhostState,
        living_world: Option<[[f32; 4]; 5]>,
    ) -> RenderResult<SelectedCharacterGpuObject> {
        let view = materials.view(palette, admission.part, admission.mesh)?;
        let expected = match admission.family {
            CharacterShaderFamily::Character => "defaultcharacter",
            CharacterShaderFamily::Hair => "defaulthair",
            CharacterShaderFamily::HairRopa => "defaulthair_ropa",
            CharacterShaderFamily::Ropa => "defaultcharacter_ropa",
            CharacterShaderFamily::LivingWorld => "defaultlivingworld",
            CharacterShaderFamily::LivingWorldStamp => "livingworld_stamp",
        };
        let cas_family = match view.effect {
            "cacstamp_cloth" | "cacstamp_skin" | "cacstamp_face" | "cacstamp_shift" => {
                Some(CharacterShaderFamily::Character)
            }
            "cacstamp_cloth_ropa" => Some(CharacterShaderFamily::Ropa),
            "cac_hair" | "cac_hair_ropa" => Some(CharacterShaderFamily::Hair),
            _ => None,
        };
        if view.effect != expected && cas_family != Some(admission.family) {
            return Err("GPU family differs from actual effect".into());
        }
        let mesh = view.geometry;
        if view.cpu_rows.is_some(){self.validate_cpu_character_stream(&mesh.geometry)?;}
        let pose = pose_rows(palette, admission, camera, view.ropa_vertex_params_x, view.cpu_rows,ghost,living_world)?;
        let mut object = self.create_character_object(
            &mesh.geometry,
            u16::try_from(mesh.skin_palette.len())?,
            admission.family,
            &pose,
            diffuse,
            alpha,
        )?;
        let object_identity = std::sync::Arc::new(());
        let character = object
            .character
            .as_mut()
            .ok_or("missing character GPU state")?;
        character.selected_identity = Some(std::sync::Arc::clone(&object_identity));
        character.draw_order = admission.draw_order;
        Ok(SelectedCharacterGpuObject {
            object,
            binding: SelectedCharacterGpuBinding {
                object_identity,
                palette: palette.identity(),
                admission,
            },
        })
    }
    pub(crate) fn update_selected_character_binding(
        &self,
        binding: &SelectedCharacterGpuBinding,
        object: &mut GpuObject,
        palette: &CharacterPalette,
        materials: &skate2_core::character::material_admission::CharacterMaterialAdmission,
        camera: &CameraRelativeClip,
        ghost: CharacterGhostState,
        living_world: Option<[[f32; 4]; 5]>,
    ) -> RenderResult<()> {
        validate_palette(&binding.palette, palette)?;
        validate_object_identity(
            &binding.object_identity,
            object
                .character
                .as_ref()
                .and_then(|c| c.selected_identity.as_ref()),
        )?;
        let view = materials.view(palette, binding.admission.part, binding.admission.mesh)?;
        let pose = pose_rows(
            palette,
            binding.admission,
            camera,
            view.ropa_vertex_params_x,
            view.cpu_rows,
            ghost,
            living_world,
        )?;
        if view.cpu_rows.is_some(){
            self.validate_cpu_character_stream(&view.geometry.geometry)?;
            self.update_cpu_character_vertices(object,&view.geometry.geometry,pose.source_generation)?;
        }
        self.update_character_object(object, &pose)
    }
}

#[cfg(test)]
#[path = "character_palette_tests.rs"]
mod tests;

impl SelectedCharacterGpuObject {
    pub(crate) fn into_parts(self) -> (SelectedCharacterGpuBinding, GpuObject) {
        (self.binding, self.object)
    }
}

fn validate_object_identity(
    expected: &std::sync::Arc<()>,
    actual: Option<&std::sync::Arc<()>>,
) -> RenderResult<()> {
    let actual = actual.ok_or("GPU object has no selected binding")?;
    if !std::sync::Arc::ptr_eq(expected, actual) {
        return Err("selected binding belongs to another GPU object".into());
    }
    Ok(())
}

impl SelectedCharacterGpuBinding {
    pub(crate) fn part_mesh(&self) -> (usize, usize) {
        (self.admission.part, self.admission.mesh)
    }
}
