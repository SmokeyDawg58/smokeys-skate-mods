use super::{
    CharacterGhostState, CharacterGpuError, CharacterPoseRows, CharacterShaderFamily, ROW_BYTES,
    row_bytes, vertex_bytes,
};
use skate2_formats::render::{
    NativeMeshGeometry, NativeVertexAttribute, NativeVertexAttributeValues, VERTEX_FORMAT_FLOAT2,
    VERTEX_FORMAT_FLOAT16_2, VERTEX_FORMAT_SHORT4N, VERTEX_FORMAT_UBYTE4, VertexSemantic,
};

fn geometry(indices: [u8; 4]) -> NativeMeshGeometry {
    NativeMeshGeometry {
        positions: vec![[0.0, -0.4, 0.5]],
        position_format: VERTEX_FORMAT_SHORT4N,
        attributes: vec![
            NativeVertexAttribute {
                semantic: VertexSemantic::BlendWeight(0),
                original_format: VERTEX_FORMAT_UBYTE4,
                values: NativeVertexAttributeValues::UnsignedByte4(vec![[255, 255, 0, 199]]),
            },
            NativeVertexAttribute {
                semantic: VertexSemantic::BlendIndices(0),
                original_format: VERTEX_FORMAT_UBYTE4,
                values: NativeVertexAttributeValues::UnsignedByte4(vec![indices]),
            },
            NativeVertexAttribute {
                semantic: VertexSemantic::TextureCoordinate(0),
                original_format: VERTEX_FORMAT_FLOAT16_2,
                values: NativeVertexAttributeValues::Float2(vec![[0.25, 0.75]]),
            },
        ],
        indices: vec![],
        draws: vec![],
    }
}

#[test]
fn upload_keeps_original_position_and_raw_weights_for_shader_expansion() {
    let bytes = vertex_bytes(&geometry([0, 1, 2, 3]), 4, CharacterShaderFamily::Character).unwrap();
    assert_eq!(bytes.len(), 28);
    assert_eq!(&bytes[4..8], &(-0.4_f32).to_ne_bytes());
    assert_eq!(&bytes[20..24], &[255, 255, 0, 199]);
    assert_eq!(&bytes[24..28], &[0, 1, 2, 3]);
}

#[test]
fn hair_vertices_append_the_alpha_uv_or_its_native_zero_xy() {
    let mut hair = geometry([0, 1, 2, 3]);
    let absent = vertex_bytes(&hair, 4, CharacterShaderFamily::Hair).unwrap();
    assert_eq!(absent.len(), 36);
    assert_eq!(&absent[24..28], &[0, 1, 2, 3]);
    assert_eq!(&absent[28..36], &[0; 8]);
    hair.attributes.push(NativeVertexAttribute {
        semantic: VertexSemantic::TextureCoordinate(1),
        original_format: VERTEX_FORMAT_FLOAT2,
        values: NativeVertexAttributeValues::Float2(vec![[0.125, -2.0]]),
    });
    let bytes = vertex_bytes(&hair, 4, CharacterShaderFamily::Hair).unwrap();
    assert_eq!(bytes.len(), 36);
    assert_eq!(&bytes[24..28], &[0, 1, 2, 3]);
    assert_eq!(&bytes[28..32], &0.125_f32.to_ne_bytes());
    assert_eq!(&bytes[32..36], &(-2.0_f32).to_ne_bytes());
    let body = vertex_bytes(&hair, 4, CharacterShaderFamily::Character).unwrap();
    assert_eq!(body.len(), 28);
    hair.attributes.push(hair.attributes.last().unwrap().clone());
    assert!(matches!(
        vertex_bytes(&hair, 4, CharacterShaderFamily::Hair),
        Err(CharacterGpuError::MissingOrDuplicateAttribute(
            VertexSemantic::TextureCoordinate(1)
        ))
    ));
}

#[test]
fn cas_missing_decal_uv_uses_native_zero_xy_and_preserves_authored_uv() {
    let mut mesh = geometry([0, 1, 2, 3]);
    for semantic in [VertexSemantic::Tangent(0), VertexSemantic::Other { usage: 7, index: 0 }] {
        mesh.attributes.push(NativeVertexAttribute {
            semantic, original_format: skate2_formats::render::VERTEX_FORMAT_FLOAT3,
            values: NativeVertexAttributeValues::Float3(vec![[1.0, 0.0, 0.0]]),
        });
    }
    let bytes = super::cas::cas_vertex_bytes(&mesh, 4, CharacterShaderFamily::Character).unwrap();
    assert_eq!(bytes.len(), 60);
    assert_eq!(&bytes[52..60], &[0; 8]);
    mesh.attributes.push(NativeVertexAttribute {
        semantic: VertexSemantic::TextureCoordinate(1), original_format: VERTEX_FORMAT_FLOAT2,
        values: NativeVertexAttributeValues::Float2(vec![[0.125, 0.875]]),
    });
    let bytes = super::cas::cas_vertex_bytes(&mesh, 4, CharacterShaderFamily::Character).unwrap();
    assert_eq!(&bytes[52..56], &0.125_f32.to_ne_bytes());
    assert_eq!(&bytes[56..60], &0.875_f32.to_ne_bytes());
    mesh.attributes.push(mesh.attributes.last().unwrap().clone());
    assert!(super::cas::cas_vertex_bytes(&mesh, 4, CharacterShaderFamily::Character).is_err());
}

#[test]
fn fourth_and_zero_weight_indices_must_still_address_the_live_palette() {
    let body = CharacterShaderFamily::Character;
    assert!(matches!(
        vertex_bytes(&geometry([0, 0, 4, 0]), 4, body),
        Err(CharacterGpuError::PaletteIndex { index: 4, .. })
    ));
    assert!(matches!(
        vertex_bytes(&geometry([0, 0, 0, 4]), 4, body),
        Err(CharacterGpuError::PaletteIndex { index: 4, .. })
    ));
    assert!(vertex_bytes(&geometry([0, 0, 0, 0]), 0, body).is_err());
    assert!(vertex_bytes(&geometry([0, 0, 0, 0]), 62, body).is_err());
}

#[test]
fn live_rows_are_exact_and_ropa_requires_the_actual_gpu_branch() {
    let rows = [[f32::from_bits(0x7fc0_1234), -0.0, 3.0, 7.0]; 3];
    let matrix = [0.0; 16];
    let mut pose = CharacterPoseRows {
        living_world: None,
        rows: &rows,
        world_to_clip: &matrix,
        camera_origin: [0.0; 3],
        source_generation: 17,
        ropa_vertex_params_x: None,
        ghost: CharacterGhostState {
            opacity: 1.0,
            ghosted: false,
        },
    };
    let bytes = row_bytes(1, CharacterShaderFamily::Character, &pose).unwrap();
    assert_eq!(bytes.len(), ROW_BYTES);
    assert_eq!(&bytes[..4], &0x7fc0_1234_u32.to_ne_bytes());
    assert_eq!(&bytes[4..8], &(-0.0_f32).to_ne_bytes());
    assert!(matches!(
        row_bytes(1, CharacterShaderFamily::Ropa, &pose),
        Err(CharacterGpuError::MissingRopaParameter)
    ));
    for x in [0.0, -0.0, -1.0, f32::NAN] {
        pose.ropa_vertex_params_x = Some(x);
        assert!(matches!(
            row_bytes(1, CharacterShaderFamily::Ropa, &pose),
            Err(CharacterGpuError::CpuClothVerticesRequired)
        ));
    }
    pose.ropa_vertex_params_x = Some(1.0);
    assert!(row_bytes(1, CharacterShaderFamily::Ropa, &pose).is_ok());
    assert!(matches!(
        row_bytes(2, CharacterShaderFamily::Hair, &pose),
        Err(CharacterGpuError::RowCount {
            expected: 6,
            actual: 3
        })
    ));
}

#[test]
fn ghost_technique_exists_only_in_the_body_manifests() {
    assert!(CharacterShaderFamily::Character.has_ghost_technique());
    assert!(CharacterShaderFamily::Ropa.has_ghost_technique());
    assert!(!CharacterShaderFamily::Hair.has_ghost_technique());
}

#[test]
#[ignore = "requires original SKATE2_GAME_ROOT archives"]
fn original_tutorial_male_lod0_character_vertex_admission() {
    use skate2_formats::recipe::CharacterRecipe;
    use std::{collections::BTreeSet, path::PathBuf};

    let root = PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT"));
    let mounts = crate::mount_reference_archives(&root).expect("original archives");
    let definition =
        crate::load_default_marquee_definition(&mounts).expect("actual default recipe");
    let recipe_path = definition.binary_recipe_path();
    assert!(
        recipe_path.ends_with("tutorial_male.recipe"),
        "unexpected selected recipe {recipe_path}"
    );
    let recipe = CharacterRecipe::parse(&mounts.read_path(&recipe_path).unwrap()).unwrap();
    assert_eq!(recipe.version, 7);
    let scene =
        crate::own_character_scene(&mounts, &recipe).expect("retained original character scene");
    let mut failures = Vec::new();
    let mut descriptors = BTreeSet::new();
    let mut mesh_count = 0;
    let mut part_count = 0;
    let mut palette_entries = 0;
    for component in &scene.components {
        for model in &component.models {
            for lod in model.lods.iter().filter(|lod| lod.level == 0) {
                part_count += 1;
                for instance in &lod.arena.instances {
                    let model_index = instance.render_model.expect("original instance model");
                    let render_model = &lod.arena.models[model_index];
                    for reference in &render_model.meshes {
                        let mesh = &lod.arena.meshes[reference.mesh_index];
                        let geometry = &mesh.geometry;
                        let count = u16::try_from(mesh.skin_palette.len()).unwrap();
                        for &bone in &mesh.skin_palette {
                            assert!(usize::from(bone) < render_model.bone_names.len());
                        }
                        let formats = geometry
                            .attributes
                            .iter()
                            .map(|a| (a.semantic, a.original_format))
                            .collect::<Vec<_>>();
                        descriptors.insert((geometry.position_format, formats.clone()));
                        if let Err(error) =
                            vertex_bytes(geometry, count, CharacterShaderFamily::Character)
                        {
                            failures.push(format!("{} {} mesh{} palette{count}: {error}; position={} attributes={formats:?}", component.name, lod.entry_path, reference.mesh_index, geometry.position_format));
                        }
                        mesh_count += 1;
                        palette_entries += mesh.skin_palette.len();
                    }
                }
            }
        }
    }
    eprintln!(
        "actual tutorial_male LOD0: {part_count} parts, {mesh_count} meshes, {palette_entries} palette entries; descriptors={descriptors:?}"
    );
    assert_eq!(part_count, 10);
    assert!(mesh_count >= part_count);
    assert!(
        failures.is_empty(),
        "unsupported original character descriptors:\n{}",
        failures.join("\n")
    );
}
