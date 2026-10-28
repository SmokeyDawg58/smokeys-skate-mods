//! Query-receive phase of LWPedestrianExecutionMan82e96ef8. Borrows the sole
//! EntityMan40; buffers never own or recreate pedestrian entities.
use crate::{
    living_world_grind_services::entities::EntityMan,
    living_world_mesh_store::MeshStore,
    living_world_pedestrian_entity::PedestrianConstruction,
    pedestrian_ground_batches::PedestrianGroundBatches,
    pedestrian_ground_dmo_batches::{GroundDmoTest, PedestrianGroundDmoBatches},
    pedestrian_volume_batches::PedestrianVolumeBatches,
    simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineQueries,
};

pub(crate) struct PedestrianExecutionQueries {
    initialization_started: bool,
    initialized: bool,
    pub(crate) ground: PedestrianGroundBatches,
    pub(crate) volume: PedestrianVolumeBatches,
    pub(crate) ground_dmo: PedestrianGroundDmoBatches,
}
impl PedestrianExecutionQueries {
    /// Host lifetime boundary for native raw NavRig pointers. The destructor
    /// frees that allocation at82e94380; no later result may target that dead
    /// allocation or a replacement entity reusing its pool slot. Retain every
    /// batch index, since outstanding native query results are index-addressed.
    pub(crate) fn retire_rig(&mut self, id: u32) {
        self.ground.retire_rig(id);
        self.volume.retire_rig(id);
        self.ground_dmo.retire_rig(id);
    }
    pub(crate) fn new() -> Self {
        Self { initialization_started: false, initialized: false,
            ground: PedestrianGroundBatches::new(),
            volume: PedestrianVolumeBatches::new(),
            ground_dmo: PedestrianGroundDmoBatches::new() }
    }

    /// LWPedestrianExecutionMan::Init82e9dc50: ground A/B leases precede
    /// default A/B leases. Volume buffers already retain their constructor
    /// capacity80 and null mask. A failed prefix must not reacquire leases.
    pub(crate) fn initialize(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        dmos: &crate::simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    ) -> Result<(), String> {
        if self.initialization_started {
            return Err("pedestrian execution queries initialization already attempted".into());
        }
        self.initialization_started = true;
        self.ground.initialize(queries, dmos)?;
        self.ground_dmo.initialize(queries, dmos)?;
        self.initialized = true;
        Ok(())
    }

    /// All three swaps precede any processing. Native ground/volume/jump
    /// requests retain NavRig pointers, so their owner must survive receive.
    /// Only positioning/taze requests perform nullable entity-ID lookups.
    pub(crate) fn receive_current_frame(
        &mut self,
        queries: &mut SimulationWorldLineQueries,
        pedestrians: &mut EntityMan<40, PedestrianConstruction>,
        pending_culls: &mut [PedestrianConstruction],
        mesh: &MeshStore,
        y_tolerance: f32,
    ) -> Result<(), String> {
        if !self.initialized {
            return Err("pedestrian execution queries not initialized".into());
        }
        self.ground.swap_buffers();
        self.volume.swap_buffers();
        self.ground_dmo.swap_buffers();
        for request in self.ground.process_received(queries)? {
            if !request.result.submitted { continue; }
            let entity = pedestrians.get_mut(request.pedestrian_id)
                .or_else(|| pending_culls.iter_mut().find(|p| p.id == request.pedestrian_id))
                .ok_or("pedestrian ground query lost its retained entity")?;
            entity.nav_rig.as_mut().ok_or("pedestrian ground query lost its rig")?
                .distances.receive_ground(&request.result)?;
        }
        for request in self.volume.process_received()? {
            if !request.result.submitted { continue; }
            let entity = pedestrians.get_mut(request.pedestrian_id)
                .or_else(|| pending_culls.iter_mut().find(|p| p.id == request.pedestrian_id))
                .ok_or("pedestrian volume query lost its retained entity")?;
            entity.nav_rig.as_mut().ok_or("pedestrian volume query lost its rig")?
                .distances.receive_volume(&request.result)?;
        }
        for request in self.ground_dmo.process_received(queries)? {
            if !request.submitted { continue; }
            // Ground/volume/jump hooks retain NavRig pointers, including after
            // RequestCull8271a5e8 detaches their still-live owner into mCullMap.
            // Positioning/taze instead do a nullable active EntityMan lookup.
            let entity = pedestrians.get_mut(request.pedestrian_id).or_else(|| {
                if matches!(request.test, GroundDmoTest::Jump(_)) {
                    pending_culls.iter_mut().find(|p| p.id == request.pedestrian_id)
                } else { None }
            });
            if let Some(entity) = entity {
                entity.receive_ground_dmo_result(request, mesh, y_tolerance)?;
            } else if matches!(request.test, GroundDmoTest::Jump(_)) {
                return Err("pedestrian jump query lost its retained entity".into());
            }
        }
        Ok(())
    }
}
