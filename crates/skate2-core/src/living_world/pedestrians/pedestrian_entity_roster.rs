//! Consumers of the sole typed EntityMan40. Factory completion remains the
//! publication gate; a construction prefix must never be added to this roster.
use super::PedestrianConstruction;
use crate::living_world_grind_services::entities::SpatialEntity;

impl SpatialEntity for PedestrianConstruction {
    fn core_position_xz(&self) -> [f32; 2] {
        self.core.as_ref().expect("published pedestrian core").position_xz
    }
    fn core_data_valid(&self) -> bool { self.core.is_some() }
}
impl PedestrianConstruction {
    /// SetHonkingVehicle82e6ed98 writes the native signed ID word unchanged.
    pub(crate) fn set_honking_vehicle(&mut self, vehicle_id: u32) -> Result<(), String> {
        self.state.as_mut().ok_or("published pedestrian state absent")?
            .honking_vehicle_id = vehicle_id as i32;
        Ok(())
    }
    /// NotifyPedsAboutCulls82e84d90 dispatches every retained head tracker.
    pub(crate) fn notify_entity_culled(&mut self, entity_id: u32) -> Result<(), String> {
        self.head_tracking.as_mut().ok_or("published pedestrian head tracking absent")?
            .notify_entity_culled(entity_id);
        Ok(())
    }
}
