//! Vehicle::UpdateTargetRAINInfo82d25d98. Advances at most one RAIN item per
//! call, retaining source queue changes and its asymmetric backwards lookup.
use super::{
    VehicleMotion,
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
};
use crate::{
    living_world_mesh_store::MeshStore, living_world_rain::RainMan,
    reckoning_numeric::ReckoningNumeric, simulation_drive_jacobian::refine_drive_reciprocal,
};

/// Writes belong to the entity's existing core and census owner. A route does
/// not own a second cull queue or a second explicit-update flag.
pub(crate) trait VehicleRouteControl {
    type Error;
    /// LWEntityCoreData+f0 |= 0x40.
    fn request_explicit_update(&mut self);
    /// Vehicle::RequestExplicitCull82d25920: core+f0 |=0x80, then RequestCull.
    fn request_explicit_cull(&mut self) -> Result<(), Self::Error>;
}

impl VehicleRoute {
    pub(crate) fn update_target_rain_info<V, N, C>(
        &mut self,
        motion: &VehicleMotion,
        id: u32,
        length: f32,
        dt: f32,
        colliding: bool,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        control: &mut C,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>>
    where
        V: DrivingVehicles,
        N: ReckoningNumeric,
        C: VehicleRouteControl<Error = V::Error>,
    {
        if colliding {
            return Ok(());
        }
        let mut position = motion.curve_position_update(self.item_position, dt);
        let item_length;
        if self.item_type == 1 {
            let Some(road) = rain.roads.get(&self.item_id) else {
                return control
                    .request_explicit_cull()
                    .map_err(DrivingError::Vehicle);
            };
            item_length = road.record.scalar(0x8c)?;
            if self.lane_change_passage.valid {
                let aggregate = road.record.scalar(0x90)?;
                let road_start = aggregate - item_length;
                let lane_position = motion.curve_position_update(self.lane_change_position, dt);
                let passage = &self.lane_change_passage;
                let curve_length = passage
                    .curve
                    .length
                    .ok_or(DrivingError::MissingLaneChangeLength)?;
                let inverse = refine_drive_reciprocal(
                    curve_length,
                    numeric
                        .reciprocal_estimate(curve_length)
                        .map_err(DrivingError::Numeric)?,
                );
                let super_position = (passage.parameter_end_position
                    - passage.parameter_start_position)
                    .mul_add(inverse * lane_position, passage.parameter_start_position);
                if super_position > aggregate {
                    let super_id = road.record.id(0x98)?;
                    let super_road = rain
                        .super_roads
                        .get(&super_id)
                        .ok_or(DrivingError::SuperRoad(super_id))?;
                    let next = super_road
                        .find_road(super_position, rain)
                        .map_err(DrivingError::Geometry)?;
                    // Native VMX ordered greater-than assertion1046.
                    if !(super_road.data.length > super_position) {
                        return Err(DrivingError::LaneChangeBeyondSuperRoad);
                    }
                    let Some(next) = next else {
                        return control
                            .request_explicit_cull()
                            .map_err(DrivingError::Vehicle);
                    };
                    self.item_type = 1;
                    self.item_id = next.record.id(0)?;
                    self.item_position =
                        super_position - (next.record.scalar(0x90)? - next.record.scalar(0x8c)?);
                } else {
                    self.item_position = super_position - road_start;
                }
                self.lane_change_position = lane_position;
                if lane_position >= curve_length {
                    self.road_lane = self.lane_change_passage.destination_lane;
                    control.request_explicit_update();
                    self.lane_change_passage.valid = false;
                }
                return self.finish_target_rain_info(motion, id, length, rain, control);
            }
        } else if self.item_type == 0 && self.passage_index >= 0 {
            let Some(intersection) = rain
                .intersection(self.item_id, mesh)
                .map_err(DrivingError::Rain)?
            else {
                return control
                    .request_explicit_cull()
                    .map_err(DrivingError::Vehicle);
            };
            item_length = intersection
                .passages
                .get(self.passage_index as usize)
                .ok_or(DrivingError::Passage(self.passage_index))?
                .curve
                .length;
        } else {
            return control
                .request_explicit_cull()
                .map_err(DrivingError::Vehicle);
        }

        if position > item_length {
            position -= item_length;
            let (next_type, next_id);
            if self.item_type == 1 {
                let road = rain
                    .roads
                    .get(&self.item_id)
                    .ok_or(DrivingError::Road(self.item_id))?;
                next_type = road.destination.item_type;
                next_id = road.destination.item_id;
                let super_id = road.record.id(0x98)?;
                if next_type == 0 {
                    let Some(intersection) = rain
                        .intersection(next_id, mesh)
                        .map_err(DrivingError::Rain)?
                    else {
                        return control
                            .request_explicit_cull()
                            .map_err(DrivingError::Vehicle);
                    };
                    if self.passage_index < 0 {
                        return control
                            .request_explicit_cull()
                            .map_err(DrivingError::Vehicle);
                    }
                    let lane = intersection
                        .passages
                        .get(self.passage_index as usize)
                        .ok_or(DrivingError::Passage(self.passage_index))?
                        .record
                        .word(0x58)?;
                    if !self.registered_in_intersection {
                        return Err(DrivingError::Unregistered(id));
                    }
                    rain.super_roads
                        .get_mut(&super_id)
                        .ok_or(DrivingError::SuperRoad(super_id))?
                        .unregister_vehicle(id, lane, length + motion.minimum_vehicle_distance)
                        .map_err(DrivingError::Lanes)?;
                    self.super_road_position = -1.;
                } else if next_type == 1 && !rain.roads.contains_key(&next_id) {
                    return control
                        .request_explicit_cull()
                        .map_err(DrivingError::Vehicle);
                }
            } else {
                let intersection = rain
                    .intersections
                    .get(&self.item_id)
                    .ok_or(DrivingError::Intersection(self.item_id))?;
                let passage = intersection
                    .passages
                    .get(self.passage_index as usize)
                    .ok_or(DrivingError::Passage(self.passage_index))?;
                let pin_index = passage.record.word(0x5c)?;
                let pin = intersection
                    .outgoing
                    .get(pin_index as usize)
                    .ok_or(DrivingError::Pin(pin_index))?;
                next_type = pin.item_type;
                next_id = pin.item_id;
                if next_type == 1 {
                    let Some(road) = rain.roads.get(&next_id) else {
                        return control
                            .request_explicit_cull()
                            .map_err(DrivingError::Vehicle);
                    };
                    let super_id = road.record.id(0x98)?;
                    if !self.registered_in_intersection {
                        return Err(DrivingError::Unregistered(id));
                    }
                    self.super_road_position =
                        (road.record.scalar(0x90)? - road.record.scalar(0x8c)?) + position;
                    self.road_lane = passage.record.word(0x60)?;
                    rain.super_roads
                        .get_mut(&super_id)
                        .ok_or(DrivingError::SuperRoad(super_id))?
                        .register_vehicle(
                            id,
                            self.road_lane,
                            self.super_road_position,
                            length + motion.minimum_vehicle_distance,
                            vehicles,
                        )
                        .map_err(DrivingError::Lanes)?;
                }
            }
            if next_type == 2 {
                return control
                    .request_explicit_cull()
                    .map_err(DrivingError::Vehicle);
            }
            self.item_type = next_type;
            self.item_position = position;
            self.item_id = next_id;
            control.request_explicit_update();
        } else if 0. > position {
            if self.item_type != 1 {
                return control
                    .request_explicit_cull()
                    .map_err(DrivingError::Vehicle);
            }
            let road = rain
                .roads
                .get(&self.item_id)
                .ok_or(DrivingError::Road(self.item_id))?;
            let next_type = road.origin.item_type;
            let next_id = road.origin.item_id;
            if next_type == 1 {
                // Missing predecessor deliberately still publishes its ID/pos.
                if let Some(previous) = rain.roads.get(&next_id) {
                    position += previous.record.scalar(0x8c)?;
                }
            } else if next_type == 2 {
                return control
                    .request_explicit_cull()
                    .map_err(DrivingError::Vehicle);
            }
            self.item_type = next_type;
            self.item_position = position;
            self.item_id = next_id;
            control.request_explicit_update();
        } else {
            self.item_position = position;
        }
        self.finish_target_rain_info(motion, id, length, rain, control)
    }

    fn finish_target_rain_info<C: VehicleRouteControl, N>(
        &mut self,
        motion: &VehicleMotion,
        id: u32,
        length: f32,
        rain: &RainMan,
        control: &mut C,
    ) -> Result<(), DrivingError<C::Error, N>> {
        if self.item_type == 1 {
            if let Some(road) = rain.roads.get(&self.item_id) {
                self.super_road_position =
                    (road.record.scalar(0x90)? - road.record.scalar(0x8c)?) + self.item_position;
                if !self.registered_in_intersection {
                    let super_id = road.record.id(0x98)?;
                    let super_road = rain
                        .super_roads
                        .get(&super_id)
                        .ok_or(DrivingError::SuperRoad(super_id))?;
                    let lane = super_road
                        .lane_vehicles
                        .get(self.road_lane as usize)
                        .ok_or(DrivingError::Lane(self.road_lane))?;
                    if lane.last().copied().unwrap_or(0) == id
                        && (length * 0.5 + self.super_road_position) + motion.speed
                            >= super_road.data.length - motion.minimum_intersection_distance
                    {
                        control.request_explicit_update();
                    }
                }
            }
        }
        Ok(())
    }
}
