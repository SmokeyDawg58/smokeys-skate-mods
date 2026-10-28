//! Intersection right of way:82e59270,82e595c0,82e59918. Uses the SAME authored
//! pins/passages, streamed roads, traffic lights and vehicle provider.
use super::{
    RainMan,
    asset::RainAssetError,
    network::Intersection,
    traffic::{Colour, TrafficError},
    vehicle_lanes::RainVehicles,
};
use crate::input_cvar_runtime::{InputCvarError, InputCvarReferences};

pub(crate) trait IntersectionVehicles: RainVehicles {
    fn rain_item_type(&mut self, id: u32) -> Result<u32, Self::Error>;
    fn super_road_id(&mut self, id: u32) -> Result<u64, Self::Error>;
    fn current_position(&mut self, id: u32) -> Result<f32, Self::Error>;
    fn breaking_rules(&mut self, id: u32) -> Result<bool, Self::Error>;
    fn minimum_intersection_distance(&mut self, id: u32) -> Result<f32, Self::Error>;
    fn minimum_vehicle_distance(&mut self, id: u32) -> Result<f32, Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum PassageState {
    Empty = 0,
    OccupiedStandard = 1,
    OccupiedRuleBroken = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum PassageApproval {
    Approved = 0,
    LightState = 1,
    Speed = 2,
    Occupied = 3,
    Capacity = 4,
}
#[derive(Debug)]
pub(crate) enum IntersectionError<E> {
    Vehicle(E),
    Asset(RainAssetError),
    Traffic(TrafficError),
    Cvar(InputCvarError),
    Pin(u32),
    Lane(u32),
    Passage(i32),
    MissingSuperRoad(u64),
}
impl<E> From<RainAssetError> for IntersectionError<E> {
    fn from(e: RainAssetError) -> Self {
        Self::Asset(e)
    }
}
impl Intersection {
    /// Shared loop of CheckIfPassage and CheckIfPassageToLane. `to_lane=None`
    /// selects the first function; Some retains the second's destination bound.
    pub(crate) fn check_passage<V: IntersectionVehicles>(
        &self,
        vehicle: u32,
        from: u32,
        to: u32,
        to_lane: Option<u32>,
        state: &mut PassageState,
        proximity: bool,
        threshold: f32,
        rain: &RainMan,
        vehicles: &mut V,
    ) -> Result<bool, IntersectionError<V::Error>> {
        use IntersectionError::Vehicle;
        let incoming = self
            .incoming
            .get(from as usize)
            .ok_or(IntersectionError::Pin(from))?;
        let outgoing = self
            .outgoing
            .get(to as usize)
            .ok_or(IntersectionError::Pin(to))?;
        let mut occupied = false;
        let mut result = PassageState::Empty;
        if self.incoming_valid[from as usize]
            && self.outgoing_valid[to as usize]
            && to_lane.is_none_or(|lane| lane < outgoing.lanes)
        {
            for lane in 0..incoming.lanes {
                for index in incoming.available_passages(lane as usize)? {
                    let passage = self
                        .passages
                        .get(index as usize)
                        .ok_or(IntersectionError::Passage(index))?;
                    if passage.record.word(0x5c)? != to {
                        continue;
                    }
                    if let Some(lane) = to_lane {
                        if passage.record.word(0x60)? != lane {
                            continue;
                        }
                    }
                    for &other in &passage.vehicles {
                        vehicles.require_vehicle(other).map_err(Vehicle)?;
                        if result == PassageState::OccupiedRuleBroken {
                            continue;
                        }
                        let mut qualifies = vehicles.rain_item_type(other).map_err(Vehicle)? == 0;
                        if !qualifies && proximity {
                            let super_id = vehicles.super_road_id(other).map_err(Vehicle)?;
                            if let Some(road) = rain.super_roads.get(&super_id) {
                                // Raw ld/cmpld82e594a0: full64-bit destination ID.
                                if road.data.destination == self.record.id(0x248)? {
                                    let remaining = road.data.length
                                        - vehicles.position(other).map_err(Vehicle)?;
                                    let length = vehicles.length(other).map_err(Vehicle)?;
                                    let distance = -(length.mul_add(0.5, -remaining)); //fnmsubs
                                    qualifies = distance < threshold
                                        || (distance == threshold && vehicle > other);
                                }
                            }
                        }
                        if qualifies {
                            occupied = true;
                            result = PassageState::OccupiedStandard;
                        }
                        // Native uses accumulated occupied, not just this car's
                        // qualification. A later rule-breaker can promote state.
                        if occupied && vehicles.breaking_rules(other).map_err(Vehicle)? {
                            result = PassageState::OccupiedRuleBroken;
                        }
                    }
                }
            }
        }
        *state = result;
        Ok(occupied)
    }

    pub(crate) fn passage_approved<V: IntersectionVehicles>(
        &self,
        vehicle: u32,
        passage_index: i32,
        state: &mut PassageState,
        check_light: bool,
        rain: &RainMan,
        vehicles: &mut V,
        cvars: &InputCvarReferences,
    ) -> Result<PassageApproval, IntersectionError<V::Error>> {
        use IntersectionError::Vehicle;
        use PassageApproval as A;
        vehicles.require_vehicle(vehicle).map_err(Vehicle)?;
        let passage = self
            .passages
            .get(passage_index as usize)
            .ok_or(IntersectionError::Passage(passage_index))?;
        let from = passage.record.word(0x54)?;
        let lane = passage.record.word(0x58)?;
        let to = passage.record.word(0x5c)?;
        let dest_lane = passage.record.word(0x60)?;
        let incoming = self
            .incoming
            .get(from as usize)
            .ok_or(IntersectionError::Pin(from))?;
        let outgoing = self
            .outgoing
            .get(to as usize)
            .ok_or(IntersectionError::Pin(to))?;
        let origin_road = rain.roads.get(&incoming.item_id);
        let dest_road = rain.roads.get(&outgoing.item_id);
        let (Some(origin_road), Some(dest_road)) = (origin_road, dest_road) else {
            return Ok(A::Capacity);
        };
        let origin_super = origin_road.record.id(0x98)?;
        let dest_super = dest_road.record.id(0x98)?;
        let origin_super = rain
            .super_roads
            .get(&origin_super)
            .ok_or(IntersectionError::MissingSuperRoad(origin_super))?;
        let dest_super = rain
            .super_roads
            .get(&dest_super)
            .ok_or(IntersectionError::MissingSuperRoad(dest_super))?;
        let half_length = vehicles.length(vehicle).map_err(Vehicle)? * 0.5;
        let distance =
            origin_super.data.length - (vehicles.position(vehicle).map_err(Vehicle)? + half_length);
        let left = from.wrapping_add(1) & 3;
        let right = from.wrapping_sub(1) & 3;
        let opposite = from.wrapping_sub(2) & 3;
        if cvars
            .try_override_intersection_rules()
            .map_err(IntersectionError::Cvar)?
        {
            return Ok(A::Approved);
        }
        let mut max_speed = passage.record.scalar(0x50)?;
        let mut min_distance = vehicles
            .minimum_intersection_distance(vehicle)
            .map_err(Vehicle)?;
        let light_controlled = self.record.word(0x244)? != 0;
        if check_light {
            if light_controlled {
                let light = rain
                    .traffic_lights
                    .get(from as usize)
                    .ok_or(IntersectionError::Pin(from))?
                    .current(false)
                    .map_err(IntersectionError::Traffic)?;
                let reachable = vehicles.speed(vehicle).map_err(Vehicle)? * light.remaining;
                // Length is only read for GREEN (native short-circuit).
                let cannot_clear = if light.colour == Colour::Green {
                    let length = vehicles.length(vehicle).map_err(Vehicle)?;
                    reachable < -(length.mul_add(0.5, -distance))
                } else {
                    true
                };
                if cannot_clear && to != right {
                    return Ok(A::LightState);
                }
            } else {
                max_speed = f32::from_bits(0x3dcc_cccd);
                min_distance = vehicles
                    .minimum_intersection_distance(vehicle)
                    .map_err(Vehicle)?;
            }
        }
        if vehicles.speed(vehicle).map_err(Vehicle)? > max_speed && !(distance < min_distance) {
            return Ok(A::Speed);
        }
        let dest_entries = dest_super
            .lane_vehicles
            .get(dest_lane as usize)
            .ok_or(IntersectionError::Lane(dest_lane))?;
        if let Some(&other) = dest_entries.first() {
            vehicles.require_vehicle(other).map_err(Vehicle)?;
            let ahead = vehicles.speed(other).map_err(Vehicle)?
                + vehicles.position(other).map_err(Vehicle)?;
            let clearance = -(vehicles
                .length(other)
                .map_err(Vehicle)?
                .mul_add(0.5, -ahead));
            if vehicles.length(vehicle).map_err(Vehicle)? > clearance {
                return Ok(A::Capacity);
            }
        }
        let indices = incoming.available_passages(lane as usize)?;
        for &index in &indices {
            let other = self
                .passages
                .get(index as usize)
                .ok_or(IntersectionError::Passage(index))?;
            if index != passage_index && !other.vehicles.is_empty() {
                return Ok(A::Occupied);
            }
        }
        for index in indices {
            let other_passage = self
                .passages
                .get(index as usize)
                .ok_or(IntersectionError::Passage(index))?;
            if let Some(&other) = other_passage.vehicles.first() {
                if other == vehicle {
                    continue;
                }
                vehicles.require_vehicle(other).map_err(Vehicle)?;
                let pos = vehicles.current_position(other).map_err(Vehicle)?;
                if vehicles.rain_item_type(other).map_err(Vehicle)? == 0 {
                    let back = pos - vehicles.length(other).map_err(Vehicle)? * 0.5;
                    let gap = back + distance;
                    if vehicles
                        .minimum_vehicle_distance(vehicle)
                        .map_err(Vehicle)?
                        > gap
                    {
                        return Ok(A::Capacity);
                    }
                }
            }
        }
        // These calls deliberately overwrite `state` per attempted conflicting
        // movement and short-circuit on first true, matching original order.
        let proximity = !light_controlled;
        let mut conflict = |a, b, lane, near, threshold| {
            self.check_passage(vehicle, a, b, lane, state, near, threshold, rain, vehicles)
        };
        if to == left {
            if (dest_lane as i32) >= 0 {
                for target_lane in (0..=dest_lane).rev() {
                    if conflict(right, to, Some(target_lane), false, 0.)?
                        || conflict(opposite, to, Some(target_lane), false, 0.)?
                    {
                        return Ok(A::Occupied);
                    }
                }
            }
            if conflict(left, right, None, proximity, distance)?
                || conflict(left, opposite, None, proximity, distance)?
                || conflict(right, from, None, proximity, distance)?
                || conflict(opposite, from, None, true, distance)?
                || conflict(opposite, right, None, true, distance)?
            {
                return Ok(A::Occupied);
            }
        } else if to == right {
            for target_lane in dest_lane..dest_super.data.lanes {
                if conflict(left, to, Some(target_lane), false, 0.)?
                    || conflict(opposite, to, Some(target_lane), false, 0.)?
                {
                    return Ok(A::Occupied);
                }
            }
        } else {
            for target_lane in dest_lane..dest_super.data.lanes {
                if conflict(left, to, Some(target_lane), false, 0.)? {
                    return Ok(A::Occupied);
                }
            }
            if (dest_lane as i32) >= 0 {
                for target_lane in (0..=dest_lane).rev() {
                    if conflict(right, to, Some(target_lane), false, 0.)? {
                        return Ok(A::Occupied);
                    }
                }
            }
            if conflict(opposite, right, None, true, distance)?
                || conflict(left, right, None, proximity, distance)?
                || conflict(right, left, None, proximity, distance)?
                || conflict(right, from, None, proximity, distance)?
            {
                return Ok(A::Occupied);
            }
        }
        Ok(A::Approved)
    }
}
