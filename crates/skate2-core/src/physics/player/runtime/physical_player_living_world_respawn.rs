//! Offline `LWMan` respawn queries over the selected retained owners.
//! The caller supplies the Actor breadcrumb's native constants and capsule.
use super::world_line_query::SimulationWorldLineError;
use crate::{
    attribute_runtime::AttributeRuntimeError,
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::InputCvarReferences,
    living_world_grind_services::{
        LivingWorldGrindServices,
        scoring::pedestrian_sphere::PedestrianSphereQueryError,
        vehicle_queries::VehicleSphereQueryError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningHostNumericError,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum LivingWorldRespawnQueryError {
    ForeignManager,
    ManagerLock,
    UninitializedEntities,
    Vehicle(VehicleSphereQueryError),
    Pedestrian(PedestrianSphereQueryError),
    CapsuleAttribute(AttributeRuntimeError),
    MissingWorldQueries,
    MissingWorldDmoPool,
    World(SimulationWorldLineError<ReckoningHostNumericError>),
}

impl std::fmt::Display for LivingWorldRespawnQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "selected LivingWorld respawn query: {self:?}")
    }
}

impl PhysicalAssemblyWorld {
    /// Offline `TestForLWRespawnContraints` 0x82ce1320. The original checks
    /// vehicle 0x82ce139c, pedestrian 0x82ce13d8, then the DMO capsule
    /// 0x82ce149c (`BF_Dmo` line batch over the DMO collection), returning
    /// immediately on either sphere hit.
    /// `expected_manager` is the Actor breadcrumb's LW owner identity; the
    /// supplied capsule retains its separately sourced start/end/radius.
    pub(crate) fn try_offline_respawn_lw_constraints(
        &mut self,
        expected_manager: &Arc<Mutex<LivingWorldGrindServices>>,
        ground: &StaticGroundPool,
        position: SkeletonRootVector,
        vehicle_radius: f32,
        pedestrian_radius: f32,
        cvars: &InputCvarReferences,
        capsule: impl FnOnce() -> Result<CollisionLineInput, AttributeRuntimeError>,
    ) -> Result<bool, LivingWorldRespawnQueryError> {
        use LivingWorldRespawnQueryError as E;
        let owner = Arc::clone(self.grind_services.living_world());
        if !Arc::ptr_eq(&owner, expected_manager) {
            return Err(E::ForeignManager);
        }
        let living = owner.lock().map_err(|_| E::ManagerLock)?;
        if !living.selected_entities_initialized() {
            return Err(E::UninitializedEntities);
        }
        // Use the same native vehicle narrowphase as UpdateCollisionStatus.
        // Breadcrumb passes the candidate translation without lifting its Y.
        if living
            .try_vehicle_intersects_sphere(position.xyz, vehicle_radius, cvars, &self.debug_overlay)
            .map_err(E::Vehicle)?
        {
            return Ok(false);
        }
        // 82703600..3928: six-ID broadphase followed by each pedestrian's
        // actual Part0 local-box test. Nearby candidates alone are not hits.
        if living
            .try_pedestrian_intersects_sphere(
                position.xyz,
                pedestrian_radius,
                self,
                cvars,
                &self.debug_overlay,
            )
            .map_err(E::Pedestrian)?
        {
            return Ok(false);
        }
        drop(living);
        // Native TestForLWRespawnContraints reads the two capsule attributes
        // only after the vehicle and pedestrian queries both miss.
        let capsule = capsule().map_err(E::CapsuleAttribute)?;
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(E::MissingWorldQueries)?;
        let dmos = self.dmos.as_ref().ok_or(E::MissingWorldDmoPool)?;
        let hit = queries
            .try_dmo_capsule_test(ground, dmos, capsule, &mut HostCollisionLineNumeric)
            .map_err(E::World)?;
        Ok(!hit)
    }
}
