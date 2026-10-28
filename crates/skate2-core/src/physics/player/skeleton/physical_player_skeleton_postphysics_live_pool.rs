//! Draft skater-pool child: source preparation/history and late input assembly.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skeleton_normal::{
        history_owner::{SkeletonHistoryInitialization, SkeletonHistoryUpdateError},
        postphysics_preparation::PreparationError,
    },
    input_cvar_runtime::{
        postphysics_history_input::{HistoryInputError, PostPhysicsHistoryInput},
        skeleton_postphysics_live::{SkeletonPostPhysicsCvars, SkeletonPostPhysicsServiceError},
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    skeleton_collision_data::update::SkeletonCollisionUpdateInput,
    skeleton_physics_history::HostSkeletonPhysicsHistoryNumeric,
    skeleton_processed_contact::{
        SkeletonProcessedContact, prior_output::postphysics_collision_inputs::CollisionInputError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum PostPhysicsPoolError<N> {
    Pool(SimulationSkaterPoolError),
    ForeignOwner,
    Missing(&'static str),
    AlreadyAttempted,
    Poisoned,
    Preparation(PreparationError<SkeletonPostPhysicsServiceError, N>),
    History(SkeletonHistoryUpdateError<HistoryInputError>),
    Input(CollisionInputError),
}
impl<N: std::fmt::Debug> std::fmt::Display for PostPhysicsPoolError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(e) => e.fmt(f),
            Self::ForeignOwner => f.write_str("Skeleton post-physics owner mismatch"),
            Self::Missing(s) => write!(f, "Skeleton post-physics missing {s}"),
            Self::AlreadyAttempted => {
                f.write_str("Skeleton history already attempted on this frame")
            }
            Self::Poisoned => f.write_str("Skeleton post-physics owner poisoned"),
            Self::Preparation(e) => e.fmt(f),
            Self::History(e) => e.fmt(f),
            Self::Input(e) => e.fmt(f),
        }
    }
}

/// Sole continuation from the current source history, not a copied pose/list.
pub(crate) struct BeforePostPhysicsCollision {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) processed: Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) frame: Arc<SimulationContactSpyFrame>,
    system_velocity: SkeletonRootVector,
    pub(crate) timestep: f32,
    request_partial_ragdoll: bool,
}

impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        reason = "actual paired owners, completed frame and native passed arguments"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one source-ordered caller over retained owners and frame checks"
    )]
    pub(crate) fn try_prepare_skeleton_postphysics_live<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board_handle: &SimulationSkaterAssemblyHandle,
        skeleton_handle: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        frame: Arc<SimulationContactSpyFrame>,
        timestep: f32,
        request_partial_ragdoll: bool,
        cvars: &mut SkeletonPostPhysicsCvars<'_>,
        history_input: &mut PostPhysicsHistoryInput<'_>,
        numeric: &mut N,
    ) -> Result<BeforePostPhysicsCollision, PostPhysicsPoolError<N::Error>> {
        use PostPhysicsPoolError as E;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton_handle)
            .map_err(E::Pool)?;
        let board_index = self.member_index(board_handle).map_err(E::Pool)?;
        let (kind, actual_board) = self
            .try_physical_member_owner(backend, board_handle)
            .map_err(E::Pool)?;
        if kind != super::super::physical::SimulationPhysicalBodyKind::Board {
            return Err(E::Pool(SimulationSkaterPoolError::WrongMemberKind));
        }
        let (_, actual_skeleton) = self
            .try_physical_member_owner(backend, skeleton_handle)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(actual_board, expected_owner)
            || !Arc::ptr_eq(actual_skeleton, expected_owner)
            || !Arc::ptr_eq(history_input.processed, processed)
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
            unreachable!("checked kind")
        };
        let MemberContent::Skeleton {
            candidate: skeleton,
            ..
        } = &mut skeleton.content
        else {
            unreachable!("checked kind")
        };
        let construction = skeleton
            .construction
            .as_mut()
            .ok_or(E::Missing("retained Skeleton construction"))?;
        let bindings = construction
            .constructor
            .bindings
            .as_ref()
            .ok_or(E::Missing("constructor bindings"))?;
        if !Arc::ptr_eq(&bindings.processed, processed)
            || bindings
                .owner
                .upgrade()
                .is_none_or(|owner| !Arc::ptr_eq(&owner, expected_owner))
        {
            return Err(E::ForeignOwner);
        }
        if skeleton.owner.physics_history_initialization()
            != SkeletonHistoryInitialization::Complete
        {
            return Err(E::Missing("completed initial history"));
        }
        // Host one-attempt bookkeeping starts before any source mutation.
        if !skeleton
            .owner
            .try_begin_physics_history_frame(Arc::clone(&frame))
        {
            return Err(E::AlreadyAttempted);
        }
        let (outer, attributes) = construction
            .postphysics_inputs_mut()
            .ok_or(E::Missing("outer and body attributes"))?;
        skeleton
            .owner
            .try_prepare_postphysics(backend, board.session(), outer, attributes, cvars, numeric)
            .map_err(E::Preparation)?;
        // Required narrow existing-method amendment returns its cc4c capture.
        // No duplicate kernel, board query, history snapshot or reinitialization.
        let captured_deck = skeleton
            .owner
            .try_update_physics_history_from_board(
                backend,
                board.session(),
                history_input,
                &mut HostSkeletonPhysicsHistoryNumeric,
            )
            .map_err(E::History)?;
        skeleton
            .owner
            .finish_postphysics_history_local(outer, captured_deck);
        skeleton.owner.complete_physics_history_frame();
        let system_velocity = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .capture_postphysics_system_velocity(skeleton.owner.physics_history(), outer)
            .map_err(E::Input)?;
        Ok(BeforePostPhysicsCollision {
            owner: Arc::clone(expected_owner),
            processed: Arc::clone(processed),
            frame,
            system_velocity,
            timestep,
            request_partial_ragdoll,
        })
    }

    /// Call after world `GetCollisions`. No input or Reckoning lock escapes this
    /// method, so later existing live post-record callbacks can reacquire them.
    pub(crate) fn try_read_skeleton_postphysics_collision_input<N>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: &BeforePostPhysicsCollision,
    ) -> Result<SkeletonCollisionUpdateInput, PostPhysicsPoolError<N>> {
        use PostPhysicsPoolError as E;
        let index = self
            .skeleton_mutation_index(backend, skeleton)
            .map_err(E::Pool)?;
        let (_, actual) = self
            .try_physical_member_owner(backend, skeleton)
            .map_err(E::Pool)?;
        if !Arc::ptr_eq(actual, &before.owner) {
            return Err(E::ForeignOwner);
        }
        let MemberContent::Skeleton { candidate, .. } = &self.members[index].content else {
            unreachable!("checked kind")
        };
        // Lock order matches existing physical callers: Reckoning then its
        // processed owner. Locks merely protect captures; payload reads remain
        // inside try_postphysics_collision_input in the source's order.
        let reckoning = before.owner.reckoning().lock().map_err(|_| E::Poisoned)?;
        if !Arc::ptr_eq(reckoning.processed(), &before.processed) {
            return Err(E::ForeignOwner);
        }
        let processed = before.processed.lock().map_err(|_| E::Poisoned)?;
        processed
            .try_postphysics_collision_input(
                before.system_velocity,
                reckoning.reckoning(),
                candidate.owner.post_physics_records(),
                before.timestep,
                before.request_partial_ragdoll,
            )
            .map_err(E::Input)
    }
}
