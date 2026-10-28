//! ContinuePassingIntersection82d20038: crossing/merging cars and exit-road
//! queue are considered before the shared obstacle/acceleration updates.
use super::{
    StopReason, VehicleMotion,
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
    obstacles::{FrontEntityGeometry, StopDistances},
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance, living_world_rain::RainMan,
    reckoning_numeric::ReckoningNumeric,
};

impl VehicleRoute {
    pub(crate) fn continue_passing_intersection<V: DrivingVehicles, N: ReckoningNumeric>(
        &self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        dt: f32,
        rain: &mut RainMan,
        mesh: &crate::living_world_mesh_store::MeshStore,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        vision: Option<FrontEntityGeometry>,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use DrivingError::Vehicle;
        rain.intersection(self.item_id, mesh)
            .map_err(DrivingError::Rain)?;
        let intersection = rain
            .intersections
            .get(&self.item_id)
            .ok_or(DrivingError::Intersection(self.item_id))?;
        let passage = intersection
            .passages
            .get(self.passage_index as usize)
            .ok_or(DrivingError::Passage(self.passage_index))?;
        motion.stopping_reason = StopReason::None;
        motion.travel_intention = true;
        motion.front_vehicle_distance = f32::MAX;
        let mut stop = StopDistances::new();
        let to = passage.record.word(0x5c)?;
        let lane = passage.record.word(0x60)?;
        let outgoing = intersection
            .outgoing
            .get(to as usize)
            .ok_or(DrivingError::Pin(to))?;
        let remaining = (passage.curve.length - self.item_position) - length * 0.5;
        for index in outgoing.available_passages(lane as usize)? {
            if index == self.passage_index {
                continue;
            }
            let other_passage = intersection
                .passages
                .get(index as usize)
                .ok_or(DrivingError::Passage(index))?;
            for &other in &other_passage.vehicles {
                if other == id {
                    continue;
                }
                vehicles.require_vehicle(other).map_err(Vehicle)?;
                if vehicles.rain_item_type(other).map_err(Vehicle)? != 0 {
                    continue;
                }
                let other_remaining = (other_passage.curve.length
                    - vehicles.current_position(other).map_err(Vehicle)?)
                    - vehicles.length(other).map_err(Vehicle)? * 0.5;
                if remaining > other_remaining {
                    // Source subtracts this vehicle's FULL length here, rather
                    // than the pair's half-length used on a shared passage.
                    let gap = remaining - (other_remaining + length);
                    let reason = vehicles.stopping_reason(other).map_err(Vehicle)?;
                    motion.update_front_vehicle(
                        gap,
                        motion.minimum_vehicle_distance,
                        reason,
                        &mut stop,
                        dt,
                        driver,
                    )?;
                    break;
                }
            }
        }
        let next = if passage.vehicles.is_empty() || passage.vehicles.last().copied() == Some(id) {
            0
        } else {
            let index = passage
                .vehicles
                .iter()
                .position(|&i| i == id)
                .ok_or(DrivingError::Unregistered(id))?;
            passage.vehicles.get(index + 1).copied().unwrap_or(0)
        };
        if let Some(road) = rain.roads.get(&outgoing.item_id) {
            let super_id = road.record.id(0x98)?;
            let super_road = rain
                .super_roads
                .get(&super_id)
                .ok_or(DrivingError::SuperRoad(super_id))?;
            let first = super_road
                .lane_vehicles
                .get(lane as usize)
                .ok_or(DrivingError::Lane(lane))?
                .first()
                .copied()
                .unwrap_or(0);
            if next != 0 || first != 0 {
                let mut gap = f32::MAX;
                let mut reason = StopReason::None;
                if next != 0 {
                    vehicles.require_vehicle(next).map_err(Vehicle)?;
                    if vehicles.rain_item_type(next).map_err(Vehicle)? == 0 {
                        let delta =
                            vehicles.current_position(next).map_err(Vehicle)? - self.item_position;
                        let candidate =
                            delta - (vehicles.length(next).map_err(Vehicle)? + length) * 0.5;
                        if gap > candidate {
                            gap = candidate;
                            reason = vehicles.stopping_reason(next).map_err(Vehicle)?;
                        }
                    }
                }
                if first != 0 {
                    vehicles.require_vehicle(first).map_err(Vehicle)?;
                    let position = vehicles.position(first).map_err(Vehicle)?;
                    if position >= 0. {
                        // ordered VMX, rejects NaN
                        let candidate = ((position + passage.curve.length) - self.item_position)
                            - (vehicles.length(first).map_err(Vehicle)? + length) * 0.5;
                        if gap > candidate {
                            gap = candidate;
                            reason = vehicles.stopping_reason(first).map_err(Vehicle)?;
                        }
                    }
                }
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
        }
        motion
            .update_front_entity(vision, &mut stop, dt, driver, numeric)
            .map_err(DrivingError::Obstacle)?;
        motion.update_acceleration(dt, stop.destination, stop.minimum, characteristics)?;
        if !motion.update_speed(dt, intersection.record.scalar(0x240)?) {
            return Err(DrivingError::MissingSkitchDelta);
        }
        let max_speed = passage.record.scalar(0x50)?;
        motion.cornering = Some(if motion.speed > max_speed {
            (motion.speed - max_speed) / max_speed
        } else {
            0.
        });
        Ok(())
    }
}
