//! Borrowed camera query access to the already-held selected world.
use super::super::{
    SimulationAssemblyConstraintSession, grind_world::PhysicalGrindServices,
    world_line_query::SimulationWorldLineQueries,
};
use crate::{
    collision_line_numeric::HostCollisionLineNumeric,
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::AssemblyPartMaterial;

/// These references do not allocate a query pool or acquire a world lock.
/// Missing services are checked only at a native query call, not on construction.
pub(crate) struct CameraLineServices<'a> {
    pub(crate) queries: &'a mut Option<SimulationWorldLineQueries>,
    pub(crate) dmos: Option<&'a SimulationDmoAssemblyPool>,
    pub(crate) ground: &'a StaticGroundPool,
    pub(crate) numeric: &'a mut HostCollisionLineNumeric,
    pub(crate) overlay: &'a crate::debug_overlay::DebugOverlaySlot,
    /// `GrindData::BoxTest` for the camera's `GrindDetector`.
    pub(crate) grinds: &'a PhysicalGrindServices,
}

impl<C> SimulationAssemblyConstraintSession<AssemblyPartMaterial, C> {
    pub(crate) fn camera_line_services<'a>(
        &'a mut self,
        ground: &'a StaticGroundPool,
        numeric: &'a mut HostCollisionLineNumeric,
    ) -> CameraLineServices<'a> {
        CameraLineServices {
            queries: &mut self.world_line_queries,
            dmos: self.dmos.as_ref(),
            ground,
            numeric,
            overlay: &self.debug_overlay,
            grinds: &self.grind_services,
        }
    }
}
