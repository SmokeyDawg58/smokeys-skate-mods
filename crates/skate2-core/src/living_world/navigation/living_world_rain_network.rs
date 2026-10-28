//! Streamed RAIN entities in the existing RainMan owner.
//! Admission is source ordered; spatial publication is a separate Update phase.
use super::{
    RainMan, RainMemStoreConstructor,
    asset::{CurveData, Pin, RainAssetError, RainRecord, SuperRoadData},
    ped_nodes::PedRainNode,
};
use crate::{
    reckoning_numeric::{HostReckoningNumeric, cross_full4, try_normalize_safe},
    selected_simulation_asset_services::{SelectedAssetError, SelectedSimulationAssetServices},
};
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RainNetworkError {
    Numeric(crate::reckoning_numeric::ReckoningHostNumericError),
    Asset(RainAssetError),
    Registry(SelectedAssetError),
    Allocation,
    NotInitialized,
    AlreadyInitialized,
    SpatialItem(u64),
    SpatialQuery,
    MissingItem(u64),
    Vehicle(u32),
    MissingOwner,
    OwnerLock,
    CvarRead,
    Duplicate(u64),
    MissingSuperRoad(u64),
    PinIndex(u32),
    NavMesh(crate::living_world_mesh_store::MeshStoreError),
    NavTargetCapacity,
}
impl std::fmt::Display for RainNetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RAIN network: {self:?}")
    }
}
impl std::error::Error for RainNetworkError {}
impl From<RainAssetError> for RainNetworkError {
    fn from(e: RainAssetError) -> Self {
        Self::Asset(e)
    }
}
pub(crate) struct Segment {
    pub(crate) record: RainRecord,
    pub(crate) curve: CurveData,
    pub(crate) normal: [f32; 4],
}
pub(crate) struct Road {
    pub(crate) record: RainRecord,
    pub(crate) slot: u32,
    pub(crate) origin: Pin,
    pub(crate) destination: Pin,
    pub(crate) origin_valid: bool,
    pub(crate) destination_valid: bool,
    pub(crate) segments: Vec<Segment>,
    pub(crate) ped: PedRainNode,
}
pub(crate) struct Passage {
    pub(crate) record: RainRecord,
    pub(crate) curve: CurveData,
    pub(crate) vehicles: Vec<u32>,
}
pub(crate) struct Intersection {
    pub(crate) record: RainRecord,
    pub(crate) slot: u32,
    pub(crate) incoming: [Pin; 4],
    pub(crate) outgoing: [Pin; 4],
    pub(crate) incoming_valid: [bool; 4],
    pub(crate) outgoing_valid: [bool; 4],
    pub(crate) passages: Vec<Passage>,
    pub(crate) ped: [PedRainNode; 4],
    pub(crate) nav_targets: [super::nav_targets::NavTargetList; 4],
    pub(crate) nav_targets_generated: bool,
}
pub(crate) struct SuperRoad {
    pub(crate) data: SuperRoadData,
    pub(crate) slot: u32,
    /// Authored start segment plus road ID; AddRoad82e61210 sorts on start only.
    pub(crate) roads: Vec<(u32, u64)>,
    pub(crate) lane_vehicles: Vec<Vec<u32>>,
    pub(crate) lane_total_lengths: Vec<f32>,
    pub(crate) parking_vehicles: Vec<u32>,
    pub(crate) parking_positions: Vec<f32>,
}
impl RainMemStoreConstructor {
    fn initialize(&mut self, size: u32, count: u32) -> Result<(), RainNetworkError> {
        if self.free_list.is_some() {
            return Err(RainNetworkError::AlreadyInitialized);
        }
        let mut free = Vec::new();
        free.try_reserve_exact(count as usize)
            .map_err(|_| RainNetworkError::Allocation)?;
        free.extend((0..count).rev());
        let mut memory = Vec::new();
        memory
            .try_reserve_exact(size as usize * count as usize)
            .map_err(|_| RainNetworkError::Allocation)?;
        memory.resize_with(size as usize * count as usize, MaybeUninit::uninit);
        self.maximum_member_size = size;
        self.maximum_members = count;
        self.free_list = Some(free.into_boxed_slice());
        self.managed_memory = Some(memory.into_boxed_slice());
        self.free_list_index = Some(count as i32 - 1);
        Ok(())
    }
    pub(crate) fn request(&mut self) -> Result<Option<u32>, RainNetworkError> {
        let i = self
            .free_list_index
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?;
        if *i < 0 {
            return Ok(None);
        }
        let free = self
            .free_list
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?;
        let slot = free[*i as usize];
        free[*i as usize] = 0xdead_beef;
        *i -= 1;
        Ok(Some(slot))
    }
    pub(crate) fn release(&mut self, slot: u32) -> Result<(), RainNetworkError> {
        let i = self
            .free_list_index
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?;
        if slot >= self.maximum_members || *i + 1 >= self.maximum_members as i32 {
            return Err(RainNetworkError::NotInitialized);
        }
        *i += 1;
        self.free_list
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?[*i as usize] = slot;
        Ok(())
    }
    pub(crate) fn reset(&mut self) -> Result<(), RainNetworkError> {
        let free = self
            .free_list
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?;
        for (entry, slot) in free.iter_mut().zip((0..self.maximum_members).rev()) {
            *entry = slot;
        }
        self.free_list_index = Some(self.maximum_members as i32 - 1);
        Ok(())
    }
}
impl Road {
    fn new(record: RainRecord, slot: u32) -> Result<Self, RainNetworkError> {
        let origin = record.pin(8)?;
        let destination = record.pin(0x40)?;
        let mut segments = Vec::new();
        for segment in record.road_segments()? {
            let a = segment.vector(0x50)?;
            let b = segment.vector(0x60)?;
            let c = segment.vector(0x70)?;
            let d = segment.vector(0x80)?;
            // Segment82e54478: cross(V3-V0,V2-V1), shared NormalizeSafe(+Y).
            let da = std::array::from_fn(|i| d[i] - a[i]);
            let cb = std::array::from_fn(|i| c[i] - b[i]);
            let normal = try_normalize_safe(
                cross_full4(da, cb),
                [0., 1., 0., 0.],
                &mut HostReckoningNumeric,
            )
            .map_err(RainNetworkError::Numeric)?;
            segments.push(Segment {
                curve: segment.curve()?,
                record: segment,
                normal,
            });
        }
        Ok(Self {
            record,
            slot,
            origin,
            destination,
            origin_valid: false,
            destination_valid: false,
            segments,
            ped: PedRainNode::default(),
        })
    }
}
impl Intersection {
    fn new(record: RainRecord, slot: u32) -> Result<Self, RainNetworkError> {
        let incoming = [
            record.pin(0x80)?,
            record.pin(0xb8)?,
            record.pin(0xf0)?,
            record.pin(0x128)?,
        ];
        let outgoing = [
            record.pin(0x160)?,
            record.pin(0x198)?,
            record.pin(0x1d0)?,
            record.pin(0x208)?,
        ];
        let mut passages = Vec::new();
        for passage in record.intersection_passages()? {
            passages.push(Passage {
                curve: passage.curve()?,
                record: passage,
                vehicles: Vec::new(),
            });
        }
        Ok(Self {
            record,
            slot,
            incoming,
            outgoing,
            incoming_valid: [false; 4],
            outgoing_valid: [false; 4],
            passages,
            ped: [PedRainNode::default(); 4],
            nav_targets: std::array::from_fn(|_| super::nav_targets::NavTargetList::new()),
            nav_targets_generated: false,
        })
    }
}
impl RainMan {
    /// RAINMemory.Init826f3820 and RAIN.Init8271330c/24 storage subsequence.
    pub(crate) fn initialize_storage(&mut self) -> Result<(), RainNetworkError> {
        for (memory, (size, count)) in
            self.memory
                .iter_mut()
                .zip([(0xbf0, 72), (0xe0, 144), (0x98, 144), (8, 3168)])
        {
            memory.initialize(size, count)?;
        }
        use crate::living_world_grind_services::dmo_locators::LocatorKdTree;
        self.intersection_points =
            Some(LocatorKdTree::try_with_blocks(288).ok_or(RainNetworkError::Allocation)?);
        self.road_points =
            Some(LocatorKdTree::try_with_blocks(2880).ok_or(RainNetworkError::Allocation)?);
        self.spatial_items
            .try_reserve_exact(3168)
            .map_err(|_| RainNetworkError::Allocation)?;
        self.spatial_items.resize(3168, None);
        Ok(())
    }
    pub(crate) fn add_data(
        &mut self,
        id: u64,
        assets: &SelectedSimulationAssetServices,
    ) -> Result<(), RainNetworkError> {
        let Some(asset) = assets.try_inner(id).map_err(RainNetworkError::Registry)? else {
            return Ok(());
        };
        let record = RainRecord::new(asset)?;
        for source in record.super_roads()? {
            let data = source.super_road()?;
            if self.super_roads.contains_key(&data.id) {
                continue;
            }
            let Some(slot) = self.memory[2].request()? else {
                break;
            };
            let lanes = data.lanes + u32::from(data.flags & 1 != 0);
            self.super_roads.insert(
                data.id,
                Box::new(SuperRoad {
                    data,
                    slot,
                    roads: Vec::new(),
                    lane_vehicles: vec![Vec::new(); lanes as usize],
                    lane_total_lengths: vec![0.; lanes as usize],
                    parking_vehicles: Vec::new(),
                    parking_positions: Vec::new(),
                }),
            );
        }
        let mut added_roads = Vec::new();
        for source in record.roads()? {
            let Some(slot) = self.memory[1].request()? else {
                break;
            };
            let mut road = Road::new(source, slot)?;
            let road_id = road.record.id(0)?;
            self.connect_road(&mut road, true)?;
            let super_id = road.record.id(0x98)?;
            let super_road = self
                .super_roads
                .get_mut(&super_id)
                .ok_or(RainNetworkError::MissingSuperRoad(super_id))?;
            super_road.roads.push((road.record.word(0x94)?, road_id));
            super_road.roads.sort_unstable_by_key(|r| r.0);
            if self.roads.contains_key(&road_id) {
                return Err(RainNetworkError::Duplicate(road_id));
            }
            self.roads.insert(road_id, Box::new(road));
            added_roads.push(road_id);
        }
        let mut added_intersections = Vec::new();
        for source in record.intersections()? {
            let Some(slot) = self.memory[0].request()? else {
                break;
            };
            let mut intersection = Intersection::new(source, slot)?;
            let intersection_id = intersection.record.id(0x248)?;
            self.connect_intersection(&mut intersection, true);
            if self.intersections.contains_key(&intersection_id) {
                return Err(RainNetworkError::Duplicate(intersection_id));
            }
            self.intersections
                .insert(intersection_id, Box::new(intersection));
            added_intersections.push(intersection_id);
        }
        for id in added_roads {
            let road = self.roads.get_mut(&id).expect("admitted road");
            road.ped = PedRainNode::road(id, &road.origin, &road.destination);
        }
        for id in added_intersections {
            let intersection = self
                .intersections
                .get_mut(&id)
                .expect("admitted intersection");
            intersection.ped =
                PedRainNode::intersection(id, &intersection.incoming, &intersection.outgoing);
        }
        self.chunks.insert(id, record.clone(), true)?;
        self.recalculate_bounds()?;
        self.pending_tree_chunks.insert(id, record, false)?;
        Ok(())
    }
    /// Connect82701d68 / Disconnect82701ea0. The original destination-road
    /// branch reads the ORIGIN pin ID and modifies that road's destination;
    /// raw82701e20..48 confirms this asymmetry, so it is not "corrected" here.
    pub(crate) fn connect_road(
        &mut self,
        road: &mut Road,
        connect: bool,
    ) -> Result<(), RainNetworkError> {
        let origin = &road.origin;
        let found = match origin.item_type {
            0 => {
                if let Some(i) = self.intersections.get_mut(&origin.item_id) {
                    *i.outgoing_valid
                        .get_mut(origin.item_pin as usize)
                        .ok_or(RainNetworkError::PinIndex(origin.item_pin))? = connect;
                    true
                } else {
                    false
                }
            }
            1 => {
                if let Some(r) = self.roads.get_mut(&origin.item_id) {
                    r.destination_valid = connect;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if found || !connect {
            road.origin_valid = connect;
        }
        let dest = &road.destination;
        let found = match dest.item_type {
            0 => {
                if let Some(i) = self.intersections.get_mut(&dest.item_id) {
                    *i.incoming_valid
                        .get_mut(dest.item_pin as usize)
                        .ok_or(RainNetworkError::PinIndex(dest.item_pin))? = connect;
                    true
                } else {
                    false
                }
            }
            1 => {
                if let Some(r) = self.roads.get_mut(&origin.item_id) {
                    r.destination_valid = connect;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if found || !connect {
            road.destination_valid = connect;
        }
        Ok(())
    }
    pub(crate) fn connect_intersection(&mut self, intersection: &mut Intersection, connect: bool) {
        for i in 0..4 {
            if let Some(road) = self.roads.get_mut(&intersection.outgoing[i].item_id) {
                road.origin_valid = connect;
                intersection.outgoing_valid[i] = connect;
            }
            if !connect {
                intersection.outgoing_valid[i] = false;
            }
            if let Some(road) = self.roads.get_mut(&intersection.incoming[i].item_id) {
                road.destination_valid = connect;
                intersection.incoming_valid[i] = connect;
            }
            if !connect {
                intersection.incoming_valid[i] = false;
            }
        }
    }
    pub(crate) fn recalculate_bounds(&mut self) -> Result<(), RainNetworkError> {
        let mut min: Option<[f32; 4]> = None;
        let mut max = [0.; 4];
        for record in self.chunks.values() {
            let lo = record.vector(0)?;
            let hi = record.vector(16)?;
            match min.as_mut() {
                None => {
                    min = Some([lo[0], lo[2], lo[0], lo[0]]);
                    max = [hi[0], hi[2], hi[0], hi[0]];
                }
                Some(low) => {
                    for i in 0..2 {
                        let lane = i * 2;
                        if low[i] > lo[lane] {
                            low[i] = lo[lane];
                        }
                        if hi[lane] > max[i] {
                            max[i] = hi[lane];
                        }
                    }
                }
            }
        }
        let min = min.unwrap_or([0.; 4]);
        self.data_offset = crate::skeleton_root_transforms::SkeletonRootVector::known(min);
        self.data_width =
            crate::skeleton_root_transforms::SkeletonRootVector::known(std::array::from_fn(|i| {
                max[i] - min[i]
            }));
        Ok(())
    }
}
