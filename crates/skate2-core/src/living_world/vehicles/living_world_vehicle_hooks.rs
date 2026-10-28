//! All11 Vehicle.xml transition hooks. Native queue insert/remove order and
//! same-entity fields; no synthetic lane memberships or independent road owner.
use super::{
    VehicleMotion,
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
    wheels::fill_lane_curve,
};
use crate::{
    attribute_runtime::AttributeRuntimeInstance,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::vehicle_graph::{VehicleHook, conditions::VehicleAction},
    living_world_mesh_store::MeshStore,
    living_world_rain::{
        RainMan,
        intersection_vehicles::{PassageApproval, PassageState},
    },
    reckoning_numeric::ReckoningNumeric,
};

impl VehicleRoute {
    pub(crate) fn on_transition<V: DrivingVehicles, N: ReckoningNumeric>(
        &mut self,
        hook: VehicleHook,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        driver: &AttributeRuntimeInstance,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>> {
        use VehicleHook as H;
        match hook {
            H::ToImpatience => self.backup_position = self.super_road_position, //82d16fa0
            H::StopAlarming => motion.alarming = false,                         //82d17190
            H::ToPassingIntersection => {
                //82d170e0
                if self.registered_in_parking {
                    if let Some(road) = rain.super_roads.get_mut(&self.super_road_id) {
                        road.unregister_parking(id).map_err(DrivingError::Lanes)?;
                    }
                    self.registered_in_parking = false;
                }
                self.super_road_id = 0;
                self.horizontal_road_position = -1.;
            }
            H::ToStayingParked | H::FromChangingLaneToFollowingLane => {
                let super_id = self.super_road::<V::Error, N::Error>(rain)?.data.id;
                let road = rain
                    .super_roads
                    .get_mut(&super_id)
                    .ok_or(DrivingError::SuperRoad(super_id))?;
                road.unregister_vehicle(
                    id,
                    self.lane_change_passage.origin_lane,
                    length + motion.minimum_vehicle_distance,
                )
                .map_err(DrivingError::Lanes)?;
                self.road_lane = self.lane_change_passage.destination_lane;
                self.lane_change_passage.deinit();
                if hook == H::ToStayingParked {
                    //82d16fe0
                    self.passage_index = -1;
                    self.lights = 0;
                    self.lane_change_position = -1.;
                    self.pull_over_position = -1.;
                    motion.parking_timer = 0.;
                } else {
                    //82d1c870
                    self.lane_change_position = -1.;
                    self.pull_over_position = -1.;
                    self.action = VehicleAction::None;
                    self.action_decision_mileage = 0.;
                    self.passage_index = -1;
                    motion.parking_started = false;
                    self.lights &= 0xe7;
                }
            }
            H::FromPassingIntersectionToFollowingLane => {
                //82d1c990
                let road = self.super_road::<V::Error, N::Error>(rain)?;
                self.super_road_id = road.data.id;
                let lanes = road.data.lanes;
                self.pull_over_position = -1.;
                self.force_new_behaviours = true;
                self.action = VehicleAction::None;
                self.action_decision_mileage = 0.;
                self.lights &= 0xe7;
                self.horizontal_road_position =
                    (self.road_lane as f32).mul_add(2., 1.) / (lanes as f32 * 2.);
            }
            H::ToChangingLane | H::ToPullingOver | H::ToPullingOut => {
                let road = self.super_road::<V::Error, N::Error>(rain)?;
                let super_id = road.data.id;
                let passage = &mut self.lane_change_passage;
                passage.init();
                passage.parameter_start_position = self.super_road_position;
                passage.parameter_end_position = match hook {
                    //82d20930: multiplication then two adds, NOT a fused madd.
                    H::ToChangingLane => {
                        (length * motion.minimum_lane_change_distance_factor
                            + self.super_road_position)
                            + motion.speed
                    }
                    H::ToPullingOver => self.pull_over_position, //82d20ba8
                    H::ToPullingOut => motion.minimum_parking_distance + self.super_road_position, //82d20d20
                    _ => unreachable!(),
                };
                passage.origin_lane = self.road_lane;
                passage.destination_lane = match hook {
                    H::ToChangingLane => self.change_to_lane,
                    H::ToPullingOver => road.data.lanes,
                    H::ToPullingOut => self.road_lane.wrapping_sub(1),
                    _ => unreachable!(),
                };
                fill_lane_curve(passage, road, self.offset_from_lane, length, rain, numeric)
                    .map_err(DrivingError::Wheel)?;
                self.lane_change_position = 0.;
                let road = rain
                    .super_roads
                    .get_mut(&super_id)
                    .ok_or(DrivingError::SuperRoad(super_id))?;
                if hook == H::ToChangingLane {
                    self.backup_position = -1.;
                }
                if hook == H::ToPullingOut {
                    road.unregister_parking(id).map_err(DrivingError::Lanes)?;
                    self.registered_in_parking = false;
                }
                road.register_vehicle(
                    id,
                    passage.destination_lane,
                    self.super_road_position,
                    length + motion.minimum_vehicle_distance,
                    vehicles,
                )
                .map_err(DrivingError::Lanes)?;
                match hook {
                    H::ToChangingLane => {
                        let (off, on) = if passage.destination_lane < passage.origin_lane {
                            (16, 8)
                        } else {
                            (8, 16)
                        };
                        self.lights = (self.lights & !off) | on;
                    }
                    H::ToPullingOver => self.lights = self.lights & 0xe7 | 16,
                    H::ToPullingOut => self.lights = self.lights & 0xe7 | 8,
                    _ => unreachable!(),
                }
            }
            H::UnregisterFromIntersection => {
                //82d241f0
                let origin = self.super_road::<V::Error, N::Error>(rain)?.data.origin;
                if let Some(intersection) = rain
                    .intersection(origin, mesh)
                    .map_err(DrivingError::Rain)?
                {
                    if self.passage_index >= 0 {
                        let passage = intersection
                            .passages
                            .get_mut(self.passage_index as usize)
                            .ok_or(DrivingError::Passage(self.passage_index))?;
                        // Native pop_back (not erase by ID). Membership/order
                        // is supplied by real registration and departure flow.
                        passage.vehicles.pop().ok_or(DrivingError::EmptyPassage)?;
                    }
                }
                self.passage_index = -1;
                self.intersection_id = 0;
                self.registered_in_intersection = false;
                motion.breaking_rules = Some(false);
            }
            H::RegisterToIntersection => {
                //82d25738
                let destination = self
                    .super_road::<V::Error, N::Error>(rain)?
                    .data
                    .destination;
                let found = rain
                    .intersection(destination, mesh)
                    .map_err(DrivingError::Rain)?
                    .is_some();
                if found && self.passage_index >= 0 {
                    let intersection = rain
                        .intersections
                        .get_mut(&destination)
                        .ok_or(DrivingError::Intersection(destination))?;
                    let passage = intersection
                        .passages
                        .get_mut(self.passage_index as usize)
                        .ok_or(DrivingError::Passage(self.passage_index))?;
                    self.intersection_id = destination;
                    self.registered_in_intersection = true;
                    if passage.vehicles.len() >= 16 {
                        return Err(DrivingError::PassageCapacity);
                    }
                    passage.vehicles.insert(0, id);
                    if motion.will_run_red_light(driver)? {
                        let intersection = rain
                            .intersections
                            .get(&destination)
                            .ok_or(DrivingError::Intersection(destination))?;
                        // Source supplies null pState; scratch result unobserved.
                        let mut ignored = PassageState::Empty;
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
                                &mut ignored,
                                true,
                                rain,
                                &mut query,
                                cvars,
                            )
                            .map_err(DrivingError::Approval)?;
                        if approval == PassageApproval::LightState {
                            motion.breaking_rules = Some(true);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
