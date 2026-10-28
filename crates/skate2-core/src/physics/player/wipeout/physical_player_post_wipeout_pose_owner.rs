//! W-lane knowledge on the actual `AnimOutPhysIn` pose allocations.
//! Child of `actor_anim_out_phys_in`; no substitute pose owner is allocated.
use super::AnimOutPhysInOwner;
use crate::{
    animation::AnimationMatrix,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoseBuffer {
    Global,
    Local,
}

/// False means the raw lane is storage only and must not be read as native W.
#[derive(Default)]
pub(crate) struct PoseWKnowledge {
    pub(crate) global: Vec<[bool; 4]>,
    pub(crate) local: Vec<[bool; 4]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PoseAccessError {
    Missing(PoseBuffer),
    Size(PoseBuffer),
    Bone(PoseBuffer, usize),
    UnknownW(PoseBuffer),
}
impl std::fmt::Display for PoseAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(buffer) => write!(f, "actual {buffer:?} pose allocation unavailable"),
            Self::Size(buffer) => write!(f, "{buffer:?} pose lane metadata size mismatch"),
            Self::Bone(buffer, bone) => write!(f, "{buffer:?} pose bone {bone} unavailable"),
            Self::UnknownW(buffer) => write!(f, "{buffer:?} pose has unknown native W lanes"),
        }
    }
}

impl AnimOutPhysInOwner {
    /// Reset825ecbc0 writes both complete matrices; call after its pose loop.
    /// Animation `GetPhysUpdateData` memcpy calls this separately after each copy.
    pub(crate) fn mark_pose_w_known(&mut self, buffer: PoseBuffer) -> Result<(), PoseAccessError> {
        let (pose, knowledge) = self.pose_storage_mut(buffer)?;
        knowledge.resize(pose.len(), [true; 4]);
        knowledge.fill([true; 4]);
        Ok(())
    }

    pub(crate) fn pose_matrix_typed(
        &self,
        buffer: PoseBuffer,
        bone: usize,
    ) -> Result<SkeletonRootMatrix, PoseAccessError> {
        let (pose, knowledge) = self.pose_storage(buffer)?;
        if pose.len() != knowledge.len() {
            return Err(PoseAccessError::Size(buffer));
        }
        let matrix = pose.get(bone).ok_or(PoseAccessError::Bone(buffer, bone))?;
        let known = knowledge[bone];
        Ok(SkeletonRootMatrix::from_columns(std::array::from_fn(
            |axis| SkeletonRootVector {
                xyz: [matrix[axis * 4], matrix[axis * 4 + 1], matrix[axis * 4 + 2]],
                w: known[axis].then_some(matrix[axis * 4 + 3]),
            },
        )))
    }

    /// `UpdatePoseBuffer82b4db78` and following stores. Unknown W invalidates its
    /// lane while retaining storage bits; no homogeneous lane is manufactured.
    pub(crate) fn set_pose_matrix_typed(
        &mut self,
        buffer: PoseBuffer,
        bone: usize,
        value: &SkeletonRootMatrix,
    ) -> Result<(), PoseAccessError> {
        let (pose, knowledge) = self.pose_storage_mut(buffer)?;
        if pose.len() != knowledge.len() {
            return Err(PoseAccessError::Size(buffer));
        }
        let matrix = pose
            .get_mut(bone)
            .ok_or(PoseAccessError::Bone(buffer, bone))?;
        for (axis, column) in value.columns().iter().enumerate() {
            matrix[axis * 4..axis * 4 + 3].copy_from_slice(&column.xyz);
            if let Some(w) = column.w {
                matrix[axis * 4 + 3] = w;
            }
            knowledge[bone][axis] = column.w.is_some();
        }
        Ok(())
    }

    /// Compatibility boundary for consumers that still require all 16 lanes.
    /// XYZ-only consumers should use `pose_matrix_typed` and select their axes.
    pub(crate) fn pose_known_lanes(
        &self,
        buffer: PoseBuffer,
    ) -> Result<&[AnimationMatrix], PoseAccessError> {
        let (pose, knowledge) = self.pose_storage(buffer)?;
        if pose.len() != knowledge.len() {
            return Err(PoseAccessError::Size(buffer));
        }
        if knowledge.iter().flatten().any(|known| !known) {
            return Err(PoseAccessError::UnknownW(buffer));
        }
        Ok(pose)
    }

    fn pose_storage(
        &self,
        buffer: PoseBuffer,
    ) -> Result<(&[AnimationMatrix], &[[bool; 4]]), PoseAccessError> {
        let (pose, knowledge) = match buffer {
            PoseBuffer::Global => (&self.global_pose, &self.pose_w.global),
            PoseBuffer::Local => (&self.local_pose, &self.pose_w.local),
        };
        Ok((
            pose.as_deref().ok_or(PoseAccessError::Missing(buffer))?,
            knowledge,
        ))
    }
    fn pose_storage_mut(
        &mut self,
        buffer: PoseBuffer,
    ) -> Result<(&mut [AnimationMatrix], &mut Vec<[bool; 4]>), PoseAccessError> {
        let (pose, knowledge) = match buffer {
            PoseBuffer::Global => (&mut self.global_pose, &mut self.pose_w.global),
            PoseBuffer::Local => (&mut self.local_pose, &mut self.pose_w.local),
        };
        Ok((
            pose.as_deref_mut()
                .ok_or(PoseAccessError::Missing(buffer))?,
            knowledge,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_w_survives_copy_and_real_reset_restores_knowledge() {
        let mut owner = AnimOutPhysInOwner::new_sim_allocation();
        owner.try_construct(2).unwrap();
        let mut matrix = owner.pose_matrix_typed(PoseBuffer::Global, 0).unwrap();
        matrix.set_column(
            3,
            SkeletonRootVector {
                xyz: [2.0, 3.0, 4.0],
                w: None,
            },
        );
        owner
            .set_pose_matrix_typed(PoseBuffer::Global, 0, &matrix)
            .unwrap();
        let copied = owner.pose_matrix_typed(PoseBuffer::Global, 0).unwrap();
        owner
            .set_pose_matrix_typed(PoseBuffer::Local, 0, &copied)
            .unwrap();
        assert_eq!(
            owner
                .pose_matrix_typed(PoseBuffer::Local, 0)
                .unwrap()
                .columns()[3]
                .w,
            None
        );
        assert!(matches!(
            owner.pose_known_lanes(PoseBuffer::Global),
            Err(PoseAccessError::UnknownW(_))
        ));
        assert_eq!(
            owner
                .pose_matrix_typed(PoseBuffer::Global, 1)
                .unwrap()
                .columns()[3]
                .w,
            Some(0.0)
        );
        owner.try_reset().unwrap();
        assert!(owner.pose_known_lanes(PoseBuffer::Global).is_ok());
        assert!(owner.pose_known_lanes(PoseBuffer::Local).is_ok());
    }
}
