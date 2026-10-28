//! Complete locomotion branch of `SkaterAnim::GetPhysUpdateData82c96d58`.
pub(crate) use super::trajectory::AnimationLocomotionError;
use super::{ActorAnimationConstruction, AnimationPhysicsDataError, xyz_nan};
use crate::{
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner, animation::AnimationMatrix,
    input_cvar_runtime::InputCvarReferences, reckoning_numeric::ReckoningNumeric,
};

fn rotate_root(root: &mut AnimationMatrix, extra: [[f32; 4]; 4], global: bool) {
    let original = *root;
    // Global stores Z,T,X,Y; local stores Z,Y,X,T. Every arithmetic load uses
    // the original axes. Translation has its original T in the first FMA.
    for axis in if global { [2, 3, 0, 1] } else { [2, 1, 0, 3] } {
        for lane in 0..4 {
            let first = if axis == 3 {
                extra[axis][0].mul_add(original[lane], original[12 + lane])
            } else {
                extra[axis][0] * original[lane]
            };
            let second = extra[axis][1].mul_add(original[4 + lane], first);
            root[axis * 4 + lane] = extra[axis][2].mul_add(original[8 + lane], second);
        }
    }
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// Full `82c96d58` invocation with primitive numerical services. Requires
    /// real synchronized pose buffers and retained trajectory producers.
    pub(crate) fn try_get_physics_update<N: ReckoningNumeric>(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
        time_delta: f32,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AnimationLocomotionError<N::Error>> {
        if self.begin_physics_update(destination, time_delta)? {
            self.update_locomotion_physics(destination, cvars, numeric)?;
        } else {
            self.clear_non_locomotion_history()?;
        }
        self.finish_physics_update(destination, cvars)?;
        Ok(())
    }

    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered retained trajectory, pose, and history writes must remain observable at each failure"
    )]
    fn update_locomotion_physics<N: ReckoningNumeric>(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), AnimationLocomotionError<N::Error>> {
        use AnimationLocomotionError as E;
        use AnimationPhysicsDataError as P;
        let displacement = self
            .before_face
            .as_ref()
            .ok_or(P::Missing("constructor suffix"))?
            .collision_trajectory_displacement;
        let trajectory = &mut self
            .skater
            .as_mut()
            .ok_or(P::Missing("trajectory"))?
            .trajectory;
        let index = usize::try_from(trajectory.physical_foot_index)
            .ok()
            .filter(|index| *index < 4)
            .ok_or(P::FootIndex(trajectory.physical_foot_index))?;
        let foot_position = &mut trajectory.physical_feet[index].data.position;
        *foot_position = foot_position.zip(displacement, |position, delta| position + delta);
        trajectory.try_current_com(&mut destination.center_of_mass_delta, true, cvars, numeric)?;
        trajectory.try_current_com(
            &mut destination.center_of_mass_animation,
            false,
            cvars,
            numeric,
        )?;
        if xyz_nan(&destination.center_of_mass_delta.xyz) {
            return Err(E::InvalidVector("world COM"));
        }
        if xyz_nan(&destination.center_of_mass_animation.xyz) {
            return Err(E::InvalidVector("animation COM"));
        }
        // Valid physical indices are unchanged by the source modulo operation.
        let current = &trajectory.physical_feet[index];
        let next = &trajectory.physical_feet[(index + 1) % 4];
        let valid = current.initialized;
        if valid {
            let foot =
                if next.initialized && next.data.position.xyz[1] > current.data.position.xyz[1] {
                    next
                } else {
                    current
                };
            destination.next_step_position = foot.data.position;
            destination.next_step_normal =
                if cvars.try_sa_use_world_y_for_normal().map_err(P::Cvar)? {
                    crate::skeleton_root_transforms::SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0])
                } else {
                    foot.data.normal
                };
        }
        destination.flags.next_step_valid = valid;
        let extra = self
            .before_face
            .as_ref()
            .expect("suffix checked")
            .applied_extra_y_rotation;
        let initialization = self
            .initialization
            .as_ref()
            .ok_or(P::Missing("initialized body buffers"))?;
        initialization
            .global_matrices
            .try_update_root_matrix(|root| rotate_root(root, extra, true))
            .map_err(P::Pose)?;
        initialization
            .local_matrices
            .try_update_root_matrix(|root| rotate_root(root, extra, false))
            .map_err(P::Pose)?;
        destination.foot_ik[1] = 0.0;
        destination.foot_ik[0] = 0.0;
        if self.physics_flag(8)? {
            let prior = self
                .before_face
                .as_ref()
                .expect("suffix checked")
                .last_centre_of_mass;
            let delta = destination
                .center_of_mass_delta
                .zip(prior, |current, prior| current - prior);
            let threshold = cvars.try_loco_wipeout_delta().map_err(P::Cvar)?;
            let squared_threshold = threshold * threshold;
            let squared = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
            // Raw BLT skips; unordered continues, unlike a >= Rust condition.
            if squared
                .partial_cmp(&squared_threshold)
                .is_none_or(|order| !order.is_lt())
                && !self.physics_flag(0x200)?
            {
                destination.locomotion_flags |= super::REQUEST_WIPEOUT;
            }
        }
        self.before_face
            .as_mut()
            .expect("suffix checked")
            .last_centre_of_mass = destination.center_of_mass_delta;
        self.player.as_mut().expect("flags checked").flags_bits |= 8;
        let trajectory = &self.skater.as_ref().expect("trajectory checked").trajectory;
        if matches!(trajectory.current_state, 0 | 1) {
            let foot_down = self
                .before_sprint
                .as_ref()
                .ok_or(P::Missing("foot-down state"))?
                .foot_down;
            if foot_down == -1 {
                return Err(E::FootDownRequired);
            }
            let side = usize::from(foot_down & 1 == 0);
            destination.foot_ik[side] = 1.0;
            destination.foot_targets[side] = trajectory.physical_feet[index].data.position;
            destination.foot_surface_ids[side] = trajectory.physical_feet[index].data.surface;
        }
        if !self.physics_flag(0x0010_0000)? {
            self.player.as_mut().expect("flags checked").flags_bits &= 0xfffd_9fff;
            let trajectory = &mut self.skater.as_mut().expect("trajectory checked").trajectory;
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
        }
        Ok(())
    }
}
