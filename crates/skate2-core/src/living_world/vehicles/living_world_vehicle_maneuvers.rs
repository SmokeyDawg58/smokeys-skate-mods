//! ContinueImpatience82d1b298, ChangingLane82d1b680, PullingOver82d1bc38,
//! PullingOut82d1c2e0. Lane membership changes remain in shared SuperRoad.
use super::{
    StopReason, VehicleMotion,
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
    obstacles::{FrontEntityGeometry, StopDistances},
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance, input_cvar_runtime::InputCvarReferences,
    living_world_rain::RainMan, reckoning_numeric::ReckoningNumeric,
};

impl VehicleRoute {
    pub(crate) fn continue_impatience<V: DrivingVehicles, N: ReckoningNumeric>(
        &self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        dt: f32,
        rain: &RainMan,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        vision: Option<FrontEntityGeometry>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use DrivingError::Vehicle;
        let road = self.super_road(rain)?;
        motion.travel_intention = true;
        motion.front_vehicle_distance = f32::MAX;
        motion.stopping_reason = StopReason::None;
        let mut stop = StopDistances::new();
        let distance = self.super_road_position - self.backup_position;
        if motion.speed > distance && stop.destination > distance {
            stop.destination = distance;
            stop.minimum = 0.;
            motion.stopping_reason = StopReason::Parking;
        }
        let rear = road
            .neighbor(self.road_lane, id, false)
            .map_err(DrivingError::Lanes)?;
        if rear != 0 {
            vehicles.require_vehicle(rear).map_err(Vehicle)?;
            let delta = self.super_road_position - vehicles.position(rear).map_err(Vehicle)?;
            // lfs82d1b5d8 reloads the VMX-written half-length SUM. Hex-Rays
            // incorrectly reports the overwritten stack constant0.5 here.
            let gap = delta - (vehicles.length(rear).map_err(Vehicle)? + length) * 0.5;
            let reason = vehicles.stopping_reason(rear).map_err(Vehicle)?;
            motion.update_front_vehicle(
                gap,
                motion.minimum_vehicle_distance * 0.5,
                reason,
                &mut stop,
                dt,
                driver,
            )?;
        } else {
            motion.vehicle_waiting_timer = 0.;
        }
        finish(
            motion,
            stop,
            dt,
            road.data.speed_limit,
            characteristics,
            driver,
            vision,
            numeric,
        )
    }
    /// Shared source-identical sequence of ChangingLane and PullingOver. Only
    /// PullingOver adds the curve-end stop and optional parking-lane check.
    pub(crate) fn continue_lane_maneuver<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        pulling_over: bool,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        dt: f32,
        rain: &mut RainMan,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        vision: Option<FrontEntityGeometry>,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use DrivingError::Vehicle;
        let super_id = self.super_road::<V::Error, N::Error>(rain)?.data.id;
        let road = rain
            .super_roads
            .get_mut(&super_id)
            .ok_or(DrivingError::SuperRoad(super_id))?;
        motion.stopping_reason = StopReason::None;
        motion.travel_intention = true;
        motion.front_vehicle_distance = f32::MAX;
        let mut stop = StopDistances::new();
        if pulling_over {
            let distance = self
                .lane_change_passage
                .curve
                .length
                .ok_or(DrivingError::MissingLaneChangeLength)?
                - self.lane_change_position;
            if motion.speed > distance && stop.destination > distance {
                stop.destination = distance;
                stop.minimum = 0.;
                motion.stopping_reason = StopReason::Parking;
            }
        }
        let origin = self.lane_change_passage.origin_lane;
        let destination = self.lane_change_passage.destination_lane;
        let mut old_front = 0;
        if self.road_lane == origin {
            let half_curve = self
                .lane_change_passage
                .curve
                .length
                .ok_or(DrivingError::MissingLaneChangeLength)?
                * 0.5;
            if self.lane_change_position - length * 0.5 > half_curve {
                road.unregister_vehicle(id, origin, length + motion.minimum_vehicle_distance)
                    .map_err(DrivingError::Lanes)?;
                self.road_lane = destination;
            } else {
                old_front = road
                    .neighbor(origin, id, true)
                    .map_err(DrivingError::Lanes)?;
            }
        }
        let check_destination = if pulling_over {
            cvars
                .try_check_vehicles_in_parking_lane()
                .map_err(DrivingError::Cvar)?
        } else {
            true
        };
        let new_front = if check_destination {
            road.neighbor(destination, id, true)
                .map_err(DrivingError::Lanes)?
        } else {
            0
        };
        if old_front != 0 || new_front != 0 {
            let mut gap = f32::MAX;
            let mut reason = StopReason::None;
            if old_front != 0 {
                vehicles.require_vehicle(old_front).map_err(Vehicle)?;
                // Actual82d1b9a4 /82d1c040 reads the OTHER vehicle's DynPassage
                // valid byte at10f1. This is not its parked/alarming flag.
                if vehicles
                    .lane_change_passage_valid(old_front)
                    .map_err(Vehicle)?
                {
                    let delta =
                        vehicles.position(old_front).map_err(Vehicle)? - self.super_road_position;
                    let candidate =
                        delta - (vehicles.length(old_front).map_err(Vehicle)? + length) * 0.5;
                    if gap > candidate {
                        gap = candidate;
                        reason = vehicles.stopping_reason(old_front).map_err(Vehicle)?;
                    }
                }
            }
            if new_front != 0 {
                vehicles.require_vehicle(new_front).map_err(Vehicle)?;
                let delta =
                    vehicles.position(new_front).map_err(Vehicle)? - self.super_road_position;
                let candidate =
                    delta - (vehicles.length(new_front).map_err(Vehicle)? + length) * 0.5;
                if gap > candidate {
                    gap = candidate;
                    reason = vehicles.stopping_reason(new_front).map_err(Vehicle)?;
                }
            }
            let minimum = motion.minimum_vehicle_distance * if pulling_over { 0.25 } else { 0.5 };
            motion.update_front_vehicle(gap, minimum, reason, &mut stop, dt, driver)?;
        } else {
            motion.vehicle_waiting_timer = 0.;
        }
        finish(
            motion,
            stop,
            dt,
            road.data.speed_limit,
            characteristics,
            driver,
            vision,
            numeric,
        )
    }
    pub(crate) fn continue_pulling_out<V: DrivingVehicles, N: ReckoningNumeric>(
        &self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        dt: f32,
        rain: &RainMan,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        vision: Option<FrontEntityGeometry>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use DrivingError::Vehicle;
        let road = self.super_road(rain)?;
        motion.front_vehicle_distance = f32::MAX;
        motion.travel_intention = true;
        motion.stopping_reason = StopReason::None;
        let mut stop = StopDistances::new();
        let front = road
            .neighbor(self.lane_change_passage.destination_lane, id, true)
            .map_err(DrivingError::Lanes)?;
        if front != 0 {
            vehicles.require_vehicle(front).map_err(Vehicle)?;
            let delta = vehicles.position(front).map_err(Vehicle)? - self.super_road_position;
            let gap = delta - (vehicles.length(front).map_err(Vehicle)? + length) * 0.5;
            let reason = vehicles.stopping_reason(front).map_err(Vehicle)?;
            motion.update_front_vehicle(
                gap,
                motion.minimum_vehicle_distance * 0.25,
                reason,
                &mut stop,
                dt,
                driver,
            )?;
        } else {
            motion.vehicle_waiting_timer = 0.;
        }
        finish(
            motion,
            stop,
            dt,
            road.data.speed_limit,
            characteristics,
            driver,
            vision,
            numeric,
        )
    }
}
fn finish<E, N: ReckoningNumeric>(
    motion: &mut VehicleMotion,
    mut stop: StopDistances,
    dt: f32,
    speed_limit: f32,
    characteristics: &AttributeRuntimeInstance,
    driver: &AttributeRuntimeInstance,
    vision: Option<FrontEntityGeometry>,
    numeric: &mut N,
) -> Result<(), DrivingError<E, N::Error>> {
    motion
        .update_front_entity(vision, &mut stop, dt, driver, numeric)
        .map_err(DrivingError::Obstacle)?;
    motion.update_acceleration(dt, stop.destination, stop.minimum, characteristics)?;
    if !motion.update_speed(dt, speed_limit) {
        return Err(DrivingError::MissingSkitchDelta);
    }
    Ok(())
}
