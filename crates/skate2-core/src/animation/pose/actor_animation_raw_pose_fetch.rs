//! RAW `FetchPose` destination mapping for the retained hand/face ACS hierarchy.
use super::{ActorFaceError, AnimationPoseRef};
use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};

impl AnimationPoseRef {
    /// Same mapped-part lookup as the existing VBR `FetchPose`. The original RAW
    /// pose decoder materializes source S/Q/T channels; no VBR/default fallback.
    pub(crate) fn raw_sqt_for(
        &self,
        hierarchy: &AndaleHierarchy,
    ) -> Result<Vec<AndaleSqt>, ActorFaceError> {
        let parsed = std::sync::Arc::clone(
            self.database
                .lock()
                .map_err(|_| ActorFaceError::Lock)?
                .parsed
                .as_ref()
                .ok_or(ActorFaceError::MissingRecord)?,
        );
        let record = parsed
            .records
            .get(self.record_index)
            .ok_or(ActorFaceError::MissingRecord)?;
        if record.header.codec != 0x0000_4452 {
            return Err(ActorFaceError::UnsupportedPose);
        }
        let pose = parsed
            .poses
            .iter()
            .find(|pose| pose.record_offset == record.offset)
            .ok_or(ActorFaceError::MissingRecord)?;
        let mut output = vec![None; hierarchy.bone_names.len()];
        for target in &hierarchy.parts {
            let part = pose
                .parts
                .iter()
                .find(|part| u16::from(part.part_name_index) == target.name_index)
                .ok_or(ActorFaceError::PartMapping)?;
            let sqts = part.sample_raw_pose_sqt()?;
            let start =
                usize::try_from(target.bone_offset).map_err(|_| ActorFaceError::PartMapping)?;
            if sqts.len()
                != usize::try_from(target.bone_count).map_err(|_| ActorFaceError::PartMapping)?
            {
                return Err(ActorFaceError::PartMapping);
            }
            let target = output
                .get_mut(start..start + sqts.len())
                .ok_or(ActorFaceError::PartMapping)?;
            for (destination, value) in target.iter_mut().zip(sqts) {
                *destination = Some(value);
            }
        }
        output
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(ActorFaceError::PartMapping)
    }
}
