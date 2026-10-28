//! MakeNewActionDecision82d190c8 and ReevaluateActionDecisions82d1d850.
use super::{
    VehicleMotion,
    attributes::{layout, scalar},
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    living_world_grind_services::vehicle_graph::conditions::VehicleAction,
    living_world_rain::{RainMan, vehicle_lanes::LaneChoiceError},
    native_skater_random::rw_math::{RwRandomLockError, bernoulli_choice},
};
impl VehicleRoute {
    pub(crate) fn make_new_action_decision<V: DrivingVehicles, N>(
        &mut self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        rain: &mut RainMan,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        random_word: &mut impl FnMut() -> Result<u32, RwRandomLockError>,
    ) -> Result<bool, DrivingError<V::Error, N>> {
        use DrivingError::{GlobalRandom, Lanes, Vehicle};
        if self.registered_in_intersection {
            return Ok(false);
        }
        let Some(road) = rain.roads.get(&self.item_id) else {
            return Ok(false);
        };
        let super_id = road.record.id(0x98)?;
        let road = rain
            .super_roads
            .get_mut(&super_id)
            .ok_or(DrivingError::SuperRoad(super_id))?;
        if self.action != VehicleAction::None {
            return Ok(false);
        }
        let impatient = bernoulli_choice(
            scalar(driver, 0x9366_c677_55a2_4d89)?,
            random_word().map_err(GlobalRandom)?,
        );
        if impatient
            && !self.lane_change_passage.valid
            && motion.vehicle_waiting_timer > layout(driver, 0x20)?
        {
            let lane = road
                .nearest_most_empty_lane(self.road_lane, false, random_word)
                .map_err(choice_error)?;
            let front = road.neighbor(self.road_lane, id, true).map_err(Lanes)?;
            let rear = road.neighbor(self.road_lane, id, false).map_err(Lanes)?;
            let mut allowed = true;
            for other in [front, rear] {
                if other != 0 {
                    vehicles.require_vehicle(other).map_err(Vehicle)?;
                    if vehicles.lane_change_passage_valid(other).map_err(Vehicle)?
                        && vehicles.backup_position(other).map_err(Vehicle)? < 0.
                    {
                        allowed = false;
                    }
                }
            }
            if allowed
                && lane != self.road_lane
                && road
                    .can_place_vehicle(
                        self.super_road_position,
                        lane,
                        length,
                        motion.speed,
                        true,
                        vehicles,
                    )
                    .map_err(Lanes)?
            {
                motion.vehicle_waiting_timer = 0.;
                motion.entity_waiting_timer = 0.;
                self.change_to_lane = lane;
                self.action = VehicleAction::Impatience;
                return Ok(true);
            }
        }
        // Scalar ble rejects unordered here as well as <=.
        if self.action == VehicleAction::Impatience
            || !(self.action_decision_mileage > layout(characteristics, 0x30)?)
        {
            return Ok(false);
        }
        self.action_decision_mileage = 0.;
        if bernoulli_choice(layout(driver, 0x1c)?, random_word().map_err(GlobalRandom)?) {
            // Density draw is consumed BEFORE checking the road's lane-change flag.
            let check_density =
                bernoulli_choice(layout(driver, 0x18)?, random_word().map_err(GlobalRandom)?);
            if road.data.flags & 2 != 0 {
                let lane = road
                    .nearest_most_empty_lane(self.road_lane, check_density, random_word)
                    .map_err(choice_error)?;
                if lane != self.road_lane
                    && road
                        .can_place_vehicle(
                            self.super_road_position,
                            lane,
                            length,
                            motion.speed,
                            true,
                            vehicles,
                        )
                        .map_err(Lanes)?
                {
                    self.change_to_lane = lane;
                    self.action = VehicleAction::ChangeLane;
                    return Ok(true);
                }
            }
        }
        let parking_factor = if !motion.being_skitched || self.park_when_skitched {
            1.
        } else {
            0.
        };
        let park = bernoulli_choice(
            parking_factor * scalar(driver, 0x559b_a807_f95f_f93e)?,
            random_word().map_err(GlobalRandom)?,
        );
        if park {
            self.pull_over_position = -1.;
            if road.data.flags & 1 != 0 && self.road_lane == road.data.lanes.wrapping_sub(1) {
                let mut query = super::query::CurrentVehicle {
                    id,
                    length,
                    position: self.super_road_position,
                    motion,
                    others: vehicles,
                };
                let position = road.best_parking_distance(id, &mut query).map_err(Lanes)?;
                if position >= motion.minimum_parking_distance + self.super_road_position {
                    self.pull_over_position = position;
                    road.register_parking(id, position).map_err(Lanes)?;
                    self.action = VehicleAction::Park;
                    self.registered_in_parking = true;
                    return Ok(true);
                }
            }
        }
        self.action = VehicleAction::None;
        Ok(true)
    }
    pub(crate) fn reevaluate_action_decisions<V: DrivingVehicles, N>(
        &mut self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        rain: &mut RainMan,
        vehicles: &mut V,
        characteristics: &AttributeRuntimeInstance,
        driver: &AttributeRuntimeInstance,
        random_word: &mut impl FnMut() -> Result<u32, RwRandomLockError>,
    ) -> Result<(), DrivingError<V::Error, N>> {
        self.make_new_action_decision(
            motion,
            id,
            length,
            rain,
            vehicles,
            characteristics,
            driver,
            random_word,
        )?;
        if self.registered_in_parking
            && !motion.parking_started
            && motion.being_skitched
            && !self.park_when_skitched
        {
            if let Some(road) = rain.super_roads.get_mut(&self.super_road_id) {
                self.pull_over_position = -1.;
                road.unregister_parking(id).map_err(DrivingError::Lanes)?;
            }
            self.action = VehicleAction::None;
            self.action_decision_mileage = 0.;
            self.registered_in_parking = false;
        }
        Ok(())
    }
}
fn choice_error<E, N>(e: LaneChoiceError<RwRandomLockError>) -> DrivingError<E, N> {
    match e {
        LaneChoiceError::Lane(lane) => DrivingError::Lane(lane),
        LaneChoiceError::Random(e) => DrivingError::GlobalRandom(e),
    }
}
