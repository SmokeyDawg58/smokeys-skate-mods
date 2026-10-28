//! Vehicles from the completed simulation roster, sharing the existing scene,
//! palette, texture upload, material sorting and shadow passes with other draws.
use crate::{
    gpu_renderer::{
        DynamicObjectDraw, DynamicObjectEffect, DynamicObjectMeshInput, GpuDynamicObject,
        GpuDynamicObjectModel, GpuRenderer, GpuTexture, RenderResult, VehicleDrawConstants,
        vehicle_bone_index,
    },
    render_environment::EnvironmentLighting,
};
use skate2_core::{
    LivingWorldOpacityState, SelectedVehicleVisual,
    character::{CharacterScene, palette::CharacterPalette},
    presentation::{PresentationMaterial, PresentationMaterialValue},
    presentation_restore::PresentationRestorer,
    vehicle_animation::VehicleAnimation,
};
use skate2_formats::{andale::AndaleDatabase, mount::MountSet};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

struct Mesh {
    part: usize,
    mesh: usize,
    model: Arc<GpuDynamicObjectModel>,
    maximum_bone: u8,
}
struct Entity {
    recipe: u64,
    lod: u8,
    animation: VehicleAnimation,
    objects: Vec<Arc<GpuDynamicObject>>,
    opacity: LivingWorldOpacityState,
}
pub(crate) struct VehicleRender {
    mounts: Arc<MountSet>,
    environment: Arc<EnvironmentLighting>,
    database: Option<AndaleDatabase>,
    meshes: BTreeMap<(u64, u8), Arc<Vec<Mesh>>>,
    textures: BTreeMap<u64, GpuTexture>,
    entities: BTreeMap<u32, Entity>,
}
impl VehicleRender {
    pub(crate) fn new(mounts: Arc<MountSet>, environment: Arc<EnvironmentLighting>) -> Self {
        Self {
            mounts,
            environment,
            database: None,
            meshes: BTreeMap::new(),
            textures: BTreeMap::new(),
            entities: BTreeMap::new(),
        }
    }
    pub(crate) fn update(
        &mut self,
        renderer: &GpuRenderer,
        draws: &mut Vec<DynamicObjectDraw>,
        visuals: &[SelectedVehicleVisual],
        camera: [f32; 3],
        restore: Option<&PresentationRestorer>,
    ) -> RenderResult<()> {
        if visuals.is_empty() {
            self.entities.clear();
            self.meshes.clear();
            self.textures.clear();
            return Ok(());
        }
        if self.database.is_none() {
            self.database = Some(AndaleDatabase::parse(
                &self.mounts.read_path("data\\anim\\VehiclesDB.abin")?,
            )?);
        }
        let mut alive = BTreeSet::new();
        let mut recipes = BTreeSet::new();
        for visual in visuals {
            alive.insert(visual.entity_id);
            // LivingWorldPresentation::DoSimRestore 0x82813a90's blend, else the tick's.
            let (transform, wheels) = restore
                .and_then(|restore| restore.vehicle(visual.entity_id))
                .unwrap_or((visual.transform, visual.wheels));
            // SceneRenderView.UpdateEntityLOD82826a10: camera distance,
            // followed by PresentationEntity.UpdateLOD's missing-level fallback.
            let lod = visual.scene.distance_lod(
                [transform[12],transform[13],transform[14],1.],
                [camera[0],camera[1],camera[2],1.],self.environment.vehicle_lod_squared)?;
            let mesh_key = (visual.recipe_id, lod);
            // Keep all uploaded LODs of a resident recipe. Discarding the last
            // unused LOD here caused a fresh decode/upload on each boundary
            // crossing. Actual LOD selection and entity animation still update.
            recipes.insert(visual.recipe_id);
            let replace = self
                .entities
                .get(&visual.entity_id)
                .is_none_or(|e| e.recipe != visual.recipe_id || e.lod != lod);
            if replace {
                if std::env::var_os("SKATE2_VEHICLE_MATERIAL_TRACE").is_some() {
                    std::eprintln!(
                        "vehicle appearance id={} model={:016x} recipe={:016x} pose={} lod={} colours={:?} irradiance={:?}",
                        visual.entity_id,
                        visual.model_key,
                        visual.recipe_id,
                        visual.pose_name,
                        lod,
                        visual.colours,
                        visual.irradiance
                    );
                }
                let animation = VehicleAnimation::new(
                    self.database.as_ref().expect("loaded"),
                    &visual.pose_name,
                    Arc::clone(&visual.scene),
                    lod,
                )?;
                if !self.meshes.contains_key(&mesh_key) {
                    let meshes = self.upload(renderer, &animation.palette, None)?;
                    self.meshes.insert(mesh_key, Arc::new(meshes));
                }
                let meshes = &self.meshes[&mesh_key];
                let opacity = self
                    .entities
                    .remove(&visual.entity_id)
                    .map(|e| e.opacity)
                    .unwrap_or_default();
                self.entities.insert(
                    visual.entity_id,
                    Entity {
                        recipe: visual.recipe_id,
                        lod,
                        animation,
                        objects: meshes
                            .iter()
                            .map(|m| renderer.create_dynamic_object(Arc::clone(&m.model)))
                            .collect(),
                        opacity,
                    },
                );
            }
            let entity = self
                .entities
                .get_mut(&visual.entity_id)
                .ok_or("vehicle presentation membership")?;
            entity.animation.update(&wheels, transform)?;
            let meshes = self
                .meshes
                .get(&mesh_key)
                .ok_or("vehicle presentation recipe meshes")?;
            // One opacity state per entity, not once per mesh/material.
            let first = entity
                .objects
                .first()
                .ok_or("vehicle recipe contains no meshes")?;
            let probe = DynamicObjectDraw {
                object: Arc::clone(first),
                part_transform: transform,
                opacity: 1.,
                ghosted: false,
                vehicle: None,
                hips: None,
            };
            entity.opacity.try_update(
                visual.opacity_settings,
                visual.opacity_parameters,
                visual.simulation_center_velocity,
                probe.bounds_center(),
                camera,
            )?;
            for (object, mesh) in entity.objects.iter().zip(meshes.iter()) {
                let rows = entity
                    .animation
                    .palette
                    .mesh_rows(mesh.part, mesh.mesh)?
                    .rows;
                if rows.len() > 61 * 3 {
                    return Err("vehicle palette exceeds shader's 61 bones".into());
                }
                if usize::from(mesh.maximum_bone) >= rows.len() / 3 {
                    return Err(format!(
                        "vehicle {} LOD{} mesh{}/{} vertex bone {} outside {}-bone palette",
                        visual.pose_name,
                        lod,
                        mesh.part,
                        mesh.mesh,
                        mesh.maximum_bone,
                        rows.len() / 3
                    )
                    .into());
                }
                draws.push(DynamicObjectDraw {
                    object: Arc::clone(object),
                    part_transform: transform,
                    opacity: entity.opacity.opacity,
                    ghosted: entity.opacity.ghosted,
                    vehicle: Some(VehicleDrawConstants {
                        colours: visual.colours,
                        lights: wheels.lights,
                        irradiance: visual.irradiance,
                        rows: rows.to_vec(),
                    }),
                    hips: None,
                });
            }
        }
        self.entities.retain(|id, _| alive.contains(id));
        self.meshes
            .retain(|(recipe, _), _| recipes.contains(recipe));
        // Textures remain shared across resident recipes; retire unused ones.
        let used = visuals
            .iter()
            .flat_map(|v| v.scene.textures.iter().map(|t| t.source_asset_id))
            .collect::<BTreeSet<_>>();
        self.textures.retain(|id, _| used.contains(id));
        Ok(())
    }
    /// A NIS vehicle's `SkaterPresEntity` culls by its own `m_localBB`
    /// (`local_bounds`) instead of the LOD0 union.
    fn upload(
        &mut self,
        renderer: &GpuRenderer,
        palette: &CharacterPalette,
        local_bounds: Option<[[f32; 3]; 2]>,
    ) -> RenderResult<Vec<Mesh>> {
        let scene = palette.scene();
        let keys = scene
            .components
            .iter()
            .flat_map(|c| &c.models)
            .flat_map(|m| &m.lods)
            .flat_map(|l| &l.arena.materials)
            .filter_map(|m| m.effect.as_ref())
            .filter_map(|e| e.attribute_class_id.zip(e.material_variant_id))
            .collect::<BTreeSet<_>>();
        let bindings = crate::load_material_attribute_bindings(&self.mounts, &keys)?;
        // CalculateBoundsInfo827f2708 unions the LOD0 model bounds once for
        // the entity, including when a lower-detail mesh is being drawn.
        let mut part_bounds = scene
            .components
            .iter()
            .flat_map(|c| &c.models)
            .flat_map(|m| &m.lods)
            .filter(|l| l.level == 0)
            .filter_map(|l| {
                l.arena
                    .instances
                    .iter()
                    .find_map(|i| i.render_model)
                    .and_then(|i| l.arena.models.get(i))
            })
            .map(|m| {
                [
                    std::array::from_fn::<_, 3, _>(|i| m.bounding_box.minimum[i]),
                    std::array::from_fn::<_, 3, _>(|i| m.bounding_box.maximum[i]),
                ]
            });
        let mut bounds = part_bounds.next().ok_or("vehicle LOD0 bounds absent")?;
        for part in part_bounds {
            for i in 0..3 {
                if part[0][i] < bounds[0][i] {
                    bounds[0][i] = part[0][i];
                }
                if part[1][i] > bounds[1][i] {
                    bounds[1][i] = part[1][i];
                }
            }
        }
        let bounds = local_bounds.unwrap_or(bounds);
        let mut result = Vec::new();
        for (part_index, part) in palette.skinning().parts.iter().enumerate() {
            let lod = &scene.components[part.component_index].models[part.model_index].lods
                [part.lod_index];
            let model_index = lod
                .arena
                .instances
                .iter()
                .find_map(|i| i.render_model)
                .ok_or("vehicle tInstance RModel absent")?;
            let model = lod
                .arena
                .models
                .get(model_index)
                .ok_or("vehicle RModel absent")?;
            for (mesh_index, link) in model.meshes.iter().enumerate() {
                let mesh = lod
                    .arena
                    .meshes
                    .get(link.mesh_index)
                    .ok_or("vehicle model mesh absent")?;
                let material = mesh
                    .material
                    .and_then(|i| lod.arena.materials.get(i))
                    .ok_or("vehicle mesh material absent")?;
                let effect = material
                    .effect
                    .as_ref()
                    .ok_or("vehicle material metadata absent")?;
                let key = effect
                    .attribute_class_id
                    .zip(effect.material_variant_id)
                    .ok_or("vehicle material key absent")?;
                let binding = bindings
                    .get(&key)
                    .ok_or("vehicle material Vault binding absent")?;
                let effect = DynamicObjectEffect::from_manifest(&binding.manifest_name)
                    .ok_or_else(|| {
                        format!(
                            "vehicle material effect {} unsupported",
                            binding.manifest_name
                        )
                    })?;
                if !matches!(
                    effect,
                    DynamicObjectEffect::Vehicle
                        | DynamicObjectEffect::VehicleGlass
                        | DynamicObjectEffect::VehicleLight
                ) {
                    return Err("vehicle recipe selected a non-vehicle shader".into());
                }
                let gpu = self.material(renderer, scene, material, key, effect)?;
                use skate2_formats::render::{NativeVertexAttributeValues, VertexSemantic};
                let indices = mesh
                    .geometry
                    .attributes
                    .iter()
                    .find(|a| a.semantic == VertexSemantic::BlendIndices(0))
                    .ok_or("vehicle BLENDINDICES absent")?;
                let NativeVertexAttributeValues::UnsignedByte4(indices) = &indices.values else {
                    return Err("vehicle BLENDINDICES are not bytes".into());
                };
                let maximum_bone = indices
                    .iter()
                    .map(vehicle_bone_index)
                    .max()
                    .ok_or("vehicle mesh has no bone indices")?;
                let uploaded = renderer.upload_dynamic_object_model(
                    vec![DynamicObjectMeshInput {
                        geometry: &mesh.geometry,
                        material: gpu,
                        effect,
                        draw_order: u8::try_from(binding.draw_order)?,
                    }],
                    bounds,
                )?;
                result.push(Mesh {
                    part: part_index,
                    mesh: mesh_index,
                    model: uploaded,
                    maximum_bone,
                });
            }
        }
        Ok(result)
    }
    fn material(
        &mut self,
        renderer: &GpuRenderer,
        scene: &CharacterScene,
        material: &PresentationMaterial,
        key: (u64, u64),
        effect: DynamicObjectEffect,
    ) -> RenderResult<Arc<wgpu::BindGroup>> {
        let channel = |name: &str| -> RenderResult<u64> {
            let value = material
                .channels
                .get(2..)
                .unwrap_or_default()
                .iter()
                .rev()
                .find(|c| c.shader_input == name)
                .map(|c| &c.value);
            match value {
                Some(PresentationMaterialValue::Raster { guid, .. }) => Ok(*guid),
                _ => Err(format!("vehicle material {name} texture absent").into()),
            }
        };
        // The common material layout has two unused 2D slots. Alias an actual
        // texture for those slots; no substitute is sampled by vehicle shaders.
        let normal = if effect == DynamicObjectEffect::VehicleLight {
            // vehiclelight.fpo samples diffuse only. Its optimized material
            // need not contain a normal channel; this binding is never read.
            channel("diffuse")?
        } else {
            channel("normal")?
        };
        let diffuse = if effect == DynamicObjectEffect::VehicleGlass {
            normal
        } else {
            channel("diffuse")?
        };
        let environment = channel("environment").or_else(|e| {
            if effect != DynamicObjectEffect::VehicleLight {
                return Err(e);
            }
            scene
                .textures
                .iter()
                .find(|t| {
                    matches!(
                        t.texture.texture_type,
                        skate2_formats::material::TextureType::Cube
                    )
                })
                .map(|t| t.source_asset_id)
                .ok_or(e)
        })?;
        for id in [diffuse, normal, environment] {
            if !self.textures.contains_key(&id) {
                let asset = scene
                    .textures
                    .iter()
                    .find(|t| t.source_asset_id == id)
                    .ok_or_else(|| format!("vehicle texture {id:016x} not resident"))?;
                self.textures
                    .insert(id, renderer.upload_native_texture(&asset.texture)?);
            }
        }
        let source = self
            .environment
            .material_parameter_array(key.0, key.1)
            .ok_or("vehicle m_params absent")?;
        if std::env::var_os("SKATE2_VEHICLE_MATERIAL_TRACE").is_some() {
            std::eprintln!(
                "vehicle material key={key:016x?} effect={effect:?} diffuse={diffuse:016x} normal={normal:016x} environment={environment:016x} params={source:?}"
            );
        }
        let mut params = [[0.; 4]; 8];
        for (to, from) in params.iter_mut().zip(source) {
            *to = *from;
        }
        if source.len()
            < if effect == DynamicObjectEffect::VehicleLight {
                2
            } else {
                6
            }
        {
            return Err("vehicle m_params too short".into());
        }
        let char_params = &self.environment.character_attributes;
        let bias = char_params.first().ok_or("g_charattributes[0] absent")?[0];
        let shadow_radius = char_params[0][2];
        let bloom = char_params.get(2).ok_or("g_charattributes[2] absent")?[2];
        renderer.create_vehicle_material(
            [
                &self.textures[&diffuse],
                &self.textures[&diffuse],
                &self.textures[&normal],
                &self.textures[&diffuse],
                &self.textures[&environment],
            ],
            params,
            [bias, bloom, shadow_radius, 0.],
        )
    }
}

/// A NIS scene character's Vehicle recipe: a `ColorizedSkaterPresEntity`
/// whose meshes take the vehicle effects, drawn with the traffic vehicles'
/// shaders and the entity's own constants.
pub(crate) struct SceneVehicle {
    meshes: Vec<Mesh>,
    objects: Vec<Arc<GpuDynamicObject>>,
}

impl VehicleRender {
    /// The NIS vehicle's meshes at its palette's LOD.
    pub(crate) fn scene_vehicle(
        &mut self,
        renderer: &GpuRenderer,
        palette: &CharacterPalette,
        local_bounds: [[f32; 3]; 2],
    ) -> RenderResult<SceneVehicle> {
        let meshes = self.upload(renderer, palette, Some(local_bounds))?;
        let objects = meshes
            .iter()
            .map(|m| renderer.create_dynamic_object(Arc::clone(&m.model)))
            .collect();
        Ok(SceneVehicle { meshes, objects })
    }
}

impl SceneVehicle {
    /// This frame's draws. `ColorizedSkaterPresEntity::BindConstants`
    /// (0x827dd1a0) gives the vehicle effect `i_colorize_red`/`_blue` from
    /// `mCSColours` and `SkaterPresEntity::BindConstants` (0x827dcf50) the
    /// entity's own `i_shmult` and `i_params` (full opacity, as every scene
    /// character). Neither binds `i_isLightOn`, which only
    /// `cPresVehicle::BindConstants` (0x8280b8f8) does, so
    /// `cScope::DispatchParamsHelper` (0x82e14aa0) leaves its register as the
    /// last draw set it; zero (lights off) stands in for that.
    #[expect(clippy::too_many_arguments, reason = "one scene character's pose, constants and draw list")]
    pub(crate) fn draws(
        &self,
        palette: &CharacterPalette,
        root: [f32; 16],
        hips: [f32; 3],
        colours: [[f32; 4]; 2],
        irradiance: [[f32; 4]; 9],
        draws: &mut Vec<DynamicObjectDraw>,
    ) -> RenderResult<()> {
        for (object, mesh) in self.objects.iter().zip(&self.meshes) {
            let rows = palette.mesh_rows(mesh.part, mesh.mesh)?.rows;
            if rows.len() > 61 * 3 {
                return Err("NIS vehicle palette exceeds shader's 61 bones".into());
            }
            if usize::from(mesh.maximum_bone) >= rows.len() / 3 {
                return Err(format!(
                    "NIS vehicle mesh{}/{} vertex bone {} outside {}-bone palette",
                    mesh.part,
                    mesh.mesh,
                    mesh.maximum_bone,
                    rows.len() / 3
                )
                .into());
            }
            draws.push(DynamicObjectDraw {
                object: Arc::clone(object),
                part_transform: root,
                opacity: 1.0,
                ghosted: false,
                vehicle: Some(VehicleDrawConstants {
                    colours,
                    lights: [0.0; 4],
                    irradiance,
                    rows: rows.to_vec(),
                }),
                hips: Some(hips),
            });
        }
        Ok(())
    }
}

/// Whether a colorized scene character's meshes take the vehicle effects (a
/// Vehicle recipe's) rather than the living world ones.
pub(crate) fn takes_vehicle_effects(mounts: &MountSet, palette: &CharacterPalette) -> RenderResult<bool> {
    let scene = palette.scene();
    let mut keys = BTreeSet::new();
    for part in &palette.skinning().parts {
        let lod = &scene.components[part.component_index].models[part.model_index].lods[part.lod_index];
        let model = lod
            .arena
            .instances
            .iter()
            .find_map(|i| i.render_model)
            .and_then(|i| lod.arena.models.get(i))
            .ok_or("scene character RModel absent")?;
        for link in &model.meshes {
            let mesh = lod.arena.meshes.get(link.mesh_index).ok_or("scene character mesh absent")?;
            let effect = mesh
                .material
                .and_then(|i| lod.arena.materials.get(i))
                .and_then(|m| m.effect.as_ref())
                .ok_or("scene character material metadata absent")?;
            keys.insert(
                effect
                    .attribute_class_id
                    .zip(effect.material_variant_id)
                    .ok_or("scene character material Vault key absent")?,
            );
        }
    }
    let bindings = crate::load_material_attribute_bindings(mounts, &keys)?;
    let vehicle = bindings
        .values()
        .filter(|b| {
            matches!(
                DynamicObjectEffect::from_manifest(&b.manifest_name),
                Some(DynamicObjectEffect::Vehicle | DynamicObjectEffect::VehicleGlass | DynamicObjectEffect::VehicleLight)
            )
        })
        .count();
    match vehicle {
        0 => Ok(false),
        n if n == bindings.len() => Ok(true),
        _ => Err("a scene character mixes vehicle and living world effects".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicObjectEffect;
    use std::collections::BTreeSet;

    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT archives"]
    fn the_nis_car_takes_only_the_vehicle_effects() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
        let mounts = crate::mount_reference_archives(&root).expect("original archives");
        let path = skate2_core::nis::characters::recipe_path("vehicle", "reda_car");
        let mut recipe = skate2_formats::recipe::CharacterRecipe::parse(&mounts.read_path(&path).unwrap()).unwrap();
        recipe.category = 2;
        let scene = skate2_core::mounted_character_scene::own_character_scene_category(&mounts, &recipe, "vehicle").unwrap();
        let keys = scene
            .components
            .iter()
            .flat_map(|c| &c.models)
            .flat_map(|m| &m.lods)
            .flat_map(|l| &l.arena.materials)
            .filter_map(|m| m.effect.as_ref())
            .filter_map(|e| e.attribute_class_id.zip(e.material_variant_id))
            .collect::<BTreeSet<_>>();
        let bindings = crate::load_material_attribute_bindings(&mounts, &keys).unwrap();
        let names = bindings.values().map(|b| b.manifest_name.as_str()).collect::<BTreeSet<_>>();
        eprintln!("reda_car effects {names:?}");
        assert!(names.iter().all(|name| matches!(
            DynamicObjectEffect::from_manifest(name),
            Some(DynamicObjectEffect::Vehicle | DynamicObjectEffect::VehicleGlass | DynamicObjectEffect::VehicleLight)
        )));
    }
}
