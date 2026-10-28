//! Mounted shipped-corpus collaborator for global Location arenas.
//!
//! This module proves the concrete payload path used below
//! `cGlobalFileLocationManager`: named global-locator RX2 entries can be read,
//! decoded, projected, prepared, and registered by the reconstructed Location
//! runtime through the exact eight-slot activation owner. Mounted reads and
//! registrations complete synchronously as an explicit host scheduling policy;
//! native asset-loader worker timing is not claimed. Case-insensitive `.rx2`
//! matching is also a host archive-enumeration policy because the native
//! wildcard matcher semantics have not been recovered.

use skate2_core::{
    decoded_arena_asset::{DecodedArenaAsset, WORLD_STREAM_ARENA_ASSET_TYPE},
    global_location_activation::{
        ArenaLoadCompletion, ArenaRegistrationCompletion, GlobalLocationActivationEffects,
        GlobalLocationActivationMachine, GlobalLocationLoadRequest,
    },
    location_arena_adapter::{
        LocationArenaRegistration, LocationArenaRegistrationAdapter, LocationArenaRuntimeSource,
        PreparedLocationArena,
    },
    location_arena_plan::{LocationArenaPlan, LocationDataHandle},
    simulation_collision_asset::SimulationCollisionAsset,
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
};
use skate2_formats::{
    big::BigEntryKey,
    mount::MountSet,
    rw4::Arena,
    stream::{ArenaResources, BaseResourceDescriptor},
};
use std::{collections::BTreeMap, error::Error, fmt, sync::Arc};

const GLOBAL_LOCATION_DIRECTORY: &str = "data\\content\\global_locators\\";

#[derive(Debug)]
struct LocationSourceRuntime {
    sources: Vec<(u64, LocationDataHandle)>,
}

impl LocationArenaRuntimeSource for LocationSourceRuntime {
    fn resolve_location_data(&mut self, asset_id: u64) -> Option<LocationDataHandle> {
        self.sources
            .iter()
            .find(|(candidate, _)| *candidate == asset_id)
            .map(|(_, source)| source.clone())
    }

    fn high_priority_raw(&mut self) -> u8 {
        // The shipped-corpus check has no recovered live cVar owner. Keep this
        // explicit probe policy out of the native scheduling boundary.
        0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlobalLocationInfo {
    pub path: String,
    pub asset_id: u64,
    pub loaded: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GlobalLocationCorpusSummary {
    pub files: usize,
    pub containers: usize,
    pub locations: usize,
    pub sub_locations: usize,
    pub normal_operations: usize,
    pub registered_entries: usize,
    pub live_registrations: usize,
    pub allocated_assets: u32,
}

#[derive(Debug)]
struct MountedGlobalLocationEffects {
    mounts: Arc<MountSet>,
    infos: BTreeMap<u64, GlobalLocationInfo>,
    prepared: BTreeMap<u64, PreparedLocationArena>,
    adapter: LocationArenaRegistrationAdapter<LocationSourceRuntime>,
    registrations: Vec<LocationArenaRegistration>,
    summary: GlobalLocationCorpusSummary,
}

#[derive(Debug)]
struct MountedGlobalLocationEffectError(Box<dyn Error>);

impl fmt::Display for MountedGlobalLocationEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for MountedGlobalLocationEffectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl From<Box<dyn Error>> for MountedGlobalLocationEffectError {
    fn from(source: Box<dyn Error>) -> Self {
        Self(source)
    }
}

impl MountedGlobalLocationEffects {
    fn new(mounts: Arc<MountSet>, infos: BTreeMap<u64, GlobalLocationInfo>) -> Self {
        Self {
            mounts,
            infos,
            prepared: BTreeMap::new(),
            adapter: LocationArenaRegistrationAdapter::new(LocationSourceRuntime {
                sources: Vec::new(),
            }),
            registrations: Vec::new(),
            summary: GlobalLocationCorpusSummary::default(),
        }
    }
}

impl GlobalLocationActivationEffects for MountedGlobalLocationEffects {
    type Error = MountedGlobalLocationEffectError;

    fn begin_arena_load(
        &mut self,
        request: &GlobalLocationLoadRequest,
        completion: ArenaLoadCompletion,
    ) -> Result<(), Self::Error> {
        (|| -> Result<(), Box<dyn Error>> {
            let bytes = self.mounts.read_path(request.path())?;
            let prepared = prepare_location_arena(request.asset_id(), &bytes)?;
            self.summary.normal_operations += prepared.plan().normal_plan().operations().len();
            self.summary.containers += prepared.container_count();
            for operation_index in 0..prepared.container_count() {
                let operation = prepared
                    .operation(operation_index)
                    .ok_or("prepared global Location container disappeared")?;
                self.summary.locations += usize::try_from(operation.location_count)?;
                self.summary.sub_locations += usize::try_from(operation.sub_location_count)?;
            }
            if self.prepared.insert(request.asset_id(), prepared).is_some() {
                return Err("global Location prepared owner was replaced".into());
            }
            completion.complete()?;
            Ok(())
        })()
        .map_err(MountedGlobalLocationEffectError::from)
    }

    fn begin_sim_registration(
        &mut self,
        asset_id: u64,
        completion: ArenaRegistrationCompletion,
    ) -> Result<(), Self::Error> {
        (|| -> Result<(), Box<dyn Error>> {
            let prepared = self
                .prepared
                .remove(&asset_id)
                .ok_or("global Location registration had no prepared arena")?;
            for operation_index in 0..prepared.container_count() {
                let operation = prepared
                    .operation(operation_index)
                    .ok_or("prepared global Location container disappeared")?;
                let source = prepared
                    .planned_source(operation_index)
                    .ok_or("prepared global Location source disappeared")?;
                self.adapter
                    .runtime_mut()
                    .sources
                    .push((operation.asset_id, source));
            }
            self.registrations.push(self.adapter.register(&prepared)?);
            completion.complete()?;
            Ok(())
        })()
        .map_err(MountedGlobalLocationEffectError::from)
    }

    fn asset_missing(&mut self, _asset_id: u64) -> Result<(), Self::Error> {
        Ok(())
    }

    fn asset_loaded(&mut self, _asset_id: u64) -> Result<(), Self::Error> {
        self.summary.allocated_assets = self.summary.allocated_assets.wrapping_add(1);
        Ok(())
    }

    fn asset_added(&mut self, asset_id: u64) -> Result<(), Self::Error> {
        (|| -> Result<(), Box<dyn Error>> {
            let info = self
                .infos
                .get_mut(&asset_id)
                .ok_or("global Location manager record disappeared")?;
            info.loaded = true;
            self.summary.files += 1;
            Ok(())
        })()
        .map_err(MountedGlobalLocationEffectError::from)
    }
}

/// Retains the activation owner, payload owners, and live registrations.
#[derive(Debug)]
pub struct MountedGlobalLocationCorpus {
    activation: GlobalLocationActivationMachine<MountedGlobalLocationEffectError>,
    effects: MountedGlobalLocationEffects,
}

impl MountedGlobalLocationCorpus {
    /// Loads every named `data\content\global_locators\*.rx2` entry in mounted
    /// archive/table order, skipping a repeated full-path ID already present
    /// in this newly built corpus before materialization.
    ///
    /// Mounted archive/table order is a documented host enumeration policy.
    /// Native `AllDeviceDirectoryEntry` ordering across devices is not yet
    /// recovered.
    ///
    /// # Errors
    ///
    /// Returns the first checked mount, arena, plan, preparation, registration,
    /// activation, or integer-conversion failure.
    pub fn load(mounts: &Arc<MountSet>) -> Result<Self, Box<dyn Error>> {
        let mut candidates = Vec::new();
        for mounted in mounts.archives() {
            for entry in &mounted.archive().entries {
                let BigEntryKey::Name(path) = &entry.key else {
                    continue;
                };
                if is_global_location_path(path) {
                    candidates.push(path.clone());
                }
            }
        }

        let mut infos = BTreeMap::new();
        let mut activation = GlobalLocationActivationMachine::new();
        for path in candidates {
            let asset_id = skate2_formats::hash::rw_hash64_string(&path);
            if infos.contains_key(&asset_id) {
                continue;
            }
            let queued_id = activation.enqueue_filename_load(&path)?;
            if queued_id != asset_id {
                return Err("global Location request returned a different path hash".into());
            }
            infos.insert(
                asset_id,
                GlobalLocationInfo {
                    path,
                    asset_id,
                    loaded: false,
                },
            );
        }

        let mut effects = MountedGlobalLocationEffects::new(Arc::clone(mounts), infos);
        while activation.is_pending() {
            activation
                .update(&mut effects)
                .map_err(|source| -> Box<dyn Error> { Box::new(source) })?;
        }
        effects.summary.registered_entries = effects.adapter.entries().len();
        effects.summary.live_registrations = effects.adapter.live_registration_count();
        Ok(Self {
            activation,
            effects,
        })
    }

    #[must_use]
    pub const fn summary(&self) -> GlobalLocationCorpusSummary {
        self.effects.summary
    }

    #[must_use]
    pub fn infos(&self) -> &BTreeMap<u64, GlobalLocationInfo> {
        &self.effects.infos
    }

    #[must_use]
    pub fn registration_count(&self) -> usize {
        self.effects.registrations.len()
    }

    #[must_use]
    pub fn adapter_entry_count(&self) -> usize {
        self.effects.adapter.entries().len()
    }

    #[must_use]
    pub fn activation_is_pending(&self) -> bool {
        self.activation.is_pending()
    }
}

fn prepare_location_arena(
    asset_id: u64,
    bytes: &[u8],
) -> Result<PreparedLocationArena, Box<dyn Error>> {
    let resources = Arena::split_file_resources(bytes)?;
    let arena = Arena::parse(&resources)?;
    let descriptors = std::array::from_fn(|index| BaseResourceDescriptor {
        size: arena.resource_descriptor[index].size,
        alignment: arena.resource_descriptor[index].alignment,
    });
    let decoded = DecodedArenaAsset {
        asset_id,
        asset_type: WORLD_STREAM_ARENA_ASSET_TYPE,
        asset_resource_descriptors: descriptors,
        resources: ArenaResources { resources },
        arena,
    };
    let collision = SimulationCollisionAsset::from_decoded(decoded)?;
    let normal = Arc::new(SimulationNormalArenaPlan::from_collision_asset(collision)?);
    let location = Arc::new(LocationArenaPlan::from_normal_plan(normal)?);
    Ok(PreparedLocationArena::try_new(location)?)
}

fn is_global_location_path(path: &str) -> bool {
    let Some(relative) = path.strip_prefix(GLOBAL_LOCATION_DIRECTORY) else {
        return false;
    };
    !relative.is_empty()
        && !relative.contains(['\\', '/'])
        && relative.to_ascii_lowercase().ends_with(".rx2")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_directory_filter_selects_only_direct_named_rx2_assets() {
        assert!(is_global_location_path(
            "data\\content\\global_locators\\DIST_SoHo_[0x1].rx2"
        ));
        assert!(is_global_location_path(
            "data\\content\\global_locators\\FE_Main_[0x2].RX2"
        ));
        assert!(!is_global_location_path(
            "data\\content\\global_locator\\DIST_SoHo_[0x1].rx2"
        ));
        assert!(!is_global_location_path(
            "data\\content\\global_locators\\nested\\one.rx2"
        ));
        assert!(!is_global_location_path(
            "data\\content\\global_locators\\one.xml"
        ));
    }
}
