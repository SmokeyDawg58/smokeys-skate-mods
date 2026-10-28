//! Subject field publication suffix of `GameCameraManager::UpdateSubject82e38e80`.
//! Anchor publication precedes this function; reference/compass updates follow it.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_GROUND_ANIMATION,
    PHYSICS_STATE_OFFBOARD_AIR,
};
use super::selected_runtime::CameraGlobalCondition;
use crate::{
    actor_animation_construction::physics_update_data::AnimationPhysicsDataError,
    actor_construction::animation_mount::MountedActorAnimation,
    camera_subject::CameraSubject,
    physical_player_phys_out::{
        PhysOutGroundVector, PhysOutMotionVector, PhysOutSystemVector, PhysicalPlayerPhysOut,
        construct::ConcretePhysOutError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

pub(crate) trait CameraSubjectAnimationRead {
    type Error;
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    fn running_out(&mut self) -> Result<bool, Self::Error>;
}
pub(crate) struct CameraAnimationRead<'a>(pub(crate) &'a Arc<Mutex<MountedActorAnimation>>);
#[derive(Debug)]
pub(crate) enum CameraAnimationReadError {
    OwnerLock,
    Native(AnimationPhysicsDataError),
}
impl CameraSubjectAnimationRead for CameraAnimationRead<'_> {
    type Error = CameraAnimationReadError;
    fn mirrored(&mut self) -> Result<bool, Self::Error> {
        self.0
            .lock()
            .map_err(|_| Self::Error::OwnerLock)?
            .try_camera_mirrored()
            .map_err(Self::Error::Native)
    }
    fn running_out(&mut self) -> Result<bool, Self::Error> {
        self.0
            .lock()
            .map_err(|_| Self::Error::OwnerLock)?
            .try_camera_running_out()
            .map_err(Self::Error::Native)
    }
}
#[derive(Debug)]
pub(crate) enum CameraSubjectUpdateError<E> {
    Component(ConcretePhysOutError<Infallible>),
    Animation(E),
}

impl CameraSubject {
    /// Source `82e38ff0..82e396a8`; each reached read precedes its own store.
    /// Offboard-air502 skips the trajectory payload, retaining its prior values.
    #[expect(
        clippy::too_many_lines,
        reason = "native ordered Subject publication with failure prefixes"
    )]
    pub(crate) fn try_publish_fields<A: CameraSubjectAnimationRead>(
        &mut self,
        output: &PhysicalPlayerPhysOut,
        animation: &mut A,
        global: &CameraGlobalCondition,
    ) -> Result<(), CameraSubjectUpdateError<A::Error>> {
        use CameraSubjectUpdateError::{Animation, Component};
        let offboard_air = output.try_state_output().map_err(Component)?.physics_state == PHYSICS_STATE_OFFBOARD_AIR;
        let valid = output.try_air_output().map_err(Component)?.is_traj_valid;
        self.flags.trajectory_valid = valid || offboard_air;
        if !offboard_air {
            self.trajectory_elapsed = output
                .try_air_output()
                .map_err(Component)?
                .time_in_known_air;
            self.trajectory_start_position = output
                .try_air_output()
                .map_err(Component)?
                .traj_start_position;
            self.trajectory_start_normal = output
                .try_camera_ground_vector(PhysOutGroundVector::LastValidGroundUp)
                .map_err(Component)?;
            self.trajectory_end_position = output
                .try_air_output()
                .map_err(Component)?
                .traj_coll_position;
            self.trajectory_end_time = output.try_air_output().map_err(Component)?.traj_total_time;
            self.trajectory_end_normal =
                output.try_air_output().map_err(Component)?.traj_coll_normal;
            self.trajectory_apex_position = output.try_air_output().map_err(Component)?.traj_apex;
            self.trajectory_apex_time = output
                .try_air_output()
                .map_err(Component)?
                .total_time_to_apex;
            self.trajectory_target_heading = output
                .try_air_output()
                .map_err(Component)?
                .traj_target_heading;
        }
        self.flags.teleported = output
            .try_state_output()
            .map_err(Component)?
            .has_just_teleported;
        self.flags.wiping_out = output.try_state_output().map_err(Component)?.is_wiping_out;
        self.flags.wipeout_over = output
            .try_skeleton_output()
            .map_err(Component)?
            .is_wipeout_over;
        self.flags.riding_fakie = output
            .try_animation_output()
            .map_err(Component)?
            .is_riding_fakie;
        self.flags.starting_to_jump = output
            .try_air_output()
            .map_err(Component)?
            .is_starting_to_jump;
        self.flags.ground_animation =
            output.try_state_output().map_err(Component)?.physics_state == PHYSICS_STATE_GROUND_ANIMATION;
        self.flags.ground_physics = output
            .try_state_output()
            .map_err(Component)?
            .physics_state_category
            == PHYSICS_CATEGORY_GROUND;
        self.flags.anticipating = output
            .try_camera_intents_output()
            .map_err(Component)?
            .is_anticipating;
        self.flags.manual = output.try_state_output().map_err(Component)?.is_manual
            && !output
                .try_camera_intents_output()
                .map_err(Component)?
                .is_kickturning;
        self.flags.physically_pushing = output
            .try_state_output()
            .map_err(Component)?
            .is_physically_pushing;
        self.flags.pushable_speed = output
            .try_camera_motion_output()
            .map_err(Component)?
            .is_at_pushable_speed;
        self.flags.mirrored = animation.mirrored().map_err(Animation)?;
        self.flags.offboard = output
            .try_state_output()
            .map_err(Component)?
            .physics_state_category
            == PHYSICS_CATEGORY_OFFBOARD;
        self.flags.running_out = animation.running_out().map_err(Animation)?;
        self.transform = global.subject_transform;
        self.board_effective_transform = output
            .try_camera_board_output()
            .map_err(Component)?
            .effective_transform;
        self.board_position = output
            .try_camera_board_output()
            .map_err(Component)?
            .deck_position;
        self.board_velocity = output
            .try_camera_motion_vector(PhysOutMotionVector::DeckVelocity)
            .map_err(Component)?;
        self.board_angular_velocity = output
            .try_camera_motion_vector(PhysOutMotionVector::AngularVelocity)
            .map_err(Component)?;
        self.centre_of_mass_to_deck = output
            .try_animation_output()
            .map_err(Component)?
            .centre_of_mass_to_deck;
        self.pumping_acceleration = output
            .try_ground_output()
            .map_err(Component)?
            .pumping_acceleration;
        self.animation_input_lean = output
            .try_animation_output()
            .map_err(Component)?
            .cam_animation_input_lean;
        self.animation_turn = output
            .try_animation_output()
            .map_err(Component)?
            .cam_animation_turn;
        self.input_turn = output.try_animation_output().map_err(Component)?.input_turn;
        self.input_kickturn = output
            .try_animation_output()
            .map_err(Component)?
            .input_kickturn;
        self.flags.grinding = output
            .try_camera_grinds_output()
            .map_err(Component)?
            .is_grinding;
        self.grind_direction = output
            .try_camera_grinds_output()
            .map_err(Component)?
            .grind_primitive_dir;
        self.flags.grind_forwards = output
            .try_camera_grinds_output()
            .map_err(Component)?
            .is_grind_facing_forwards;
        self.hips_position = output
            .try_skeleton_output()
            .map_err(Component)?
            .skeleton_hips_pos;
        // Raw3957c..88 passes Skeleton+0, its effective animation Z, despite
        // the Subject setter's HipsLook name. Wipeout LookAt uses distinct+30.
        self.hips_look = output
            .try_skeleton_output()
            .map_err(Component)?
            .effective_anim_z;
        self.skeleton_root = output
            .try_skeleton_output()
            .map_err(Component)?
            .effective_skeleton_root_matrix;
        self.ground_normal = output
            .try_camera_ground_vector(PhysOutGroundVector::GroundUp)
            .map_err(Component)?;
        self.last_valid_ground_normal = output
            .try_camera_ground_vector(PhysOutGroundVector::LastValidGroundUp)
            .map_err(Component)?;
        self.skitch_grab_position = output
            .try_ground_output()
            .map_err(Component)?
            .skitch_parametric_pos;
        self.up = output
            .try_camera_system_vector(PhysOutSystemVector::Up)
            .map_err(Component)?;
        let skeleton = output.try_skeleton_output().map_err(Component)?;
        let mut x = skeleton.off_board_look_at_x;
        let mut y = skeleton.off_board_look_at_y;
        if global.invert_x {
            x = -x;
        }
        if global.invert_y {
            y = -y;
        }
        self.look_stick = SkeletonRootVector::known([x, y, 0.0, 0.0]);
        self.painted_region_blend = global.painted_region_blend;
        Ok(())
    }
}

#[cfg(test)]
#[path = "camera_subject_update_tests.rs"]
mod tests;
