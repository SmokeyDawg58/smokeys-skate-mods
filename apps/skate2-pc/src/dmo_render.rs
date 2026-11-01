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
        let pieces = load_park_pieces(mounts, &mut arenas, &mut textures)?;
        if pieces > 0 {
            eprintln!("park editor: {pieces} Skate 3 park pieces ready to draw");
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

/// The texture Skate 3 park materials name for their detail normal (`default_normal`).
const PARK_DEFAULT_NORMAL: u64 = 0x2f83_15d7_5f82_c2f1;

/// A Skate 3 park material's decal texture, when it carries one in its UV0.zw: an
/// `environmentpark.decal` material names a `decal` raster and sets `embeddedDecal`.
fn park_decal(material: &skate2_core::presentation::PresentationMaterial) -> Option<u64> {
    let channels = material.channels.get(2..).unwrap_or_default();
    let find = |name: &str| channels.iter().rev().find(|channel| channel.shader_input == name).map(|channel| &channel.value);
    let embedded = matches!(find("embeddedDecal"), Some(PresentationMaterialValue::ScalarConstant(lanes)) if lanes[0] != 0.0);
    match find("decal") {
        Some(PresentationMaterialValue::Raster { guid, .. }) if embedded && *guid != 0 => Some(*guid),
        _ => None,
    }
}

/// Skate 3's park editor pieces (`PARK_PIECES`), when its archive is beside the game's: each
/// piece's arena under its catalog ID, with every texture its materials name. Their
/// `environmentpark` materials have no Skate 2 Vault class, so each takes the `dynamicobject`
/// effect (alpha-tested ones its `alphatest` variant) and the channels it samples that the
/// piece's material lacks: the bank's environment map, the default detail normal (as the
/// specular or normal map too, when absent), a unit detail scale. Meshes with a normal and no
/// binormal get one (the effect's normal is `cross(tangent, binormal)`). An embedded decal's
/// UV0.zw moves to UV1, where the `Decal` draw reads it. Returns the number added.
fn load_park_pieces(
    mounts: &MountSet,
    arenas: &mut BTreeMap<u64, PresentationArena>,
    textures: &mut BTreeMap<u64, (u64, usize)>,
) -> RenderResult<usize> {
    use skate2_core::presentation::{PresentationMaterialChannel, PresentationMaterialEffect};
    use skate2_core::ui::object_dropper::{PARK_PIECES, park_assets_archive, read_park_arena};
    use skate2_formats::render::{NativeVertexAttribute, NativeVertexAttributeValues, VertexSemantic};
    let Some(archive) = park_assets_archive(mounts) else {
        return Ok(0);
    };
    let environment = arenas
        .values()
        .flat_map(|arena| &arena.materials)
        .flat_map(|material| &material.channels)
        .find_map(|channel| match &channel.value {
            PresentationMaterialValue::Raster { guid, .. } if channel.shader_input == "environment" && *guid != 0 => Some(*guid),
            _ => None,
        })
        .ok_or("the DMO bank names no environment map for park pieces")?;
    let raster = |name: &str, guid: u64| PresentationMaterialChannel {
        shader_input: name.to_owned(),
        flags: 0,
        image_channel: u16::MAX,
        value: PresentationMaterialValue::Raster { guid, stream_name: String::new(), resident_texture: None },
    };
    let mut added = 0;
    for piece in PARK_PIECES {
        if arenas.contains_key(&piece.id) {
            eprintln!("park editor: piece {:016x} shares a DMO bank asset ID; not drawn", piece.id);
            continue;
        }
        let (arena, resources) = match read_park_arena(&archive, piece.path) {
            Ok(read) => read,
            Err(e) => {
                eprintln!("park editor: piece {} not drawn: {e}", piece.recipe);
                continue;
            }
        };
        let mut owned = own_presentation_arena(&arena, &resources)?;
        for material in &mut owned.materials {
            let alpha = material.effect.as_ref().is_some_and(|effect| effect.serialized_name.contains("alphatest"));
            material.effect = Some(PresentationMaterialEffect::from_serialized_name(if alpha { "dynamicobject.alphatest" } else { "dynamicobject.default" }));
            let has = |channels: &[PresentationMaterialChannel], name: &str| channels.iter().skip(2).any(|channel| channel.shader_input == name);
            for (name, guid) in [("specular", PARK_DEFAULT_NORMAL), ("normal", PARK_DEFAULT_NORMAL), ("detail", PARK_DEFAULT_NORMAL), ("environment", environment)] {
                if !has(&material.channels, name) {
                    material.channels.push(raster(name, guid));
                }
            }
            if !has(&material.channels, "detailNormalUVScale") {
                material.channels.push(PresentationMaterialChannel {
                    shader_input: "detailNormalUVScale".to_owned(),
                    flags: 2,
                    image_channel: 0,
                    value: PresentationMaterialValue::ScalarConstant([1.0, 0.0, 0.0, 0.0]),
                });
            }
        }
        for mesh in &mut owned.meshes {
            let decal = mesh.material.and_then(|index| owned.materials.get(index)).and_then(park_decal).is_some();
            let geometry = &mut mesh.geometry;
            let three = |values: &NativeVertexAttributeValues| match values {
                NativeVertexAttributeValues::Float3(v) => Some(v.clone()),
                NativeVertexAttributeValues::Float4(v) => Some(v.iter().map(|v| [v[0], v[1], v[2]]).collect()),
                _ => None,
            };
            let binormal = VertexSemantic::Other { usage: 7, index: 0 };
            // A decal material packs its decal's UVs after the surface's: the surface keeps x, y.
            if let Some(attribute) = geometry.attributes.iter_mut().find(|attribute| attribute.semantic == VertexSemantic::TextureCoordinate(0))
                && let NativeVertexAttributeValues::Float4(values) = &attribute.values
            {
                let decal_uv = NativeVertexAttributeValues::Float2(values.iter().map(|v| [v[2], v[3]]).collect());
                attribute.values = NativeVertexAttributeValues::Float2(values.iter().map(|v| [v[0], v[1]]).collect());
                if decal {
                    geometry.attributes.retain(|attribute| attribute.semantic != VertexSemantic::TextureCoordinate(1));
                    geometry.attributes.push(NativeVertexAttribute { semantic: VertexSemantic::TextureCoordinate(1), original_format: 0, values: decal_uv });
                }
            }
            for semantic in [VertexSemantic::Tangent(0), binormal, VertexSemantic::Normal(0)] {
                if let Some(attribute) = geometry.attributes.iter_mut().find(|attribute| attribute.semantic == semantic)
                    && let Some(values) = three(&attribute.values)
                {
                    attribute.values = NativeVertexAttributeValues::Float3(values);
                }
            }
            if geometry.attributes.iter().any(|attribute| attribute.semantic == binormal) {
                continue;
            }
            let lanes = |semantic| match geometry.attributes.iter().find(|attribute| attribute.semantic == semantic).map(|attribute| &attribute.values) {
                Some(NativeVertexAttributeValues::Float3(v)) => Some(v.clone()),
                _ => None,
            };
            let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
            // Without a normal the surface faces up; without a tangent, any direction across it.
            let normals = lanes(VertexSemantic::Normal(0)).unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; geometry.positions.len()]);
            let tangents = lanes(VertexSemantic::Tangent(0)).unwrap_or_else(|| {
                let tangents: Vec<[f32; 3]> = normals
                    .iter()
                    .map(|&n| {
                        let t = cross(if n[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] }, n);
                        let length = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt().max(1e-6);
                        [t[0] / length, t[1] / length, t[2] / length]
                    })
                    .collect();
                geometry.attributes.push(NativeVertexAttribute { semantic: VertexSemantic::Tangent(0), original_format: 0, values: NativeVertexAttributeValues::Float3(tangents.clone()) });
                tangents
            });
            let values = normals.iter().zip(&tangents).map(|(&n, &t)| cross(n, t)).collect();
            geometry.attributes.push(NativeVertexAttribute { semantic: binormal, original_format: 0, values: NativeVertexAttributeValues::Float3(values) });
        }
        let wanted: BTreeSet<u64> = owned
            .materials
            .iter()
            .flat_map(|material| material.channels.iter().skip(2))
            .filter_map(|channel| match &channel.value {
                PresentationMaterialValue::Raster { guid, .. } if *guid != 0 => Some(*guid),
                _ => None,
            })
            .collect();
        for guid in wanted {
            if textures.contains_key(&guid) {
                continue;
            }
            let read = ["dynamic", "static"]
                .iter()
                .find_map(|kind| read_park_arena(&archive, &format!("data\\content\\{kind}\\texture\\0x{guid:016x}.rx2")).ok());
            let Some((texture_arena, texture_resources)) = read else {
                continue;
            };
            let owned_textures = own_presentation_arena(&texture_arena, &texture_resources)?;
            for (index, texture) in owned_textures.textures.iter().enumerate() {
                textures.insert(texture.guid, (guid, index));
            }
            arenas.insert(guid, owned_textures);
        }
        arenas.insert(piece.id, owned);
        added += 1;
    }
    Ok(added)
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
    let mut dmo_effect = DynamicObjectEffect::from_manifest(&binding.manifest_name)
        .ok_or_else(|| format!("DMO effect {:?} is not ported", binding.manifest_name))?;
    // A Skate 3 park piece's embedded decal (load_park_pieces gave it the default effect).
    let decal = park_decal(material).filter(|_| {
        dmo_effect == DynamicObjectEffect::Default && skate2_core::ui::object_dropper::park_piece(asset).is_some()
    });
    if let Some(guid) = decal {
        dmo_effect = DynamicObjectEffect::Decal;
        if !uploads.textures.contains_key(&guid) {
            let native = bank
                .texture(guid)
                .ok_or_else(|| format!("DMO decal texture {guid:016x} is not in the bank"))?;
            uploads.textures.insert(guid, renderer.upload_native_texture(native)?);
        }
    }
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
        decal.map(|guid| &textures[&guid]),
    )?;
    let result = MaterialBinding {
        binding: bind_group,
        effect: dmo_effect,
        draw_order,
    };
    uploads.materials.insert((asset, index), result.clone());
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::render::{NativeVertexAttributeValues, VertexSemantic};

    /// Every Skate 3 park piece in the bank as the DMO draw admits it: its RModel, each mesh's
    /// TEXCOORD0, tangent and binormal lanes, each material's ported effect, its five sampled
    /// textures and its detail scale.
    #[test]
    #[ignore = "requires SKATE2_GAME_ROOT with Skate 3's parkassets.big in data\\content"]
    fn park_pieces_admit_as_dmo_draws() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mounts = crate::mount_reference_archives(&root).unwrap();
        let bank = Bank::load(&mounts).unwrap();
        let mut bad = Vec::new();
        let mut meshes = 0;
        for piece in skate2_core::ui::object_dropper::PARK_PIECES {
            let Some(arena) = bank.arenas.get(&piece.id) else {
                bad.push(format!("{}: not in the bank", piece.recipe));
                continue;
            };
            let Some(model) = arena.models.iter().find(|model| model.dictionary_index == piece.model) else {
                bad.push(format!("{}: no RModel {}", piece.recipe, piece.model));
                continue;
            };
            for model_mesh in &model.meshes {
                meshes += 1;
                let mesh = &arena.meshes[model_mesh.mesh_index];
                let lanes = |semantic: VertexSemantic| mesh.geometry.attributes.iter().find(|a| a.semantic == semantic).map(|a| &a.values);
                if !matches!(lanes(VertexSemantic::TextureCoordinate(0)), Some(NativeVertexAttributeValues::Float2(_))) {
                    let kind = lanes(VertexSemantic::TextureCoordinate(0)).map(|values| match values {
                        NativeVertexAttributeValues::Float2(_) => "float2",
                        NativeVertexAttributeValues::Float3(_) => "float3",
                        NativeVertexAttributeValues::Float4(_) => "float4",
                        NativeVertexAttributeValues::UnsignedByte4(_) => "ubyte4",
                        NativeVertexAttributeValues::SignedShort2(_) => "short2",
                    });
                    bad.push(format!("{}: TEXCOORD0 {kind:?}, attributes {:?}", piece.recipe, mesh.geometry.attributes.iter().map(|a| a.semantic).collect::<Vec<_>>()));
                }
                for semantic in [VertexSemantic::Tangent(0), VertexSemantic::Other { usage: 7, index: 0 }] {
                    if !matches!(lanes(semantic), Some(NativeVertexAttributeValues::Float3(_))) {
                        bad.push(format!("{}: no float3 {semantic:?}", piece.recipe));
                    }
                }
                let material = &arena.materials[mesh.material.unwrap()];
                let key = material.effect.as_ref().and_then(|effect| effect.attribute_class_id.zip(effect.material_variant_id)).unwrap();
                match bank.bindings.get(&key) {
                    Some(binding) if DynamicObjectEffect::from_manifest(&binding.manifest_name).is_some() => {}
                    other => bad.push(format!("{}: effect binding {other:?}", piece.recipe)),
                }
                let channels = &material.channels[2..];
                for name in CHANNELS {
                    match channels.iter().rev().find(|channel| channel.shader_input == name).map(|channel| &channel.value) {
                        Some(PresentationMaterialValue::Raster { guid, .. }) if bank.texture(*guid).is_some() => {}
                        other => bad.push(format!("{}: {name} {other:?}", piece.recipe)),
                    }
                }
                if !channels.iter().any(|channel| channel.shader_input == "detailNormalUVScale") {
                    bad.push(format!("{}: no detailNormalUVScale", piece.recipe));
                }
            }
        }
        eprintln!("park pieces render RESULT {} pieces, {meshes} meshes, {} problems", skate2_core::ui::object_dropper::PARK_PIECES.len(), bad.len());
        let mut kinds = BTreeMap::<&str, usize>::new();
        for problem in &bad {
            *kinds.entry(problem.split_once(": ").map_or(problem.as_str(), |(_, kind)| kind)).or_default() += 1;
        }
        eprintln!("park pieces render RESULT problem kinds {kinds:?}");
        assert!(bad.is_empty(), "{}", bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
    }
}
