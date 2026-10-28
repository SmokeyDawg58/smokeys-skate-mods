//! Selected camera input composition over retained terrain and supplied startup owners.
use super::SelectedTerrainStartup;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum CameraTerrainError {
    Missing(&'static str),
    Bounds(crate::world_stream_runtime::WorldBoundsError),
    Trigger(crate::trigger_camera_group::CameraTriggerError),
}
impl std::fmt::Display for CameraTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "selected camera input missing {name}"),
            Self::Bounds(e) => write!(f, "selected camera bounds: {e}"),
            Self::Trigger(e) => write!(f, "selected camera triggers: {e:?}"),
        }
    }
}
impl SelectedTerrainStartup {
    /// Caller supplies the same held world and its handle, plus authored camera set.
    /// This does not acquire the world mutex or manufacture query ownership.
    pub(crate) fn try_camera_inputs(
        &self,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
        world_handle: &Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
        players: &crate::actor_construction::selected_startup::SelectedPlayerStartupOwners,
        bounds: &crate::world_stream_runtime::WorldBoundsPublication,
        camera_set: String,
    ) -> Result<crate::camera_session::selected_runtime::CameraSelectedInputs, CameraTerrainError>
    {
        if !self.complete {
            return Err(CameraTerrainError::Missing(
                "completed terrain registration",
            ));
        }
        let _ = bounds.read().map_err(CameraTerrainError::Bounds)?;
        Ok(
            crate::camera_session::selected_runtime::CameraSelectedInputs {
                vault: Arc::clone(&players.services.vault),
                cvars: players.cvars.clone(),
                mounts: Arc::clone(&players.mounts),
                monitor: Arc::new(
                    crate::attribute_notifications::AttributeNotifications::constructed(),
                ),
                cache: players
                    .graphs
                    .as_ref()
                    .ok_or(CameraTerrainError::Missing("graph owners"))?
                    .lock().map_err(|_|CameraTerrainError::Missing("graph owner lock"))?
                    .retained_dynamic_cache()
                    .ok_or(CameraTerrainError::Missing("dynamic graph cache"))?,
                world: Arc::downgrade(world_handle),
                predictor: Arc::clone(
                    world
                        .prediction_owner()
                        .ok_or(CameraTerrainError::Missing("World predictor"))?,
                ),
                living_world: Arc::clone(world.grind_services.living_world()),
                skaters: Arc::clone(&players.services.skaters),
                assets: Arc::clone(&self.assets),
                layers: Arc::clone(&self.layers),
                triggers: Arc::clone(&self.triggers),
                sim_rate: Arc::clone(&world.sim_rate),
                camera_set,
            },
        )
    }
}
