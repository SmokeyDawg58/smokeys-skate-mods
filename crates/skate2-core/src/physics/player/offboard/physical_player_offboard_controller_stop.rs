//! Reached StopController/LetGo on the selected OffBoard-to-Wipeout transition.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_entry::{SkateboardEntrySessionError, air_drive::AirDriveError},
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    simulation_assembly_backend::constraint_session::physical_board_forces::{
        PhysicalPlayerBoardForceError, board_wipeout::PhysicalBoardWipeoutError,
    },
    skateboard_controller::{SkateboardController, SkateboardControllerError},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OffBoardStopServiceError<N = std::convert::Infallible> {
    Poisoned,
    Missing(&'static str),
    BoardAnimation(SkateboardEntrySessionError),
    Free(PhysicalBoardWipeoutError),
    Volumes(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Pair(crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError),
    ForeignOwner,
    HandDrive(Box<SkateboardControllerError<Self>>),
    Cvar(InputCvarError),
    ReturningVelocity(PhysicalPlayerBoardForceError),
    Throw(super::controller_throw::ControllerThrowError),
    LinearHook(AirDriveError<std::convert::Infallible>),
    AngularDrive(AirDriveError<N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for OffBoardStopServiceError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoard StopController: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_stop_pair_offboard_controller(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
    ) -> Result<(), SkateboardControllerError<OffBoardStopServiceError>> {
        use OffBoardStopServiceError as S;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| SkateboardControllerError::Service(S::Poisoned))?
                .processed(),
        );
        let mut controller = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| SkateboardControllerError::Service(S::Poisoned))?;
        controller.try_stop_with(|controller| {
            self.try_let_go_retained_controller(board, skeleton, refs, &processed, controller)
        })
    }

    pub(crate) fn try_request_pair_drop_board<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), SkateboardControllerError<OffBoardStopServiceError<N::Error>>> {
        use OffBoardStopServiceError as S;
        let same = self
            .try_controller_pair_matches_owner(owner, board, skeleton)
            .map_err(|error| SkateboardControllerError::Service(S::Pair(error)))?;
        if !same {
            return Err(SkateboardControllerError::Service(S::ForeignOwner));
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| SkateboardControllerError::Service(S::Poisoned))?
                .processed(),
        );
        let mut controller = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| SkateboardControllerError::Service(S::Poisoned))?;
        controller.try_request_drop_with(|controller, hiding| {
            if hiding {
                return self
                    .try_initialize_board_hiding(board, refs, &processed, controller, numeric);
            }
            self.try_let_go_retained_controller(board, skeleton, refs, &processed, controller)
        })
    }

    /// PhysicalPlayerHiLOD.RequestHideSkateboard82dffe38 uses its retained controller.
    pub(crate) fn try_request_owner_hide_board(&mut self,
        owner:&Arc<PhysicalPlayerAssemblyOwner>,board:&SimulationSkaterAssemblyHandle,
        refs:&InputCvarReferences)->Result<(),String>{
        let processed=Arc::clone(owner.reckoning().lock().map_err(|_|"hide board reckoning lock")?.processed());
        let mut controller=owner.skateboard_controller().lock().map_err(|_|"hide board controller lock")?;
        controller.try_request_hide_with(|controller|self.try_initialize_board_hiding(board,refs,&processed,controller,
            &mut crate::reckoning_numeric::HostReckoningNumeric)).map_err(|e|format!("RequestHideBoard: {e:?}"))
    }

    /// `InitSkateboardHiding` 0x82dc76f0 on the same retained pair.
    pub(super) fn try_initialize_board_hiding<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        controller: &mut SkateboardController,
        numeric: &mut N,
    ) -> Result<(), SkateboardControllerError<OffBoardStopServiceError<N::Error>>> {
        use OffBoardStopServiceError as S;
        self.try_disable_board_collision_volumes(board)
            .map_err(|e| SkateboardControllerError::Service(S::Volumes(e)))?;
        self.try_disable_board_linear_drive(board)
            .map_err(|e| SkateboardControllerError::Service(S::LinearHook(e)))?;
        self.try_clear_board_grip_enabled(board)
            .map_err(|e| SkateboardControllerError::Service(S::Pool(e)))?;
        if controller.hand() != Some(2) {
            controller
                .try_disable_hand_drive_with(|| refs.try_skc_use_hand_drives().map_err(S::Cvar))
                .map_err(|e| SkateboardControllerError::Service(S::HandDrive(Box::new(e))))?;
        }
        controller.try_initialize_hiding_retrieval(|| {
            processed
                .lock()
                .map_err(|_| S::Poisoned)?
                .deck_transform()
                .copied()
                .ok_or(S::Missing("Processed deck transform+40"))
        })?;
        self.try_enable_board_angular_drive_only(board, 1.0, refs, numeric)
            .map_err(|e| SkateboardControllerError::Service(S::AngularDrive(e)))?;
        Ok(())
    }

    pub(super) fn try_let_go_retained_controller<N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        processed: &Arc<
            std::sync::Mutex<crate::skeleton_processed_contact::SkeletonProcessedContact>,
        >,
        controller: &mut SkateboardController,
    ) -> Result<(), SkateboardControllerError<OffBoardStopServiceError<N>>> {
        use OffBoardStopServiceError as S;
        // LetGo82dd03e8: board animation off, SetFree (including its own
        // collision enable), then a *second* collision enable.
        self.try_disable_board_animation(board)
            .map_err(|error| SkateboardControllerError::Service(S::BoardAnimation(error)))?;
        self.try_set_board_free(board, refs)
            .map_err(|error| SkateboardControllerError::Service(S::Free(error)))?;
        self.try_enable_board_collision_volumes(board)
            .map_err(|error| SkateboardControllerError::Service(S::Volumes(error)))?;
        // Source reloads SkateboardBody and clears +0x2098 bit 0x40.
        self.try_clear_board_grip_enabled(board)
            .map_err(|error| SkateboardControllerError::Service(S::Pool(error)))?;
        if controller.hand() != Some(2) {
            controller
                .try_disable_hand_drive_with(|| refs.try_skc_use_hand_drives().map_err(S::Cvar))
                .map_err(|error| {
                    SkateboardControllerError::Service(S::HandDrive(Box::new(error)))
                })?;
        }
        if controller.state() == Some(4) {
            // LetGo82dd0470..4a0: reset the actual board velocity before
            // loading the subsequent throw flag from Processed.
            self.try_set_board_velocity(
                board,
                &crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            )
            .map_err(|e| SkateboardControllerError::Service(S::ReturningVelocity(e)))?;
        }
        if processed
            .lock()
            .map_err(|_| SkateboardControllerError::Service(S::Poisoned))?
            .physics_state_fields()
            .flags_8cc
            .ok_or(SkateboardControllerError::Service(S::Missing(
                "Processed+8cc",
            )))?
            & 0x2000
            != 0
        {
            self.try_throw_retained_controller(board, skeleton, processed, controller)
                .map_err(|e| SkateboardControllerError::Service(S::Throw(e)))?;
        }
        Ok(())
    }
}
