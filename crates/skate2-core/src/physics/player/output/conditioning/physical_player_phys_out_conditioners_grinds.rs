//! Grinds conditioner82e2d2c8; real thirty-entry approach history and chromosomes.
use super::super::{FORWARD, RIGHT, SkeletonRootVector, ZERO, grinds::GrindChromosome};
use crate::physics::GrindType;

/// `GrindApproachReckoning`82b38f60. Left foot position negates every lane of K,
/// including its zero lanes (raw VMX128 source A is v63, not displayed v95).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Approach {
    pub(super) deck_side: SkeletonRootVector,
    pub(super) position: SkeletonRootVector,
    pub(super) velocity: SkeletonRootVector,
    pub(super) right_foot_position: SkeletonRootVector,
    pub(super) left_foot_position: SkeletonRootVector,
    pub(super) right_foot_direction: SkeletonRootVector,
    pub(super) left_foot_direction: SkeletonRootVector,
}
impl Approach {
    pub(super) fn new() -> Self {
        Self {
            deck_side: RIGHT,
            position: ZERO,
            velocity: ZERO,
            right_foot_position: FORWARD,
            left_foot_position: SkeletonRootVector::known([-0.0, -0.0, -1.0, -0.0]),
            right_foot_direction: RIGHT,
            left_foot_direction: RIGHT,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TimedApproach {
    pub(super) value: Approach,
    pub(super) tick: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Grinds {
    pub(super) last_physics_category: i32,
    pub(super) last_grind_type: GrindType,
    pub(super) approach: i32,
    pub(super) location: i32,
    pub(super) last_ollie_approach: Approach,
    pub(super) stable_chromosome: GrindChromosome,
    pub(super) scoring_stable_chromosome: GrindChromosome,
    pub(super) candidate_chromosome: GrindChromosome,
    pub(super) consecutive_non_grind_frames: i32,
    pub(super) frames_of_chromosome_equality: i32,
    pub(super) updates: i32,
    pub(super) grind_name: [u8; 256],
    pub(super) scoring_grind_name: [u8; 256],
    pub(super) approach_ground_history: [TimedApproach; 30],
    pub(super) valid_history_entries: i32,
}
impl Grinds {
    /// Reset82e2afa8 prefix; the linked cvar read occurs after these stores.
    pub(super) fn reset_prefix(&mut self) {
        self.grind_name[0] = 0;
        self.last_physics_category = 0;
        self.last_grind_type = GrindType::None;
        self.approach = 0;
        self.location = 0;
    }
    /// Allocate and retain this object before the late linked-cvar read. Zeros
    /// not explicitly stored by the prefix come from the original Sim category.
    pub(super) fn before_cvar() -> Self {
        Self {
            last_physics_category: 0,
            last_grind_type: GrindType::None,
            approach: 0,
            location: 0,
            last_ollie_approach: Approach::new(),
            stable_chromosome: GrindChromosome::reset_values(),
            scoring_stable_chromosome: GrindChromosome::reset_values(),
            candidate_chromosome: GrindChromosome::reset_values(),
            consecutive_non_grind_frames: 0,
            frames_of_chromosome_equality: 0,
            updates: 0,
            grind_name: [0; 256],
            scoring_grind_name: [0; 256],
            approach_ground_history: std::array::from_fn(|_| TimedApproach {
                value: Approach::new(),
                tick: 0,
            }),
            valid_history_entries: 0,
        }
    }
    pub(super) fn finish_constructor(&mut self, minimum_non_grind_frames: i32) {
        self.frames_of_chromosome_equality = 0;
        self.valid_history_entries = 0;
        self.updates = 0;
        self.consecutive_non_grind_frames = minimum_non_grind_frames.wrapping_add(1);
    }
}
