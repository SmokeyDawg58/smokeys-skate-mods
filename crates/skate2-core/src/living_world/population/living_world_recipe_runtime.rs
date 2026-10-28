//! RecipeMan's reference counts, completion callbacks and throttled load queues.
//! Source: ctor82727140, Load827239e0, Update82728380, callbacks82712f30/82713178.
//! Asset IO and the two registration queues are supplied by the SAME session;
//! a load request does not itself certify a loaded or registered model.
use super::{
    RecipeLoadingQueueKind, RecipeLoadingQueues, RecipeManagerInitializationState,
    RecipeMappingTable,
};
use crate::living_world_grind_services::dmo_models::NULL_ASSET_ID;
use skate2_formats::hash::sk8_hash_value_32;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum RecipeInitError {
    Attributes(super::attributes::RecipeAttributeError),
    Mapping(super::RecipeMappingBuildError<super::attributes::RecipeAttributeError>),
    Queue(super::RecipeMappingError),
}
struct Initialization<'a, 'b> {
    mappings: &'a mut RecipeMappingTable,
    queues: &'a mut RecipeLoadingQueues,
    attributes: &'a mut super::attributes::RecipeAttributes<'b>,
}
impl super::RecipeManagerInitializationRuntime for Initialization<'_, '_> {
    type Error = RecipeInitError;
    fn initialize_recipe_mappings(&mut self) -> Result<(), Self::Error> {
        super::build_recipe_mappings(self.mappings, self.attributes)
            .map_err(RecipeInitError::Mapping)?;
        self.attributes
            .finish()
            .map_err(RecipeInitError::Attributes)
    }
    fn initialize_loading_queues(&mut self) -> Result<(), Self::Error> {
        use super::attributes::RecipeAttributeError as A;
        let initial = self
            .attributes
            .cvars
            .try_recipe_initial_load_limit()
            .map_err(|e| RecipeInitError::Attributes(A::Cvar(e)))?;
        let secondary = self
            .attributes
            .cvars
            .try_recipe_secondary_load_limit()
            .map_err(|e| RecipeInitError::Attributes(A::Cvar(e)))?;
        *self.queues = super::build_recipe_loading_queues(
            self.mappings,
            initial.max(0) as usize,
            secondary as u32 as usize,
        )
        .map_err(RecipeInitError::Queue)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum ModelCategory {
    RegularPed = 0,
    Marquee = 1,
    Prop = 2,
    Vehicle = 3,
    DynamicObject = 4,
}
impl ModelCategory {
    pub(crate) fn from_word(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::RegularPed,
            1 => Self::Marquee,
            2 => Self::Prop,
            3 => Self::Vehicle,
            4 => Self::DynamicObject,
            _ => return None,
        })
    }
    /// Character::Category is a different enum from LivingWorld::ModelCategory.
    pub(crate) const fn character_category(self) -> u32 {
        match self {
            Self::Marquee => 0,
            Self::Vehicle => 2,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum RecipeStatus {
    WaitingForLoad = 1,
    WaitingForLoadToUnload = 2,
    Loaded = 3,
    LoadedToUnload = 4,
    Unloading = 5,
}
#[derive(Clone, Debug)]
pub(crate) struct RecipeModel {
    pub(crate) key: u64,
    pub(crate) filename: String,
    pub(crate) category: ModelCategory,
}
#[derive(Debug)]
pub(crate) struct RecipeInfo {
    pub(crate) asset_id: u64,
    pub(crate) status: RecipeStatus,
    pub(crate) count: u32,
    pub(crate) model: RecipeModel,
    pub(crate) request_id: u32,
}

pub(crate) trait RecipeServices {
    type Error;
    fn model(&mut self, key: u64) -> Result<RecipeModel, Self::Error>;
    fn load(&mut self, category: u32, filename: &str) -> Result<u32, Self::Error>;
    fn unload(&mut self, asset_id: u64) -> Result<u32, Self::Error>;
    fn register_simulation(&mut self, asset_id: u64, key: u64) -> Result<(), Self::Error>;
    fn register_presentation(&mut self, asset_id: u64, key: u64) -> Result<(), Self::Error>;
    fn deregister_simulation(&mut self, asset_id: u64) -> Result<(), Self::Error>;
    fn deregister_presentation(&mut self, asset_id: u64) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum RecipeRuntimeError<E> {
    Service(E),
    RecordCapacity,
    UnknownRequest(u32),
    MissingRecord(u32),
    Status { hash: u32, status: RecipeStatus },
    NonzeroReferences(u32),
    NoActiveLoads,
    NoActiveUnloads,
}
impl<E: std::fmt::Debug> std::fmt::Display for RecipeRuntimeError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LivingWorld recipe: {self:?}")
    }
}

pub(crate) struct RecipeMan {
    pub(crate) initialization: RecipeManagerInitializationState,
    pub(crate) mappings: RecipeMappingTable,
    pub(crate) queues: RecipeLoadingQueues,
    records: BTreeMap<u32, RecipeInfo>,
    active_loads: u32,
    active_unloads: u32,
    limits: [u32; 5],
}
#[derive(Debug)]
pub(crate) enum RecipeSelectionError {
    Attribute(super::attributes::RecipeAttributeError),
    Random(crate::native_skater_random::rw_math::RwRandomLockError),
}
impl RecipeMan {
    /// Complete non-Juice GetRandomSubModelKey8271afc8, sharing the existing
    /// loaded-only selection loop and the source-global rw::math RNG.
    pub(crate) fn select_model(
        &self,
        parent: u64,
        attributes: &super::attributes::RecipeAttributes<'_>,
    ) -> Result<u64, RecipeSelectionError> {
        use RecipeSelectionError as E;
        if let Some(forced) = attributes.forced_model(parent).map_err(E::Attribute)? {
            return Ok(forced);
        }
        self.select_sub_model(
            parent,
            &mut |key| attributes.filename(key).map_err(E::Attribute),
            &mut || crate::native_skater_random::rw_math::global_next().map_err(E::Random),
        )
    }

    /// GenerateModelAssetID82712b18. Status is deliberately not a gate here;
    /// SpawnVehicle performs the distinct shared ModelMan presence check.
    pub(crate) fn model_asset_id(
        &self,
        key: u64,
        attributes: &super::attributes::RecipeAttributes<'_>,
    ) -> Result<u64, super::attributes::RecipeAttributeError> {
        let selected = attributes.forced_model(key)?.unwrap_or(key);
        Ok(self.asset_id_for_filename(&attributes.filename(selected)?))
    }
    /// IsLivingWorldModelLoaded8270f590 accepts exactly status3. A retained
    /// asset being unloaded is not an eligible model during queue filtering.
    pub(crate) fn is_loaded(&self, filename: &str) -> bool {
        self.records
            .get(&filename_hash(filename))
            .is_some_and(|entry| entry.status == RecipeStatus::Loaded)
    }
    /// IsLivingWorldModelTracked8270f610: the model has a record in any status.
    pub(crate) fn is_tracked(&self, filename: &str) -> bool {
        self.records.contains_key(&filename_hash(filename))
    }
    /// GenerateModelAssetID82712ca4 returns the retained ID irrespective of
    /// status. A caller must still perform its native ModelMan lookup.
    pub(crate) fn asset_id_for_filename(&self, filename: &str) -> u64 {
        self.records
            .get(&filename_hash(filename))
            .map_or(NULL_ASSET_ID, |r| r.asset_id)
    }

    /// Non-forced branch of GetRandomSubModelKey8271b054..2e0. Critically,
    /// loaded-only filtering is active while ANY of the three queues has work;
    /// after all queues empty the entire authored group participates again.
    pub(crate) fn select_sub_model<E>(
        &self,
        parent: u64,
        filename: &mut impl FnMut(u64) -> Result<String, E>,
        random: &mut impl FnMut() -> Result<u32, E>,
    ) -> Result<u64, E> {
        let Some(group) = self.mappings.groups.iter().find(|g| g.group_id == parent) else {
            return Ok(parent);
        };
        let filter = !self.queues.required.is_empty()
            || !self.queues.initial.is_empty()
            || !self.queues.secondary.is_empty();
        let mut candidates = Vec::with_capacity(group.entries.len());
        for entry in &group.entries {
            if !filter || self.is_loaded(&filename(entry.model_id)?) {
                candidates.push(entry.model_id);
            }
        }
        if candidates.is_empty() {
            return Ok(parent);
        }
        // Native consumes Random even when exactly one model is eligible.
        Ok(candidates[(random()? as usize) % candidates.len()])
    }
    pub(crate) fn initialize(
        &mut self,
        attributes: &mut super::attributes::RecipeAttributes<'_>,
    ) -> Result<super::RecipeManagerInitialization, RecipeInitError> {
        super::initialize_recipe_manager(
            &mut self.initialization,
            &mut Initialization {
                mappings: &mut self.mappings,
                queues: &mut self.queues,
                attributes,
            },
        )
    }
    /// ResetLimits82701cf8 reads this array at820c856c, not population caps.
    pub(crate) fn new() -> Self {
        Self {
            initialization: RecipeManagerInitializationState::from_raw(0, 0),
            mappings: RecipeMappingTable::new(),
            queues: RecipeLoadingQueues::default(),
            records: BTreeMap::new(),
            active_loads: 0,
            active_unloads: 0,
            limits: [50, 2, 1, 25, 1],
        }
    }
    /// GameModule::OneOffLoad8273add8 / GameLoad8273be50 wait for the
    /// Required/Initial queues, then the asset loading barrier completes the
    /// last submitted request. Secondary models stay on the in-game cadence.
    pub(crate) fn initial_load_pending(&self) -> bool {
        !self.queues.required.is_empty() || !self.queues.initial.is_empty()
            || self.active_loads != 0
    }
    pub(crate) fn records(&self) -> impl Iterator<Item = (u32, &RecipeInfo)> {
        self.records.iter().map(|(&hash, info)| (hash, info))
    }
    /// GetActiveRecipeCount8270f540 includes entries being unloaded.
    pub(crate) fn active_count(&self, category: ModelCategory) -> u32 {
        self.records
            .values()
            .filter(|r| r.model.category == category)
            .count() as u32
    }
    pub(crate) fn load_key<S: RecipeServices>(
        &mut self,
        key: u64,
        services: &mut S,
    ) -> Result<bool, RecipeRuntimeError<S::Error>> {
        let model = services.model(key).map_err(RecipeRuntimeError::Service)?;
        self.load_model(model, services)
    }
    pub(crate) fn load_model<S: RecipeServices>(
        &mut self,
        model: RecipeModel,
        services: &mut S,
    ) -> Result<bool, RecipeRuntimeError<S::Error>> {
        use RecipeStatus as Status;
        let hash = filename_hash(&model.filename);
        if let Some(info) = self.records.get_mut(&hash) {
            match info.status {
                Status::WaitingForLoad | Status::Loaded => info.count = info.count.wrapping_add(1),
                Status::WaitingForLoadToUnload | Status::LoadedToUnload => {
                    if info.count != 0 {
                        return Err(RecipeRuntimeError::NonzeroReferences(hash));
                    }
                    info.count = 1;
                    info.status = if info.status == Status::WaitingForLoadToUnload {
                        Status::WaitingForLoad
                    } else {
                        Status::Loaded
                    };
                }
                Status::Unloading => return Ok(false),
            }
            return Ok(true);
        }
        if self.active_count(model.category) + 1 > self.limits[model.category as usize] {
            return Ok(false);
        }
        if self.records.len() == 64 {
            return Err(RecipeRuntimeError::RecordCapacity);
        }
        self.records.insert(
            hash,
            RecipeInfo {
                asset_id: NULL_ASSET_ID,
                status: Status::WaitingForLoad,
                count: 1,
                model,
                request_id: u32::MAX,
            },
        );
        self.active_loads = self.active_loads.wrapping_add(1);
        let info = &self.records[&hash];
        let request = services
            .load(
                info.model.category.character_category(),
                &info.model.filename,
            )
            .map_err(RecipeRuntimeError::Service)?;
        self.records
            .get_mut(&hash)
            .expect("inserted recipe")
            .request_id = request;
        Ok(true)
    }
    /// OnLoaded writes the asset/counters/status before emitting SIM then PRES.
    pub(crate) fn loaded<S: RecipeServices>(
        &mut self,
        request: u32,
        asset_id: u64,
        services: &mut S,
    ) -> Result<(), RecipeRuntimeError<S::Error>> {
        use RecipeStatus as Status;
        let (&hash, info) = self
            .records
            .iter_mut()
            .find(|(_, r)| r.request_id == request)
            .ok_or(RecipeRuntimeError::UnknownRequest(request))?;
        info.asset_id = asset_id;
        if self.active_loads == 0 {
            return Err(RecipeRuntimeError::NoActiveLoads);
        }
        self.active_loads -= 1;
        info.status = match info.status {
            Status::WaitingForLoad => Status::Loaded,
            Status::WaitingForLoadToUnload => Status::LoadedToUnload,
            status => return Err(RecipeRuntimeError::Status { hash, status }),
        };
        if asset_id != NULL_ASSET_ID {
            services
                .register_simulation(asset_id, info.model.key)
                .map_err(RecipeRuntimeError::Service)?;
            services
                .register_presentation(asset_id, info.model.key)
                .map_err(RecipeRuntimeError::Service)?;
        }
        Ok(())
    }
    /// UnloadLivingWorldModel(name)82715278 releases one reference. Native Count
    /// is unsigned; retain its decrement order rather than inventing a clamp.
    pub(crate) fn release<S: RecipeServices>(
        &mut self,
        filename: &str,
        services: &mut S,
    ) -> Result<(), RecipeRuntimeError<S::Error>> {
        use RecipeStatus as Status;
        let hash = filename_hash(filename);
        let Some(info) = self.records.get_mut(&hash) else {
            return Ok(());
        };
        info.count = info.count.wrapping_sub(1);
        if info.count == 0 {
            match info.status {
                Status::WaitingForLoad => info.status = Status::WaitingForLoadToUnload,
                Status::Loaded => self.unload_record(hash, services)?,
                Status::Unloading => (),
                status => return Err(RecipeRuntimeError::Status { hash, status }),
            }
        }
        Ok(())
    }
    fn unload_record<S: RecipeServices>(
        &mut self,
        hash: u32,
        services: &mut S,
    ) -> Result<(), RecipeRuntimeError<S::Error>> {
        let info = self
            .records
            .get(&hash)
            .ok_or(RecipeRuntimeError::MissingRecord(hash))?;
        if !matches!(
            info.status,
            RecipeStatus::Loaded | RecipeStatus::LoadedToUnload
        ) {
            return Err(RecipeRuntimeError::Status {
                hash,
                status: info.status,
            });
        }
        let asset = info.asset_id;
        if asset == NULL_ASSET_ID {
            self.records.remove(&hash);
        } else {
            self.active_unloads = self.active_unloads.wrapping_add(1);
            services
                .deregister_simulation(asset)
                .map_err(RecipeRuntimeError::Service)?;
            services
                .deregister_presentation(asset)
                .map_err(RecipeRuntimeError::Service)?;
            let request = services
                .unload(asset)
                .map_err(RecipeRuntimeError::Service)?;
            let info = self
                .records
                .get_mut(&hash)
                .expect("retained unloading recipe");
            info.request_id = request;
            info.status = RecipeStatus::Unloading;
        }
        Ok(())
    }
    pub(crate) fn unloaded<E>(&mut self, request: u32) -> Result<(), RecipeRuntimeError<E>> {
        let (&hash, info) = self
            .records
            .iter()
            .find(|(_, r)| r.request_id == request)
            .ok_or(RecipeRuntimeError::UnknownRequest(request))?;
        if info.status != RecipeStatus::Unloading {
            return Err(RecipeRuntimeError::Status {
                hash,
                status: info.status,
            });
        }
        if self.active_unloads == 0 {
            return Err(RecipeRuntimeError::NoActiveUnloads);
        }
        self.active_unloads -= 1;
        self.records.remove(&hash);
        Ok(())
    }
    pub(crate) fn load_all<S: RecipeServices>(
        &mut self,
        queue: RecipeLoadingQueueKind,
        services: &mut S,
    ) -> Result<(), RecipeRuntimeError<S::Error>> {
        loop {
            let next = match queue {
                RecipeLoadingQueueKind::Required => self.queues.required.pop(),
                RecipeLoadingQueueKind::Initial => self.queues.initial.pop(),
                RecipeLoadingQueueKind::Secondary => self.queues.secondary.pop(),
            };
            let Some(key) = next else {
                return Ok(());
            };
            self.load_key(key, services)?;
        }
    }
    pub(crate) fn update<S: RecipeServices>(
        &mut self,
        services: &mut S,
    ) -> Result<(), RecipeRuntimeError<S::Error>> {
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if *TRACE.get_or_init(||std::env::var_os("SKATE2_TRACE_PEDESTRIANS").is_some()) {
            let n=CALLS.fetch_add(1,std::sync::atomic::Ordering::Relaxed);
            if n % 300 == 0 {
                std::eprintln!("pedestrian recipe call={n} initialized={} delay={} loads={} queues={:?} records={:?}",
                    self.initialization.initialized_raw,self.initialization.frame_delay_raw,self.active_loads,
                    [self.queues.required.len(),self.queues.initial.len(),self.queues.secondary.len()],
                    self.records.values().map(|r|(&r.model.filename,r.model.category,r.status,r.asset_id)).collect::<Vec<_>>());
            }
        }
        if self.initialization.initialized_raw == 0 {
            return Ok(());
        }
        // Native ordered map iteration; each unload may erase its own entry.
        let pending = self
            .records
            .iter()
            .filter_map(|(&hash, r)| (r.status == RecipeStatus::LoadedToUnload).then_some(hash))
            .collect::<Vec<_>>();
        for hash in pending {
            self.unload_record(hash, services)?;
        }
        if self.active_loads == 0 {
            if self.initialization.frame_delay_raw == 0 {
                let next = self
                    .queues
                    .required
                    .pop()
                    .or_else(|| self.queues.initial.pop())
                    .or_else(|| self.queues.secondary.pop());
                if let Some(key) = next {
                    self.load_key(key, services)?;
                }
                self.initialization.frame_delay_raw = 120;
            } else {
                self.initialization.frame_delay_raw -= 1;
            }
        }
        Ok(())
    }
}
fn filename_hash(filename: &str) -> u32 {
    sk8_hash_value_32(filename.to_ascii_lowercase().as_bytes())
}

#[cfg(test)]
#[path = "living_world_recipe_runtime_tests.rs"]
mod tests;
