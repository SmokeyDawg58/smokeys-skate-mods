use super::*;
use crate::actor_face_playback::FaceAnimationManagers;
use skate2_formats::andale::{AndaleBlendSpaceVolume, AndaleBlendSpaceVolumeSet};

fn fixture() -> (
    AnimationTreePool,
    AnimationNodeHandle,
    [AnimationNodeHandle; 2],
) {
    let mut pool = AnimationTreePool::new(2048, 8);
    let node = pool.push_blend_space().unwrap();
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let header = AnimationClipRef {
        database,
        record_index: 0,
    };
    let data = Arc::new(AndaleBlendSpace {
        record_offset: 0,
        name: "fixture".into(),
        guid: 0,
        maximum_dimensions: 2,
        animation_names: vec!["a".into(), "b".into()],
        volume_sets: vec![AndaleBlendSpaceVolumeSet {
            name: "one dimension".into(),
            dimensions: 1,
            dimension_names: vec!["x".into()],
            packed_dimension_names: vec![
                skate2_formats::andale::PackedFastString::encode("x").unwrap(),
            ],
            serialized_size: 0,
            volumes: vec![AndaleBlendSpaceVolume {
                animation_indexes_offset: 0,
                heights_offset: 0,
                normals_offset: 0,
                matrix_coordinates: vec![],
                serialized_data: [1u32.to_be_bytes(), 0u32.to_be_bytes()].concat(),
                serialized_following_word: None,
            }],
        }],
    });
    pool.initialize_blend_space(&node, header.clone(), &data)
        .unwrap();
    let a = pool.push_clip().unwrap();
    let b = pool.push_clip().unwrap();
    for (index, child) in [&a, &b].into_iter().enumerate() {
        let AnimationNodeKind::Clip(clip) = &mut pool.node_mut(child).unwrap().kind else {
            unreachable!()
        };
        clip.header = Some(header.clone());
        clip.length = if index == 0 { 2.0 } else { 4.0 };
        clip.frames = clip.length + 1.0;
        clip.fps = 1.0;
        clip.speed = 1.0;
        pool.set_blend_space_child(&node, index, Some(child.clone()))
            .unwrap();
    }
    (pool, node, [a, b])
}
#[test]
fn initialized_vectors_and_active_indexes_drive_time_without_parent_set_time_store() {
    let (mut pool, node, [a, b]) = fixture();
    let state = pool.blend_space(&node).unwrap();
    assert_eq!(
        state
            .attributes
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        [0, 1.0f32.to_bits(), 0]
    );
    assert_eq!(state.vector_location, 48);
    assert_eq!(pool.length(&node).unwrap().to_bits(), 4.0f32.to_bits());
    pool.set_time(&node, 2.0).unwrap();
    assert_eq!(pool.time(&a).unwrap().to_bits(), 1.0f32.to_bits());
    assert_eq!(pool.time(&b).unwrap().to_bits(), 2.0f32.to_bits());
    assert_eq!(pool.time(&node).unwrap().to_bits(), 0);
    let mut property = AnimationProperties {
        is_playing: true,
        ..AnimationProperties::default()
    };
    pool.add_time(&node, 0.5, &mut property).unwrap();
    assert_eq!(pool.time(&node).unwrap().to_bits(), 2.5f32.to_bits());
    assert!(property.is_playing);
    assert_eq!(pool.blend_space(&node).unwrap().vector_location, 48);
}
#[test]
fn zero_weight_child_is_still_read_and_failed_add_time_retains_stack_prefix() {
    let (mut pool, node, [_, _]) = fixture();
    // Volume slot1 has coefficient zero but still dereferences declaration slot0.
    pool.set_blend_space_child(&node, 0, None).unwrap();
    assert!(matches!(
        pool.length(&node),
        Err(AnimationTreeError::MissingChild)
    ));
    let AnimationNodeKind::BlendSpace(state) = &mut pool.node_mut(&node).unwrap().kind else {
        unreachable!()
    };
    state.properties[0].is_playing = true;
    state.properties[1].is_playing = true;
    let mut property = AnimationProperties::default();
    assert!(matches!(
        pool.add_time(&node, 0.1, &mut property),
        Err(AnimationTreeError::MissingChild)
    ));
    let state = pool.blend_space(&node).unwrap();
    assert_eq!(state.vector_location, 60);
    assert!(!state.properties[0].is_playing);
    assert!(state.properties[1].is_playing);
}
#[test]
fn speed_visits_declared_order_and_preserves_first_write_before_null_second_child() {
    let (mut pool, node, [a, _]) = fixture();
    pool.set_blend_space_child(&node, 1, None).unwrap();
    assert!(matches!(
        pool.set_speed(&node, 2.0),
        Err(AnimationTreeError::MissingChild)
    ));
    assert_eq!(pool.length(&a).unwrap().to_bits(), 1.0f32.to_bits());
}
