//! PC park editor: mission-only props. A mission hull fuses every prop it places into one mesh
//! (`<hull>_Fuse_Seed_0_Split_0` collision, `_Split_1` render), so a mission prop is the part of
//! that mesh inside the box its hull's Spatial record gives it. Placed copies are static: the
//! cut collision, moved to the placement, is an immovable Skate 3 body.

use crate::ui::object_dropper::{MissionProp, MissionPropShape, in_mission_cut, mission_prop_origin};
use skate2_formats::{
    mount::MountSet,
    rw4::Arena,
    stream::{CollectionMap, CollectionPayload, decode_arena},
};

/// Reads the prop's hull Simulation collections and cuts its collision out of their meshes.
///
/// # Errors
///
/// A hull stream that cannot be read or decoded, or a prop with no collision in its box.
#[cfg(feature = "skate3-runtime")]
pub(crate) fn load_mission_prop_shape(mounts: &MountSet, prop: &MissionProp) -> Result<MissionPropShape, String> {
    use crate::game_stream::{StreamKind, StreamPaths};
    let paths = StreamPaths::derive(crate::challenge::assets::CONTENT_STREAM_DIRECTORY, prop.hull, StreamKind::Simulation)
        .map_err(|e| format!("{} simulation stream path: {e:?}", prop.hull))?;
    let manifest = mounts.read_path(&format!("{}.xsm", paths.manifest_stem)).map_err(|e| format!("{} simulation manifest: {e}", prop.hull))?;
    let map = CollectionMap::parse(&manifest).map_err(|e| format!("{} simulation manifest: {e:?}", prop.hull))?;
    let origin = mission_prop_origin(prop);
    let mut shape = MissionPropShape::default();
    for entry in &map.collections {
        let xsf = mounts
            .read_path(&format!("{}{}.xsf", paths.collection_directory, entry.filename))
            .map_err(|e| format!("{} {}: {e}", prop.hull, entry.filename))?;
        let payload = xsf.get(entry.payload_offset as usize..).ok_or("mission prop collection payload offset")?;
        let file = CollectionPayload::parse(payload, entry.asset_count).map_err(|e| format!("{} assets: {e:?}", prop.hull))?;
        for block in &file.assets {
            let decoded = decode_arena(&payload[block.data_range.clone()]).map_err(|e| format!("{} arena: {e:?}", prop.hull))?;
            let arena = Arena::parse(&decoded.resources).map_err(|e| format!("{} arena: {e:?}", prop.hull))?;
            let collision = crate::simulation_collision_asset::project_collision_arena(&arena, &decoded.resources)
                .map_err(|e| format!("{} collision: {e:?}", prop.hull))?;
            for mesh in &collision.meshes {
                let (triangles, features) = crate::static_ground_pool::transformed_mesh_ground(mesh, &IDENTITY)
                    .map_err(|e| format!("{} collision mesh: {e:?}", prop.hull))?;
                for (mut triangle, feature) in triangles.into_iter().zip(features) {
                    if !in_mission_cut(prop, &triangle.vertices) {
                        continue;
                    }
                    triangle.vertices = triangle.vertices.map(|vertex| std::array::from_fn(|axis| vertex[axis] - origin[axis]));
                    shape.triangles.push(triangle);
                    shape.features.push(feature);
                }
            }
        }
    }
    if shape.triangles.is_empty() {
        return Err(format!("{} has no collision in its box", prop.name));
    }
    Ok(shape)
}

/// Without Skate 3 physics there is nothing for a mission prop's collision to join.
#[cfg(not(feature = "skate3-runtime"))]
pub(crate) fn load_mission_prop_shape(_: &MountSet, prop: &MissionProp) -> Result<MissionPropShape, String> {
    Err(format!("{} needs Skate 3 physics", prop.name))
}

const IDENTITY: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];

#[cfg(all(test, feature = "skate3-runtime"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn mission_speaker_collision_cuts_out() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
        let mut mounts = MountSet::new();
        for spec in &skate2_formats::mount::SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() {
                mounts.push(skate2_formats::mount::MountedBig::open(path, spec.use_external_header).unwrap());
            }
        }
        let mut bad = Vec::new();
        for prop in crate::ui::object_dropper::MISSION_PROPS {
            let shape = match load_mission_prop_shape(&mounts, prop) {
                Ok(shape) => shape,
                Err(e) => {
                    bad.push(format!("{}: {e}", prop.name));
                    continue;
                }
            };
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for vertex in shape.triangles.iter().flat_map(|triangle| triangle.vertices) {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(vertex[axis]);
                    hi[axis] = hi[axis].max(vertex[axis]);
                }
            }
            eprintln!("mission prop RESULT {}: {} triangles, model-space bounds {lo:?}..{hi:?}", prop.name, shape.triangles.len());
            let size: [f32; 3] = std::array::from_fn(|axis| prop.max[axis] - prop.min[axis]);
            for axis in 0..3 {
                let cut = hi[axis] - lo[axis];
                if cut > size[axis] + 0.2 || cut < size[axis] * 0.5 {
                    bad.push(format!("{}: axis {axis} cut {cut:.2} m, box {:.2} m", prop.name, size[axis]));
                }
            }
        }
        assert!(bad.is_empty(), "mission prop cuts off their boxes:
{}", bad.join("
"));
    }
}
