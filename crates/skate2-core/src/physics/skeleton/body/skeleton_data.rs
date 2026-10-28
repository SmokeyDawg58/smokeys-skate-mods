//! Retained `SkeletonData::Init` (0x82b56778) and physics-ancestor lookup
//! (0x82b566e8), before any `SkeletonBody`, joint, or pool admission.
//!
//! The original stores borrowed hierarchy/PhysicsParam pointers and copies the
//! supplied pose. Arc ownership makes those inputs immutable and keeps the
//! actual authored bone records alive; it does not model concurrent alias edits.
//! Checked host preflight prevents fixed-array overflow and invalid pointer
//! dereferences. In particular, exact24 is checked here BEFORE mapping, whereas
//! native `Skeleton::Init` asserts it only AFTER this routine and other work.
//! Unwritten root-joint/child-tail slots are represented by absent semantic
//! entries, not invented native default words. `RootPartIdx` is not produced here.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "physical skeleton awaits actual body construction"
    )
)]

use crate::{
    animation::AnimationMatrix,
    ragdoll::{SKELETON_PART_COUNT, SkeletonPartIndex},
    simulation_drive_jacobian::drive_basis_host,
    world::inverse_collision_affine_reset,
};
use skate2_formats::andale::{AndaleHierarchy, AndalePhysicsParameters};
use std::{fmt, sync::Arc};

const ANIMATION_CAPACITY: usize = 100;
const JOINT_CAPACITY: usize = 22;
const CHILD_CAPACITY: usize = 3;

/// The original `PoseBufferExt` has its own hierarchy pointer. It need not be
/// pointer-identical to the mapping hierarchy: only the requested index and
/// matrix storage are checked by the source final-copy path.
#[derive(Clone, Copy)]
pub(crate) struct SkeletonPoseView<'a> {
    pub(crate) hierarchy: &'a AndaleHierarchy,
    pub(crate) matrices: &'a [AnimationMatrix],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SkeletonJointMapping {
    pub(crate) parent: SkeletonPartIndex,
    pub(crate) child: SkeletonPartIndex,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SkeletonDataError {
    Allocation,
    PhysicsBoneCount {
        actual: usize,
    },
    AnimationBoneCapacity {
        actual: usize,
    },
    HierarchyTableLengths {
        names: usize,
        packed_names: usize,
        parents: usize,
    },
    MissingBone {
        part: SkeletonPartIndex,
    },
    InvalidAncestor {
        part: SkeletonPartIndex,
        bone: i32,
    },
    AncestorCycle {
        part: SkeletonPartIndex,
    },
    ParentPartUnavailable {
        part: SkeletonPartIndex,
        bone: usize,
    },
    JointCapacity {
        child: SkeletonPartIndex,
    },
    ChildCapacity {
        parent: SkeletonPartIndex,
        child: SkeletonPartIndex,
    },
    PoseBoneOutsideHierarchy {
        part: SkeletonPartIndex,
        bone: usize,
        count: usize,
    },
    PoseMatrixUnavailable {
        part: SkeletonPartIndex,
        bone: usize,
        count: usize,
    },
}

impl fmt::Display for SkeletonDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Allocation => f.write_str("physics skeleton host storage allocation failed"),
            Self::PhysicsBoneCount { actual } => write!(
                f,
                "physics skeleton requires 24 authored bones, found {actual}"
            ),
            Self::AnimationBoneCapacity { actual } => write!(
                f,
                "physics skeleton animation capacity 100 exceeded by {actual}"
            ),
            Self::HierarchyTableLengths {
                names,
                packed_names,
                parents,
            } => write!(
                f,
                "physics skeleton hierarchy table lengths differ: {names} names, {packed_names} packed names, {parents} parents"
            ),
            Self::MissingBone { part } => {
                write!(f, "physics Part {part:?} has no matching hierarchy bone")
            }
            Self::InvalidAncestor { part, bone } => write!(
                f,
                "physics Part {part:?} traversed invalid animation ancestor {bone}"
            ),
            Self::AncestorCycle { part } => write!(
                f,
                "physics Part {part:?} ancestor lookup does not terminate"
            ),
            Self::ParentPartUnavailable { part, bone } => write!(
                f,
                "physics Part {part:?} matched ancestor {bone} without a mapped Part"
            ),
            Self::JointCapacity { child } => {
                write!(f, "physics joint capacity 22 exceeded at child {child:?}")
            }
            Self::ChildCapacity { parent, child } => write!(
                f,
                "physics Part {parent:?} exceeds three children at {child:?}"
            ),
            Self::PoseBoneOutsideHierarchy { part, bone, count } => write!(
                f,
                "physics Part {part:?} pose index {bone} exceeds pose hierarchy count {count}"
            ),
            Self::PoseMatrixUnavailable { part, bone, count } => write!(
                f,
                "physics Part {part:?} pose matrix {bone} missing from {count} matrices"
            ),
        }
    }
}

impl std::error::Error for SkeletonDataError {}

/// Source initialized semantic fields only; no synthetic Body or generic
/// humanoid geometry is hidden behind this retained authored-data owner.
#[derive(Debug)]
pub(crate) struct SkeletonData {
    hierarchy: Arc<AndaleHierarchy>,
    parameters: Arc<AndalePhysicsParameters>,
    bone_ids: [usize; SKELETON_PART_COUNT],
    part_from_bone: Vec<Option<SkeletonPartIndex>>,
    parents: [Option<SkeletonPartIndex>; SKELETON_PART_COUNT],
    part_joint_indices: [Option<usize>; SKELETON_PART_COUNT],
    joints: Vec<SkeletonJointMapping>,
    children: [Vec<SkeletonPartIndex>; SKELETON_PART_COUNT],
    bone_to_volume: [AnimationMatrix; SKELETON_PART_COUNT],
    volume_to_bone: [AnimationMatrix; SKELETON_PART_COUNT],
    animation_parents: Vec<i32>,
    physics_pose: [AnimationMatrix; SKELETON_PART_COUNT],
}

impl SkeletonData {
    /// Source ordered passes: reverse-map initialization; all named Parts;
    /// all ancestor/joint mappings; all children; per-Part matrix and inverse;
    /// copied animation parents; then the supplied full-lane physics poses.
    #[expect(
        clippy::too_many_lines,
        reason = "keeps the independently audited complete source passes in order"
    )]
    pub(crate) fn try_new(
        hierarchy: Arc<AndaleHierarchy>,
        parameters: Arc<AndalePhysicsParameters>,
        pose: SkeletonPoseView<'_>,
    ) -> Result<Self, SkeletonDataError> {
        let count = hierarchy.bone_names.len();
        if parameters.bones.len() != SKELETON_PART_COUNT {
            return Err(SkeletonDataError::PhysicsBoneCount {
                actual: parameters.bones.len(),
            });
        }
        if count > ANIMATION_CAPACITY {
            return Err(SkeletonDataError::AnimationBoneCapacity { actual: count });
        }
        if hierarchy.packed_bone_names.len() != count || hierarchy.parents.len() != count {
            return Err(SkeletonDataError::HierarchyTableLengths {
                names: count,
                packed_names: hierarchy.packed_bone_names.len(),
                parents: hierarchy.parents.len(),
            });
        }
        // Host storage is allocated before source work. These capacities do
        // not initialize any otherwise unwritten native output slots.
        let mut part_from_bone = Vec::new();
        part_from_bone
            .try_reserve_exact(count)
            .map_err(|_| SkeletonDataError::Allocation)?;
        part_from_bone.resize(count, None);
        let mut joints = Vec::new();
        joints
            .try_reserve_exact(JOINT_CAPACITY)
            .map_err(|_| SkeletonDataError::Allocation)?;
        let mut children: [Vec<SkeletonPartIndex>; SKELETON_PART_COUNT] =
            std::array::from_fn(|_| Vec::new());
        for entries in &mut children {
            entries
                .try_reserve_exact(CHILD_CAPACITY)
                .map_err(|_| SkeletonDataError::Allocation)?;
        }
        let mut animation_parents = Vec::new();
        animation_parents
            .try_reserve_exact(count)
            .map_err(|_| SkeletonDataError::Allocation)?;

        let mut bone_ids = [0; SKELETON_PART_COUNT];
        let mut parents = [None; SKELETON_PART_COUNT];
        let mut part_joint_indices = [None; SKELETON_PART_COUNT];
        for part in SkeletonPartIndex::ALL {
            let bone = &parameters.bones[part.index()];
            // GetBone 0x82590fc0: first complete five-word match. In particular,
            // duplicate physics names overwrite the reverse map in source order.
            let bone_id = hierarchy
                .packed_bone_names
                .iter()
                .position(|name| *name == bone.packed_name)
                .ok_or(SkeletonDataError::MissingBone { part })?;
            bone_ids[part.index()] = bone_id;
            part_from_bone[bone_id] = Some(part);
            parents[part.index()] = None;
        }

        for part in SkeletonPartIndex::ALL {
            if let Some(parent_bone) =
                physics_parent(&hierarchy, &parameters, bone_ids[part.index()], part)?
            {
                let parent = part_from_bone[parent_bone].ok_or(
                    SkeletonDataError::ParentPartUnavailable {
                        part,
                        bone: parent_bone,
                    },
                )?;
                if joints.len() == JOINT_CAPACITY {
                    return Err(SkeletonDataError::JointCapacity { child: part });
                }
                part_joint_indices[part.index()] = Some(joints.len());
                parents[part.index()] = Some(parent);
                joints.push(SkeletonJointMapping {
                    parent,
                    child: part,
                });
            }
        }

        for parent in SkeletonPartIndex::ALL {
            for child in SkeletonPartIndex::ALL {
                if parents[child.index()] == Some(parent) {
                    let entries = &mut children[parent.index()];
                    if entries.len() == CHILD_CAPACITY {
                        return Err(SkeletonDataError::ChildCapacity { parent, child });
                    }
                    entries.push(child);
                }
            }
        }

        let mut bone_to_volume = [[0.0; 16]; SKELETON_PART_COUNT];
        let mut volume_to_bone = [[0.0; 16]; SKELETON_PART_COUNT];
        for part in SkeletonPartIndex::ALL {
            let bone = &parameters.bones[part.index()];
            // Native 0x82b56aa8..6b58 shares the audited sqrt2/FMA graph with
            // Drive Build, including X/Y/Z W = diagonal.x/difference.x/sum.x.
            // Shared host multiplication operand ordering is not a promise of
            // native NaN payload selection. No normalization or W repair.
            let axes = drive_basis_host(bone.orientation);
            let matrix = &mut bone_to_volume[part.index()];
            matrix[12..16].copy_from_slice(&bone.position);
            matrix[8..12].copy_from_slice(&axes[2]);
            matrix[4..8].copy_from_slice(&axes[1]);
            matrix[0..4].copy_from_slice(&axes[0]);
            // 0x82b56b5c..6c10 is the same cofactor/general-affine-inverse
            // graph as ClusteredMeshData::Reset, not a rigid transpose. Retain
            // its explicit host dot3/vrefp seed and exceptional-value limits.
            volume_to_bone[part.index()] = inverse_collision_affine_reset(*matrix);
        }

        animation_parents.extend_from_slice(&hierarchy.parents);
        let mut physics_pose = [[0.0; 16]; SKELETON_PART_COUNT];
        for part in SkeletonPartIndex::ALL {
            let bone = bone_ids[part.index()];
            let pose_bone_count = pose.hierarchy.bone_names.len();
            if bone >= pose_bone_count {
                return Err(SkeletonDataError::PoseBoneOutsideHierarchy {
                    part,
                    bone,
                    count: pose_bone_count,
                });
            }
            let matrix =
                pose.matrices
                    .get(bone)
                    .ok_or(SkeletonDataError::PoseMatrixUnavailable {
                        part,
                        bone,
                        count: pose.matrices.len(),
                    })?;
            physics_pose[part.index()] = *matrix;
        }

        Ok(Self {
            hierarchy,
            parameters,
            bone_ids,
            part_from_bone,
            parents,
            part_joint_indices,
            joints,
            children,
            bone_to_volume,
            volume_to_bone,
            animation_parents,
            physics_pose,
        })
    }

    pub(crate) fn hierarchy(&self) -> &Arc<AndaleHierarchy> {
        &self.hierarchy
    }
    pub(crate) fn parameters(&self) -> &Arc<AndalePhysicsParameters> {
        &self.parameters
    }
    pub(crate) const fn bone_ids(&self) -> &[usize; SKELETON_PART_COUNT] {
        &self.bone_ids
    }
    pub(crate) fn part_from_bone(&self) -> &[Option<SkeletonPartIndex>] {
        &self.part_from_bone
    }
    pub(crate) const fn parents(&self) -> &[Option<SkeletonPartIndex>; SKELETON_PART_COUNT] {
        &self.parents
    }
    pub(crate) const fn part_joint_indices(&self) -> &[Option<usize>; SKELETON_PART_COUNT] {
        &self.part_joint_indices
    }
    pub(crate) fn joints(&self) -> &[SkeletonJointMapping] {
        &self.joints
    }
    pub(crate) fn children(&self, part: SkeletonPartIndex) -> &[SkeletonPartIndex] {
        &self.children[part.index()]
    }
    pub(crate) const fn bone_to_volume(&self) -> &[AnimationMatrix; SKELETON_PART_COUNT] {
        &self.bone_to_volume
    }
    pub(crate) const fn volume_to_bone(&self) -> &[AnimationMatrix; SKELETON_PART_COUNT] {
        &self.volume_to_bone
    }
    pub(crate) fn animation_parents(&self) -> &[i32] {
        &self.animation_parents
    }
    pub(crate) const fn physics_pose(&self) -> &[AnimationMatrix; SKELETON_PART_COUNT] {
        &self.physics_pose
    }
}

/// `GetParentID` 0x82b566e8: test each animation ancestor's name against all
/// `PhysicsParam` names, returning the first matching ancestor, not necessarily
/// the immediate parent. Cycle detection only refuses a nonterminating walk;
/// it does not impose a new global uniqueness or tree-shape requirement.
fn physics_parent(
    hierarchy: &AndaleHierarchy,
    parameters: &AndalePhysicsParameters,
    bone: usize,
    part: SkeletonPartIndex,
) -> Result<Option<usize>, SkeletonDataError> {
    let mut ancestor = hierarchy.parents[bone];
    let mut visited = [false; ANIMATION_CAPACITY];
    while ancestor != -1 {
        let index = usize::try_from(ancestor)
            .ok()
            .filter(|&index| index < hierarchy.parents.len())
            .ok_or(SkeletonDataError::InvalidAncestor {
                part,
                bone: ancestor,
            })?;
        if visited[index] {
            return Err(SkeletonDataError::AncestorCycle { part });
        }
        visited[index] = true;
        let name = hierarchy.packed_bone_names[index];
        if parameters
            .bones
            .iter()
            .any(|candidate| candidate.packed_name == name)
        {
            return Ok(Some(index));
        }
        ancestor = hierarchy.parents[index];
    }
    Ok(None)
}
