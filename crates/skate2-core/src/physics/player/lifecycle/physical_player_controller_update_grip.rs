//! Reached SkateboardController::UpdateState(state 1) / UpdateGrip.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    dynamic_assembly_session::skeleton_normal::part_transform_query::SkeletonPartTransformQueryError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{StateConstructorError, read_floats},
    },
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric, try_normalize_safe,
    },
    reckoning_sin_cos::{HostReckoningSinCosNumeric, try_cosine},
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    skateboard_controller::drive_frames::ControllerDriveFramesError,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum ControllerGripError {
    Poisoned,
    Missing(&'static str),
    WrongControllerState(Option<u32>),
    Board(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Part(SkeletonPartTransformQueryError),
    Numeric(ReckoningHostNumericError),
    Attribute(StateConstructorError),
    Cosine(Infallible),
    MissingProcessedFlags8cc,
    MissingProcessedFlags8d0,
    PendingLetGo,
    DriveFrames(ControllerDriveFramesError<ControllerDriveSourceError>),
}

#[derive(Debug)]
pub(crate) enum ControllerDriveSourceError {
    Cvar(InputCvarError),
    Pool(SimulationSkaterPoolError),
    Missing(&'static str),
    Numeric(ReckoningHostNumericError),
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_controller_pair_matches_owner(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<bool, crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError>
    {
        use crate::dynamic_assembly_session::skater_pool::SimulationSkeletonJumpPairError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(owner, actual)
            })
    }

    /// UpdateState 0x82dd1950..1b68 gates release after UpdateGrip; when
    /// state 1 remains, Controller.Update calls the retained DriveFrames leaf.
    pub(crate) fn try_finish_pair_controller_after_grip(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        prior_body_flag: bool,
        cvars: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), ControllerGripError> {
        use ControllerGripError as E;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let flags_8cc = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .flags_8cc
            .ok_or(E::MissingProcessedFlags8cc)?;
        if flags_8cc & 0x4000 != 0 || prior_body_flag {
            let flags_8d0 = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .physics_state_fields()
                .flags_8d0
                .ok_or(E::MissingProcessedFlags8d0)?;
            if flags_8d0 & 0x0020_0000 == 0 {
                return Err(E::PendingLetGo);
            }
        }
        self.try_update_pair_controller_drive_frames(owner, skeleton, cvars, numeric)
    }
    pub(crate) fn try_update_pair_controller_drive_frames(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
    ) -> Result<(), ControllerGripError> {
        use ControllerGripError as E;
        let mut controller = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?;
        if controller.state() != Some(1) {
            return Err(E::WrongControllerState(controller.state()));
        }
        controller
            .try_update_drive_frames_with(
                || {
                    cvars
                        .try_skc_use_hand_drives()
                        .map_err(ControllerDriveSourceError::Cvar)
                },
                |part| {
                    self.skeleton(skeleton)
                        .map_err(ControllerDriveSourceError::Pool)?
                        .animation_targets()
                        .volume_row(part)
                        .copied()
                        .ok_or(ControllerDriveSourceError::Missing(
                            "Skeleton animation Part row",
                        ))
                },
                |magnitudes| {
                    let mut estimates = [0.0; 4];
                    for (result, magnitude) in estimates.iter_mut().zip(magnitudes) {
                        *result = numeric
                            .reciprocal_sqrt_estimate(magnitude)
                            .map_err(ControllerDriveSourceError::Numeric)?;
                    }
                    Ok(estimates)
                },
            )
            .map_err(E::DriveFrames)
    }
    /// UpdateState 0x82dd1928..194c snapshots Body+0x364 before UpdateGrip.
    /// UpdateGrip 0x82dcc7f8..a10 then writes the same body's grip fields.
    pub(crate) fn try_update_pair_controller_grip(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<bool, ControllerGripError> {
        use ControllerGripError as E;
        {
            let controller = owner
                .skateboard_controller()
                .lock()
                .map_err(|_| E::Poisoned)?;
            if controller.state() != Some(1) {
                return Err(E::WrongControllerState(controller.state()));
            }
        }
        // The selected Hold case reads this flag before calling UpdateGrip.
        let contacts = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Board)?;
        let body_flag = contacts
            .lock()
            .map_err(|_| E::Poisoned)?
            .collision_state
            .flags
            .read(0x0800_0000)
            .ok_or(E::Missing("board Body+364 flag"))?
            != 0;
        let hand = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::Poisoned)?
            .hand()
            .ok_or(E::Missing("controller hand+1a8"))?;
        let part = 4 + if hand == 0 { 0 } else { 4 };
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        // Native loads Processed+0x70 before Skeleton::GetPartTransform.
        let deck_p = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .deck_transform()
            .ok_or(E::Missing("Processed deck transform+70"))?
            .columns()[3];
        let part_p = self
            .skeleton(skeleton)
            .map_err(E::Pool)?
            .try_query_part_transform(part)
            .map_err(E::Part)?
            .columns()[3];
        let mut numeric = HostReckoningNumeric;
        let zero = SkeletonRootVector::known([0.0; 4]);
        let forearm_delta = part_p.zip(deck_p, |part, deck| part - deck);
        let forearm = try_normalize_safe(forearm_delta, zero, &mut numeric).map_err(E::Numeric)?;
        let com = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .prior_output()
            .centre_of_mass()
            .ok_or(E::Missing("Processed centre of mass+250"))?
            .value;
        let mut com_delta = com.zip(deck_p, |com, deck| com - deck);
        com_delta.xyz[1] = 0.0;
        let com_direction =
            try_normalize_safe(com_delta, zero, &mut numeric).map_err(E::Numeric)?;
        let sum = forearm.zip(com_direction, |forearm, com| forearm + com);
        let average = sum.map(|lane| lane * 0.5);
        let mixed = try_normalize_safe(average, zero, &mut numeric).map_err(E::Numeric)?;
        let up = SkeletonRootVector::known([0.0, 1.0, 0.0, 0.0]);
        let grip_direction = try_normalize_safe(mixed, up, &mut numeric).map_err(E::Numeric)?;
        let degrees = {
            let controller = owner
                .skateboard_controller()
                .lock()
                .map_err(|_| E::Poisoned)?;
            let attributes = controller
                .attributes()
                .ok_or(E::Missing("controller attribute Instance"))?;
            read_floats::<1>(attributes, 0x5700_7609_f4d4_f733).map_err(E::Attribute)?[0]
        };
        let radians = degrees * f32::from_bits(0x3c8e_fa35);
        let cosine = try_cosine(radians, &mut HostReckoningSinCosNumeric).map_err(E::Cosine)?;
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .with_board_post_fields(board, |fields| {
                fields.grip_cosine = cosine;
                fields.grip_vector = grip_direction;
                fields.grip_enabled = true;
            })
            .map_err(E::Pool)?;
        Ok(body_flag)
    }

    /// LetGoOfSkateboard82dd0434..44 clears Body+0x2098 bit 0x40 on the
    /// same retained post-fields owner used by UpdateGrip's set operation.
    pub(crate) fn try_clear_board_grip_enabled(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), SimulationSkaterPoolError> {
        self.skaters
            .as_mut()
            .ok_or(SimulationSkaterPoolError::Uninitialized)?
            .with_board_post_fields(board, |fields| fields.grip_enabled = false)
    }
}
