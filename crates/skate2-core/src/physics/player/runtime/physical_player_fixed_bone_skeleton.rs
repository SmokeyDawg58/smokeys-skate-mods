//! Live Boneless.Update82da0d4c -> Skeleton.UpdateFixedWorldPositionOfBone82b5d588.
use crate::{
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkeletonOffboardTrajectoryResetError,
        },
        skeleton_normal::general_owner::{
            SkeletonGroundedProgress, fixed_bone::FixedBoneSkeletonError,
        },
    },
    input_cvar_runtime::{
        InputCvarError, InputCvarReferences,
        ground_skateboard::{GroundSkateboardCvarError, GroundSkateboardCvars},
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
type CvarError = super::GroundedWorldServiceError<GroundSkateboardCvarError>;

#[derive(Debug)]
pub(crate) enum FixedBoneUpdateError<N, S> {
    Skeleton(FixedBoneSkeletonError<N, S, CvarError>),
    Trajectory(SimulationSkeletonOffboardTrajectoryResetError),
    Cvar(InputCvarError),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for FixedBoneUpdateError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skeleton(error) => write!(f, "{error}"),
            Self::Trajectory(error) => write!(f, "fixed-bone trajectory reset: {error:?}"),
            Self::Cvar(error) => write!(f, "fixed-bone cvar: {error:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    /// The root update, then its trajectory and root-pose identity stores
    /// 0x82b4283c..8c8 through the same retained bridge OffBoard uses, then the
    /// remaining Skeleton update and `GeneralUpdate`.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair and independent numeric services"
    )]
    pub(crate) fn try_update_pair_fixed_bone_skeleton<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        position: SkeletonRootVector,
        part: usize,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut N,
        rounding: &mut S,
    ) -> Result<SkeletonGroundedProgress, FixedBoneUpdateError<N::Error, S::Error>> {
        use FixedBoneSkeletonError as S2;
        use FixedBoneUpdateError as E;
        // 0x82b426f0: live `kDebugUseRawBonePos`; a pure read before the roots.
        let raw_bone = refs.try_debug_use_raw_bone_pos().map_err(E::Cvar)?;
        let transform = *owner
            .reckoning()
            .lock()
            .map_err(|_| E::Skeleton(S2::Poisoned))?
            .reckoning()
            .ok_or(E::Skeleton(S2::Missing("Reckoning")))?
            .transform();
        self.skaters
            .as_mut()
            .ok_or(E::Skeleton(S2::Missing("skater pool")))?
            .try_update_paired_fixed_part_roots(
                &self.backend,
                skeleton,
                board,
                owner,
                &transform,
                position,
                part,
                raw_bone,
            )
            .map_err(E::Skeleton)?;
        self.try_reset_offboard_trajectory_and_root_poses(skeleton, owner)
            .map_err(E::Trajectory)?;
        let state_changer = owner
            .state_changer()
            .lock()
            .map_err(|_| E::Skeleton(S2::Poisoned))?;
        let air_attributes = state_changer
            .attributes
            .as_ref()
            .ok_or(E::Skeleton(S2::Missing("retained global Air attributes")))?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Skeleton(S2::Poisoned))?
                .processed(),
        );
        let mut cvars = GroundSkateboardCvars::new(refs, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Skeleton(S2::Missing("skater pool")))?
            .try_update_paired_skeleton_fixed_bone(
                &mut self.backend,
                &mut self.hooks,
                skeleton,
                board,
                owner,
                &processed,
                air_attributes,
                numeric,
                rounding,
                &mut services,
            )
            .map_err(E::Skeleton)
    }
}
