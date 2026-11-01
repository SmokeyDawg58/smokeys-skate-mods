//! PC park editor: placed mission props drawn as world geometry. A mission hull fuses its props
//! into one render mesh whose vertices are already in world space (the environment effects
//! draw them untransformed), so each prop is the part of that mesh `in_mission_cut` keeps, and
//! each placed copy is that part with its vertices moved to the placement on the CPU: one
//! presentation collection per copy, keyed by the copy, so a moved prop uploads afresh and a
//! resting one keeps its GPU mesh.
use skate2_core::{
    challenge::assets::CONTENT_STREAM_DIRECTORY,
    game_stream::{StreamKind, StreamPaths},
    presentation::{
        PresentationArena, PresentationCollection, PresentationInstance, PresentationMaterial, PresentationMaterialValue,
        PresentationMesh, PresentationModel, PresentationModelMesh, PresentationScene,
    },
    texture::NativeTexture,
    ui::object_dropper::{MISSION_PROPS, MissionProp, in_mission_cut, mission_prop_origin},
};
use skate2_formats::{
    mount::MountSet,
    render::{AxisAlignedBox, DrawIndexedParameters, NativeMeshGeometry, NativeVertexAttributeValues, PRIMITIVE_TYPE_TRIANGLE_LIST, VertexSemantic},
    stream::CollectionMap,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    sync::{Arc, Mutex},
};

/// One mission prop's render cut: model-space meshes (each indexing `materials`) and the
/// textures they sample.
struct Template {
    textures: Vec<NativeTexture>,
    materials: Vec<PresentationMaterial>,
    meshes: Vec<PresentationMesh>,
}

/// Each mission prop's cut, read from its hull the first time a copy is drawn.
#[derive(Clone, Default)]
pub(crate) struct MissionPropTemplates(Arc<Mutex<BTreeMap<usize, Arc<Template>>>>);

impl MissionPropTemplates {
    fn template(&self, mounts: &MountSet, kind: usize) -> Result<Arc<Template>, Box<dyn Error>> {
        if let Some(template) = self.0.lock().map_err(|_| "mission prop templates lock poisoned")?.get(&kind) {
            return Ok(Arc::clone(template));
        }
        let prop = MISSION_PROPS.get(kind).ok_or("unknown mission prop")?;
        let template = Arc::new(cut(mounts, prop)?);
        self.0.lock().map_err(|_| "mission prop templates lock poisoned")?.insert(kind, Arc::clone(&template));
        Ok(template)
    }
}

/// The scene's placed collections become one per placed copy. Returns whether any changed.
pub(crate) fn sync(
    scene: &mut PresentationScene,
    mounts: &MountSet,
    templates: &MissionPropTemplates,
    placed: &[(usize, [f32; 16])],
) -> Result<bool, Box<dyn Error>> {
    let mut held = std::mem::take(&mut scene.placed);
    let mut collections = Vec::with_capacity(placed.len());
    let mut changed = false;
    for (at, &(kind, transform)) in placed.iter().enumerate() {
        // Copies at the same spot count up; any other copy coming or going keeps this ID.
        let ordinal = placed[..at].iter().filter(|other| **other == (kind, transform)).count();
        let id = collection_id(kind, &transform, ordinal);
        if let Some(index) = held.iter().position(|collection| collection.id == id) {
            collections.push(held.swap_remove(index));
            continue;
        }
        // A prop whose cut cannot be read stays undrawn rather than stopping the city.
        let template = match templates.template(mounts, kind) {
            Ok(template) => template,
            Err(e) => {
                eprintln!("park editor: mission prop {kind} not drawn: {e}");
                continue;
            }
        };
        collections.push(placed_copy(&template, id, kind, &transform));
        changed = true;
    }
    changed |= !held.is_empty();
    scene.placed = collections;
    Ok(changed)
}

/// A copy's collection ID: its kind, placement and how many copies before it share both, hashed (FNV-1a) into
/// a range no stream collection uses.
fn collection_id(kind: usize, transform: &[f32; 16], ordinal: usize) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut mix = |word: u64| {
        for byte in word.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    mix(kind as u64);
    mix(ordinal as u64);
    for value in transform {
        mix(u64::from(value.to_bits()));
    }
    0x4d50_0000_0000_0000 | (hash & 0x0000_ffff_ffff_ffff)
}

/// The prop's part of its hull's fused render mesh, moved so the prop's base centre is the
/// origin, with the materials and textures it uses.
fn cut(mounts: &MountSet, prop: &MissionProp) -> Result<Template, Box<dyn Error>> {
    let paths = StreamPaths::derive(CONTENT_STREAM_DIRECTORY, prop.hull, StreamKind::Presentation)
        .map_err(|e| format!("{} presentation stream path: {e:?}", prop.hull))?;
    let map = CollectionMap::parse(&mounts.read_path(&format!("{}.xsm", paths.manifest_stem))?)?;
    let origin = mission_prop_origin(prop);
    let mut materials = Vec::new();
    let mut meshes = Vec::new();
    let mut wanted = BTreeSet::new();
    let mut textures_by_guid = BTreeMap::new();
    for entry in &map.collections {
        let collection = crate::hull_presentation::load_collection(mounts, &paths, &map, entry.id)?;
        for arena in &collection.arenas {
            for texture in &arena.textures {
                textures_by_guid.entry(texture.guid).or_insert_with(|| texture.clone());
            }
            let mut remap = BTreeMap::new();
            for instance in &arena.instances {
                let Some(model) = instance.render_model.and_then(|index| arena.models.get(index)) else { continue };
                for model_mesh in &model.meshes {
                    let Some(mesh) = arena.meshes.get(model_mesh.mesh_index) else { continue };
                    let Some(geometry) = cut_geometry(&mesh.geometry, prop, origin) else { continue };
                    let Some(source_material) = mesh.material.and_then(|index| arena.materials.get(index).map(|material| (index, material))) else { continue };
                    let material = *remap.entry(source_material.0).or_insert_with(|| {
                        let mut material = source_material.1.clone();
                        for channel in &mut material.channels {
                            if let PresentationMaterialValue::Raster { guid, resident_texture, .. } = &mut channel.value {
                                wanted.insert(*guid);
                                *resident_texture = None;
                            }
                        }
                        materials.push(material);
                        materials.len() - 1
                    });
                    meshes.push(PresentationMesh {
                        dictionary_index: mesh.dictionary_index,
                        bounding_box: bounds(&geometry.positions),
                        material: Some(material),
                        skin_palette: Vec::new(),
                        geometry,
                    });
                }
            }
        }
    }
    if meshes.is_empty() {
        return Err(format!("{} has no render geometry in its box", prop.name).into());
    }
    // A texture the hull does not carry is one the city's streamed textures bind later.
    let textures = wanted.iter().filter_map(|guid| textures_by_guid.remove(guid)).collect();
    Ok(Template { textures, materials, meshes })
}

/// The triangles of a triangle-list mesh whose centroids the cut keeps, their vertices moved by
/// `-origin`, as one draw.
fn cut_geometry(geometry: &NativeMeshGeometry, prop: &MissionProp, origin: [f32; 3]) -> Option<NativeMeshGeometry> {
    let mut remap: BTreeMap<usize, u32> = BTreeMap::new();
    let mut order = Vec::new();
    let mut indices = Vec::new();
    for draw in &geometry.draws {
        if draw.primitive_type != PRIMITIVE_TYPE_TRIANGLE_LIST {
            continue;
        }
        let start = draw.start_index as usize;
        let end = (start + draw.index_count as usize).min(geometry.indices.len());
        for triangle in (start..end).collect::<Vec<_>>().chunks_exact(3) {
            let corners: Option<Vec<usize>> = triangle
                .iter()
                .map(|&at| usize::try_from(i64::from(geometry.indices[at]) + i64::from(draw.base_vertex_index)).ok())
                .collect();
            let Some(corners) = corners.filter(|corners| corners.iter().all(|&vertex| vertex < geometry.positions.len())) else { continue };
            let vertices = [geometry.positions[corners[0]], geometry.positions[corners[1]], geometry.positions[corners[2]]];
            if !in_mission_cut(prop, &vertices) {
                continue;
            }
            for vertex in corners {
                let next = u32::try_from(order.len()).ok()?;
                let index = *remap.entry(vertex).or_insert_with(|| {
                    order.push(vertex);
                    next
                });
                indices.push(index);
            }
        }
    }
    if indices.is_empty() {
        return None;
    }
    let attributes = geometry
        .attributes
        .iter()
        .map(|attribute| {
            let mut attribute = attribute.clone();
            attribute.values = pick(&attribute.values, &order);
            attribute
        })
        .collect();
    Some(NativeMeshGeometry {
        positions: order.iter().map(|&vertex| std::array::from_fn(|axis| geometry.positions[vertex][axis] - origin[axis])).collect(),
        attributes,
        draws: vec![DrawIndexedParameters { primitive_type: PRIMITIVE_TYPE_TRIANGLE_LIST, base_vertex_index: 0, start_index: 0, index_count: u32::try_from(indices.len()).ok()? }],
        indices,
        position_format: geometry.position_format,
    })
}

fn pick(values: &NativeVertexAttributeValues, order: &[usize]) -> NativeVertexAttributeValues {
    fn take<T: Copy + Default>(values: &[T], order: &[usize]) -> Vec<T> {
        order.iter().map(|&vertex| values.get(vertex).copied().unwrap_or_default()).collect()
    }
    match values {
        NativeVertexAttributeValues::Float2(v) => NativeVertexAttributeValues::Float2(take(v, order)),
        NativeVertexAttributeValues::Float3(v) => NativeVertexAttributeValues::Float3(take(v, order)),
        NativeVertexAttributeValues::Float4(v) => NativeVertexAttributeValues::Float4(take(v, order)),
        NativeVertexAttributeValues::UnsignedByte4(v) => NativeVertexAttributeValues::UnsignedByte4(take(v, order)),
        NativeVertexAttributeValues::SignedShort2(v) => NativeVertexAttributeValues::SignedShort2(take(v, order)),
    }
}

/// The template's meshes moved to `transform` (row major: axis rows, translation row 3):
/// positions, and the normal and tangent directions the environment effects read.
fn placed_copy(template: &Template, id: u64, kind: usize, transform: &[f32; 16]) -> PresentationCollection {
    let point = |p: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|row| transform[row] * p[0] + transform[4 + row] * p[1] + transform[8 + row] * p[2] + transform[12 + row])
    };
    let direction = |d: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|row| transform[row] * d[0] + transform[4 + row] * d[1] + transform[8 + row] * d[2])
    };
    let meshes: Vec<PresentationMesh> = template
        .meshes
        .iter()
        .map(|mesh| {
            let mut mesh = mesh.clone();
            for position in &mut mesh.geometry.positions {
                *position = point(*position);
            }
            for attribute in &mut mesh.geometry.attributes {
                if !matches!(attribute.semantic, VertexSemantic::Normal(_) | VertexSemantic::Tangent(_)) {
                    continue;
                }
                match &mut attribute.values {
                    NativeVertexAttributeValues::Float3(values) => values.iter_mut().for_each(|v| *v = direction(*v)),
                    NativeVertexAttributeValues::Float4(values) => values.iter_mut().for_each(|v| {
                        let [x, y, z] = direction([v[0], v[1], v[2]]);
                        *v = [x, y, z, v[3]];
                    }),
                    _ => {}
                }
            }
            mesh.bounding_box = bounds(&mesh.geometry.positions);
            mesh
        })
        .collect();
    let all: Vec<[f32; 3]> = meshes.iter().flat_map(|mesh| mesh.geometry.positions.iter().copied()).collect();
    let bounding_box = bounds(&all);
    // No model-mesh materials: an old-format model, whose world shadow takes the whole entity
    // rather than per-island ranges a cut has none of.
    let model = PresentationModel {
        dictionary_index: 0,
        bounding_box,
        island_bounding_boxes: Vec::new(),
        island_areas: Vec::new(),
        total_bone_count: 0,
        named_bone_count: 0,
        inverse_bind_pose: Vec::new(),
        bone_names: Vec::new(),
        meshes: (0..meshes.len()).map(|mesh_index| PresentationModelMesh { mesh_index, material: None }).collect(),
    };
    let name = format!("park_editor_{}", MISSION_PROPS[kind].name);
    let arena = PresentationArena {
        id: 0,
        textures: template.textures.clone(),
        materials: template.materials.clone(),
        meshes,
        models: vec![model],
        instances: vec![PresentationInstance {
            guid: id,
            local_guid: id,
            name: name.clone(),
            transform: [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.],
            bounding_box_min: [bounding_box.minimum[0], bounding_box.minimum[1], bounding_box.minimum[2]],
            bounding_box_max: [bounding_box.maximum[0], bounding_box.maximum[1], bounding_box.maximum[2]],
            render_model: Some(0),
        }],
        anti_frustums: Vec::new(),
        emitter_waypoints: Vec::new(),
    };
    PresentationCollection {
        light_probes: Vec::new(),
        navigation: Vec::new(),
        id,
        name,
        position: [transform[12], transform[13], transform[14]],
        distance_xz: 0.0,
        arenas: vec![arena],
    }
}

fn bounds(points: &[[f32; 3]]) -> AxisAlignedBox {
    let mut minimum = [f32::MAX, f32::MAX, f32::MAX, 0.0];
    let mut maximum = [f32::MIN, f32::MIN, f32::MIN, 0.0];
    for point in points {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(point[axis]);
            maximum[axis] = maximum[axis].max(point[axis]);
        }
    }
    AxisAlignedBox { minimum, maximum }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every mission prop's render cut: speaker-sized, textured, and a placed copy lands where
    /// it is put.
    #[test]
    #[ignore = "requires external SKATE2_GAME_ROOT"]
    fn mission_props_cut_from_their_hulls() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
        let mounts = crate::mount_reference_archives(&root).unwrap();
        let mut bad = Vec::new();
        for (kind, prop) in MISSION_PROPS.iter().enumerate() {
            let template = match cut(&mounts, prop) {
                Ok(template) => template,
                Err(e) => {
                    bad.push(format!("{}: {e}", prop.name));
                    continue;
                }
            };
            let triangles: usize = template.meshes.iter().map(|mesh| mesh.geometry.indices.len() / 3).sum();
            let all: Vec<[f32; 3]> = template.meshes.iter().flat_map(|mesh| mesh.geometry.positions.iter().copied()).collect();
            let size: [f32; 3] = {
                let b = bounds(&all);
                std::array::from_fn(|axis| b.maximum[axis] - b.minimum[axis])
            };
            eprintln!("mission prop render RESULT {}: {} meshes, {triangles} triangles, {} materials, {} textures, size {size:?}",
                prop.name, template.meshes.len(), template.materials.len(), template.textures.len());
            let wanted: [f32; 3] = std::array::from_fn(|axis| prop.max[axis] - prop.min[axis]);
            if (0..3).any(|axis| size[axis] > wanted[axis] + 0.2 || size[axis] < wanted[axis] * 0.5) {
                bad.push(format!("{}: render cut size {size:?} vs box {wanted:?}", prop.name));
            }
            let mut at = [0.0f32; 16];
            for axis in 0..4 {
                at[axis * 5] = 1.0;
            }
            at[12..15].copy_from_slice(&[10.0, 2.0, -5.0]);
            let copy = placed_copy(&template, collection_id(kind, &at, 0), kind, &at);
            let instance = &copy.arenas[0].instances[0];
            if (instance.bounding_box_min[1] - 2.0).abs() > 0.05 {
                bad.push(format!("{}: placed copy's base at {:?}", prop.name, instance.bounding_box_min));
            }
        }
        assert!(bad.is_empty(), "mission prop render cuts:
{}", bad.join("
"));
    }
}
