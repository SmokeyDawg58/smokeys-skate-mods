//! The spawned LivingWorld DMOs as their `cPresDynamicObject` entities draw
//! them (docs/evidence/dmo-render-2026-09-28.md).
//!
//! - `cLivingWorldPresModelManager::AddDMOModel` 0x828107d8 keys each model's
//!   presentation `tInstance`; `CreatePresDynamicObject` 0x82811100 builds
//!   the entity from its `m_pRModel` (`AddLODModelInstances` 0x827fa998), one
//!   LOD and one Part.
//! - `cModelInstance::Resolve` takes each mesh's own material (optimesh
//!   +0x24); its `effect` and `DrawOrder` come from the material's Vault
//!   class, `m_params` from `cMaterialScope::Resolve`, and the channels from
//!   index 2 on bind by name (`AddPegasusMaterialToScope` 0x82e17bb0), a
//!   repeated name's last occurrence winning.
//! - The main DMO bank (`DMO_Pres`, one `cPres_Global` collection) keeps every
//!   RModel, optimesh, material and `tInstance` in one arena and each texture
//!   in its own; material rasters name textures by TOC GUID.
use crate::{
    gpu_renderer::{
        DynamicObjectDraw, DynamicObjectEffect, DynamicObjectMeshInput, GpuDynamicObject,
        GpuDynamicObjectModel, GpuRenderer, GpuTexture, RenderResult,
    },
    render_environment::EnvironmentLighting,
};
use skate2_core::{
    SelectedDynamicObjectVisual,
    mounted_character_scene::own_presentation_arena,
    presentation::{PresentationArena, PresentationMaterialBinding, PresentationMaterialValue},
    presentation_restore::PresentationRestorer,
    texture::NativeTexture,
};
use skate2_formats::{
    mount::MountSet,
    rw4::Arena as Rw4Arena,
    stream::{CollectionMap, StreamFile, decode_arena},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// `cDMOBankManager::LoadMainDMOBank`'s presentation manifest.
const BANK_MANIFEST: &str = "data\\content\\world\\dmo\\DMO\\DMO_Pres";
const BANK_DIRECTORY: &str = "data\\content\\world\\dmo\\DMO";
/// The sampled channels in bind order: `i_diffuse`, `i_specular`,
/// `i_normal`, `i_detail`, `i_environment`.
const CHANNELS: [&str; 5] = ["diffuse", "specular", "normal", "detail", "environment"];

/// The decoded bank: presentation arenas by asset ID, texture GUIDs and the
/// materials' Vault bindings.
struct Bank {
    arenas: BTreeMap<u64, PresentationArena>,
    /// TOC GUID -> (asset ID, texture index).
    textures: BTreeMap<u64, (u64, usize)>,
    /// Material class/variant -> manifest and draw order.
    bindings: BTreeMap<(u64, u64), PresentationMaterialBinding>,
}

impl Bank {
    fn load(mounts: &MountSet) -> RenderResult<Self> {
        let collections =
            CollectionMap::parse(&mounts.read_path(&format!("{BANK_MANIFEST}.xsm"))?)?;
        let [collection] = collections.collections.as_slice() else {
            return Err("DMO bank manifest does not hold exactly one collection".into());
        };
        let path = format!("{BANK_DIRECTORY}\\{}.xsf", collection.filename);
        let bytes = mounts.read_path(&path)?;
        let stream = StreamFile::parse(&bytes, collection.asset_count)?;
        let mut arenas = BTreeMap::new();
        let mut textures = BTreeMap::new();
        for block in &stream.assets {
            let payload = bytes
                .get(block.data_range.clone())
                .ok_or("DMO bank block lies outside its XSF")?;
            let resources = decode_arena(payload)?;
            let arena = Rw4Arena::parse(&resources.resources)?;
            let owned = own_presentation_arena(&arena, &resources.resources)?;
            for (index, texture) in owned.textures.iter().enumerate() {
                textures.insert(texture.guid, (block.id, index));
            }
            arenas.insert(block.id, owned);
        }
        let keys = arenas
            .values()
            .flat_map(|arena| &arena.materials)
            .filter_map(|material| material.effect.as_ref())
            .filter_map(|effect| effect.attribute_class_id.zip(effect.material_variant_id))
            .collect::<BTreeSet<_>>();
        let bindings = crate::load_material_attribute_bindings(mounts, &keys)?;
        Ok(Self {
            arenas,
            textures,
            bindings,
        })
    }

    fn texture(&self, guid: u64) -> Option<&NativeTexture> {
        let (asset, index) = self.textures.get(&guid)?;
        self.arenas.get(asset)?.textures.get(*index)
    }
}

#[derive(Clone)]
struct MaterialBinding {
    binding: Arc<wgpu::BindGroup>,
    effect: DynamicObjectEffect,
    draw_order: u8,
}

/// GPU objects shared by every model.
#[derive(Default)]
struct Uploads {
    textures: BTreeMap<u64, GpuTexture>,
    materials: BTreeMap<(u64, usize), MaterialBinding>,
}

/// Retained GPU state for the spawned DMOs.
pub(crate) struct DmoRender {
    mounts: Arc<MountSet>,
    environment: Arc<EnvironmentLighting>,
    bank: Option<Bank>,
    /// The bank decoding on its own thread from construction, taken at the first DMO
    /// drawn after it finishes.
    bank_load: Option<std::thread::JoinHandle<Result<Bank, String>>>,
    uploads: Uploads,
    models: BTreeMap<(u64, u32), Arc<GpuDynamicObjectModel>>,
    /// Models whose admission failed, reported once each.
    failed: BTreeSet<(u64, u32)>,
    /// Entity ID -> (model key, entity constant).
    objects: BTreeMap<u32, ((u64, u32), Arc<GpuDynamicObject>)>,
    opacity: BTreeMap<u32, skate2_core::LivingWorldOpacityState>,
}

impl DmoRender {
    pub(crate) fn new(mounts: Arc<MountSet>, environment: Arc<EnvironmentLighting>) -> Self {
        let load_mounts = Arc::clone(&mounts);
        let bank_load = std::thread::Builder::new()
            .name("sk8 DMO bank".to_owned())
            .spawn(move || Bank::load(&load_mounts).map_err(|error| error.to_string()))
            .ok();
        Self {
            mounts,
            environment,
            bank: None,
            bank_load,
            uploads: Uploads::default(),
            models: BTreeMap::new(),
            failed: BTreeSet::new(),
            objects: BTreeMap::new(),
            opacity: BTreeMap::new(),
        }
    }

    /// Replaces the scene's DMO draws with this frame's spawned entities, in
    /// entity-map order, uploading each RModel on first use.
    pub(crate) fn update(
        &mut self,
        renderer: &GpuRenderer,
        draws: &mut Vec<DynamicObjectDraw>,
        visuals: &[SelectedDynamicObjectVisual],
        camera_position: [f32; 3],
        restore: Option<&PresentationRestorer>,
    ) -> RenderResult<()> {
        draws.clear();
        if visuals.is_empty() {
            self.objects.clear();
            self.opacity.clear();
            return Ok(());
        }
        if self.bank.is_none() {
            // A bank still decoding draws no DMO this frame rather than holding the frame.
            if self.bank_load.as_ref().is_some_and(|load| !load.is_finished()) {
                return Ok(());
            }
            let bank = match self.bank_load.take().map(std::thread::JoinHandle::join) {
                Some(Ok(loaded)) => loaded?,
                Some(Err(_)) => return Err("DMO bank loader panicked".into()),
                None => Bank::load(&self.mounts)?,
            };
            eprintln!(
                "LivingWorld DMO presentation: {} bank arenas, {} textures, {} material bindings",
                bank.arenas.len(),
                bank.textures.len(),
                bank.bindings.len()
            );
            self.bank = Some(bank);
        }
        let mut live = BTreeSet::new();
        for visual in visuals {
            let key = (visual.asset_id, visual.render_model_entry);
            let Some(model) = self.model(renderer, key) else {
                continue;
            };
            let object = match self.objects.get(&visual.entity_id) {
                Some((retained, object)) if *retained == key => Arc::clone(object),
                _ => {
                    let object = renderer.create_dynamic_object(model);
                    self.opacity.insert(visual.entity_id, Default::default());
                    self.objects
                        .insert(visual.entity_id, (key, Arc::clone(&object)));
                    object
                }
            };
            live.insert(visual.entity_id);
            let mut draw = DynamicObjectDraw {
                object,
                // cPresDynamicObject::UpdateFromReplayData 0x8280a040's part, else the tick's.
                part_transform: restore
                    .and_then(|restore| restore.dynamic_object(visual.entity_id))
                    .unwrap_or(visual.part_transform),
                opacity: 1.0,
                ghosted: false,
                vehicle: None,
                hips: None,
            };
            let opacity = self
                .opacity
                .get_mut(&visual.entity_id)
                .ok_or("DMO presentation opacity owner absent")?;
            opacity.try_update(
                visual.opacity_settings,
                visual.opacity_parameters,
                visual.simulation_center_velocity,
                draw.bounds_center(),
                camera_position,
            )?;
            draw.opacity = opacity.opacity;
            draw.ghosted = opacity.ghosted;
            draws.push(draw);
        }
        self.objects.retain(|entity, _| live.contains(entity));
        self.opacity.retain(|entity, _| live.contains(entity));
        Ok(())
    }

    /// The uploaded RModel, or None once its admission has failed; the
    /// failure is reported the first time.
    fn model(
        &mut self,
        renderer: &GpuRenderer,
        key: (u64, u32),
    ) -> Option<Arc<GpuDynamicObjectModel>> {
        if let Some(model) = self.models.get(&key) {
            return Some(Arc::clone(model));
        }
        if self.failed.contains(&key) {
            return None;
        }
        let uploaded = match self.bank.as_ref() {
            Some(bank) => upload_model(renderer, bank, &self.environment, &mut self.uploads, key),
            None => Err("DMO bank not loaded".into()),
        };
        match uploaded {
            Ok(model) => {
                self.models.insert(key, Arc::clone(&model));
                Some(model)
            }
            Err(error) => {
                // A failed model admission is an error, not routine tracing.
                // Keep the existing once-per-model report visible in --quiet.
                std::eprintln!(
                    "LivingWorld DMO model {:016x}/{} not drawn: {error}",
                    key.0, key.1
                );
                self.failed.insert(key);
                None
            }
        }
    }
}

/// One RModel: every mesh with its own material, and the model bounds.
fn upload_model(
    renderer: &GpuRenderer,
    bank: &Bank,
    environment: &EnvironmentLighting,
    uploads: &mut Uploads,
    (asset, entry): (u64, u32),
) -> RenderResult<Arc<GpuDynamicObjectModel>> {
    let arena = bank
        .arenas
        .get(&asset)
        .ok_or("DMO model asset is not in the bank")?;
    let model = arena
        .models
        .iter()
        .find(|model| model.dictionary_index == entry)
        .ok_or("DMO RModel entry is not a presentation model")?;
    let mut inputs = Vec::with_capacity(model.meshes.len());
    for model_mesh in &model.meshes {
        let mesh = arena
            .meshes
            .get(model_mesh.mesh_index)
            .ok_or("DMO model mesh index is out of range")?;
        let material = mesh.material.ok_or("DMO mesh has no material")?;
        let material = material_binding(renderer, bank, environment, uploads, asset, material)?;
        inputs.push(DynamicObjectMeshInput {
            geometry: &mesh.geometry,
            material: material.binding,
            effect: material.effect,
            draw_order: material.draw_order,
        });
    }
    let minimum = model.bounding_box.minimum;
    let maximum = model.bounding_box.maximum;
    renderer.upload_dynamic_object_model(
        inputs,
        [
            [minimum[0], minimum[1], minimum[2]],
            [maximum[0], maximum[1], maximum[2]],
        ],
    )
}

/// One material's effect, draw order, `m_params[0..2]`, sampled channels
/// and `i_detailNormalUVScale`. An absent input is an error: the original
/// would read a previous draw's stale value.
fn material_binding(
    renderer: &GpuRenderer,
    bank: &Bank,
    environment: &EnvironmentLighting,
    uploads: &mut Uploads,
    asset: u64,
    index: usize,
) -> RenderResult<MaterialBinding> {
    if let Some(cached) = uploads.materials.get(&(asset, index)) {
        return Ok(cached.clone());
    }
    let material = bank
        .arenas
        .get(&asset)
        .and_then(|arena| arena.materials.get(index))
        .ok_or("DMO mesh material index is out of range")?;
    let effect = material
        .effect
        .as_ref()
        .ok_or("DMO material has no Attribulator material name")?;
    let key = effect
        .attribute_class_id
        .zip(effect.material_variant_id)
        .ok_or("DMO material name has no Vault class and variant")?;
    let binding = bank
        .bindings
        .get(&key)
        .ok_or("DMO material has no Vault binding")?;
    let dmo_effect = DynamicObjectEffect::from_manifest(&binding.manifest_name)
        .ok_or_else(|| format!("DMO effect {:?} is not ported", binding.manifest_name))?;
    let draw_order = u8::try_from(binding.draw_order).map_err(|_| {
        format!(
            "DMO draw order {} is not a sort-key byte",
            binding.draw_order
        )
    })?;
    let m_params = environment
        .material_params_pair(key.0, key.1)
        .ok_or("DMO material m_params[0..2] unresolved")?;
    let channels = material.channels.get(2..).unwrap_or_default();
    let find = |name: &str| {
        channels
            .iter()
            .rev()
            .find(|channel| channel.shader_input == name)
    };
    let mut guids = [0_u64; 5];
    for (guid, name) in guids.iter_mut().zip(CHANNELS) {
        *guid = match find(name).map(|channel| &channel.value) {
            Some(PresentationMaterialValue::Raster { guid, .. }) => *guid,
            Some(PresentationMaterialValue::ScalarConstant(_)) => {
                return Err(format!("DMO {name} channel is a scalar").into());
            }
            None => return Err(format!("DMO material has no {name} channel").into()),
        };
        if !uploads.textures.contains_key(guid) {
            let native = bank
                .texture(*guid)
                .ok_or_else(|| format!("DMO texture {guid:016x} is not in the bank"))?;
            uploads
                .textures
                .insert(*guid, renderer.upload_native_texture(native)?);
        }
    }
    let detail_normal_uv_scale = match find("detailNormalUVScale").map(|channel| &channel.value) {
        Some(PresentationMaterialValue::ScalarConstant(lanes)) => *lanes,
        _ => return Err("DMO material has no detailNormalUVScale scalar".into()),
    };
    let textures = &uploads.textures;
    let bind_group = renderer.create_dynamic_object_material(
        [
            &textures[&guids[0]],
            &textures[&guids[1]],
            &textures[&guids[2]],
            &textures[&guids[3]],
            &textures[&guids[4]],
        ],
        m_params,
        detail_normal_uv_scale,
    )?;
    let result = MaterialBinding {
        binding: bind_group,
        effect: dmo_effect,
        draw_order,
    };
    uploads.materials.insert((asset, index), result.clone());
    Ok(result)
}
