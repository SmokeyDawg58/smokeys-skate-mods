//! Retained `PhysicalPlayerHiLOD::TeleportPlayerNow` operations and the shared
//! `Skateboard::TeleportReset` used by teleporting and post-physics recovery.

use super::physical_skeleton_controller::{
    PhysicalSkeletonControllerError, SkeletonCollisionTransition,
};
use super::physical_teleport_now_matrix::{HostTeleportNowMatrixNumeric, adjusted_teleport_matrix};
use super::world_services::PhysicalWorldServicesError;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::skater_pool::board_construction::SimulationConstructedBoardError,
    dynamic_assembly_session::skater_pool::board_entry::{
        SkateboardEntrySessionError, try_current_board_unflipped_transform,
    },
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError, SimulationSkeletonJumpPairError,
        SimulationSkeletonMutationError, SimulationSkeletonTeleportResetError,
        board_collision_mode::BoardCollisionModeError,
    },
    input_cvar_runtime::{InputCvarError, reckoning::ReckoningCvarAdapter},
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        hilod_tail::{HiLodCollisionChange, HiLodTailError},
    },
    physical_player_grab_spline::invalidate::GrabInvalidateError,
    physical_player_phys_out::conditioners::ResetAllError,
    physical_player_phys_out::construct::ConcretePhysOutError,
    physical_player_reckoning_owner::{
        PhysicalPlayerReckoningStalePhase, PhysicalPlayerReckoningUpdateError,
    },
    physical_player_state_changer::StateChangerTeleportResetError,
    physical_player_wipeout::WipeoutOwnerError,
    physical_player_wipeout::ground_access::GroundOwnerError,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError},
    simulation_assembly_backend::constraint_session::physical_board_forces::PhysicalPlayerBoardForceError,
    skateboard_controller::SkateboardControllerError,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum TeleportNowPrefixError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    Collision(PhysicalSkeletonControllerError),
    PrefixPoisoned,
    MissingPrefix,
    PrepareVelocityPoisoned,
    TailPoisoned,
    Tail(HiLodTailError),
    PhysOutPoisoned,
    PhysOutReset(ConcretePhysOutError<std::convert::Infallible>),
    ConditionersPoisoned,
    MissingConditioners,
    ConditionersReset(ResetAllError),
    OuterBoardStandard(PhysicalPlayerBoardForceError),
    OuterSkeletonNormal(
        SimulationSkeletonMutationError<AttributeRuntimeError, std::convert::Infallible>,
    ),
    NetworkServices(PhysicalWorldServicesError),
    NetworkBoardCollision(BoardCollisionModeError),
    ControllerPoisoned,
    ControllerReset(SkateboardControllerError<std::convert::Infallible>),
    BoardInitialize(SimulationConstructedBoardError),
    ReckoningPoisoned,
    ReckoningReset(PhysicalPlayerReckoningStalePhase),
    ReckoningGrounded(
        PhysicalPlayerReckoningUpdateError<
            std::convert::Infallible,
            InputCvarError,
            ReckoningHostNumericError,
        >,
    ),
    WipeoutPoisoned,
    WipeoutReset(WipeoutOwnerError),
    WipeoutGroundTimer(GroundOwnerError),
    TrajectoryPoisoned,
    MissingOnboardTrajectory,
    StateChangerPoisoned,
    StateChangerReset(StateChangerTeleportResetError),
    RemainingPoisoned,
    MissingFeetIk,
    MissingLandingOnDeck,
    MissingOffboardJumping,
    MissingJumpingQueries,
    ForeignJumpingQueries,
    JumpingReset(
        crate::physical_player_remaining_constructor_owners::jumping_runtime::JumpingError<
            ReckoningHostNumericError,
            InputCvarError,
        >,
    ),
    SkeletonControllerPoisoned,
    GrabSplinePoisoned,
    MissingGrabSpline,
    GrabInvalidate(GrabInvalidateError),
    SkeletonTeleportReset(SimulationSkeletonTeleportResetError),
    BoardTransformPool(SimulationSkaterPoolError),
    BoardTransform(SkateboardEntrySessionError),
    ProcessedPoisoned,
    UnknownBoardFlip,
    BodySet(
        crate::dynamic_assembly_session::skater_pool::controller_target::ControllerBoardTargetError,
    ),
    BoardMotionReset(SimulationConstructedBoardError),
    BoardCollisionResetPrefix(SimulationConstructedBoardError),
    BoardCollisionPartLoop(SimulationConstructedBoardError),
    BoardCollisionWheelLoop(SimulationConstructedBoardError),
    BoardGripReset(SimulationConstructedBoardError),
    OuterBoardReset(SimulationConstructedBoardError),
}

#[derive(Debug)]
pub(crate) enum TeleportNowFlagError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
}

impl PhysicalAssemblyWorld {
    /// `Skateboard::TeleportReset` 0x82b360b8..1c4. Both callers have already
    /// validated this board's physical owner. This resets only the board;
    /// the surrounding HiLOD/Skeleton resets belong to TeleportPlayerNow.
    pub(crate) fn try_teleport_reset_board_live(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        reset_transform: SkeletonRootMatrix,
    ) -> Result<(), TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        // AIProcess::OnEnd82b360e4 is an empty BLR. InitializeTransforms
        // writes attached Parts6,0..5; it does not change Processed inputs.
        self.try_teleport_initialize_board_transforms(board)
            .map_err(E::BoardInitialize)?;
        // SetEffectiveTransform82b35ae0 samples the live flip after those
        // writes and flips every lane of X/Z before setting the Body/hook.
        let processed = Arc::clone(
            owner.reckoning().lock().map_err(|_| E::ReckoningPoisoned)?.processed(),
        );
        let flipped = processed.lock().map_err(|_| E::ProcessedPoisoned)?
            .board_flipped().ok_or(E::UnknownBoardFlip)?;
        let effective = reset_transform.with_board_flip(flipped);
        self.try_set_board_transform_typed(board, effective, &owner.inputs().cvars)
            .map_err(E::BodySet)?;
        self.try_teleport_reset_board_motion_prefix(board)
            .map_err(E::BoardMotionReset)?;
        self.try_teleport_reset_collision_prefix(board)
            .map_err(E::BoardCollisionResetPrefix)?;
        self.try_teleport_reset_collision_part_loop(board)
            .map_err(E::BoardCollisionPartLoop)?;
        self.try_teleport_reset_collision_wheel_loop(board)
            .map_err(E::BoardCollisionWheelLoop)?;
        self.try_teleport_reset_board_grip(board)
            .map_err(E::BoardGripReset)?;
        self.try_teleport_reset_outer_board(board)
            .map_err(E::OuterBoardReset)
    }

    /// TeleportPlayerNow 0x82e009c8..a4c: optional network query, retained
    /// HiLOD flag write, then board and Skeleton collision changes if needed.
    pub(crate) fn try_finish_teleport_network_collision(
        &mut self,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        let network = owner
            .network_query()
            .map(|query| query.try_is_network_game())
            .transpose()
            .map_err(E::NetworkServices)?;
        let change = owner
            .tail()
            .lock()
            .map_err(|_| E::TailPoisoned)?
            .select_network_collision(network);
        if let Some(change) = change {
            let world_only = change == HiLodCollisionChange::WorldOnly;
            self.try_set_pair_board_collision_mode(board, skeleton, world_only)
                .map_err(E::NetworkBoardCollision)?;
            self.try_pair_skeleton_collision_transition(
                board,
                skeleton,
                if world_only {
                    SkeletonCollisionTransition::StartWorldOnly
                } else {
                    SkeletonCollisionTransition::StopWorldOnly
                },
            )
            .map_err(E::Collision)?;
        }
        Ok(())
    }
    /// TeleportPlayerNow 0x82e009ac..c4 repeats SetStandard after Ground.Enter,
    /// then invokes SetUpNormal on the retained Skeleton body.
    pub(crate) fn try_finish_teleport_outer_normal(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        self.try_set_board_standard(board)
            .map_err(E::OuterBoardStandard)?;
        self.try_set_constructed_skeleton_normal(skeleton)
            .map_err(E::OuterSkeletonNormal)
    }
    /// TeleportPlayerNow 0x82e00988 resets the existing eleven-child owner.
    pub(crate) fn try_reset_teleport_conditioners(
        &mut self,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        let mut output = owner.phys_out().lock().map_err(|_| E::PhysOutPoisoned)?;
        let mut conditioners = owner
            .conditioners()
            .lock()
            .map_err(|_| E::ConditionersPoisoned)?;
        conditioners
            .as_mut()
            .ok_or(E::MissingConditioners)?
            .try_reset_all(&mut output, &owner.inputs().cvars)
            .map_err(E::ConditionersReset)
    }
    /// TeleportPlayerNow 0x82e0096c..80 resumes after Skeleton::TeleportNow:
    /// same controller's second Reset, then same Wipeout timer store.
    pub(crate) fn try_finish_teleport_after_ground_skateboard(
        &mut self,
        owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    ) -> Result<(), TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::ControllerPoisoned)?
            .try_reset_after_teleport()
            .map_err(E::ControllerReset)?;
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::WipeoutPoisoned)?
            .try_store_ground_invincibility(f32::from_bits(0x3ecc_cccd))
            .map_err(E::WipeoutGroundTimer)
    }
    /// Skeleton::TeleportNow writes the retained Skeleton+3e44 flag around
    /// the lower GroundSkateboard call on this same admitted pair.
    pub(crate) fn try_set_pair_teleporting_now(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        expected: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
        value: bool,
    ) -> Result<(), TeleportNowFlagError> {
        use TeleportNowFlagError as E;
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, owner| {
                if !Arc::ptr_eq(owner, expected) {
                    return Err(E::ForeignOwner);
                }
                normal.set_teleporting_now(value);
                Ok(())
            })
            .map_err(E::Pair)?
    }
    /// Raw 0x82e0062c..3c: set the retained SkeletonController to
    /// `SCS_STANDARD` (6), then clear this HiLOD owner's dismount count.
    /// The native collision transition's partial writes survive a failure;
    /// the counter is untouched until that call returns. Then the source
    /// constructs its yaw-only reset matrix and clears the existing HiLOD
    /// flags and spin history, then clears the three retained skeleton line
    /// records in place. The same retained PhysOut then runs its complete
    /// source-ordered reset, then the existing board controller runs its
    /// StopController and in-place retrieval reset. It starts the board's
    /// TeleportReset with the original attached seven-Part initializer; the
    /// board effective transform is copied and sign-flipped using the
    /// retained Processed bit; the existing Body setter moves the attached
    /// board and hook before TeleportReset's next explicit boundary.
    pub(crate) fn try_begin_pair_teleport_player_now(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        reset_transform: SkeletonRootMatrix,
    ) -> Result<SkeletonRootMatrix, TeleportNowPrefixError> {
        use TeleportNowPrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(6),
        )
        .map_err(E::Collision)?;
        owner
            .prefix()
            .lock()
            .map_err(|_| E::PrefixPoisoned)?
            .as_mut()
            .ok_or(E::MissingPrefix)?
            .physics_requests_dismount_count = 0;
        let adjusted =
            adjusted_teleport_matrix(&reset_transform, &mut HostTeleportNowMatrixNumeric);
        {
            let mut prefix = owner.prefix().lock().map_err(|_| E::PrefixPoisoned)?;
            let prefix = prefix.as_mut().ok_or(E::MissingPrefix)?;
            prefix.non_collision_flags = (prefix.non_collision_flags & 0x81ff_ffff) | 0x6000_0000;
        }
        owner
            .prepare_velocity()
            .lock()
            .map_err(|_| E::PrepareVelocityPoisoned)?
            .reset_after_teleport();
        owner
            .tail()
            .lock()
            .map_err(|_| E::TailPoisoned)?
            .reset_teleport_line_records()
            .map_err(E::Tail)?;
        owner
            .phys_out()
            .lock()
            .map_err(|_| E::PhysOutPoisoned)?
            .try_reset_complete()
            .map_err(E::PhysOutReset)?;
        owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::ControllerPoisoned)?
            .try_reset_after_teleport()
            .map_err(E::ControllerReset)?;
        self.try_teleport_reset_board_live(board, &owner, adjusted)?;
        // ResetSystems 0x82dfff20 first invokes the already retained
        // Reckoning::Reset at 0x82dfff38. Its next board-transform read is
        // distinct from GetEffectiveTransform and remains a typed boundary.
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::ReckoningPoisoned)?
            .try_reset()
            .map_err(E::ReckoningReset)?;
        // ResetSystems 0x82dfff44 next reads Skateboard::GetTransform:
        // attached Part6's live body position, without a Processed flip.
        // Its UpdateGroundStates call takes two full gJ vectors and this
        // unflipped Part6 column Z. It runs on the same retained Reckoning.
        let board_transform = try_current_board_unflipped_transform(
            self.board(board).map_err(E::BoardTransformPool)?,
            self.backend(),
        )
        .map_err(E::BoardTransform)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::ReckoningPoisoned)?
                .processed(),
        );
        owner
            .reckoning()
            .lock()
            .map_err(|_| E::ReckoningPoisoned)?
            .try_update_grounded_reset_systems(
                &processed,
                board_transform.columns()[2],
                &mut ReckoningCvarAdapter(&owner.inputs().cvars),
                &mut HostReckoningNumeric,
            )
            .map_err(E::ReckoningGrounded)?;
        // ResetSystems 0x82dfff80 resets only state+0xbc and request
        // count+0xb4 on the retained Wipeout object. Request arrays survive.
        owner
            .wipeout()
            .lock()
            .map_err(|_| E::WipeoutPoisoned)?
            .try_reset()
            .map_err(E::WipeoutReset)?;
        // ResetSystems 0x82dfff88 resets the already constructed onboard
        // TrajectorySelector. The unrelated offboard/query owners survive.
        owner
            .trajectories()
            .lock()
            .map_err(|_| E::TrajectoryPoisoned)?
            .onboard
            .as_mut()
            .ok_or(E::MissingOnboardTrajectory)?
            .reset_after_teleport();
        // ResetSystems 0x82dfff90 clears seven retained counters, then
        // samples the live teleport countdown cvar.
        owner
            .state_changer()
            .lock()
            .map_err(|_| E::StateChangerPoisoned)?
            .try_teleport_reset(&owner.inputs().cvars)
            .map_err(E::StateChangerReset)?;
        // ResetSystems 0x82dfff98 invokes the existing OffBoardFeetIK
        // Reset, preserving its Processed/Skeleton bindings.
        owner
            .remaining()
            .lock()
            .map_err(|_| E::RemainingPoisoned)?
            .feet_ik
            .as_mut()
            .ok_or(E::MissingFeetIk)?
            .reset_after_teleport();
        // ResetSystems 0x82dfffa0 resets only the current landing
        // trajectory and selected timers/flags, not its ideal trajectory.
        owner
            .remaining()
            .lock()
            .map_err(|_| E::RemainingPoisoned)?
            .landing
            .as_mut()
            .ok_or(E::MissingLandingOnDeck)?
            .reset_after_teleport();
        // ResetSystems 0x82dfffa4 calls OffboardJumpingTest::Reset82dda070: an
        // active batch completes through CalculateResults, then the counter clears.
        {
            let mut remaining = owner.remaining().lock().map_err(|_| E::RemainingPoisoned)?;
            let jumping = remaining
                .jumping
                .as_mut()
                .ok_or(E::MissingOffboardJumping)?;
            if jumping.line_batch.is_some() {
                let queries = self
                    .world_line_queries
                    .as_mut()
                    .ok_or(E::MissingJumpingQueries)?;
                if !queries.belongs_to(owner.manager()) {
                    return Err(E::ForeignJumpingQueries);
                }
                jumping
                    .reset(
                        queries,
                        &mut owner.inputs().cvars.clone(),
                        &mut HostReckoningNumeric,
                    )
                    .map_err(E::JumpingReset)?;
            } else {
                jumping.result_valid_counter = 0;
            }
        }
        // ResetSystems 0x82dfffac..c4 inlines five writes to the retained
        // SkeletonController. +0x2ec is this owner, not LocationTest.
        owner
            .skeleton_controller()
            .lock()
            .map_err(|_| E::SkeletonControllerPoisoned)?
            .reset_after_teleport();
        // 0x82dfffcc calls the existing query invalidation on the retained
        // PlayerGrabSpline. No second query owner is created.
        owner
            .grab_spline()
            .lock()
            .map_err(|_| E::GrabSplinePoisoned)?
            .as_mut()
            .ok_or(E::MissingGrabSpline)?
            .try_invalidate_query(self.world_line_owners().0)
            .map_err(E::GrabInvalidate)?;
        // TeleportPlayerNow 0x82e00968 calls Skeleton::TeleportNow. Its first
        // callee is Skeleton::TeleportReset on the existing admitted Skeleton.
        self.skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_teleport_reset_skeleton_live(&self.backend, skeleton, &owner)
            .map_err(E::SkeletonTeleportReset)?;
        Ok(board_transform)
    }
}
