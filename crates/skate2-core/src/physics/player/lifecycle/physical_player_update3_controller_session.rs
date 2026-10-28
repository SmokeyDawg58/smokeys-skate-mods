//! Shared `Update3_State82dfe7d8` controller after the selected state returns.
use super::ground_finish_session::GroundUpdateComplete;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    skateboard_controller::inactive_update::ControllerUpdateError,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update3ControllerError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Poisoned,
    Controller(ControllerUpdateError),
    Grip(super::controller_update_grip::ControllerGripError),
    Active(crate::physical_player_assembly::controller_active::ControllerActiveError),
}
impl std::fmt::Display for Update3ControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Update3 controller requires actual skater pool"),
            Self::Pair(error) => error.fmt(f),
            Self::ForeignOwner => {
                f.write_str("Update3 controller continuation belongs to another pair")
            }
            Self::Poisoned => f.write_str("Update3 controller owner poisoned"),
            Self::Controller(error) => error.fmt(f),
            Self::Active(error) => write!(f, "Update3 active controller: {error:?}"),
            Self::Grip(error) => write!(f, "Update3 controller UpdateGrip: {error:?}"),
        }
    }
}
/// Controller returned. Empty shared stub82dfe7f0 and real `PrePhysics` follow.
pub(crate) struct Update3BeforePrePhysics {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl PhysicalAssemblyWorld {
    ///82dfe7d8: same retained controller, never a newly stopped substitute.
    pub(crate) fn try_update3_controller_after_ground(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        completed: GroundUpdateComplete,
    ) -> Result<Update3BeforePrePhysics, Update3ControllerError> {
        self.try_update3_controller_after_state(board, skeleton, completed.0.0.owner)
    }
    pub(crate) fn try_update3_controller_after_air(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        completed: super::air_update_finish::AirUpdateComplete,
    ) -> Result<
        (
            Update3BeforePrePhysics,
            crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
        ),
        Update3ControllerError,
    > {
        let before = self.try_update3_controller_after_state(board, skeleton, completed.owner)?;
        Ok((before, completed.reckoning))
    }
    pub(crate) fn try_update3_controller_after_wipeout(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        completed: super::physical_grounded::wipeout::WipeoutUpdateComplete,
    ) -> Result<
        (
            Update3BeforePrePhysics,
            super::update5_managers::Update5ReckoningRead,
        ),
        Update3ControllerError,
    > {
        let before = self.try_update3_controller_after_state(
            board,
            skeleton,
            Arc::clone(completed.owner()),
        )?;
        Ok((
            before,
            super::update5_managers::Update5ReckoningRead::RetainedWipeout(completed),
        ))
    }
    pub(crate) fn try_update3_controller_after_ground_animation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        completed: super::ground_animation_update::GroundAnimationComplete,
    ) -> Result<
        (
            Update3BeforePrePhysics,
            crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
        ),
        Update3ControllerError,
    > {
        let before = self.try_update3_controller_after_state(board, skeleton, completed.owner)?;
        Ok((before, completed.reckoning))
    }
    /// Teleporting.Update has returned from either its Actor request or its
    /// Processed reset copy. Update3 then calls the same retained controller.
    pub(crate) fn try_update3_controller_after_teleporting(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<Update3BeforePrePhysics, Update3ControllerError> {
        if owner
            .physics_state()
            .lock()
            .map_err(|_| Update3ControllerError::Poisoned)?
            .current
            != crate::physical_player_physics_state::CurrentPhysicalState::Teleporting
        {
            return Err(Update3ControllerError::ForeignOwner);
        }
        self.try_update3_controller_after_state(board, skeleton, owner)
    }
    /// Pushing has no Reckoning phase to return. The active offboard board
    /// controller still follows state.Update at82dfe7d8, exactly as for walking.
    pub(crate) fn try_update3_controller_after_pushing(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: Arc<PhysicalPlayerAssemblyOwner>,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut crate::reckoning_numeric::HostReckoningNumeric,
        trig: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
    ) -> Result<Update3BeforePrePhysics, Update3ControllerError> {
        if owner
            .physics_state()
            .lock()
            .map_err(|_| Update3ControllerError::Poisoned)?
            .current
            != crate::physical_player_physics_state::CurrentPhysicalState::OffBoardPushing
        {
            return Err(Update3ControllerError::ForeignOwner);
        }
        self.try_update_pair_active_controller(&owner, board, skeleton, cvars, numeric, trig)
            .map_err(Update3ControllerError::Active)?;
        Ok(Update3BeforePrePhysics { owner })
    }

    pub(crate) fn try_update3_controller_after_offboard(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        completed: crate::physical_player_assembly::offboard_feet_ik_session::OffboardUpdateComplete,
        cvars: &crate::input_cvar_runtime::InputCvarReferences,
        numeric: &mut crate::reckoning_numeric::HostReckoningNumeric,
        trig: &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric,
    ) -> Result<
        (
            Update3BeforePrePhysics,
            crate::physical_player_reckoning_owner::PhysicalPlayerReckoningPhase,
        ),
        Update3ControllerError,
    > {
        if !matches!(
            completed
                .owner
                .physics_state()
                .lock()
                .map_err(|_| Update3ControllerError::Poisoned)?
                .current,
            crate::physical_player_physics_state::CurrentPhysicalState::OffBoard
                | crate::physical_player_physics_state::CurrentPhysicalState::OffBoardAir
                | crate::physical_player_physics_state::CurrentPhysicalState::OffBoardJump
                | crate::physical_player_physics_state::CurrentPhysicalState::LandingOnDeck
                | crate::physical_player_physics_state::CurrentPhysicalState::OffBoardLocomotion
        ) {
            return Err(Update3ControllerError::ForeignOwner);
        }
        self.try_update_pair_active_controller(
            &completed.owner,
            board,
            skeleton,
            cvars,
            numeric,
            trig,
        )
        .map_err(Update3ControllerError::Active)?;
        let before = Update3BeforePrePhysics {
            owner: completed.owner,
        };
        Ok((before, completed.reckoning))
    }
    pub(super) fn try_update3_controller_after_state(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: Arc<PhysicalPlayerAssemblyOwner>,
    ) -> Result<Update3BeforePrePhysics, Update3ControllerError> {
        use Update3ControllerError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        let update = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_inactive();
        match update {
            Ok(()) => {}
            Err(ControllerUpdateError::PendingUpdateState) => {
                let body_flag = self
                    .try_update_pair_controller_grip(&owner, board, skeleton)
                    .map_err(E::Grip)?;
                return Err(E::Controller(
                    ControllerUpdateError::PendingPostGripStateGate(body_flag),
                ));
            }
            Err(error) => return Err(E::Controller(error)),
        }
        Ok(Update3BeforePrePhysics { owner })
    }
}
