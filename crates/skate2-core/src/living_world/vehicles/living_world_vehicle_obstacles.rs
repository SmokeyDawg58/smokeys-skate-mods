//! Vehicle.UpdateFrontVehicleParams82d18d20 / UpdateFrontEntityParams82d18ef8.
//! Shared motion fields; the vision geometry is a borrowed observation, not a
//! replacement perception system. No world query or guessed obstacle radius.
use super::{METRES_TO_KM, StopReason, VehicleCharacteristics, VehicleMotion};
use crate::reckoning_numeric::{ReckoningNumeric, try_length};

const WAITING_TOLERANCE_SPEED: u64 = 0x4054_0e1a_0a5d_447a;

/// Stack locals in driving callbacks, reset to MAX at each decision update.
pub(crate) struct StopDistances {
    pub(crate) destination: f32,
    pub(crate) minimum: f32,
}
impl StopDistances {
    pub(crate) fn new() -> Self {
        Self {
            destination: f32::MAX,
            minimum: f32::MAX,
        }
    }
}
pub(crate) struct FrontEntityGeometry {
    pub(crate) position: [f32; 4],
    pub(crate) nearest_position: [f32; 4],
    pub(crate) nearest_radius: f32,
    pub(crate) vehicle_length: f32,
}
#[derive(Debug)]
pub(crate) enum ObstacleError<A, N> {
    Attribute(A),
    Numeric(N),
}
impl VehicleMotion {
    pub(crate) fn update_front_vehicle<A: VehicleCharacteristics>(
        &mut self,
        distance: f32,
        minimum: f32,
        front_reason: StopReason,
        stop: &mut StopDistances,
        dt: f32,
        driver: &A,
    ) -> Result<(), A::Error> {
        if distance < self.front_vehicle_distance {
            self.front_vehicle_distance = distance;
        }
        // Scalar bgt skips the branch; unlike ordered <=, NaN reaches it.
        if !(distance > self.speed + minimum) {
            self.travel_intention = self.travel_intention && distance > minimum;
            if stop.destination > distance {
                stop.destination = distance;
                stop.minimum = minimum;
                self.stopping_reason = match front_reason {
                    StopReason::Intersection | StopReason::RuleBreakingVehicle => {
                        StopReason::Intersection
                    }
                    _ => StopReason::Vehicle,
                };
            }
        }
        if self.travel_intention {
            self.vehicle_waiting_timer = 0.;
        } else if self.stopping_reason == StopReason::Vehicle {
            let speed = self.speed * METRES_TO_KM;
            if speed < driver.scalar(WAITING_TOLERANCE_SPEED)? {
                self.vehicle_waiting_timer = dt + self.vehicle_waiting_timer;
            }
        }
        Ok(())
    }
    pub(crate) fn update_front_entity<A: VehicleCharacteristics, N: ReckoningNumeric>(
        &mut self,
        observation: Option<FrontEntityGeometry>,
        stop: &mut StopDistances,
        dt: f32,
        driver: &A,
        numeric: &mut N,
    ) -> Result<(), ObstacleError<A::Error, N::Error>> {
        let Some(observation) = observation else {
            self.entity_waiting_timer = 0.;
            return Ok(());
        };
        let delta: [f32; 4] =
            std::array::from_fn(|i| observation.nearest_position[i] - observation.position[i]);
        let length = try_length(delta, numeric).map_err(ObstacleError::Numeric)?;
        let distance = (length - observation.nearest_radius) - observation.vehicle_length * 0.5;
        if stop.destination > distance {
            stop.destination = distance;
            stop.minimum = self.minimum_entity_distance;
            self.stopping_reason = StopReason::Entity;
        }
        // Source82d19060 requires travel intention TRUE. Do not invert this to
        // match the vehicle-wait branch; this helper does not alter intention.
        if self.stopping_reason == StopReason::Entity && self.travel_intention {
            let speed = self.speed * METRES_TO_KM;
            if speed
                < driver
                    .scalar(WAITING_TOLERANCE_SPEED)
                    .map_err(ObstacleError::Attribute)?
            {
                self.entity_waiting_timer = dt + self.entity_waiting_timer;
            }
        }
        Ok(())
    }
    /// ContinueStayingParked.Update82d162d0; graph owns entry/exit parked flag.
    pub(crate) fn continue_staying_parked(&mut self, dt: f32) {
        self.travel_intention = false;
        self.stopping_reason = StopReason::Parking;
        self.front_vehicle_distance = f32::MAX;
        self.acceleration = 0.;
        self.speed = 0.;
        if self.alarming {
            self.alarm_timer += dt;
            self.parking_timer = 0.;
        } else {
            self.alarm_timer = 0.;
            self.parking_timer = dt + self.parking_timer;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Driver;
    impl VehicleCharacteristics for Driver {
        type Error = u64;
        fn acceleration_increment(&self) -> Result<f32, u64> {
            Err(0)
        }
        fn scalar(&self, key: u64) -> Result<f32, u64> {
            if key == WAITING_TOLERANCE_SPEED {
                Ok(5.)
            } else {
                Err(key)
            }
        }
    }
    #[test]
    fn queue_for_red_light_is_not_vehicle_impatience() {
        let mut vehicle = VehicleMotion::new();
        vehicle.front_vehicle_distance = f32::MAX;
        vehicle.vehicle_waiting_timer = 7.;
        let mut stop = StopDistances::new();
        vehicle
            .update_front_vehicle(
                1.,
                2.,
                StopReason::RuleBreakingVehicle,
                &mut stop,
                1.,
                &Driver,
            )
            .unwrap();
        assert_eq!(vehicle.stopping_reason, StopReason::Intersection);
        assert!(!vehicle.travel_intention);
        assert_eq!(vehicle.vehicle_waiting_timer, 7.);
        vehicle
            .update_front_vehicle(0.5, 2., StopReason::None, &mut stop, 1., &Driver)
            .unwrap();
        assert_eq!(vehicle.stopping_reason, StopReason::Vehicle);
        assert_eq!(vehicle.vehicle_waiting_timer, 8.);
    }
    #[test]
    fn parked_alarm_restarts_parking_wait() {
        let mut vehicle = VehicleMotion::new();
        vehicle.parking_timer = 4.;
        vehicle.alarming = true;
        vehicle.continue_staying_parked(0.5);
        assert_eq!(vehicle.parking_timer, 0.);
        assert_eq!(vehicle.alarm_timer, 0.5);
        vehicle.alarming = false;
        vehicle.continue_staying_parked(0.25);
        assert_eq!(vehicle.parking_timer, 0.25);
        assert_eq!(vehicle.alarm_timer, 0.);
    }
}
