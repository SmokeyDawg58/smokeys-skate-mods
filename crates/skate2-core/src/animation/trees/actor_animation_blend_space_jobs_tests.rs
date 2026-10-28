use super::super::update::BlendSpaceAttributeContext;
use super::*;
use crate::actor_animation_tree::attributes::SettableTreeAttribute;
use crate::{
    actor_animation_tree::{AnimationClipRef, AnimationNodeKind, AnimationTreePool},
    actor_face_playback::FaceAnimationManagers,
    animation_job_owners::{
        AnimationEntryPoint, AnimationJobManager, AnimationJobScheduler,
        execution::AnimationQueuedJob,
    },
    input_cvar_runtime::InputCvarManager,
};
use skate2_formats::andale::{
    AndaleBlendSpace, AndaleBlendSpaceVolume, AndaleBlendSpaceVolumeSet, PackedFastString,
};
fn fixture() -> (
    AnimationTreePool,
    AnimationNodeHandle,
    InputCvarReferences,
    Arc<Mutex<BlendSpaceJobBuffer>>,
) {
    let refs = InputCvarReferences::new_static_initialized();
    InputCvarManager::new().try_link(&refs).unwrap();
    let mut bytes = vec![0; 184];
    let vertices = [
        [0., 0., 0., 1.],
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
    ];
    let normals = [
        [-1., -1., -1., 0.],
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
    ];
    for (offset, values) in [(20, vertices), (88, normals)] {
        for (i, value) in values.into_iter().flatten().enumerate() {
            bytes[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&f32::to_be_bytes(value));
        }
    }
    for i in 0..4 {
        bytes[152 + 4 * i..156 + 4 * i].copy_from_slice(&1f32.to_be_bytes());
        bytes[168 + 4 * i..172 + 4 * i].copy_from_slice(&u32::try_from(i).unwrap().to_be_bytes());
    }
    let data = Arc::new(AndaleBlendSpace {
        record_offset: 0,
        name: "tetra".into(),
        guid: 0,
        maximum_dimensions: 3,
        animation_names: vec!["a".into(), "b".into(), "c".into(), "d".into()],
        volume_sets: vec![AndaleBlendSpaceVolumeSet {
            name: "3D".into(),
            dimensions: 3,
            dimension_names: vec!["x".into(), "y".into(), "z".into()],
            packed_dimension_names: ["x", "y", "z"]
                .map(|v| PackedFastString::encode(v).unwrap())
                .to_vec(),
            serialized_size: 184,
            volumes: vec![AndaleBlendSpaceVolume {
                animation_indexes_offset: 168,
                heights_offset: 152,
                normals_offset: 84,
                matrix_coordinates: vertices.into_iter().flatten().collect(),
                serialized_data: bytes,
                serialized_following_word: None,
            }],
        }],
    });
    let mut pool = AnimationTreePool::new(4096, 16);
    let handle = pool.push_blend_space().unwrap();
    let managers = FaceAnimationManagers::new();
    let database = managers.database.lock().unwrap().allocate().unwrap();
    let header = AnimationClipRef {
        database,
        record_index: 0,
    };
    pool.initialize_blend_space(&handle, header.clone(), &data)
        .unwrap();
    for index in 0..4 {
        let child = pool.push_clip().unwrap();
        let AnimationNodeKind::Clip(clip) = &mut pool.node_mut(&child).unwrap().kind else {
            unreachable!()
        };
        clip.header = Some(header.clone());
        clip.length = 2.;
        clip.frames = 3.;
        clip.fps = 1.;
        clip.speed = 1.;
        pool.set_blend_space_child(&handle, index, Some(child))
            .unwrap();
    }
    (
        pool,
        handle,
        refs,
        Arc::new(Mutex::new(BlendSpaceJobBuffer::initialized())),
    )
}
fn attributes(x: f32) -> [SettableTreeAttribute; 3] {
    [
        SettableTreeAttribute {
            sequence_id: -1,
            name: PackedFastString::encode("x").unwrap(),
            value: x,
            normalized: false,
        },
        SettableTreeAttribute {
            sequence_id: -1,
            name: PackedFastString::encode("y").unwrap(),
            value: 0.25,
            normalized: false,
        },
        SettableTreeAttribute {
            sequence_id: -1,
            name: PackedFastString::encode("z").unwrap(),
            value: 0.25,
            normalized: false,
        },
    ]
}
#[test]
fn actual_scheduler_writes_same_node_vectors_before_complete_retimes() {
    let (mut pool, node, cvars, buffer) = fixture();
    pool.set_blend_space_attributes(
        &node,
        &attributes(0.25),
        &mut BlendSpaceAttributeContext {
            cvars: &cvars,
            queue: Arc::clone(&buffer),
            pool: BlendSpacePool::Body(0),
        },
    )
    .unwrap();
    assert_eq!(buffer.lock().unwrap().records.len(), 1);
    let parameters = Arc::clone(&pool.blend_space(&node).unwrap().parameters);
    assert!(!parameters.lock().unwrap().ready);
    let mut manager = AnimationJobManager::new();
    manager.try_initialize_local_backend().unwrap();
    let backend = manager.scheduler.as_ref().unwrap();
    let job = AnimationJobScheduler::new().create_job();
    job.lock().unwrap().entry_point =
        AnimationEntryPoint::new(Some(AnimationJobCode::BlendSpace), Some(b"AnimBlendSpace"));
    let jobs = [AnimationQueuedJob {
        job: Arc::clone(&job),
        dependencies: vec![],
        work: Some(Arc::new(BlendSpaceWork {
            buffer: Arc::clone(&buffer),
            cvars: cvars.clone(),
        })),
    }];
    backend.submit_blend_space(&jobs).unwrap();
    backend.wait_in_category(&job, 0).unwrap();
    {
        let output = parameters.lock().unwrap();
        assert!(output.ready);
        assert!(!output.is_data_needed);
        assert_eq!(
            output
                .projected
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            [0.25, 0.25, 0.25, 1.].map(f32::to_bits)
        );
    }
    pool.complete_blend_space3(&node, &cvars).unwrap();
    backend.release_selected(&jobs).unwrap();
    assert!(buffer.lock().unwrap().failure.is_none());
}
#[test]
fn duplicate_node_uses_latest_params_then_second_record_asserts_without_clearing_queue() {
    let (mut pool, node, cvars, buffer) = fixture();
    let mut context = BlendSpaceAttributeContext {
        cvars: &cvars,
        queue: Arc::clone(&buffer),
        pool: BlendSpacePool::Body(0),
    };
    pool.set_blend_space_attributes(&node, &attributes(0.125), &mut context)
        .unwrap();
    pool.set_blend_space_attributes(&node, &attributes(0.25), &mut context)
        .unwrap();
    let work = BlendSpaceWork {
        buffer: Arc::clone(&buffer),
        cvars,
    };
    assert!(work.run(AnimationJobCode::BlendSpace).is_err());
    let queue = buffer.lock().unwrap();
    assert_eq!(queue.records.len(), 2);
    assert!(matches!(
        queue.failure.as_deref(),
        Some(BlendSpaceJobError::Missing(
            "BlendSpace isDataNeeded assertion"
        ))
    ));
    let output = queue.records[0].parameters.lock().unwrap();
    assert!(output.ready);
    assert!(!output.is_data_needed);
    assert_eq!(output.projected[0].to_bits(), 0.25f32.to_bits());
}
#[test]
fn ninth_register_fails_after_initiate_resets_same_params() {
    let (mut pool, node, cvars, buffer) = fixture();
    let mut context = BlendSpaceAttributeContext {
        cvars: &cvars,
        queue: Arc::clone(&buffer),
        pool: BlendSpacePool::Body(0),
    };
    for _ in 0..8 {
        pool.set_blend_space_attributes(&node, &attributes(0.125), &mut context)
            .unwrap();
    }
    assert!(matches!(
        pool.set_blend_space_attributes(&node, &attributes(0.25), &mut context),
        Err(BlendSpaceJobError::Capacity)
    ));
    assert_eq!(buffer.lock().unwrap().records.len(), 8);
    let output = pool.blend_space(&node).unwrap().parameters.lock().unwrap();
    assert!(!output.is_data_needed);
    assert!(!output.ready);
    assert_eq!(output.query.unwrap()[0].to_bits(), 0.25f32.to_bits());
}

#[test]
fn complete_retimes_new_children_in_order_without_ready_gate_or_early_volume_publish() {
    let (mut pool, node, cvars, _) = fixture();
    let old = pool.blend_space(&node).unwrap().active_child(0).unwrap();
    pool.set_time(&old, 1.).unwrap();
    let changed = pool.blend_space(&node).unwrap().active_child(2).unwrap();
    let AnimationNodeKind::Clip(clip) = &mut pool.node_mut(&changed).unwrap().kind else {
        unreachable!()
    };
    clip.length = 4.;
    clip.frames = 5.;
    let AnimationNodeKind::BlendSpace(state) = &mut pool.node_mut(&node).unwrap().kind else {
        unreachable!()
    };
    let mut data = (**state.data.as_ref().unwrap()).clone();
    let mut volume = data.volume_sets[0].volumes[0].clone();
    for (i, value) in [2u32, 3, 0, 1].into_iter().enumerate() {
        volume.serialized_data[168 + 4 * i..172 + 4 * i].copy_from_slice(&value.to_be_bytes());
    }
    data.volume_sets[0].volumes.push(volume);
    state.data = Some(Arc::new(data));
    {
        let mut output = state.parameters.lock().unwrap();
        output.found = Some(1);
        output.ready = false;
    }
    pool.set_blend_space_child(&node, 3, None).unwrap();
    assert!(matches!(
        pool.complete_blend_space3(&node, &cvars),
        Err(BlendSpaceJobError::Tree(AnimationTreeError::MissingChild))
    ));
    assert_eq!(pool.time(&changed).unwrap().to_bits(), 2f32.to_bits());
    assert_eq!(pool.blend_space(&node).unwrap().volume, 0);
}
