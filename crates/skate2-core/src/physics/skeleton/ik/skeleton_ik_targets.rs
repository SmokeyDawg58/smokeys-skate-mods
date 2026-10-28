//! Retained target preparation from `SkeletonIK::Init` (0x82b5eb80),
//! `CalculateTargetTransforms` (0x82b4a140), and `GetTargetVolumeMtx`
//! (0x82b3de60). This is the target stage of `ProcessData`, not the IK solve.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "target consumers await full Skeleton IK lifecycle"
    )
)]
//! Reuses the audited host affine graph; Xenon exceptional-result bits and
//! floating-point status remain unproven. Matrix commits occur under one
//! exclusive borrow, without modeling concurrent native per-column observers.
use crate::{
    animation::AnimationMatrix, ragdoll::SkeletonPartIndex as Part, skeleton_data::SkeletonData,
};
use skate2_formats::andale::PackedFastString;
use std::sync::Arc;

const TARGET_NAMES: [&str; 4] = [
    "LeftToeBase_Reparented",
    "RightToeBase_Reparented",
    "LeftHand_Reparented",
    "RightHand_Reparented",
];
const PARTS: [Part; 4] = [
    Part::LeftToeBase,
    Part::RightToeBase,
    Part::LeftHand,
    Part::RightHand,
];
const RESET_TRANSFORM: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonIkTargetError {
    NotInitialized,
    PoseUnavailable {
        target: usize,
        /// None represents the original missing-name result, signed -1.
        bone: Option<usize>,
        pose_count: usize,
    },
    ActualHandPoseUnavailable {
        target: usize,
        bone: Option<usize>,
        pose_count: usize,
    },
    InvalidPart {
        target: usize,
    },
}

impl std::fmt::Display for SkeletonIkTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInitialized => {
                f.write_str("Skeleton IK target mapping has not been initialized")
            }
            Self::PoseUnavailable {
                target,
                bone,
                pose_count,
            } => write!(
                f,
                "Skeleton IK target {target} bone {bone:?} is unavailable in {pose_count} pose matrices"
            ),
            Self::ActualHandPoseUnavailable {
                target,
                bone,
                pose_count,
            } => write!(
                f,
                "Skeleton IK hand {target} actual bone {bone:?} is unavailable in {pose_count} pose matrices"
            ),
            Self::InvalidPart { target } => {
                write!(f, "Skeleton IK target {target} has an invalid Part")
            }
        }
    }
}
impl std::error::Error for SkeletonIkTargetError {}

/// Owns the actual retained data authority; current raw pose is borrowed for
/// each calculation. Missing named bones survive Init as source -1 results.
/// The initialized flag is host sequencing evidence for otherwise unknown
/// pre-Init IDs, not a native IK field. No alternate animation-volume owner.
pub(crate) struct SkeletonIkTargets {
    data: Option<Arc<SkeletonData>>,
    bone_ids: [Option<usize>; 4],
    target_bone_ids: [Option<usize>; 4],
    parts: [Option<Part>; 4],
    parents: [Option<Part>; 4],
    targets: [AnimationMatrix; 4],
    initialized: bool,
}

impl SkeletonIkTargets {
    /// Only the target projection of constructor/Reset is represented here.
    /// Reset 0x82b5146c..0x82b514c8 stores I/J/K/zero P, including all W zero.
    pub(crate) fn new(data: Arc<SkeletonData>) -> Self {
        let mut value = Self::before_skeleton_data_init();
        value.data = Some(data);
        value
    }

    /// `SkeletonIK` construction precedes `SkeletonData.Init`. The actual storage
    /// exists, but no initialized mapping may be fabricated for constructor use.
    pub(crate) fn before_skeleton_data_init() -> Self {
        Self {
            data: None,
            bone_ids: [None; 4],
            target_bone_ids: [None; 4],
            parts: [None; 4],
            parents: [None; 4],
            targets: [RESET_TRANSFORM; 4],
            initialized: false,
        }
    }

    pub(crate) fn bind_initialized_data(&mut self, data: Arc<SkeletonData>) {
        assert!(
            self.data.is_none(),
            "same SkeletonData can only initialize once"
        );
        self.data = Some(data);
    }

    /// `SkeletonIK::Init` 0x82b5eb80. All four actual bone IDs precede the
    /// ordered five-word `GetBone` lookups. `GetBone` returns -1 when absent;
    /// there is no assertion or early return here, including after a miss.
    pub(crate) fn initialize(&mut self) {
        let data = self
            .data
            .as_ref()
            .expect("actual SkeletonData.Init precedes IK.Init");
        for (target, part) in PARTS.into_iter().enumerate() {
            self.bone_ids[target] = Some(data.bone_ids()[part.index()]);
        }
        for (target, name) in TARGET_NAMES.into_iter().enumerate() {
            let packed = PackedFastString::<5>::encode(name).expect("original fixed IK bone name");
            self.target_bone_ids[target] = data
                .hierarchy()
                .packed_bone_names
                .iter()
                .position(|candidate| *candidate == packed);
        }
        // Exact Init suffix order, 0x82b5ec90..0x82b5ecbc.
        self.parents[2] = None;
        self.parts[0] = Some(Part::LeftToeBase);
        self.parts[1] = Some(Part::RightToeBase);
        self.parts[2] = Some(Part::LeftHand);
        self.parts[3] = Some(Part::RightHand);
        self.parents[0] = Some(Part::LeftFoot);
        self.parents[1] = Some(Part::RightFoot);
        self.parents[3] = None;
        self.initialized = true;
    }

    /// Source 0x82b4a174..0x82b4a1b8 visits four entries ascending. Each
    /// completed matrix is committed before the next target read. Typed host
    /// stops replace invalid native reads/asserts and retain the older suffix.
    pub(crate) fn try_calculate(
        &mut self,
        pose: &[AnimationMatrix],
    ) -> Result<(), SkeletonIkTargetError> {
        if !self.initialized {
            return Err(SkeletonIkTargetError::NotInitialized);
        }
        for target in 0..4 {
            self.targets[target] = self.target_volume_transform(pose, target)?;
        }
        Ok(())
    }

    fn target_volume_transform(
        &self,
        pose: &[AnimationMatrix],
        target: usize,
    ) -> Result<AnimationMatrix, SkeletonIkTargetError> {
        let bone = self.target_bone_ids[target];
        // Raw 0x82b3de94..0x82b3dea4 loads ALL pose columns before the
        // invalid-Part assertion at 0x82b3dea8. Do not preflight all targets.
        let matrix = *bone.and_then(|index| pose.get(index)).ok_or(
            SkeletonIkTargetError::PoseUnavailable {
                target,
                bone,
                pose_count: pose.len(),
            },
        )?;
        let part = self.parts[target].ok_or(SkeletonIkTargetError::InvalidPart { target })?;
        // Corrected VMX128 VA operands prove X multiply (translation X FMA
        // into outer P), Y FMA, Z FMA, all four lanes. Inner W is unused.
        Ok(crate::skateboard_spawn_math::skateboard_spawn_compose(
            &matrix,
            &self.data().bone_to_volume()[part.index()],
        ))
    }

    /// Target matrices only: the 0x82b51488..0x82b514c8 slice of Reset.
    /// Leaves Init mappings intact; does not claim the remainder of IK Reset.
    pub(crate) fn reset_target_transforms(&mut self) {
        self.targets.fill(RESET_TRANSFORM);
    }

    /// `SkeletonIK::AddOffsetTransform`0x82b3df68..0x82b3e030. Compose the
    /// supplied outer offset with targets0..3 ascending. No bone lookup,
    /// initialization gate, normalization, cvar or assertion occurs here.
    pub(crate) fn add_offset_transform(&mut self, offset: &AnimationMatrix) {
        for target in &mut self.targets {
            let adjusted = crate::skateboard_spawn_math::skateboard_spawn_compose(offset, target);
            // Native stores Z before X/Y/P for each complete target. The
            // exclusive borrow excludes concurrent per-column observations.
            target[8..12].copy_from_slice(&adjusted[8..12]);
            target[..4].copy_from_slice(&adjusted[..4]);
            target[4..8].copy_from_slice(&adjusted[4..8]);
            target[12..16].copy_from_slice(&adjusted[12..16]);
        }
    }

    /// `GetIKHandToAnimHandMapping`0x82b3dd48..0x82b3de5c. Mirrored selects
    /// left target2; otherwise right target3. Raw target pose precedes raw
    /// actual-hand pose. Neither current IK targets nor bone-to-volume frames
    /// enter this operation; it does not normalize the result.
    pub(crate) fn try_hand_to_animation_mapping(
        &self,
        pose: &[AnimationMatrix],
        mirrored: bool,
    ) -> Result<AnimationMatrix, SkeletonIkTargetError> {
        if !self.initialized {
            // Native reads otherwise unknown pre-Init IDs; this is the same
            // explicit host sequencing boundary as current target preparation.
            return Err(SkeletonIkTargetError::NotInitialized);
        }
        let target = if mirrored { 2 } else { 3 };
        let target_bone = self.target_bone_ids[target];
        let target_pose = *target_bone.and_then(|bone| pose.get(bone)).ok_or(
            SkeletonIkTargetError::PoseUnavailable {
                target,
                bone: target_bone,
                pose_count: pose.len(),
            },
        )?;
        let actual_bone = self.bone_ids[target];
        let actual_pose = *actual_bone.and_then(|bone| pose.get(bone)).ok_or(
            SkeletonIkTargetError::ActualHandPoseUnavailable {
                target,
                bone: actual_bone,
                pose_count: pose.len(),
            },
        )?;
        // Native inverse transposes target XYZ, inserts basis W0, and uses
        // arithmetic zero-P with Z product/Y FMA/X FMA. Actual hand is outer.
        // Native result stores Z/Y/X/P; no fallible operation follows reads,
        // so the completed scratch result is returned under one borrow.
        let inverse = crate::skateboard_spawn_math::skateboard_spawn_rigid_inverse(&target_pose);
        Ok(crate::skateboard_spawn_math::skateboard_spawn_compose(
            &actual_pose,
            &inverse,
        ))
    }

    pub(crate) fn data(&self) -> &Arc<SkeletonData> {
        self.data
            .as_ref()
            .expect("initialized target consumers require SkeletonData.Init")
    }

    pub(crate) fn bone_ids(&self) -> &[Option<usize>; 4] {
        &self.bone_ids
    }
    pub(crate) const fn parts(&self) -> &[Option<Part>; 4] {
        &self.parts
    }
    pub(crate) const fn parents(&self) -> &[Option<Part>; 4] {
        &self.parents
    }
    pub(crate) const fn initialized(&self) -> bool {
        self.initialized
    }
    pub(crate) const fn target_bone_ids(&self) -> &[Option<usize>; 4] {
        &self.target_bone_ids
    }
    pub(crate) const fn targets(&self) -> &[AnimationMatrix; 4] {
        &self.targets
    }
}

#[cfg(test)]
#[path = "skeleton_ik_targets_tests.rs"]
mod tests;

impl SkeletonIkTargets {
    pub(crate) fn add_wipeout_height(&mut self, value: f32) {
        for target in &mut self.targets {
            target[13] += value;
        }
    }
}
