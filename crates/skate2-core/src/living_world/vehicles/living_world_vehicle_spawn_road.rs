//! VehicleCensus.Spawn82d27de0 road/lane admission prefix, before factory use.
//! Uses the SAME RAIN spatial data and vehicle queues; does not fabricate a
//! nearest-road placement, spawn an entity, or consume randomness too early.
use crate::{
    living_world_rain::{
        RainMan,
        asset::RainAssetError,
        network::RainNetworkError,
        spatial::RainItemKind,
        vehicle_lanes::{LaneError, RainVehicles},
    },
    native_skater_random::{NativeRandomGate, NativeRandomLocked, NativeSkaterRandom},
};

pub(crate) struct VehicleSpawnRoad {
    pub(crate) road_id: u64,
    pub(crate) super_position: f32,
    lanes: Vec<u32>,
}
#[derive(Debug)]
pub(crate) enum SpawnRoadError<E> {
    Rain(RainNetworkError),
    Asset(RainAssetError),
    MissingRoad(u64),
    MissingSegment(u32),
    Lanes(LaneError<E>),
}
impl VehicleSpawnRoad {
    pub(crate) fn select<V: RainVehicles>(
        rain: &RainMan,
        spawn_position: [f32; 4],
        force_lane_zero: bool,
        vehicles: &mut V,
    ) -> Result<Option<Self>, SpawnRoadError<V::Error>> {
        use SpawnRoadError as E;
        let Some(item) = rain
            .road_data([spawn_position[0], spawn_position[2]])
            .map_err(E::Rain)?
        else {
            return Ok(None);
        };
        if item.kind != RainItemKind::Road {
            return Ok(None);
        }
        let road = rain.roads.get(&item.id).ok_or(E::MissingRoad(item.id))?;
        let super_id = road.record.id(0x98).map_err(E::Asset)?;
        let Some(super_road) = rain.super_roads.get(&super_id) else {
            return Ok(None);
        };
        let segment = road
            .segments
            .get(item.part as usize / 2)
            .ok_or(E::MissingSegment(item.part))?;
        // Raw fsubs/fsubs/fadds82d27ecc..eec, not a closest-point curve fit.
        let road_start = road.record.scalar(0x90).map_err(E::Asset)?
            - road.record.scalar(0x8c).map_err(E::Asset)?;
        let segment_start = segment.record.scalar(0xd0).map_err(E::Asset)? - segment.curve.length;
        let super_position = segment_start + road_start;
        // Raw blt/bgt: both endpoints allowed; unordered is not rejected here.
        if super_position < 15. || super_position > super_road.data.length - 15. {
            return Ok(None);
        }
        let mut lanes = Vec::new();
        if force_lane_zero {
            // Original override flag branch82d27f3c bypasses clearance checks.
            lanes.push(0);
        } else {
            for lane in 0..super_road.data.lanes {
                if super_road
                    .can_place_vehicle(super_position, lane, 15., 0., false, vehicles)
                    .map_err(E::Lanes)?
                {
                    lanes.push(lane);
                }
            }
        }
        if lanes.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self {
            road_id: item.id,
            super_position,
            lanes,
        }))
    }

    /// Call ONLY after successful InitModels/model check. Native82d28150..b8
    /// uses Manager.GetRandom(0,1), rounded *100, fctiwz then unsigned modulo.
    /// It draws even for one eligible lane; source-global rw::math is distinct.
    pub(crate) fn choose_lane(
        &self,
        random: &mut NativeSkaterRandom,
        gate: &NativeRandomGate,
    ) -> Result<u32, NativeRandomLocked> {
        let value = random.try_living_world_range(gate, 0., 1.)? * 100.;
        let word = value as i32 as u32;
        Ok(self.lanes[word as usize % self.lanes.len()])
    }
}
