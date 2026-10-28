//! Reached OffBoard500 StartController/HoldSkateboard on the live pair.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_entry::animation::BoardAnimationRuntimeError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    skateboard_controller::{SkateboardControllerError, drive_frames::ControllerDriveFramesError},
};
use std::{cell::RefCell, sync::Arc};

#[derive(Debug)]
pub(crate) enum OffBoardStartServiceError<N> {
    Poisoned,
    Missing(&'static str),
    Pool(SimulationSkaterPoolError),
    Cvar(InputCvarError),
    BoardAnimation(BoardAnimationRuntimeError<N>),
    Standard(PhysicalPlayerBoardForceError),
    HandDrive(Box<SkateboardControllerError<Self>>),
    DriveFrames(Box<ControllerDriveFramesError<Self>>),
    Numeric(N),
    LetGo(
        Box<SkateboardControllerError<super::offboard_controller_stop::OffBoardStopServiceError>>,
    ),
}
impl<N: std::fmt::Debug> std::fmt::Display for OffBoardStartServiceError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoard StartController: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    /// EnterNewPhysicsState500 calls StartController before transition fields.
    /// All callbacks use the same registered board, Skeleton, processed input,
    /// and controller; no success substitute is supplied for the free branch.
    pub(crate) fn try_start_pair_offboard_controller<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), SkateboardControllerError<OffBoardStartServiceError<N::Error>>> {
        self.try_start_or_hold_pair_controller(owner, board, skeleton, refs, numeric, true)
    }
    pub(crate) fn try_hold_pair_controller<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), SkateboardControllerError<OffBoardStartServiceError<N::Error>>> {
        self.try_start_or_hold_pair_controller(owner, board, skeleton, refs, numeric, false)
    }
    fn try_start_or_hold_pair_controller<N: ReckoningNumeric>(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
        start: bool,
    ) -> Result<(), SkateboardControllerError<OffBoardStartServiceError<N::Error>>> {
        use OffBoardStartServiceError as S;
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
        let world = RefCell::new(self);
        let numeric = RefCell::new(numeric);
        controller.try_start_or_hold_with(
            start,
            || {
                processed
                    .lock()
                    .map_err(|_| S::Poisoned)?
                    .physics_state_fields()
                    .flags_8d0
                    .ok_or(S::Missing("Processed+8d0"))
            },
            || {
                let soft = refs.try_skc_use_soft_hook().map_err(S::Cvar)?;
                world
                    .borrow_mut()
                    .try_enable_board_animation_for_hold(
                        board,
                        soft,
                        refs,
                        &mut **numeric.borrow_mut(),
                    )
                    .map_err(S::BoardAnimation)?;
                world
                    .borrow_mut()
                    .try_set_board_standard(board)
                    .map_err(S::Standard)?;
                world
                    .borrow_mut()
                    .try_enable_board_collision_volumes(board)
                    .map_err(S::Standard)
            },
            || {
                processed
                    .lock()
                    .map_err(|_| S::Poisoned)?
                    .physics_state_fields()
                    .flags_8cc
                    .ok_or(S::Missing("Processed+8cc"))
            },
            |controller, _hand| {
                controller
                    .try_enable_hand_drive_with(
                        || refs.try_skc_use_hand_drives().map_err(S::Cvar),
                        || refs.try_skc_hand_drive_strength().map_err(S::Cvar),
                        || refs.try_skc_hand_drive_displacement().map_err(S::Cvar),
                        |controller, _hand| {
                            controller
                                .try_update_drive_frames_with(
                                    || refs.try_skc_use_hand_drives().map_err(S::Cvar),
                                    |part| {
                                        let world = world.borrow();
                                        let skater = world.skeleton(skeleton).map_err(S::Pool)?;
                                        skater
                                            .animation_targets()
                                            .volume_row(part)
                                            .copied()
                                            .ok_or(S::Missing("Skeleton animation Part row"))
                                    },
                                    |magnitudes| {
                                        let mut numeric = numeric.borrow_mut();
                                        let mut estimates = [0.0; 4];
                                        for (result, magnitude) in
                                            estimates.iter_mut().zip(magnitudes)
                                        {
                                            *result = numeric
                                                .reciprocal_sqrt_estimate(magnitude)
                                                .map_err(S::Numeric)?;
                                        }
                                        Ok(estimates)
                                    },
                                )
                                .map_err(|error| S::DriveFrames(Box::new(error)))
                        },
                    )
                    .map_err(|error| S::HandDrive(Box::new(error)))
            },
            |controller| {
                world
                    .borrow_mut()
                    .try_let_go_retained_controller(board, skeleton, refs, &processed, controller)
                    .map_err(|error| S::LetGo(Box::new(error)))
            },
        )
    }
}
