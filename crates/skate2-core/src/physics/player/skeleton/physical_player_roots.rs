//! Bounded grounded root update on the actual registered pair. Current
//! Reckoning/animation inputs remain external projections, not a fabricated
//! `ProcessedPhysIn`, `UpdateGroundSkateboard` or `GeneralUpdate` owner.
use super::{SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        SimulationSkeletonRootUpdateError,
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
    skeleton_root_transforms::SkeletonRootInput,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::fmt;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerRootError<N, S> {
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Update(SimulationSkeletonRootUpdateError<N, S>),
    AnimationNotCurrent,
}
impl<N: fmt::Debug, S: fmt::Debug> fmt::Display for PhysicalPlayerRootError<N, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "physical player root owner: {error}"),
            Self::Pool(error) => write!(f, "physical player root pool: {error}"),
            Self::Update(error) => write!(f, "{error}"),
            Self::AnimationNotCurrent => {
                f.write_str("current Skeleton animation preparation has not completed")
            }
        }
    }
}
impl<N: fmt::Debug, S: fmt::Debug> std::error::Error for PhysicalPlayerRootError<N, S> {}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Consume the SAME owner's successful current pose projection. This
    /// closes the bounded producer/root connection, not full `GeneralUpdate`.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair and separate original numerical services"
    )]
    pub(crate) fn try_update_skeleton_roots_from_current_animation<A, N, S, M>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        time_delta: f32,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> Result<(), PhysicalPlayerRootError<N::Error, S::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        let input = self
            .skeleton(skeleton)
            .map_err(PhysicalPlayerRootError::Pool)?
            .current_animation()
            .root_input(time_delta)
            .ok_or(PhysicalPlayerRootError::AnimationNotCurrent)?;
        self.try_update_skeleton_roots_from_controlled_ground_input(
            skeleton, board, &input, reckoning, numeric, sin_cos, ortho,
        )
    }

    /// Source grounded heading-store/root-update prefix only. Both registered
    /// members and their shared construction owner must still be current.
    /// There is no invented contact-response prerequisite or once-per-solve
    /// restriction: the missing gameplay scheduler is an explicit boundary.
    /// Root-only writes do not mutate/invalidate body/drive/ground authorities.
    #[expect(
        clippy::too_many_arguments,
        reason = "actual pair and distinct source services"
    )]
    pub(crate) fn try_update_skeleton_roots_from_controlled_ground_input<A, N, S, M>(
        &mut self,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        input: &SkeletonRootInput,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> Result<(), PhysicalPlayerRootError<N::Error, S::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        use PhysicalPlayerRootError as Error;
        self.physical_body_owner_for_contacts(skeleton)
            .map_err(Error::Owner)?;
        self.physical_body_owner_for_contacts(board)
            .map_err(Error::Owner)?;
        self.skaters
            .as_mut()
            .ok_or(Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_update_paired_skeleton_roots(
                &self.backend,
                skeleton,
                board,
                input,
                reckoning,
                numeric,
                sin_cos,
                ortho,
            )
            .map_err(Error::Update)
    }
}
