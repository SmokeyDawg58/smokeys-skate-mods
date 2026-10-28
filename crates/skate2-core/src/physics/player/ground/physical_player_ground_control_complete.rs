//! One live Ground control invocation a048..a12c, before wallride force.
use super::{
    ground_control::GroundControlPrefixError,
    physical_wheel_material::PhysicalPlayerWheelMaterialError,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        board_construction::{SimulationConstructedBoardError, SimulationConstructedSteeringError},
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_phys_out::construct::board_fill::board_speed_from_squared_and_estimate,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::{ReckoningSinCosNumeric, sine::try_sine},
    skateboard_force_runtime::speed_wobble::calculate::{
        WobbleCalculateError, WobbleInput, WobbleServices, attributes::WobbleVault,
    },
    skateboard_steering::{
        SkateboardSteeringInput,
        calculate::{SteeringCalculationError, try_calculate},
        calculate_attributes::{SteeringCalculateAttributeError, SteeringCalculateVault},
    },
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum GroundControlCompleteError<N, R> {
    Prefix(GroundControlPrefixError<N>),
    Pair(SimulationSkeletonJumpPairError),
    Owner(SimulationConstructedBoardError),
    Missing(&'static str),
    Poisoned,
    Steering(SteeringCalculationError<&'static str, SteeringCalculateAttributeError>),
    Numeric(N),
    Wobble(WobbleCalculateError<AttributeRuntimeError, WobbleServiceError<R>>),
    SteeringStore(SimulationConstructedSteeringError<&'static str>),
    Material(PhysicalPlayerWheelMaterialError<&'static str>),
}
impl<N: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Display
    for GroundControlCompleteError<N, R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prefix(e) => write!(f, "{e}"),
            Self::Pair(e) => write!(f, "Ground control pair: {e:?}"),
            Self::Owner(e) => write!(f, "Ground control board: {e}"),
            Self::Missing(e) => write!(f, "Ground control missing {e}"),
            Self::Poisoned => f.write_str("Ground control owner lock poisoned"),
            Self::Steering(e) => write!(f, "{e}"),
            Self::Numeric(e) => write!(f, "Ground COM length numeric: {e:?}"),
            Self::Wobble(e) => write!(f, "{e}"),
            Self::SteeringStore(e) => write!(f, "Ground steering application: {e:?}"),
            Self::Material(e) => write!(f, "{e}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum WobbleServiceError<R> {
    Numeric(R),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl<R: std::fmt::Debug> std::fmt::Display for WobbleServiceError<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "wobble sine numeric: {error:?}"),
            Self::Cvar(error) => write!(f, "wobble debug cvar: {error:?}"),
            Self::Overlay(error) => write!(f, "wobble debug overlay: {error:?}"),
        }
    }
}
pub(super) struct Services<'a, R> {
    pub(super) references: &'a InputCvarReferences,
    pub(super) overlay: &'a DebugOverlaySlot,
    pub(super) numeric: &'a mut R,
}
impl<R: ReckoningSinCosNumeric> WobbleServices for Services<'_, R> {
    type Error = WobbleServiceError<R::Error>;
    fn sine(&mut self, angle: f32) -> Result<f32, Self::Error> {
        try_sine(angle, self.numeric).map_err(Self::Error::Numeric)
    }
    fn draw_enabled(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_draw_speed_wobble_info()
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
pub(crate) struct SteeringInput<'a>(pub(crate) &'a Arc<Mutex<SkeletonProcessedContact>>);
impl SkateboardSteeringInput for SteeringInput<'_> {
    type Error = &'static str;
    type Processed = Arc<Mutex<SkeletonProcessedContact>>;
    fn capture_processed(&mut self) -> Result<Self::Processed, Self::Error> {
        Ok(Arc::clone(self.0))
    }
    fn flags_8c4(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error> {
        let input = processed.lock().map_err(|_| "processed lock")?;
        let (value, known) = input.known_flags_8c4();
        if known & 0x0010_0000 == 0 {
            return Err("steering flipped flag8c4");
        }
        // This trait consumes only the selected bit; other bits remain unknown.
        Ok(value & 0x0010_0000)
    }
    fn flags_8c8(&mut self, processed: &Self::Processed) -> Result<u32, Self::Error> {
        processed
            .lock()
            .map_err(|_| "processed lock")?
            .known_flags_8c8()
            .ok_or("steering flags8c8")
    }
}
impl PhysicalAssemblyWorld {
    /// Captured damping and steering values stay in this invocation until their
    /// source consumers. No partial-stage float token can be paired with another board.
    pub(crate) fn try_update_pair_ground_control<N: ReckoningNumeric, R: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        references: &InputCvarReferences,
        numeric: &mut N,
        sine_numeric: &mut R,
    ) -> Result<(), GroundControlCompleteError<N::Error, R::Error>> {
        use GroundControlCompleteError as E;
        let damping = self
            .try_update_pair_ground_control_prefix(board, skeleton, references, numeric)
            .map_err(E::Prefix)?;
        let pool = self.skaters.as_mut().ok_or(E::Missing("skater pool"))?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let tilt = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let ground = state.ground.as_mut().ok_or(E::Missing("Ground state"))?;
            let input = processed.lock().map_err(|_| E::Poisoned)?;
            pool.try_with_ground_control_board(&self.backend, board, |runtime, manual| {
                try_calculate(
                    &*input,
                    &mut SteeringCalculateVault {
                        steering: &mut runtime.steering_attributes,
                        manual,
                    },
                    Some(&mut ground.fields.push_scalar),
                    Some(&mut ground.fields.damped_turn),
                )
            })
            .map_err(E::Owner)?
            .map_err(E::Steering)?
        };
        let com = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, _| {
                normal.current_animation().record().com_to_deck()
            })
            .map_err(E::Pair)?;
        // a0a8..a100: same squared/estimate/two VNMSUB refinements/zero select
        // as the already-reviewed board speed body. Unknown W never feeds dot3.
        let squared = numeric.dot_vector(com, com).map_err(E::Numeric)?;
        let estimate = numeric
            .reciprocal_sqrt_estimate(squared)
            .map_err(E::Numeric)?;
        let length = board_speed_from_squared_and_estimate(squared, estimate);
        let tilt = pool
            .try_with_ground_control_board(&self.backend, board, |runtime, _| {
                let wobble = runtime
                    .speed_wobble
                    .value_mut()
                    .ok_or(E::Missing("constructed SpeedWobble"))?;
                // UpdateSpeedWobble3077c..84: DT, truck tightness, speed.
                let input = processed.lock().map_err(|_| E::Poisoned)?;
                let dt = input
                    .physics_state_fields()
                    .time_delta
                    .ok_or(E::Missing("DT93c"))?;
                let truck_tightness = input
                    .animation_input()
                    .truck_tightness
                    .ok_or(E::Missing("truck9c4"))?;
                let speed = input
                    .prior_output()
                    .z_speed()
                    .ok_or(E::Missing("ZSpeed944"))?
                    .value;
                drop(input);
                let attributes = Arc::clone(wobble.attributes());
                wobble
                    .try_calculate(
                        WobbleInput {
                            tilt,
                            speed,
                            com_to_deck_length: length,
                            truck_tightness,
                            dt,
                        },
                        &mut WobbleVault(&attributes),
                        &mut Services {
                            references,
                            overlay: &self.debug_overlay,
                            numeric: sine_numeric,
                        },
                    )
                    .map_err(E::Wobble)
            })
            .map_err(E::Owner)??;
        self.try_with_constructed_board_truck_drives(board, |drives, _| {
            drives.set_speed_damping_factor(damping);
        })
        .map_err(E::Owner)?;
        self.try_update_constructed_board_steering(board, tilt, &mut SteeringInput(&processed))
            .map_err(E::SteeringStore)?;
        self.try_set_board_wheel_material(board, || {
            processed
                .lock()
                .map_err(|_| "processed lock")?
                .selected_surface()
                .and_then(|s| s.material.clone())
                .ok_or("selected wheel material")
        })
        .map_err(E::Material)?;
        // Exact next source boundary: a130 locals and WallRideForce at a1a0.
        Ok(())
    }
}
