//! Reached PhysState_OffBoard::UpdatePostPhysics 0x82d87290.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_construction::SimulationConstructedBoardError,
    },
    input_cvar_runtime::InputCvarError,
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_wipeout::{WipeoutOwnerError, ground_access::GroundOwnerError},
    reckoning_numeric::ReckoningNumeric,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum OffBoardPostPhysicsError<N> {
    Cvar(InputCvarError),
    Pool(SimulationSkaterPoolError),
    BoardAttributes(SimulationConstructedBoardError),
    Attribute(SkateboardAttributeError),
    Numeric(N),
    Poisoned,
    Missing(&'static str),
    Wipeout(WipeoutOwnerError),
    Request(GroundOwnerError),
    PendingDisplayInfo,
    HighSpeed(Box<crate::simulation_assembly_backend::constraint_session::ground_wipeout::GroundWipeoutRuntimeError<N>>),
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_offboard_locomotion_speed_limit(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<f32, OffBoardPostPhysicsError<std::convert::Infallible>> {
        use OffBoardPostPhysicsError as E;
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_ground_force_board(&self.backend, board, |_, _, attributes| {
                attributes.read_float(SkateboardAttributeClass::Wipeout, 0xf784_ac3b_a442_2ffd)
            })
            .map_err(E::BoardAttributes)?
            .map_err(E::Attribute)
    }
    pub(crate) fn try_check_pair_offboard_wipeout<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), OffBoardPostPhysicsError<N::Error>> {
        use OffBoardPostPhysicsError as E;
        if owner
            .inputs()
            .cvars
            .try_wipeout_offboard_enabled()
            .map_err(E::Cvar)?
        {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_set_state(4)
                .map_err(E::Wipeout)?;
            // Wipeout::CheckForOffBoardWipeout reads Skeleton+3da0 before
            // the physics_wipeout/default attribute pointer at 0x82dded20.
            let velocity = self
                .skeleton(skeleton)
                .map_err(E::Pool)?
                .general_update()
                .animation_world_velocity();
            let [x, y, z] = velocity.xyz;
            let speed_squared = numeric
                .dot3([x, y, z, 0.0], [x, y, z, 0.0])
                .map_err(E::Numeric)?;
            let threshold = self
                .skaters
                .as_mut()
                .ok_or(E::Missing("skater pool"))?
                .try_with_ground_force_board(&self.backend, board, |_, _, attributes| {
                    attributes.read_float(SkateboardAttributeClass::Wipeout, 0x3560_f8c1_0ae6_fd15)
                })
                .map_err(E::BoardAttributes)?
                .map_err(E::Attribute)?;
            let threshold_squared = threshold * threshold;
            if owner
                .inputs()
                .cvars
                .try_wipeout_display_offboard_info()
                .map_err(E::Cvar)?
            {
                return Err(E::PendingDisplayInfo);
            }
            if speed_squared > threshold_squared {
                let overlay = self.debug_overlay.clone();
                self.try_check_offboard_high_speed_wipeout(
                    board, skeleton, owner, &overlay, numeric,
                )
                .map_err(|error| E::HighSpeed(Box::new(error)))?;
            }
        }
        Ok(())
    }
    /// The high-speed collision branch remains explicit after the first native
    /// threshold; no low-speed result is assumed from the default cvars.
    pub(crate) fn try_update_offboard_postphysics<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
    ) -> Result<(), OffBoardPostPhysicsError<N::Error>> {
        use OffBoardPostPhysicsError as E;
        if owner
            .inputs()
            .cvars
            .try_enable_offboard_wipeouts()
            .map_err(E::Cvar)?
        {
            self.try_check_pair_offboard_wipeout(board, skeleton, owner, numeric)?;
        }
        // DoesRequireWipeout is the retained FeetIK+131 byte read.
        let feet_request = owner
            .remaining()
            .lock()
            .map_err(|_| E::Poisoned)?
            .feet_ik
            .as_ref()
            .ok_or(E::Missing("OffBoardFeetIK"))?
            .request_wipeout;
        if feet_request {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(25)
                .map_err(E::Request)?;
        }
        // Native checks OffBoard+74 before OffBoard+73. Each event increments
        // the same retained request ledger independently.
        let states = owner.remaining_states().lock().map_err(|_| E::Poisoned)?;
        let Some(StatePayload::OffBoard(offboard)) = states
            .states
            .get(&StateKind::OffBoard)
            .filter(|state| state.published)
            .and_then(|state| state.value.as_ref())
        else {
            return Err(E::Missing("OffBoard state"));
        };
        if offboard
            .is_moving_to_fast_entering_offboard
            .ok_or(E::Missing("OffBoard too-fast flag"))?
        {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(23)
                .map_err(E::Request)?;
        }
        if offboard
            .is_not_upright_entering_offboard
            .ok_or(E::Missing("OffBoard not-upright flag"))?
        {
            owner
                .wipeout()
                .lock()
                .map_err(|_| E::Poisoned)?
                .try_ground_request(28)
                .map_err(E::Request)?;
        }
        Ok(())
    }
}
