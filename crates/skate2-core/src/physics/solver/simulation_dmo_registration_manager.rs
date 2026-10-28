//! Private staged owner for the two native manager-scoped DMO registration
//! tables.
//!
//! Native construction initializes models before locators, while destruction
//! destroys locators before models. Rust mirrors those ownership facts by
//! constructing local model state first and declaring the locator field first.
//! This module does not install a live adapter or reproduce `SpawnStatic`.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "staged manager owner awaits the live DMO registration adapter"
    )
)]

use super::{
    DmoBankResidentModelCatalog, SimulationDmoLocatorRegisterError, SimulationDmoLocatorRegistry,
    SimulationDmoLocatorRegistryInitError, SimulationDmoModelRegisterError,
    SimulationDmoModelRegistrationSource, SimulationDmoModelRegistry,
    SimulationDmoModelRegistryInitError, SimulationDmoRegisteredLocatorOccurrenceError,
    SimulationDmoResolvedLocatorPartOwner, SimulationDmoResolvedLocatorPartOwnerError,
    SourceBackend,
};
use crate::simulation_dmo_arena_plan::{SimulationDmoArenaPlan, SimulationDmoDataHandle};
use std::{error::Error, fmt, sync::Arc};

/// Safe-host construction refusal for the combined manager-scoped registration
/// owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SimulationDmoRegistrationManagerInitError {
    Models(SimulationDmoModelRegistryInitError),
    Locators(SimulationDmoLocatorRegistryInitError),
}

impl fmt::Display for SimulationDmoRegistrationManagerInitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Models(source) => {
                write!(
                    formatter,
                    "could not initialize DMO model registrations: {source}"
                )
            }
            Self::Locators(source) => write!(
                formatter,
                "could not initialize DMO locator registrations: {source}"
            ),
        }
    }
}

impl Error for SimulationDmoRegistrationManagerInitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Models(source) => Some(source),
            Self::Locators(source) => Some(source),
        }
    }
}

/// Keyless refusal while resolving a manager-qualified locator occurrence into
/// its exact resident model and backend-bound Part owner.
#[derive(Debug)]
pub(crate) enum SimulationDmoRegistrationManagerResolveError {
    Locator(SimulationDmoRegisteredLocatorOccurrenceError),
    Part(SimulationDmoResolvedLocatorPartOwnerError),
}

impl fmt::Display for SimulationDmoRegistrationManagerResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locator(source) => {
                write!(
                    formatter,
                    "could not select registered DMO locator: {source}"
                )
            }
            Self::Part(source) => {
                write!(
                    formatter,
                    "could not bind registered DMO locator Parts: {source}"
                )
            }
        }
    }
}

impl Error for SimulationDmoRegistrationManagerResolveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Locator(source) => Some(source),
            Self::Part(source) => Some(source),
        }
    }
}

/// Combined manager-scoped ownership of locator then model registrations.
///
/// Field declaration order deliberately makes Rust drop locator ownership
/// before model ownership. No registry getter, iterator, or raw lookup escapes.
pub(crate) struct SimulationDmoRegistrationManager {
    locators: SimulationDmoLocatorRegistry,
    models: SimulationDmoModelRegistry,
}

impl SimulationDmoRegistrationManager {
    /// Builds safe-host model storage before locator storage, matching only the
    /// recovered native constructor order. Allocation refusal is Rust policy.
    pub(super) fn try_new() -> Result<Self, SimulationDmoRegistrationManagerInitError> {
        let models = SimulationDmoModelRegistry::try_new()
            .map_err(SimulationDmoRegistrationManagerInitError::Models)?;
        let locators = SimulationDmoLocatorRegistry::try_new()
            .map_err(SimulationDmoRegistrationManagerInitError::Locators)?;
        Ok(Self { locators, models })
    }

    pub(super) fn register_model(
        &mut self,
        source: SimulationDmoModelRegistrationSource,
    ) -> Result<(), SimulationDmoModelRegisterError> {
        self.models.register(source)
    }

    pub(super) fn deregister_model(&mut self, source: SimulationDmoModelRegistrationSource) {
        self.models.deregister(source);
    }

    #[cfg(test)]
    pub(super) fn has_model_for_test(&self, model_key: u64) -> bool {
        self.models.resolver().try_current(model_key).is_ok()
    }

    #[cfg(test)]
    pub(super) fn model_matches_asset_operation_for_test(
        &self,
        model_key: u64,
        asset: &std::sync::Weak<crate::dmo_bank_processed_asset::DmoBankProcessedArenaAsset>,
        operation_index: usize,
    ) -> bool {
        self.models
            .resolver()
            .try_current(model_key)
            .is_ok_and(|winner| winner.matches_catalog_route(asset, operation_index))
    }

    pub(crate) fn register_locator(
        &mut self,
        source: &SimulationDmoDataHandle,
        container_id: u64,
    ) -> Result<(), SimulationDmoLocatorRegisterError> {
        self.locators.register(source, container_id)
    }

    pub(crate) fn deregister_locator(
        &mut self,
        source: &SimulationDmoDataHandle,
        container_id: u64,
    ) {
        self.locators.deregister(source, container_id);
    }

    /// Mirrors native `Registration::Clear`: only locator ownership is
    /// released, and the model table remains unchanged.
    pub(crate) fn clear_locators(&mut self) {
        self.locators.clear();
    }

    /// Resolves locator registration before consulting model or catalog state.
    pub(crate) fn try_resolve_locator_part_owner(
        &self,
        plan: &SimulationDmoArenaPlan,
        operation_index: usize,
        locator_ordinal: usize,
        candidate_catalog: &DmoBankResidentModelCatalog,
        backend: &SourceBackend,
    ) -> Result<
        Arc<SimulationDmoResolvedLocatorPartOwner>,
        SimulationDmoRegistrationManagerResolveError,
    > {
        let locator = self
            .locators
            .try_locator_occurrence(plan, operation_index, locator_ordinal)
            .map_err(SimulationDmoRegistrationManagerResolveError::Locator)?;
        SimulationDmoResolvedLocatorPartOwner::try_new(
            locator,
            &self.models,
            candidate_catalog,
            backend,
        )
        .map_err(SimulationDmoRegistrationManagerResolveError::Part)
    }
}
