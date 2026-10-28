//! Driving callbacks, beginning with ContinueFollowingLane82d1f310. Retains
//! native per-vehicle route fields and borrows the sole RAIN/vehicle registries.
use super::{
    StopReason, VehicleMotion,
    obstacles::{FrontEntityGeometry, ObstacleError, StopDistances},
};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::vehicle_graph::conditions::VehicleAction,
    living_world_rain::{
        RainMan,
        asset::RainAssetError,
        intersection_vehicles::{
            IntersectionError, IntersectionVehicles, PassageApproval, PassageState,
        },
        network::SuperRoad,
        vehicle_lanes::LaneError,
    },
    native_skater_random::{NativeRandomGate, NativeRandomLocked, NativeSkaterRandom},
    reckoning_numeric::ReckoningNumeric,
};
pub(crate) trait DrivingVehicles: IntersectionVehicles {
    fn stopping_reason(&mut self, id: u32) -> Result<StopReason, Self::Error>;
    fn lane_change_passage_valid(&mut self, id: u32) -> Result<bool, Self::Error>;
    fn backup_position(&mut self, id: u32) -> Result<f32, Self::Error>;
}
/// Selected route/decision fields from Vehicle ctor82d1cc64..cd80. Dynamic
/// passage geometry remains a separate original member, not fabricated here.
pub(crate) struct VehicleRoute {
    pub(crate) lane_change_passage: crate::living_world_rain::dynamic_passage::DynPassage,
    pub(crate) item_type: u32,
    pub(crate) item_id: u64,
    pub(crate) super_road_id: u64,
    pub(crate) intersection_id: u64,
    pub(crate) item_position: f32,
    pub(crate) super_road_position: f32,
    pub(crate) pull_over_position: f32,
    pub(crate) lane_change_position: f32,
    pub(crate) intersection_position: f32,
    pub(crate) backup_position: f32,
    pub(crate) road_lane: u32,
    pub(crate) change_to_lane: u32,
    pub(crate) passage_index: i32,
    pub(crate) action: VehicleAction,
    pub(crate) lights: u8,
    pub(crate) avoid_pileup: bool,
    pub(crate) registered_in_intersection: bool,
    pub(crate) registered_in_parking: bool,
    pub(crate) registered_to_road: bool,
    pub(crate) action_decision_mileage: f32,
    pub(crate) horizontal_road_position: f32,
    pub(crate) offset_from_lane: f32,
    pub(crate) force_new_behaviours: bool,
    pub(crate) avoid_skaters: bool,
    pub(crate) avoid_npcs: bool,
    pub(crate) honk: bool,
    pub(crate) park_when_skitched: bool,
}
impl VehicleRoute {
    pub(crate) fn new() -> Self {
        Self {
            lane_change_passage: crate::living_world_rain::dynamic_passage::DynPassage::new(),
            item_type: 2,
            item_id: 0,
            super_road_id: 0,
            intersection_id: 0,
            item_position: -1.,
            super_road_position: -1.,
            pull_over_position: -1.,
            lane_change_position: -1.,
            intersection_position: -1.,
            backup_position: -1.,
            road_lane: 0,
            change_to_lane: 0,
            passage_index: -1,
            action: VehicleAction::None,
            lights: 0,
            avoid_pileup: true,
            registered_in_intersection: false,
            registered_in_parking: false,
            registered_to_road: false,
            action_decision_mileage: 0.,
            horizontal_road_position: -1.,
            offset_from_lane: 0.,
            force_new_behaviours: true,
            avoid_skaters: true,
            avoid_npcs: true,
            honk: false,
            park_when_skitched: false,
        }
    }
    pub(super) fn super_road<'a, E, N>(
        &self,
        rain: &'a RainMan,
    ) -> Result<&'a SuperRoad, DrivingError<E, N>> {
        let road = rain
            .roads
            .get(&self.item_id)
            .ok_or(DrivingError::Road(self.item_id))?;
        let id = road.record.id(0x98)?;
        rain.super_roads
            .get(&id)
            .map(|r| &**r)
            .ok_or(DrivingError::SuperRoad(id))
    }
}
#[derive(Debug)]
pub(crate) enum DrivingError<E, N> {
    Road(u64),
    SuperRoad(u64),
    Intersection(u64),
    Pin(u32),
    Passage(i32),
    Lane(u32),
    MissingSkitchDelta,
    MissingCollisionModel,
    Core(crate::living_world_grind_services::entity_core::CoreError<std::convert::Infallible>),
    Vision(super::vision::VisionError<AttributeRuntimeError, N>),
    MissingLaneChangeLength,
    Cvar(crate::input_cvar_runtime::InputCvarError),
    Rain(crate::living_world_rain::network::RainNetworkError),
    Wheel(super::wheels::WheelError<N>),
    PassageCapacity,
    EmptyPassage,
    LaneChangeBeyondSuperRoad,
    Decision(crate::living_world_grind_services::vehicle_graph::conditions::DecisionError<E>),
    Unregistered(u32),
    Asset(RainAssetError),
    Attribute(AttributeRuntimeError),
    Numeric(N),
    Geometry(crate::living_world_rain::geometry::GeometryError<N>),
    Curve(crate::living_world_rain::curve::CurveError<N>),
    Random(NativeRandomLocked),
    GlobalRandom(crate::native_skater_random::rw_math::RwRandomLockError),
    Vehicle(E),
    Approval(IntersectionError<E>),
    Lanes(LaneError<E>),
    Obstacle(ObstacleError<AttributeRuntimeError, N>),
}
impl<E, N> From<RainAssetError> for DrivingError<E, N> {
    fn from(e: RainAssetError) -> Self {
        Self::Asset(e)
    }
}
impl<E, N> From<AttributeRuntimeError> for DrivingError<E, N> {
    fn from(e: AttributeRuntimeError) -> Self {
        Self::Attribute(e)
    }
}

impl VehicleRoute {
    pub(crate) fn continue_following_lane<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
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
        cvars: &InputCvarReferences,
        random: &mut NativeSkaterRandom,
        random_gate: &NativeRandomGate,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use DrivingError::Vehicle;
        let road_data = self.super_road::<V::Error, N::Error>(rain)?.data;
        let mut stop = StopDistances::new();
        if self.passage_index < 0 && self.intersection_id != road_data.origin {
            let mut selected = -1;
            rain.intersection(road_data.destination, mesh)
                .map_err(DrivingError::Rain)?;
            if let Some(intersection) = rain.intersections.get(&road_data.destination) {
                let pin = intersection
                    .incoming
                    .get(road_data.destination_pin as usize)
                    .ok_or(DrivingError::Pin(road_data.destination_pin))?;
                let passages = pin.available_passages(self.road_lane as usize)?;
                if self.avoid_pileup {
                    let mut best = f32::MAX;
                    for index in passages {
                        let passage = intersection
                            .passages
                            .get(index as usize)
                            .ok_or(DrivingError::Passage(index))?;
                        let destination = passage.record.word(0x5c)?;
                        let outgoing = intersection
                            .outgoing
                            .get(destination as usize)
                            .ok_or(DrivingError::Pin(destination))?;
                        if let Some(next_road) = rain.roads.get(&outgoing.item_id) {
                            let next_id = next_road.record.id(0x98)?;
                            let next = rain
                                .super_roads
                                .get(&next_id)
                                .ok_or(DrivingError::SuperRoad(next_id))?;
                            let lane = passage.record.word(0x60)?;
                            // Native scalar fdiv(1,length), then vector multiply.
                            let reciprocal = 1. / next.data.length;
                            let occupancy = reciprocal
                                * *next
                                    .lane_total_lengths
                                    .get(lane as usize)
                                    .ok_or(DrivingError::Lane(lane))?;
                            if best > occupancy {
                                best = occupancy;
                                selected = index;
                            }
                        }
                    }
                } else if !passages.is_empty() {
                    let fraction = random
                        .try_living_world_range(random_gate, 0., 1.)
                        .map_err(DrivingError::Random)?;
                    let draw = numeric
                        .truncate_i32(fraction * 100.)
                        .map_err(DrivingError::Numeric)? as u32;
                    selected = passages[draw as usize % passages.len()];
                }
            }
            self.passage_index = selected;
        }
        motion.front_vehicle_distance = f32::MAX;
        motion.stopping_reason = StopReason::None;
        motion.travel_intention = true;
        if self.passage_index >= 0 && self.intersection_id != road_data.origin {
            let distance = (road_data.length - self.super_road_position) - length * 0.5;
            rain.intersection(road_data.destination, mesh)
                .map_err(DrivingError::Rain)?;
            if let Some(intersection) = rain.intersections.get(&road_data.destination) {
                let passage = intersection
                    .passages
                    .get(self.passage_index as usize)
                    .ok_or(DrivingError::Passage(self.passage_index))?;
                if motion.minimum_intersection_distance + motion.speed >= distance {
                    let mut state = PassageState::Empty;
                    let check_light = !motion.will_run_red_light(driver)?;
                    let mut query = super::query::CurrentVehicle {
                        id,
                        length,
                        position: self.super_road_position,
                        motion,
                        others: vehicles,
                    };
                    let approval = intersection
                        .passage_approved(
                            id,
                            self.passage_index,
                            &mut state,
                            check_light,
                            rain,
                            &mut query,
                            cvars,
                        )
                        .map_err(DrivingError::Approval)?;
                    if approval != PassageApproval::Approved {
                        motion.travel_intention = motion.travel_intention
                            && distance > motion.minimum_intersection_distance;
                        if stop.destination > distance {
                            stop.destination = distance;
                            stop.minimum = motion.minimum_intersection_distance;
                            motion.stopping_reason = if approval == PassageApproval::Occupied
                                && state == PassageState::OccupiedRuleBroken
                            {
                                StopReason::RuleBreakingVehicle
                            } else {
                                StopReason::Intersection
                            };
                        }
                    }
                }
                let from = passage.record.word(0x54)?;
                let lane = passage.record.word(0x58)?;
                let incoming = intersection
                    .incoming
                    .get(from as usize)
                    .ok_or(DrivingError::Pin(from))?;
                for index in incoming.available_passages(lane as usize)? {
                    let passage = intersection
                        .passages
                        .get(index as usize)
                        .ok_or(DrivingError::Passage(index))?;
                    for &other in &passage.vehicles {
                        if other == id {
                            continue;
                        }
                        vehicles.require_vehicle(other).map_err(Vehicle)?;
                        if vehicles.rain_item_type(other).map_err(Vehicle)? != 0 {
                            continue;
                        }
                        let delta = (vehicles.current_position(other).map_err(Vehicle)?
                            + road_data.length)
                            - self.super_road_position;
                        let gap = delta - (vehicles.length(other).map_err(Vehicle)? + length) * 0.5;
                        let reason = vehicles.stopping_reason(other).map_err(Vehicle)?;
                        motion.update_front_vehicle(
                            gap,
                            motion.minimum_vehicle_distance,
                            reason,
                            &mut stop,
                            dt,
                            driver,
                        )?;
                        break; // Source advances to next passage after first car in intersection.
                    }
                }
                let signal_distance = motion.minimum_intersection_distance * 10.;
                if motion.speed + signal_distance >= distance {
                    let to = passage.record.word(0x5c)?;
                    if to == from.wrapping_add(1) & 3 {
                        self.lights |= 8;
                    } else if to == from.wrapping_sub(1) & 3 {
                        self.lights |= 16;
                    }
                }
            }
        }
        // Scalar blt skip accepts unordered; ordered following predicates reject it.
        if !(self.pull_over_position < 0.) {
            let distance =
                (self.pull_over_position - self.super_road_position) + f32::from_bits(0x3dcc_cccd);
            if distance > 0. && motion.minimum_parking_distance + motion.speed >= distance {
                motion.travel_intention =
                    motion.travel_intention && distance > motion.minimum_parking_distance;
                if stop.destination > distance {
                    stop.destination = distance;
                    stop.minimum = motion.minimum_parking_distance;
                    motion.stopping_reason = StopReason::Parking;
                }
            }
        }
        let road = rain
            .super_roads
            .get(&road_data.id)
            .ok_or(DrivingError::SuperRoad(road_data.id))?;
        let other = road
            .neighbor(self.road_lane, id, true)
            .map_err(DrivingError::Lanes)?;
        if other != 0 {
            vehicles.require_vehicle(other).map_err(Vehicle)?;
            let delta = vehicles.position(other).map_err(Vehicle)? - self.super_road_position;
            let gap = delta - (vehicles.length(other).map_err(Vehicle)? + length) * 0.5;
            let reason = vehicles.stopping_reason(other).map_err(Vehicle)?;
            motion.update_front_vehicle(
                gap,
                motion.minimum_vehicle_distance,
                reason,
                &mut stop,
                dt,
                driver,
            )?;
        } else {
            motion.vehicle_waiting_timer = 0.;
        }
        motion
            .update_front_entity(vision, &mut stop, dt, driver, numeric)
            .map_err(DrivingError::Obstacle)?;
        motion.update_acceleration(dt, stop.destination, stop.minimum, characteristics)?;
        if !motion.update_speed(dt, road_data.speed_limit) {
            return Err(DrivingError::MissingSkitchDelta);
        }
        Ok(())
    }
}
