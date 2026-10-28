//! Runtime-owned skeletal poses and hierarchy evaluation.
//!
//! This is the readable native equivalent of `Anim_Tools::SQTtoLocal`,
//! `Anim_Tools::Pose`, and the named-part copies in
//! `SkaterPresentation::ReadSkater`. It contains no translated PPC code.

use skate2_formats::andale::{AndaleHierarchy, AndaleSqt};
use std::{fmt, ops::Range};

pub type AnimationMatrix = [f32; 16];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneTransform {
    pub scale: [f32; 4],
    pub rotation: [f32; 4],
    pub translation: [f32; 4],
}

impl BoneTransform {
    pub const IDENTITY: Self = Self {
        scale: [1.0, 1.0, 1.0, 1.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [0.0, 0.0, 0.0, 1.0],
    };
}

impl From<AndaleSqt> for BoneTransform {
    fn from(value: AndaleSqt) -> Self {
        Self {
            scale: value.scale,
            rotation: value.rotation,
            translation: value.translation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkeletonPart {
    pub name: String,
    pub bones: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnimationSkeleton {
    pub parents: Vec<i32>,
    pub trajectory_bone: Option<usize>,
    pub parts: Vec<SkeletonPart>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalPose {
    pub bones: Vec<BoneTransform>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnimationError {
    HierarchyBoneCountMismatch {
        parents: usize,
        names: usize,
    },
    MissingTrajectoryBone,
    PartOutsideHierarchy {
        name: String,
        start: usize,
        end: usize,
        bone_count: usize,
    },
    PoseBoneCountMismatch {
        expected: usize,
        actual: usize,
    },
    MatrixCountMismatch {
        global: usize,
        inverse_bind: usize,
    },
    MissingTargetPart(String),
    PartBoneCountMismatch {
        name: String,
        source: usize,
        target: usize,
    },
    InvalidParent {
        bone: usize,
        parent: i32,
    },
}

impl fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid skeletal animation state: {self:?}")
    }
}

impl std::error::Error for AnimationError {}

impl AnimationSkeleton {
    /// Converts the serialized hierarchy into runtime indices and named ranges.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError`] for inconsistent bone tables, part ranges,
    /// or a hierarchy that claims a trajectory without naming its bone.
    pub fn from_andale(hierarchy: &AndaleHierarchy) -> Result<Self, AnimationError> {
        let bone_count = hierarchy.bone_names.len();
        if hierarchy.parents.len() != bone_count {
            return Err(AnimationError::HierarchyBoneCountMismatch {
                parents: hierarchy.parents.len(),
                names: bone_count,
            });
        }
        let trajectory_bone = hierarchy
            .has_trajectory
            .then(|| {
                hierarchy
                    .bone_names
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case("trajectory"))
                    .ok_or(AnimationError::MissingTrajectoryBone)
            })
            .transpose()?;
        let mut parts = Vec::with_capacity(hierarchy.parts.len());
        for part in &hierarchy.parts {
            let start = usize::try_from(part.bone_offset).map_err(|_| {
                AnimationError::PartOutsideHierarchy {
                    name: part.name.clone(),
                    start: usize::MAX,
                    end: usize::MAX,
                    bone_count,
                }
            })?;
            let count = usize::try_from(part.bone_count).map_err(|_| {
                AnimationError::PartOutsideHierarchy {
                    name: part.name.clone(),
                    start,
                    end: usize::MAX,
                    bone_count,
                }
            })?;
            let end =
                start
                    .checked_add(count)
                    .ok_or_else(|| AnimationError::PartOutsideHierarchy {
                        name: part.name.clone(),
                        start,
                        end: usize::MAX,
                        bone_count,
                    })?;
            if end > bone_count {
                return Err(AnimationError::PartOutsideHierarchy {
                    name: part.name.clone(),
                    start,
                    end,
                    bone_count,
                });
            }
            parts.push(SkeletonPart {
                name: part.name.clone(),
                bones: start..end,
            });
        }
        Ok(Self {
            parents: hierarchy.parents.clone(),
            trajectory_bone,
            parts,
        })
    }

    #[must_use]
    pub fn identity_pose(&self) -> LocalPose {
        LocalPose {
            bones: vec![BoneTransform::IDENTITY; self.parents.len()],
        }
    }

    /// Converts every local SQT into a matrix and composes the hierarchy.
    /// Children of the trajectory node are copied as roots, matching the
    /// reference `Anim_Tools::Pose` traversal.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError`] for a short pose or a parent that is outside
    /// the already-evaluated hierarchy prefix.
    pub fn global_matrices(
        &self,
        pose: &LocalPose,
    ) -> Result<Vec<AnimationMatrix>, AnimationError> {
        let local = self.local_matrices(pose)?;
        self.global_matrices_from_local(&local)
    }

    /// Evaluates the hierarchy from the supplied local matrices without
    /// regenerating them from SQT. Used by buffered `LocaltoGlobal82a22f18`.
    ///
    /// # Errors
    /// Returns an error for a mismatched matrix count or invalid parent order.
    pub fn global_matrices_from_local(
        &self,
        local: &[AnimationMatrix],
    ) -> Result<Vec<AnimationMatrix>, AnimationError> {
        if local.len() != self.parents.len() {
            return Err(AnimationError::PoseBoneCountMismatch {
                expected: self.parents.len(),
                actual: local.len(),
            });
        }
        let mut global = Vec::with_capacity(local.len());
        for (bone, (&parent, &local_matrix)) in self.parents.iter().zip(local).enumerate() {
            let parent_is_trajectory =
                usize::try_from(parent).is_ok_and(|parent| self.trajectory_bone == Some(parent));
            if parent == -1 || parent_is_trajectory {
                global.push(local_matrix);
                continue;
            }
            let parent_index = usize::try_from(parent)
                .ok()
                .filter(|parent| *parent < bone)
                .ok_or(AnimationError::InvalidParent { bone, parent })?;
            global.push(compose_pose_matrix(global[parent_index], local_matrix));
        }
        Ok(global)
    }

    /// Converts every bone SQT to the local matrix buffer supplied alongside
    /// the global pose to Skate 2's physics handoff.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError`] when the pose does not match the hierarchy.
    pub fn local_matrices(&self, pose: &LocalPose) -> Result<Vec<AnimationMatrix>, AnimationError> {
        if pose.bones.len() != self.parents.len() {
            return Err(AnimationError::PoseBoneCountMismatch {
                expected: self.parents.len(),
                actual: pose.bones.len(),
            });
        }
        Ok(pose
            .bones
            .iter()
            .copied()
            .map(sqt_to_local_matrix)
            .collect())
    }
}

impl LocalPose {
    #[must_use]
    pub fn from_andale(transforms: &[AndaleSqt]) -> Self {
        Self {
            bones: transforms
                .iter()
                .copied()
                .map(BoneTransform::from)
                .collect(),
        }
    }

    /// Copies every source part into the same named target part.
    ///
    /// This is the generic body-pose portion of `ReadSkater`; specialized hand,
    /// face, and procedural overlays remain separate runtime stages.
    ///
    /// # Errors
    ///
    /// Returns [`AnimationError`] for inconsistent pose sizes, a missing
    /// target part, or a named part whose source and target sizes differ.
    pub fn overlay_matching_parts(
        &mut self,
        target: &AnimationSkeleton,
        source: &AnimationSkeleton,
        source_pose: &Self,
    ) -> Result<(), AnimationError> {
        if self.bones.len() != target.parents.len() {
            return Err(AnimationError::PoseBoneCountMismatch {
                expected: target.parents.len(),
                actual: self.bones.len(),
            });
        }
        if source_pose.bones.len() != source.parents.len() {
            return Err(AnimationError::PoseBoneCountMismatch {
                expected: source.parents.len(),
                actual: source_pose.bones.len(),
            });
        }
        for source_part in &source.parts {
            let target_part = target
                .parts
                .iter()
                .find(|part| part.name.eq_ignore_ascii_case(&source_part.name))
                .ok_or_else(|| AnimationError::MissingTargetPart(source_part.name.clone()))?;
            if source_part.bones.len() != target_part.bones.len() {
                return Err(AnimationError::PartBoneCountMismatch {
                    name: source_part.name.clone(),
                    source: source_part.bones.len(),
                    target: target_part.bones.len(),
                });
            }
            self.bones[target_part.bones.clone()]
                .copy_from_slice(&source_pose.bones[source_part.bones.clone()]);
        }
        Ok(())
    }
}

/// Builds the reference column-major local matrix from scale, quaternion, and
/// translation lanes. The four contiguous vectors are x/y/z/w axes.
#[must_use]
pub fn sqt_to_local_matrix(transform: BoneTransform) -> AnimationMatrix {
    let [x, y, z, w] = transform.rotation;
    // SQTtoLocal82a22d24..dd8: rounded quaternion products precede
    // sums/differences, then doubling; diagonal subtraction is fused.
    let xx = x * x;
    let xy = x * y;
    let xz = x * z;
    let yy = y * y;
    let yz = y * z;
    let zz = z * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;
    let [scale_x, scale_y, scale_z, _] = transform.scale;
    let [translation_x, translation_y, translation_z, _] = transform.translation;
    [
        -(yy + zz).mul_add(2.0, -1.0) * scale_x,
        ((xy + wz) * 2.0) * scale_x,
        ((xz - wy) * 2.0) * scale_x,
        0.0,
        ((xy - wz) * 2.0) * scale_y,
        -(xx + zz).mul_add(2.0, -1.0) * scale_y,
        ((yz + wx) * 2.0) * scale_y,
        0.0,
        ((xz + wy) * 2.0) * scale_z,
        ((yz - wx) * 2.0) * scale_z,
        -(xx + yy).mul_add(2.0, -1.0) * scale_z,
        0.0,
        translation_x,
        translation_y,
        translation_z,
        1.0,
    ]
}

/// Pose82a23488..e8 composes affine axes with ordered fused operations.
/// Translation starts with the parent's translation as the first addend.
pub(crate) fn compose_pose_matrix(
    parent: AnimationMatrix,
    local: AnimationMatrix,
) -> AnimationMatrix {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            let first = if column == 3 {
                parent[row].mul_add(local[column * 4], parent[12 + row])
            } else {
                parent[row] * local[column * 4]
            };
            let second = parent[4 + row].mul_add(local[column * 4 + 1], first);
            result[column * 4 + row] = parent[8 + row].mul_add(local[column * 4 + 2], second);
        }
    }
    result
}

#[must_use]
pub fn multiply_matrices(left: AnimationMatrix, right: AnimationMatrix) -> AnimationMatrix {
    let mut result = [0.0_f32; 16];
    for column in 0..4 {
        for row in 0..4 {
            result[column * 4 + row] = (0..4)
                .map(|index| left[index * 4 + row] * right[column * 4 + index])
                .sum();
        }
    }
    result
}

/// Applies the hierarchy-ordered inverse bind pose after global hierarchy
/// evaluation, matching `PoseWithMatMult`.
///
/// # Errors
///
/// Returns [`AnimationError`] if the two hierarchy buffers differ in length.
pub fn apply_inverse_bind(
    global_pose: &[AnimationMatrix],
    inverse_bind_pose: &[AnimationMatrix],
) -> Result<Vec<AnimationMatrix>, AnimationError> {
    if global_pose.len() != inverse_bind_pose.len() {
        return Err(AnimationError::MatrixCountMismatch {
            global: global_pose.len(),
            inverse_bind: inverse_bind_pose.len(),
        });
    }
    Ok(global_pose
        .iter()
        .zip(inverse_bind_pose)
        .map(|(&global, &inverse_bind)| multiply_matrices(global, inverse_bind))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(x: f32, y: f32, z: f32) -> BoneTransform {
        BoneTransform {
            translation: [x, y, z, 1.0],
            ..BoneTransform::IDENTITY
        }
    }

    #[test]
    fn diagonal_negative_multiply_subtract_preserves_negative_zero() {
        let matrix = sqt_to_local_matrix(BoneTransform {
            rotation: [0.5, 0.5, 0.5, 0.5],
            ..BoneTransform::IDENTITY
        });
        for index in [0, 5, 10] {
            assert_eq!(matrix[index].to_bits(), (-0.0_f32).to_bits());
        }
    }

    #[test]
    fn pose_translation_preserves_fused_cancellation_residue() {
        let mut parent = sqt_to_local_matrix(BoneTransform::IDENTITY);
        parent[0] = 1.0 + f32::EPSILON;
        parent[12] = -1.0;
        let local = sqt_to_local_matrix(translated(1.0 - f32::EPSILON, 0.0, 0.0));
        assert_eq!(
            compose_pose_matrix(parent, local)[12].to_bits(),
            (-f32::EPSILON.powi(2)).to_bits()
        );
        assert_eq!(
            multiply_matrices(parent, local)[12].to_bits(),
            0.0_f32.to_bits()
        );
    }

    #[test]
    fn quaternion_products_precede_doubling_without_overflowing_zero_cross_terms() {
        let matrix = sqt_to_local_matrix(BoneTransform {
            rotation: [f32::MAX, 0.0, 0.0, 0.0],
            ..BoneTransform::IDENTITY
        });
        assert_eq!(matrix[1].to_bits(), 0.0_f32.to_bits());
        assert_eq!(matrix[2].to_bits(), 0.0_f32.to_bits());
        assert_eq!(matrix[6].to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn converts_quaternion_to_scaled_column_axes() {
        let half_sqrt = 0.5_f32.sqrt();
        let matrix = sqt_to_local_matrix(BoneTransform {
            rotation: [0.0, 0.0, half_sqrt, half_sqrt],
            ..BoneTransform::IDENTITY
        });
        assert!(matrix[0].abs() < 1.0e-6);
        assert!((matrix[1] - 1.0).abs() < 1.0e-6);
        assert!((matrix[4] + 1.0).abs() < 1.0e-6);
        assert!(matrix[5].abs() < 1.0e-6);
    }

    #[test]
    fn composes_parent_and_child_translation() {
        let skeleton = AnimationSkeleton {
            parents: vec![-1, 0],
            trajectory_bone: None,
            parts: Vec::new(),
        };
        let pose = LocalPose {
            bones: vec![translated(1.0, 0.0, 0.0), translated(0.0, 2.0, 0.0)],
        };
        let matrices = skeleton.global_matrices(&pose).unwrap();
        assert_eq!(&matrices[1][12..15], &[1.0, 2.0, 0.0]);
    }

    #[test]
    fn exposes_local_matrices_without_hierarchy_composition() {
        let skeleton = AnimationSkeleton {
            parents: vec![-1, 0],
            trajectory_bone: None,
            parts: Vec::new(),
        };
        let pose = LocalPose {
            bones: vec![translated(1.0, 0.0, 0.0), translated(0.0, 2.0, 0.0)],
        };
        let matrices = skeleton.local_matrices(&pose).unwrap();
        assert_eq!(&matrices[0][12..15], &[1.0, 0.0, 0.0]);
        assert_eq!(&matrices[1][12..15], &[0.0, 2.0, 0.0]);
    }

    #[test]
    fn trajectory_children_start_new_global_roots() {
        let skeleton = AnimationSkeleton {
            parents: vec![-1, 0, 1],
            trajectory_bone: Some(0),
            parts: Vec::new(),
        };
        let pose = LocalPose {
            bones: vec![
                translated(10.0, 0.0, 0.0),
                translated(1.0, 0.0, 0.0),
                translated(0.0, 2.0, 0.0),
            ],
        };
        let matrices = skeleton.global_matrices(&pose).unwrap();
        assert_eq!(&matrices[1][12..15], &[1.0, 0.0, 0.0]);
        assert_eq!(&matrices[2][12..15], &[1.0, 2.0, 0.0]);
    }

    #[test]
    fn vehicle_acs_trajectory_excludes_authored_root_scale_from_chassis() {
        // NewACS824d6340 supplies index0 even though VehiclesDB has no named
        // trajectory. PoseWithMatMult82a23950 skips that parent's transform.
        let hierarchy = AndaleHierarchy {
            has_trajectory: false,
            parents: vec![-1, 0, 1],
            bone_metadata: vec![0; 3],
            parts: vec![],
            bone_names: vec![
                "Vehicle_Root".into(),
                "Chassis".into(),
                "LeftFront_wheel".into(),
            ],
            packed_bone_names: vec![],
        };
        let mut skeleton = AnimationSkeleton::from_andale(&hierarchy).unwrap();
        skeleton.trajectory_bone = Some(0);
        let pose = LocalPose {
            bones: vec![
                BoneTransform {
                    scale: [0.01; 4],
                    ..BoneTransform::IDENTITY
                },
                BoneTransform::IDENTITY,
                translated(0.8, 0.3, 1.2),
            ],
        };
        let matrices = skeleton.global_matrices(&pose).unwrap();
        assert_eq!(matrices[0][0], 0.01); // Root itself is preserved.
        for bone in [1, 2] {
            for axis in [0, 5, 10] {
                assert_eq!(matrices[bone][axis], 1.0);
            }
        }
        assert_eq!(&matrices[2][12..15], &[0.8, 0.3, 1.2]);
    }

    #[test]
    fn overlays_matching_named_part_ranges() {
        let target = AnimationSkeleton {
            parents: vec![-1; 5],
            trajectory_bone: None,
            parts: vec![SkeletonPart {
                name: "Torso".to_owned(),
                bones: 1..3,
            }],
        };
        let source = AnimationSkeleton {
            parents: vec![-1; 2],
            trajectory_bone: None,
            parts: vec![SkeletonPart {
                name: "torso".to_owned(),
                bones: 0..2,
            }],
        };
        let mut target_pose = target.identity_pose();
        let source_pose = LocalPose {
            bones: vec![translated(1.0, 0.0, 0.0), translated(2.0, 0.0, 0.0)],
        };
        target_pose
            .overlay_matching_parts(&target, &source, &source_pose)
            .unwrap();
        assert!((target_pose.bones[1].translation[0] - 1.0).abs() < f32::EPSILON);
        assert!((target_pose.bones[2].translation[0] - 2.0).abs() < f32::EPSILON);
        assert_eq!(target_pose.bones[3], BoneTransform::IDENTITY);
    }

    #[test]
    fn applies_inverse_bind_after_global_pose() {
        let global = sqt_to_local_matrix(translated(2.0, 3.0, 4.0));
        let inverse = sqt_to_local_matrix(translated(-2.0, -3.0, -4.0));
        let skinned = apply_inverse_bind(&[global], &[inverse]).unwrap();
        let identity = sqt_to_local_matrix(BoneTransform::IDENTITY);
        assert!(
            skinned[0]
                .iter()
                .zip(identity)
                .all(|(actual, expected)| (actual - expected).abs() < f32::EPSILON)
        );
    }
}
