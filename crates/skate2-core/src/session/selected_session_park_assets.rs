//! PC park editor: Skate 3's park editor pieces (`parkassets.big`), each a standalone `.rx2`
//! arena holding its render model, its collision and its grind splines. Placed copies are
//! static, as mission props are: the piece's collision mesh, moved to the placement, is an
//! immovable Skate 3 body and Skate 2 ground.

use crate::ui::object_dropper::{MissionPropShape, ParkPiece};
use skate2_formats::mount::MountSet;

/// Reads the piece's collision meshes (model space) from Skate 3's park editor archive.
///
/// # Errors
///
/// The archive is absent, or the piece's arena cannot be read or holds no collision.
#[cfg(feature = "skate3-runtime")]
pub(crate) fn load_park_piece_shape(mounts: &MountSet, piece: &ParkPiece) -> Result<MissionPropShape, String> {
    let archive = crate::ui::object_dropper::park_assets_archive(mounts).ok_or("parkassets.big is not beside the game's archives")?;
    let (arena, resources) = crate::ui::object_dropper::read_park_arena(&archive, piece.path)?;
    let collision = crate::simulation_collision_asset::project_collision_arena(&arena, &resources)
        .map_err(|e| format!("{} collision: {e:?}", piece.recipe))?;
    let mut shape = MissionPropShape::default();
    for mesh in &collision.meshes {
        let (triangles, features) = crate::static_ground_pool::transformed_mesh_ground(mesh, &IDENTITY)
            .map_err(|e| format!("{} collision mesh: {e:?}", piece.recipe))?;
        shape.triangles.extend(triangles);
        shape.features.extend(features);
    }
    if shape.triangles.is_empty() {
        return Err(format!("{} has no collision", piece.recipe));
    }
    // Its grind splines (tSplineData, the format Skate 2's are in), owned by its collision as a
    // character recipe's are.
    let owner = std::sync::Arc::new(collision);
    for (index, entry) in arena.dictionary.iter().enumerate().filter(|(_, entry)| entry.type_id == skate2_formats::pegasus::SPLINE_DATA_TYPE_ID) {
        let bytes = arena.object_bytes(&resources, entry).ok_or_else(|| format!("{} spline outside its arena", piece.recipe))?;
        let index = u32::try_from(index).map_err(|e| e.to_string())?;
        let source = crate::grind_data_runtime::GrindSplineSource::try_from_character_spline_data(std::sync::Arc::clone(&owner), index, piece.id, bytes)
            .map_err(|e| format!("{} grind spline: {e:?}", piece.recipe))?;
        for segment in &source.data().segments {
            let (low, high) = shape.grind_bounds.get_or_insert(([f32::MAX; 3], [f32::MIN; 3]));
            for axis in 0..3 {
                low[axis] = low[axis].min(segment.bounding_box_min[axis]);
                high[axis] = high[axis].max(segment.bounding_box_max[axis]);
            }
        }
        for spline in 0..source.data().splines.len() {
            let header_offset = source.data().spline_object_offset(spline).ok_or_else(|| format!("{} spline header out of range", piece.recipe))?;
            shape.grind_splines.push(crate::living_world_grind_services::dmo_models::DmoGrindSpline { source: source.clone(), header_offset });
        }
    }
    Ok(shape)
}

/// Without Skate 3 physics there is nothing for a piece's collision to join.
#[cfg(not(feature = "skate3-runtime"))]
pub(crate) fn load_park_piece_shape(_: &MountSet, piece: &ParkPiece) -> Result<MissionPropShape, String> {
    Err(format!("{} needs Skate 3 physics", piece.recipe))
}

#[cfg(feature = "skate3-runtime")]
const IDENTITY: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

/// The arena object listing a piece's snap points (Skate 3's park editor snaps by them).
const SNAP_POINTS_TYPE_ID: u32 = 0x00eb_001e;
/// Its header, then one record per point: a 4x4 matrix (axis rows, then the position), the
/// point's bounds (two vec4s), a word and the point's face index.
const SNAP_HEADER: usize = 16;
const SNAP_RECORD: usize = 0x70;
const SNAP_FACE_AT: usize = 0x64;

/// One face a piece snaps by (model space): the way it faces (its points' z axis) and its
/// corners. Face 0 holds loose points (a rail's), not a face.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SnapFace {
    pub(crate) normal: [f32; 3],
    pub(crate) corners: Vec<[f32; 3]>,
}

/// The piece's snap faces, read from its arena's snap points, grouped by face index.
///
/// # Errors
///
/// The archive is absent, or the piece's arena cannot be read.
pub(crate) fn load_park_piece_snap_faces(mounts: &MountSet, piece: &ParkPiece) -> Result<Vec<SnapFace>, String> {
    let archive = crate::ui::object_dropper::park_assets_archive(mounts).ok_or("parkassets.big is not beside the game's archives")?;
    let (arena, resources) = crate::ui::object_dropper::read_park_arena(&archive, piece.path)?;
    let mut faces: std::collections::BTreeMap<u8, SnapFace> = std::collections::BTreeMap::new();
    for entry in arena.dictionary.iter().filter(|entry| entry.type_id == SNAP_POINTS_TYPE_ID) {
        let bytes = arena.object_bytes(&resources, entry).ok_or("snap points outside the arena")?;
        let word = |at: usize| bytes.get(at..at + 4).map(|w| u32::from_be_bytes([w[0], w[1], w[2], w[3]]));
        let float = |at: usize| word(at).map(f32::from_bits);
        let count = word(0).ok_or("snap points truncated")? as usize;
        for point in 0..count {
            let at = SNAP_HEADER + point * SNAP_RECORD;
            let face = *bytes.get(at + SNAP_FACE_AT).ok_or("snap point truncated")?;
            if face == 0 {
                continue;
            }
            let lanes = |row: usize| -> Option<[f32; 3]> { Some([float(at + row)?, float(at + row + 4)?, float(at + row + 8)?]) };
            let (normal, corner) = (lanes(0x20).ok_or("snap point truncated")?, lanes(0x30).ok_or("snap point truncated")?);
            faces.entry(face).or_insert_with(|| SnapFace { normal, corners: Vec::new() }).corners.push(corner);
        }
    }
    Ok(faces.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::MountSet;
    use skate2_formats::rw4::Arena;

    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("rx2")) {
                out.push(path);
            }
        }
    }

    /// Every park piece through the port's own collision and presentation projections.
    #[test]
    #[ignore = "requires the extracted parkassets.big via SKATE3_PARKASSETS"]
    fn park_assets_project() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_PARKASSETS").expect("SKATE3_PARKASSETS"));
        let mut files = Vec::new();
        walk(&root.join("data/content"), &mut files);
        files.sort();
        let mut by_folder: std::collections::BTreeMap<String, [usize; 9]> = std::collections::BTreeMap::new();
        let mut failures = Vec::new();
        for file in &files {
            let folder = file.parent().and_then(|p| p.file_name()).map_or(String::new(), |n| n.to_string_lossy().into_owned());
            if folder == "texture" {
                continue;
            }
            let slot = by_folder.entry(folder.clone()).or_default();
            slot[0] += 1;
            let bytes = std::fs::read(file).unwrap();
            let resources = Arena::split_file_resources(&bytes).unwrap();
            let arena = Arena::parse(&resources).unwrap();
            match crate::simulation_collision_asset::project_collision_arena(&arena, &resources) {
                Ok(collision) => {
                    slot[1] += usize::from(!collision.meshes.is_empty() || !collision.primitive_sets.is_empty() || !collision.simple_primitive_sets.is_empty());
                    slot[2] += collision.meshes.len();
                    slot[3] += collision.primitive_sets.len();
                    slot[4] += collision.simple_primitive_sets.len();
                    slot[5] += collision.grind_assets.len();
                    for mesh in &collision.meshes {
                        if let Ok((triangles, _)) = crate::static_ground_pool::transformed_mesh_ground(mesh, &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]) {
                            slot[6] += triangles.len();
                        }
                    }
                }
                Err(e) => failures.push(format!("{folder}/{}: collision {e:?}", file.file_name().unwrap().to_string_lossy())),
            }
            match crate::mounted_character_scene::own_presentation_arena(&arena, &resources) {
                Ok(presentation) => {
                    slot[7] += presentation.meshes.len();
                    slot[8] += usize::from(!presentation.instances.is_empty());
                }
                Err(e) => failures.push(format!("{folder}/{}: presentation {e}", file.file_name().unwrap().to_string_lossy())),
            }
        }
        for (folder, n) in &by_folder {
            eprintln!(
                "park assets RESULT {folder}: {} files, {} with collision ({} meshes, {} primitive sets, {} simple sets, {} triangles), {} grind assets, {} render meshes, {} with instances",
                n[0], n[1], n[2], n[3], n[4], n[6], n[5], n[7], n[8]
            );
        }
        for failure in failures.iter().take(20) {
            eprintln!("park assets FAIL {failure}");
        }
        eprintln!("park assets RESULT {} failures", failures.len());
    }

    /// One line per park piece for the catalog generator: folder, file, collision triangles and
    /// their bounds, render bounds, and the first instance's translation.
    #[test]
    #[ignore = "requires the extracted parkassets.big via SKATE3_PARKASSETS"]
    fn park_assets_catalog_dump() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_PARKASSETS").expect("SKATE3_PARKASSETS"));
        let out = std::path::PathBuf::from(std::env::var_os("PARK_CATALOG_OUT").expect("PARK_CATALOG_OUT"));
        let mut files = Vec::new();
        walk(&root.join("data/content"), &mut files);
        files.sort();
        let mut lines = Vec::new();
        let identity = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
        for file in &files {
            let folder = file.parent().and_then(|p| p.file_name()).map_or(String::new(), |n| n.to_string_lossy().into_owned());
            if folder == "texture" {
                continue;
            }
            let bytes = std::fs::read(file).unwrap();
            let resources = Arena::split_file_resources(&bytes).unwrap();
            let arena = Arena::parse(&resources).unwrap();
            let collision = crate::simulation_collision_asset::project_collision_arena(&arena, &resources).unwrap();
            let (mut lo, mut hi, mut count) = ([f32::MAX; 3], [f32::MIN; 3], 0usize);
            for mesh in &collision.meshes {
                let (triangles, _) = crate::static_ground_pool::transformed_mesh_ground(mesh, &identity).unwrap();
                for vertex in triangles.iter().flat_map(|triangle| triangle.vertices) {
                    for axis in 0..3 {
                        lo[axis] = lo[axis].min(vertex[axis]);
                        hi[axis] = hi[axis].max(vertex[axis]);
                    }
                }
                count += triangles.len();
            }
            let presentation = crate::mounted_character_scene::own_presentation_arena(&arena, &resources).unwrap();
            let (mut rlo, mut rhi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for mesh in &presentation.meshes {
                for p in &mesh.geometry.positions {
                    for axis in 0..3 {
                        rlo[axis] = rlo[axis].min(p[axis]);
                        rhi[axis] = rhi[axis].max(p[axis]);
                    }
                }
            }
            let at = presentation.instances.first().map_or([0.0; 3], |instance| [instance.transform[12], instance.transform[13], instance.transform[14]]);
            let textures: std::collections::BTreeSet<u64> = presentation
                .materials
                .iter()
                .flat_map(|material| material.channels.iter())
                .filter(|channel| channel.shader_input != "Name" && channel.shader_input != "AttribulatorMaterialName")
                .filter_map(|channel| match &channel.value {
                    crate::presentation::PresentationMaterialValue::Raster { guid, .. } => Some(*guid),
                    _ => None,
                })
                .collect();
            let effects: std::collections::BTreeSet<&str> =
                presentation.materials.iter().filter_map(|material| material.effect.as_ref().map(|effect| effect.serialized_name.as_str())).collect();
            lines.push(format!(
                "{folder}\t{}\t{count}\t{lo:?}\t{hi:?}\t{rlo:?}\t{rhi:?}\t{at:?}\t{}\t{}\t{}\t{}",
                file.file_stem().unwrap().to_string_lossy(),
                presentation.meshes.len(),
                textures.iter().map(|guid| format!("{guid:016x}")).collect::<Vec<_>>().join(","),
                effects.into_iter().collect::<Vec<_>>().join(","),
                presentation.models.iter().map(|model| format!("{}:{}", model.dictionary_index, model.meshes.len())).collect::<Vec<_>>().join(",")
            ));
        }
        std::fs::write(out, lines.join("\n")).unwrap();
    }

    /// One park piece's materials, channel by channel (PARK_DUMP_FILE: path under the root).
    #[test]
    #[ignore = "requires the extracted parkassets.big via SKATE3_PARKASSETS"]
    fn park_assets_material_dump() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_PARKASSETS").expect("SKATE3_PARKASSETS"));
        let file = root.join(std::env::var("PARK_DUMP_FILE").expect("PARK_DUMP_FILE"));
        let bytes = std::fs::read(file).unwrap();
        let resources = Arena::split_file_resources(&bytes).unwrap();
        let arena = Arena::parse(&resources).unwrap();
        let presentation = crate::mounted_character_scene::own_presentation_arena(&arena, &resources).unwrap();
        for (index, mesh) in presentation.meshes.iter().enumerate() {
            eprintln!("mesh {index}: material {:?}, {} vertices, attributes {:?}", mesh.material, mesh.geometry.positions.len(),
                mesh.geometry.attributes.iter().map(|a| format!("{:?}", a.semantic)).collect::<Vec<_>>());
        }
        for (index, model) in presentation.models.iter().enumerate() {
            eprintln!("model {index}: meshes {:?}", model.meshes.iter().map(|m| (m.mesh_index, m.material)).collect::<Vec<_>>());
        }
        for (index, material) in presentation.materials.iter().enumerate() {
            eprintln!("material {index}: flags {:#x} effect {:?}", material.flags, material.effect);
            for channel in &material.channels {
                eprintln!("    {channel:?}");
            }
        }
    }

    /// Every catalog piece's collision through the game's own mounts (parkassets.big copied
    /// into the ROM's data\content).
    #[cfg(feature = "skate3-runtime")]
    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT, with parkassets.big"]
    fn park_piece_shapes_load() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
        let mut mounts = MountSet::new();
        for spec in &skate2_formats::mount::SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() {
                mounts.push(skate2_formats::mount::MountedBig::open(path, spec.use_external_header).unwrap());
            }
        }
        let mut bad = Vec::new();
        let mut triangles = 0;
        for piece in crate::ui::object_dropper::PARK_PIECES {
            match super::load_park_piece_shape(&mounts, piece) {
                Ok(shape) => triangles += shape.triangles.len(),
                Err(e) => bad.push(e),
            }
        }
        eprintln!("park pieces RESULT {} shapes, {triangles} triangles, {} failed", crate::ui::object_dropper::PARK_PIECES.len() - bad.len(), bad.len());
        assert!(bad.is_empty(), "{}", bad.iter().take(10).cloned().collect::<Vec<_>>().join("
"));
    }

    /// Every catalog piece's snap faces read from the game's own mounts: Terrain and Other
    /// pieces have faces (rails aside: loose points only).
    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT, with parkassets.big"]
    fn park_piece_snap_faces_load() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
        let mut mounts = MountSet::new();
        for spec in &skate2_formats::mount::SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() {
                mounts.push(skate2_formats::mount::MountedBig::open(path, spec.use_external_header).unwrap());
            }
        }
        let mut faceless = std::collections::BTreeMap::<&str, usize>::new();
        let mut faces = 0;
        for piece in crate::ui::object_dropper::PARK_PIECES.iter().filter(|piece| piece.tab < 2) {
            let found = super::load_park_piece_snap_faces(&mounts, piece).unwrap();
            if found.is_empty() {
                *faceless.entry(piece.path.split('\\').nth(5).unwrap_or("?")).or_default() += 1;
            }
            faces += found.len();
        }
        eprintln!("park pieces snap RESULT {faces} faces; pieces without faces by folder {faceless:?}");
        let street = crate::ui::object_dropper::PARK_PIECES.iter().find(|piece| piece.recipe == "aa_padStreet_lrg_Downtown").unwrap();
        let pad = super::load_park_piece_snap_faces(&mounts, street).unwrap();
        assert_eq!(pad.len(), 6, "a street pad is a box: {pad:?}");
        assert!(pad.iter().all(|face| face.corners.len() == 4));
    }
}
