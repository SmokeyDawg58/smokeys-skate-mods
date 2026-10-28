//! Actor-owned `AnimOutPhysIn` construction825fd8d8 and Reset825ecbc0.
//! The caller retains this allocation before constructor
//! work, publish Actor+204 only after return, and consume actual animation
//! outputs before a physical frame. No pose identity is inferred from Reset.
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    actor_graph_construction::ActorGraphIntents, animation::AnimationMatrix,
    physical_player_phys_out::trick::AnimOutTrickData, play_animation::FastString30,
    skeleton_root_transforms::SkeletonRootMatrix,
};

#[path = "../animation/runtime/actor_animation_physics_publication.rs"]
pub(crate) mod publication;

#[path = "../physics/player/wipeout/physical_player_post_wipeout_pose_owner.rs"]
pub(crate) mod pose_lanes;

#[derive(Clone, Debug)]
pub(crate) struct PhysicsAnimationAttribute {
    pub(crate) payload: [Option<u8>; 24],
    pub(crate) begin: f32,
    pub(crate) end: f32,
    pub(crate) name: FastString30,
    pub(crate) flags: u8,
    pub(crate) kind: i8,
    pub(crate) sequence: Option<i32>,
}

#[derive(Default)]
pub(crate) struct AnimationAiInput {
    pub(crate) desired_deck_transform: AnimationMatrix,
    pub(crate) desired_deck_velocity: [f32; 4],
    pub(crate) trajectory_position: [f32; 4],
    pub(crate) trajectory_velocity: [f32; 4],
    pub(crate) trajectory_gravity: [f32; 4],
    pub(crate) trajectory_time: [f32; 4],
    pub(crate) trajectory_offset: [f32; 4],
    pub(crate) flags: u32,
}

#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent original handoff flags"
)]
pub(crate) struct AnimationPhysicsFlags {
    pub(crate) board_flipped: bool,
    pub(crate) next_step_valid: bool,
    pub(crate) mirrored: bool,
    pub(crate) riding_switch: bool,
    pub(crate) riding_fakie: bool,
    pub(crate) weight_forwards: bool,
    pub(crate) regular_stance: bool,
    pub(crate) living_world_impulse_active: bool,
    pub(crate) ai_input_active: bool,
    pub(crate) under_ai_control: bool,
    pub(crate) prevent_manual_respawn: bool,
    pub(crate) ignore_respawn_reset_button: bool,
    pub(crate) force_braking: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnimOutConstructionStage {
    Allocated,
    GlobalPose,
    LocalPose,
    Intents,
    Attributes,
    Returned,
}
#[derive(Debug)]
pub(crate) enum AnimOutConstructionError {
    AlreadyAttempted,
    Allocation,
    InvalidSize,
    Incomplete,
}

#[cfg_attr(
    test,
    expect(
        dead_code,
        reason = "retained constructor payloads await Actor SetUpPhysics publication"
    )
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent constructor and external-reset publication flags"
)]
pub(crate) struct AnimOutPhysInOwner {
    pub(crate) stage: AnimOutConstructionStage,
    attempted: bool,
    pub(crate) global_pose: Option<Vec<AnimationMatrix>>,
    pub(crate) local_pose: Option<Vec<AnimationMatrix>>,
    pub(crate) pose_w: pose_lanes::PoseWKnowledge,
    pub(crate) intents: Option<ActorGraphIntents>,
    // Inline capacity120, no Attribute constructors run for unoccupied cells.
    pub(crate) attributes: [Option<PhysicsAnimationAttribute>; 120],
    pub(crate) attribute_count: usize,
    pub(crate) size: u32,
    pub(crate) time_delta: f32,
    pub(crate) compression: f32,
    pub(crate) flags: AnimationPhysicsFlags,
    pub(crate) trick: AnimOutTrickData,
    pub(crate) gesture: u32,
    pub(crate) gesturing: bool,
    pub(crate) wipeout_gesture: [f32; 2],
    pub(crate) living_world_impulse: [f32; 4],
    pub(crate) ai: AnimationAiInput,
    pub(crate) external_reset_transform: SkeletonRootMatrix,
    pub(crate) external_reset_on_board: bool,
    pub(crate) external_reset_active: bool,
    pub(crate) reset_on_board: bool,
    pub(crate) truck_tightness: f32,
    pub(crate) wheel_hardness: f32,
    pub(crate) air_dismount_revert_frames: u32,
    pub(crate) center_of_mass_delta: WalkVector,
    pub(crate) center_of_mass_animation: WalkVector,
    pub(crate) foot_targets: [WalkVector; 2],
    pub(crate) next_step_position: WalkVector,
    pub(crate) next_step_normal: WalkVector,
    pub(crate) foot_ik: [f32; 2],
    pub(crate) foot_surface_ids: [u32; 2],
    pub(crate) locomotion_flags: u32,
}
impl AnimOutPhysInOwner {
    /// Actual outer allocation is from Sim (maskFFFFFFFF, OR100), hence all
    /// scalar members begin zero before the native constructor writes them.
    pub(crate) fn new_sim_allocation() -> Self {
        Self {
            stage: AnimOutConstructionStage::Allocated,
            attempted: false,
            global_pose: None,
            local_pose: None,
            pose_w: pose_lanes::PoseWKnowledge::default(),
            intents: None,
            attributes: std::array::from_fn(|_| None),
            attribute_count: 0,
            size: 0,
            time_delta: 0.0,
            compression: 0.0,
            flags: AnimationPhysicsFlags::default(),
            trick: AnimOutTrickData::reset_values(),
            gesture: 0,
            gesturing: false,
            wipeout_gesture: [0.0; 2],
            living_world_impulse: [0.0; 4],
            ai: AnimationAiInput::default(),
            external_reset_transform: SkeletonRootMatrix::known([0.0; 16]),
            external_reset_on_board: false,
            external_reset_active: false,
            reset_on_board: false,
            truck_tightness: 0.0,
            wheel_hardness: 0.0,
            air_dismount_revert_frames: 0,
            center_of_mass_delta: WalkVector::known([0.0; 4]),
            center_of_mass_animation: WalkVector::known([0.0; 4]),
            foot_targets: [WalkVector::known([0.0; 4]); 2],
            next_step_position: WalkVector::known([0.0; 4]),
            next_step_normal: WalkVector::known([0.0; 4]),
            foot_ik: [0.0; 2],
            foot_surface_ids: [0; 2],
            locomotion_flags: 0,
        }
    }

    pub(crate) fn try_construct(&mut self, size: u32) -> Result<(), AnimOutConstructionError> {
        if self.attempted {
            return Err(AnimOutConstructionError::AlreadyAttempted);
        }
        self.attempted = true;
        // Native oversized array requests UINT_MAX bytes. This checked host
        // boundary stops before attempting that invalid typed allocation.
        if size > 0x03ff_ffff {
            return Err(AnimOutConstructionError::InvalidSize);
        }
        self.global_pose = Some(allocate_pose(size)?);
        self.stage = AnimOutConstructionStage::GlobalPose;
        self.local_pose = Some(allocate_pose(size)?);
        self.stage = AnimOutConstructionStage::LocalPose;
        self.intents = Some(
            ActorGraphIntents::try_new::<std::convert::Infallible>()
                .map_err(|_| AnimOutConstructionError::Allocation)?,
        );
        self.stage = AnimOutConstructionStage::Intents;
        self.attribute_count = 0;
        self.stage = AnimOutConstructionStage::Attributes;
        self.size = size;
        self.trick = AnimOutTrickData::reset_values();
        self.gesturing = false;
        self.gesture = 35;
        self.wipeout_gesture = [0.0; 2];
        self.ai = AnimationAiInput::default();
        self.ai.trajectory_time = [-1.0; 4];
        self.reset_fields();
        self.stage = AnimOutConstructionStage::Returned;
        Ok(())
    }

    pub(crate) fn try_reset(&mut self) -> Result<(), AnimOutConstructionError> {
        if self.stage != AnimOutConstructionStage::Returned {
            return Err(AnimOutConstructionError::Incomplete);
        }
        self.reset_fields();
        Ok(())
    }
    fn reset_fields(&mut self) {
        self.time_delta = f32::from_bits(0x3c88_8889);
        self.compression = 0.5;
        self.flags = AnimationPhysicsFlags::default();
        self.foot_ik = [0.0; 2];
        self.truck_tightness = 0.0;
        self.wheel_hardness = 0.0;
        self.air_dismount_revert_frames = 0;
        self.locomotion_flags &= 0x01ff_ffff;
        self.foot_surface_ids = [0; 2];
        self.center_of_mass_delta = WalkVector::known([0.0; 4]);
        self.center_of_mass_animation = WalkVector::known([0.0; 4]);
        self.foot_targets = [WalkVector::known([0.0; 4]); 2];
        self.next_step_position = WalkVector::known([0.0; 4]);
        self.next_step_normal = WalkVector::known([0.0; 4]);
        let identity_axes = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let global = self
            .global_pose
            .as_mut()
            .expect("constructed global allocation");
        let local = self
            .local_pose
            .as_mut()
            .expect("constructed local allocation");
        for bone in 0..self.size as usize {
            local[bone] = identity_axes;
            global[bone] = identity_axes;
        }
        self.mark_pose_w_known(pose_lanes::PoseBuffer::Global)
            .expect("constructed global allocation");
        self.mark_pose_w_known(pose_lanes::PoseBuffer::Local)
            .expect("constructed local allocation");
        // Reset does not clear map/attributes/trick/gesture/AI payloads or
        // external reset/impulse payloads and external-reset flags.
    }
}
fn allocate_pose(size: u32) -> Result<Vec<AnimationMatrix>, AnimOutConstructionError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(size as usize)
        .map_err(|_| AnimOutConstructionError::Allocation)?;
    values.resize(size as usize, [0.0; 16]);
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_preserves_prior_payloads_but_restores_both_pose_arrays_and_live_flags() {
        let mut owner = AnimOutPhysInOwner::new_sim_allocation();
        owner.try_construct(2).unwrap();
        assert_eq!(owner.gesture, 35);
        assert_eq!(
            owner.ai.trajectory_time.map(f32::to_bits),
            [(-1.0_f32).to_bits(); 4]
        );
        owner.trick.bits = 42;
        owner.gesturing = true;
        owner.gesture = 7;
        owner.ai.trajectory_time = [3.0; 4];
        owner.external_reset_active = true;
        owner.external_reset_transform = SkeletonRootMatrix::known([2.0; 16]);
        owner.living_world_impulse = [5.0; 4];
        owner.flags.force_braking = true;
        owner.flags.ai_input_active = true;
        owner.locomotion_flags = u32::MAX;
        owner.global_pose.as_mut().unwrap()[0] = [9.0; 16];
        owner.local_pose.as_mut().unwrap()[1] = [8.0; 16];
        owner.try_reset().unwrap();
        assert_eq!(owner.trick.bits, 42);
        assert!(owner.gesturing);
        assert_eq!(owner.gesture, 7);
        assert_eq!(
            owner.ai.trajectory_time.map(f32::to_bits),
            [3.0_f32.to_bits(); 4]
        );
        assert!(owner.external_reset_active);
        assert_eq!(
            owner
                .external_reset_transform
                .known_lanes()
                .unwrap()
                .map(f32::to_bits),
            [2.0_f32.to_bits(); 16]
        );
        assert_eq!(
            owner.living_world_impulse.map(f32::to_bits),
            [5.0_f32.to_bits(); 4]
        );
        assert!(!owner.flags.force_braking);
        assert!(!owner.flags.ai_input_active);
        assert_eq!(owner.locomotion_flags, 0x01ff_ffff);
        for array in [&owner.global_pose, &owner.local_pose] {
            for matrix in array.as_ref().unwrap() {
                assert_eq!(matrix[15].to_bits(), 0.0_f32.to_bits());
                for index in [0, 5, 10] {
                    assert_eq!(matrix[index].to_bits(), 1.0_f32.to_bits());
                }
            }
        }
    }

    #[test]
    fn invalid_size_stops_before_any_inner_allocation_and_cannot_be_retried() {
        let mut owner = AnimOutPhysInOwner::new_sim_allocation();
        assert!(matches!(
            owner.try_construct(0x0400_0000),
            Err(AnimOutConstructionError::InvalidSize)
        ));
        assert_eq!(owner.stage, AnimOutConstructionStage::Allocated);
        assert!(owner.global_pose.is_none());
        assert!(owner.local_pose.is_none());
        assert!(owner.intents.is_none());
        assert!(matches!(
            owner.try_construct(1),
            Err(AnimOutConstructionError::AlreadyAttempted)
        ));
    }
}
