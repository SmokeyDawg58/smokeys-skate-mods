//! Vehicle road-network teardown82d24488. Reuses native lane/intersection
//! membership owners, including both lanes of an initialized lane change.
use super::{
    VehicleMotion,
    driving::{DrivingError, DrivingVehicles, VehicleRoute},
    target_transform::VehicleTargetControl,
};
use crate::{
    living_world_mesh_store::MeshStore, living_world_rain::RainMan,
    reckoning_numeric::ReckoningNumeric,
};
impl VehicleRoute {
    /// AddToRoadNetwork82d17428. Raw ABI is IVehicle-this, f1 position,
    /// r4 full64 road ID, r5 lane; Hex-Rays incorrectly splits that ID. Lane
    /// insertion precedes the zero-dT target update; registered flag is LAST.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_to_road_network<V, C, N>(
        &mut self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        super_position: f32,
        road_id: u64,
        lane: u32,
        colliding: bool,
        rain: &mut RainMan,
        mesh: &MeshStore,
        vehicles: &mut V,
        control: &mut C,
        numeric: &mut N,
    ) -> Result<(), DrivingError<V::Error, N::Error>>
    where
        V: DrivingVehicles,
        C: VehicleTargetControl<Error = V::Error>,
        N: ReckoningNumeric,
    {
        let road = rain
            .roads
            .get(&road_id)
            .ok_or(DrivingError::Road(road_id))?;
        let aggregate = road.record.scalar(0x90)?;
        let road_length = road.record.scalar(0x8c)?;
        self.item_type = 1;
        self.item_id = road_id;
        self.item_position = super_position - (aggregate - road_length);
        motion.current_speed_limit = road.record.scalar(0x80)?;
        let super_id = road.record.id(0x98)?;
        self.super_road_position = super_position;
        self.super_road_id = super_id;
        self.road_lane = lane;
        rain.super_roads
            .get_mut(&super_id)
            .ok_or(DrivingError::SuperRoad(super_id))?
            .register_vehicle(
                id,
                lane,
                super_position,
                length + motion.minimum_vehicle_distance,
                vehicles,
            )
            .map_err(DrivingError::Lanes)?;
        // The selected update ignores dT; it evaluates current route position.
        self.update_target_transform(motion, colliding, length, rain, mesh, control, numeric)?;
        self.registered_to_road = true;
        Ok(())
    }

    pub(crate) fn remove_from_road_network<E, N>(
        &mut self,
        motion: &mut VehicleMotion,
        id: u32,
        length: f32,
        rain: &mut RainMan,
        mesh: &MeshStore,
    ) -> Result<(), DrivingError<E, N>> {
        if !self.registered_to_road {
            return Ok(());
        }
        self.remove_from_super_road(
            id,
            self.super_road_id,
            length + motion.minimum_vehicle_distance,
            rain,
        )?;
        self.remove_from_intersection(id, self.intersection_id, rain, mesh)?;
        if self.item_type == 1 {
            if let Some(road) = rain.roads.get(&self.item_id) {
                let super_id = road.record.id(0x98)?;
                self.remove_from_super_road(
                    id,
                    super_id,
                    length + motion.minimum_vehicle_distance,
                    rain,
                )?;
            }
        } else if self.item_type == 0 {
            self.remove_from_intersection(id, self.item_id, rain, mesh)?;
        }
        self.item_id = 0;
        self.item_type = 2;
        self.super_road_id = 0;
        self.item_position = -1.;
        self.intersection_id = 0;
        motion.current_speed_limit = 0.;
        self.passage_index = -1;
        self.super_road_position = -1.;
        // Raw IVehicle-this(-8c) store+df8 resolves to Vehicle+e84.
        self.horizontal_road_position = -1.;
        self.registered_in_intersection = false;
        self.registered_to_road = false;
        Ok(())
    }
    fn remove_from_super_road<E, N>(
        &mut self,
        id: u32,
        super_id: u64,
        length_span: f32,
        rain: &mut RainMan,
    ) -> Result<(), DrivingError<E, N>> {
        let Some(road) = rain.super_roads.get_mut(&super_id) else {
            return Ok(());
        };
        if self.lane_change_passage.initialized {
            road.unregister_vehicle(id, self.lane_change_passage.origin_lane, length_span)
                .map_err(DrivingError::Lanes)?;
            road.unregister_vehicle(id, self.lane_change_passage.destination_lane, length_span)
                .map_err(DrivingError::Lanes)?;
            self.lane_change_passage.deinit();
        } else {
            road.unregister_vehicle(id, self.road_lane, length_span)
                .map_err(DrivingError::Lanes)?;
        }
        if self.registered_in_parking {
            road.unregister_parking(id).map_err(DrivingError::Lanes)?;
            self.registered_in_parking = false;
        }
        Ok(())
    }
    fn remove_from_intersection<E, N>(
        &self,
        id: u32,
        intersection_id: u64,
        rain: &mut RainMan,
        mesh: &MeshStore,
    ) -> Result<(), DrivingError<E, N>> {
        let Some(intersection) = rain
            .intersection(intersection_id, mesh)
            .map_err(DrivingError::Rain)?
        else {
            return Ok(());
        };
        if self.passage_index >= 0 {
            if let Some(passage) = intersection.passages.get_mut(self.passage_index as usize) {
                // RemoveFromIntersection82d24340 removes first matching ID;
                // distinct from the normal departure hook's pop_back.
                if let Some(index) = passage.vehicles.iter().position(|&v| v == id) {
                    passage.vehicles.remove(index);
                }
            }
        }
        Ok(())
    }
}
