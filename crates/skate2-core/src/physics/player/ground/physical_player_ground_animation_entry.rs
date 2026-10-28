//! SAME retained state103 Enter82d83e68; outer transition owns state selection.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_entry::air_drive::AirDriveError,
    },
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    physical_player_wipeout::WipeoutOwnerError,
    reckoning_numeric::ReckoningNumeric,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum GroundAnimationEntryError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    Pair(SimulationSkeletonJumpPairError),
    Drive(AirDriveError<N>),
    Wipeout(WipeoutOwnerError),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundAnimationEntryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "GroundAnimation entry missing {v}"),
            Self::Poisoned => f.write_str("GroundAnimation entry lock poisoned"),
            Self::ForeignOwner => f.write_str("GroundAnimation entry owner mismatch"),
            Self::Pair(e) => write!(f, "GroundAnimation entry pair: {e:?}"),
            Self::Drive(e) => write!(f, "GroundAnimation entry drive: {e}"),
            Self::Wipeout(e) => write!(f, "GroundAnimation entry Wipeout: {e:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_enter_pair_ground_animation<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), GroundAnimationEntryError<N::Error>> {
        use GroundAnimationEntryError as E;
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
        {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let retained = states
                .states
                .get_mut(&StateKind::GroundAnimation)
                .filter(|s| s.published)
                .ok_or(E::Missing("published state103"))?;
            let Some(StatePayload::GroundAnimation(state)) = retained.value.as_mut() else {
                return Err(E::Missing("state103 allocation"));
            };
            let bindings = &state.base.physical;
            if !bindings.board.same_as(board)
                || !bindings.skeleton.same_as(skeleton)
                || !bindings
                    .owner
                    .upgrade()
                    .is_some_and(|p| Arc::ptr_eq(&p, &owner))
            {
                return Err(E::ForeignOwner);
            }
            state.jumped_this_frame = Some(false); //3e90
            state.jump_vel = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
                [0.0; 4],
            )); //3e94 full vector, constructor does not set it
        }
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.enable_physics_truck_tilt();
            })
            .map_err(E::Pair)?;
        self.try_enable_board_angular_drive_only(board, 1.0, refs, numeric)
            .map_err(E::Drive)?;
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_set_state(1)
            .map_err(E::Wipeout)
    }
}
