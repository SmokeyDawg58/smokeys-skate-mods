//! Actual `UpdateFixedWorldPositionOfBone82b5d588`, its root update
//! `UpdateRootTransformsFixedPart82b42110` and the shared `GeneralUpdate`.
use super::{
    AssemblyPartMaterial, GroundedInputError, ReckoningNumeric, ReckoningSinCosNumeric,
    SimulationHookAssemblyPool, SkateboardSimulationBackend, SkeletonGeneralCvars,
    SkeletonGroundedProgress, SkeletonGroundedUpdateError, SkeletonNormalAssembly,
    SkeletonRootMatrix,
};
use crate::{
    dynamic_assembly_session::hook_pool::SimulationHookAssemblyHandle,
    physical_player_state_changer::attributes::StateChangerAttributes,
    skateboard_steering::SkateboardSteeringState,
    skeleton_processed_contact::grounded_runtime::GroundedProcessed,
    skeleton_root_transforms::{SkeletonRootVector, fixed_part::FixedPartAssertion},
};

#[derive(Debug)]
pub(crate) enum FixedBoneSkeletonError<N, S, C> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pool(crate::dynamic_assembly_session::skater_pool::SimulationSkaterPoolError),
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    Roots(FixedPartAssertion),
    Processed(GroundedInputError),
    Board(super::known_air::KnownAirSkeletonError<N, S, C>),
    General(SkeletonGroundedUpdateError<N, S, C>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug, C: std::fmt::Debug> std::fmt::Display
    for FixedBoneSkeletonError<N, S, C>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(error) => write!(f, "fixed-bone Skeleton Missing: {error}"),
            Self::Poisoned => f.write_str("fixed-bone Skeleton: Poisoned"),
            Self::ForeignOwner => f.write_str("fixed-bone Skeleton: ForeignOwner"),
            Self::Pool(error) => write!(f, "fixed-bone Skeleton Pool: {error:?}"),
            Self::Pair(error) => write!(f, "fixed-bone Skeleton Pair: {error:?}"),
            Self::Roots(error) => write!(f, "fixed-bone Skeleton assertion: {error:?}"),
            Self::Processed(error) => write!(f, "fixed-bone Skeleton Processed: {error:?}"),
            Self::Board(error) => write!(f, "fixed-bone Skeleton board: {error}"),
            Self::General(error) => write!(f, "fixed-bone Skeleton General: {error:?}"),
        }
    }
}

impl SkeletonNormalAssembly {
    /// 0x82b42110 through its inverse. Source-default-true `kDebugUseRawBonePos`
    /// selects the live mapped volume translation (+3070 + 64*Part); otherwise
    /// `GetLastAnimVolumeTransform82b3d498` reads the saved AnimRecord.
    pub(crate) fn try_update_fixed_part_roots<N, S, C>(
        &mut self,
        reckoning: &SkeletonRootMatrix,
        position: SkeletonRootVector,
        part: usize,
        raw_bone: bool,
    ) -> Result<(), FixedBoneSkeletonError<N, S, C>> {
        use FixedBoneSkeletonError as E;
        let bone = if raw_bone {
            *self
                .animation_targets
                .volume_row(part)
                .ok_or(E::Missing("mapped volume Part"))?
        } else {
            *self
                .current_animation
                .record()
                .poses()
                .get(part)
                .ok_or(E::Missing("AnimRecord Part"))?
        };
        let part_position = SkeletonRootVector::known([bone[12], bone[13], bone[14], bone[15]]);
        self.root_transforms
            .try_update_fixed_part(reckoning, position, part_position)
            .map_err(E::Roots)
    }

    /// `Skeleton::DisableLeftFootCollisions`/`DisableRightFootCollisions` on
    /// the retained Body counters and Volume headers.
    pub(crate) fn disable_foot_collisions(&mut self, right: bool, frames: i32) {
        self.state.disable_foot_collisions(right, frames);
    }

    /// 0x82b5d5b0..da1c after the root update and the caller's trajectory and
    /// root-pose identity stores: AnimToWorld times AnimRecord Part0 (not the
    /// mapped volume) to Processed with its flag, fast `ApplyBoardAnimation`,
    /// that result as both physics-skateboard and skate-root matrices,
    /// `GeneralUpdate`, then the identity next trajectory.
    #[expect(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn try_finish_fixed_bone_complete<N, S, H, C>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        hooks: &mut SimulationHookAssemblyPool<AssemblyPartMaterial, H>,
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
        let record = self.current_animation.record().poses()[0];
        let orientation = self
            .root_transforms
            .animation_to_world()
            .compose(&SkeletonRootMatrix::known(record));
        if orientation.has_nan_xyz() {
            return Err(E::Roots(FixedPartAssertion::Orientation));
        }
        // 0x82b5d950..968: four orientation rows, then the 8c4 animated bit.
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
