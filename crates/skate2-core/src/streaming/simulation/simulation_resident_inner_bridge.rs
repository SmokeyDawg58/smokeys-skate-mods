//! Selected inner publication under the existing resident registry guard.
use super::{
    CollectionId, ContainerRegistryAsset, ContainerRegistryRow, OwnedResidentWeakAsset,
    RegistryState, SimulationInnerRegistryError, SimulationResidentLookup,
    SimulationResidentRegistry,
};
use crate::{
    selected_simulation_asset_services::{
        SelectedAssetError,
        bridge::{SelectedArenaProcessor, SelectedAssetPublicationSink, SelectedInnerLookup},
    },
    simulation_inner_registry_asset::SimulationInnerRegistryAsset,
};
use std::sync::{Arc, Weak};

pub(super) fn lookup<A>(
    state: &RegistryState<A>,
    id: CollectionId,
) -> Result<Option<Arc<SimulationInnerRegistryAsset>>, SimulationInnerRegistryError> {
    let Some(row) = state.container.iter().find(|row| row.id == id) else {
        return Ok(None);
    };
    let ContainerRegistryAsset::Inner { asset, .. } = &row.asset else {
        return Err(SimulationInnerRegistryError::KindMismatch {
            id,
            actual: row.asset.kind(),
        });
    };
    asset
        .upgrade()
        .map(Some)
        .ok_or(SimulationInnerRegistryError::Expired { id })
}

pub(super) fn add_inner<A>(
    state: &mut RegistryState<A>,
    asset: &Arc<SimulationInnerRegistryAsset>,
) -> Result<(), SimulationInnerRegistryError> {
    let id = CollectionId(asset.id);
    if state.container.iter().any(|row| row.id == id) {
        return Ok(());
    }
    state
        .container
        .try_reserve(1)
        .map_err(|_| SimulationInnerRegistryError::Allocation)?;
    state.container.push(ContainerRegistryRow {
        id,
        asset: ContainerRegistryAsset::Inner {
            kind: asset.kind as u32,
            asset: Arc::downgrade(asset),
        },
    });
    Ok(())
}

impl<A: Send + Sync + 'static> SelectedInnerLookup for SimulationResidentLookup<A> {
    fn publish_inner(&self, asset: &Arc<SimulationInnerRegistryAsset>) -> Result<(), SimulationInnerRegistryError> {
        let state = self.state.upgrade().ok_or(SimulationInnerRegistryError::Detached)?;
        let mut state = state.lock().map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        add_inner(&mut state, asset)
    }
    fn remove_inner(&self, asset: &Arc<SimulationInnerRegistryAsset>) -> Result<(), SimulationInnerRegistryError> {
        let state = self.state.upgrade().ok_or(SimulationInnerRegistryError::Detached)?;
        let mut state = state.lock().map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        if let Some(index) = state.container.iter().position(|row| row.id.0 == asset.id && matches!(&row.asset,
            ContainerRegistryAsset::Inner {asset: current, ..} if Weak::ptr_eq(current, &Arc::downgrade(asset)))) {
            state.container.remove(index);
        }
        Ok(())
    }
    fn try_inner(
        &self,
        id: CollectionId,
    ) -> Result<Option<Arc<SimulationInnerRegistryAsset>>, SimulationInnerRegistryError> {
        let state = self
            .state
            .upgrade()
            .ok_or(SimulationInnerRegistryError::Detached)?;
        let state = state
            .lock()
            .map_err(|_| SimulationInnerRegistryError::Poisoned)?;
        lookup(&state, id)
    }
}
impl<A, M, E> SimulationResidentRegistry<A, M, E> {
    pub(crate) fn selected_inner_lookup(&self) -> Arc<dyn SelectedInnerLookup>
    where
        A: Send + Sync + 'static,
    {
        Arc::new(SimulationResidentLookup {
            state: Arc::downgrade(&self.state),
        })
    }
    pub(crate) fn set_selected_processor(&mut self, processor: Arc<dyn SelectedArenaProcessor<A>>) {
        self.selected_processor = Some(processor);
    }
}

pub(super) struct SelectedPublication<'a, A> {
    pub(super) state: &'a mut RegistryState<A>,
    pub(super) candidate: &'a OwnedResidentWeakAsset<A>,
    pub(super) inners: &'a mut Vec<(u64, Weak<SimulationInnerRegistryAsset>)>,
}
impl<A> SelectedAssetPublicationSink for SelectedPublication<'_, A> {
    fn try_add_inner(
        &mut self,
        asset: &Arc<SimulationInnerRegistryAsset>,
    ) -> Result<(), SelectedAssetError> {
        self.inners
            .try_reserve(1)
            .map_err(|_| SelectedAssetError::Allocation)?;
        add_inner(self.state, asset).map_err(SelectedAssetError::Registry)?;
        self.inners.push((asset.id, Arc::downgrade(asset)));
        Ok(())
    }
    fn try_add_outer(&mut self) -> Result<(), SelectedAssetError> {
        let id = self.candidate.id();
        if self.state.container.iter().any(|row| row.id == id) {
            return Ok(());
        }
        self.state
            .container
            .try_reserve(1)
            .map_err(|_| SelectedAssetError::Allocation)?;
        self.state.container.push(ContainerRegistryRow {
            id,
            asset: ContainerRegistryAsset::Arena {
                asset: Weak::clone(self.candidate.weak()),
            },
        });
        Ok(())
    }
}
