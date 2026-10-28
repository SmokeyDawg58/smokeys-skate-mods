//! Actual remaining offboard constructor fields in the retained physical owner.
//! `FeetIK`82dcd048, Landing82dce3a0, Location82dd1f08, Jump82dd4fd8.
#![expect(
    dead_code,
    reason = "retained source constructor fields await original offboard update consumers"
)]
use crate::physics_state_ids::{
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_LOCOMOTION,
    PHYSICS_STATE_OFFBOARD_PUSHING,
};
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_footplant::RetainedTrajectory,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex, Weak};

#[path = "../offboard/physical_player_offboard_feet_ik_ground_finish.rs"]
pub(crate) mod feet_ik_ground_finish;
#[path = "../offboard/physical_player_offboard_feet_ik_prefix.rs"]
pub(crate) mod feet_ik_prefix;

#[path = "../offboard/physical_player_offboard_jumping_runtime.rs"]
pub(crate) mod jumping_runtime;
#[path = "../offboard/physical_player_offboard_location_runtime.rs"]
pub(crate) mod location_runtime;

#[derive(Default)]
pub(crate) struct RemainingConstructorOwners {
    pub(crate) feet_ik: Option<OffboardFeetIk>,
    pub(crate) landing: Option<LandingOnDeck>,
    pub(crate) location: Option<OffboardLocation>,
    pub(crate) jumping: Option<OffboardJumping>,
}

/// Actual parent and world occurrence; queries remain absent until their producer runs.
pub(crate) struct WorldBinding {
    owner: Weak<PhysicalPlayerAssemblyOwner>,
    manager: SimulationManagerContactAuthority,
}
impl WorldBinding {
    fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            manager: owner.manager(),
        }
    }
    pub(crate) fn matches(&self, owner: &Arc<PhysicalPlayerAssemblyOwner>) -> bool {
        Weak::ptr_eq(&self.owner, &Arc::downgrade(owner)) && self.manager == owner.manager()
    }
}

pub(crate) struct SkeletonBinding {
    owner: Weak<PhysicalPlayerAssemblyOwner>,
    processed: Arc<Mutex<SkeletonProcessedContact>>,
    skeleton: SimulationSkaterAssemblyHandle,
}
impl SkeletonBinding {
    fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: Arc<Mutex<SkeletonProcessedContact>>,
        skeleton: SimulationSkaterAssemblyHandle,
    ) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            processed,
            skeleton,
        }
    }
    pub(crate) fn matches(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> bool {
        Weak::ptr_eq(&self.owner, &Arc::downgrade(owner))
            && Arc::ptr_eq(&self.processed, processed)
            && self.skeleton.same_as(skeleton)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SurfaceResult {
    pub(crate) position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) normal: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) surface_id: i32,
    pub(crate) valid: bool,
}
impl SurfaceResult {
    const fn reset() -> Self {
        Self {
            position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            normal: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            surface_id: 0,
            valid: false,
        }
    }
}

/// PDB320 bytes. Reset82dc8728 writes every named value and preserves both bindings.
#[expect(
    clippy::struct_excessive_bools,
    reason = "eight independently produced native foot IK flags"
)]
pub(crate) struct OffboardFeetIk {
    pub(crate) binding: SkeletonBinding,
    pub(crate) left_foot_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) right_foot_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) foot_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) current_animation_position_left: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) current_animation_position_right:
        crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) bad_ground_correction: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) last_valid_trajectory_direction: [f32; 4],
    pub(crate) last_good_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) ground_normal_left: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) ground_normal_right: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) result_left: SurfaceResult,
    pub(crate) result_right: SurfaceResult,
    pub(crate) animation_space_y: f32,
    pub(crate) time_in_state: f32,
    pub(crate) left_slope_adjust_y: f32,
    pub(crate) right_slope_adjust_y: f32,
    pub(crate) height_above_ground: f32,
    pub(crate) left_fixed_blend_out_timer: f32,
    pub(crate) right_fixed_blend_out_timer: f32,
    pub(crate) state: i32,
    pub(crate) bad_ground_correction_counter: i32,
    pub(crate) wipeout_counter: i32,
    pub(crate) foot_not_down_wipeout_counter: i32,
    pub(crate) animation_left_foot_down: bool,
    pub(crate) animation_right_foot_down: bool,
    pub(crate) left_foot_down: bool,
    pub(crate) right_foot_down: bool,
    pub(crate) foot_locking_active: bool,
    pub(crate) request_wipeout: bool,
    pub(crate) trajectory_direction_valid: bool,
    pub(crate) last_good_position_valid: bool,
}
impl OffboardFeetIk {
    /// `ConsiderReset` 0x82dcd110. The caller performs Skeleton.DisableIK
    /// immediately after a reset, before the foot-locking store.
    pub(crate) fn consider_reset_before_disable_ik(&mut self, last: i32) -> bool {
        let reset = !(PHYSICS_STATE_OFFBOARD..=PHYSICS_STATE_OFFBOARD_AIR).contains(&last) && last != PHYSICS_STATE_OFFBOARD_LOCOMOTION;
        if reset {
            self.reset_after_teleport();
        }
        reset
    }

    /// `Reset` 0x82dc8728; the processed and Skeleton bindings survive.
    pub(crate) fn reset_after_teleport(&mut self) {
        let zero = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        let up = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        self.left_foot_position = zero;
        self.animation_space_y = 0.0;
        self.time_in_state = 0.0;
        self.left_slope_adjust_y = 0.0;
        self.right_slope_adjust_y = 0.0;
        self.animation_left_foot_down = false;
        self.state = 2;
        self.height_above_ground = 0.0;
        self.left_fixed_blend_out_timer = 0.0;
        self.animation_right_foot_down = false;
        self.right_fixed_blend_out_timer = 0.0;
        self.left_foot_down = false;
        self.right_foot_down = false;
        self.right_foot_position = zero;
        self.foot_locking_active = false;
        self.request_wipeout = false;
        self.trajectory_direction_valid = false;
        self.last_good_position_valid = false;
        self.foot_position = zero;
        self.bad_ground_correction_counter = 0;
        self.current_animation_position_left = zero;
        self.wipeout_counter = 0;
        self.current_animation_position_right = zero;
        self.foot_not_down_wipeout_counter = 0;
        self.bad_ground_correction = zero;
        self.last_valid_trajectory_direction = [0.0; 4];
        self.last_good_position = zero;
        self.ground_normal_left = up;
        self.ground_normal_right = up;
        self.result_left.surface_id = 0;
        self.result_left.valid = false;
        self.result_left.position = zero;
        self.result_left.normal = zero;
        self.result_right.surface_id = 0;
        self.result_right.position = zero;
        self.result_right.valid = false;
        self.result_right.normal = zero;
    }

    /// `ConsiderReset` 0x82dcd16c, after the optional Skeleton.DisableIK.
    pub(crate) fn finish_consider_reset(&mut self, current: i32) {
        self.foot_locking_active = current == PHYSICS_STATE_OFFBOARD_PUSHING;
    }

    pub(crate) fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: Arc<Mutex<SkeletonProcessedContact>>,
        skeleton: SimulationSkaterAssemblyHandle,
    ) -> Self {
        Self {
            binding: SkeletonBinding::new(owner, processed, skeleton),
            left_foot_position: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            right_foot_position: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            foot_position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            current_animation_position_left:
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            current_animation_position_right:
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            bad_ground_correction: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            last_valid_trajectory_direction: [0.0; 4],
            last_good_position: crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            ),
            ground_normal_left: crate::skeleton_root_transforms::SkeletonRootVector::known([
                0.0, 1.0, 0.0, 0.0,
            ]),
            ground_normal_right: crate::skeleton_root_transforms::SkeletonRootVector::known([
                0.0, 1.0, 0.0, 0.0,
            ]),
            result_left: SurfaceResult::reset(),
            result_right: SurfaceResult::reset(),
            animation_space_y: 0.0,
            time_in_state: 0.0,
            left_slope_adjust_y: 0.0,
            right_slope_adjust_y: 0.0,
            height_above_ground: 0.0,
            left_fixed_blend_out_timer: 0.0,
            right_fixed_blend_out_timer: 0.0,
            state: 2,
            bad_ground_correction_counter: 0,
            wipeout_counter: 0,
            foot_not_down_wipeout_counter: 0,
            animation_left_foot_down: false,
            animation_right_foot_down: false,
            left_foot_down: false,
            right_foot_down: false,
            foot_locking_active: false,
            request_wipeout: false,
            trajectory_direction_valid: false,
            last_good_position_valid: false,
        }
    }
}

#[path = "../offboard/physical_player_landing_on_deck_runtime.rs"]
pub(crate) mod landing_runtime;

/// PDB224 bytes. Constructor82dce3a0 calls Reset82dc8c80 after binding attributes.
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent native landing query flags"
)]
pub(crate) struct LandingOnDeck {
    pub(crate) skeleton: SkeletonBinding,
    pub(crate) world: WorldBinding,
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) trajectory_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::trajectory::SimulationWorldTrajectoryBatch>,
    pub(crate) trajectory: RetainedTrajectory,
    pub(crate) ideal_trajectory: RetainedTrajectory,
    pub(crate) trajectory_time: f32,
    pub(crate) trajectory_valid: bool,
    pub(crate) landing_error: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) landing_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) test_trajectory_end_y: f32,
    pub(crate) time_to_land: f32,
    pub(crate) ideal_time_to_land: f32,
    pub(crate) can_land: bool,
    pub(crate) landing_hippy_jump: bool,
    pub(crate) something_in_the_way: bool,
    pub(crate) tested_trajectory: bool,
}
impl LandingOnDeck {
    /// LandingOnDeckManager::Reset82dc8c80; the ideal trajectory, query
    /// batch and bindings survive this selected in-place reset.
    pub(crate) fn reset_after_teleport(&mut self) {
        let zero = crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.trajectory.position = zero;
        self.trajectory.velocity = zero;
        self.trajectory.gravity = zero;
        self.trajectory.time = [-1.0; 4];
        self.trajectory_valid = false;
        self.trajectory_time = 0.0;
        self.can_land = false;
        self.test_trajectory_end_y = 0.0;
        self.landing_hippy_jump = false;
        self.time_to_land = 0.0;
        self.something_in_the_way = false;
        self.ideal_time_to_land = 0.0;
        self.tested_trajectory = false;
        self.landing_error = zero;
        self.landing_velocity = zero;
    }

    pub(crate) fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: Arc<Mutex<SkeletonProcessedContact>>,
        skeleton: SimulationSkaterAssemblyHandle,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Self {
        Self {
            attributes: vault.instance(0x2377_cef0_ff59_c4d2, 0xd7ed_bd36_2d7d_2152),
            skeleton: SkeletonBinding::new(owner, processed, skeleton),
            world: WorldBinding::new(owner),
            trajectory_batch: None,
            trajectory: RetainedTrajectory::constructed(),
            ideal_trajectory: RetainedTrajectory::constructed(),
            trajectory_time: 0.0,
            trajectory_valid: false,
            landing_error: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            landing_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            test_trajectory_end_y: 0.0,
            time_to_land: 0.0,
            ideal_time_to_land: 0.0,
            can_land: false,
            landing_hippy_jump: false,
            something_in_the_way: false,
            tested_trajectory: false,
        }
    }
}

/// PDB80 bytes. Raw82dd1f28..64 initializes all vector lanes, including W=0.
pub(crate) struct OffboardLocation {
    pub(crate) world: WorldBinding,
    pub(crate) ground_position: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) look_direction: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) jump_velocity: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) jumping: bool,
    pub(crate) collide_with_world_only: bool,
    pub(crate) sync_allowed: bool,
    pub(crate) line_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch>,
    pub(crate) trajectory_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::trajectory::SimulationWorldTrajectoryBatch>,
}
impl OffboardLocation {
    pub(crate) fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            world: WorldBinding::new(owner),
            ground_position: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            look_direction: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            jump_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            jumping: false,
            collide_with_world_only: false,
            sync_allowed: false,
            line_batch: None,
            trajectory_batch: None,
        }
    }
}

/// PDB112 bytes. Raw82dd4ff8..5034 explicitly initializes each result/input vector.
pub(crate) struct OffboardJumping {
    pub(crate) world: WorldBinding,
    pub(crate) ground_position: [f32; 4],
    pub(crate) jump_direction: [f32; 4],
    pub(crate) jump_distance: f32,
    pub(crate) landing_position: [f32; 4],
    pub(crate) landing_position_valid: bool,
    pub(crate) leap_of_faith: bool,
    pub(crate) line_batch: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineBatch>,
    pub(crate) trajectory_batch_one: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::trajectory::SimulationWorldTrajectoryBatch>,
    pub(crate) trajectory_batch_two: Option<crate::simulation_assembly_backend::constraint_session::world_line_query::trajectory::SimulationWorldTrajectoryBatch>,
    pub(crate) result_valid_counter: i32,
}
impl OffboardJumping {
    pub(crate) fn new(owner: &Arc<PhysicalPlayerAssemblyOwner>) -> Self {
        Self {
            world: WorldBinding::new(owner),
            ground_position: [0.0; 4],
            jump_direction: [0.0; 4],
            jump_distance: 0.0,
            landing_position: [0.0; 4],
            landing_position_valid: false,
            leap_of_faith: false,
            line_batch: None,
            trajectory_batch_one: None,
            trajectory_batch_two: None,
            result_valid_counter: 0,
        }
    }
}

/// Corpus caller supplies the actual retained owner fields after their constructor stages.
#[cfg(test)]
pub(crate) fn check_original(
    values: &RemainingConstructorOwners,
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    processed: &Arc<Mutex<SkeletonProcessedContact>>,
    skeleton: &SimulationSkaterAssemblyHandle,
) {
    let feet = values.feet_ik.as_ref().unwrap();
    let landing = values.landing.as_ref().unwrap();
    let location = values.location.as_ref().unwrap();
    let jump = values.jumping.as_ref().unwrap();
    assert!(feet.binding.matches(owner, processed, skeleton));
    assert!(landing.skeleton.matches(owner, processed, skeleton));
    assert!(
        landing.world.matches(owner) && location.world.matches(owner) && jump.world.matches(owner)
    );
    assert!(!landing.attributes.is_null());
    assert_eq!(feet.state, 2);
    assert_eq!(
        feet.ground_normal_left
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0, 1.0_f32.to_bits(), 0, 0]
    );
    assert_eq!(
        feet.ground_normal_right
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        feet.ground_normal_left
            .known_lanes()
            .unwrap()
            .map(f32::to_bits)
    );
    assert!(!feet.result_left.valid && !feet.result_right.valid && !feet.request_wipeout);
    assert_eq!(
        landing.trajectory.time.map(f32::to_bits),
        [(-1.0_f32).to_bits(); 4]
    );
    assert_eq!(
        landing.ideal_trajectory.time.map(f32::to_bits),
        [(-1.0_f32).to_bits(); 4]
    );
    assert!(
        landing.trajectory_batch.is_none()
            && !landing.trajectory_valid
            && !landing.tested_trajectory
    );
    assert!(
        location.line_batch.is_none()
            && location.trajectory_batch.is_none()
            && !location.sync_allowed
    );
    assert_eq!(
        location
            .look_direction
            .known_lanes()
            .unwrap()
            .map(f32::to_bits),
        [0; 4]
    );
    assert!(
        jump.line_batch.is_none()
            && jump.trajectory_batch_one.is_none()
            && jump.trajectory_batch_two.is_none()
    );
    assert!(!jump.landing_position_valid && !jump.leap_of_faith);
    assert_eq!(jump.result_valid_counter, 0);
}
