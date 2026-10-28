//! Exact simple state constructor stores; absent cells remain source-unwritten.
use super::{
    Arc, AttributeRuntimeVault, Boneless, DEFAULT_COLLECTION, FollowPath, FootPlant,
    GrabDescriptor, GrindCommon, GroundAnimation, HandPlant, IDENTITY, JumpInfo, KnownAir,
    LandingOnDeck, Nonspecific, OffBoard, OffBoardAir, OffBoardJump, OffBoardLocomotion,
    OffboardTrajectoryResult, OwnerMember, RetainedTrajectory, RevertGround, SlideGround,
    Teleporting, UP, ZERO,
};
impl SlideGround {
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {
        self.target_speed = Some(0.0);
        self.board_is_flipped = Some(false);
    }
}
impl RevertGround {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x018e_8a2e_5028_ab3f, DEFAULT_COLLECTION));
        self.pumping = Some(self.base.link(OwnerMember::Pumping));
        self.revert_direction = Some(0.0);
        self.direction_scalar = Some(1.0);
        self.time_in_state = Some(0.0);
        self.is_reverting = Some(false);
    }
}
impl GroundAnimation {
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {
        self.jump_info = Some(JumpInfo {
            velocity: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            com_height: 0.0,
            do_jump: false,
        });
    }
}
impl KnownAir {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attributes = Some(vault.instance(0x28ab_b872_cb8c_b9ed, DEFAULT_COLLECTION));
        self.grind_air_adjust = Some(self.base.link(OwnerMember::GrindAirAdjust));
        self.foot_plant_manager = Some(self.base.link(OwnerMember::FootPlantManager));
        self.landing_normal = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            UP,
        ));
        self.apex_pos = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.coll_pos = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.start_pos = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.trajectory_error = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.target_com_pos = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.time_in_state = Some(0.0);
        self.max_y = Some(0.0);
        self.coll_time = Some(0.0);
        self.time_to_apex = Some(0.0);
        self.body_flip_speed_target = Some(0.0);
        self.reached_apex = Some(false);
        self.landing_heading_valid = Some(false);
        self.start_flipped = Some(false);
        self.body_flipping = Some(false);
        self.grind_air_adjust_activated = Some(false);
        self.targeting_grind = Some(false);
        self.traj_index = Some(0);
    }
}
impl GrindCommon {
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {
        self.grind_dir = Some(UP);
        self.grind_normal = Some(UP);
        self.grind_skew = Some(UP);
        self.leaving_grind = Some(false);
        self.grind_substate = Some(0);
        self.precontact_target_orientation = Some(IDENTITY);
        self.already_jumped = Some(false);
        self.jumped_this_frame = Some(false);
        self.frames_in_grind = Some(0);
        self.inv_exit_frames = Some(0);
        self.crouch = Some(0.0);
        self.grind_jumper = Some(self.base.link(OwnerMember::GrindJumper));
        self.jump_vel = Some(ZERO);
    }
}
impl Nonspecific {
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {
        self.grind_jumper = Some(self.base.link(OwnerMember::GrindJumper));
    }
}
impl Teleporting {
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {
        self.phys_out_conditioners = Some(self.base.link(OwnerMember::PhysOutConditioners));
        self.teleport_to_on_board = Some(1);
        self.data_acquired = Some(0);
        self.teleport_ready = Some(0);
    }
}
impl FollowPath {
    #[expect(
        clippy::unused_self,
        reason = "source derived constructor adds only its vtable to the already retained base"
    )]
    pub(super) fn construct(&mut self, _vault: &Arc<AttributeRuntimeVault>) {}
}
impl HandPlant {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x4ab4_b3c7_411f_583a, DEFAULT_COLLECTION));
        self.hand_plant_manager = Some(self.base.link(OwnerMember::HandPlantManager));
        self.grab_spline_desc = Some(GrabDescriptor::INVALID);
    }
}
impl FootPlant {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x172d_5539_37d8_e07c, DEFAULT_COLLECTION));
        self.foot_plant_manager = Some(self.base.link(OwnerMember::FootPlantManager));
    }
}
impl Boneless {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0xccb9_5a83_c78b_4ff9, DEFAULT_COLLECTION));
        self.landing_toe_index = Some(-1);
        self.landing_toe_pos = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.right_leg = Some(false);
    }
}
impl OffBoard {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x4920_26e0_f096_f283, DEFAULT_COLLECTION));
        self.player_grab_spline = Some(self.base.link(OwnerMember::PlayerGrabSpline));
        self.off_board_feet_ik = Some(self.base.link(OwnerMember::OffBoardFeetIK));
        self.skateboard_controller = Some(self.base.link(OwnerMember::SkateboardController));
        self.offboard_location_test = Some(self.base.link(OwnerMember::OffboardLocationTest));
        self.offboard_jumping_test = Some(self.base.link(OwnerMember::OffboardJumpingTest));
        self.last_collision_response_direction = Some(
            crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
        );
        self.starting_com_vel = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            ZERO,
        ));
        self.last_collision_response_direction_valid = Some(false);
        self.grab_object = Some(false);
        self.is_locomotion_viable = Some(false);
        self.is_not_upright_entering_offboard = Some(false);
        self.is_moving_to_fast_entering_offboard = Some(false);
        self.grab_spline_desc = Some(GrabDescriptor::INVALID);
        self.ground_blend = Some(0.0);
        self.auto_turn = Some(0.0);
    }
}
impl OffBoardLocomotion {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x4920_26e0_f096_f283, DEFAULT_COLLECTION));
        self.player_grab_spline = Some(self.base.link(OwnerMember::PlayerGrabSpline));
        self.off_board_feet_ik = Some(self.base.link(OwnerMember::OffBoardFeetIK));
        self.skateboard_controller = Some(self.base.link(OwnerMember::SkateboardController));
        self.landing_on_deck_manager = Some(self.base.link(OwnerMember::LandingOnDeckManager));
        self.grab_object = Some(false);
        self.grab_spline_desc = Some(GrabDescriptor::INVALID);
        self.ground_blend = Some(0.0);
        self.trying_to_land_on_board = Some(false);
        self.try_to_land_on_board_delay = Some(0);
        self.is_moving_to_fast_entering_locomotion = Some(false);
    }
}
impl LandingOnDeck {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0xcdd9_26b3_5ef0_f6c4, DEFAULT_COLLECTION));
        self.lod_attribs = Some(vault.instance(0x2377_cef0_ff59_c4d2, DEFAULT_COLLECTION));
        self.landing_on_deck_manager = Some(self.base.link(OwnerMember::LandingOnDeckManager));
        self.time_to_land = Some(0.0);
        self.heavy_landing_wipeout = Some(false);
        self.on_deck = Some(false);
        self.flip_board = Some(false);
        self.hippy_jumping = Some(false);
        self.just_hippy_jumped = Some(0);
        self.turn_speed = Some(0.0);
        self.turn_speed_adjusted = Some(0.0);
        self.transition_angle = Some(0.0);
    }
}
impl OffBoardAir {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0xcdd9_26b3_5ef0_f6c4, DEFAULT_COLLECTION));
        self.offboard_trajectory_selector =
            Some(self.base.link(OwnerMember::OffboardTrajectorySelector));
        self.off_board_feet_ik = Some(self.base.link(OwnerMember::OffBoardFeetIK));
        self.skateboard_controller = Some(self.base.link(OwnerMember::SkateboardController));
        self.landing_on_deck_manager = Some(self.base.link(OwnerMember::LandingOnDeckManager));
        self.body_spin = Some(0.0);
        self.body_spin_adjusted = Some(0.0);
        self.transition_angle = Some(0.0);
        self.current_angle = Some(0.0);
        self.air_frame = Some(0);
        self.has_landed = Some(false);
        self.request_wipeout = Some(false);
        self.offboard_trajectory_result = Some(OffboardTrajectoryResult {
            com_position: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            com_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            landing_normal: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            landing_velocity: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            landing_position: crate::skeleton_root_transforms::SkeletonRootVector::known(ZERO),
            time_to_land: 0.0,
            frame: 0,
            landing_valid: false,
        });
    }
}
impl OffBoardJump {
    pub(super) fn construct(&mut self, vault: &Arc<AttributeRuntimeVault>) {
        self.attribs = Some(vault.instance(0x118b_9162_e026_2fbf, DEFAULT_COLLECTION));
        self.trajectory = Some(RetainedTrajectory::constructed());
        self.off_board_feet_ik = Some(self.base.link(OwnerMember::OffBoardFeetIK));
        self.skateboard_controller = Some(self.base.link(OwnerMember::SkateboardController));
        self.landing_on_deck_manager = Some(self.base.link(OwnerMember::LandingOnDeckManager));
        self.offboard_jumping_test = Some(self.base.link(OwnerMember::OffboardJumpingTest));
        self.body_spin = Some(0.0);
        self.body_spin_adjusted = Some(0.0);
        self.transition_angle = Some(0.0);
        self.current_angle = Some(0.0);
        self.air_time = Some(0.0);
        self.total_air_time = Some(0.0);
        self.time_to_land = Some(0.0);
        self.com_height_on_landing = Some(0.0);
        self.has_landed = Some(false);
        self.is_leap_of_faith = Some(false);
    }
}
