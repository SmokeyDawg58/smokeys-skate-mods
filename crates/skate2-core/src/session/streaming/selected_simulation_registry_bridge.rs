//! Same resident namespace for selected inner assets and actual outer arenas.
//! The resident registry still publishes an atomic host batch; this bridge does
//! not claim the native mid-load visibility window or complete asset processing.
use super::{SelectedAssetError, SelectedSimulationAssetServices, SessionRegistry};
use crate::{
    collection_resident_streamer::CollectionId,
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_resident_collection::SimulationResidentArenaAsset,
    simulation_resident_registry::{SimulationInnerRegistryError, SimulationResidentRegistry},
};
use std::sync::Arc;

pub(crate) trait SelectedInnerLookup: Send + Sync {
    fn publish_inner(&self, asset: &Arc<SimulationInnerRegistryAsset>) -> Result<(), SimulationInnerRegistryError>;
    fn remove_inner(&self, asset: &Arc<SimulationInnerRegistryAsset>) -> Result<(), SimulationInnerRegistryError>;
    fn try_inner(
        &self,
        id: CollectionId,
    ) -> Result<Option<Arc<SimulationInnerRegistryAsset>>, SimulationInnerRegistryError>;
}

/// Runs while the actual registry holds its existing publication guard. Neither
/// method may reacquire that mutex. Outer means the current actual candidate.
pub(crate) trait SelectedAssetPublicationSink {
    fn try_add_inner(
        &mut self,
        asset: &Arc<SimulationInnerRegistryAsset>,
    ) -> Result<(), SelectedAssetError>;
    fn try_add_outer(&mut self) -> Result<(), SelectedAssetError>;
}

pub(crate) trait SelectedArenaProcessor<A>: Send + Sync {
    fn try_process(
        &self,
        asset: &Arc<A>,
        sink: &mut dyn SelectedAssetPublicationSink,
    ) -> Result<(), SelectedAssetError>;
}

pub(super) enum RegistryBinding {
    Standalone(SessionRegistry),
    Resident(Arc<dyn SelectedInnerLookup>),
}

pub(super) struct StandaloneSink<'a> {
    pub(super) registry: &'a SessionRegistry,
    pub(super) plan: &'a Arc<SimulationNormalArenaPlan>,
}
impl SelectedAssetPublicationSink for StandaloneSink<'_> {
    fn try_add_inner(
        &mut self,
        asset: &Arc<SimulationInnerRegistryAsset>,
    ) -> Result<(), SelectedAssetError> {
        self.registry
            .try_add_inner(asset)
            .map_err(SelectedAssetError::Registry)
    }
    fn try_add_outer(&mut self) -> Result<(), SelectedAssetError> {
        self.registry
            .try_add_selected_arena(
                CollectionId(self.plan.static_plan().asset().decoded.asset_id),
                self.plan,
            )
            .map_err(SelectedAssetError::Registry)
    }
}

impl SelectedSimulationAssetServices {
    /// Installs processing in the existing resident publisher. The service's
    /// lookup is weak; the publisher retains the service, so there is no cycle.
    pub(crate) fn construct_resident<M, E>(
        registry: &mut SimulationResidentRegistry<SimulationResidentArenaAsset, M, E>,
    ) -> Arc<Self> {
        let services =
            Self::with_binding(RegistryBinding::Resident(registry.selected_inner_lookup()));
        registry
            .set_selected_processor(Arc::clone(&services)
                as Arc<dyn SelectedArenaProcessor<SimulationResidentArenaAsset>>);
        services
    }
}
impl SelectedArenaProcessor<SimulationResidentArenaAsset> for SelectedSimulationAssetServices {
    fn try_process(
        &self,
        asset: &Arc<SimulationResidentArenaAsset>,
        sink: &mut dyn SelectedAssetPublicationSink,
    ) -> Result<(), SelectedAssetError> {
        self.try_process_plan_into(&asset.plan_owner(), sink)
    }
}

#[cfg(test)]
#[path = "selected_simulation_registry_bridge_tests.rs"]
mod tests;
