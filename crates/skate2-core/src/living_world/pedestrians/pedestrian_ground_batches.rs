//! Retained ground portion of LWPedestrianExecutionMan's native double buffer.
//! Entity roster and pre/current-frame scheduling remain with the execution owner.
use crate::{
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_navigation::rig::{NavRigPhysicsRequest, NavRigProbeTuning},
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineQueries, group::WorldLineGroup,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
struct GroundBuffer {
    group: WorldLineGroup,
    requests: Vec<NavRigPhysicsRequest>,
    accepting: bool,
}
pub(crate) struct PedestrianGroundBatches {
    buffers: [GroundBuffer; 2],
    send: usize,
}
impl PedestrianGroundBatches {
    pub(crate) fn retire_rig(&mut self, id: u32) {
        for buffer in &mut self.buffers {
            for request in &mut buffer.requests {
                if request.pedestrian_id == id { request.result.submitted = false; }
            }
        }
    }
    /// DoubleBuffer ctor82e7c1e0: send B, receive A, zero test counts.
    pub(crate) fn new() -> Self {
        Self { buffers: std::array::from_fn(|_| GroundBuffer {
            group: WorldLineGroup::new(), requests: Vec::with_capacity(80), accepting: false,
        }), send: 1 }
    }
    /// Init82e9dc50 acquires five BF_Ground batches per side once.
    pub(crate) fn initialize(&mut self, queries: &mut SimulationWorldLineQueries,
        dmos: &SimulationDmoAssemblyPool,
    ) -> Result<(), String> {
        for buffer in &mut self.buffers {
            buffer.group.acquire(queries, dmos, "LWPedestrianExecutionMan::Init", 1, 5)
                .map_err(|e| format!("pedestrian ground acquire: {e:?}"))?;
        }
        Ok(())
    }
    pub(crate) fn begin(&mut self, queries: &mut SimulationWorldLineQueries) -> Result<(), String> {
        let buffer = &mut self.buffers[self.send];
        buffer.requests.clear();
        buffer.accepting = true;
        buffer.group.begin(queries)
    }
    /// CreateGroundLineTestBatch82e72320 builds and submits one entity at a
    /// time; stop visiting subsequent entities after a rejected AddTest.
    pub(crate) fn add_entity(&mut self, entity: &mut PedestrianConstruction,
        enabled: bool, ignore_volume: u32, tuning: &NavRigProbeTuning,
        queries: &mut SimulationWorldLineQueries, ground: &StaticGroundPool,
        dmos: &SimulationDmoAssemblyPool, numeric: &mut HostCollisionLineNumeric,
    ) -> Result<bool, String> {
        let buffer = &mut self.buffers[self.send];
        if !buffer.accepting { return Ok(false); }
        let begin = buffer.requests.len();
        entity.add_ground_probes(enabled, ignore_volume, tuning, &mut buffer.requests,
            80, &mut HostReckoningNumeric)?;
        for request in &mut buffer.requests[begin..] {
            if !buffer.accepting { break; }
            let accepted = buffer.group.add(queries, CollisionLineInput {
                start: SkeletonRootVector::known(request.start),
                end: SkeletonRootVector::known(request.result.end), fatness: request.fatness,
            }, ground, dmos, numeric)?;
            request.result.submitted = accepted;
            buffer.accepting = accepted;
        }
        Ok(buffer.accepting)
    }
    pub(crate) fn end(&mut self, queries: &mut SimulationWorldLineQueries,
        ground: &StaticGroundPool, dmos: &SimulationDmoAssemblyPool,
        numeric: &mut HostCollisionLineNumeric,
    ) -> Result<(), String> {
        self.buffers[self.send].group.end(queries, ground, dmos, numeric)
    }
    /// UpdateCurrentFrame82e96ef8 swaps before Process/Receive. Caller then
    /// distributes these retained requests to the existing pedestrian roster.
    pub(crate) fn swap_buffers(&mut self) {
        self.send = 1 - self.send;
    }
    /// Called only after all three query owners have swapped (82e96f20..48).
    pub(crate) fn process_received(&mut self, queries: &mut SimulationWorldLineQueries)
        -> Result<&[NavRigPhysicsRequest], String> {
        let receive = &mut self.buffers[1 - self.send];
        receive.group.sync(queries).map_err(|e| format!("pedestrian ground sync: {e:?}"))?;
        for (index, request) in receive.requests.iter_mut().enumerate() {
            if let Some(hit) = receive.group.result(queries, index as u32)
                .map_err(|e| format!("pedestrian ground result: {e:?}"))? {
                let position = hit.position.known_lanes().ok_or("pedestrian ground hit position unknown")?;
                let normal = hit.normal.known_lanes().ok_or("pedestrian ground hit normal unknown")?;
                if hit.owner.is_some() { return Err("BF_Ground unexpectedly returned dynamic owner".into()); }
                request.apply_ground_hit(position, normal, u32::from(hit.surface_id), 0,
                    &mut HostReckoningNumeric).map_err(|e| format!("pedestrian ground distance: {e:?}"))?;
            }
        }
        Ok(&receive.requests)
    }
    pub(crate) fn release(&self, queries: &mut SimulationWorldLineQueries) -> Result<(), String> {
        for buffer in &self.buffers {
            buffer.group.release(queries).map_err(|e| format!("pedestrian ground release: {e:?}"))?;
        }
        Ok(())
    }
}
