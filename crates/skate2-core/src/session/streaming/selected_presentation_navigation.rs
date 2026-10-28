//! NotifyCollectionReg82d5c920 sends Simulation registration for BAM_Pres too.
//! Reuse the shared registry and MeshStore; retain the original decoded plans.
use super::{SelectedSimulationAssetServices, RegistryBinding};
use crate::{living_world_mesh_store::MeshStore, presentation::PresentationNavigationArena,
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
    simulation_normal_arena_plan::{SimulationNormalOperation, SimulationRegistryAssetKind}};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct NavigationRegistration {
    entries: Vec<(PresentationNavigationArena, Vec<Arc<SimulationInnerRegistryAsset>>)>,
    failed: bool,
}
impl SelectedSimulationAssetServices {
    pub(crate) fn sync_presentation_navigation(&self, requested: &[PresentationNavigationArena], mesh: &MeshStore,
        mut release: impl FnMut(&Arc<crate::living_world_mesh_store::asset::NavMeshAsset>) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut state = self.presentation_navigation.lock().map_err(|_| "presentation navigation lock")?;
        if state.failed { return Err("previous presentation navigation registration failed".into()); }
        state.failed = true;
        // Deregister while registry payloads and tile handles are still alive.
        let mut index = 0;
        while index < state.entries.len() {
            if requested.iter().any(|plan| plan == &state.entries[index].0) { index += 1; continue; }
            for asset in &state.entries[index].1 {
                if self.try_inner(asset.id).map_err(|e|format!("navigation lookup: {e:?}"))?
                    .as_ref().is_some_and(|current| Arc::ptr_eq(current, asset)) {
                    if let Some(tile) = mesh.tile(asset.id).map_err(|e|e.to_string())? {
                        release(&tile)?;
                        mesh.try_remove_tile(asset.id).map_err(|e|e.to_string())?;
                    }
                }
                match &self.registry {
                    RegistryBinding::Standalone(registry) => registry.try_remove_inner(asset),
                    RegistryBinding::Resident(registry) => registry.remove_inner(asset),
                }.map_err(|e|format!("navigation unpublish: {e:?}"))?;
            }
            state.entries.remove(index);
        }
        for plan in requested {
            if state.entries.iter().any(|(current,_)| current == plan) { continue; }
            state.entries.push((plan.clone(), Vec::new()));
            let retained = &mut state.entries.last_mut().expect("inserted navigation arena").1;
            for (index, operation) in plan.0.operations().iter().enumerate() {
                if !matches!(operation, SimulationNormalOperation::RegistryAsset {kind: SimulationRegistryAssetKind::NavMesh, ..}) { continue; }
                let asset = SimulationInnerRegistryAsset::try_from_operation(Arc::clone(&plan.0), index)
                    .map_err(|e|format!("navigation asset: {e:?}"))?;
                // Keep ownership even if a later publication/registration fails.
                retained.push(Arc::clone(&asset));
                match &self.registry {
                    RegistryBinding::Standalone(registry) => registry.try_add_inner(&asset),
                    RegistryBinding::Resident(registry) => registry.publish_inner(&asset),
                }.map_err(|e|format!("navigation publish: {e:?}"))?;
            }
            // ProcessArena publishes the complete inner list before RegArena.
            for asset in retained.iter() {
                if self.try_inner(asset.id).map_err(|e|format!("navigation lookup: {e:?}"))?
                    .as_ref().is_some_and(|current|Arc::ptr_eq(current, asset)) {
                    mesh.try_add_tile(asset.id).map_err(|e|e.to_string())?;
                }
            }
        }
        state.failed = false;
        Ok(())
    }
}
