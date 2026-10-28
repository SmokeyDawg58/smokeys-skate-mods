//! SuperRoad's original vehicle queues; same lists retained by streamed RAIN.
use super::network::SuperRoad;

pub(crate) trait RainVehicles {
    type Error;
    fn position(&mut self, id: u32) -> Result<f32, Self::Error>;
    fn length(&mut self, id: u32) -> Result<f32, Self::Error>;
    fn speed(&mut self, id: u32) -> Result<f32, Self::Error>;
    fn require_vehicle(&mut self, id: u32) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum LaneError<E> {
    Vehicle(E),
    Lane(u32),
    Capacity,
    Unregistered(u32),
    ParkingListMismatch,
}
#[derive(Debug)]
pub(crate) enum LaneChoiceError<E> {
    Lane(u32),
    Random(E),
}
impl SuperRoad {
    /// GetNearestMostEmptyLane82e62048: only adjacent driving lanes, right
    /// first. Density ties keep current lane; random mode draws even for one.
    pub(crate) fn nearest_most_empty_lane<E>(
        &self,
        lane: u32,
        check_density: bool,
        random_word: &mut impl FnMut() -> Result<u32, E>,
    ) -> Result<u32, LaneChoiceError<E>> {
        let mut candidates = Vec::with_capacity(2);
        if self.data.lanes > 1 && lane < self.data.lanes - 1 {
            candidates.push(lane + 1);
        }
        if lane != 0 && lane <= self.data.lanes.wrapping_sub(1) {
            candidates.push(lane - 1);
        }
        if check_density {
            let inverse = 1. / self.data.length;
            let mut density = inverse
                * *self
                    .lane_total_lengths
                    .get(lane as usize)
                    .ok_or(LaneChoiceError::Lane(lane))?;
            let mut selected = lane;
            for candidate in candidates {
                let next = inverse
                    * *self
                        .lane_total_lengths
                        .get(candidate as usize)
                        .ok_or(LaneChoiceError::Lane(candidate))?;
                if next < density {
                    selected = candidate;
                    density = next;
                }
            }
            Ok(selected)
        } else if candidates.is_empty() {
            Ok(lane)
        } else {
            Ok(candidates
                [random_word().map_err(LaneChoiceError::Random)? as usize % candidates.len()])
        }
    }
    /// RegisterVehicleToLane82e5db10: stable ascending position insertion,
    /// capacity assertion32, supplied lengthSpan added only after insertion.
    pub(crate) fn register_vehicle<V: RainVehicles>(
        &mut self,
        id: u32,
        lane: u32,
        distance: f32,
        length_span: f32,
        vehicles: &mut V,
    ) -> Result<(), LaneError<V::Error>> {
        let entries = self
            .lane_vehicles
            .get_mut(lane as usize)
            .ok_or(LaneError::Lane(lane))?;
        if entries.len() >= 32 {
            return Err(LaneError::Capacity);
        }
        let mut index = entries.len();
        for (i, &other) in entries.iter().enumerate() {
            if distance < vehicles.position(other).map_err(LaneError::Vehicle)? {
                index = i;
                break;
            }
        }
        entries.insert(index, id);
        *self
            .lane_total_lengths
            .get_mut(lane as usize)
            .ok_or(LaneError::Lane(lane))? += length_span;
        Ok(())
    }
    /// UnregisterVehicleFromLane82e5c350: first matching occurrence only.
    /// Missing membership leaves the total untouched; do not clamp subtraction.
    pub(crate) fn unregister_vehicle<E>(
        &mut self,
        id: u32,
        lane: u32,
        length_span: f32,
    ) -> Result<(), LaneError<E>> {
        let entries = self
            .lane_vehicles
            .get_mut(lane as usize)
            .ok_or(LaneError::Lane(lane))?;
        if let Some(index) = entries.iter().position(|&i| i == id) {
            entries.remove(index);
            *self
                .lane_total_lengths
                .get_mut(lane as usize)
                .ok_or(LaneError::Lane(lane))? -= length_span;
        }
        Ok(())
    }
    /// RegisterVehicleToParkingLane82e5dc98 preserves paired list order.
    pub(crate) fn register_parking<E>(
        &mut self,
        id: u32,
        distance: f32,
    ) -> Result<(), LaneError<E>> {
        if self.parking_vehicles.len() >= 32 {
            return Err(LaneError::Capacity);
        }
        if self.parking_vehicles.len() != self.parking_positions.len() {
            return Err(LaneError::ParkingListMismatch);
        }
        let index = self
            .parking_positions
            .iter()
            .position(|&p| p > distance)
            .unwrap_or(self.parking_positions.len());
        self.parking_vehicles.insert(index, id);
        self.parking_positions.insert(index, distance);
        Ok(())
    }
    /// UnregisterVehicleFromParkingLane82e5c418.
    pub(crate) fn unregister_parking<E>(&mut self, id: u32) -> Result<(), LaneError<E>> {
        if self.parking_vehicles.len() != self.parking_positions.len() {
            return Err(LaneError::ParkingListMismatch);
        }
        if let Some(index) = self.parking_vehicles.iter().position(|&i| i == id) {
            self.parking_vehicles.remove(index);
            self.parking_positions.remove(index);
        }
        Ok(())
    }
    /// GetFrontVehicleIndex82e5abe0/GetRearVehicleIndex82e5ac50. EntityID0
    /// is the source no-neighbor sentinel; absent membership is a host error.
    pub(crate) fn neighbor<E>(&self, lane: u32, id: u32, front: bool) -> Result<u32, LaneError<E>> {
        let entries = self
            .lane_vehicles
            .get(lane as usize)
            .ok_or(LaneError::Lane(lane))?;
        let endpoint = if front {
            entries.last()
        } else {
            entries.first()
        };
        if endpoint.copied().unwrap_or(0) == id {
            return Ok(0);
        }
        let index = entries
            .iter()
            .position(|&i| i == id)
            .ok_or(LaneError::Unregistered(id))?;
        Ok(if front {
            entries.get(index + 1).copied().unwrap_or(0)
        } else {
            index
                .checked_sub(1)
                .and_then(|i| entries.get(i))
                .copied()
                .unwrap_or(0)
        })
    }
    /// IsGoodToPutVehicleOnPos82e5a720. Front/rear clearance is asymmetric;
    /// source uses candidate speed ahead and existing rear vehicle speed behind.
    pub(crate) fn can_place_vehicle<V: RainVehicles>(
        &self,
        position: f32,
        lane: u32,
        length: f32,
        speed: f32,
        changing_lane: bool,
        vehicles: &mut V,
    ) -> Result<bool, LaneError<V::Error>> {
        if self.data.lanes == 0
            || position < length
            || position > self.data.length - length
            || changing_lane && (self.data.lanes < 2 || lane >= self.data.lanes)
        {
            return Ok(false);
        }
        let entries = self
            .lane_vehicles
            .get(lane as usize)
            .ok_or(LaneError::Lane(lane))?;
        let mut front = None;
        let mut rear = None;
        for &id in entries {
            let other = vehicles.position(id).map_err(LaneError::Vehicle)?;
            // Raw blt to front branch; an unordered position stays behind.
            if position < other {
                front = Some(id);
                break;
            } else {
                rear = Some(id);
            }
        }
        if let Some(id) = front {
            let other = vehicles.position(id).map_err(LaneError::Vehicle)?;
            let other_length = vehicles.length(id).map_err(LaneError::Vehicle)?;
            if (position + length) + speed > -(other_length.mul_add(0.5, -other)) {
                return Ok(false);
            }
        }
        if let Some(id) = rear {
            let other_speed = vehicles.speed(id).map_err(LaneError::Vehicle)?;
            let other_position = vehicles.position(id).map_err(LaneError::Vehicle)?;
            let other_length = vehicles.length(id).map_err(LaneError::Vehicle)?;
            if other_length + (other_speed + other_position) > -(length.mul_add(0.5, -position)) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// FindBestParkingSpotDistance82e5a990 searches gaps from farthest parked
    /// vehicle backwards; empty parking list returns midpoint of25%/75% bounds.
    pub(crate) fn best_parking_distance<V: RainVehicles>(
        &self,
        id: u32,
        vehicles: &mut V,
    ) -> Result<f32, LaneError<V::Error>> {
        vehicles.require_vehicle(id).map_err(LaneError::Vehicle)?;
        let mut upper = self.data.length * 0.75;
        if self.parking_vehicles.is_empty() {
            return Ok((self.data.length * 0.25 + upper) * 0.5);
        }
        if self.parking_vehicles.len() != self.parking_positions.len() {
            return Err(LaneError::ParkingListMismatch);
        }
        let speed = vehicles.speed(id).map_err(LaneError::Vehicle)?;
        let length = vehicles.length(id).map_err(LaneError::Vehicle)?;
        let position = vehicles.position(id).map_err(LaneError::Vehicle)?;
        let minimum = length.mul_add(1.5, position) + speed;
        for (&other, &parked_at) in self
            .parking_vehicles
            .iter()
            .zip(&self.parking_positions)
            .rev()
        {
            let other_length = vehicles.length(other).map_err(LaneError::Vehicle)?;
            let lower = parked_at + other_length * 0.5;
            if lower < upper {
                let candidate = (lower + upper) * 0.5;
                if candidate > minimum && upper - lower > length + 5. {
                    return Ok(candidate);
                }
                upper = lower - vehicles.length(other).map_err(LaneError::Vehicle)?;
            }
        }
        Ok(-1.)
    }
}
