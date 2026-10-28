//! DoesAnyPedestrianIntersectSphere82703600. The decompiler omits the
//! populated loop; raw82703698..3928 and PDB vtables establish this route.
use crate::{
    living_world_grind_services::{LivingWorldGrindServices, PedestrianNearQueryError,
        vehicle_queries::vehicle_local_point},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    physical_player_assembly::PhysicalAssemblyWorld,
};
#[derive(Debug)]
pub(crate) enum PedestrianSphereQueryError {
    Range(PedestrianNearQueryError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Body(String),
}
impl LivingWorldGrindServices {
    /// Uses real skater collision Part0, including a detached body if retained.
    /// No active/visible/initialized filter occurs after the native GetPedestrian.
    pub(crate) fn try_pedestrian_intersects_sphere(
        &self, position: [f32; 3], radius: f32,
        world: &PhysicalAssemblyWorld, cvars: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
    ) -> Result<bool, PedestrianSphereQueryError> {
        use PedestrianSphereQueryError as E;
        let mut count = 0;
        let mut ids = [0; 6];
        let mut distances = [0.; 6];
        self.try_get_pedestrians_within_range([position[0], position[2]], radius + 2., 6,
            &mut count, &mut ids, &mut distances).map_err(E::Range)?;
        let pedestrians = self.pedestrian_entities.as_ref()
            .ok_or(E::Range(PedestrianNearQueryError::MissingEntityMan))?;
        for id in &ids[..count as usize] {
            let Some(pedestrian) = pedestrians.get(*id) else { continue; };
            // LWCollideableObject+3c -> collision; collision+28 -> skater body.
            let flat = pedestrian.collision.as_ref()
                .ok_or_else(|| E::Body("published pedestrian collision absent".into()))?
                .skater_part_transform(0, world).map_err(E::Body)?;
            let matrix: [[f32; 4]; 4] = std::array::from_fn(|axis|
                std::array::from_fn(|lane| flat[axis * 4 + lane]));
            let dimensions = pedestrian.model.as_ref()
                .ok_or_else(|| E::Body("published pedestrian model absent".into()))?.dimensions;
            let local = vehicle_local_point(&matrix, position, matrix[3]);
            if cvars.try_draw_lw_position_test().map_err(E::Cvar)? {
                overlay.require().map_err(E::Overlay)?;
            }
            // Raw8270383c..38e4: strict Z, X, Y against half dimensions+radius.
            // Pedestrians have no vehicle query's extra one-metre Y clearance.
            if dimensions[2].mul_add(0.5, radius) > local[2].abs()
                && dimensions[0].mul_add(0.5, radius) > local[0].abs()
                && dimensions[1].mul_add(0.5, radius) > local[1].abs() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
