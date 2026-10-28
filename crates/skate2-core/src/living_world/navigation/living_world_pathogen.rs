//! Pathogen retained mesh adjacency and native solver82e62ef0. Cell/edge
//! handles stay weak so streamed removal invalidates a path immediately.
use super::{MeshCell, MeshStore, MeshStoreError, asset::NavMeshAsset};
use crate::reckoning_numeric::{ReckoningNumeric, try_length};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex, Weak},
};

/// Shared parameter outputs of CustomCollision::Line_Line_Test82cf9548.
/// Original writes both distances even when its parallel-line flag is false;
/// callers must preserve their own subsequent comparisons on these values.
pub(crate) fn line_line_parameters<N: ReckoningNumeric>(
    start: [f32; 4], direction: [f32; 4], origin: [f32; 4],
    other_direction: [f32; 4], numeric: &mut N,
) -> Result<[f32; 2], N::Error> {
    Ok(line_line_result(start, direction, origin, other_direction, numeric)?.distances)
}

pub(crate) struct LineLineResult {
    pub(crate) distances: [f32; 2],
    pub(crate) parallel_score: f32,
}
/// geotest::ComputeBarycentricY82e54878. Despite its name this solves the
/// normalized triangle plane; it does not test whether X/Z is inside it.
pub(crate) fn triangle_height<N: ReckoningNumeric>(
    position: [f32; 4], points: [[f32; 4]; 3], numeric: &mut N,
) -> Result<Option<f32>, N::Error> {
    let a = std::array::from_fn(|i| points[1][i] - points[0][i]);
    let b = std::array::from_fn(|i| points[2][i] - points[0][i]);
    let normal = crate::reckoning_numeric::try_normalize_safe(
        crate::reckoning_numeric::cross_full4(a, b), [0.; 4], numeric)?;
    let vertical = numeric.dot3([0., 1., 0., 0.], normal)?;
    // EPSILON_190 at8238b0c0, raw abs(dot)>epsilon comparison.
    if !(vertical.abs() > f32::from_bits(0x3400_0000)) { return Ok(None); }
    let origin = points[0];
    let plane = origin[2].mul_add(normal[2],
        origin[0].mul_add(normal[0], origin[1] * normal[1]));
    let delta = plane - numeric.dot3(position, normal)?;
    let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal(
        vertical, numeric.reciprocal_estimate(vertical)?);
    Ok(Some(delta.mul_add(inverse, position[1])))
}
pub(crate) fn line_line_result<N: ReckoningNumeric>(
    start: [f32; 4], direction: [f32; 4], origin: [f32; 4],
    other_direction: [f32; 4], numeric: &mut N,
) -> Result<LineLineResult, N::Error> {
    use crate::reckoning_numeric::cross_full4;
    let cross = cross_full4(direction, other_direction);
    let difference = std::array::from_fn(|i| origin[i] - start[i]);
    let first = cross_full4(difference, other_direction);
    let second = cross_full4(difference, direction);
    let denominator = cross[2].mul_add(cross[2], cross[0].mul_add(cross[0], cross[1] * cross[1]));
    let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal(
        denominator, numeric.reciprocal_estimate(denominator)?);
    Ok(LineLineResult {
        distances: [numeric.dot3(first, cross)? * inverse, numeric.dot3(second, cross)? * inverse],
        parallel_score: denominator - f32::from_bits(0x3089_705f),
    })
}

#[derive(Clone)]
pub(crate) struct MeshEdge {
    tile: Weak<NavMeshAsset>,
    id: u32,
}
impl MeshEdge {
    pub(crate) fn points(&self) -> Result<Option<[[f32; 4]; 2]>, MeshStoreError> {
        self.tile
            .upgrade()
            .map(|tile| tile.edge_points(self.id).map_err(MeshStoreError::Nav))
            .transpose()
    }
    fn address(&self) -> Result<Option<usize>, MeshStoreError> {
        self.tile
            .upgrade()
            .map(|tile| tile.edge_address(self.id).map_err(MeshStoreError::Nav))
            .transpose()
    }
}
impl MeshCell {
    fn address(&self) -> Result<Option<usize>, MeshStoreError> {
        match self {
            Self::Absent => Ok(None),
            Self::Present { tile, cell } => tile
                .upgrade()
                .map(|tile| tile.cell_address(cell.id).map_err(MeshStoreError::Nav))
                .transpose(),
        }
    }
    pub(crate) fn edges(&self) -> Result<Vec<MeshEdge>, MeshStoreError> {
        let Self::Present { tile, cell } = self else {
            return Ok(Vec::new());
        };
        let Some(owner) = tile.upgrade() else {
            return Ok(Vec::new());
        };
        Ok(owner
            .cell_edges(cell.id)
            .map_err(MeshStoreError::Nav)?
            .into_iter()
            .map(|id| MeshEdge {
                tile: tile.clone(),
                id,
            })
            .collect())
    }
}
impl MeshStore {
    fn resolve_cell(
        &self,
        owner: &NavMeshAsset,
        reference: u32,
    ) -> Result<MeshCell, MeshStoreError> {
        let index = reference & 0x1fff_ffff;
        let neighbour = (reference >> 29) as usize;
        if reference == u32::MAX || neighbour == 7 || index == 0x1fff_ffff {
            return Ok(MeshCell::Absent);
        }
        let fields = self.fields.lock().map_err(|_| MeshStoreError::OwnerLock)?;
        let Some(source) = fields.tile_map.get(&owner.asset_id()) else {
            return Ok(MeshCell::Absent);
        };
        // A query handle from an earlier admission must not resolve through a
        // replacement tile of the same asset ID.
        if !std::ptr::eq(source.data.as_ref(), owner) {
            return Ok(MeshCell::Absent);
        }
        let Some(id) = source.neighbours.get(neighbour).copied().flatten() else {
            return Ok(MeshCell::Absent);
        };
        let Some(tile) = fields.tile_map.get(&id) else {
            return Ok(MeshCell::Absent);
        };
        Ok(MeshCell::Present {
            tile: Arc::downgrade(&tile.data),
            cell: tile.data.cell(index).map_err(MeshStoreError::Nav)?,
        })
    }
    pub(crate) fn edge_cells(&self, edge: &MeshEdge) -> Result<[MeshCell; 2], MeshStoreError> {
        let Some(tile) = edge.tile.upgrade() else {
            return Ok([MeshCell::Absent, MeshCell::Absent]);
        };
        let ids = tile.edge_cells(edge.id).map_err(MeshStoreError::Nav)?;
        Ok([
            self.resolve_cell(&tile, ids[0])?,
            self.resolve_cell(&tile, ids[1])?,
        ])
    }
    pub(crate) fn cell_neighbours(&self, cell: &MeshCell) -> Result<Vec<MeshCell>, MeshStoreError> {
        let MeshCell::Present { tile, cell } = cell else {
            return Ok(Vec::new());
        };
        let Some(tile) = tile.upgrade() else {
            return Ok(Vec::new());
        };
        tile.cell_neighbours(cell.id)
            .map_err(MeshStoreError::Nav)?
            .into_iter()
            .map(|id| self.resolve_cell(&tile, id))
            .collect()
    }
}

pub(crate) struct NavPath {
    pub(crate) start: MeshCell,
    pub(crate) end: MeshCell,
    pub(crate) current_cell: MeshCell,
    pub(crate) target_cell: MeshCell,
    pub(crate) valid: bool,
    pub(crate) high_fidelity_validation: bool,
    pub(crate) navigation_cost: f32,
    pub(crate) cells: Vec<MeshCell>,
    pub(crate) edges: Vec<MeshEdge>,
}
impl NavPath {
    /// MeshNavigation::ReleaseNavMeshTile82e6c060 scans edges, then valid cells.
    pub(crate) fn uses_tile(&self, owner: &Arc<NavMeshAsset>) -> bool {
        let handle = Arc::downgrade(owner);
        self.edges.iter().any(|edge|edge.tile.ptr_eq(&handle)) || self.cells.iter().any(|cell|
            matches!(cell, MeshCell::Present {tile, ..} if tile.ptr_eq(&handle) && tile.upgrade().is_some()))
    }
    /// TargetProvider::AddNavRigPathBoundsLineTestsToBatch82e52a88.
    /// Preserve ray-major submission order and authored edge direction/length.
    /// Unlike LineIntersectsEdgeList, the edge origin is not flattened to Y.
    pub(crate) fn add_nav_rig_tests(
        &self,
        position: [f32; 4],
        directions: &[[f32; 4]; 7],
        distances: &crate::pedestrian_navigation::rig::NavRigDistances,
        owner: u32,
        batch: &mut crate::pedestrian_navigation::rig::NavRigBatch,
    ) -> Result<std::ops::Range<usize>, String> {
        use crate::pedestrian_navigation::rig::NavRigTest;
        let begin = batch.count();
        for (ray, &direction) in directions.iter().enumerate() {
            for edge in &self.edges {
                let tile = edge.tile.upgrade().ok_or("Pathogen boundary tile unloaded")?;
                let origin = tile.edge_points(edge.id).map_err(|e| format!("{e:?}"))?[0];
                let (edge_direction, edge_length) =
                    tile.edge_line(edge.id).map_err(|e| format!("{e:?}"))?;
                batch.add(owner, NavRigTest {
                    starts: [position, origin],
                    headings: [direction, edge_direction],
                    magnitudes: [distances.distance(ray)?, edge_length],
                    ray,
                })?;
            }
        }
        Ok(begin..batch.count())
    }

    /// LineIntersectsEdgeList82e58f00 flattens the test line and edge origin to
    /// the start Y. Edge unit direction/length are authored, not recomputed.
    fn line_intersects<N: ReckoningNumeric>(
        &self,
        start: [f32; 4],
        destination: [f32; 4],
        numeric: &mut N,
    ) -> Result<bool, String>
    where N::Error: Debug,
    {
        Ok(self.line_intersection_distance(start, destination, numeric)?.is_some())
    }

    /// Native returns the first qualifying edge in authored order, not the
    /// nearest edge. Retain its distance output for navigation-rig validation.
    fn line_intersection_distance<N: ReckoningNumeric>(
        &self,
        start: [f32; 4],
        mut destination: [f32; 4],
        numeric: &mut N,
    ) -> Result<Option<f32>, String>
    where
        N::Error: Debug,
    {
        use crate::reckoning_numeric::try_normalize_unsafe;
        destination[1] = start[1];
        let delta: [f32; 4] = std::array::from_fn(|i| destination[i] - start[i]);
        let direction = try_normalize_unsafe(delta, numeric)
            .map_err(|e| format!("Pathogen line normalize: {e:?}"))?;
        let length =
            try_length(delta, numeric).map_err(|e| format!("Pathogen line length: {e:?}"))?;
        for edge in &self.edges {
            let Some(tile) = edge.tile.upgrade() else {
                return Err("Pathogen boundary tile unloaded".into());
            };
            let mut origin = tile.edge_points(edge.id).map_err(|e| format!("{e:?}"))?[0];
            origin[1] = start[1];
            let (edge_direction, edge_length) =
                tile.edge_line(edge.id).map_err(|e| format!("{e:?}"))?;
            let [distance1, distance2] = line_line_parameters(start, direction, origin, edge_direction, numeric)
                .map_err(|e| format!("Pathogen line intersection: {e:?}"))?;
            if distance2 >= 0. && edge_length >= distance2 && distance1 >= 0. && length >= distance1
            {
                return Ok(Some(distance1));
            }
        }
        Ok(None)
    }
    /// ResolveNavRigWithinPathBounds82e5b780, the default non-batched path.
    pub(crate) fn resolve_nav_rig<N: ReckoningNumeric>(
        &self,
        position: [f32; 4],
        directions: &[[f32; 4]; 7],
        distances: &mut crate::pedestrian_navigation::rig::NavRigDistances,
        numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: Debug,
    {
        for (ray, direction) in directions.iter().enumerate() {
            let length = distances.distance(ray)?;
            let end = std::array::from_fn(|i| direction[i].mul_add(length, position[i]));
            if let Some(distance) = self.line_intersection_distance(position, end, numeric)? {
                distances.set_volume(ray, distance)?;
            }
        }
        Ok(())
    }
    /// FindFurthestValidPathTarget82e5d430: original destination first, then
    /// cell centroids and each authored vertex in reverse path-cell order.
    pub(crate) fn furthest_target<N: ReckoningNumeric>(
        &self,
        position: [f32; 4],
        destination: [f32; 4],
        numeric: &mut N,
    ) -> Result<Option<[f32; 4]>, String>
    where
        N::Error: Debug,
    {
        if !self.valid || self.edges.is_empty() {
            return Ok(None);
        }
        if !self.line_intersects(position, destination, numeric)? {
            return Ok(Some(destination));
        }
        for cell in self.cells.iter().rev() {
            let Some((_, centroid)) = cell.cost_and_centroid() else {
                return Err("Pathogen path tile unloaded".into());
            };
            if !self.line_intersects(position, centroid, numeric)? {
                return Ok(Some(centroid));
            }
            let MeshCell::Present { tile, cell } = cell else {
                unreachable!()
            };
            let tile = tile.upgrade().ok_or("Pathogen path tile unloaded")?;
            for point in tile.cell_points(cell.id).map_err(|e| format!("{e:?}"))? {
                if !self.line_intersects(position, point, numeric)? {
                    return Ok(Some(point));
                }
            }
        }
        Ok(None)
    }
    /// Reset824b3890 is also the owner initialization publication.
    pub(crate) fn reset(start: MeshCell, end: MeshCell) -> Self {
        Self {
            current_cell: start.clone(),
            target_cell: end.clone(),
            start,
            end,
            valid: false,
            high_fidelity_validation: false,
            navigation_cost: 0.,
            cells: Vec::new(),
            edges: Vec::new(),
        }
    }
}
struct Proxy {
    cell: MeshCell,
    weight: f32,
    address: usize,
    parent: Option<usize>,
}
fn less(a: &Proxy, b: &Proxy) -> bool {
    a.weight < b.weight || a.weight == b.weight && a.address < b.address
}
fn insert(set: &mut Vec<usize>, index: usize, proxies: &[Proxy]) {
    let position = set.partition_point(|&i| less(&proxies[i], &proxies[index]));
    if set
        .get(position)
        .is_none_or(|&i| less(&proxies[index], &proxies[i]))
    {
        set.insert(position, index);
    }
}

/// Original single Solver owner and mutex. Scratch lists reset on each solve;
/// neither a second mesh registry nor a retained clone of streamed tile data.
pub(crate) struct PathSolver {
    lock: Mutex<()>,
}
impl PathSolver {
    pub(crate) fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }
    /// SolvePathBoundary82e60750 broadens through zero-score neighbors once,
    /// then twice more; the final unique perimeter is capped at120 edges.
    pub(crate) fn boundary(
        &self,
        mesh: &MeshStore,
        path: &mut NavPath,
        zero_threshold: f32,
        tests: [bool; 3],
    ) -> Result<(), String> {
        let _lock = self
            .lock
            .lock()
            .map_err(|_| "Pathogen solver lock".to_string())?;
        if !path.valid {
            return Ok(());
        }
        if path.cells.len() >= 256 {
            return Err("Pathogen boundary path capacity256".into());
        }
        let insert_cell =
            |set: &mut Vec<(usize, MeshCell)>, cell: MeshCell| -> Result<(), String> {
                let Some(address) = cell.address().map_err(|e| e.to_string())? else {
                    return Ok(());
                };
                let index = set.partition_point(|(key, _)| *key < address);
                if set.get(index).is_none_or(|(key, _)| *key != address) {
                    if set.len() >= 512 {
                        return Err("Pathogen broaden capacity512".into());
                    }
                    set.insert(index, (address, cell));
                }
                Ok(())
            };
        let mut broaden = Vec::new();
        for cell in &path.cells {
            insert_cell(&mut broaden, cell.clone())?;
            if tests[0] {
                for neighbor in mesh.cell_neighbours(cell).map_err(|e| e.to_string())? {
                    if neighbor.is_valid() && neighbor.score_with_threshold(zero_threshold) == 0. {
                        insert_cell(&mut broaden, neighbor)?;
                    }
                }
            }
        }
        if tests[1] {
            for _ in 0..2 {
                let mut temporary = Vec::new();
                for (_, cell) in &broaden {
                    for neighbor in mesh.cell_neighbours(cell).map_err(|e| e.to_string())? {
                        if neighbor.is_valid()
                            && neighbor.score_with_threshold(zero_threshold) == 0.
                        {
                            insert_cell(&mut temporary, neighbor)?;
                        }
                    }
                }
                for (_, cell) in temporary {
                    insert_cell(&mut broaden, cell)?;
                }
            }
        }
        let mut boundary: Vec<(usize, MeshEdge)> = Vec::new();
        if tests[2] {
            for (_, cell) in &broaden {
                for edge in cell.edges().map_err(|e| e.to_string())? {
                    let [a, b] = mesh.edge_cells(&edge).map_err(|e| e.to_string())?;
                    let other = if a.is_valid() && a.same(cell) { b } else { a };
                    let inside =
                        other
                            .address()
                            .map_err(|e| e.to_string())?
                            .is_some_and(|address| {
                                broaden
                                    .binary_search_by_key(&address, |(key, _)| *key)
                                    .is_ok()
                            });
                    if inside {
                        continue;
                    }
                    let Some(address) = edge.address().map_err(|e| e.to_string())? else {
                        continue;
                    };
                    let index = boundary.partition_point(|(key, _)| *key < address);
                    if boundary.get(index).is_none_or(|(key, _)| *key != address) {
                        if boundary.len() >= 256 {
                            return Err("Pathogen boundary set capacity256".into());
                        }
                        boundary.insert(index, (address, edge));
                    }
                }
            }
        }
        path.edges = boundary
            .into_iter()
            .take(120)
            .map(|(_, edge)| edge)
            .collect();
        Ok(())
    }
    fn populate(path: &mut NavPath, proxies: &[Proxy], best: usize) -> Result<(), String> {
        path.end = proxies[best].cell.clone();
        let mut reverse = Vec::new();
        let mut current = best;
        while let Some(parent) = proxies[current].parent {
            if reverse.len() >= 256 {
                return Err("Pathogen native path capacity256".into());
            }
            reverse.push(proxies[current].cell.clone());
            current = parent;
        }
        path.cells.clear();
        path.cells.push(path.start.clone());
        path.cells.extend(reverse.into_iter().rev());
        path.valid = true;
        Ok(())
    }
    /// SolveAStar82e62ef0 is a native best-first cost ordering, NOT an invented
    /// textbook accumulated-g algorithm. maxPathLength counts expanded cells.
    pub(crate) fn solve<N: ReckoningNumeric>(
        &self,
        mesh: &MeshStore,
        path: &mut NavPath,
        max_path_length: u32,
        mut zero_threshold: f32,
        allow_incomplete: bool,
        numeric: &mut N,
    ) -> Result<(), String>
    where
        N::Error: Debug,
    {
        let _lock = self
            .lock
            .lock()
            .map_err(|_| "Pathogen solver lock".to_string())?;
        path.cells.clear();
        path.valid = false;
        let Some((start_cost, start)) = path.start.cost_and_centroid() else {
            return Ok(());
        };
        let Some((_, end)) = path.end.cost_and_centroid() else {
            return Ok(());
        };
        if start_cost > zero_threshold {
            zero_threshold = start_cost + 0.01;
        }
        let distance = |position: [f32; 4], numeric: &mut N| {
            let delta: [f32; 4] = std::array::from_fn(|i| position[i] - end[i]);
            try_length(delta, numeric).map_err(|e| format!("Pathogen distance: {e:?}"))
        };
        let score = path.start.score_with_threshold(zero_threshold);
        let address = path
            .start
            .address()
            .map_err(|e| e.to_string())?
            .ok_or("Pathogen start unloaded")?;
        let mut proxies = vec![Proxy {
            cell: path.start.clone(),
            weight: distance(start, numeric)? + score,
            address,
            parent: None,
        }];
        let mut open = vec![0];
        let mut closed = Vec::new();
        let mut expanded = 0u32;
        while !open.is_empty() {
            let best = open.remove(0);
            insert(&mut closed, best, &proxies);
            expanded = expanded.wrapping_add(1);
            if proxies[best].cell.same(&path.end) || expanded >= max_path_length {
                if proxies[best].cell.same(&path.end) || allow_incomplete {
                    Self::populate(path, &proxies, best)?;
                }
                break;
            }
            for edge in proxies[best].cell.edges().map_err(|e| e.to_string())? {
                let [a, b] = mesh.edge_cells(&edge).map_err(|e| e.to_string())?;
                let cell = if a.same(&proxies[best].cell) { b } else { a };
                let Some((_, centroid)) = cell.cost_and_centroid() else {
                    continue;
                };
                if proxies.len() == 254 {
                    Self::populate(path, &proxies, best)?;
                    break;
                }
                // Raw82e63390 passes only-zero-cost=true (r9=1). Distance is
                // evaluated before the native positive-score rejection.
                let heuristic = distance(centroid, numeric)?;
                let cell_score = cell.score_with_threshold(zero_threshold);
                if cell_score > 0.0 {
                    continue;
                }
                let candidate = Proxy {
                    weight: heuristic + cell_score,
                    address: cell
                        .address()
                        .map_err(|e| e.to_string())?
                        .ok_or("Pathogen neighbor unloaded")?,
                    cell,
                    parent: Some(best),
                };
                let position = closed.partition_point(|&i| less(&proxies[i], &candidate));
                let existing = closed
                    .get(position)
                    .copied()
                    .filter(|&i| !less(&candidate, &proxies[i]));
                if let Some(existing) = existing {
                    // Raw82e619a0 and82e61a14 both call GetFromClosed. The
                    // second lookup on a miss is unchanged; preserve that flow.
                    if proxies[existing].weight >= proxies[best].weight {
                        open.retain(|&i| {
                            less(&proxies[i], &proxies[existing])
                                || less(&proxies[existing], &proxies[i])
                        });
                        insert(&mut closed, existing, &proxies);
                    }
                    continue;
                }
                if proxies.len() >= 256 {
                    return Err("Pathogen proxy capacity256".into());
                }
                let index = proxies.len();
                proxies.push(candidate);
                insert(&mut open, index, &proxies);
            }
        }
        Ok(())
    }
}
