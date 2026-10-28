use super::*;

fn fixture(head_parent: i32, include_wheels: bool) -> SkeletonData {
    let (mut hierarchy, mut parameters, matrices) = crate::skeleton_data_tests::fixture();
    let mut rename = |index: usize, name: &str| {
        let packed = PackedFastString::encode(name).unwrap();
        hierarchy.bone_names[index] = name.into();
        hierarchy.packed_bone_names[index] = packed;
        parameters.bones[index].name = name.into();
        parameters.bones[index].packed_name = packed;
    };
    rename(1, "Head");
    if include_wheels {
        for (index, name) in NAMES[..6].iter().enumerate() {
            rename(10 + index, name);
        }
    }
    hierarchy.parents[1] = head_parent;
    crate::skeleton_data_tests::build(hierarchy, parameters, &matrices).unwrap()
}

#[test]
fn outer_named_indices_retain_exact_order_and_head_part_ancestry() {
    let data = fixture(2, true);
    let mut state = SkeletonInitState::new();
    assert_eq!(state.animation_bones, 0);
    assert_eq!(state.named_bones, [None; 7]);
    state.try_initialize(&data).unwrap();
    assert!(state.complete());
    assert_eq!(state.animation_bones, 24);
    assert_eq!(state.trajectory_node, 0);
    assert_eq!(
        state.named_bones,
        [
            Some(10),
            Some(11),
            Some(12),
            Some(13),
            Some(14),
            Some(15),
            Some(1)
        ]
    );
    assert_eq!(state.head_parent, Some(2));
    assert_eq!(
        state.head_parent_part,
        Some(SkeletonPartIndex::from_index(2).unwrap())
    );
    assert_eq!(
        state.head_grandparent_part,
        Some(Some(SkeletonPartIndex::from_index(3).unwrap()))
    );
    assert_eq!(
        state.try_initialize(&data),
        Err(SkeletonInitIndicesError::AlreadyAttempted)
    );
}

#[test]
fn absent_trucks_and_wheels_survive_as_source_minus_one() {
    let data = fixture(2, false);
    let mut state = SkeletonInitState::new();
    state.try_initialize(&data).unwrap();
    assert_eq!(&state.named_bones[..6], &[Some(-1); 6]);
    assert!(state.complete());
}

#[test]
fn missing_head_retains_all_lookups_without_ancestry_publication() {
    let (hierarchy, parameters, matrices) = crate::skeleton_data_tests::fixture();
    let data = crate::skeleton_data_tests::build(hierarchy, parameters, &matrices).unwrap();
    let mut state = SkeletonInitState::new();
    assert_eq!(
        state.try_initialize(&data),
        Err(SkeletonInitIndicesError::MissingHead)
    );
    assert_eq!(state.named_bones, [Some(-1); 7]);
    assert_eq!(state.head_parent, None);
    assert_eq!(state.head_grandparent_part, None);
    assert!(!state.complete());
}

#[test]
fn invalid_head_parent_retains_prior_store_and_stops_before_part() {
    let data = fixture(-1, true);
    let mut state = SkeletonInitState::new();
    assert_eq!(
        state.try_initialize(&data),
        Err(SkeletonInitIndicesError::InvalidHeadParent {
            parent: -1,
            animation_bones: 24
        })
    );
    assert_eq!(state.head_parent, Some(-1));
    assert_eq!(state.head_parent_part, None);
    assert!(!state.complete());
}

#[test]
fn init_buffers_have_zero_w_and_do_not_publish_a_processed_frame() {
    let mut buffers = crate::skeleton_animation_targets::SkeletonAnimationTargets::new();
    assert!(buffers.animation_rows().iter().all(Option::is_none));
    buffers.initialize_skeleton_init_buffers();
    for rows in [buffers.animation_rows(), buffers.volume_rows()] {
        for row in rows {
            assert_eq!(row, &Some(crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM));
            let row = row.unwrap();
            assert_eq!([row[3], row[7], row[11], row[15]].map(f32::to_bits), [0; 4]);
        }
    }
    assert!(buffers.completed_volumes().is_none());
}

#[test]
fn animation_activation_uses_transferred_instance_instead_of_rebinding_default() {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    const CLASS: u64 = 0x74bc_9c93_affe_6aac;
    const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
    const THRESHOLD: u64 = 0xb997_72c1_07b9_a8a6;
    let mut vault = graph(CLASS, vec![definition(THRESHOLD, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        DEFAULT,
        0,
        None,
        vec![node(THRESHOLD, 0x40, 5.0_f32.to_bits())],
    );
    collection(
        &mut vault,
        CLASS,
        17,
        0,
        None,
        vec![node(THRESHOLD, 0x40, 17.0_f32.to_bits())],
    );
    let vault = std::sync::Arc::new(vault);
    let instance = vault.instance(CLASS, 17);
    let retained =
        crate::skeleton_current_animation::SkeletonCurrentAnimationAttributes::from_constructor(
            &vault, instance,
        );
    assert_eq!(
        retained.skateboard_animation_threshold().unwrap().to_bits(),
        17.0_f32.to_bits()
    );
}

#[test]
fn constructor_handoff_installs_reader_without_reinitializing_ik() {
    use crate::attribute_runtime::tests::{collection, definition, graph, node};
    use crate::skeleton_assembly_construction::constructor::{
        SkeletonConstructorRuntime, SkeletonOuterAttributes,
    };
    const CLASS: u64 = 0x74bc_9c93_affe_6aac;
    const KEY: u64 = 0xb997_72c1_07b9_a8a6;
    let mut vault = graph(CLASS, vec![definition(KEY, 4, 4)]);
    collection(
        &mut vault,
        CLASS,
        17,
        0,
        None,
        vec![node(KEY, 0x40, 17.0_f32.to_bits())],
    );
    let vault = std::sync::Arc::new(vault);
    let mut constructor = SkeletonConstructorRuntime::new();
    constructor.outer_attributes = Some(SkeletonOuterAttributes {
        animation: None,
        physics_animation: Some(vault.instance(CLASS, 17)),
        physics_animation_layout: None,
        physics_skeleton: None,
        physics_skeleton_layout: None,
        physics_animation_transferred: false,
    });
    let mut normal = crate::dynamic_assembly_session::skeleton_normal::tests::fixture();
    assert!(!normal.current_animation_initialized());
    // This partial-constructor fixture deliberately has no IK initialization.
    // Installing the attribute reader must not manufacture that separate step.
    let initialized = normal.current_animation().ik().initialized();
    normal.retain_constructor_runtime(&mut constructor, &vault);
    assert!(normal.current_animation_initialized());
    assert_eq!(
        crate::dynamic_assembly_session::skeleton_normal::tests::retained_animation_threshold(
            &normal
        )
        .to_bits(),
        17.0_f32.to_bits()
    );
    assert!(constructor.take_physics_animation_instance().is_none());
    assert_eq!(normal.current_animation().ik().initialized(), initialized);
    assert!(!normal.initialize_current_animation(&vault));
}
