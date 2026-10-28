//! Actual ISkaterAnim::SetIsInLocomotion82cac690; draft uses typed retained world.
use super::ActorAnimationConstruction;
use crate::{
    generated_motion_load::runtime::MotionUpdateError,
    play_animation::FastString30,
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
};

#[derive(Debug)]
pub(crate) enum LocomotionLifecycleError {
    Missing(&'static str),
    UnknownFlags(u32),
    Motion(MotionUpdateError),
}

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];

impl<K, W> ActorAnimationConstruction<K, W> {
    /// ComputeNormalizedSpeed82c92ba0 transposes the retained world basis,
    /// evaluates local speed Z, then samples mirror state and scales by 1/4.2.
    pub(crate) fn try_compute_normalized_locomotion_speed(
        &self,
        speed: V,
    ) -> Result<f32, LocomotionLifecycleError> {
        use LocomotionLifecycleError as E;
        let world = &self
            .before_face
            .as_ref()
            .ok_or(E::Missing("locomotion suffix"))?
            .world_position;
        let forward = world.columns()[2];
        let x = forward.xyz[0] * speed.xyz[0];
        let xy = forward.xyz[1].mul_add(speed.xyz[1], x);
        let z = forward.xyz[2].mul_add(speed.xyz[2], xy);
        let player = self
            .player
            .as_ref()
            .ok_or(E::Missing("SkaterAnim player"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(E::UnknownFlags(0x4000_0000));
        }
        let factor = f32::from_bits(if player.flags_bits & 0x4000_0000 != 0 {
            0xbe73_cf3e
        } else {
            0x3e73_cf3e
        });
        Ok(z * factor)
    }

    /// Source reads world/speed only after the entering reset. Missing inputs
    /// retain that reset prefix; leaving removes construction keys in order.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn try_set_is_in_locomotion(
        &mut self,
        enabled: bool,
        world: Option<M>,
        speed: Option<V>,
    ) -> Result<(), LocomotionLifecycleError> {
        use LocomotionLifecycleError as E;
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        suffix.desired_speed = 0.0; //614c
        suffix.time_to_foot_change = 0.0; //6158
        suffix.collision_normal_timer = 0.0; //5fa0
        suffix.step_turn_adjustment = 0.0; //5fa8
        suffix.turn = 0.0; //6160
        suffix.sprint_level = 0.0; //6164
        let fields = self
            .before_sprint
            .as_mut()
            .ok_or(E::Missing("locomotion state"))?;
        fields.step_failure_counts = [0; 2];
        fields.second_flags_bits &= !0x4000_0000;
        fields.second_flags_known_mask |= 0x4000_0000;
        let player = self
            .player
            .as_mut()
            .ok_or(E::Missing("SkaterAnim player"))?;
        player.flags_bits =
            (player.flags_bits & 0xffea_f613) | 0x0004_0000 | (u32::from(enabled) << 20);
        player.flags_known_mask |= !0xffea_f613;
        suffix.collision_trajectory_displacement = V::known([0.0; 4]);
        suffix.last_centre_of_mass = V::known([0.0; 4]);
        suffix.collision_displacement = V::known([0.0; 4]);
        suffix.last_collision_normal = V::known([0.0; 4]);
        if enabled {
            fields.foot_down = -1;
            fields.last_foot_down = -1;
            // Native stores left/right separately before both rotation matrices.
            suffix.feet[1] = V::known([0.0; 4]);
            suffix.feet[0] = V::known([0.0; 4]);
            suffix.applied_extra_y_rotation = IDENTITY;
            suffix.extra_y_rotation = IDENTITY;
            let trajectory = &mut self
                .skater
                .as_mut()
                .ok_or(E::Missing("locomotion trajectory"))?
                .trajectory;
            for (input, physical) in trajectory
                .input_feet
                .iter_mut()
                .zip(&mut trajectory.physical_feet)
            {
                input.reset();
                physical.reset();
            }
            trajectory.physical_foot_index = 0;
            trajectory.first_update = true;
            trajectory.foot_locked = 0;
            trajectory.time = 0.0;
            trajectory.current_state = -1;
            trajectory.raw_time = 0.0;
            let world = world.ok_or(E::Missing("enter locomotion world assertion"))?;
            let speed = speed.ok_or(E::Missing("enter locomotion speed assertion"))?;
            suffix.world_position = world;
            let normalized = self.try_compute_normalized_locomotion_speed(speed)?;
            self.before_sprint.as_mut().expect("retained").speed = normalized;
        } else {
            suffix.time_since_leaving_locomotion = 0.0;
            suffix.world_position = M::from_columns(IDENTITY.map(V::known));
            fields.speed = 0.0;
            player.flags_bits &= 0xfffd_9fff;
            player.flags_known_mask |= !0xfffd_9fff;
            self.motion_remove_construction_attribute(
                FastString30::encode("StartSequenceID").expect("native literal"),
            )
            .map_err(E::Motion)?;
            self.motion_remove_construction_attribute(
                FastString30::encode("StartSelected").expect("native literal"),
            )
            .map_err(E::Motion)?;
        }
        Ok(())
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Concrete ISkaterAnim setters82cb7678..76e8, one field store each.
    pub(crate) fn motion_locomotion_flag(
        &mut self,
        mask: u32,
        value: bool,
    ) -> Result<(), MotionUpdateError> {
        let player = self
            .player
            .as_mut()
            .ok_or(MotionUpdateError::MissingOwner("locomotion player flags"))?;
        player.flags_bits = (player.flags_bits & !mask) | if value { mask } else { 0 };
        player.flags_known_mask |= mask;
        Ok(())
    }
    /// SetStartingFoot82cd4a60 writes Trajectory+0xa00/+0xa08, not mFootDown.
    pub(crate) fn motion_starting_locomotion_foot(
        &mut self,
        foot: u32,
    ) -> Result<(), MotionUpdateError> {
        let trajectory = &mut self
            .skater
            .as_mut()
            .ok_or(MotionUpdateError::MissingOwner("locomotion trajectory"))?
            .trajectory;
        trajectory.foot_locked = foot as i32;
        trajectory.physical_foot_index = foot as i32;
        let text = (foot as i32).wrapping_mul(3).to_string();
        self.motion_set_construction_attribute(
            FastString30::encode("StartSequenceID").expect("native literal"),
            FastString30::encode(&text).expect("i32 decimal"),
        )
    }
    /// Native82c8adf0 left then82c8ae00 right; these are retained mFeet, not trajectory feet.
    pub(crate) fn motion_locomotion_feet(
        &mut self,
        left: V,
        right: V,
    ) -> Result<(), MotionUpdateError> {
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(MotionUpdateError::MissingOwner("locomotion feet"))?;
        suffix.feet[1] = left;
        suffix.feet[0] = right;
        Ok(())
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    pub(crate) fn motion_locomotion_valid(&self) -> Result<bool, MotionUpdateError> {
        let player = self
            .player
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("locomotion flags"))?;
        if player.flags_known_mask & 0x40000 == 0 {
            return Err(MotionUpdateError::MissingOwner(
                "known locomotion valid flag",
            ));
        }
        if player.flags_bits & 0x40000 == 0 {
            return Ok(false);
        }
        let suffix = self
            .before_face
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("locomotion leave timer"))?;
        // raw82cb791c branches only orderedGT; NaN rejects.
        Ok(suffix.time_since_leaving_locomotion > 0.5)
    }
    /// ISkaterAnim+88 -> IsFootDown82c8b788. This reads Trajectory.FootLocked,
    /// CurrentState and Time, NOT SkaterAnim.mFootDown. Interface this+52e0.
    pub(crate) fn motion_locomotion_foot_down(&self, foot: i32) -> Result<bool, MotionUpdateError> {
        let player = self
            .player
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("locomotion player flags"))?;
        if player.flags_known_mask & 0x4000_0000 == 0 {
            return Err(MotionUpdateError::MissingOwner("locomotion mirrored flag"));
        }
        let trajectory = &self
            .skater
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("locomotion trajectory"))?
            .trajectory;
        let locked = trajectory
            .foot_locked
            .wrapping_add(i32::from(player.flags_bits & 0x4000_0000 != 0));
        // Native BGE rejects >=0.1 but allows unordered; signed remainder.
        Ok(trajectory.current_state == 0 && !(trajectory.time >= 0.1) && locked % 2 == foot)
    }
    /// ISkaterAnim+c0 -> IsTryingToSprint82c8b840; ordered GE0.5.
    pub(crate) fn motion_trying_to_sprint(&self) -> Result<bool, MotionUpdateError> {
        Ok(self
            .before_face
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("sprint level"))?
            .sprint_level
            >= 0.5)
    }
    pub(crate) fn motion_locomotion_quit_allowed(&self) -> Result<bool, MotionUpdateError> {
        let fields = self
            .before_sprint
            .as_ref()
            .ok_or(MotionUpdateError::MissingOwner("locomotion speed"))?;
        if fields.speed > 0.2 {
            return Ok(false);
        }
        Ok(fields.last_start_state != 2)
    }
}
