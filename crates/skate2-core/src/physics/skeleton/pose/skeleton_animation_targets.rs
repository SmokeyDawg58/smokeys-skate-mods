//! `Skeleton::ProcessData` mapped-pose loop (0x82b5efe0..0x82b5f148).
//! These are the actual owner's raw animation and animation-volume rows. This
//! mapping precedes state adjustments/IK. The current-animation owner mutates
//! these same volume rows before recording the adjusted pose.
use crate::{animation::AnimationMatrix, skeleton_data::SkeletonData};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkPoseError {
    AnimationUnavailable {
        part: crate::ragdoll::SkeletonPartIndex,
    },
    MappedUnavailable {
        part: crate::ragdoll::SkeletonPartIndex,
    },
    UnknownMappedW {
        part: crate::ragdoll::SkeletonPartIndex,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SkeletonAnimationTargetError {
    pub(crate) part: usize,
    pub(crate) bone: usize,
    pub(crate) pose_count: usize,
}

/// Unwritten rows remain None; a failed later row retains earlier source writes
/// and previous later rows. `completed` describes only the most recent mapping.
pub(crate) struct SkeletonAnimationTargets {
    animation: [Option<AnimationMatrix>; 24],
    volumes: [Option<AnimationMatrix>; 24],
    completed: bool,
}
impl SkeletonAnimationTargets {
    pub(crate) const fn new() -> Self {
        Self {
            animation: [None; 24],
            volumes: [None; 24],
            completed: false,
        }
    }

    /// `Skeleton::Init82b62a18..58` writes both24 buffers after Drives and before
    /// Set. Native I/J/K/zero P has all four W lanes zero. This does not publish
    /// a processed animation frame, even though every row is now initialized.
    pub(crate) fn initialize_skeleton_init_buffers(&mut self) {
        let identity = crate::skateboard_wheels::FRESH_VOLUME_TRANSFORM;
        for part in 0..24 {
            self.animation[part] = Some(identity);
            self.volumes[part] = Some(identity);
        }
        self.completed = false;
    }

    pub(crate) fn try_map_current_pose(
        &mut self,
        data: &SkeletonData,
        pose: &[AnimationMatrix],
    ) -> Result<(), SkeletonAnimationTargetError> {
        self.completed = false;
        for part in 0..24 {
            let bone = data.bone_ids()[part];
            let matrix = *pose.get(bone).ok_or(SkeletonAnimationTargetError {
                part,
                bone,
                pose_count: pose.len(),
            })?;
            self.animation[part] = Some(matrix);
            self.volumes[part] = Some(crate::skateboard_spawn_math::skateboard_spawn_compose(
                &matrix,
                &data.bone_to_volume()[part],
            ));
        }
        self.completed = true;
        Ok(())
    }

    pub(crate) fn completed_volumes(&self) -> Option<[AnimationMatrix; 24]> {
        self.completed.then(|| {
            self.volumes
                .map(|row| row.expect("completed source mapping"))
        })
    }

    /// Actual retained board volume, after successful mapping. Offset application
    /// at 0x82b42040 writes this row before updating retained IK targets.
    pub(crate) fn completed_skateboard_mut(&mut self) -> Option<&mut AnimationMatrix> {
        if self.completed {
            self.volumes[0].as_mut()
        } else {
            None
        }
    }

    #[cfg(test)]
    pub(crate) const fn animation_rows(&self) -> &[Option<AnimationMatrix>; 24] {
        &self.animation
    }
    pub(crate) fn volume_row(&self, part: usize) -> Option<&AnimationMatrix> {
        self.volumes.get(part).and_then(Option::as_ref)
    }
    /// Skeleton+3640 raw bone rows, distinct from the volume-offset rows+3040.
    pub(crate) fn animation_row(&self, part: usize) -> Option<&AnimationMatrix> {
        self.animation.get(part).and_then(Option::as_ref)
    }
    #[cfg(test)]
    pub(crate) const fn volume_rows(&self) -> &[Option<AnimationMatrix>; 24] {
        &self.volumes
    }
}

/// Same actual Part-order buffers used by `ProcessData` and retained Drives.
/// Each row is checked at its source access; earlier IK writes survive errors.
impl crate::skeleton_ik_prephysics::SkeletonIkPrePhysicsRows for SkeletonAnimationTargets {
    type Error = SkeletonIkPoseError;
    fn animation_position(
        &mut self,
        part: crate::ragdoll::SkeletonPartIndex,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, Self::Error> {
        let row = self.animation[part.index()]
            .as_ref()
            .ok_or(Self::Error::AnimationUnavailable { part })?;
        Ok(crate::skeleton_root_transforms::SkeletonRootVector::known(
            std::array::from_fn(|lane| row[12 + lane]),
        ))
    }
    fn mapped_transform(
        &mut self,
        part: crate::ragdoll::SkeletonPartIndex,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error> {
        let row = self.volumes[part.index()].ok_or(Self::Error::MappedUnavailable { part })?;
        Ok(crate::skeleton_root_transforms::SkeletonRootMatrix::known(
            row,
        ))
    }
    fn set_mapped_transform(
        &mut self,
        part: crate::ragdoll::SkeletonPartIndex,
        value: &crate::skeleton_root_transforms::SkeletonRootMatrix,
    ) -> Result<(), Self::Error> {
        let row = self.volumes[part.index()]
            .as_mut()
            .ok_or(Self::Error::MappedUnavailable { part })?;
        // These actual buffers store all16 lanes. A partial host value cannot
        // be represented by replacing its unknown W with guessed bits.
        *row = value
            .known_lanes()
            .ok_or(Self::Error::UnknownMappedW { part })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton_data_tests::{build, fixture};

    #[test]
    fn ik_rows_use_part_order_preserve_raw_pose_and_keep_completed_volume_authority() {
        use crate::{
            ragdoll::SkeletonPartIndex as Part,
            skeleton_ik_prephysics::SkeletonIkPrePhysicsRows,
            skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
        };
        let (mut hierarchy, parameters, mut pose) = fixture();
        hierarchy.bone_names.swap(1, 2);
        hierarchy.packed_bone_names.swap(1, 2);
        let data = build(hierarchy, parameters, &pose).unwrap();
        pose[1][12] = 11.0;
        pose[2][12] = 23.0;
        let mut targets = SkeletonAnimationTargets::new();
        targets.try_map_current_pose(&data, &pose).unwrap();
        let part = Part::from_index(1).unwrap();
        assert_eq!(
            targets.animation_position(part).unwrap().xyz[0].to_bits(),
            23.0_f32.to_bits()
        );
        let raw = *targets.animation_rows();
        let before = targets.completed_volumes().unwrap();
        let replacement =
            SkeletonRootMatrix::translation(SkeletonRootVector::known([3.0, 5.0, 7.0, 11.0]));
        targets.set_mapped_transform(part, &replacement).unwrap();
        assert_eq!(targets.mapped_transform(part).unwrap(), replacement);
        assert_eq!(targets.animation_rows(), &raw);
        let after = targets.completed_volumes().unwrap();
        for index in 0..24 {
            assert_eq!(
                after[index].map(f32::to_bits),
                if index == 1 {
                    replacement.known_lanes().unwrap().map(f32::to_bits)
                } else {
                    before[index].map(f32::to_bits)
                }
            );
        }
    }

    #[test]
    fn ik_row_missing_and_partial_w_errors_preserve_prior_actual_writes() {
        use crate::{
            ragdoll::SkeletonPartIndex as Part,
            skeleton_ik_prephysics::SkeletonIkPrePhysicsRows,
            skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
        };
        let (hierarchy, parameters, pose) = fixture();
        let data = build(hierarchy, parameters, &pose).unwrap();
        let first = Part::from_index(0).unwrap();
        let missing = Part::from_index(1).unwrap();
        let mut targets = SkeletonAnimationTargets::new();
        assert!(targets.try_map_current_pose(&data, &pose[..1]).is_err());
        let replacement =
            SkeletonRootMatrix::translation(SkeletonRootVector::known([13.0, 17.0, 19.0, 23.0]));
        targets.set_mapped_transform(first, &replacement).unwrap();
        assert_eq!(
            targets.animation_position(missing),
            Err(SkeletonIkPoseError::AnimationUnavailable { part: missing })
        );
        assert_eq!(
            targets.mapped_transform(missing),
            Err(SkeletonIkPoseError::MappedUnavailable { part: missing })
        );
        assert_eq!(
            targets.set_mapped_transform(missing, &replacement),
            Err(SkeletonIkPoseError::MappedUnavailable { part: missing })
        );
        let mut columns = *replacement.columns();
        columns[0].w = None;
        assert_eq!(
            targets.set_mapped_transform(first, &SkeletonRootMatrix::from_columns(columns)),
            Err(SkeletonIkPoseError::UnknownMappedW { part: first })
        );
        assert_eq!(targets.mapped_transform(first).unwrap(), replacement);
        assert!(
            targets.completed_volumes().is_none(),
            "individual writes do not invent completed pose mapping"
        );
    }

    #[test]
    fn maps_actual_bone_id_before_copy_and_stops_at_first_missing_mapped_row() {
        let (mut hierarchy, parameters, initial) = fixture();
        hierarchy.bone_names.swap(1, 2);
        hierarchy.packed_bone_names.swap(1, 2);
        let data = build(hierarchy, parameters, &initial).unwrap();
        assert_eq!(data.bone_ids()[1], 2);
        assert_eq!(data.bone_ids()[2], 1);
        let mut pose = initial;
        pose[1][12] = 11.0;
        pose[2][12] = 22.0;
        let mut targets = SkeletonAnimationTargets::new();
        targets.try_map_current_pose(&data, &pose).unwrap();
        assert_eq!(
            targets.animation_rows()[1].unwrap().map(f32::to_bits),
            pose[2].map(f32::to_bits)
        );
        assert_eq!(
            targets.animation_rows()[2].unwrap().map(f32::to_bits),
            pose[1].map(f32::to_bits)
        );
        pose[0][12] = 33.0;
        let previous = *targets.animation_rows();
        assert_eq!(
            targets.try_map_current_pose(&data, &pose[..2]),
            Err(SkeletonAnimationTargetError {
                part: 1,
                bone: 2,
                pose_count: 2
            })
        );
        assert_eq!(
            targets.animation_rows()[0].unwrap().map(f32::to_bits),
            pose[0].map(f32::to_bits)
        );
        assert_eq!(
            targets.animation_rows()[1..]
                .iter()
                .map(|row| row.unwrap().map(f32::to_bits))
                .collect::<Vec<_>>(),
            previous[1..]
                .iter()
                .map(|row| row.unwrap().map(f32::to_bits))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "literal exact fixture stores")]
    fn current_pose_replaces_constructor_rows_and_preserves_full_lane_graph() {
        let (hierarchy, mut parameters, initial) = fixture();
        parameters.bones[2].position = [2.0, -3.0, 4.0, 71.0];
        let data = build(hierarchy, parameters, &initial).unwrap();
        let mut pose = initial.clone();
        pose[2] = [
            2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
        ];
        let mut targets = SkeletonAnimationTargets::new();
        assert!(targets.completed_volumes().is_none());
        targets.try_map_current_pose(&data, &pose).unwrap();
        assert_eq!(
            targets.animation_rows()[2].unwrap().map(f32::to_bits),
            pose[2].map(f32::to_bits)
        );
        assert_ne!(pose[2], data.physics_pose()[2]);
        let inner = &data.bone_to_volume()[2];
        let actual = targets.completed_volumes().unwrap()[2];
        for column in 0..4 {
            for lane in 0..4 {
                let mut expected = if column == 3 {
                    inner[column * 4].mul_add(pose[2][lane], pose[2][12 + lane])
                } else {
                    inner[column * 4] * pose[2][lane]
                };
                expected = inner[column * 4 + 1].mul_add(pose[2][4 + lane], expected);
                expected = inner[column * 4 + 2].mul_add(pose[2][8 + lane], expected);
                assert_eq!(actual[column * 4 + lane].to_bits(), expected.to_bits());
            }
        }
        assert_ne!(actual[15].to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "literal exact fixture stores")]
    fn short_new_pose_retains_updated_prefix_old_suffix_and_no_completion() {
        let (hierarchy, parameters, initial) = fixture();
        let data = build(hierarchy, parameters, &initial).unwrap();
        let mut targets = SkeletonAnimationTargets::new();
        targets.try_map_current_pose(&data, &initial).unwrap();
        let old = *targets.volume_rows();
        let mut next = initial[..2].to_vec();
        next[0][12] = 11.0;
        next[1][12] = 12.0;
        assert_eq!(
            targets.try_map_current_pose(&data, &next),
            Err(SkeletonAnimationTargetError {
                part: 2,
                bone: 2,
                pose_count: 2
            })
        );
        assert!(targets.completed_volumes().is_none());
        assert_eq!(targets.animation_rows()[0].unwrap()[12], 11.0);
        assert_eq!(targets.animation_rows()[1].unwrap()[12], 12.0);
        assert_eq!(&targets.volume_rows()[2..], &old[2..]);
    }
}
