//! RAIN spatial publication82713700/removal82726288 over the shared KD tree.
use super::{RainMan, asset::RainRecord, network::RainNetworkError};
use crate::{
    reckoning_numeric::HostReckoningNumeric,
    selected_simulation_asset_services::SelectedSimulationAssetServices,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RainItemKind {
    Intersection,
    Road,
}
/// Host identity for native tRAINItemData { IRAINItem*, int partIndex }.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RainItem {
    pub(crate) kind: RainItemKind,
    pub(crate) id: u64,
    pub(crate) part: u32,
}
fn vertices(record: &RainRecord, offset: usize) -> Result<[[f32; 2]; 4], RainNetworkError> {
    let mut result = [[0.; 2]; 4];
    for (i, point) in result.iter_mut().enumerate() {
        let v = record.vector(offset + i * 16)?;
        *point = [v[0], v[2]];
    }
    Ok(result)
}
fn triangle_centres(v: [[f32; 2]; 4]) -> [[f32; 2]; 2] {
    // Two adds, then multiply; no fused multiply/add in the original.
    std::array::from_fn(|tri| {
        std::array::from_fn(|lane| {
            ((v[tri][lane] + v[tri + 1][lane]) + v[tri + 2][lane]) * f32::from_bits(0x3eaa_aaab)
        })
    })
}
impl RainMan {
    /// GetNearestIntersection82713b18 uses the same published KD tree and
    /// traversal tie order, without a fabricated search radius.
    pub(crate) fn nearest_intersection(
        &self,
        point: [f32; 2],
    ) -> Result<Option<u64>, RainNetworkError> {
        let found = self
            .intersection_points
            .as_ref()
            .ok_or(RainNetworkError::NotInitialized)?
            .nearest(point, &mut HostReckoningNumeric)
            .map_err(RainNetworkError::Numeric)?;
        let Some((slot, _)) = found else {
            return Ok(None);
        };
        let item = self
            .spatial_items
            .get(slot as usize)
            .and_then(|item| *item)
            .ok_or(RainNetworkError::SpatialItem(slot))?;
        Ok(Some(item.id))
    }
    /// GetRoadData827267a8: the existing road KD/triangle path only; an
    /// overlapping intersection must not replace the selected road for Spawn.
    pub(crate) fn road_data(&self, point: [f32; 2]) -> Result<Option<RainItem>, RainNetworkError> {
        self.point_data(point, RainItemKind::Road)
    }
    /// Intersection-only lookup used by IsNearCrossWalk82706a60. Never
    /// substitute nearest_intersection, which lacks the containment test.
    pub(crate) fn intersection_data(&self,point:[f32;2])->Result<Option<RainItem>,RainNetworkError>{
        self.point_data(point,RainItemKind::Intersection)
    }
    /// Update8271e3ec..45c publishes exactly one pending chunk in native hash
    /// iteration order. Population rereads the live registry winner.
    pub(crate) fn publish_pending_zone(
        &mut self,
        assets: &SelectedSimulationAssetServices,
    ) -> Result<(), RainNetworkError> {
        if let Some(id) = self.pending_tree_chunks.first_id() {
            self.populate_zones(id, assets)?;
            self.pending_tree_chunks.remove(id);
        }
        Ok(())
    }
    /// GetPointRAINData82702118: intersections first, then roads. These are
    /// triangle containment queries, not an unrestricted nearest-road snap.
    pub(crate) fn point_rain_data(
        &self,
        point: [f32; 3],
    ) -> Result<Option<RainItem>, RainNetworkError> {
        if let Some(item) = self.point_data([point[0], point[2]], RainItemKind::Intersection)? {
            return Ok(Some(item));
        }
        self.point_data([point[0], point[2]], RainItemKind::Road)
    }
    fn point_data(
        &self,
        point: [f32; 2],
        kind: RainItemKind,
    ) -> Result<Option<RainItem>, RainNetworkError> {
        use crate::living_world_grind_services::dmo_locators::LocatorError;
        let (tree, radius) = match kind {
            RainItemKind::Intersection => (&self.intersection_points, 40.),
            RainItemKind::Road => (&self.road_points, 10.),
        };
        let found = tree
            .as_ref()
            .ok_or(RainNetworkError::NotInitialized)?
            .find_near_numeric(point, radius, 16, &mut HostReckoningNumeric)
            .map_err(|e| match e {
                LocatorError::Services(e) => RainNetworkError::Numeric(e),
                _ => RainNetworkError::SpatialQuery,
            })?;
        for (slot, _) in found {
            let item = self
                .spatial_items
                .get(slot as usize)
                .and_then(|i| *i)
                .ok_or(RainNetworkError::SpatialItem(slot))?;
            let triangle = match kind {
                RainItemKind::Intersection => {
                    let record = &self
                        .intersections
                        .get(&item.id)
                        .ok_or(RainNetworkError::MissingItem(item.id))?
                        .record;
                    [
                        record.vector(0x40)?,
                        record.vector((item.part as usize + 5) * 16)?,
                        record.vector((item.part as usize + 6) * 16)?,
                    ]
                }
                RainItemKind::Road => {
                    let road = self
                        .roads
                        .get(&item.id)
                        .ok_or(RainNetworkError::MissingItem(item.id))?;
                    let record = &road
                        .segments
                        .get(item.part as usize / 2)
                        .ok_or(RainNetworkError::SpatialItem(slot))?
                        .record;
                    let at = (item.part as usize % 2 + 5) * 16;
                    [
                        record.vector(at)?,
                        record.vector(at + 16)?,
                        record.vector(at + 32)?,
                    ]
                }
            };
            if crate::living_world_mesh_store::asset::point_in_triangle_xz(
                [point[0], 0., point[1]],
                triangle.map(|v| [v[0], v[1], v[2]]),
            ) {
                return Ok(Some(item));
            }
        }
        Ok(None)
    }
    fn allocate_spatial_item(&mut self, item: RainItem) -> Result<Option<u32>, RainNetworkError> {
        let Some(slot) = self.memory[3].request()? else {
            return Ok(None);
        };
        *self
            .spatial_items
            .get_mut(slot as usize)
            .ok_or(RainNetworkError::SpatialItem(slot as u64))? = Some(item);
        Ok(Some(slot))
    }
    fn release_spatial_item(&mut self, slot: u64) -> Result<(), RainNetworkError> {
        let item = self
            .spatial_items
            .get_mut(slot as usize)
            .ok_or(RainNetworkError::SpatialItem(slot))?;
        if item.take().is_none() {
            return Err(RainNetworkError::SpatialItem(slot));
        }
        self.memory[3].release(slot as u32)
    }
    pub(crate) fn populate_zones(
        &mut self,
        id: u64,
        assets: &SelectedSimulationAssetServices,
    ) -> Result<(), RainNetworkError> {
        let Some(asset) = assets.try_inner(id).map_err(RainNetworkError::Registry)? else {
            return Ok(());
        };
        let record = RainRecord::new(asset)?;
        for source in record.roads()? {
            let id = source.id(0)?;
            let Some(road) = self.roads.get(&id) else {
                continue;
            };
            let segments: Vec<_> = road.segments.iter().map(|s| s.record.clone()).collect();
            for (index, segment) in segments.iter().enumerate() {
                let coords = triangle_centres(vertices(segment, 0x50)?);
                let Some(first) = self.allocate_spatial_item(RainItem {
                    kind: RainItemKind::Road,
                    id,
                    part: index as u32 * 2,
                })?
                else {
                    break;
                };
                // Native retains the first allocation if the second fails.
                let Some(second) = self.allocate_spatial_item(RainItem {
                    kind: RainItemKind::Road,
                    id,
                    part: index as u32 * 2 + 1,
                })?
                else {
                    break;
                };
                let tree = self
                    .road_points
                    .as_mut()
                    .ok_or(RainNetworkError::NotInitialized)?;
                // KD Insert's allocation failure is silent in PopulateRAINZones.
                tree.insert(first as u64, coords[0]);
                tree.insert(second as u64, coords[1]);
            }
        }
        for source in record.intersections()? {
            let id = source.id(0x248)?;
            let Some(intersection) = self.intersections.get(&id) else {
                continue;
            };
            let coords = vertices(&intersection.record, 0x40)?;
            let Some(first) = self.allocate_spatial_item(RainItem {
                kind: RainItemKind::Intersection,
                id,
                part: 0,
            })?
            else {
                return Ok(());
            };
            let Some(second) = self.allocate_spatial_item(RainItem {
                kind: RainItemKind::Intersection,
                id,
                part: 1,
            })?
            else {
                return Ok(());
            };
            let tree = self
                .intersection_points
                .as_mut()
                .ok_or(RainNetworkError::NotInitialized)?;
            for (slot, coord) in [first, first, second, second].into_iter().zip(coords) {
                tree.insert(slot as u64, coord);
            }
        }
        Ok(())
    }
    pub(crate) fn depopulate_zones(
        &mut self,
        id: u64,
        assets: &SelectedSimulationAssetServices,
    ) -> Result<(), RainNetworkError> {
        let Some(asset) = assets.try_inner(id).map_err(RainNetworkError::Registry)? else {
            return Ok(());
        };
        let record = RainRecord::new(asset)?;
        for source in record.roads()? {
            let Some(road) = self.roads.get(&source.id(0)?) else {
                continue;
            };
            let segments: Vec<_> = road.segments.iter().map(|s| s.record.clone()).collect();
            for segment in segments {
                for coord in triangle_centres(vertices(&segment, 0x50)?) {
                    let removed = self
                        .road_points
                        .as_mut()
                        .ok_or(RainNetworkError::NotInitialized)?
                        .remove_coordinate(coord, &mut HostReckoningNumeric)
                        .map_err(RainNetworkError::Numeric)?;
                    if let Some(slot) = removed {
                        self.release_spatial_item(slot)?;
                    }
                }
            }
        }
        for source in record.intersections()? {
            let Some(intersection) = self.intersections.get(&source.id(0x248)?) else {
                continue;
            };
            let coords = vertices(&intersection.record, 0x40)?;
            let mut removed = [None; 2];
            for (i, coord) in coords.into_iter().enumerate() {
                if let Some(slot) = self
                    .intersection_points
                    .as_mut()
                    .ok_or(RainNetworkError::NotInitialized)?
                    .remove_coordinate(coord, &mut HostReckoningNumeric)
                    .map_err(RainNetworkError::Numeric)?
                {
                    removed[i / 2] = Some(slot);
                }
            }
            for slot in removed.into_iter().flatten() {
                self.release_spatial_item(slot)?;
            }
        }
        Ok(())
    }
    pub(crate) fn reset_spatial(&mut self) -> Result<(), RainNetworkError> {
        self.intersection_points
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?
            .reset();
        self.road_points
            .as_mut()
            .ok_or(RainNetworkError::NotInitialized)?
            .reset();
        self.memory[3].reset()?;
        self.spatial_items.fill(None);
        Ok(())
    }
}
