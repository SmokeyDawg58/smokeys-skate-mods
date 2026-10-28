//! Vehicle UpdateLights82d18b20 / UpdateHornState82d18870. Lights are the same
//! byte changed by the route hooks. Horn output also informs the original ped.
use super::{
    METRES_TO_KM, StopReason, VehicleCharacteristics, VehicleMotion, driving::VehicleRoute,
    obstacles::FrontEntityGeometry,
};
use crate::{
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    reckoning_numeric::{ReckoningNumeric, try_length},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum HornState {
    Off = 0,
    EntityCuts = 1,
    EntityWaits = 2,
    VehicleCuts = 3,
    VehicleWaits = 4,
    Jam = 5,
    Alarm = 6,
}
pub(crate) struct VehicleSignals {
    pub(crate) horn: HornState,
    pub(crate) brake_timer: f32,
    pub(crate) blink_timer: f32,
}
/// Borrowed result of the existing vehicle vision test. None means its active
/// flag is clear. The compulsory-stop flag suppresses both the query and horn.
pub(crate) struct HornObstacle {
    pub(crate) geometry: FrontEntityGeometry,
    pub(crate) compulsory_stop: bool,
    pub(crate) pedestrian_id: Option<u32>,
}
pub(crate) trait HonkingPedestrians {
    type Error;
    /// GetPedestrian(id), and only if found IPedestrian::SetHonkingVehicle
    /// (vtable+104). Missing pedestrian is a source no-op, not a new spawn.
    fn set_honking_vehicle_if_present(
        &mut self,
        pedestrian: u32,
        vehicle: u32,
    ) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SignalError<A, N, P> {
    Attribute(A),
    Numeric(N),
    Pedestrian(P),
    Cvar(InputCvarError),
}
impl VehicleSignals {
    pub(crate) fn new() -> Self {
        Self {
            horn: HornState::Off,
            brake_timer: 0.,
            blink_timer: 0.,
        }
    }
    pub(crate) fn update_lights<A: VehicleCharacteristics>(
        &mut self,
        dt: f32,
        motion: &VehicleMotion,
        lights: &mut u8,
        characteristics: &A,
        cvars: &InputCvarReferences,
    ) -> Result<(), SignalError<A::Error, std::convert::Infallible, std::convert::Infallible>> {
        if motion.parked {
            *lights = if motion.alarming { 24 } else { 0 };
        } else {
            if motion.backing_up {
                *lights |= 4;
            } else {
                *lights &= !4;
            }
            if motion.speed > 0. && motion.acceleration < 0. {
                self.brake_timer += dt;
            } else if motion.acceleration > 0. {
                self.brake_timer = 0.;
            }
            if self.brake_timer
                > cvars
                    .try_vehicle_brake_light_delay()
                    .map_err(SignalError::Cvar)?
            {
                *lights |= 2;
            } else {
                *lights &= !2;
            }
        }
        if *lights & 24 == 0 {
            self.blink_timer = 0.;
            *lights &= 0x7f;
        } else {
            self.blink_timer += dt;
            let interval = characteristics
                .scalar(0x7a52_5925_b4b6_04ba)
                .map_err(SignalError::Attribute)?;
            if self.blink_timer > interval {
                self.blink_timer = 0.; // No remainder/catch-up loop in source.
                *lights ^= 0x80;
            }
        }
        Ok(())
    }
    pub(crate) fn update_horn<
        A: VehicleCharacteristics,
        N: ReckoningNumeric,
        P: HonkingPedestrians,
    >(
        &mut self,
        motion: &VehicleMotion,
        route: &VehicleRoute,
        vehicle_id: u32,
        observation: Option<&HornObstacle>,
        driver: &A,
        numeric: &mut N,
        pedestrians: &mut P,
    ) -> Result<(), SignalError<A::Error, N::Error, P::Error>> {
        self.horn = HornState::Off;
        if motion.parked {
            if motion.alarming {
                self.horn = HornState::Alarm;
            }
            return Ok(());
        }
        if !route.honk {
            return Ok(());
        }
        if motion.stopping_reason == StopReason::RuleBreakingVehicle {
            self.horn = HornState::VehicleCuts;
            return Ok(());
        }
        if motion.vehicle_waiting_timer
            > driver
                .scalar(0xbc1a_827c_2191_9c3b)
                .map_err(SignalError::Attribute)?
        {
            self.horn = if route.avoid_pileup {
                HornState::VehicleWaits
            } else {
                HornState::Jam
            };
            return Ok(());
        }
        let Some(observation) = observation else {
            return Ok(());
        };
        if observation.compulsory_stop {
            return Ok(());
        }
        let geometry = &observation.geometry;
        let delta = std::array::from_fn(|i| geometry.nearest_position[i] - geometry.position[i]);
        let distance = try_length(delta, numeric).map_err(SignalError::Numeric)?;
        let gap = (distance - geometry.nearest_radius) - geometry.vehicle_length * 0.5;
        if motion.entity_waiting_timer
            > driver
                .scalar(0xfe83_e2e0_a19a_9afe)
                .map_err(SignalError::Attribute)?
        {
            if let Some(pedestrian) = observation.pedestrian_id {
                pedestrians
                    .set_honking_vehicle_if_present(pedestrian, vehicle_id)
                    .map_err(SignalError::Pedestrian)?;
            }
            self.horn = HornState::EntityWaits;
        } else if geometry.vehicle_length * 2. > gap
            && motion.speed * METRES_TO_KM
                > driver
                    .scalar(0x4054_0e1a_0a5d_447a)
                    .map_err(SignalError::Attribute)?
        {
            self.horn = HornState::EntityCuts;
        }
        Ok(())
    }
}
