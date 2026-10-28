use super::*;
use skate2_formats::andale::{AndaleSqt, PackedFastString};

fn hierarchy(bones: usize, parts: &[(&str, u32, u32)]) -> Arc<AndaleHierarchy> {
    Arc::new(AndaleHierarchy {
        has_trajectory: false,
        parents: vec![-1; bones],
        bone_metadata: vec![0; bones],
        parts: parts
            .iter()
            .map(|&(name, offset, count)| AndaleHierarchyPart {
                name: name.to_owned(),
                packed_name: PackedFastString::encode(name).unwrap(),
                bone_count: count,
                bone_offset: offset,
                is_animated: true,
                name_index: 0,
            })
            .collect(),
        bone_names: vec![String::new(); bones],
        packed_bone_names: vec![PackedFastString { words: [0; 5] }; bones],
    })
}
fn pose(value: f32) -> AndaleSqt {
    AndaleSqt {
        scale: [value; 4],
        rotation: [-value; 4],
        translation: [value, -value, 0.0, f32::from_bits(0x7fc0_1234)],
    }
}
fn initialized(hierarchy: Arc<AndaleHierarchy>, values: &[f32]) -> OwnedAnimationSqtBuffer {
    let output = OwnedAnimationSqtBuffer::allocate_for_hierarchy(hierarchy);
    output.begin_extract().unwrap();
    output
        .extract(&values.iter().copied().map(pose).collect::<Vec<_>>())
        .unwrap();
    output.complete_sync().unwrap();
    output
}
fn assert_values(output: &OwnedAnimationSqtBuffer, expected: &[f32]) {
    let (_, values) = output.read().unwrap();
    assert_eq!(
        values
            .iter()
            .map(|value| value.scale[0].to_bits())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
    for value in values {
        assert_eq!(value.translation[3].to_bits(), 0x7fc0_1234);
    }
}

#[test]
fn nested_subranges_share_allocation_and_only_extract_selected_records() {
    let output = initialized(hierarchy(4, &[("All", 0, 4)]), &[1.0, 2.0, 3.0, 4.0]);
    let view = output
        .try_subrange(1..4)
        .unwrap()
        .try_subrange(1..2)
        .unwrap();
    assert!(Arc::ptr_eq(&output.storage, &view.storage));
    assert_eq!(view.range, 2..3);
    assert_eq!(view.len().unwrap(), 1);
    assert!(view.hierarchy.is_none());
    view.begin_extract().unwrap();
    assert!(output.read().is_err());
    view.extract(&[pose(9.0)]).unwrap();
    assert!(output.read().is_err());
    view.complete_sync().unwrap();
    assert_values(&output, &[1.0, 2.0, 9.0, 4.0]);
    assert_values(&view, &[9.0]);
    assert_eq!(output.read().unwrap().0, 2);
}

#[test]
fn checked_views_do_not_invent_descriptor_or_escape_parent() {
    let output = OwnedAnimationSqtBuffer::allocate_for_hierarchy(hierarchy(4, &[]));
    assert!(output.try_subrange(3..5).is_err());
    assert!(output.try_subrange(Range { start: 3, end: 2 }).is_err());
    let view = output.try_subrange(1..3).unwrap();
    assert!(view.try_subrange(0..3).is_err());
    assert!(
        output
            .try_subrange_for_hierarchy(1..3, hierarchy(3, &[]))
            .is_err()
    );
    let actual = hierarchy(2, &[("Face", 0, 2)]);
    let view = output
        .try_subrange_for_hierarchy(1..3, Arc::clone(&actual))
        .unwrap();
    assert!(Arc::ptr_eq(view.hierarchy.as_ref().unwrap(), &actual));
}

#[test]
fn override_matches_parts_and_uses_source_count_without_clearing_untouched_bones() {
    let source = initialized(
        hierarchy(3, &[("Missing", 0, 1), ("Face", 1, 2)]),
        &[1.0, 2.0, 3.0],
    );
    // Destination part count intentionally differs: native uses source count.
    let destination = initialized(hierarchy(5, &[("Face", 2, 1)]), &[5.0; 5]);
    destination.try_override_from(&source).unwrap();
    assert_values(&destination, &[5.0, 5.0, 2.0, 3.0, 5.0]);
    assert_eq!(destination.read().unwrap().0, 2);
}

#[test]
fn same_allocation_positive_overlap_uses_forward_copy_not_snapshot_or_memmove() {
    let output = initialized(hierarchy(4, &[]), &[1.0, 2.0, 3.0, 4.0]);
    let actual = hierarchy(3, &[("All", 0, 3)]);
    let source = output
        .try_subrange_for_hierarchy(0..3, Arc::clone(&actual))
        .unwrap();
    let destination = output.try_subrange_for_hierarchy(1..4, actual).unwrap();
    destination.try_override_from(&source).unwrap();
    assert_values(&output, &[1.0; 4]);
    assert_eq!(output.read().unwrap().0, 2);
}

#[test]
fn later_source_part_observes_earlier_same_allocation_write() {
    let output = initialized(hierarchy(3, &[]), &[1.0, 2.0, 3.0]);
    let source = output
        .try_subrange_for_hierarchy(0..3, hierarchy(3, &[("A", 0, 1), ("B", 1, 1)]))
        .unwrap();
    let destination = output
        .try_subrange_for_hierarchy(0..3, hierarchy(3, &[("B", 2, 1), ("A", 1, 1)]))
        .unwrap();
    destination.try_override_from(&source).unwrap();
    assert_values(&output, &[1.0; 3]);
}

#[test]
fn later_bad_range_keeps_earlier_copy_and_failed_publication_unreadable() {
    let source = initialized(hierarchy(2, &[("A", 0, 1), ("B", 1, 3)]), &[1.0, 2.0]);
    let destination = initialized(hierarchy(2, &[("A", 0, 1), ("B", 1, 1)]), &[8.0, 9.0]);
    assert!(destination.try_override_from(&source).is_err());
    assert!(destination.read().is_err());
    let storage = destination.storage.lock().unwrap();
    assert_eq!(
        storage.values[0].unwrap().scale[0].to_bits(),
        1.0_f32.to_bits()
    );
    assert_eq!(
        storage.values[1].unwrap().scale[0].to_bits(),
        9.0_f32.to_bits()
    );
    assert_eq!(storage.generation, 2);
}

#[test]
fn missing_parts_are_skipped_and_anonymous_metadata_is_not_fabricated() {
    let source = initialized(hierarchy(1, &[("Missing", 0, 1)]), &[1.0]);
    let destination = initialized(hierarchy(1, &[("Present", 0, 1)]), &[2.0]);
    destination.try_override_from(&source).unwrap();
    assert_values(&destination, &[2.0]);
    assert_eq!(destination.read().unwrap().0, 1);
    let anonymous = OwnedAnimationSqtBuffer::allocate_unknown(1);
    assert!(anonymous.try_override_from(&source).is_err());
}

#[test]
fn synchronous_override_does_not_publish_over_pending_extraction() {
    let source = initialized(hierarchy(1, &[("A", 0, 1)]), &[1.0]);
    let destination = initialized(hierarchy(1, &[("A", 0, 1)]), &[2.0]);
    destination.begin_extract().unwrap();
    assert!(destination.try_override_from(&source).is_err());
    let storage = destination.storage.lock().unwrap();
    assert!(storage.extraction_pending);
    assert!(!storage.synchronized);
    assert_eq!(storage.generation, 1);
    assert_eq!(
        storage.values[0].unwrap().scale[0].to_bits(),
        2.0_f32.to_bits()
    );
}
