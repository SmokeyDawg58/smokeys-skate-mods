//! World forwarding to the SAME registered board owner.
#[path = "../air/physical_player_air_drive.rs"]
pub(crate) mod air_drive;
use super::{
    Arc, AssemblyPartMaterial, RwLock, SimulationAssemblyConstraintSession,
    SimulationDriveDynamics, SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    SkateboardHookDefinition,
};
use crate::{
    dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
};
impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    pub(crate) fn try_scale_board_manual_for_ground_entry(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_scale_board_manual_for_ground_entry(&self.backend, board)
    }
    pub(crate) fn try_reset_board_manual(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_reset_board_manual(&self.backend, board)
    }
    /// Actual PhysicsGround.Enter 82d99cc0..9d04. Current processed inputs are
    /// captured before both Body reads, and the second body supplies Part6.
    pub(crate) fn try_project_board_ground_entry_omega<
        N: crate::reckoning_numeric::ReckoningNumeric,
    >(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<
        (),
        crate::skateboard_force_runtime::entry::ResetSpeedModelError<
            SkateboardEntrySessionError,
            N::Error,
        >,
    > {
        use crate::skateboard_force_runtime::entry::ResetSpeedModelError as R;
        use SkateboardEntrySessionError as E;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(|_| R::Source(E::MissingConstruction))?,
        );
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| R::Source(E::ReckoningPoisoned))?
                .processed(),
        );
        let (normal, angular) = {
            let current = processed
                .lock()
                .map_err(|_| R::Source(E::ProcessedPoisoned))?;
            let normal = current
                .prior_output()
                .ground_normal()
                .ok_or(R::Source(E::UnknownGroundMotion))?
                .value;
            let angular = current
                .prior_output()
                .angular_velocity()
                .ok_or(R::Source(E::UnknownGroundMotion))?
                .value;
            (normal, angular)
        };
        self.board(board).map_err(|e| R::Source(E::Pool(e)))?;
        let second_body = self.board(board).map_err(|e| R::Source(E::Pool(e)))?;
        let dot = numeric
            .dot3(
                [angular.xyz[0], angular.xyz[1], angular.xyz[2], 0.0],
                [normal.xyz[0], normal.xyz[1], normal.xyz[2], 0.0],
            )
            .map_err(R::Numeric)?;
        let body = second_body
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(|e| R::Source(E::Part(e)))?;
        let omega = normal.xyz.map(|value| value * dot);
        self.backend
            .try_set_part_angular_velocity(body, omega)
            .map_err(|e| R::Source(E::Body(e)))
    }
    pub(crate) fn try_reset_board_speed_model<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<
        (),
        crate::skateboard_force_runtime::entry::ResetSpeedModelError<
            SkateboardEntrySessionError,
            N::Error,
        >,
    > {
        use crate::skateboard_force_runtime::entry::ResetSpeedModelError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Source(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            )))?
            .try_reset_board_speed_model(&self.backend, board, numeric)
    }
    pub(crate) fn try_reset_board_speed_wobble(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_reset_board_speed_wobble(&self.backend, board)
    }
    #[cfg(test)]
    pub(crate) fn with_board_speed_wobble_for_test<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        inspect: impl FnOnce(
            &mut crate::skateboard_force_runtime::speed_wobble::SkateboardSpeedWobbleOwner,
        ) -> T,
    ) -> T {
        self.skaters
            .as_mut()
            .unwrap()
            .with_board_speed_wobble_for_test(board, inspect)
    }

    pub(crate) fn try_retain_board_hook_dynamics(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_retain_board_hook_dynamics(&self.backend, board, owner, dynamics)
    }
    pub(crate) fn try_finish_board_entry_constructor(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_finish_board_entry_constructor(&self.backend, board)
    }
    pub(crate) fn try_disable_board_animation(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SkateboardEntrySessionError> {
        self.skaters
            .as_mut()
            .ok_or(SkateboardEntrySessionError::Pool(
                SimulationSkaterPoolError::Uninitialized,
            ))?
            .try_disable_board_animation(&self.backend, board)
    }
    #[cfg(test)]
    pub(crate) fn board_entry_snapshot(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> (bool, f32, Arc<RwLock<SimulationDriveDynamics>>) {
        self.skaters.as_ref().unwrap().board_entry_snapshot(board)
    }
}
