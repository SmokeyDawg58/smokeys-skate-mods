//! Live FootPlant.Update82d976f0 -> Skeleton.UpdateHandPlant82b5da30.
use super::fixed_bone::FixedBoneUpdateError;
use crate::{
    dynamic_assembly_session::{
        skater_pool::SimulationSkaterAssemblyHandle,
        skeleton_normal::general_owner::{
            SkeletonGroundedProgress, fixed_bone::FixedBoneSkeletonError,
        },
    },
    input_cvar_runtime::{InputCvarReferences, ground_skateboard::GroundSkateboardCvars},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

impl PhysicalAssemblyWorld {
    /// The hand-plant root update at `position` (the ground output COM), its
    /// trajectory and root-pose identity stores 0x82b44028..0b4 through the
    /// same retained bridge the fixed-bone and OffBoard updates use, then the
    /// remaining Skeleton update and `GeneralUpdate`.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair and independent numeric services"
    )]
    pub(crate) fn try_update_pair_hand_plant_skeleton<
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        position: SkeletonRootVector,
        ground: &StaticGroundPool,
        refs: &InputCvarReferences,
        numeric: &mut N,
        rounding: &mut S,
    ) -> Result<SkeletonGroundedProgress, FixedBoneUpdateError<N::Error, S::Error>> {
        use FixedBoneSkeletonError as S2;
        use FixedBoneUpdateError as E;
        // 0x82b43a34: Reckoning+60, the retained transform, read by the roots.
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
            .try_update_paired_hand_plant_roots(
                &self.backend,
                skeleton,
                board,
                owner,
                &transform,
                position,
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
            .try_update_paired_skeleton_hand_plant(
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
