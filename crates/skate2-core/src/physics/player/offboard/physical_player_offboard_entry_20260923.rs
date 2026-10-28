//! First selected OffBoard entry, `PhysState_OffBoard::Enter` 0x82d9c1d8.
//! Ground-origin entry uses the retained processed COM velocity and linked cvar.
use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD_AIR, PHYSICS_STATE_OFFBOARD_LOCOMOTION};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, wipeout::WipeoutSkeletonOperation,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{
        PhysicalAssemblyWorld,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_physics_state::{CurrentPhysicalState, PhysicalStateStage},
    reckoning_numeric::{ReckoningClampError, ReckoningNumeric, try_clamp, try_length},
    simulation_assembly_backend::constraint_session::physical_skeleton_controller::{
        PhysicalSkeletonControllerError, SkeletonCollisionTransition,
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OffBoardEntryError<N> {
    Missing(&'static str),
    Poisoned,
    ForeignOwner,
    WrongState,
    Skeleton(super::wipeout_helpers::WipeoutHelperError<N>),
    Collision(PhysicalSkeletonControllerError),
    Grab(crate::physical_player_grab_spline::invalidate::GrabInvalidateError),
    Cvar(InputCvarError),
    Clamp(ReckoningClampError<N>),
    Numeric(N),
    WipeoutAttribute(String),
}
impl<N: std::fmt::Debug> std::fmt::Display for OffBoardEntryError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OffBoard Enter: {self:?}")
    }
}

fn offboard_state<N>(
    owner: &Arc<crate::physical_player_assembly::PhysicalPlayerAssemblyOwner>,
    f: impl FnOnce(&mut crate::physical_player_assembly::remaining_states::OffBoard),
) -> Result<(), OffBoardEntryError<N>> {
    let mut states = owner
        .remaining_states()
        .lock()
        .map_err(|_| OffBoardEntryError::Poisoned)?;
    let Some(StatePayload::OffBoard(offboard)) = states
        .states
        .get_mut(&StateKind::OffBoard)
        .and_then(|s| s.value.as_mut())
    else {
        return Err(OffBoardEntryError::Missing("retained OffBoard state"));
    };
    f(&mut **offboard);
    Ok(())
}

impl PhysicalAssemblyWorld {
    /// Native order through both GrabSpline invalidations. The retained source
    /// prefix is observable if a later collaborator is unavailable.
    pub(crate) fn try_enter_pair_offboard<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        refs: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), OffBoardEntryError<N::Error>> {
        use OffBoardEntryError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(|_| E::ForeignOwner)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::OffBoard
                || state.stage != PhysicalStateStage::OffBoardSelected
            {
                return Err(E::WrongState);
            }
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let last = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .last
            .ok_or(E::Missing("Processed last state"))?;
        {
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get_mut(&StateKind::OffBoard)
                .filter(|s| s.published)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("retained OffBoard state"));
            };
            let bindings = &offboard.base.physical;
            if !bindings.board.same_as(board)
                || !bindings.skeleton.same_as(skeleton)
                || !bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignOwner);
            }
            // Enter c230..c260. Constructor fields are reset again on every entry.
            offboard.last_collision_response_direction_valid = Some(false);
            offboard.grab_object = Some(false);
            offboard.is_locomotion_viable = Some(false);
            offboard.is_not_upright_entering_offboard = Some(false);
            offboard.is_moving_to_fast_entering_offboard = Some(false);
            offboard.last_collision_response_direction = Some(
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            );
            offboard.starting_com_vel = Some(
                crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            );
            if let Some(desc) = offboard.grab_spline_desc.as_mut() {
                desc.object_type = 0;
                desc.object_id = 0;
            } else {
                return Err(E::Missing("OffBoard grab descriptor"));
            }
            offboard.ground_blend = Some(0.0);
            offboard.auto_turn = Some(0.0);
        }
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardEntryFields;
        let reset = {
            let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let feet = remaining
                .feet_ik
                .as_mut()
                .ok_or(E::Missing("OffBoardFeetIK"))?;
            if !feet.binding.matches(&owner, &processed, skeleton) {
                return Err(E::ForeignOwner);
            }
            feet.consider_reset_before_disable_ik(last)
        };
        if reset {
            self.try_wipeout_skeleton_operation(
                board,
                skeleton,
                WipeoutSkeletonOperation::DisableIk,
                numeric,
            )
            .map_err(E::Skeleton)?;
        }
        owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_mut()
            .ok_or(E::Missing("OffBoardFeetIK"))?
            .finish_consider_reset(500);
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardFeetIk;
        self.try_pair_skeleton_collision_transition(
            board,
            skeleton,
            SkeletonCollisionTransition::Set(4),
        )
        .map_err(E::Collision)?;
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardCollision;
        {
            let mut grab = owner.grab_spline().lock().map_err(|_| E::Poisoned)?;
            let grab = grab.as_mut().ok_or(E::Missing("PlayerGrabSpline"))?;
            grab.try_invalidate_query(self.world_line_owners().0)
                .map_err(E::Grab)?;
            grab.interactable.lock().map_err(|_| E::Poisoned)?.ready = false; // InvalidateInteractableObjectQuery 82dcc32c.
        }
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardGrabQuery;
        {
            let mut remaining = owner.remaining().lock().map_err(|_| E::Poisoned)?;
            let jumping = remaining
                .jumping
                .as_mut()
                .ok_or(E::Missing("OffboardJumpingTest"))?;
            if !jumping.world.matches(&owner) {
                return Err(E::ForeignOwner);
            }
            // PDB JumpingTestResult+0/+16/+17; Enter c2a4..b0 clears these only.
            jumping.landing_position_valid = false;
            jumping.leap_of_faith = false;
            jumping.landing_position = [0.0; 4];
        }
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardJumpResult;
        if last != PHYSICS_STATE_OFFBOARD_LOCOMOTION {
            // Raw Enter c2cc/2d8/2e0: load Processed+0x260, then the live
            // cVar target, then call the common PhysicsUtility clamp.
            let velocity = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("Processed COM velocity+0x260"))?
                .value;
            let maximum = refs.try_ob_max_landing_speed_for_blend().map_err(E::Cvar)?;
            let clamped = try_clamp(velocity, maximum, numeric).map_err(E::Clamp)?;
            let mut states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
            let Some(StatePayload::OffBoard(offboard)) = states
                .states
                .get_mut(&StateKind::OffBoard)
                .and_then(|s| s.value.as_mut())
            else {
                return Err(E::Missing("retained OffBoard state"));
            };
            // ClampVectorWithinMaxLength stores full4. Attached-body history
            // can leave the native W address lane unknown; retain that knowledge
            // without replacing it with host scratch or blocking known XYZ.
            offboard.starting_com_vel = Some(clamped);
        }
        owner.physics_state().lock().map_err(|_| E::Poisoned)?.stage =
            PhysicalStateStage::OffBoardStartingVelocity;
        if last == PHYSICS_STATE_OFFBOARD_AIR {
            // CheckForWipeoutTriggers82d91858: physics_wipeout/default attribute,
            // then |Processed+0x260| (two-refinement length, zero when squared
            // is zero) and the ordered compare stored at +0x74.
            let limit = self
                .try_offboard_locomotion_speed_limit(board)
                .map_err(|e| E::WipeoutAttribute(format!("{e:?}")))?;
            let velocity = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .prior_output()
                .centre_of_mass_velocity()
                .ok_or(E::Missing("Processed COM velocity+0x260"))?
                .value;
            let speed = try_length(velocity, numeric).map_err(E::Numeric)?;
            offboard_state::<N::Error>(&owner, |offboard| {
                offboard.is_moving_to_fast_entering_offboard = Some(speed > limit);
            })?;
            // 82d91984..d4: kOB_MinUpVectorY > Processed+0xd0 lane1, stored at +0x73.
            let minimum = refs.try_ob_min_up_vector_y().map_err(E::Cvar)?;
            let up = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .ground_position_fields()
                .effective_animation
                .ok_or(E::Missing("Processed effective animation+0xc0"))?
                .columns()[1]
                .xyz[1];
            offboard_state::<N::Error>(&owner, |offboard| {
                offboard.is_not_upright_entering_offboard = Some(minimum > up);
            })?;
        }
        Ok(())
    }
}
