//! Ground/default-world portion of LWPedestrianExecutionMan's double buffer.
//! Entity and mesh-cell facts are borrowed from their existing owners.
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    living_world_mesh_store::MeshCell,
    reckoning_numeric::{HostReckoningNumeric, try_length},
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineQueries, group::WorldLineGroup,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::{
        SimulationDmoAssemblyHandle, SimulationDmoAssemblyPool,
    },
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};

pub(crate) struct GroundDmoTuning {
    pub(crate) distance_up_default: f32,
    pub(crate) distance_up_knock_down: f32,
    pub(crate) distance_down: f32,
    pub(crate) jump_forward_distance: f32,
    pub(crate) jump_forward_fatness: f32,
    pub(crate) jump_forward_height: f32,
    pub(crate) test_start_offset: f32,
    pub(crate) jump_height_distance: f32,
    pub(crate) jump_height_height: f32,
    pub(crate) jump_height_lead: f32,
    pub(crate) jump_height_fatness: f32,
    pub(crate) taze_line_start_offset: f32,
}

/// Only supplied after state bit3244:80, TazerState_Cyc, valid target ID and
/// ConvertTargetableTypeToITargetable all succeed (82e791fc..9274).
pub(crate) struct TazeProbeFacts {
    pub(crate) animation_origin: [f32; 4],
    pub(crate) target_centre_of_mass: [f32; 4],
}
pub(crate) struct GroundDmoProbeFacts<'a> {
    pub(crate) pedestrian_id: u32,
    pub(crate) core_position: [f32; 4],
    pub(crate) position: [f32; 4],
    pub(crate) facing: [f32; 4],
    pub(crate) knocked_down: bool,
    pub(crate) ignore_nav_mesh: bool,
    /// NavRig::mMeshInfo, not a new lookup or MeshNavigation's target cell.
    pub(crate) mesh_cell: &'a MeshCell,
    /// IsAbleToJump AND pedestrian byte6724 bit80.
    pub(crate) jump_enabled: bool,
    pub(crate) taze: Option<TazeProbeFacts>,
}
#[derive(Clone, Copy)]
pub(crate) enum GroundDmoTest { Jump(usize), Positioning, Taze }

pub(crate) struct GroundDmoRequest {
    pub(crate) pedestrian_id: u32,
    pub(crate) test: GroundDmoTest,
    pub(crate) start: [f32; 4],
    pub(crate) end: [f32; 4],
    pub(crate) fatness: f32,
    pub(crate) submitted: bool,
    pub(crate) hit_position: [f32; 4],
    pub(crate) hit_normal: [f32; 4],
    pub(crate) hit_distance: f32,
    pub(crate) surface_id: u32,
    pub(crate) hit_owner: Option<SimulationDmoAssemblyHandle>,
}
impl GroundDmoRequest {
    fn new(id: u32, test: GroundDmoTest, start: [f32; 4], end: [f32; 4],
        fatness: f32, initial_distance: Option<f32>,
    ) -> Result<Self, String> {
        let distance = match initial_distance {
            Some(value) => value,
            None => try_length(std::array::from_fn(|i| end[i] - start[i]), &mut HostReckoningNumeric)
                .map_err(|e| format!("pedestrian default probe length: {e:?}"))?,
        };
        Ok(Self { pedestrian_id: id, test, start, end, fatness, submitted: false,
            hit_position: end, hit_normal: [0., 1., 0., 0.], hit_distance: distance,
            surface_id: 0, hit_owner: None })
    }
    /// ReceiveGroundDMOLineTestBatch82e72e74..eb4: Vector3 epsilon compare.
    pub(crate) fn taze_obstructed(&self) -> bool {
        (0..3).any(|i| (self.hit_position[i] - self.end[i]).abs() > f32::from_bits(0x3400_0000))
    }
}

struct GroundDmoBuffer {
    group: WorldLineGroup,
    requests: Vec<GroundDmoRequest>,
    accepting: bool,
}
pub(crate) struct PedestrianGroundDmoBatches {
    buffers: [GroundDmoBuffer; 2],
    send: usize,
}
impl PedestrianGroundDmoBatches {
    pub(crate) fn retire_rig(&mut self, id: u32) {
        for buffer in &mut self.buffers {
            for request in &mut buffer.requests {
                if request.pedestrian_id == id && matches!(request.test, GroundDmoTest::Jump(_)) {
                    request.submitted = false;
                }
            }
        }
    }
    pub(crate) fn new() -> Self {
        Self { buffers: std::array::from_fn(|_| GroundDmoBuffer {
            group: WorldLineGroup::new(), requests: Vec::with_capacity(80), accepting: false,
        }), send: 1 }
    }
    /// Init82e9dda4..de70: ONE BF_Default(7) lease on each side.
    pub(crate) fn initialize(&mut self, queries: &mut SimulationWorldLineQueries,
        dmos: &SimulationDmoAssemblyPool,
    ) -> Result<(), String> {
        for buffer in &mut self.buffers {
            buffer.group.acquire(queries, dmos, "LWPedestrianExecutionMan::Init", 7, 1)
                .map_err(|e| format!("pedestrian default acquire: {e:?}"))?;
        }
        Ok(())
    }
    pub(crate) fn begin(&mut self, queries: &mut SimulationWorldLineQueries) -> Result<(), String> {
        let buffer = &mut self.buffers[self.send];
        buffer.requests.clear();
        buffer.accepting = true;
        buffer.group.begin(queries)
    }
    /// AddTestsToGroundDMOBatch82e789c8. Caller retains the selected start
    /// height in Pedestrian::mPositioningTestStartHeight even at capacity.
    pub(crate) fn positioning_start_height(facts: &GroundDmoProbeFacts<'_>, tuning: &GroundDmoTuning) -> f32 {
        if facts.knocked_down { tuning.distance_up_knock_down } else { tuning.distance_up_default }
    }
    /// Create82e72b08 appends one entity's requests, then submits in order.
    /// A rejected AddTest stops subsequent entity visits.
    pub(crate) fn has_taze_capacity(&self,jump_enabled:bool)->bool {
        let buffer=&self.buffers[self.send];
        buffer.accepting && buffer.requests.len()+1+if jump_enabled {2} else {0}<80
    }
    pub(crate) fn add_entity(&mut self, facts: &GroundDmoProbeFacts<'_>, tuning: &GroundDmoTuning,
        queries: &mut SimulationWorldLineQueries, ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool, numeric: &mut HostCollisionLineNumeric,
    ) -> Result<bool, String> {
        let buffer = &mut self.buffers[self.send];
        if !buffer.accepting { return Ok(false); }
        let begin = buffer.requests.len();
        let height = Self::positioning_start_height(facts, tuning);
        let mut base = facts.core_position;
        if !facts.ignore_nav_mesh {
            if let Some((_, centroid)) = facts.mesh_cell.cost_and_centroid() {
                if (centroid[1] - facts.position[1]).abs() < tuning.distance_down + height {
                    if let Some(y) = facts.mesh_cell.height_at(base, &mut HostReckoningNumeric)? { base[1] = y; }
                }
            }
        }
        let up = [0.0_f32, 1., 0., 0.];
        let mut append = |test, start, end, fatness, distance| -> Result<(), String> {
            if buffer.requests.len() < 80 {
                buffer.requests.push(GroundDmoRequest::new(facts.pedestrian_id, test, start, end, fatness, distance)?);
            }
            Ok(())
        };
        append(GroundDmoTest::Positioning,
            std::array::from_fn(|i| up[i].mul_add(height, base[i])),
            std::array::from_fn(|i| base[i] + up[i] * -tuning.distance_down), 0., None)?;
        if facts.jump_enabled {
            let lifted: [f32; 4] = std::array::from_fn(|i| up[i].mul_add(
                tuning.jump_forward_fatness + tuning.jump_forward_height, facts.position[i]));
            let start: [f32; 4] = std::array::from_fn(|i| facts.facing[i].mul_add(tuning.test_start_offset, lifted[i]));
            append(GroundDmoTest::Jump(0), start,
                std::array::from_fn(|i| facts.facing[i].mul_add(tuning.jump_forward_distance, start[i])),
                tuning.jump_forward_fatness, Some(tuning.jump_forward_distance))?;
            let lifted: [f32; 4] = std::array::from_fn(|i| up[i].mul_add(tuning.jump_height_height, facts.position[i]));
            let start: [f32; 4] = std::array::from_fn(|i| facts.facing[i].mul_add(tuning.jump_height_lead, lifted[i]));
            append(GroundDmoTest::Jump(1), start,
                std::array::from_fn(|i| start[i] - (up[i] * tuning.jump_height_height) * 2.),
                tuning.jump_height_fatness, Some(tuning.jump_height_distance))?;
        }
        if let Some(taze) = &facts.taze {
            append(GroundDmoTest::Taze,
                std::array::from_fn(|i| facts.facing[i].mul_add(tuning.taze_line_start_offset, taze.animation_origin[i])),
                taze.target_centre_of_mass, 0., None)?;
        }
        for request in &mut buffer.requests[begin..] {
            if !buffer.accepting { break; }
            let accepted = buffer.group.add(queries, CollisionLineInput {
                start: SkeletonRootVector::known(request.start), end: SkeletonRootVector::known(request.end),
                fatness: request.fatness,
            }, ground, dmos, numeric)?;
            request.submitted = accepted;
            buffer.accepting = accepted;
        }
        Ok(buffer.accepting)
    }
    pub(crate) fn end(&mut self, queries: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool, dmos: &SimulationDmoAssemblyPool, numeric: &mut HostCollisionLineNumeric,
    ) -> Result<(), String> { self.buffers[self.send].group.end(queries, ground, dmos, numeric) }

    /// Process82e72c50: misses preserve initialized endpoint/normal/distance.
    /// Receive82e72d70 consumes submitted misses too, including positioning.
    pub(crate) fn swap_buffers(&mut self) {
        self.send = 1 - self.send;
    }
    pub(crate) fn process_received(&mut self, queries: &mut SimulationWorldLineQueries)
        -> Result<&[GroundDmoRequest], String> {
        let receive = &mut self.buffers[1 - self.send];
        receive.group.sync(queries).map_err(|e| format!("pedestrian default sync: {e:?}"))?;
        for (index, request) in receive.requests.iter_mut().enumerate() {
            if let Some(hit) = receive.group.result(queries, index as u32)
                .map_err(|e| format!("pedestrian default result: {e:?}"))? {
                request.hit_position = hit.position.known_lanes().ok_or("pedestrian default hit position unknown")?;
                request.hit_normal = hit.normal.known_lanes().ok_or("pedestrian default hit normal unknown")?;
                request.surface_id = u32::from(hit.surface_id);
                request.hit_owner = hit.owner;
                request.hit_distance = try_length(std::array::from_fn(|i| request.hit_position[i] - request.start[i]),
                    &mut HostReckoningNumeric).map_err(|e| format!("pedestrian default hit length: {e:?}"))?;
            }
        }
        Ok(&receive.requests)
    }
    pub(crate) fn release(&self, queries: &mut SimulationWorldLineQueries) -> Result<(), String> {
        for buffer in &self.buffers {
            buffer.group.release(queries).map_err(|e| format!("pedestrian default release: {e:?}"))?;
        }
        Ok(())
    }
}
