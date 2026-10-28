//! Retained animation record from `SkeletonState::UpdateAnimRecord` (0x82b50e40).
//! Masses remain owned by the same assembly's existing physics history. This
//! kernel receives adjusted volume matrices after the original adjustment stages.
//! Host f32 FMA arithmetic does not establish Xenon exceptional-value/FP status.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "animation record consumers await full Skeleton lifecycle"
    )
)]

use crate::{
    animation::AnimationMatrix,
    ragdoll::SKELETON_PART_COUNT,
    skateboard_wheels::FRESH_VOLUME_TRANSFORM,
    skeleton_physics_history::SkeletonPhysicsHistory,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};

pub(crate) trait SkeletonAnimationRecordInput {
    type Error;
    /// Sample current `kUseXYZCentreOfMass` after all 24 weighted positions.
    fn use_xyz_centre_of_mass(&mut self) -> Result<u8, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SkeletonAnimationRecordError<E> {
    InvalidTotalMass,
    Cvar(E),
}
impl<E: std::fmt::Display> std::fmt::Display for SkeletonAnimationRecordError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTotalMass => f.write_str("Skeleton animation total mass is not positive"),
            Self::Cvar(error) => write!(f, "Skeleton animation COM cvar: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for SkeletonAnimationRecordError<E> {}

const ZERO: SkeletonRootVector = SkeletonRootVector {
    xyz: [0.0; 3],
    w: Some(0.0),
};

/// One non-Clone projection owned alongside the actual physics history and roots.
pub(crate) struct SkeletonAnimationRecord {
    poses: [AnimationMatrix; SKELETON_PART_COUNT],
    centre_of_mass: [f32; 4],
    centre_of_mass_delta: [f32; 4],
    com_to_deck: SkeletonRootVector,
    com_to_deck_delta: SkeletonRootVector,
    reset_scalar: f32,
}
impl SkeletonAnimationRecord {
    /// `AnimRecord` ctor 0x82b4b870 and `SkeletonState` ctor 0x82b50d08.
    pub(crate) fn new() -> Self {
        Self {
            poses: [FRESH_VOLUME_TRANSFORM; SKELETON_PART_COUNT],
            centre_of_mass: [0.0; 4],
            centre_of_mass_delta: [0.0; 4],
            com_to_deck: ZERO,
            com_to_deck_delta: ZERO,
            reset_scalar: 0.0,
        }
    }

    pub(crate) fn poses(&self) -> &[AnimationMatrix; SKELETON_PART_COUNT] {
        &self.poses
    }
    pub(crate) fn centre_of_mass(&self) -> [f32; 4] {
        self.centre_of_mass
    }
    pub(crate) fn centre_of_mass_delta(&self) -> [f32; 4] {
        self.centre_of_mass_delta
    }
    pub(crate) fn com_to_deck(&self) -> SkeletonRootVector {
        self.com_to_deck
    }
    pub(crate) fn com_to_deck_delta(&self) -> SkeletonRootVector {
        self.com_to_deck_delta
    }
    pub(crate) fn reset_scalar(&self) -> f32 {
        self.reset_scalar
    }

    /// 0x82b50e40..0x82b50f40. Copy precedes the mass assertion; a failed cvar
    /// retains that copy but none of the COM writes. No vector validity checks
    /// or dt division occur here. Separate transformed points preserve original
    /// rounding and cannot be replaced by transforming their difference.
    pub(crate) fn try_update<I: SkeletonAnimationRecordInput>(
        &mut self,
        adjusted: &[AnimationMatrix; SKELETON_PART_COUNT],
        masses: &SkeletonPhysicsHistory,
        anim_to_board: &SkeletonRootMatrix,
        input: &mut I,
    ) -> Result<(), SkeletonAnimationRecordError<I::Error>> {
        self.poses = *adjusted;
        // CalculateCentreOfMassAnim 0x82b48550, ordered >0 also rejects NaN.
        if masses.total_mass().partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(SkeletonAnimationRecordError::InvalidTotalMass);
        }
        let mut centre = [0.0; 4];
        for (pose, &weight) in self.poses.iter().zip(masses.fractional_part_masses()) {
            for lane in 0..4 {
                centre[lane] = pose[12 + lane].mul_add(weight, centre[lane]);
            }
        }
        if input
            .use_xyz_centre_of_mass()
            .map_err(SkeletonAnimationRecordError::Cvar)?
            == 0
        {
            centre = [
                self.poses[0][12],
                centre[1],
                self.poses[0][14],
                self.poses[0][15],
            ];
        }
        self.centre_of_mass_delta =
            std::array::from_fn(|lane| centre[lane] - self.centre_of_mass[lane]);
        self.centre_of_mass = centre;

        let columns = anim_to_board.columns();
        let world_com = transform_point(columns, centre);
        let deck = std::array::from_fn(|lane| self.poses[0][12 + lane]);
        let world_deck = transform_point(columns, deck);
        let next = subtract(world_com, world_deck);
        self.com_to_deck_delta = subtract(next, self.com_to_deck);
        self.com_to_deck = next;
        self.com_to_deck_delta.xyz = self.com_to_deck_delta.xyz.map(|v| v * self.reset_scalar);
        self.com_to_deck_delta.w = self.com_to_deck_delta.w.map(|v| v * self.reset_scalar);
        self.reset_scalar = 1.0;
        Ok(())
    }

    /// Animation fields from Reset's suffix 0x82b3d3c8..0x82b3d3e4, only after
    /// the actual physics-record reset succeeds. Pose matrices and `ResetScalar`
    /// persist. This is not `TeleportReset` or the full live physics reset.
    pub(crate) fn reset_after_physics_reset(
        &mut self,
        proxy_target: &mut Option<SkeletonRootVector>,
    ) {
        self.com_to_deck_delta = ZERO;
        self.com_to_deck = ZERO;
        *proxy_target = Some(SkeletonRootVector::known([0.0; 4]));
        self.centre_of_mass_delta = [0.0; 4];
        self.centre_of_mass = [0.0; 4];
    }
}

fn transform_point(columns: &[SkeletonRootVector; 4], point: [f32; 4]) -> SkeletonRootVector {
    let coordinate = |lane: usize| {
        let x = columns[0].xyz[lane].mul_add(point[0], columns[3].xyz[lane]);
        let y = columns[1].xyz[lane].mul_add(point[1], x);
        columns[2].xyz[lane].mul_add(point[2], y)
    };
    // Point W is never read. Unknown outer W remains unknown even when the
    // coefficient or eventual ResetScalar is zero, or both points are equal.
    let w = columns[0]
        .w
        .zip(columns[1].w)
        .zip(columns[2].w)
        .zip(columns[3].w)
        .map(|(((x, y), z), p)| z.mul_add(point[2], y.mul_add(point[1], x.mul_add(point[0], p))));
    SkeletonRootVector {
        xyz: std::array::from_fn(coordinate),
        w,
    }
}

fn subtract(left: SkeletonRootVector, right: SkeletonRootVector) -> SkeletonRootVector {
    SkeletonRootVector {
        xyz: std::array::from_fn(|lane| left.xyz[lane] - right.xyz[lane]),
        w: left.w.zip(right.w).map(|(left, right)| left - right),
    }
}

#[cfg(test)]
#[path = "skeleton_animation_record_tests.rs"]
mod tests;
