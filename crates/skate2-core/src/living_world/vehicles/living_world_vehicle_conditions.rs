//! The eleven Vehicle.xml predicates, reading the real RAIN owner and authored
//! vehicle attributes. A borrowed view; not another entity or road registry.
use super::VehicleCondition;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    living_world_grind_services::vehicle_motion::VehicleMotion,
    living_world_rain::{
        RainMan,
        asset::RainAssetError,
        geometry::GeometryError,
        network::SuperRoad,
        vehicle_lanes::{LaneError, RainVehicles},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum VehicleAction {
    None = 0,
    ChangeLane = 1,
    Impatience = 2,
    Park = 3,
}
#[derive(Debug)]
pub(crate) enum DecisionError<E> {
    MissingRoad(u64),
    MissingSuperRoad(u64),
    Lane(u32),
    MissingLaneChangeLength,
    Asset(RainAssetError),
    Attribute(AttributeRuntimeError),
    Geometry(GeometryError<E>),
    Lanes(LaneError<E>),
}
impl<E> From<RainAssetError> for DecisionError<E> {
    fn from(e: RainAssetError) -> Self {
        Self::Asset(e)
    }
}
impl<E> From<AttributeRuntimeError> for DecisionError<E> {
    fn from(e: AttributeRuntimeError) -> Self {
        Self::Attribute(e)
    }
}
pub(crate) struct VehicleDecisionView<'a> {
    pub(crate) entity_id: u32,
    pub(crate) rain_item_type: u32,
    pub(crate) rain_item_id: u64,
    pub(crate) intersection_id: u64,
    pub(crate) road_lane: u32,
    pub(crate) change_to_lane: u32,
    pub(crate) super_road_position: f32,
    pub(crate) pull_over_position: f32,
    pub(crate) backup_position: f32,
    pub(crate) lane_change_position: f32,
    pub(crate) lane_change_length: Option<f32>,
    pub(crate) action: VehicleAction,
    pub(crate) registered_in_intersection: bool,
    pub(crate) length: f32,
    pub(crate) motion: &'a VehicleMotion,
    pub(crate) characteristics: &'a AttributeRuntimeInstance,
    pub(crate) driver: &'a AttributeRuntimeInstance,
}
impl VehicleDecisionView<'_> {
    fn super_road<'a, E>(&self, rain: &'a RainMan) -> Result<&'a SuperRoad, DecisionError<E>> {
        let road = rain
            .roads
            .get(&self.rain_item_id)
            .ok_or(DecisionError::MissingRoad(self.rain_item_id))?;
        let id = road.record.id(0x98)?;
        rain.super_roads
            .get(&id)
            .map(|r| &**r)
            .ok_or(DecisionError::MissingSuperRoad(id))
    }
    pub(crate) fn evaluate<V: RainVehicles>(
        &self,
        condition: VehicleCondition,
        rain: &RainMan,
        vehicles: &mut V,
    ) -> Result<bool, DecisionError<V::Error>> {
        use VehicleCondition as C;
        let motion = self.motion;
        let pos = self.super_road_position;
        Ok(match condition {
            C::RequiredToPassIntersection => self.rain_item_type == 0, //82d163e0
            C::IntersectionPassageComplete => self.rain_item_type == 1, //82d16cc0
            C::ImpatienceComplete => !(pos > self.backup_position),    //ble82d16c54
            C::LaneChangePassageComplete => {
                self.lane_change_position
                    >= self
                        .lane_change_length
                        .ok_or(DecisionError::MissingLaneChangeLength)?
            } //82d16c78
            C::RequiredToStopAlarming => {
                motion.alarm_timer > scalar(self.characteristics, 0xe199_fc7c_ea22_2809)?
            } //82d16ba8
            C::RequiredToRegisterToIntersection => {
                //82d1c5e8: lookup comes BEFORE registration flag test.
                let road = self.super_road(rain)?;
                if self.registered_in_intersection {
                    return Ok(false);
                }
                let lane = road
                    .lane_vehicles
                    .get(self.road_lane as usize)
                    .ok_or(DecisionError::Lane(self.road_lane))?;
                if lane.last().copied().unwrap_or(0) != self.entity_id {
                    return Ok(false);
                }
                let threshold = road.data.length - motion.minimum_intersection_distance;
                (self.length * 0.5 + pos) + motion.speed >= threshold
            }
            C::RequiredToUnregisterFromIntersection => {
                //82d16d00 uses super-road ORIGIN intersection; all64 ID bits.
                let road = self.super_road(rain)?;
                if !self.registered_in_intersection
                    || self.intersection_id != road.data.origin
                    || pos < motion.minimum_intersection_distance
                {
                    return Ok(false);
                }
                motion.minimum_intersection_distance + pos < road.data.length
            }
            C::RequiredToBeImpatient => {
                //82d16420. Length*factor and position add are separately rounded.
                let road = self.super_road(rain)?;
                if self.action != VehicleAction::Impatience
                    || motion.speed > f32::from_bits(0x3dcc_cccd)
                    || !(pos > self.length)
                {
                    return Ok(false);
                }
                road.data.length - self.length
                    > (self.length * motion.minimum_lane_change_distance_factor + pos)
                        + motion.speed
            }
            C::RequiredToChangeLane => {
                //82d16668: unlike the preceding predicate, length*factor+speed
                // is a fused VMX operation. Preserve the live front-gap test.
                let road = self.super_road(rain)?;
                let travel = self
                    .length
                    .mul_add(motion.minimum_lane_change_distance_factor, motion.speed);
                let half = travel * 0.5;
                let destination = pos + travel;
                if self.action != VehicleAction::ChangeLane
                    || !(motion.speed > half)
                    || !(motion.front_vehicle_distance > half)
                    || !(pos > self.length)
                    || !(road.data.length - self.length > destination)
                {
                    return Ok(false);
                }
                if !road
                    .can_place_vehicle(
                        pos,
                        self.change_to_lane,
                        self.length,
                        motion.speed,
                        true,
                        vehicles,
                    )
                    .map_err(DecisionError::Lanes)?
                {
                    return Ok(false);
                }
                road.find_road::<V::Error>(destination, rain)
                    .map_err(DecisionError::Geometry)?
                    .is_some()
            }
            C::RequiredToPullOver => {
                //82d16958: authored minimum PARKING distance, not lane-change.
                let road = self.super_road(rain)?;
                if self.action != VehicleAction::Park
                    || self.pull_over_position < 0.
                    || !(self.pull_over_position > pos)
                    || self.pull_over_position - pos > motion.minimum_parking_distance
                    || !(motion.speed < 1.)
                {
                    return Ok(false);
                }
                road.find_road::<V::Error>(self.pull_over_position, rain)
                    .map_err(DecisionError::Geometry)?
                    .is_some()
            }
            C::RequiredToPullOut => {
                let road = self.super_road(rain)?; //82d16a90
                if motion.alarming {
                    return Ok(false);
                }
                if !(motion.parking_timer > scalar(self.driver, 0x988b_b0f6_f043_eb3d)?) {
                    return Ok(false);
                }
                road.find_road::<V::Error>(motion.minimum_parking_distance + pos, rain)
                    .map_err(DecisionError::Geometry)?
                    .is_some()
            }
        })
    }
}
fn scalar(instance: &AttributeRuntimeInstance, key: u64) -> Result<f32, AttributeRuntimeError> {
    Ok(f32::from_bits(
        instance.try_scalar_word(key, 0)?.unwrap_or(0),
    ))
}
