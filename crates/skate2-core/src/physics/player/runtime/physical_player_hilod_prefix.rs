//! Inline constructor stores82e03c94..82e03eb0, before `ProcessedPhysIn`.
//! State timers/counters and prepared velocity retain their existing owners.
use crate::{
    actor_construction::anim_out_phys_in::AnimationAiInput,
    skeleton_root_transforms::SkeletonRootVector,
};

#[expect(
    dead_code,
    reason = "retained HiLOD members await frame update consumers"
)]
pub(crate) struct PhysicalHiLodPrefix {
    pub(crate) ai: AnimationAiInput,
    pub(crate) ground_position: SkeletonRootVector,
    pub(crate) ground_position_all_states: SkeletonRootVector,
    pub(crate) ground_position_delta: SkeletonRootVector,
    pub(crate) ground_velocity_damped: SkeletonRootVector,
    /// Word+120 excluding bit01000000, owned by `PhysicalHiLodTail`.
    pub(crate) non_collision_flags: u32,
    pub(crate) ground_state_count: i32,
    pub(crate) frames_since_jump_correct: i32,
    pub(crate) update_count: i32,
    pub(crate) grind_illegal_count: i32,
    pub(crate) frames_since_teleport: i32,
    pub(crate) physics_requests_dismount_count: i32,
    pub(crate) time_in_state: f32,
    pub(crate) time_since_last_input: f32,
    pub(crate) time_on_ground: f32,
    pub(crate) last_crouch_input: f32,
    pub(crate) time_offboard: f32,
    pub(crate) skitching_vehicle_id: u32,
    pub(crate) skitch_vehicle_collision_avoid_timer: f32,
    pub(crate) skitch_delay_until_reskitchable: f32,
    pub(crate) pushing_delay_until_repushable: f32,
}

impl PhysicalHiLodPrefix {
    /// Actor825ff2d8 uses cleared Sim storage. AI trajectory time is -1 in all
    /// lanes; deck transform is zero, not a coordinate frame. Raw3de8 replaces
    /// high10 flag bits with e0000000 while preserving cleared low22 bits.
    pub(crate) fn construct_in_sim_allocation() -> Self {
        Self {
            ai: AnimationAiInput {
                trajectory_time: [-1.0; 4],
                ..AnimationAiInput::default()
            },
            ground_position: SkeletonRootVector::known([0.0; 4]),
            ground_position_all_states: SkeletonRootVector::known([0.0; 4]),
            ground_position_delta: SkeletonRootVector::known([0.0; 4]),
            ground_velocity_damped: SkeletonRootVector::known([0.0; 4]),
            non_collision_flags: 0xe000_0000,
            ground_state_count: 0,
            frames_since_jump_correct: 1000,
            update_count: 0,
            grind_illegal_count: 0,
            frames_since_teleport: 0,
            physics_requests_dismount_count: 0,
            time_in_state: 0.0,
            time_since_last_input: 0.0,
            time_on_ground: 0.0,
            last_crouch_input: 0.0,
            time_offboard: 0.0,
            skitching_vehicle_id: 0,
            skitch_vehicle_collision_avoid_timer: 0.0,
            skitch_delay_until_reskitchable: 0.0,
            pushing_delay_until_repushable: 0.0,
        }
    }
}

/// PDB `PhysicsDebugSystem` is an empty one-byte class. Allocation82e04158
/// precedes board `RemainOnReset82e04188`; no debug-system constructor is called.
pub(crate) struct PhysicsDebugSystem {
    // Empty C++ classes still occupy one byte; no constructor writes that byte.
    _storage: std::mem::MaybeUninit<u8>,
}
impl PhysicsDebugSystem {
    #[expect(
        clippy::unnecessary_box_returns,
        reason = "source allocates a distinct one-byte owner"
    )]
    pub(crate) fn allocate() -> Box<Self> {
        Box::new(Self {
            _storage: std::mem::MaybeUninit::uninit(),
        })
    }
}
