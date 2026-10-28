//! Skeleton.UpdateOffBoardPushing82b5def0 on the existing attached player pair.
use super::{
    Arc, GroundedProcessed, HostReckoningNumeric, HostReckoningSinCosNumeric, InputCvarReferences,
    OffboardGroundFeetIkError as E, PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
    SimulationSkaterAssemblyHandle, SkeletonRootMatrix, SkeletonRootVector, StaticGroundPool,
    transform_point_stage,
};

/// 82b5df64..dffc: transform the displacement by the inverse rotation only.
/// The initial multiply is distinct from the following two fused additions.
/// Translation and motion.W do not participate; unknown basis W stays unknown.
fn pushing_trajectory(
    world_to_animation: &SkeletonRootMatrix,
    motion: SkeletonRootVector,
) -> SkeletonRootMatrix {
    let columns = world_to_animation.columns();
    let x = columns[0].map(|lane| lane * motion.xyz[0]);
    let xy = transform_point_stage(x, columns[1], motion.xyz[1]);
    let local = transform_point_stage(xy, columns[2], motion.xyz[2]);
    SkeletonRootMatrix::translation(local)
}

impl PhysicalAssemblyWorld {
    /// State501 output and FeetIK output use the same retained state owners.
    pub(crate) fn try_fill_pair_offboard_pushing<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        num: &mut N,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError>
    where
        N::Error: std::fmt::Debug,
    {
        use crate::{
            physical_player_assembly::remaining_states::{StateKind, StatePayload},
            physical_player_grind_manager::view::{GrindError, error, need},
        };
        if !self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(error)?
        {
            return Err(GrindError("foreign Pushing output pair".into()));
        }
        let states = owner.remaining_states().lock().map_err(error)?;
        let value = need(
            states
                .states
                .get(&StateKind::OffBoardPushing)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_ref()),
            "constructed Pushing501",
        )?;
        let StatePayload::OffBoardPushing(state) = value else {
            return Err(GrindError("wrong Pushing payload".into()));
        };
        let bindings = &state.base.physical;
        if !bindings.board.same_as(board)
            || !bindings.skeleton.same_as(skeleton)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|v| Arc::ptr_eq(&v, owner))
        {
            return Err(GrindError("foreign Pushing state binding".into()));
        }
        let processed = Arc::clone(owner.reckoning().lock().map_err(error)?.processed());
        owner
            .phys_out()
            .lock()
            .map_err(error)?
            .try_fill_pushing_state(state, num, |foot| {
                let remaining = owner.remaining().lock().map_err(error)?;
                let feet = need(remaining.feet_ik.as_ref(), "Pushing FeetIK")?;
                if !feet.binding.matches(owner, &processed, skeleton) {
                    return Err(GrindError("foreign Pushing FeetIK".into()));
                }
                Ok(if foot == 0 {
                    feet.left_foot_down
                } else {
                    feet.right_foot_down
                })
            })
    }

    /// Pushing.Update82d9cff8 issues the two controller calls in this order.
    pub(crate) fn try_disable_pair_grab_collisions(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::error;
        self.try_with_offboard_pair(board, skeleton, owner, |normal| {
                normal.disable_hand_collisions(5);
                normal.disable_lower_leg_collisions(5);
                Ok(())
            })
            .map_err(error)?
    }

    /// Same retained SkeletonIK gets left then right; a failed right clamp
    /// preserves the completed left setter, as in the native Update.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_set_pair_grab_hand_targets(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        left: SkeletonRootVector,
        right: SkeletonRootVector,
        cvars: &InputCvarReferences,
        blend: f32,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), crate::physical_player_grind_manager::view::GrindError> {
        use crate::physical_player_grind_manager::view::error;
        self.try_with_offboard_pair(board, skeleton, owner, |normal| {
                let max_arm_length = cvars
                    .try_pushing_float(
                        crate::input_cvar_runtime::pushing::PushingFloat::kPushingMaxArmLengthForIK,
                    )
                    .map_err(error)?;
                normal
                    .try_set_world_hand_ik_target(true, left, max_arm_length, blend, numeric)
                    .map_err(error)?;
                let max_arm_length = cvars
                    .try_pushing_float(
                        crate::input_cvar_runtime::pushing::PushingFloat::kPushingMaxArmLengthForIK,
                    )
                    .map_err(error)?;
                normal
                    .try_set_world_hand_ik_target(false, right, max_arm_length, blend, numeric)
                    .map_err(error)
            })
            .map_err(error)?
    }

    /// Native uses height0, correction0, minimum velocity0, reorient=true.
    /// It publishes the root, optionally applies held-board animation, calls
    /// GeneralUpdate, then resets NextTrajectory. There is no UpdateGrindStates
    /// call in this state (unlike OffBoardLocomotion/Air).
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_pair_offboard_pushing_skeleton(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        motion: SkeletonRootVector,
        turn: f32,
        cvars: &InputCvarReferences,
        ground: &StaticGroundPool,
        numeric: &mut HostReckoningNumeric,
        sin_cos: &mut HostReckoningSinCosNumeric,
    ) -> Result<(), E> {
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        self.try_begin_pair_offboard_skeleton_update(board, skeleton, owner)
            .map_err(E::SkeletonPair)?;
        self.try_set_pair_offboard_skate_root(owner, board, skeleton, &processed, cvars)?;
        let trajectory = pushing_trajectory(
            self.skeleton(skeleton)
                .map_err(E::Pool)?
                .root_transforms()
                .world_to_animation(),
            motion,
        );
        self.try_update_pair_generic_offboard_roots(
            owner,
            board,
            skeleton,
            &processed,
            cvars,
            SkeletonRootVector::known([0.0; 4]),
            0.0,
            turn,
            SkeletonRootVector::known([0.0; 4]),
            true,
            Some(trajectory),
        )?;
        let root = *self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .root_transforms()
            .animation_to_world();
        if root.has_nan_xyz() {
            return Err(E::CallerRootNaN);
        }
        let mapped = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .offboard_volume_root_row()
            .ok_or(E::MissingOffboardVolumeRoot)?;
        let orientation = root.compose(&mapped);
        if orientation.has_nan_xyz() {
            return Err(E::ComposedVolumeRootNaN);
        }
        let flags = {
            let mut p = processed.lock().map_err(|_| E::Poisoned)?;
            GroundedProcessed::new(&mut p)
                .publish_offboard_orientation(orientation)
                .map_err(E::PublishOffboardOrientation)?;
            p.physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?
        };
        if flags & 0x40000 != 0 {
            self.try_apply_pair_offboard_board_animation(
                owner, board, skeleton, &processed, numeric, sin_cos,
            )
            .map_err(E::ApplyBoardAnimation)?;
        }
        self.try_update_pair_offboard_general(
            owner, board, skeleton, &processed, ground, cvars, numeric, sin_cos,
        )
        .map_err(E::GeneralUpdate)?;
        self.try_finish_pair_offboard_next_trajectory(board, skeleton, owner)
            .map_err(E::NextTrajectory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_displacement_uses_inverse_axes_without_city_translation() {
        let inverse = SkeletonRootMatrix::from_columns([
            [0.0, 0.0, 1.0, 0.0].into(),
            [0.0, 1.0, 0.0, 0.0].into(),
            [-1.0, 0.0, 0.0, 0.0].into(),
            [3500.0, 12.0, -1900.0, 0.0].into(),
        ]);
        let trajectory = pushing_trajectory(
            &inverse,
            SkeletonRootVector {
                xyz: [2.0, 3.0, 4.0],
                w: None,
            },
        );
        assert_eq!(trajectory.columns()[3].xyz, [-4.0, 3.0, 2.0]);
        assert_eq!(trajectory.columns()[3].w, Some(0.0));
        assert_eq!(trajectory.columns()[0].xyz, [1.0, 0.0, 0.0]);
        assert_eq!(trajectory.columns()[1].xyz, [0.0, 1.0, 0.0]);
        assert_eq!(trajectory.columns()[2].xyz, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn unknown_axis_lane_is_retained_even_with_zero_motion() {
        let mut inverse = SkeletonRootMatrix::translation([99.0, 98.0, 97.0, 0.0].into());
        inverse.set_column(
            1,
            SkeletonRootVector {
                xyz: [0.0, 1.0, 0.0],
                w: None,
            },
        );
        let result = pushing_trajectory(&inverse, [0.0; 4].into());
        assert_eq!(result.columns()[3].xyz, [0.0; 3]);
        assert_eq!(result.columns()[3].w, None);
    }
}
