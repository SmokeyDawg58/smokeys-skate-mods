use super::{PresentationOffsets, PresentationOffsetsError};
use skate2_formats::andale::{AndaleHierarchy, AndaleHierarchyPart, PackedFastString};

fn hierarchy(bones: usize, parts: &[(&str, u32, u32)]) -> AndaleHierarchy {
    AndaleHierarchy {
        has_trajectory: false,
        parents: vec![-1; bones],
        bone_metadata: vec![0; bones],
        parts: parts
            .iter()
            .map(|&(name, start, count)| AndaleHierarchyPart {
                name: name.to_owned(),
                packed_name: PackedFastString::encode(name).unwrap(),
                bone_count: count,
                bone_offset: start,
                is_animated: true,
                name_index: 0,
            })
            .collect(),
        bone_names: vec![String::new(); bones],
        packed_bone_names: vec![PackedFastString { words: [0; 5] }; bones],
    }
}

#[test]
fn compression_order_uses_named_ranges_not_serialized_part_order() {
    let body = hierarchy(
        13,
        &[
            ("Legs", 0, 2),
            ("Head", 3, 1),
            ("Arms", 4, 2),
            ("Torso", 8, 2),
            ("Skateboard_Trucks", 11, 2),
            ("Skateboard", 10, 1),
        ],
    );
    let face = hierarchy(9, &[("Face2", 6, 3), ("Face0", 2, 2), ("Face1", 4, 1)]);
    let offsets =
        PresentationOffsets::try_initialize(&body, &hierarchy(2, &[]), &hierarchy(3, &[]), &face)
            .unwrap();
    assert_eq!(offsets.matrix_rotations, [8, 9, 3, 4, 5, 0, 1, 10, 11, 12]);
    assert_eq!(offsets.matrix_translations, [11, 12]);
    assert_eq!(offsets.face, [2..4, 4..5, 6..9]);
    assert_eq!(offsets.quaternion3_count, 11);
    assert_eq!(offsets.translation_count, 8);
    assert_eq!(offsets.pose_bytes, 194);
}

#[test]
fn missing_and_outside_parts_are_not_replaced_with_empty_ranges() {
    let empty = hierarchy(0, &[]);
    assert_eq!(
        PresentationOffsets::try_initialize(&empty, &empty, &empty, &empty),
        Err(PresentationOffsetsError::MissingPart("Torso"))
    );
    let outside = hierarchy(1, &[("Torso", 1, 1)]);
    assert_eq!(
        PresentationOffsets::try_initialize(&outside, &empty, &empty, &empty),
        Err(PresentationOffsetsError::PartOutsideHierarchy("Torso"))
    );
}
