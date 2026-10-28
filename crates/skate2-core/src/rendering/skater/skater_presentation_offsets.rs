//! Original `InitOffsets827dd548` channel ordering, derived from loaded hierarchies.
use crate::actor_graph_construction::ActorGraphIntents;
use skate2_formats::andale::{AndaleHierarchy, AndaleHierarchyPart};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PresentationOffsetsError {
    MissingPart(&'static str),
    PartOutsideHierarchy(&'static str),
    TooManyMatrixRotations,
    SizeOverflow,
}
impl std::fmt::Display for PresentationOffsetsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPart(name) => write!(f, "presentation part missing: {name}"),
            Self::PartOutsideHierarchy(name) => {
                write!(f, "presentation part outside hierarchy: {name}")
            }
            Self::TooManyMatrixRotations => {
                f.write_str("presentation matrix rotation count exceeds 120")
            }
            Self::SizeOverflow => f.write_str("presentation pose size overflow"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PresentationOffsets {
    pub(crate) matrix_rotations: Vec<usize>,
    pub(crate) matrix_translations: Vec<usize>,
    pub(crate) left_hand_count: usize,
    pub(crate) right_hand_count: usize,
    pub(crate) face: [Range<usize>; 3],
    pub(crate) quaternion3_count: usize,
    pub(crate) translation_count: usize,
    pub(crate) pose_bytes: usize,
}

pub(crate) fn part<'a>(
    hierarchy: &'a AndaleHierarchy,
    name: &'static str,
) -> Result<&'a AndaleHierarchyPart, PresentationOffsetsError> {
    let packed = ActorGraphIntents::native_name(name.as_bytes());
    hierarchy
        .parts
        .iter()
        .find(|part| part.packed_name == packed)
        .ok_or(PresentationOffsetsError::MissingPart(name))
}

pub(crate) fn part_range(
    hierarchy: &AndaleHierarchy,
    name: &'static str,
) -> Result<Range<usize>, PresentationOffsetsError> {
    let part = part(hierarchy, name)?;
    let start =
        usize::try_from(part.bone_offset).map_err(|_| PresentationOffsetsError::SizeOverflow)?;
    let count =
        usize::try_from(part.bone_count).map_err(|_| PresentationOffsetsError::SizeOverflow)?;
    let end = start
        .checked_add(count)
        .ok_or(PresentationOffsetsError::SizeOverflow)?;
    if end > hierarchy.bone_names.len() {
        return Err(PresentationOffsetsError::PartOutsideHierarchy(name));
    }
    Ok(start..end)
}

impl PresentationOffsets {
    pub(crate) fn try_initialize(
        body: &AndaleHierarchy,
        left: &AndaleHierarchy,
        right: &AndaleHierarchy,
        face: &AndaleHierarchy,
    ) -> Result<Self, PresentationOffsetsError> {
        let mut matrix_rotations = Vec::new();
        for name in ["Torso", "Head", "Arms", "Legs", "Skateboard"] {
            matrix_rotations.extend(part_range(body, name)?);
        }
        let trucks = part_range(body, "Skateboard_Trucks")?;
        matrix_rotations.extend(trucks.clone());
        let matrix_translations = trucks.collect::<Vec<_>>();
        // Native assertion occurs after the truck append at827dd948.
        if matrix_rotations.len() > 120 {
            return Err(PresentationOffsetsError::TooManyMatrixRotations);
        }
        let left_hand_count = left.bone_names.len();
        let right_hand_count = right.bone_names.len();
        let face = [
            part_range(face, "Face0")?,
            part_range(face, "Face1")?,
            part_range(face, "Face2")?,
        ];
        let face_count = face.iter().map(Range::len).sum::<usize>();
        let quaternion3_count = left_hand_count
            .checked_add(right_hand_count)
            .and_then(|count| count.checked_add(face_count))
            .ok_or(PresentationOffsetsError::SizeOverflow)?;
        let translation_count = matrix_translations
            .len()
            .checked_add(face_count)
            .ok_or(PresentationOffsetsError::SizeOverflow)?;
        let pose_bytes = quaternion3_count
            .checked_add(translation_count)
            .and_then(|count| count.checked_mul(6))
            .and_then(|bytes| matrix_rotations.len().checked_mul(8)?.checked_add(bytes))
            .ok_or(PresentationOffsetsError::SizeOverflow)?;
        Ok(Self {
            matrix_rotations,
            matrix_translations,
            left_hand_count,
            right_hand_count,
            face,
            quaternion3_count,
            translation_count,
            pose_bytes,
        })
    }
}

#[cfg(test)]
#[path = "skater_presentation_offsets_tests.rs"]
mod tests;
