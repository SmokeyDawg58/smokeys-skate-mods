//! Actual `PhysicsAir::Update`82d9f4b0 through Reckoning82d9f6d0.
//! No Skeleton or full state completion is issued by this prefix.
use super::{
    air_launch_runtime::AirLaunchRuntimeError, physical_process_input_selector::SelectorPollError,
    skeleton_air_collision::PairAirCollisionError,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::DebugOverlayError,
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, reckoning::ReckoningCvarAdapter},
    physical_player_air_state::spin_target::AirSpinTargetError,
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::air_post_solve::{AirStateAccessError, validate_air},
    physical_player_reckoning_owner::{PhysicalPlayerReckoningPhase, air_live::AirOwnerError},
    reckoning::grounded_input::{GroundInputError, GroundInputRead},
    reckoning_numeric::{ReckoningNumeric, try_unsigned_angle, wrap_turns_to_radians},
    reckoning_sin_cos::ReckoningSinCosNumeric,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum AirUpdatePrefixError<N, R> {
    Missing(&'static str),
    Poisoned,
    State(AirStateAccessError),
    Pair(SimulationSkeletonJumpPairError),
    Collision(PairAirCollisionError),
    Launch(AirLaunchRuntimeError<N, R>),
    Selector(SelectorPollError),
    CompleteSelector(super::physical_selector_finish_runtime::SelectorCompletionError<N, R>),
    Input(GroundInputError),
    Numeric(N),
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Spin(AirSpinTargetError),
    Reckoning(AirOwnerError<InputCvarError, N, R>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display for AirUpdatePrefixError<N, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("AirUpdatePrefixError: Poisoned"),
            Self::Missing(error) => write!(f, "AirUpdatePrefixError Missing: {error:?}"),
            Self::State(error) => write!(f, "AirUpdatePrefixError State: {error:?}"),
            Self::Pair(error) => write!(f, "AirUpdatePrefixError Pair: {error:?}"),
            Self::Collision(error) => write!(f, "AirUpdatePrefixError Collision: {error:?}"),
            Self::Launch(error) => write!(f, "AirUpdatePrefixError Launch: {error:?}"),
            Self::Selector(error) => write!(f, "AirUpdatePrefixError Selector: {error:?}"),
            Self::CompleteSelector(error) => {
                write!(f, "AirUpdatePrefixError CompleteSelector: {error:?}")
            }
            Self::Input(error) => write!(f, "AirUpdatePrefixError Input: {error:?}"),
            Self::Numeric(error) => write!(f, "AirUpdatePrefixError Numeric: {error:?}"),
            Self::Attribute(error) => write!(f, "AirUpdatePrefixError Attribute: {error:?}"),
            Self::Cvar(error) => write!(f, "AirUpdatePrefixError Cvar: {error:?}"),
            Self::Overlay(error) => write!(f, "AirUpdatePrefixError Overlay: {error:?}"),
            Self::Spin(error) => write!(f, "AirUpdatePrefixError Spin: {error:?}"),
            Self::Reckoning(error) => write!(f, "AirUpdatePrefixError Reckoning: {error:?}"),
        }
    }
}

pub(crate) struct AirBeforeComCorrection {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) reckoning: PhysicalPlayerReckoningPhase,
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "source ordered lazy reads and retained writes through Air Reckoning"
    )]
    pub(crate) fn try_begin_pair_air_update<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        grind_policy: super::ground_pre_solve::GroundPreparationGrindPolicy,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<AirBeforeComCorrection, AirUpdatePrefixError<N::Error, R::Error>> {
        use AirUpdatePrefixError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            validate_air(&state, &owner, board, skeleton).map_err(E::State)?;
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        {
            let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_ref()
                .ok_or(E::Missing("selector"))?;
            if !selector.bindings.matches(&owner, &processed) {
                return Err(E::Selector(SelectorPollError::ForeignOwner));
            }
        }
        self.try_update_pair_air_collision(board, skeleton)
            .map_err(E::Collision)?;
        let timer = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let fields = &mut state.air.as_mut().ok_or(E::Missing("Air state"))?.fields;
            fields.trajectory_timer = fields.trajectory_timer.wrapping_sub(1);
            fields.trajectory_timer
        };
        // 82d9f4d4 addic./bgt: title code runs in 32-bit mode, so CR0 compares the
        // low word as signed. A relaunch blocked by skeleton contact retries every
        // frame while the timer is <= 0, which recovers Air to KnownAir.
        if timer <= 0 {
            let contact = processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .flag8c4(0x8000)
                .map_err(E::Input)?;
            if contact && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                eprintln!("known air: air relaunch skipped (skeleton contact)");
            }
            if !contact {
                let active = processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .physics_state_fields()
                    .physics
                    .ok_or(E::Missing("physics state"))?
                    != 0;
                if !active && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                    eprintln!("known air: air relaunch skipped (physics state 0)");
                }
                if active {
                    let started = owner
                        .trajectories()
                        .lock()
                        .map_err(|_| E::Poisoned)?
                        .onboard
                        .as_ref()
                        .ok_or(E::Missing("selector"))?
                        .just_started_query;
                    if std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
                        eprintln!(
                            "known air: air relaunch {}",
                            if started {
                                "skipped (query in flight)"
                            } else {
                                "launched"
                            }
                        );
                    }
                    if !started {
                        self.try_launch_pair_air_trajectory(
                            board, skeleton, ground, numeric, rounding,
                        )
                        .map_err(E::Launch)?;
                        owner
                            .physics_state()
                            .lock()
                            .map_err(|_| E::Poisoned)?
                            .air
                            .as_mut()
                            .ok_or(E::Missing("Air state"))?
                            .fields
                            .trajectory_timer = 7;
                    }
                }
            }
        }
        // Air ignores returned readiness; Update1 publishes it separately.
        self.try_complete_pair_selector(board, skeleton, ground, numeric, rounding, grind_policy)
            .map_err(E::CompleteSelector)?;
        let up = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("Reckoning"))?
            .up();
        let normal = {
            let trajectories = owner.trajectories().lock().map_err(|_| E::Poisoned)?;
            let selector = trajectories
                .onboard
                .as_ref()
                .ok_or(E::Missing("selector"))?;
            if selector.suggested_landing_normal_valid {
                selector.suggested_landing_normal
            } else {
                up
            }
        };
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .air
            .as_mut()
            .ok_or(E::Missing("Air state"))?
            .fields
            .landing_normal = normal;
        let angle = try_unsigned_angle(normal, up, numeric).map_err(E::Numeric)?;
        let angle = wrap_turns_to_radians(angle * f32::from_bits(0x3e22_f983));
        let mut blend = {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let air = state.air.as_ref().ok_or(E::Missing("Air state"))?;
            let limit = air
                .try_air_scalar(0xc052_d34b_6b05_2f44)
                .map_err(E::Attribute)?;
            if angle > limit {
                0.0
            } else {
                air.try_air_scalar(0x724c_206c_94a7_3043)
                    .map_err(E::Attribute)?
            }
        };
        let timer = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .state_timer
            .ok_or(E::Missing("state timer"))?;
        if timer > 0.0 {
            if processed
                .lock()
                .map_err(|_| E::Poisoned)?
                .flag8c4(0x4000)
                .map_err(E::Input)?
            {
                owner
                    .physics_state()
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .air
                    .as_mut()
                    .ok_or(E::Missing("Air state"))?
                    .fields
                    .use_com_velocity = false;
            }
        } else {
            blend = 0.0;
        }
        if owner
            .inputs()
            .cvars
            .try_draw_physics_air_debug_info()
            .map_err(E::Cvar)?
        {
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        let spin = owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .air
            .as_ref()
            .ok_or(E::Missing("Air state"))?
            .try_body_spin_target()
            .map_err(E::Spin)?;
        let reckoning = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_update_air_live(
                &processed,
                normal,
                blend,
                spin,
                0.0,
                &mut ReckoningCvarAdapter(&owner.inputs().cvars),
                numeric,
                rounding,
                #[cfg(feature = "skate3-runtime")]
                owner.skate3_tuning.lock().map_err(|_| E::Poisoned)?.as_deref(),
            )
            .map_err(E::Reckoning)?;
        Ok(AirBeforeComCorrection { owner, reckoning })
    }
}
