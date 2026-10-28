//! Retained world-pair binding for `Skeleton::UpdatePoseBuffer82b4d970`.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::post_wipeout_pose_live::{
        PosePublicationError,
        board_visuals::{PoseBoardError, PoseBoardOwners},
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum PosePoolError<N, S> {
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    Missing(&'static str),
    Poisoned,
    Records(PosePublicationError),
    Board(PoseBoardError<N, S>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for PosePoolError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::ForeignOwner => f.write_str("pose publication owner mismatch"),
            Self::Missing(v) => write!(f, "pose publication missing {v}"),
            Self::Poisoned => f.write_str("pose publication owner poisoned"),
            Self::Records(e) => e.fmt(f),
            Self::Board(e) => e.fmt(f),
        }
    }
}
impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_lines,
        reason = "same-pair validation and ordered publication under one pose lock"
    )]
    pub(crate) fn try_publish_skeleton_pose<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board_handle: &SimulationSkaterAssemblyHandle,
        skeleton_handle: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
        sin_cos: &mut S,
    ) -> Result<(), PosePoolError<N::Error, S::Error>> {
        use PosePoolError as E;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton_handle)
            .map_err(E::Pool)?;
        let board_index = self.member_index(board_handle).map_err(E::Pool)?;
        let (kind, board_owner) = self
            .try_physical_member_owner(backend, board_handle)
            .map_err(E::Pool)?;
        if kind != super::super::physical::SimulationPhysicalBodyKind::Board {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, skeleton_owner) = self
            .try_physical_member_owner(backend, skeleton_handle)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(board_owner, expected_owner) || !Arc::ptr_eq(skeleton_owner, expected_owner)
        {
            return Err(E::ForeignOwner);
        }
        let (board, skeleton) = if board_index < skeleton_index {
            let (before, after) = self.members.split_at_mut(skeleton_index);
            (&before[board_index], &mut after[0])
        } else {
            let (before, after) = self.members.split_at_mut(board_index);
            (&after[0], &mut before[skeleton_index])
        };
        let MemberContent::Board {
            candidate: board, ..
        } = &board.content
        else {
            unreachable!("checked board")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton.content
        else {
            unreachable!("checked skeleton")
        };
        let construction = skeleton
            .construction
            .as_ref()
            .ok_or(E::Missing("Skeleton construction"))?;
        let bindings = construction
            .constructor
            .bindings
            .as_ref()
            .ok_or(E::Missing("constructor bindings"))?;
        if bindings
            .owner
            .upgrade()
            .is_none_or(|owner| !Arc::ptr_eq(&owner, expected_owner))
        {
            return Err(E::ForeignOwner);
        }
        let input = bindings
            .animation_input
            .as_ref()
            .ok_or(E::Missing("ProcessData animation input"))?;
        let actual = expected_owner
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)?;
        if actual
            .as_ref()
            .is_none_or(|owner| !Arc::ptr_eq(owner, input))
        {
            return Err(E::ForeignOwner);
        }
        drop(actual);
        let mut pose = input.lock().map_err(|_| E::Poisoned)?;
        let before = skeleton
            .owner
            .try_post_wipeout_pose_records(&mut pose, board.session(), backend)
            .map_err(E::Records)?;
        // Resolve the tail's actual retained providers after the native pose loops.
        let global = construction
            .body_reader()
            .ok_or(E::Missing("global body reader"))?
            .global_skeleton_instance();
        skeleton
            .owner
            .try_post_wipeout_pose_board(
                before,
                PoseBoardOwners {
                    pose: &mut pose,
                    board: board.session(),
                    backend,
                    board_construction: board
                        .construction
                        .as_deref()
                        .ok_or(E::Missing("board construction"))?,
                    trucks: board
                        .truck_drives
                        .as_ref()
                        .ok_or(E::Missing("truck drives"))?,
                    constructor: &construction.constructor,
                    physics_skeleton: global,
                },
                numeric,
                sin_cos,
            )
            .map_err(E::Board)
    }
}
