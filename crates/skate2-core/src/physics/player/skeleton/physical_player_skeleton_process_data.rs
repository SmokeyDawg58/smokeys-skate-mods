//! Actual same-pair Skeleton ProcessData82b5ecc8..f348 scheduling.
//! Registered below the skater pool's Skeleton member child to borrow its constructor
//! and its continuing Normal assembly together, without moving either owner.
use super::{
    MemberContent, SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool,
    SimulationSkaterPoolError, SkateboardSimulationBackend,
};
use crate::{
    actor_construction::anim_out_phys_in::AnimOutPhysInOwner,
    dynamic_assembly_session::{
        part_activation::DynamicAssemblyActivationError,
        skater_pool::{LiveBoard, SimulationSkeletonJumpPairError, SkeletonRootBoardAccessError},
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootBoard,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum ProcessDataPrefixError<N> {
    Owner(crate::simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Pair(SimulationSkeletonJumpPairError),
    Missing(&'static str),
    ForeignOwner,
    Poisoned,
    Velocity(SkeletonRootBoardAccessError),
    Activate(DynamicAssemblyActivationError<SimulationRigidBodyPoolError>),
    Attributes(crate::simulation_assembly_backend::constraint_session::physical_skeleton_anim_attributes::SkeletonAnimAttributesError<N>),
    Trajectory(crate::dynamic_assembly_session::skeleton_normal::process_data_prefix::TrajectoryError<N>),
    Continuation(crate::dynamic_assembly_session::skeleton_normal::process_data_output::ProcessDataContinuationError<crate::input_cvar_runtime::current_animation::CurrentAnimationCvarError>),
}
impl<N: std::fmt::Debug> std::fmt::Display for ProcessDataPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "ProcessData physical owner: {error}"),
            Self::Pool(error) => write!(f, "ProcessData pair: {error}"),
            Self::Pair(error) => write!(f, "ProcessData pair: {error}"),
            Self::Missing(field) => write!(f, "ProcessData missing {field}"),
            Self::ForeignOwner => f.write_str("ProcessData owners differ"),
            Self::Poisoned => f.write_str("ProcessData owner lock poisoned"),
            Self::Velocity(error) => write!(f, "ProcessData velocity: {error}"),
            Self::Activate(error) => write!(f, "ProcessData activation: {error}"),
            Self::Attributes(error) => write!(f, "ProcessData attributes: {error:?}"),
            Self::Trajectory(error) => write!(f, "ProcessData trajectory: {error}"),
            Self::Continuation(error) => write!(f, "ProcessData continuation: {error}"),
        }
    }
}

impl SimulationSkaterAssemblyPool {
    #[expect(
        clippy::too_many_arguments,
        reason = "same physical pair and three native pointer identities"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "source-ordered same-pair ProcessData stages retain preceding writes on failure"
    )]
    pub(crate) fn try_process_paired_skeleton_data_prefix<N: ReckoningNumeric>(
        &mut self,
        backend: &mut SkateboardSimulationBackend,
        board_handle: &SimulationSkaterAssemblyHandle,
        skeleton_handle: &SimulationSkaterAssemblyHandle,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        input: &Arc<Mutex<AnimOutPhysInOwner>>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        debug_overlay: &crate::debug_overlay::DebugOverlaySlot,
        numeric: &mut N,
    ) -> Result<(), ProcessDataPrefixError<N::Error>> {
        use ProcessDataPrefixError as E;
        // Existing pair helper validates kinds, manager, attachment and owner.
        self.try_with_jump_pair(backend, board_handle, skeleton_handle, |_, owner| {
            Arc::ptr_eq(owner, expected_owner)
        })
        .map_err(E::Pair)?
        .then_some(())
        .ok_or(E::ForeignOwner)?;
        let skeleton_index = self
            .skeleton_mutation_index(backend, skeleton_handle)
            .map_err(E::Pool)?;
        let board_index = self.member_index(board_handle).map_err(E::Pool)?;
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
            .as_deref_mut()
            .ok_or(E::Missing("retained constructor"))?;
        let constructor = &mut construction.constructor;
        let bindings = constructor
            .bindings
            .as_mut()
            .ok_or(E::Missing("constructor bindings"))?;
        if !Arc::ptr_eq(&bindings.processed, processed) {
            return Err(E::ForeignOwner);
        }
        let actual_input = expected_owner
            .animation_physics_input
            .lock()
            .map_err(|_| E::Poisoned)?;
        if !actual_input
            .as_ref()
            .is_some_and(|actual| Arc::ptr_eq(actual, input))
        {
            return Err(E::ForeignOwner);
        }
        drop(actual_input);
        bindings.animation_input = Some(Arc::clone(input)); //ece8
        let input = input.lock().map_err(|_| E::Poisoned)?;
        let outer = constructor
            .outer
            .as_mut()
            .ok_or(E::Missing("constructor remainder"))?;
        outer.time_delta = input.time_delta; //ecf0
        outer.skateboard_velocity = LiveBoard {
            session: board.session(),
            backend,
        }
        .velocity()
        .map_err(E::Velocity)?; //ed28
        skeleton
            .owner
            .session()
            .try_activate(backend)
            .map_err(E::Activate)?; //ed34
        let mut processed = processed.lock().map_err(|_| E::Poisoned)?;
        let mut anim_out = expected_owner.anim_out().lock().map_err(|_| E::Poisoned)?;
        let anim_out = anim_out.as_mut().ok_or(E::Missing("physical AnimOut"))?;
        crate::simulation_assembly_backend::constraint_session::physical_skeleton_anim_attributes::try_process_skeleton_anim_attributes(
            &mut skeleton.owner, constructor, &input, &mut processed, anim_out, expected_owner, debug_overlay, numeric,
        ).map_err(E::Attributes)?; //ed44
        let processed_dt = processed.physics_state_fields().time_delta;
        skeleton
            .owner
            .try_process_data_trajectory(
                constructor,
                &input,
                processed_dt,
                &expected_owner.inputs().cvars,
                numeric,
            )
            .map_err(E::Trajectory)?;
        let mut cvars = crate::input_cvar_runtime::current_animation::CurrentAnimationCvars::new(
            &expected_owner.inputs().cvars,
            debug_overlay.clone(),
        );
        skeleton
            .owner
            .try_process_data_continuation(
                constructor,
                &input,
                &mut processed,
                expected_owner,
                &mut cvars,
            )
            .map_err(E::Continuation)
    }
}
