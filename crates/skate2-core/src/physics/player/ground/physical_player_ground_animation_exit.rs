//! Same retained state103 `Exit82d83ed0`; destination selection belongs outside.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_board_forces::{
        PhysicalPlayerBoardForceError, board_wipeout::PhysicalBoardWipeoutError,
    },
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum GroundAnimationExitError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Board(PhysicalPlayerBoardForceError),
    Material(PhysicalBoardWipeoutError),
    Numeric(N),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundAnimationExitError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "state103 Exit missing {name}"),
            Self::Poisoned => f.write_str("state103 Exit owner lock poisoned"),
            Self::ForeignOwner => f.write_str("state103 Exit owner mismatch"),
            Self::Pair(e) => write!(f, "state103 Exit pair: {e:?}"),
            Self::Board(e) => write!(f, "state103 Exit board: {e}"),
            Self::Material(e) => write!(f, "state103 Exit material: {e}"),
            Self::Numeric(e) => write!(f, "state103 Exit numeric: {e:?}"),
        }
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_exit_pair_ground_animation<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), GroundAnimationExitError<N::Error>> {
        use GroundAnimationExitError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        if owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current
            != CurrentPhysicalState::GroundAnimation
        {
            return Err(E::ForeignOwner);
        }
        let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let Some(StatePayload::GroundAnimation(state)) = states
            .states
            .get(&StateKind::GroundAnimation)
            .filter(|s| s.published)
            .and_then(|s| s.value.as_ref())
        else {
            return Err(E::Missing("retained GroundAnimation"));
        };
        let bindings = &state.base.physical;
        if !bindings.board.same_as(board)
            || !bindings.skeleton.same_as(skeleton)
            || !bindings
                .owner
                .upgrade()
                .is_some_and(|v| Arc::ptr_eq(&v, &owner))
        {
            return Err(E::ForeignOwner);
        }
        self.try_set_board_linear_drag(board, 0.0)
            .map_err(E::Board)?;
        self.try_restore_board_wheel_material(board)
            .map_err(E::Material)?;
        let info = state.jump_info.as_ref().ok_or(E::Missing("jump info"))?;
        if !info.do_jump {
            return Ok(());
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let processed = processed.lock().map_err(|_| E::Poisoned)?;
        let gravity = processed
            .gravity()
            .ok_or(E::Missing("Processed gravity"))?
            .value;
        let dt = processed
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("Processed timestep"))?;
        // Uses the calculator result at state+20, not post-selector jump_vel+50.
        let jump = info.velocity;
        let acceleration = [0.0, gravity, 0.0, 0.0];
        let candidate = SkeletonRootVector {
            xyz: std::array::from_fn(|lane| acceleration[lane].mul_add(dt, jump.xyz[lane])),
            w: jump.w.map(|w| acceleration[3].mul_add(dt, w)),
        };
        let velocity = processed
            .prior_output()
            .velocity()
            .ok_or(E::Missing("Processed velocity"))?
            .value;
        let result = super::ground_exit::clamp_jump_velocity(candidate, velocity, numeric)
            .map_err(E::Numeric)?;
        drop(processed);
        drop(states);
        self.try_set_board_velocity(board, &result)
            .map_err(E::Board)
    }
}
