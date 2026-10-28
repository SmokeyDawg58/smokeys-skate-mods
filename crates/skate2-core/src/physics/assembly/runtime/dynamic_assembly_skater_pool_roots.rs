//! Actual paired-owner access for the grounded root-transform prefix.
//! Native `Body::GetPosition` (0x82de7b20) reaches the live Part getter;
//! `UpdateRootTransforms` samples board Part6 transform and, conditionally,
//! velocity at distinct points. No serialized publication is a Part lookup.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    animation::AnimationMatrix,
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        part_transform::DynamicAssemblyPartTransformError,
    },
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_assembly::SkateboardAssemblySession,
    skateboard_spawn_math::SkateboardSpawnMathNumeric,
    skeleton_root_transforms::{
        SkeletonRootBoard, SkeletonRootError, SkeletonRootInput, SkeletonRootVector,
    },
};
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum SkeletonRootBoardAccessError {
    Transform(DynamicAssemblyPartTransformError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
}
impl fmt::Display for SkeletonRootBoardAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transform(error) => write!(f, "paired board root transform: {error}"),
            Self::Part(error) => write!(f, "paired board root Part: {error}"),
            Self::Body(error) => write!(f, "paired board root velocity: {error}"),
        }
    }
}
impl std::error::Error for SkeletonRootBoardAccessError {}

#[derive(Debug)]
pub(crate) enum SimulationSkeletonRootUpdateError<N, S> {
    Pool(SimulationSkaterPoolError),
    DifferentPhysicalOwner,
    Detached,
    Update(SkeletonRootError<SkeletonRootBoardAccessError, N, S>),
}
impl<N: fmt::Debug, S: fmt::Debug> fmt::Display for SimulationSkeletonRootUpdateError<N, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "paired skeleton root pool: {error}"),
            Self::DifferentPhysicalOwner => {
                f.write_str("root board and skeleton belong to different players")
            }
            Self::Detached => f.write_str("root update requires attached board and skeleton"),
            Self::Update(error) => write!(f, "{error}"),
        }
    }
}
impl<N: fmt::Debug, S: fmt::Debug> std::error::Error for SimulationSkeletonRootUpdateError<N, S> {}

pub(crate) struct LiveBoard<'a> {
    pub(crate) session: &'a SkateboardAssemblySession,
    pub(crate) backend: &'a SkateboardSimulationBackend,
}
impl SkeletonRootBoard for LiveBoard<'_> {
    type Error = SkeletonRootBoardAccessError;
    fn transform(&mut self) -> Result<AnimationMatrix, Self::Error> {
        self.session
            .try_live_part_transform_host(self.backend, 6)
            .map_err(Self::Error::Transform)
    }
    fn transform_typed(
        &mut self,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootMatrix, Self::Error> {
        self.session
            .try_live_part_transform_typed(self.backend, 6)
            .map_err(Self::Error::Transform)
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        // Resolve and revalidate the actual Part occurrence at this later
        // source read. The checked API does not require or invent a spy flag.
        let observation = self
            .session
            .try_part_contact_spy_observation(self.backend, 6)
            .map_err(Self::Error::Part)?;
        let xyz = self
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(observation)
            .map_err(Self::Error::Body)?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
}

impl SimulationSkaterAssemblyPool {
    /// Mutates only the SAME Skeleton's retained root state. This does not
    /// move Parts, update drives or invalidate their collision publication.
    /// Reckoning and current animation inputs are explicit caller projections,
    /// not reconstructed `PhysicalPlayerHiLOD` scheduling or ownership.
    #[expect(
        clippy::too_many_arguments,
        reason = "distinct source collaborators and checked pair"
    )]
    pub(crate) fn try_update_paired_skeleton_roots<A, N, S, M>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        board: &SimulationSkaterAssemblyHandle,
        input: &SkeletonRootInput,
        reckoning: &Reckoning<A>,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> Result<(), SimulationSkeletonRootUpdateError<N::Error, S::Error>>
    where
        A: ReckoningAttributeRead,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        use super::super::physical::SimulationPhysicalBodyKind;
        use SimulationSkeletonRootUpdateError as Error;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(Error::Pool)?;
        let board_index = self.member_index(board).map_err(Error::Pool)?;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board)
            .map_err(Error::Pool)?;
        if kind != SimulationPhysicalBodyKind::Board {
            return Err(Error::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, skeleton_owner) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(Error::Pool)?;
        if !Arc::ptr_eq(board_owner, skeleton_owner) {
            return Err(Error::DifferentPhysicalOwner);
        }
        if board_owner.manager() != self.manager {
            return Err(Error::Pool(SimulationSkaterPoolError::ForeignManager));
        }
        let (board_member, skeleton_member) = if board_index < skeleton_index {
            let (before, after) = self.members.split_at_mut(skeleton_index);
            (&before[board_index], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(board_index);
            (&after[0], &mut before[skeleton_index])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &board_member.content
        else {
            unreachable!("board kind checked")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton_member.content
        else {
            unreachable!("skeleton kind checked")
        };
        if !board.session().is_attached() || !skeleton.owner().session().is_attached() {
            return Err(Error::Detached);
        }
        // Borrowed read collaborators cannot replace or mutate either session.
        // After this point native write prefixes remain visible on errors.
        let mut board = LiveBoard {
            session: board.session(),
            backend,
        };
        skeleton
            .owner
            .root_transforms_mut()
            .try_update_grounded(input, reckoning, &mut board, numeric, sin_cos, ortho)
            .map_err(Error::Update)
    }
}
