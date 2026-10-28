//! Actual `Skeleton::UpdateHandPlant82b5da30` (FootPlant Update), its root
//! update `UpdateRootTransformsHandPlant82b43880` and the shared `GeneralUpdate`.
use super::live_input::GroundedInputRead;
use super::{
    AssemblyPartMaterial, ReckoningNumeric, ReckoningSinCosNumeric, SimulationHookAssemblyPool,
    SkateboardAssemblySession, SkateboardSimulationBackend, SkeletonGeneralCvars,
    SkeletonGroundedProgress, SkeletonNormalAssembly, SkeletonRootMatrix,
    fixed_bone::FixedBoneSkeletonError, known_air::KnownAirSkeletonError,
};
use crate::{
    dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
    physical_player_state_changer::attributes::StateChangerAttributes,
    skateboard_steering::SkateboardSteeringState,
    skeleton_current_animation::SkeletonIkUnconstructed,
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::{SkeletonRootVector, fixed_part::FixedPartAssertion},
};

impl SkeletonNormalAssembly {
    /// 0x82b43880 with the current AnimRecord COM (Skeleton+2a00).
    pub(crate) fn try_update_hand_plant_roots<N, S, C>(
        &mut self,
        reckoning: &SkeletonRootMatrix,
        position: SkeletonRootVector,
    ) -> Result<(), FixedBoneSkeletonError<N, S, C>> {
        let centre = self.current_animation.record().centre_of_mass();
        self.root_transforms
            .try_update_hand_plant(reckoning, position, centre)
            .map_err(FixedBoneSkeletonError::Roots)
    }

    /// `SkeletonController::DisableFeetCollision` 0x82dd4778 on the retained Body.
    pub(crate) fn disable_feet_collisions(&mut self) {
        self.state.disable_feet_collisions();
    }

    /// `SkeletonController::EnableFeetCollision` 0x82dd4780 on the retained Body.
    pub(crate) fn try_enable_feet_collisions(
        &mut self,
    ) -> Result<(), crate::skeleton_runtime_state::SkeletonRuntimeStateError> {
        self.state.try_enable_feet_collisions()
    }

    /// `Skeleton::SetLeft/RightFootIKTargetPositionAnim` 0x82b41290/0x82b412b0:
    /// the animation-space vector is stored unchanged at IK+0x260/0x270, then
    /// the local-set byte and blend. No world-to-animation transform (unlike
    /// the Local setters 0x82b412f0/0x82b41348).
    pub(crate) fn try_set_anim_foot_ik_target(
        &mut self,
        left: bool,
        position: SkeletonRootVector,
        blend: f32,
    ) -> Result<(), SkeletonIkUnconstructed> {
        self.current_animation
            .ik_runtime_mut()
            .ok_or(SkeletonIkUnconstructed)?
            .set_local_foot_target(left, position, blend);
        Ok(())
    }

    /// `GetAnimPartTransformWorldRaw` 0x82b45f58 translation: the saved
    /// AnimRecord volume (`GetLastAnimVolumeTransform82b3d498`) composed with
    /// AnimToWorld, never the physics history `GetPartTransform` may select.
    pub(crate) fn anim_part_world_raw_position(&self, part: usize) -> Option<SkeletonRootVector> {
        let volume = self.current_animation.record().poses().get(part)?;
        Some(
            self.root_transforms
                .animation_to_world()
                .compose(&SkeletonRootMatrix::known(*volume))
                .columns()[3],
        )
    }

    /// `GetAnimCentreOfMass` 0x82b3b978: AnimToWorld transforms the current
    /// AnimRecord COM (X FMA onto the translation, then Y and Z FMAs).
    pub(crate) fn anim_centre_of_mass(&self) -> SkeletonRootVector {
        crate::skeleton_ik_state::external::transform_point(
            self.root_transforms.animation_to_world(),
            SkeletonRootVector::known(self.current_animation.record().centre_of_mass()),
        )
    }

    /// 0x82b5da54..dee0 after the roots and the caller's trajectory and
    /// root-pose identity stores: the AnimToWorld assertion 0xB50, AnimToWorld
    /// times the MAPPED volume Part0 (Skeleton+3040, not the AnimRecord the
    /// fixed-bone update uses) with assertion 0xB53, the four Processed rows
    /// then the 8c4 animated bit, fast `ApplyBoardAnimation` published as the
    /// physics-skateboard matrix, `TargetPositionWithVelocity` on its
    /// translation, the same matrix as skate root, `GeneralUpdate`, then the
    /// identity next trajectory.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_finish_hand_plant_complete<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
        board: &mut SkateboardAssemblySession,
        board_hook: Option<&SimulationHookAssemblyHandle>,
        steering: &mut SkateboardSteeringState,
        air_attributes: &StateChangerAttributes,
        root_dt: f32,
        processed: &GroundedProcessed<'_>,
        numeric: &mut N,
        rounding: &mut S,
        cvars: &mut C,
    ) -> Result<
        SkeletonGroundedProgress,
        FixedBoneSkeletonError<
            N::Error,
            S::Error,
            <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
        >,
    >
    where
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        C: SkeletonGeneralCvars
            + crate::skeleton_ik::SkeletonIkSuffixServices
            + crate::skeleton_ik_state::contact::SkeletonIkContactServices<
                Error = <C as crate::skeleton_drives::update::SkeletonDrivesUpdateCvars>::Error,
            >,
    {
        use FixedBoneSkeletonError as E;
        if self.root_transforms.animation_to_world().has_nan_xyz() {
            return Err(E::Roots(FixedPartAssertion::AnimationToWorld));
        }
        let mapped = self
            .animation_targets
            .volume_row(0)
            .ok_or(E::Missing("mapped volume zero"))?;
        let orientation = self
            .root_transforms
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(*mapped));
        if orientation.has_nan_xyz() {
            return Err(E::Roots(FixedPartAssertion::Orientation));
        }
        // 0x82b5ddf4..e0c: four orientation rows, then the 8c4 animated bit.
        processed
            .publish_offboard_orientation(orientation)
            .map_err(E::Processed)?;
        let applied = self
            .try_apply_offboard_board_animation(
                backend,
                hooks,
                board_hook,
                steering,
                air_attributes,
                processed,
                numeric,
                rounding,
            )
            .map_err(E::Board)?;
        // 0x82b5de5c..e74 passes PhysicsSkateboardMtx.wAxis (Skeleton+2ff0).
        let target = applied.columns()[3];
        crate::skateboard_set_velocity::try_target_position_with_velocity(
            board,
            backend,
            processed.time_delta().map_err(E::Processed)?,
            &target,
        )
        .map_err(|e| match e {
            crate::skateboard_set_velocity::TargetPositionError::Transform(e) => {
                E::Board(KnownAirSkeletonError::Part(e))
            }
            crate::skateboard_set_velocity::TargetPositionError::Velocity(e) => {
                E::Board(KnownAirSkeletonError::Velocity(e))
            }
        })?;
        self.general_update.set_animated_skate_root(applied);
        let root_input = self
            .current_animation
            .root_input(root_dt)
            .ok_or(E::Missing("current mapped animation"))?;
        let prefix = self
            .try_update_general_prefix(
                backend,
                hooks,
                processed,
                &root_input.skate_animation,
                numeric,
                rounding,
                cvars,
            )
            .map_err(E::General)?;
        let progress = self
            .try_finish_general_complete(
                backend, hooks, processed, processed, prefix, numeric, rounding, cvars,
            )
            .map_err(E::General)?;
        self.general_update.finish_grounded_trajectory();
        Ok(progress)
    }
}
