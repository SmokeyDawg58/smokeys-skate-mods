//! Retained selected Sleeping/`PhysicsGround` owners and source entry fields.
//! Other `PhysicsSystems` collaborators and ordinary state updates remain pending.
use crate::physics_state_ids::{
    PHYSICS_STATE_GRIND_5050, PHYSICS_STATE_GRIND_5_O, PHYSICS_STATE_GRIND_BACKSLASH,
    PHYSICS_STATE_GRIND_BOARDSLIDE, PHYSICS_STATE_GRIND_TIPSLIDE,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
};
use std::sync::{Arc, Weak};

#[derive(Clone)]
pub(crate) struct PhysicalStateBindings {
    pub(crate) owner: Weak<PhysicalPlayerAssemblyOwner>,
    pub(crate) board: SimulationSkaterAssemblyHandle,
    pub(crate) skeleton: SimulationSkaterAssemblyHandle,
}
impl PhysicalStateBindings {
    pub(crate) fn new(
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Self {
        Self {
            owner: Arc::downgrade(owner),
            board: board.clone(),
            skeleton: skeleton.clone(),
        }
    }
}

/// The five `PhysState_Grind_*` types, by `GetType` 0x82dfe548..568.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrindKind {
    Boardslide,
    FiftyFifty,
    Tipslide,
    FiveO,
    Backslash,
}
impl GrindKind {
    pub(crate) const fn id(self) -> i32 {
        match self {
            Self::Boardslide => PHYSICS_STATE_GRIND_BOARDSLIDE,
            Self::FiftyFifty => PHYSICS_STATE_GRIND_5050,
            Self::Tipslide => PHYSICS_STATE_GRIND_TIPSLIDE,
            Self::FiveO => PHYSICS_STATE_GRIND_5_O,
            Self::Backslash => PHYSICS_STATE_GRIND_BACKSLASH,
        }
    }
    pub(crate) const fn from_id(id: i32) -> Option<Self> {
        Some(match id {
            PHYSICS_STATE_GRIND_BOARDSLIDE => Self::Boardslide,
            PHYSICS_STATE_GRIND_5050 => Self::FiftyFifty,
            PHYSICS_STATE_GRIND_TIPSLIDE => Self::Tipslide,
            PHYSICS_STATE_GRIND_5_O => Self::FiveO,
            PHYSICS_STATE_GRIND_BACKSLASH => Self::Backslash,
            _ => return None,
        })
    }
    /// `eGrindType` published by each type's `FillPhysOut`.
    pub(crate) const fn grind_type(self) -> i32 {
        match self {
            Self::FiftyFifty => 0,
            Self::Boardslide => 1,
            Self::Tipslide => 2,
            Self::FiveO => 3,
            Self::Backslash => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CurrentPhysicalState {
    Unselected,
    Sleeping,
    Ground,
    GroundAnimation,
    Skitching,
    /// `PhysState_FollowPath`, 105.
    FollowPath,
    SlideGround,
    RevertGround,
    Air,
    KnownAir,
    WipeoutGround,
    Teleporting,
    OffBoard,
    OffBoardPushing,
    OffBoardAir,
    /// PhysState_OffBoard_Jump, entered as503; its GetType reports502.
    OffBoardJump,
    LandingOnDeck,
    OffBoardLocomotion,
    Boneless,
    FootPlant,
    HandPlant,
    /// `PhysState_Grind_*`, 400..404.
    Grind(GrindKind),
    /// `PhysState_Nonspecific`, 701.
    Nonspecific,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicalStateStage {
    Constructing,
    StatesRetained,
    SleepingSelected,
    Controller,
    TransitionFields,
    ActorQuery,
    Telemetry,
    GroundSelected,
    GroundExit,
    AirExit,
    KnownAirExit,
    GroundAnimationSelected,
    GroundAnimationExit,
    SkitchingSelected,
    SkitchingExit,
    FollowPathSelected,
    FollowPathExit,
    SlideGroundSelected,
    SlideGroundExit,
    RevertGroundSelected,
    RevertGroundExit,
    AirSelected,
    KnownAirSelected,
    WipeoutSelected,
    WipeoutExit,
    OffBoardExit,
    OffBoardPushingSelected,
    OffBoardPushingExit,
    OffBoardAirExit,
    OffBoardAirSelected,
    OffBoardJumpExit,
    OffBoardJumpSelected,
    LandingOnDeckSelected,
    OffBoardLocomotionSelected,
    OffBoardLocomotionExit,
    LandingOnDeckExit,
    BonelessSelected,
    BonelessExit,
    FootPlantSelected,
    HandPlantSelected,
    HandPlantExit,
    FootPlantExit,
    GrindSelected,
    GrindExit,
    NonspecificSelected,
    NonspecificExit,
    TeleportingExit,
    TeleportingSelected,
    OffBoardSelected,
    OffBoardEntryFields,
    OffBoardFeetIk,
    OffBoardCollision,
    OffBoardGrabQuery,
    OffBoardJumpResult,
    OffBoardStartingVelocity,
    SetStandard,
    EnableIkAndTilt,
    SkeletonCollision,
    LocalPrefix,
    ResetSpin,
    DisableAnimation,
    ProjectOmega,
    ResetSpeed,
    ResetWobble,
    Manual,
    ManualVelocity,
    LandingWobble,
    LandingForce,
    ResetPumping,
    Wipeout,
    EntryComplete,
}

/// Actual query calls f22c/f2ac remain separate. Logging is requested only when
/// the source category branch selects it; an absent implementation must fail.
pub(crate) trait PhysicalStateActorQuery {
    type Error;
    fn player_id(&mut self) -> Result<i32, Self::Error>;
    fn log_board_transition(&mut self, entering_offboard: bool) -> Result<(), Self::Error>;
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "retain independent one-shot allocation attempts and transition state"
)]
pub(crate) struct PhysicalPlayerPhysicsStateOwner {
    pub(crate) sleeping: Option<PhysicalStateBindings>,
    pub(crate) ground: Option<PhysicsGroundState>,
    pub(crate) air: Option<crate::physical_player_air_state::PhysicsAirState>,
    pub(crate) wipeout_ground:
        Option<crate::physical_player_wipeout_state::PhysicsWipeoutGroundState>,
    pub(crate) wipeout_attempted: bool,
    pub(crate) air_attempted: bool,
    pub(crate) sleeping_attempted: bool,
    pub(crate) ground_attempted: bool,
    pub(crate) current: CurrentPhysicalState,
    pub(crate) stage: PhysicalStateStage,
    pub(crate) state_count: Option<i32>,
    pub(crate) state_timer: Option<f32>,
    pub(crate) jphy_state: Option<i32>,
    pub(crate) transition_attempted: bool,
    /// SAME HiLOD+128. Its earlier constructor/input producers are not implied.
    pub(crate) frames_since_jump: Option<i32>,
}
impl PhysicalPlayerPhysicsStateOwner {
    pub(crate) fn new() -> Self {
        Self {
            sleeping: None,
            ground: None,
            air: None,
            wipeout_ground: None,
            wipeout_attempted: false,
            air_attempted: false,
            sleeping_attempted: false,
            ground_attempted: false,
            current: CurrentPhysicalState::Unselected,
            stage: PhysicalStateStage::Constructing,
            state_count: None,
            state_timer: None,
            jphy_state: None,
            transition_attempted: false,
            frames_since_jump: None,
        }
    }
    /// Actual Actor allocation825ff2d8 uses Sim: table82ff0eb0 has OR flag 0x100.
    /// `CategoryAllocator` 829735a4 carries that bit to `AllocHeapMem`'s full clear
    /// 82972340/3bc, before `HiLOD` constructor. `PhysicsSkater` allocations do not
    /// have this policy, so the separately allocated Ground unknowns stay unknown.
    pub(crate) fn from_sim_zeroed_allocation() -> Self {
        let mut state = Self::new();
        state.jphy_state = Some(0);
        state
    }
    /// Selected `HiLOD` scalar ctor3df0/3e0c; `JPhy`+174 has no proven early store.
    pub(crate) fn initialize_constructor_scalars(&mut self) {
        self.state_timer = Some(0.0);
        self.state_count = Some(0);
        self.frames_since_jump = Some(100);
    }
    pub(crate) fn try_construct_sleeping(
        &mut self,
        bindings: PhysicalStateBindings,
        allocated: bool,
    ) -> Result<(), PhysicalStateConstructionError> {
        if self.sleeping_attempted {
            return Err(PhysicalStateConstructionError::AlreadyAttempted);
        }
        self.sleeping_attempted = true;
        if allocated {
            self.sleeping = Some(bindings);
        }
        Ok(())
    }
    /// Air constructor82d8a9e0, call82e04688, before the Ground allocation.
    pub(crate) fn try_construct_air(
        &mut self,
        bindings: PhysicalStateBindings,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        allocated: bool,
    ) -> Result<(), PhysicalStateConstructionError> {
        if self.air_attempted {
            return Err(PhysicalStateConstructionError::AlreadyAttempted);
        }
        self.air_attempted = true;
        if allocated {
            self.air = Some(crate::physical_player_air_state::PhysicsAirState::new(
                bindings, vault,
            ));
        }
        Ok(())
    }
    /// Native constructor82d9fcb8, call82e04890, publication82e0489c.
    pub(crate) fn try_construct_wipeout(
        &mut self,
        bindings: PhysicalStateBindings,
        vault: &Arc<crate::attribute_runtime::AttributeRuntimeVault>,
        allocated: bool,
    ) -> Result<(), PhysicalStateConstructionError> {
        if self.wipeout_attempted {
            return Err(PhysicalStateConstructionError::AlreadyAttempted);
        }
        self.wipeout_attempted = true;
        if allocated {
            self.wipeout_ground = Some(
                crate::physical_player_wipeout_state::PhysicsWipeoutGroundState::new(
                    bindings, vault,
                ),
            );
        }
        Ok(())
    }
    /// Selected ctor82d8cef0, publication82e04ba4. This retains only the proven
    /// fields/references, never a fabricated trajectory/handplant owner.
    pub(crate) fn try_construct_ground(
        &mut self,
        bindings: PhysicalStateBindings,
        is_human: u8,
        allocated: bool,
    ) -> Result<(), PhysicalStateConstructionError> {
        if self.ground_attempted {
            return Err(PhysicalStateConstructionError::AlreadyAttempted);
        }
        self.ground_attempted = true;
        if allocated {
            self.ground = Some(PhysicsGroundState::new(bindings, is_human));
        }
        self.stage = PhysicalStateStage::StatesRetained;
        Ok(())
    }
    /// Actual constructor04d00/0c publishes Sleeping before `SetPhysicsState`.
    pub(crate) fn try_select_initial_sleeping(
        &mut self,
    ) -> Result<(), PhysicalStateConstructionError> {
        if self.current != CurrentPhysicalState::Unselected || !self.sleeping_attempted {
            return Err(PhysicalStateConstructionError::WrongPhase);
        }
        self.current = CurrentPhysicalState::Sleeping;
        self.stage = PhysicalStateStage::SleepingSelected;
        if self.sleeping.is_none() {
            return Err(PhysicalStateConstructionError::AllocationNull);
        }
        Ok(())
    }
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "state readback exercised by actual startup corpus; Actor tick consumer pending"
        )
    )]
    pub(crate) fn current(&self) -> CurrentPhysicalState {
        self.current
    }
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "state readback exercised by actual startup corpus; Actor tick consumer pending"
        )
    )]
    pub(crate) fn stage(&self) -> PhysicalStateStage {
        self.stage
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PhysicalStateConstructionError {
    AlreadyAttempted,
    AllocationNull,
    WrongPhase,
}

pub(crate) struct PhysicsGroundState {
    pub(crate) bindings: PhysicalStateBindings,
    pub(crate) fields: PhysicsGroundFields,
}
impl PhysicsGroundState {
    fn new(bindings: PhysicalStateBindings, is_human: u8) -> Self {
        Self {
            bindings,
            fields: PhysicsGroundFields::new(is_human),
        }
    }
}
#[expect(
    clippy::struct_excessive_bools,
    reason = "distinct native PhysicsGround flags with different write sites"
)]
pub(crate) struct PhysicsGroundFields {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained source ctor field; ordinary Ground Update consumer pending"
        )
    )]
    pub(crate) collision_force: [crate::skeleton_root_transforms::SkeletonRootVector; 2],
    pub(crate) grab_spline_type: i32,
    pub(crate) grab_spline_object: u32,
    #[expect(
        dead_code,
        reason = "retained source ctor field; ordinary Ground Update consumer pending"
    )]
    pub(crate) wipeout_direction: [f32; 4],
    pub(crate) collision_torque: crate::skeleton_root_transforms::SkeletonRootVector,
    pub(crate) hangup_force: Option<[f32; 4]>,
    pub(crate) anti_flip_torque: Option<crate::skeleton_root_transforms::SkeletonRootVector>,
    pub(crate) push_scalar: f32,
    pub(crate) damped_turn: f32,
    pub(crate) time_in_state: f32,
    pub(crate) collision_timer: f32,
    pub(crate) time_wall_riding: f32,
    pub(crate) valid_spawn_x: f32,
    pub(crate) valid_spawn_z: f32,
    pub(crate) time_to_skitch: f32,
    pub(crate) skitch_spline_height: f32,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained source ctor field; ordinary Ground Update consumer pending"
        )
    )]
    pub(crate) jump_direction: crate::skeleton_root_transforms::SkeletonRootVector,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained source ctor field; ordinary Ground Update consumer pending"
        )
    )]
    pub(crate) jump_com_height: f32,
    pub(crate) do_jump: bool,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained source ctor field; ordinary Ground Update consumer pending"
        )
    )]
    pub(crate) exiting_wallride: bool,
    pub(crate) is_push_accelerating: bool,
    pub(crate) request_push_stop: bool,
    pub(crate) anti_flip_nudge: bool,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained source ctor field; ordinary Ground Update consumer pending"
        )
    )]
    pub(crate) is_human: u8,
    pub(crate) player_provided_controls: bool,
    pub(crate) spawn_coordinates_set: bool,
    pub(crate) is_pinning: bool,
    pub(crate) was_pinning: bool,
    pub(crate) grab_object: bool,
    #[expect(
        dead_code,
        reason = "retained source ctor field; ordinary Ground Update consumer pending"
    )]
    pub(crate) grab_object_id: i32,
    pub(crate) hang_timer: Option<i32>,
    pub(crate) hang_pulse_countdown: Option<i32>,
    pub(crate) hung_up_counter: i32,
    pub(crate) anti_flip_frames: i32,
    #[expect(
        dead_code,
        reason = "retained source ctor field; ordinary Ground Update consumer pending"
    )]
    pub(crate) handplant_frames_against_direction: i32,
    #[expect(
        dead_code,
        reason = "retained source ctor field; ordinary Ground Update consumer pending"
    )]
    pub(crate) handplant_committed_direction: i32,
}
impl PhysicsGroundFields {
    fn new(is_human: u8) -> Self {
        Self {
            collision_force: [crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
                2],
            grab_spline_type: 0,
            grab_spline_object: 0,
            wipeout_direction: [0.0; 4],
            collision_torque: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            hangup_force: None,
            anti_flip_torque: None,
            push_scalar: 0.0,
            damped_turn: 0.0,
            time_in_state: 0.0,
            collision_timer: 0.0,
            time_wall_riding: 0.0,
            valid_spawn_x: 0.0,
            valid_spawn_z: 0.0,
            time_to_skitch: 0.0,
            skitch_spline_height: 0.0,
            jump_direction: crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]),
            jump_com_height: 0.0,
            do_jump: false,
            exiting_wallride: false,
            is_push_accelerating: false,
            request_push_stop: false,
            anti_flip_nudge: false,
            is_human,
            player_provided_controls: false,
            spawn_coordinates_set: false,
            is_pinning: false,
            was_pinning: false,
            grab_object: false,
            grab_object_id: 0,
            hang_timer: None,
            hang_pulse_countdown: None,
            hung_up_counter: 0,
            anti_flip_frames: 0,
            handplant_frames_against_direction: 0,
            handplant_committed_direction: 0,
        }
    }
    /// Enter82d99c80..94 only; no clearing other jump/vector/pinning fields.
    pub(crate) fn enter_local_prefix(&mut self) {
        self.grab_spline_type = 0;
        self.grab_spline_object = 0;
        self.do_jump = false;
        self.player_provided_controls = false;
        self.spawn_coordinates_set = false;
        self.was_pinning = false;
    }
    /// Final stores82d99fb4..9ff0, only after real Pumping/Wipeout operations.
    pub(crate) fn finish_enter(&mut self) {
        self.push_scalar = 1.0;
        self.damped_turn = 0.0;
        self.time_in_state = 0.0;
        self.collision_timer = 0.0;
        self.time_wall_riding = 0.0;
        self.collision_torque =
            crate::skeleton_root_transforms::SkeletonRootVector::known([0.0; 4]);
        self.hangup_force = Some([0.0; 4]);
        self.anti_flip_torque = Some(crate::skeleton_root_transforms::SkeletonRootVector::known(
            [0.0; 4],
        ));
        self.hang_timer = Some(0);
        self.hang_pulse_countdown = Some(0);
        self.hung_up_counter = 0;
        self.anti_flip_frames = 0;
        self.is_push_accelerating = false;
        self.anti_flip_nudge = false;
    }
}

#[cfg(test)]
#[path = "physical_player_physics_state_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../ground/physical_player_ground_antiflip_nudge_tests.rs"]
mod antiflip_nudge_tests;

#[cfg(test)]
#[path = "../ground/physical_player_ground_hang_pinning_tests.rs"]
mod hang_pinning_tests;

#[cfg(test)]
#[path = "../ground/physical_player_ground_finish_tests.rs"]
pub(crate) mod ground_finish_tests;
