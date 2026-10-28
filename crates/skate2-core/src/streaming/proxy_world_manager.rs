//! Proxy world: `cProxyWorldManager` and the Presentation registrations it
//! drives for the low-detail city (debug `sk82_na_m.xex`).
//!
//! Boot (`GameModule::OneOffLoad`, `kEnableProxyWorldStream` default true):
//! - `SyncLoadProxyWorldStream` 0x824df648 reads the PROXY stream's XSS
//!   (count at +16, 80-byte rows from +24), keys every row by
//!   `tProxySort::mOrder`, stable-sorts ascending (`merge_sort_buffer` with
//!   `ProxyCollectionOrderCompare` 0x824d6888, signed `<`), then per sorted row
//!   inserts the ID into `mProxyCollectionMap`, calls `LoadCollection` and
//!   appends it to `mProxyCollectionID`.
//! - `SyncActivateProxyWorld` 0x824d9ef8 sends, per `mProxyCollectionID` entry
//!   in order, `cMsgAssetPresRegCollection` (userdata 0, activate) and its
//!   callback, then waits for every activation.
//! - `cStreamFile::Update` 0x824e0ba8 calls `AddHighCollection` 0x824dc678 for
//!   every Presentation XSS row with LOD byte 0 that is not already valid.
//!
//! Runtime, per high collection:
//! - `cAssetStreamSystem::LoadComplete` 0x824dc208 passes
//!   `GetProxyCollection` 0x824daef8 to `NotifyCollectionReg`;
//!   `NotifyCollectionPresReg` 0x82d5c680 registers the high collection and
//!   then deregisters its proxy in the same message batch. The deferred path
//!   (`UpdatePresCollectionDefer` 0x82d5b5f0) sends that proxy deregistration
//!   only after the high collection's last arena.
//! - `UnloadComplete` 0x824dc3e8 → `NotifyCollectionPresDReg` 0x82d580d0
//!   deregisters the high collection, then registers its proxy (activate).
//! - `PresThreadMsgMan::ReceiveRegCollectionMsg` 0x82780958 and
//!   `ReceiveDRegCollectionMsg` 0x8277fff8 walk the collection's arenas through
//!   one `cAssetRefCounter`: only first use reaches `Presentation::RegArena`
//!   0x827f1430, only last use `DRegArena` 0x827f1680.

use crate::{
    arena_registration_ref_counter::{
        ArenaRegistrationRefCounter, ArenaRegistrationRefCounterError,
    },
    collection_resident_streamer::CollectionId,
};
use skate2_formats::{
    hash::rw_hash64_string,
    stream::{CollectionSpace, CollectionSpaceEntry},
};
use std::{
    collections::{BTreeMap, btree_map::Entry},
    error::Error,
    fmt,
};

/// `tProxySort::mOrder` of one PROXY XSS row
/// (`SyncLoadProxyWorldStream` 0x824df878..0x824df9d0): the hull centre
/// `min + (max - min) * 0.5`, `x`/`z` = `floor(centre + 0.5)`, then
/// `z / 10 + 10000 * x` in signed 32-bit arithmetic.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // fctiwz of an already floored value.
pub fn proxy_sort_order(row: &CollectionSpaceEntry) -> i32 {
    let centre =
        |axis: usize| (row.hull_max[axis] - row.hull_min[axis]).mul_add(0.5, row.hull_min[axis]);
    let x = (centre(0) + 0.5).floor() as i32;
    let z = (centre(2) + 0.5).floor() as i32;
    x.wrapping_mul(10_000).wrapping_add(z / 10)
}

/// `cProxyWorldManager`'s maps once `SyncLoadProxyWorldStream` has read the
/// PROXY stream's XSS.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProxyWorldManager {
    /// `mProxyCollectionID`: every PROXY row in native order.
    collection_ids: Vec<u64>,
    /// `mProxyCollectionMap`: proxy ID to its position in `collection_ids`
    /// (the native value is an unused zero).
    collection_map: BTreeMap<u64, usize>,
    /// `mProxyInfoMap`: high collection to proxy collection.
    info_map: BTreeMap<u64, u64>,
}

impl ProxyWorldManager {
    #[must_use]
    pub fn from_collection_space(space: &CollectionSpace) -> Self {
        let mut rows = space
            .collections
            .iter()
            .map(|row| (proxy_sort_order(row), row.id))
            .collect::<Vec<_>>();
        // `merge_sort_buffer` is stable: equal keys keep XSS order.
        rows.sort_by_key(|&(order, _)| order);
        let collection_ids = rows.into_iter().map(|(_, id)| id).collect::<Vec<_>>();
        Self {
            collection_map: collection_ids
                .iter()
                .enumerate()
                .map(|(index, &id)| (id, index))
                .collect(),
            collection_ids,
            info_map: BTreeMap::new(),
        }
    }

    /// `mProxyCollectionID`: load and activation order.
    #[must_use]
    pub fn collection_ids(&self) -> &[u64] {
        &self.collection_ids
    }

    /// `AddHighCollection` 0x824dc678: `sprintf("cPres_%d_%d_high_proxy",
    /// x, z)` (the 32-bit lanes print signed), `rw::RwHash64String`, and an
    /// `mProxyInfoMap` insert only when `mProxyCollectionMap` holds the hash.
    /// Returns the proxy only when this call inserted it; `DoInsertValue`
    /// leaves an existing high collection's entry unchanged.
    pub fn add_high_collection(&mut self, high: u64, x: u32, z: u32) -> Option<u64> {
        let name = format!(
            "cPres_{}_{}_high_proxy",
            i32::from_ne_bytes(x.to_ne_bytes()),
            i32::from_ne_bytes(z.to_ne_bytes())
        );
        let proxy = rw_hash64_string(&name);
        if !self.collection_map.contains_key(&proxy) {
            return None;
        }
        match self.info_map.entry(high) {
            Entry::Vacant(entry) => Some(*entry.insert(proxy)),
            Entry::Occupied(_) => None,
        }
    }

    /// `GetProxyCollection` 0x824daef8: the mapped proxy, or none for the
    /// invalid collection ID.
    #[must_use]
    pub fn proxy_collection(&self, high: u64) -> Option<u64> {
        self.info_map.get(&high).copied()
    }
}

/// The proxy collections' Presentation registration: each loaded proxy's
/// arena IDs (container order) under one arena ref-counter, driven by
/// activation and by the high collections' registrations.
#[derive(Debug, Default)]
pub struct ProxyWorld {
    manager: ProxyWorldManager,
    /// Loaded proxies' arena IDs; none until `SyncActivateProxyWorld`.
    containers: Option<BTreeMap<u64, Vec<u64>>>,
    arenas: ArenaRegistrationRefCounter,
    /// Registered high collections and the proxy each one deregistered.
    high: BTreeMap<u64, Option<u64>>,
    /// Advances with every proxy registration change.
    revision: u64,
}

impl ProxyWorld {
    #[must_use]
    pub fn new(manager: ProxyWorldManager) -> Self {
        Self {
            manager,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn manager(&self) -> &ProxyWorldManager {
        &self.manager
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// `SyncActivateProxyWorld` 0x824d9ef8: register every proxy in
    /// `mProxyCollectionID` order. `containers` holds each loaded proxy
    /// collection's arena IDs in its container registry order.
    ///
    /// The original activates before the world stream starts, so no high
    /// collection is registered yet. A high collection the port's renderer
    /// registered first has its proxy deregistered here, the state the
    /// original reaches.
    ///
    /// # Errors
    ///
    /// Fails when already active or when a proxy collection has no arenas.
    pub fn activate(&mut self, containers: BTreeMap<u64, Vec<u64>>) -> Result<(), ProxyWorldError> {
        if self.containers.is_some() {
            return Err(ProxyWorldError::AlreadyActive);
        }
        if let Some(&missing) = self
            .manager
            .collection_ids
            .iter()
            .find(|id| !containers.contains_key(*id))
        {
            return Err(ProxyWorldError::MissingCollection(missing));
        }
        self.containers = Some(containers);
        for index in 0..self.manager.collection_ids.len() {
            self.register(self.manager.collection_ids[index])?;
        }
        let waiting = self
            .high
            .iter()
            .filter(|(_, hidden)| hidden.is_none())
            .map(|(&high, _)| high)
            .collect::<Vec<_>>();
        for high in waiting {
            self.hide_proxy_of(high)?;
        }
        Ok(())
    }

    /// `NotifyCollectionPresReg`'s proxy half: once `high` registers, its
    /// proxy (if any) deregisters.
    ///
    /// # Errors
    ///
    /// Fails when `high` is already registered or a count is inconsistent.
    pub fn high_registered(&mut self, high: u64) -> Result<(), ProxyWorldError> {
        if self.high.insert(high, None).is_some() {
            return Err(ProxyWorldError::HighAlreadyRegistered(high));
        }
        self.hide_proxy_of(high)
    }

    /// `NotifyCollectionPresDReg`'s proxy half: once `high` deregisters, the
    /// proxy it deregistered registers again.
    ///
    /// # Errors
    ///
    /// Fails when `high` is not registered or a count is inconsistent.
    pub fn high_deregistered(&mut self, high: u64) -> Result<(), ProxyWorldError> {
        let hidden = self
            .high
            .remove(&high)
            .ok_or(ProxyWorldError::HighNotRegistered(high))?;
        if let Some(proxy) = hidden {
            self.register(proxy)?;
        }
        Ok(())
    }

    /// `AddHighCollection` 0x824dc678. The original makes every mapping
    /// before a high collection can register; when the port's renderer has
    /// already registered `high`, its proxy deregisters now, the state the
    /// original reaches.
    ///
    /// # Errors
    ///
    /// Fails when a registration count is inconsistent.
    pub fn add_high_collection(
        &mut self,
        high: u64,
        x: u32,
        z: u32,
    ) -> Result<(), ProxyWorldError> {
        if self.manager.add_high_collection(high, x, z).is_some()
            && self.high.get(&high) == Some(&None)
        {
            self.hide_proxy_of(high)?;
        }
        Ok(())
    }

    /// Whether `arena` is registered with Presentation (`RegArena` ran).
    #[must_use]
    pub fn is_arena_registered(&self, arena: u64) -> bool {
        self.arenas.count(CollectionId(arena)).is_some()
    }

    fn hide_proxy_of(&mut self, high: u64) -> Result<(), ProxyWorldError> {
        if self.containers.is_none() {
            return Ok(());
        }
        let Some(proxy) = self.manager.proxy_collection(high) else {
            return Ok(());
        };
        self.deregister(proxy)?;
        self.high.insert(high, Some(proxy));
        Ok(())
    }

    /// `ReceiveRegCollectionMsg` with `fActivate`: `AddAsset` per arena.
    fn register(&mut self, collection: u64) -> Result<(), ProxyWorldError> {
        let arenas = self
            .containers
            .as_ref()
            .and_then(|containers| containers.get(&collection))
            .ok_or(ProxyWorldError::MissingCollection(collection))?;
        for &arena in arenas {
            self.arenas.add_asset(CollectionId(arena));
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// `ReceiveDRegCollectionMsg`: `RemAsset` per arena.
    fn deregister(&mut self, collection: u64) -> Result<(), ProxyWorldError> {
        let arenas = self
            .containers
            .as_ref()
            .and_then(|containers| containers.get(&collection))
            .ok_or(ProxyWorldError::MissingCollection(collection))?;
        for &arena in arenas {
            self.arenas
                .remove_asset(CollectionId(arena))
                .map_err(ProxyWorldError::Registration)?;
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProxyWorldError {
    AlreadyActive,
    MissingCollection(u64),
    HighAlreadyRegistered(u64),
    HighNotRegistered(u64),
    Registration(ArenaRegistrationRefCounterError),
}

impl fmt::Display for ProxyWorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyActive => formatter.write_str("proxy world is already active"),
            Self::MissingCollection(id) => {
                write!(
                    formatter,
                    "proxy collection {id:#018x} has no loaded arenas"
                )
            }
            Self::HighAlreadyRegistered(id) => {
                write!(
                    formatter,
                    "high collection {id:#018x} is already registered"
                )
            }
            Self::HighNotRegistered(id) => {
                write!(formatter, "high collection {id:#018x} is not registered")
            }
            Self::Registration(error) => write!(formatter, "proxy registration: {error}"),
        }
    }
}

impl Error for ProxyWorldError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u64, centre_x: f32, centre_z: f32) -> CollectionSpaceEntry {
        CollectionSpaceEntry {
            id,
            lod: 0,
            hull_min: [centre_x - 100.0, -50.0, centre_z - 100.0, 0.0],
            hull_max: [centre_x + 100.0, 50.0, centre_z + 100.0, 0.0],
            geometry_min: [0.0; 4],
            geometry_max: [0.0; 4],
        }
    }

    fn space(rows: Vec<CollectionSpaceEntry>) -> CollectionSpace {
        CollectionSpace {
            version: 0,
            stamp: 0,
            collections: rows,
        }
    }

    fn proxy_id(x: i32, z: i32) -> u64 {
        rw_hash64_string(&format!("cPres_{x}_{z}_high_proxy"))
    }

    fn lanes(value: i32) -> u32 {
        u32::from_ne_bytes(value.to_ne_bytes())
    }

    #[test]
    fn order_key_truncates_z_toward_zero_and_sorts_stably() {
        assert_eq!(proxy_sort_order(&row(1, -500.0, -100.0)), -5_000_010);
        assert_eq!(proxy_sort_order(&row(1, -300.0, 105.0)), -2_999_990);
        // floor(centre + 0.5) rounds -0.5 up to 0 and 99.5 up to 100.
        assert_eq!(proxy_sort_order(&row(1, -0.5, 99.5)), 10);
        let manager = ProxyWorldManager::from_collection_space(&space(vec![
            row(3, 100.0, 100.0),
            row(1, -300.0, 100.0),
            row(2, 100.0, 104.0),
        ]));
        // 3 and 2 share key 1_000_010 and keep XSS order.
        assert_eq!(manager.collection_ids(), [1, 3, 2]);
    }

    #[test]
    fn the_shipped_proxy_name_hashes_to_its_collection_id() {
        // PROXY_Pres.xss's first sorted row (worldproxy.big).
        assert_eq!(proxy_id(-500, -100), 0xb2ad_1741_5ee8_147a);
    }

    #[test]
    fn add_high_collection_maps_only_listed_proxies_once() {
        let listed = proxy_id(-500, -100);
        let mut manager =
            ProxyWorldManager::from_collection_space(&space(vec![row(listed, -500.0, -100.0)]));
        assert_eq!(
            manager.add_high_collection(7, lanes(-500), lanes(-100)),
            Some(listed)
        );
        assert_eq!(manager.proxy_collection(7), Some(listed));
        assert_eq!(
            manager.add_high_collection(7, lanes(-500), lanes(-100)),
            None
        );
        assert_eq!(
            manager.add_high_collection(8, lanes(-300), lanes(-100)),
            None
        );
        assert_eq!(manager.proxy_collection(8), None);
    }

    fn world() -> (ProxyWorld, u64, u64) {
        let a = proxy_id(-500, -100);
        let b = proxy_id(-300, -100);
        let mut world = ProxyWorld::new(ProxyWorldManager::from_collection_space(&space(vec![
            row(a, -500.0, -100.0),
            row(b, -300.0, -100.0),
        ])));
        world
            .add_high_collection(70, lanes(-500), lanes(-100))
            .unwrap();
        world
            .add_high_collection(71, lanes(-300), lanes(-100))
            .unwrap();
        (world, a, b)
    }

    fn containers(a: u64, b: u64) -> BTreeMap<u64, Vec<u64>> {
        // Arena 0x30 is a texture both proxies carry.
        BTreeMap::from([(a, vec![0x10, 0x30]), (b, vec![0x20, 0x30])])
    }

    #[test]
    fn high_registration_hides_and_restores_only_its_proxy() {
        let (mut world, a, b) = world();
        world.activate(containers(a, b)).unwrap();
        assert!(world.is_arena_registered(0x10) && world.is_arena_registered(0x20));
        world.high_registered(70).unwrap();
        assert!(!world.is_arena_registered(0x10));
        assert!(world.is_arena_registered(0x20) && world.is_arena_registered(0x30));
        world.high_registered(71).unwrap();
        assert!(!world.is_arena_registered(0x20) && !world.is_arena_registered(0x30));
        world.high_deregistered(70).unwrap();
        assert!(world.is_arena_registered(0x10) && world.is_arena_registered(0x30));
        assert!(!world.is_arena_registered(0x20));
        assert_eq!(
            world.high_deregistered(70),
            Err(ProxyWorldError::HighNotRegistered(70))
        );
    }

    #[test]
    fn a_high_collection_without_a_proxy_changes_nothing() {
        let (mut world, a, b) = world();
        world.activate(containers(a, b)).unwrap();
        let revision = world.revision();
        world.high_registered(99).unwrap();
        world.high_deregistered(99).unwrap();
        assert_eq!(world.revision(), revision);
    }

    #[test]
    fn a_mapping_after_registration_reaches_the_original_state() {
        let a = proxy_id(-500, -100);
        let mut world =
            ProxyWorld::new(ProxyWorldManager::from_collection_space(&space(vec![row(
                a, -500.0, -100.0,
            )])));
        // Registered before activation and before its mapping.
        world.high_registered(70).unwrap();
        world.activate(BTreeMap::from([(a, vec![0x10])])).unwrap();
        assert!(world.is_arena_registered(0x10));
        world
            .add_high_collection(70, lanes(-500), lanes(-100))
            .unwrap();
        assert!(!world.is_arena_registered(0x10));
        world.high_deregistered(70).unwrap();
        assert!(world.is_arena_registered(0x10));
    }

    #[test]
    fn activation_hides_proxies_of_already_registered_high_collections() {
        let (mut world, a, b) = world();
        world.high_registered(71).unwrap();
        world.activate(containers(a, b)).unwrap();
        assert!(world.is_arena_registered(0x10));
        assert!(!world.is_arena_registered(0x20));
        assert_eq!(
            world.activate(containers(a, b)),
            Err(ProxyWorldError::AlreadyActive)
        );
    }
}
