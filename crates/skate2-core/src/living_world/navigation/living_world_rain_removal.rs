//! RAINMan.RemoveRAINData82727510 and the vehicle-dependent teardown paths.
use super::{RainMan, network::RainNetworkError};
use crate::selected_simulation_asset_services::SelectedSimulationAssetServices;

pub(crate) trait RainRemovalServices {
    /// GetVehicle followed by IVehicle.GetCurrentRAINItemID (full64-bit ID).
    fn vehicle_road(&mut self, vehicle: u32) -> Result<u64, RainNetworkError>;
    /// IVehicle.RemoveFromRoadNetwork, then CullVehicle. Removal can edit the
    /// same RainMan's lane/passage lists, so no list borrow spans this call.
    fn remove_and_cull_vehicle(
        &mut self,
        rain: &mut RainMan,
        vehicle: u32,
    ) -> Result<(), RainNetworkError>;
    /// ComponentMan.InvalidateAllRAINData, in ped/vehicle/prop/DMO order.
    fn invalidate_entity_rain_data(&mut self) -> Result<(), RainNetworkError>;
}
impl RainMan {
    pub(crate) fn remove_data<S: RainRemovalServices>(
        &mut self,
        id: u64,
        assets: &SelectedSimulationAssetServices,
        incremental_kd_removal: bool,
        services: &mut S,
    ) -> Result<(), RainNetworkError> {
        if let Some(record) = self.chunks.get(id).cloned() {
            if incremental_kd_removal {
                self.depopulate_zones(id, assets)?;
            }
            for source in record.roads()? {
                let road_id = source.id(0)?;
                let Some(road) = self.roads.get(&road_id) else {
                    continue;
                };
                let super_id = road.record.id(0x98)?;
                let start = road.record.word(0x94)?;
                let super_road = self
                    .super_roads
                    .get(&super_id)
                    .ok_or(RainNetworkError::MissingSuperRoad(super_id))?;
                // RemoveRoad82e62208 uses lower_bound(start), not an ID search.
                let index = super_road
                    .roads
                    .partition_point(|(segment, _)| *segment < start);
                if index != super_road.roads.len() {
                    let candidates: Vec<_> = super_road
                        .lane_vehicles
                        .iter()
                        .flatten()
                        .chain(&super_road.parking_vehicles)
                        .copied()
                        .collect();
                    let mut selected = Vec::new();
                    for vehicle in candidates {
                        if services.vehicle_road(vehicle)? == road_id {
                            selected.push(vehicle);
                        }
                    }
                    for vehicle in selected {
                        services.remove_and_cull_vehicle(self, vehicle)?;
                    }
                    let super_road = self
                        .super_roads
                        .get_mut(&super_id)
                        .ok_or(RainNetworkError::MissingSuperRoad(super_id))?;
                    super_road.roads.remove(index);
                    super_road.roads.sort_unstable_by_key(|r| r.0);
                }
                let mut road = self
                    .roads
                    .remove(&road_id)
                    .ok_or(RainNetworkError::MissingItem(road_id))?;
                self.connect_road(&mut road, false)?;
                self.memory[1].release(road.slot)?;
            }
            for source in record.intersections()? {
                let intersection_id = source.id(0x248)?;
                let Some(intersection) = self.intersections.get(&intersection_id) else {
                    continue;
                };
                let vehicles: Vec<_> = intersection
                    .passages
                    .iter()
                    .flat_map(|p| p.vehicles.iter().copied())
                    .collect();
                // Disconnect before Intersection.Deinit's vehicle callbacks.
                let mut intersection = self
                    .intersections
                    .remove(&intersection_id)
                    .ok_or(RainNetworkError::MissingItem(intersection_id))?;
                self.connect_intersection(&mut intersection, false);
                self.intersections.insert(intersection_id, intersection);
                for vehicle in vehicles {
                    services.remove_and_cull_vehicle(self, vehicle)?;
                }
                let intersection = self
                    .intersections
                    .remove(&intersection_id)
                    .ok_or(RainNetworkError::MissingItem(intersection_id))?;
                self.memory[0].release(intersection.slot)?;
            }
            for source in record.super_roads()? {
                let super_id = source.id(0)?;
                let Some(super_road) = self.super_roads.get(&super_id) else {
                    continue;
                };
                if !super_road.roads.is_empty() {
                    continue;
                }
                let vehicles: Vec<_> = super_road
                    .lane_vehicles
                    .iter()
                    .flatten()
                    .chain(&super_road.parking_vehicles)
                    .copied()
                    .collect();
                for vehicle in vehicles {
                    services.remove_and_cull_vehicle(self, vehicle)?;
                }
                let super_road = self
                    .super_roads
                    .remove(&super_id)
                    .ok_or(RainNetworkError::MissingSuperRoad(super_id))?;
                self.memory[2].release(super_road.slot)?;
            }
            self.chunks.remove(id);
        }
        // These effects also happen when the requested chunk was not present.
        self.recalculate_bounds()?;
        services.invalidate_entity_rain_data()?;
        if incremental_kd_removal {
            self.pending_tree_chunks.remove(id);
        } else {
            self.reset_spatial()?;
            self.pending_tree_chunks.assign(&self.chunks)?;
        }
        Ok(())
    }
}
