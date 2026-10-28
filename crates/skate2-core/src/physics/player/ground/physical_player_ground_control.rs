//! Ground.Update101c entry through its native truck-damping sample a074.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::ground_control::GroundControlError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum GroundControlPrefixError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Control(GroundControlError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundControlPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(value) => write!(f, "Ground control missing {value}"),
            Self::Poisoned => f.write_str("Ground control owner lock poisoned"),
            Self::ForeignState => {
                f.write_str("Ground control requires the actual selected Ground state")
            }
            Self::Pair(error) => write!(f, "Ground control pair: {error}"),
            Self::Control(error) => write!(f, "{error}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_update_pair_ground_control_prefix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<f32, GroundControlPrefixError<N::Error>> {
        use GroundControlPrefixError as E;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let ground = state.ground.as_ref().ok_or(E::Missing("Ground state"))?;
            if state.current != CurrentPhysicalState::Ground
                || !ground.bindings.board.same_as(board)
                || !ground.bindings.skeleton.same_as(skeleton)
                || !ground
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        pool.try_ground_control_prefix(&self.backend, board, &owner, &processed, cvars, numeric)
            .map_err(E::Control)
    }
}
