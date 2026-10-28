//! Distance portion of Pathogen::NavRig. Arrays must come from the actual
//! volume/physics query owner; there is deliberately no fabricated clear default.
use super::{NavigationTargetMode, PedestrianNavigation};

/// InitializeRig82e52278 stores degrees in this exact circular order.
pub(crate) const RAY_ANGLES_DEGREES: [f32; 7] = [0., -11., -55., -125., 125., 55., 11.];
pub(crate) const FRONT_TEST_GROUP: [bool; 7] = [true, true, false, false, false, false, true];

pub(crate) struct NavRigTuning {
    pub(crate) distance: f32,
    pub(crate) cache_distance: f32,
    pub(crate) physics_caching: bool,
    pub(crate) path_validation: bool,
    pub(crate) topology_angle: f32,
    pub(crate) topology_distance: f32,
    pub(crate) batch_tests: bool,
}

/// Original rotation initialization uses XMVectorSinCos. Reuse its existing
/// polynomial implementation, not host sin/cos or equally spaced ray angles.
pub(crate) fn ray_sin_cos() -> [(f32, f32); 7] {
    use crate::reckoning_sin_cos::{HostReckoningSinCosNumeric, try_sin_cos};
    let mut result = [(0., 1.); 7];
    for index in 1..7 {
        result[index] = try_sin_cos(RAY_ANGLES_DEGREES[index] * 0.017453292_f32,
            &mut HostReckoningSinCosNumeric).expect("infallible host angle reduction");
    }
    result
}

/// TransformRig82e52114..224c direction portion. Forward uses full supplied
/// heading; remaining rays use the original unnormalized heading in the matrix
/// multiplies, then explicitly clear Y/W before NormalizeSafe.
pub(crate) fn transform_directions<N: crate::reckoning_numeric::ReckoningNumeric>(
    heading: [f32; 4], rotations: &[(f32, f32); 7], numeric: &mut N,
) -> Result<[[f32; 4]; 7], N::Error> {
    use crate::reckoning_numeric::try_normalize_safe;
    let mut result = [[0.; 4]; 7];
    result[0] = try_normalize_safe(heading, [0.; 4], numeric)?;
    for index in 1..7 {
        let (sin, cos) = rotations[index];
        let x = sin.mul_add(heading[2], 0.0_f32.mul_add(heading[1], cos.mul_add(heading[0], 0.)));
        let z = cos.mul_add(heading[2], 0.0_f32.mul_add(heading[1], (-sin).mul_add(heading[0], 0.)));
        result[index] = try_normalize_safe([x, 0., z, 0.], [0.; 4], numeric)?;
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RayDirection { CounterClockwise, Clockwise }
impl RayDirection {
    /// GetNavRigDistance82e51918..44 uses closest - direction with wrap.
    pub(crate) fn adjacent(self, closest: usize) -> usize {
        match self { Self::CounterClockwise => (closest + 1) % 7,
            Self::Clockwise => (closest + 6) % 7 }
    }
}

/// GetClosestNavRay82e4d8b8. Native hill-climb starts at forward ray0, checks
/// predecessor then successor, retains ties, and lazily caches dot products.
pub(crate) fn closest_ray<N: crate::reckoning_numeric::ReckoningNumeric>(
    directions: &[[f32; 4]; 7], requested: [f32; 4], numeric: &mut N,
) -> Result<(usize, RayDirection), N::Error> {
    let mut dots = [None; 7];
    let mut selected = 0;
    loop {
        let previous = selected;
        let left = (selected + 6) % 7;
        let right = (selected + 1) % 7;
        for ray in [selected, left, right] {
            if dots[ray].is_none() { dots[ray] = Some(numeric.dot3(requested, directions[ray])?); }
        }
        let dot = |ray: usize| dots[ray].expect("native evaluated neighborhood");
        if dot(left) > dot(selected) { selected = left; }
        if dot(right) > dot(selected) { selected = right; }
        if selected == previous {
            return Ok((selected, if dot(right) > dot(left) {
                RayDirection::CounterClockwise
            } else { RayDirection::Clockwise }));
        }
    }
}

pub(crate) struct NavRigDistances {
    pub(crate) volume: [f32; 7],
    pub(crate) physics: [f32; 7],
    pub(crate) jump: [f32; 2],
    pub(crate) do_jump_test: bool,
}
/// Result projection of LWPedestrianLineTestBatchEntry (native128 bytes).
/// `submitted` is mbTestSubmitted, not a made-up hit flag: a submitted miss
/// retains its initialized endpoint/distance and still participates in receive.
pub(crate) struct NavRigPhysicsResult {
    pub(crate) submitted: bool,
    pub(crate) test_type: u32,
    pub(crate) ray: usize,
    pub(crate) end: [f32; 4],
    pub(crate) hit_position: [f32; 4],
    pub(crate) hit_normal: [f32; 4],
    pub(crate) hit_distance: f32,
    pub(crate) surface_id: u32,
    pub(crate) hit_owner: u32,
}
pub(crate) struct NavRigProbeTuning {
    pub(crate) distance: f32,
    pub(crate) fatness: f32,
    pub(crate) y_clearance: f32,
    pub(crate) start_offset: f32,
    pub(crate) down_start_offset: f32,
    pub(crate) down_start_height: f32,
    pub(crate) down_height: f32,
}
pub(crate) struct NavRigPhysicsRequest {
    pub(crate) pedestrian_id: u32,
    pub(crate) ignore_volume_id: u32,
    pub(crate) start: [f32; 4],
    pub(crate) fatness: f32,
    pub(crate) result: NavRigPhysicsResult,
}
impl NavRigPhysicsRequest {
    fn new<N: crate::reckoning_numeric::ReckoningNumeric>(
        pedestrian_id: u32, ignore_volume_id: u32, ray: usize, test_type: u32,
        start: [f32; 4], end: [f32; 4], fatness: f32, numeric: &mut N,
    ) -> Result<Self, N::Error> {
        let delta: [f32; 4] = std::array::from_fn(|i| end[i] - start[i]);
        let length = crate::reckoning_numeric::try_length(delta, numeric)?;
        Ok(Self { pedestrian_id, ignore_volume_id, start, fatness,
            result: NavRigPhysicsResult { submitted: false, test_type, ray, end,
                hit_position: end, hit_normal: [0., 1., 0., 0.], hit_distance: length,
                surface_id: 0, hit_owner: 0 } })
    }
    /// ProcessGroundLineTestBatch82e72538..25c4 copies an actual batch hit
    /// and computes its distance from start; native does not use line fraction.
    /// Call only for GetResult=Some. A miss preserves initialized endpoint data.
    pub(crate) fn apply_ground_hit<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, position: [f32; 4], normal: [f32; 4], surface_id: u32,
        owner: u32, numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.result.hit_position = position;
        self.result.hit_normal = normal;
        self.result.surface_id = surface_id;
        self.result.hit_owner = owner;
        let delta: [f32; 4] = std::array::from_fn(|i| position[i] - self.start[i]);
        self.result.hit_distance = crate::reckoning_numeric::try_length(delta, numeric)?;
        Ok(())
    }
}
/// Retained transform/query state. Native ctor82e51870 runs in a cleared
/// MemStore block; zero clearance is intentional until queries publish results.
#[path = "pedestrian_nav_rig_mesh.rs"]
pub(crate) mod mesh;

pub(crate) struct NavRig {
    pub(crate) mesh_info: mesh::NavRigMeshInfo,
    pub(crate) position: [f32; 4],
    pub(crate) directions: [[f32; 4]; 7],
    pub(crate) distances: NavRigDistances,
    pub(crate) batch_range: std::ops::Range<usize>,
    pub(crate) reuse_physics: bool,
    cache_stale_counter: i32,
    front_test_group: bool,
}
impl NavRig {
    /// IsPointInNavRig82e527f0. Full XYZ length precedes the direction query;
    /// strict greater-than rejects points exactly on the clearance boundary.
    pub(crate) fn contains_point<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, point: [f32; 4], numeric: &mut N,
    ) -> Result<bool, String> where N::Error: std::fmt::Debug {
        let direction = std::array::from_fn(|i| point[i] - self.position[i]);
        let distance = crate::reckoning_numeric::try_length(direction, numeric)
            .map_err(|e| format!("navigation point distance: {e:?}"))?;
        let (clearance, _, _) = self.distances.heading_distance(
            self.position, &self.directions, direction, numeric)?;
        Ok(clearance > distance)
    }

    /// AddTestsToVolumeBatch82e79478: all seven rays, no cache/group gate,
    /// fixed10 distance. Fatness/clearance are the distinct volume cvars.
    pub(crate) fn add_volume_probes<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, enabled: bool, pedestrian_id: u32, ignore_volume_id: u32,
        position: [f32; 4], fatness: f32, y_clearance: f32, start_offset: f32,
        requests: &mut Vec<NavRigPhysicsRequest>, capacity: usize, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        if !enabled { return Ok(()); }
        let offset = [0., y_clearance + fatness, 0., 0.];
        let adjusted: [f32; 4] = std::array::from_fn(|i| position[i] + offset[i]);
        for ray in 0..7 {
            if requests.len() >= capacity { continue; }
            let direction = self.directions[ray];
            let start = std::array::from_fn(|i| direction[i].mul_add(start_offset, adjusted[i]));
            let end = std::array::from_fn(|i| direction[i].mul_add(10., adjusted[i]));
            requests.push(NavRigPhysicsRequest::new(pedestrian_id, ignore_volume_id,
                ray, 2, start, end, fatness, numeric)
                .map_err(|e| format!("navigation volume probe: {e:?}"))?);
        }
        Ok(())
    }
    /// Pedestrian::AddTestsToGroundBatch82e78818. Forward tests precede down
    /// tests, and the native group advances even if the shared batch is full.
    pub(crate) fn add_ground_probes<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, enabled: bool, pedestrian_id: u32, ignore_volume_id: u32,
        position: [f32; 4], tuning: &NavRigProbeTuning,
        requests: &mut Vec<NavRigPhysicsRequest>, capacity: usize, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        if !enabled || self.reuse_physics { return Ok(()); }
        let offset = [0., tuning.y_clearance + tuning.fatness, 0., 0.];
        let adjusted: [f32; 4] = std::array::from_fn(|i| position[i] + offset[i]);
        for down in [false, true] {
            for ray in 0..7 {
                if FRONT_TEST_GROUP[ray] != self.front_test_group || requests.len() >= capacity { continue; }
                let direction = self.directions[ray];
                let (start, end, fatness) = if down {
                    let start: [f32; 4] = std::array::from_fn(|i|
                        [0.0_f32, 1., 0., 0.][i].mul_add(tuning.down_start_height,
                            direction[i].mul_add(tuning.down_start_offset, adjusted[i])));
                    let end = std::array::from_fn(|i| start[i] - [0., 1., 0., 0.][i] * tuning.down_height);
                    (start, end, 0.)
                } else {
                    (std::array::from_fn(|i| direction[i].mul_add(tuning.start_offset, adjusted[i])),
                     std::array::from_fn(|i| direction[i].mul_add(tuning.distance, adjusted[i])), tuning.fatness)
                };
                requests.push(NavRigPhysicsRequest::new(pedestrian_id, ignore_volume_id,
                    ray, u32::from(down), start, end, fatness, numeric)
                    .map_err(|e| format!("navigation probe length: {e:?}"))?);
            }
        }
        self.front_test_group = !self.front_test_group;
        Ok(())
    }
    /// AdjustNavRigVerticalAngleToNavMesh82e58720, including the native
    /// endpoint-first pass and signed vertical-angle clamp82e589a0..8b28.
    pub(crate) fn orient_to_topology<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, position: [f32; 4], max_angle_degrees: f32, max_distance: f32,
        mesh: &crate::living_world_mesh_store::MeshStore, numeric: &mut N,
    ) -> Result<(), String>
    where N::Error: std::fmt::Debug,
    {
        use crate::reckoning_numeric::{try_normalize_safe, try_length, try_acos};
        let endpoints = self.topology_endpoints(position, max_distance, mesh, numeric)?;
        for (ray, endpoint) in endpoints.into_iter().enumerate() {
            let delta: [f32; 4] = std::array::from_fn(|i| endpoint[i] - position[i]);
            let direction = try_normalize_safe(delta, [0.; 4], numeric)
                .map_err(|e| format!("navigation slope normalize: {e:?}"))?;
            let horizontal = [direction[0], 0., direction[2], 0.];
            if !horizontal.iter().any(|v| v.abs() > f32::from_bits(0x3400_0000)) {
                continue;
            }
            let limit = max_angle_degrees * f32::from_bits(0x3c8e_fa35);
            let sign = if direction[1] >= 0. { 1.0_f32 } else { -1.0_f32 };
            let length = try_length(horizontal, numeric)
                .map_err(|e| format!("navigation slope length: {e:?}"))?;
            let angle = try_acos(length, numeric)
                .map_err(|e| format!("navigation slope angle: {e:?}"))?;
            if limit >= angle {
                self.directions[ray] = direction;
            } else {
                let (sin, cos) = crate::reckoning_sin_cos::try_sin_cos(limit,
                    &mut crate::reckoning_sin_cos::HostReckoningSinCosNumeric)
                    .expect("infallible host angle reduction");
                let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal(
                    length, numeric.reciprocal_estimate(length)
                        .map_err(|e| format!("navigation slope reciprocal: {e:?}"))?);
                let scale = inverse * cos;
                let vertical = sign * sin;
                self.directions[ray] = std::array::from_fn(|i|
                    horizontal[i].mul_add(scale, [0., 1., 0., 0.][i] * vertical));
            }
        }
        Ok(())
    }

    /// AdjustNavRigVerticalAngleToNavMesh82e58720..8968 samples all endpoints
    /// before changing any directions. Missing/degenerate cells preserve Y.
    pub(crate) fn topology_endpoints<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, position: [f32; 4], max_distance: f32,
        mesh: &crate::living_world_mesh_store::MeshStore, numeric: &mut N,
    ) -> Result<[[f32; 4]; 7], String>
    where N::Error: std::fmt::Debug,
    {
        let mut endpoints = [[0.; 4]; 7];
        for (ray, endpoint) in endpoints.iter_mut().enumerate() {
            let distance = self.distances.distance(ray)?;
            let distance = if max_distance - distance >= 0. { distance } else { max_distance };
            *endpoint = std::array::from_fn(|i| self.directions[ray][i].mul_add(distance, position[i]));
            let cell = mesh.try_get_cell_with_tolerance(
                crate::skeleton_root_transforms::SkeletonRootVector::known(*endpoint), f32::MAX)
                .map_err(|e| format!("navigation topology: {e:?}"))?;
            if let Some(height) = cell.height_at(*endpoint, numeric)? { endpoint[1] = height; }
        }
        Ok(endpoints)
    }

    pub(crate) fn new() -> Self {
        Self {
            mesh_info: mesh::NavRigMeshInfo::new(),
            position: [0.; 4], directions: [[0.; 4]; 7],
            distances: NavRigDistances { volume: [0.; 7], physics: [0.; 7],
                jump: [0.; 2], do_jump_test: false },
            batch_range: 0..0, reuse_physics: false, cache_stale_counter: 0,
            front_test_group: true,
        }
    }
    /// TransformRig82e51f38 resets volume clearance but retains physics results.
    /// This function does NOT publish mPosition: the native topology/query flow
    /// owns that field. Tuning is supplied from the actual LW cvar owner.
    pub(crate) fn transform<N: crate::reckoning_numeric::ReckoningNumeric>(
        &mut self, position: [f32; 4], heading: [f32; 4],
        range: f32, physics_caching: bool, cache_distance: f32,
        rotations: &[(f32, f32); 7], numeric: &mut N,
    ) -> Result<(), N::Error> {
        self.distances.volume.fill(range);
        self.reuse_physics = false;
        if physics_caching {
            self.reuse_physics = numeric.dot3(self.directions[0], heading)? > 0.98_f32;
            self.cache_stale_counter = self.cache_stale_counter.wrapping_add(1);
            if self.reuse_physics {
                let delta = std::array::from_fn(|i| position[i] - self.position[i]);
                let squared = numeric.dot3(delta, delta)?;
                self.reuse_physics = squared - cache_distance * cache_distance < 0.;
            }
            if self.cache_stale_counter > 40 {
                self.cache_stale_counter = 0;
                self.reuse_physics = false;
            }
        }
        self.directions = transform_directions(heading, rotations, numeric)?;
        self.batch_range = 0..0;
        Ok(())
    }
}
pub(crate) struct JumpTuning {
    pub(crate) distance: f32,
    pub(crate) height_offset: f32,
    pub(crate) min_up: f32,
    pub(crate) max_up: f32,
    pub(crate) min_down: f32,
    pub(crate) max_down: f32,
    pub(crate) down_ledge_difference: f32,
}

/// Original tNavRigTest two-line request, retaining owning ray separately from
/// shared batch indices. Results are calculated by the existing line math.
pub(crate) struct NavRigTest {
    pub(crate) starts: [[f32; 4]; 2],
    pub(crate) headings: [[f32; 4]; 2],
    pub(crate) magnitudes: [f32; 2],
    pub(crate) ray: usize,
}
impl NavRigTest {
    /// NavRigValidator::ProcessBatch82e4dbf8 shares the same parameter math.
    pub(crate) fn process<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, numeric: &mut N,
    ) -> Result<crate::living_world_mesh_store::pathogen::LineLineResult, N::Error> {
        crate::living_world_mesh_store::pathogen::line_line_result(
            self.starts[0], self.headings[0], self.starts[1], self.headings[1], numeric)
    }
}

/// MeshMan's single NavRigValidator82ce7608. Constructor has no allocation;
/// the frame's original batch initialization must supply its capacity.
#[derive(Default)]
pub(crate) struct NavRigBatch {
    capacity: usize,
    entries: Option<Vec<(u32, NavRigTest, Option<crate::living_world_mesh_store::pathogen::LineLineResult>)>>,
}
impl NavRigBatch {
    /// MeshMan::FrameBegin82ce6c28 tail-calls folded824a6c20, which writes
    /// zero only to validator+8 (mNumBatches). Keep allocated capacity.
    pub(crate) fn frame_begin(&mut self) {
        if let Some(entries) = &mut self.entries {
            entries.clear();
        }
    }
    /// DeInitializeBatch82e4dd60 releases storage and clears count/capacity.
    pub(crate) fn deinitialize(&mut self) {
        self.entries = None;
        self.capacity = 0;
    }
    pub(crate) fn capacity(&self) -> usize { self.capacity }
    pub(crate) fn count(&self) -> usize { self.entries.as_ref().map_or(0, Vec::len) }
    pub(crate) fn is_unallocated(&self) -> bool { self.entries.is_none() }
    pub(crate) fn initialize(&mut self, capacity: usize) -> Result<(), String> {
        if self.entries.is_some() { return Err("navigation batch already allocated".into()); }
        self.capacity = capacity;
        self.entries = Some(Vec::with_capacity(capacity));
        Ok(())
    }
    /// AddNavRigTest82e4dac0 copies inputs in submission order; no eager test.
    pub(crate) fn add(&mut self, owner: u32, test: NavRigTest) -> Result<usize, String> {
        let entries = self.entries.as_mut().ok_or("navigation batch unallocated")?;
        if entries.len() >= self.capacity { return Err("navigation batch capacity".into()); }
        let index = entries.len();
        entries.push((owner, test, None));
        Ok(index)
    }
    pub(crate) fn process<N: crate::reckoning_numeric::ReckoningNumeric>(&mut self, numeric: &mut N) -> Result<(), String>
    where N::Error: std::fmt::Debug {
        if let Some(entries) = &mut self.entries {
            for (_, test, result) in entries {
                *result = Some(test.process(numeric).map_err(|e| format!("navigation batch: {e:?}"))?);
            }
        }
        Ok(())
    }
    pub(crate) fn apply(&self, range: std::ops::Range<usize>, distances: &mut NavRigDistances) -> Result<(), String> {
        if range.start >= range.end { return Ok(()); }
        let entries = self.entries.as_ref().ok_or("navigation batch unallocated")?
            .get(range).ok_or("navigation batch result range")?;
        for (_, test, result) in entries {
            distances.apply_batch_result(test, result.as_ref().ok_or("navigation batch not processed")?)?;
        }
        Ok(())
    }
}
impl NavRigDistances {
    /// LWPedestrianExecutionMan::ReceiveGroundLineTestBatch82e72850.
    pub(crate) fn receive_ground(&mut self, result: &NavRigPhysicsResult) -> Result<(), String> {
        if !result.submitted { return Ok(()); }
        match result.test_type {
            0 => self.set_physics(result.ray, result.hit_distance),
            1 => {
                let changed = result.end.iter().zip(result.hit_position).any(|(&end, hit)|
                    (end - hit).abs() > f32::from_bits(0x3400_0000));
                //82e728f0..2910 extracts CR6.EQ: vector comparison ALL FALSE,
                // not ANY TRUE. A probe reaching its end without support
                // blocks this direction, as does the authored water surface.
                if result.surface_id & 0x0f80 == 0x0300 || !changed {
                    self.set_physics(result.ray, 0.)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// ReceiveVolumeNavRigLineTestBatch82e72a60 only shortens volume clearance.
    pub(crate) fn receive_volume(&mut self, result: &NavRigPhysicsResult) -> Result<(), String> {
        if result.submitted { self.set_volume(result.ray, result.hit_distance)?; }
        Ok(())
    }
    /// UpdateNavRigWithBatchResults82e52560 applies completed entries in their
    /// original order. Four fsel membership checks use inclusive segment ends;
    /// parallel results retain current distance, while out-of-segment entries
    /// use the original1000-unit candidate before the closest-distance combine.
    pub(crate) fn apply_batch_result(&mut self, test: &NavRigTest,
        result: &crate::living_world_mesh_store::pathogen::LineLineResult,
    ) -> Result<(), String> {
        let [first, second] = result.distances;
        let flag = |value: f32| if value >= 0. { 1.0_f32 } else { 0.0_f32 };
        let inside = ((flag(second) + flag(first)) + flag(test.magnitudes[0] - first))
            + flag(test.magnitudes[1] - second);
        let candidate = if 3.5 - inside >= 0. { 1000. } else { first };
        let current = self.distance(test.ray)?;
        let nearest = if current - candidate >= 0. { candidate } else { current };
        let value = if result.parallel_score >= 0. { nearest } else { current };
        self.set_volume(test.ray, value)
    }

    /// GetNavRigDistance82e518d0: intersect the requested heading with the
    /// segment between neighboring clearance endpoints, not a dot-distance
    /// interpolation. The original ignores Line_Line_Test's validity flag.
    pub(crate) fn heading_distance<N: crate::reckoning_numeric::ReckoningNumeric>(
        &self, position: [f32; 4], directions: &[[f32; 4]; 7],
        heading: [f32; 4], numeric: &mut N,
    ) -> Result<(f32, usize, RayDirection), String>
    where N::Error: std::fmt::Debug {
        use crate::reckoning_numeric::try_normalize_safe;
        let (closest, side) = closest_ray(directions, heading, numeric)
            .map_err(|e| format!("navigation closest ray: {e:?}"))?;
        let adjacent = side.adjacent(closest);
        let first_distance = self.distance(closest)?;
        let second_distance = self.distance(adjacent)?;
        let mut first: [f32; 4] = std::array::from_fn(|i|
            directions[closest][i].mul_add(first_distance, position[i]));
        let second: [f32; 4] = std::array::from_fn(|i|
            directions[adjacent][i].mul_add(second_distance, position[i]));
        // NormalizeXZSafe82de2290 explicitly clears both Y and W.
        let edge = try_normalize_safe([second[0]-first[0], 0., second[2]-first[2], 0.], [0.;4], numeric)
            .map_err(|e| format!("navigation edge normalize: {e:?}"))?;
        first[1] = position[1];
        let requested = try_normalize_safe(heading, [0.;4], numeric)
            .map_err(|e| format!("navigation heading normalize: {e:?}"))?;
        let [_, distance] = crate::living_world_mesh_store::pathogen::line_line_parameters(
            first, edge, position, requested, numeric)
            .map_err(|e| format!("navigation heading intersection: {e:?}"))?;
        Ok((if distance > 0. { distance } else { 0. }, closest, side))
    }

    /// GetDistance82e4d408: fsubs/fsel, including native NaN/signed-zero order.
    pub(crate) fn distance(&self, ray: usize) -> Result<f32, String> {
        let volume = *self.volume.get(ray).ok_or("navigation ray outside 0..7")?;
        let physics = self.physics[ray];
        Ok(if volume - physics >= 0. { physics } else { volume })
    }
    /// GetMaxRange82e4d610; starts at zero and ignores mPathDistances.
    pub(crate) fn max_range(&self) -> f32 {
        let mut maximum = 0.;
        for ray in 0..7 {
            let distance = self.distance(ray).expect("fixed ray bounds");
            maximum = if maximum - distance >= 0. { maximum } else { distance };
        }
        maximum
    }
    pub(crate) fn set_physics(&mut self, ray: usize, value: f32) -> Result<(), String> {
        *self.physics.get_mut(ray).ok_or("navigation ray outside 0..7")? = value;
        Ok(())
    }
    /// SetVolumeDistance82e4d7d0 retains the closest submitted volume.
    pub(crate) fn set_volume(&mut self, ray: usize, value: f32) -> Result<(), String> {
        let current = self.volume.get_mut(ray).ok_or("navigation ray outside 0..7")?;
        if value < *current { *current = value; }
        Ok(())
    }
    /// IntentGen::GetNavSpeed82e6b950. No artificial minimum walking speed.
    pub(crate) fn speed(&self, navigation: &PedestrianNavigation, razor: f32) -> f32 {
        if self.max_range() > razor || navigation.target_mode == NavigationTargetMode::ExplicitNoSeekNoAstar {
            navigation.speed()
        } else { 0. }
    }
    /// HasToJump82e4d480 writes height even on a failed enabled test; ledge
    /// distance changes only on success. Keep caller-owned output semantics.
    pub(crate) fn has_to_jump(&self, tuning: &JumpTuning, height: &mut f32, ledge: &mut f32) -> bool {
        if !self.do_jump_test { return false; }
        *height = tuning.height_offset - self.jump[1];
        if !(self.jump[0] > tuning.distance) { return false; }
        if *height < 0. {
            let downward = -*height;
            if downward > tuning.min_down && downward < tuning.max_down {
                *ledge = tuning.distance - tuning.down_ledge_difference;
                return true;
            }
        } else if *height > tuning.min_up && *height < tuning.max_up {
            *ledge = self.distance(0).expect("fixed ray bounds");
            return true;
        }
        false
    }
}
