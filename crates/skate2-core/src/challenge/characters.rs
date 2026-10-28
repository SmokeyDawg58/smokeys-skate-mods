//! `Sk8::Challenge::cChallengeCharacterManager` (0x7430 bytes): the pedestrians a challenge
//! places in the world while it is available, such as Slappy outside his shop or Reda at the
//! flip-trick locator.
//!
//! An activity whose `challenges` data lists characters (`tGlobalSpawnCharacterInfo[]`, key
//! 0x555c505a36c3353a: an entity at a location) or waypoint characters
//! (`tSpawnLivingWorldOnWaypointInfo[]`, key 0x0cbe4b5ee9580e8d: an entity on a waypoint
//! group) gets one `tLivingWorldSpawnData` per entry once its prerequisites pass
//! (`IgnorePrereqForLivingWorld`). Each entry's box (`GenerateLivingWorldSpawnActivityBBox`)
//! sits in a managed oct tree; every update the local tester's box is tested against it
//! (`TestBBoxAgainstTree`): an entry the box enters gets a ground line test, its model and then
//! a forced census spawn, an entry it leaves is released.
//!
//! The living world's managers are singletons the native calls directly. Here their reads go
//! through [`LivingWorldFacts`], installed by the session, and every change of the living world
//! is a [`LivingWorldOp`] the session applies in call order after the challenge update and the
//! flush. A spawn's pedestrian is [`CharacterRef::Pending`] until the session answers
//! ([`spawn_answered`]); later operations on it name the same token.

use super::{
    keys::{class, hash64},
    signup,
    system::World,
};
use crate::grind_data_octree::OctreeBounds;
use std::collections::BTreeMap;

/// The attributes the manager reads.
pub(crate) mod key {
    /// `challenges` 0x555c505a36c3353a: `tGlobalSpawnCharacterInfo[10]`.
    pub(crate) const CHARACTERS: u64 = 0x555c_505a_36c3_353a;
    /// `challenges` 0x0cbe4b5ee9580e8d: `tSpawnLivingWorldOnWaypointInfo[10]`.
    pub(crate) const WAYPOINT_CHARACTERS: u64 = 0x0cbe_4b5e_e958_0e8d;
    /// `challenges` 0x16e7e79b28332502 (`Bool`): while the challenge runs, no other activity's
    /// characters may spawn (`IsAllowedToSpawnWithCurrentChallengeState`).
    pub(crate) const HIDES_OTHER_CHARACTERS: u64 = 0x16e7_e79b_2833_2502;
    /// `livingworld_models` 0xf6897cc7b637850c: `eLivingWorldModelCategory`.
    pub(crate) const MODEL_CATEGORY: u64 = 0xf689_7cc7_b637_850c;
}

/// `livingworld_entities`.
pub(crate) const LIVINGWORLD_ENTITIES: u64 = 0x60d7_03c7_7cab_1631;
/// `livingworld_models`.
pub(crate) const LIVINGWORLD_MODELS: u64 = 0x4e74_0a66_70f2_41a7;
/// `livingworld_census_ranges`.
const LIVINGWORLD_CENSUS_RANGES: u64 = 0xc779_d2e2_1a4f_4f35;
/// `eLivingWorldModelCategory` 1, the marquee models `RecipeMan` loads on request.
pub(crate) const MARQUEE_CATEGORY: u32 = 1;

/// `eChallengeEntityOnStartType`: what a character does when its own challenge starts.
pub(crate) mod on_start {
    pub(crate) const REMOVE: u32 = 0;
    pub(crate) const KEEP: u32 = 1;
    pub(crate) const ALTERNATE_LOCATION: u32 = 2;
}

/// `CommonBackEnd::GM_ProfileFreeSkate`.
const GM_PROFILE_FREE_SKATE: i32 = 3;
/// `AddEntityToLineTestBatch` takes at most eight tests in its batch.
const LINE_BATCH_CAPACITY: usize = 8;
/// `EPSILON_8` (`0x34000000`): `TestBBoxAgainstTree`'s zero test of a position.
const ZERO_EPSILON: f32 = f32::from_bits(0x3400_0000);
/// `kGenerateLivingWorldSpawnActivityBBox`'s widening of a marquee entity's box.
const MARQUEE_BOX_PADDING: f32 = 10.0;
/// `TriggerVolumeUtils::kTriggerAABBCharacterWidth`/`Height`.
const TRIGGER_AABB_CHARACTER_WIDTH: f32 = 0.58;
const TRIGGER_AABB_CHARACTER_HEIGHT: f32 = 1.74;

/// The challenge cvars the manager reads, at their initializers' values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    /// `kEnableChallengeSpawn` (0x82f55128): true.
    pub enable_spawn: bool,
    /// `kKillPrereqFailedCharacters` (0x82f55178): true; a released character is culled at once
    /// rather than left to the census.
    pub kill_prereq_failed: bool,
    /// `kChallengeSpawnLineTestFatness` (0x82f55218): 0.33.
    pub line_test_fatness: f32,
    /// `kChallengeSpawnLineTestOffsetsUp` (0x82f55278): 1.0.
    pub line_test_up: f32,
    /// `kChallengeSpawnLineTestOffsetsDown` (0x82f552d8): 2.5.
    pub line_test_down: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            enable_spawn: true,
            kill_prereq_failed: true,
            line_test_fatness: f32::from_bits(0x3ea8_f5c3),
            line_test_up: 1.0,
            line_test_down: 2.5,
        }
    }
}

/// The source of a `tLivingWorldSpawnData`: its `LocationInfo` or its `WaypointInfo`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnInfo {
    /// `tGlobalSpawnCharacterInfo`: `LivingWorldEntity` (class, collection), `Location`,
    /// `OnStartType` and `AlternateLocation`.
    Location {
        entity: (u64, u64),
        location: Option<String>,
        on_start: u32,
        alternate: Option<String>,
    },
    /// `tSpawnLivingWorldOnWaypointInfo`: `LivingWorldEntity`, `Waypoint.WaypointID` and
    /// `OnStartType`.
    Waypoint {
        entity: (u64, u64),
        waypoint: u64,
        on_start: u32,
    },
}

impl SpawnInfo {
    /// `LivingWorldEntity`.
    #[must_use]
    pub const fn entity(&self) -> (u64, u64) {
        match self {
            Self::Location { entity, .. } | Self::Waypoint { entity, .. } => *entity,
        }
    }
}

/// A character's pedestrian: spawned, or awaiting the session's answer to the spawn of the
/// token. Pending pedestrians order after every spawned one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterRef {
    Spawned(u32),
    Pending(u32),
}

/// `tLivingWorldSpawnData` (64 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnData {
    /// `Activity`.
    pub activity: usize,
    /// `ActivityCharacterIndex`: the location entries, then the waypoint entries.
    pub character_index: i32,
    pub info: SpawnInfo,
    /// `UseAlternateLocation`.
    pub use_alternate_location: bool,
    /// `RequestedModelLoad`; while `model_load_token` is set it holds the value the pending
    /// spawn assumes.
    pub requested_model_load: bool,
    model_load_token: Option<u32>,
    /// `EntityID`.
    pub entity: Option<CharacterRef>,
    /// `RecipeID` (`assman::kInvalidAssetID` without a pedestrian).
    pub recipe: u64,
    /// `LineTestBatchId`.
    pub line_test: i32,
    /// `SpawnPosition`, zero until the ground line test answers.
    pub spawn_position: [f32; 4],
}

impl SpawnData {
    fn new(activity: usize, character_index: i32, info: SpawnInfo) -> Self {
        Self {
            activity,
            character_index,
            info,
            use_alternate_location: false,
            requested_model_load: false,
            model_load_token: None,
            entity: None,
            recipe: crate::living_world_grind_services::dmo_models::INVALID_ASSET_ID,
            line_test: -1,
            spawn_position: [0.0; 4],
        }
    }

    fn same(&self, activity: usize, character_index: i32) -> bool {
        self.activity == activity && self.character_index == character_index
    }
}

/// A ground line test's answer: `LineBatch::GetResult`, `None` without a hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub position: [f32; 4],
    /// The hit assembly's owner (`LWCollideableObjectMan::GetAssemblyOwnerID`); `None` for the
    /// static world.
    pub owner: Option<u32>,
}

/// `PhysicsLineTestBatch`: acquired by the first test of an update, submitted at its end, read
/// and released by the next `TestBBoxAgainstTree`.
#[derive(Clone, Debug, Default)]
struct LineBatch {
    /// Which submission the session's answer belongs to.
    serial: u32,
    tests: Vec<([f32; 4], [f32; 4], f32)>,
    results: Option<Vec<Option<LineHit>>>,
}

/// A change of the living world, applied by the session in call order.
#[derive(Clone, Debug, PartialEq)]
pub enum LivingWorldOp {
    /// Manager Spawn/UnspawnBigBlack handlers82719598/8271f618.
    BigBlack(bool),
    /// `TestBBoxAgainstTree`'s spawn of an entry the tester's box holds, its ground found:
    /// with `load_model`, first `RecipeMan::LoadLivingWorldModel(model)`, whose refusal ends the
    /// spawn; then, when `spawn` (`kEnableChallengeSpawn`), `ILWCensusMan::SpawnPedestrian`
    /// (vtable +4) of the entity at the transform with `initial` false, `allowCull` false,
    /// `force` true, `allowWorldCollisions` true, `ignoreNavMeshInGroundTest` true and
    /// `allowHandProp` true. A spawned pedestrian then gets its `mChallengeOwner` (used by
    /// ChaseMan's service capping and authored respawn), the waypoint group's first
    /// waypoint when `waypoint` is set (`Pedestrian::SetCurrentWaypoint`), and with
    /// `challenge_flags` `mbProChallengeStarted`, `mbPhotographerTakingPictures` set and
    /// `mbPhotographerReviewingPictures` cleared. The session answers with [`spawn_answered`].
    Spawn {
        token: u32,
        entity: u64,
        transform: [[f32; 4]; 4],
        model: u64,
        load_model: bool,
        spawn: bool,
        waypoint: Option<u64>,
        challenge_flags: bool,
    },
    /// `Pedestrian::RequestExplicitCull` (`INavEntity` vtable +36) of a pedestrian, if it exists.
    RequestExplicitCull(CharacterRef),
    /// `INavEntity::SetAllowCull(true)` of a pedestrian, if it exists.
    AllowCull(CharacterRef),
    /// Bits of a pedestrian's `tPedestrianStateStorage` bytes 0xca8/0xca9 (`mbPhotographer*`,
    /// `mbProChallengeStarted`): `set` then `clear` per byte, if the pedestrian exists.
    StateFlags {
        character: CharacterRef,
        set: [u8; 2],
        clear: [u8; 2],
    },
    /// `Pedestrian::SetCurrentWaypoint` to the first waypoint of the group, if both exist.
    SetCurrentWaypoint {
        character: CharacterRef,
        waypoint: u64,
    },
    /// `RecipeMan::UnloadLivingWorldModel(model)`; after a pending spawn's model load (`after`),
    /// only if that load was taken.
    UnloadModel { model: u64, after: Option<u32> },
    /// The submitted `PhysicsLineTestBatch` (`AcquireLineBatch("cChallengeCharacterManager",
    /// BF_GroundDmo)`): start, end and fatness per test. The session answers with
    /// [`line_tests_answered`] and the serial.
    LineTests {
        serial: u32,
        tests: Vec<([f32; 4], [f32; 4], f32)>,
    },
    /// A crowd generator's batch (`cCrowdPositionAndGenerationState::SubmitLineTests`:
    /// `AcquireLineBatch("cCrowdPositionAndGenerationState::SubmitLineTests")`, `BF_Default`):
    /// start, end and fatness per test, added until the batch refuses one. The session answers
    /// with [`super::crowd::line_tests_answered`] and the serial.
    CrowdLineTests {
        serial: u32,
        tests: Vec<([f32; 4], [f32; 4], f32)>,
    },
    /// `LivingWorld.ActivateSpectatorWaypoint`'s `WaypointMan::ActivateSpectatorWaypoint`
    /// (`0x82ce9180`) of the waypoint group the GUID finds, when it has waypoints: its
    /// `RequestCensusManagedCull()` (`keep` the challenge characters at the call), then the
    /// group's forced spectators queued, or the group activated when none are forced.
    ActivateSpectatorWaypoint {
        guid: u64,
        force_spawn_percentage: f32,
        category_group: u64,
        keep: Vec<CharacterRef>,
    },
    /// `WaypointMan::DeactivateSpectatorWaypoint(group, true)` (`0x82ce8ed8`) of the group the
    /// GUID finds: its queued activation dropped and its spectators culled.
    DeactivateSpectatorWaypoint { guid: u64 },
    /// `LWDynamicObjectMan::CanResetDynamicObject`, then `ResetDynamicObject`, of the dynamic
    /// object owning a line test's hit.
    ResetDynamicObject(u32),
    /// `cChallengeLivingWorldManager::SetLWManagementValues`: the census enable byte's bits
    /// 0x40 (census spawner), 0x20 (pedestrians), 0x10 (prop-driven pedestrians), 0x08
    /// (vehicles), 0x04 (dynamic objects) and 0x02 (props) under `mask`.
    CensusEnable { mask: u8, value: u8 },
    /// `ILWManager::RequestCensusManagedCull()` (vtable +0x34): the census-managed pedestrians
    /// and vehicles. `keep` are the challenge characters at the call, which are not census
    /// managed (`Pedestrian::IsCensusManaged`, `IsPedestrianInvolvedInChallenges`).
    CensusManagedCullAll { keep: Vec<CharacterRef> },
    /// `ILWManager::RequestCensusManagedCull(volume)` (vtable +0x30) of the trigger instance
    /// `cTriggerVolumeManager::GetTriggerInstance` finds for the volume ID (as a float): those
    /// intersecting it.
    CensusManagedCullVolume {
        volume: u64,
        keep: Vec<CharacterRef>,
    },
    /// `AvoidVolumeList` gained a volume ID.
    AvoidVolumeAdded(u64),
    /// `AvoidVolumeList` was cleared.
    AvoidVolumesCleared,
    /// `cChallengeDMOManager::RunDMOOperations`' `tDMOResetInfo` on the trigger instance
    /// `GetTriggerInstance` finds for the volume ID (as a float), if any: Reset (0) culls the
    /// dynamic objects intersecting it (`RequestCullWithinTriggerVolume`) and resets its locators
    /// (`ResetLocatorsInTriggerVolume`); Clear (1) culls them and disables its locators
    /// (`SetLocatorsInTriggerVolumeEnabled(false)`); Save (2) first saves its locators in the DMO
    /// manager under the trigger's GUID (`SerializeDMOs`), then clears. Other types do nothing.
    DmoReset { kind: u32, volume: u64 },
    /// `RunDMOOperations`' `AddStreamedLocatorData` of each `ChallengeDMOGroups` asset: flags 0,
    /// no persistence, no priority tracking, static as the challenge's `DmoTreatAsStatic`.
    AddChallengeDmos { treat_as_static: bool },
    /// `UnwindDMOOperations`' undo of a Clear (1: the volume's locators enabled again) or a Save
    /// (2: the saved locators restored, `RestoreFromSavedState`, and forgotten).
    UnwindDmoReset { kind: u32, volume: u64 },
    /// `ResetChallengeDMOs` (0x825b06a0): `ResetStreamedLocatorData` of each `ChallengeDMOGroups`
    /// asset.
    ResetChallengeDmos,
    /// The dynamic object census range: `SetDynamicObjectsCensusRangeKey` (0x8271c088), or
    /// `ResetDynamicObjectsCensusRangeKey` (0x8271c138, "dynamicobjects") for `None`.
    DynamicObjectsCensusRange(Option<u64>),
}

/// The living world's answers the manager reads while the challenge system runs (the
/// `LW::Man` and `RecipeMan` singletons), installed by the session.
pub trait LivingWorldFacts: Send {
    /// `ILWManager::GetWaypointByGUID` (0x82715b18), then the group's first waypoint's
    /// `GetLocation`: `None` without the group.
    fn waypoint_location(&self, guid: u64) -> Option<[f32; 4]>;
    /// `RecipeMan::IsLivingWorldModelTracked` (0x82712aa8) of a `livingworld_models` key.
    fn is_model_tracked(&self, model: u64) -> bool;
    /// `ChaseMan::ChaseIsOn` (0x826f3b48).
    fn chase_is_on(&self) -> bool;
    /// MapIconManager826187b8 reads GetBigBlackEntity826f5598, then its position.
    fn big_black_position(&self) -> Option<[f32;4]> { None }
}

/// `cManagedOctTree<tLivingWorldSpawnData, 256>`: the entries up to `EntryFillIndex` (a removed
/// one holds `InvalidEntry`) with their boxes, and `FreeDataEntryIndexices`.
#[derive(Debug, Default)]
struct SpawnTree {
    entries: Vec<Option<(SpawnData, OctreeBounds)>>,
    free: Vec<u32>,
}

impl SpawnTree {
    /// `Insert` (0x825a49b8): the most recently freed slot, else the next.
    fn insert(&mut self, data: SpawnData, bounds: OctreeBounds) -> u32 {
        if let Some(index) = self.free.pop() {
            self.entries[index as usize] = Some((data, bounds));
            index
        } else {
            let index = u32::try_from(self.entries.len()).expect("256 entries");
            self.entries.push(Some((data, bounds)));
            index
        }
    }

    /// `HasEntry` (0x825a3238) / `GetEntryIndex` (0x825a32a0): the first entry of the activity's
    /// character.
    fn find(&self, activity: usize, character_index: i32) -> Option<u32> {
        self.entries
            .iter()
            .position(|e| {
                e.as_ref()
                    .is_some_and(|(d, _)| d.same(activity, character_index))
            })
            .map(|i| u32::try_from(i).expect("256 entries"))
    }

    /// `Remove(index, previous, 1)` (0x825a7d00): the entry becomes `InvalidEntry`, its slot is
    /// freed and it leaves the previous collisions.
    fn remove(&mut self, index: u32, previous: &mut Vec<u32>) {
        self.entries[index as usize] = None;
        self.free.push(index);
        if let Some(at) = previous.iter().position(|&i| i == index) {
            previous.remove(at);
        }
    }

    /// `GetCollisions(bbox)` (0x825a7eb0): the entries whose boxes the box meets, sorted.
    fn collisions(&self, bounds: OctreeBounds) -> Vec<u32> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.as_ref().is_some_and(|(_, b)| b.overlaps(bounds)))
            .map(|(i, _)| u32::try_from(i).expect("256 entries"))
            .collect()
    }

    fn entry(&self, index: u32) -> Option<&SpawnData> {
        self.entries.get(index as usize)?.as_ref().map(|(d, _)| d)
    }

    fn entry_mut(&mut self, index: u32) -> Option<&mut SpawnData> {
        self.entries
            .get_mut(index as usize)?
            .as_mut()
            .map(|(d, _)| d)
    }

    /// `Reset` (0x825a59b8).
    fn reset(&mut self) {
        self.entries.clear();
        self.free.clear();
    }
}

/// `cChallengeCharacterManager`'s state.
#[derive(Default)]
pub struct CharacterManager {
    /// `GameLoaded`.
    game_loaded: bool,
    /// `LivingWorldSpawnTree`.
    tree: SpawnTree,
    /// `LivingWorldSpawnPendingLocation`: entries whose location or waypoint group was not
    /// there yet.
    pending: Vec<SpawnData>,
    /// `UpdatePendingSpawnsNextFrame`.
    update_pending_next_frame: bool,
    /// `PreviousCollisionIndicies`.
    previous: Vec<u32>,
    /// `AllLivingWorldActivities`.
    all: Vec<usize>,
    /// `LivingWorldActivitiesPassedPrereq`, ascending.
    passed: Vec<usize>,
    /// `CharacterTreeMap`: pedestrian to tree index.
    characters: BTreeMap<CharacterRef, u32>,
    /// `WaypointGUIDMap`: waypoint group GUID to tree index.
    waypoints: BTreeMap<u64, u32>,
    /// `RecipeTreeMap` (`fixed_multimap`): recipe to tree index, equal recipes in insertion
    /// order.
    recipes: Vec<(u64, u32)>,
    /// `PhysicsLineTestBatch`.
    line_batch: Option<LineBatch>,
    next_line_batch: u32,
    /// The living world's changes, in call order, for the session.
    ops: Vec<LivingWorldOp>,
    next_token: u32,
    /// The spawns awaiting the session's answer: their tree index, and whether a spawned
    /// pedestrian clears `UseAlternateLocation` (no main challenge when it was placed).
    spawns: BTreeMap<u32, (u32, bool)>,
    pub tuning: Tuning,
    /// The session's living world.
    facts: Option<Box<dyn LivingWorldFacts>>,
    /// `CommonBackEnd::sGameInfo.securitySetting`, set by the session.
    pub security_setting: bool,
    /// `NISManager::mPreloadedSceneHash` is set, set by the session.
    pub nis_scene_preloaded: bool,
}

impl std::fmt::Debug for CharacterManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharacterManager")
            .field("game_loaded", &self.game_loaded)
            .field("tree", &self.tree)
            .field("pending", &self.pending)
            .field("previous", &self.previous)
            .field("passed", &self.passed)
            .field("characters", &self.characters)
            .finish_non_exhaustive()
    }
}

impl CharacterManager {
    /// The spawn data of a tree index.
    #[must_use]
    pub fn entry(&self, index: u32) -> Option<&SpawnData> {
        self.tree.entry(index)
    }

    /// The tree's entries in index order.
    pub fn entries(&self) -> impl Iterator<Item = (u32, &SpawnData)> {
        self.tree.entries.iter().enumerate().filter_map(|(i, e)| {
            e.as_ref()
                .map(|(d, _)| (u32::try_from(i).expect("256 entries"), d))
        })
    }

    /// The entries waiting for their location or waypoint group.
    #[must_use]
    pub fn pending(&self) -> &[SpawnData] {
        &self.pending
    }

    /// `PreviousCollisionIndicies`.
    #[must_use]
    pub fn previous_collisions(&self) -> &[u32] {
        &self.previous
    }

    /// TestBBoxAgainstTree825ab590's activity pointer assigned on a successful spawn.
    pub(crate) fn spawn_owner(&self, token: u32) -> Option<usize> {
        let &(index, _) = self.spawns.get(&token)?;
        self.tree.entry(index).map(|entry| entry.activity)
    }

    /// The changes not taken yet.
    #[must_use]
    pub fn ops(&self) -> &[LivingWorldOp] {
        &self.ops
    }

    /// The session's living world.
    #[must_use]
    pub fn facts(&self) -> Option<&dyn LivingWorldFacts> {
        self.facts.as_deref()
    }

    /// `CharacterTreeMap`'s pedestrians, in key order.
    #[must_use]
    pub fn challenge_characters(&self) -> Vec<CharacterRef> {
        self.characters.keys().copied().collect()
    }
}

/// The activity's `challenges` key.
fn challenge_key(world: &World, activity: usize) -> u64 {
    world.activities.activities[activity].key
}

/// `GetLength` of one of the activity's character arrays.
fn array_len(world: &World, activity: usize, key: u64) -> usize {
    world
        .db
        .attribute(class::CHALLENGES, challenge_key(world, activity), key)
        .map_or(0, |a| a.len())
}

/// `GetAttributePointer(key, index)` of a location character, `DefaultDataArea(0x28)` when
/// absent.
fn location_info(world: &World, activity: usize, index: usize) -> SpawnInfo {
    let db = &world.db;
    let at = db
        .attribute(
            class::CHALLENGES,
            challenge_key(world, activity),
            key::CHARACTERS,
        )
        .and_then(|a| a.data(index));
    let bytes = at
        .and_then(|at| db.bytes(at, 40))
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| vec![0; 40]);
    let word = |o: usize| u32::from_be_bytes(bytes[o..o + 4].try_into().expect("four bytes"));
    let long = |o: usize| u64::from_be_bytes(bytes[o..o + 8].try_into().expect("eight bytes"));
    let text = |o: u32| {
        at.and_then(|at| db.text_at(at.offset_by(o)))
            .map(str::to_owned)
    };
    SpawnInfo::Location {
        entity: (long(0), long(8)),
        location: text(24),
        on_start: word(28),
        alternate: text(32),
    }
}

/// `GetAttributePointer(key, index)` of a waypoint character, `DefaultDataArea(0x30)` when
/// absent.
fn waypoint_info(world: &World, activity: usize, index: usize) -> SpawnInfo {
    let db = &world.db;
    let at = db
        .attribute(
            class::CHALLENGES,
            challenge_key(world, activity),
            key::WAYPOINT_CHARACTERS,
        )
        .and_then(|a| a.data(index));
    let bytes = at
        .and_then(|at| db.bytes(at, 48))
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| vec![0; 48]);
    let word = |o: usize| u32::from_be_bytes(bytes[o..o + 4].try_into().expect("four bytes"));
    let long = |o: usize| u64::from_be_bytes(bytes[o..o + 8].try_into().expect("eight bytes"));
    SpawnInfo::Waypoint {
        entity: (long(0), long(8)),
        waypoint: long(32),
        on_start: word(40),
    }
}

/// `cLocationManager::IsValidLocation`/`GetLocationOrientation` (`0x82a8de48`): the session's
/// location manager (global, streamed and hull arenas), then the simulation location table the
/// challenge system holds.
fn location(world: &World, name: &str) -> Option<[f32; 16]> {
    let hash = super::keys::hash_value32(name);
    world
        .host
        .location_orientation(hash)
        .or_else(|| world.locations.get(&hash).copied())
}

fn rows(m: &[f32; 16]) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|c| m[r * 4 + c]))
}

/// The `livingworld_models` key the entity's layout names first (the census's parent model).
fn entity_model(world: &World, entity: (u64, u64)) -> u64 {
    let (high, low) = (
        world.db.layout_word(entity.0, entity.1, 2),
        world.db.layout_word(entity.0, entity.1, 3),
    );
    (u64::from(high) << 32) | u64::from(low)
}

/// The model's `eLivingWorldModelCategory` (`DefaultDataArea(4)`: 0 when absent).
fn model_category(world: &World, model: u64) -> u32 {
    world
        .db
        .attribute(LIVINGWORLD_MODELS, model, key::MODEL_CATEGORY)
        .and_then(|a| a.u32(0))
        .unwrap_or(0)
}

/// `GenerateLivingWorldSpawnActivityBBox` (0x825a46a0): the `pedestrians` census range's larger
/// cull radius (the minimum circle's when its speed is not below the maximum's), ten metres
/// more for a marquee model, around the location.
fn spawn_bounds(world: &World, entity: (u64, u64), at: [f32; 4]) -> OctreeBounds {
    let ranges = hash64("pedestrians");
    let scalar = |index: u32| {
        f32::from_bits(
            world
                .db
                .layout_word(LIVINGWORLD_CENSUS_RANGES, ranges, index),
        )
    };
    let (min_cull, min_speed, max_cull, max_speed) = (scalar(2), scalar(4), scalar(7), scalar(9));
    let mut radius = if min_speed >= max_speed {
        min_cull
    } else if max_cull - min_cull >= 0.0 {
        max_cull
    } else {
        min_cull
    };
    if model_category(world, entity_model(world, entity)) == MARQUEE_CATEGORY {
        radius += MARQUEE_BOX_PADDING;
    }
    OctreeBounds {
        minimum: [at[0] - radius, at[1] - radius, at[2] - radius, at[3]],
        maximum: [at[0] + radius, at[1] + radius, at[2] + radius, at[3]],
    }
}

/// `TriggerVolumeUtils::GenerateTesterBBox` (0x82e20ee0) about a base position.
pub(super) fn tester_bounds(base: [f32; 3]) -> OctreeBounds {
    let half = TRIGGER_AABB_CHARACTER_WIDTH * 0.5;
    OctreeBounds {
        minimum: [base[0] - half, base[1], base[2] - half, 0.0],
        maximum: [
            base[0] + half,
            base[1] + TRIGGER_AABB_CHARACTER_HEIGHT,
            base[2] + half,
            0.0,
        ],
    }
}

fn waypoint_location(world: &World, guid: u64) -> Option<[f32; 4]> {
    world
        .characters
        .facts
        .as_ref()
        .and_then(|facts| facts.waypoint_location(guid))
}

fn is_model_tracked(world: &World, model: u64) -> bool {
    world
        .characters
        .facts
        .as_ref()
        .is_some_and(|facts| facts.is_model_tracked(model))
}

pub(crate) fn op(world: &mut World, op: LivingWorldOp) {
    world.characters.ops.push(op);
}

/// Installs the session's living world.
pub fn install_facts(world: &mut World, facts: Box<dyn LivingWorldFacts>) {
    world.characters.facts = Some(facts);
}

/// The living world's changes since the last call, in call order.
pub fn take_ops(world: &mut World) -> Vec<LivingWorldOp> {
    std::mem::take(&mut world.characters.ops)
}

/// `challenge_global_data`: an activity only debug builds create.
const DEBUG_ONLY: u64 = 0xfd40_e4a3_1027_c984;

/// `cChallengeCharacterManager::Init` (0x825a9180), then `cActivityManager::CreateActivity`'s
/// `AddActivity` (0x825a80f8) for every activity that is not debug-only
/// (`kAllowDebugOnlyActivities` off): those with characters.
pub(crate) fn init(world: &mut World) {
    reset(world);
    for activity in 0..world.activities.activities.len() {
        let debug_only = world.activities.activities[activity]
            .global_data
            .and_then(|(c, k)| world.db.attribute(c, k, DEBUG_ONLY))
            .and_then(|a| a.bool(0))
            .unwrap_or(false);
        if debug_only
            || (array_len(world, activity, key::CHARACTERS) == 0
                && array_len(world, activity, key::WAYPOINT_CHARACTERS) == 0)
        {
            continue;
        }
        if !world.characters.all.contains(&activity) {
            world.characters.all.push(activity);
        }
    }
}

/// `Reset` (0x825a8258): every entry released, the lists and the tree emptied.
fn reset(world: &mut World) {
    let indices: Vec<u32> = world.characters.entries().map(|(i, _)| i).collect();
    for index in indices {
        release_spawn_data(world, index);
    }
    let c = &mut world.characters;
    c.update_pending_next_frame = false;
    c.passed.clear();
    c.pending.clear();
    c.tree.reset();
    c.previous.clear();
}

/// `GameLoad` (0x825a2f58): career (4) and game type 2 load the activities' characters.
pub(crate) fn game_load(world: &mut World, game_type_flags: u32) {
    if game_type_flags == 4 || game_type_flags == 2 {
        world.characters.game_loaded = true;
        reevaluate_activity_prereqs(world, game_type_flags);
    }
}

/// `GameUnload` (0x825a9270).
pub(crate) fn game_unload(world: &mut World) {
    if world.characters.game_loaded {
        world.characters.game_loaded = false;
        reset(world);
    }
}

/// `ReleaseSpawnData` (0x825a7930): the pedestrian culled (`kKillPrereqFailedCharacters`) or
/// left to the census, the line test and spawn position cleared, then the map and model data.
fn release_spawn_data(world: &mut World, index: u32) {
    let kill = world.characters.tuning.kill_prereq_failed;
    let Some(data) = world.characters.tree.entry_mut(index) else {
        return;
    };
    let entity = data.entity;
    data.line_test = -1;
    data.spawn_position = [0.0; 4];
    if let Some(entity) = entity {
        op(
            world,
            if kill {
                LivingWorldOp::RequestExplicitCull(entity)
            } else {
                LivingWorldOp::AllowCull(entity)
            },
        );
    }
    release_map_data(world, index);
    release_model_data(world, index);
}

/// `ReleaseMapData` (0x825a6bb8): the waypoint group's `WaypointGUIDMap` entry, then the
/// pedestrian's `CharacterTreeMap` entry and its `RecipeTreeMap` node for this index; the entry
/// keeps no pedestrian, line test or spawn position.
fn release_map_data(world: &mut World, index: u32) {
    let Some(data) = world.characters.tree.entry_mut(index) else {
        return;
    };
    let waypoint = match data.info {
        SpawnInfo::Waypoint { waypoint, .. } => Some(waypoint),
        SpawnInfo::Location { .. } => None,
    };
    let entity = data.entity.take();
    let recipe = data.recipe;
    if entity.is_some() {
        data.recipe = crate::living_world_grind_services::dmo_models::INVALID_ASSET_ID;
    }
    data.line_test = -1;
    data.spawn_position = [0.0; 4];
    let c = &mut world.characters;
    if let Some(waypoint) = waypoint {
        c.waypoints.remove(&waypoint);
    }
    if let Some(entity) = entity {
        c.characters.remove(&entity);
        if let Some(at) = c
            .recipes
            .iter()
            .position(|&(r, i)| r == recipe && i == index)
        {
            c.recipes.remove(at);
        }
    }
}

/// `ReleaseModelData` (0x825a4800): a requested marquee model is unloaded.
fn release_model_data(world: &mut World, index: u32) {
    let Some(data) = world.characters.tree.entry(index) else {
        return;
    };
    if !data.requested_model_load {
        return;
    }
    let after = data.model_load_token;
    let model = entity_model(world, data.info.entity());
    if model_category(world, model) == MARQUEE_CATEGORY {
        op(world, LivingWorldOp::UnloadModel { model, after });
        if let Some(data) = world.characters.tree.entry_mut(index) {
            data.requested_model_load = false;
            data.model_load_token = None;
        }
    }
}

/// `AddActivityEntries` (0x825ac948): one entry per location character with a location and per
/// waypoint character with a waypoint, in the tree when its location or group is there, else
/// pending.
fn add_activity_entries(world: &mut World, activity: usize) {
    let mut character_index = 0;
    for index in 0..array_len(world, activity, key::CHARACTERS) {
        let info = location_info(world, activity, index);
        let data = SpawnData::new(activity, character_index, info.clone());
        character_index += 1;
        let SpawnInfo::Location {
            location: Some(name),
            ..
        } = &info
        else {
            continue;
        };
        if location(world, name).is_some() {
            add_spawn_data_to_tree(world, data);
        } else {
            world.characters.pending.push(data);
        }
    }
    for index in 0..array_len(world, activity, key::WAYPOINT_CHARACTERS) {
        let info = waypoint_info(world, activity, index);
        let data = SpawnData::new(activity, character_index, info.clone());
        character_index += 1;
        let SpawnInfo::Waypoint { waypoint, .. } = info else {
            continue;
        };
        if waypoint == 0 {
            continue;
        }
        if waypoint_location(world, waypoint).is_some() {
            add_spawn_data_to_tree(world, data);
        } else {
            world.characters.pending.push(data);
        }
    }
}

/// `AddLivingWorldSpawnDataToTree` (0x825ac378): the entry's box about its location's position
/// or its waypoint group's first waypoint; a waypoint entry joins `WaypointGUIDMap`. The native
/// asserts that the location or group is there and that the tree lacks the entry.
fn add_spawn_data_to_tree(world: &mut World, data: SpawnData) {
    let at = match &data.info {
        SpawnInfo::Location {
            location: Some(name),
            ..
        } => location(world, name).map(|t| [t[12], t[13], t[14], t[15]]),
        SpawnInfo::Location { location: None, .. } => None,
        SpawnInfo::Waypoint { waypoint, .. } => waypoint_location(world, *waypoint),
    };
    let Some(at) = at else { return };
    if world
        .characters
        .tree
        .find(data.activity, data.character_index)
        .is_some()
    {
        return;
    }
    let bounds = spawn_bounds(world, data.info.entity(), at);
    let waypoint = match data.info {
        SpawnInfo::Waypoint { waypoint, .. } => Some(waypoint),
        SpawnInfo::Location { .. } => None,
    };
    let index = world.characters.tree.insert(data, bounds);
    if let Some(waypoint) = waypoint {
        world.characters.waypoints.insert(waypoint, index);
    }
}

/// `UpdatePendingBBoxCreation` (0x825ac5b0): pending entries whose location or waypoint group
/// is now there join the tree.
fn update_pending_bbox_creation(world: &mut World) {
    if world.characters.game_loaded {
        let mut at = 0;
        while at < world.characters.pending.len() {
            let data = world.characters.pending[at].clone();
            let ready = match &data.info {
                SpawnInfo::Location {
                    location: Some(name),
                    ..
                } => location(world, name).is_some(),
                SpawnInfo::Location { location: None, .. } => false,
                SpawnInfo::Waypoint { waypoint, .. } => {
                    waypoint_location(world, *waypoint).is_some()
                }
            };
            if !ready {
                at += 1;
                continue;
            }
            let at_position = match &data.info {
                SpawnInfo::Location {
                    location: Some(name),
                    ..
                } => location(world, name).map(|t| [t[12], t[13], t[14], t[15]]),
                SpawnInfo::Waypoint { waypoint, .. } => waypoint_location(world, *waypoint),
                SpawnInfo::Location { location: None, .. } => None,
            };
            if let Some(position) = at_position {
                let bounds = spawn_bounds(world, data.info.entity(), position);
                let waypoint = match data.info {
                    SpawnInfo::Waypoint { waypoint, .. } => Some(waypoint),
                    SpawnInfo::Location { .. } => None,
                };
                let index = world.characters.tree.insert(data, bounds);
                if let Some(waypoint) = waypoint {
                    world.characters.waypoints.insert(waypoint, index);
                }
            }
            world.characters.pending.remove(at);
        }
    }
    world.characters.update_pending_next_frame = false;
}

/// `ReevaluateActivityPrereqs` (0x825acc48): the activities whose prerequisites now fail lose
/// their entries (pending or released from the tree), the newly passing ones gain theirs, then
/// the local player's box is tested.
pub(crate) fn reevaluate_activity_prereqs(world: &mut World, game_type_flags: u32) {
    if !world.characters.game_loaded {
        return;
    }
    let all = world.characters.all.clone();
    let passed =
        signup::evaluate_for_prereqs(world, &all, game_type_flags, "IgnorePrereqForLivingWorld");
    let cached = world.characters.passed.clone();
    let failed: Vec<usize> = cached
        .iter()
        .copied()
        .filter(|a| passed.binary_search(a).is_err())
        .collect();
    let newly: Vec<usize> = passed
        .iter()
        .copied()
        .filter(|a| cached.binary_search(a).is_err())
        .collect();
    for activity in failed {
        let count = array_len(world, activity, key::CHARACTERS)
            + array_len(world, activity, key::WAYPOINT_CHARACTERS);
        for character_index in 0..i32::try_from(count).expect("ten characters each") {
            if let Some(at) = world
                .characters
                .pending
                .iter()
                .position(|d| d.same(activity, character_index))
            {
                world.characters.pending.remove(at);
            } else if let Some(index) = world.characters.tree.find(activity, character_index) {
                release_spawn_data(world, index);
                let c = &mut world.characters;
                c.tree.remove(index, &mut c.previous);
            }
        }
    }
    for activity in newly {
        add_activity_entries(world, activity);
    }
    world.characters.passed = passed;
    spot_test_characters_against_local_player(world);
}

/// `Update` (0x825ac8f8): the local player's box test, then the pending entries when a
/// waypoint group arrived.
pub(crate) fn update(world: &mut World) {
    if world.characters.game_loaded {
        spot_test_characters_against_local_player(world);
        if world.characters.update_pending_next_frame {
            update_pending_bbox_creation(world);
        }
    }
}

/// `SpotTestCharactersAgainstLocalPlayer` (0x825ac7e8): the local skater's tester box, while
/// its tester is valid.
fn spot_test_characters_against_local_player(world: &mut World) {
    if let Some(base) = world.player.tester_base {
        test_bbox_against_tree(world, tester_bounds(base));
    }
}

/// `SpotTestCharactersAgainstPoint` (0x825ac8a8), from `StateLoading::CharacterPrepareState`
/// (0x82760058) at the stream focus.
pub fn spot_test_characters_against_point(world: &mut World, point: [f32; 3]) {
    if world.characters.game_loaded {
        test_bbox_against_tree(world, tester_bounds(point));
    }
}

/// `IsAllowedToSpawnWithCurrentChallengeState` (0x825a3160): without an active challenge every
/// character; the active challenge's own characters unless they leave when it starts; other
/// activities' characters unless the active challenge hides them.
fn is_allowed_to_spawn(world: &World, data: &SpawnData, active: Option<usize>) -> bool {
    let Some(active) = active else { return true };
    if data.activity == active {
        return match data.info {
            SpawnInfo::Location { on_start, .. } => on_start != on_start::REMOVE,
            SpawnInfo::Waypoint { on_start, .. } => {
                on_start != on_start::REMOVE && on_start != on_start::ALTERNATE_LOCATION
            }
        };
    }
    !world
        .db
        .attribute(
            class::CHALLENGES,
            challenge_key(world, active),
            key::HIDES_OTHER_CHARACTERS,
        )
        .and_then(|a| a.bool(0))
        .unwrap_or(false)
}

/// `IsZero` of `TestBBoxAgainstTree`: no lane of x, y, z is above `EPSILON_8` in magnitude.
fn is_zero(v: [f32; 4]) -> bool {
    !(v[0].abs() > ZERO_EPSILON || v[1].abs() > ZERO_EPSILON || v[2].abs() > ZERO_EPSILON)
}

/// `AddEntityToLineTestBatch` (0x825a2ff0): a test from `kChallengeSpawnLineTestOffsetsUp` above
/// the position to `kChallengeSpawnLineTestOffsetsDown` below it, its index in the batch, or -1
/// once the batch holds eight.
fn add_entity_to_line_test_batch(world: &mut World, position: [f32; 4]) -> i32 {
    let tuning = world.characters.tuning;
    let c = &mut world.characters;
    if c.line_batch.is_none() {
        c.line_batch = Some(LineBatch {
            serial: c.next_line_batch,
            ..LineBatch::default()
        });
        c.next_line_batch = c.next_line_batch.wrapping_add(1);
    }
    let Some(batch) = c.line_batch.as_mut() else {
        return -1;
    };
    if batch.results.is_some() || batch.tests.len() >= LINE_BATCH_CAPACITY {
        return -1;
    }
    // vmaddfp(J, splat(up), position), then position - J * splat(down).
    let start = [
        position[0],
        1.0f32.mul_add(tuning.line_test_up, position[1]),
        position[2],
        position[3],
    ];
    let end = [
        position[0],
        position[1] - tuning.line_test_down,
        position[2],
        position[3],
    ];
    batch.tests.push((start, end, tuning.line_test_fatness));
    i32::try_from(batch.tests.len() - 1).expect("eight tests")
}

/// `TestBBoxAgainstTree` (0x825ab590).
///
/// The entries the box left are released (a pedestrian's `Entity` slot 35 answers true,
/// 0x826c6d80). The previous update's line tests are read: a hit on the static world is a ground
/// position, a hit on an assembly resets its dynamic object and counts as none. Then each entry
/// the box holds without a pedestrian is placed: its location's orientation (the alternate one
/// while its own challenge runs with `UseAlternateLocation`) or its waypoint group's first
/// waypoint, a `livingworld_entities` entity, allowed by the challenge state (and in profile
/// free skate only the no-skate-zone guards with security on), a marquee model only while no
/// NIS is preloaded or once tracked. Without a ground position it waits for a line test; with
/// one it is spawned there.
fn test_bbox_against_tree(world: &mut World, bounds: OctreeBounds) {
    let mut colliding = world.characters.tree.collisions(bounds);
    let previous = world.characters.previous.clone();
    let exited: Vec<u32> = previous
        .iter()
        .copied()
        .filter(|i| colliding.binary_search(i).is_err())
        .collect();
    for index in exited {
        release_spawn_data(world, index);
    }
    let mut results = Vec::new();
    if let Some(batch) = world.characters.line_batch.take() {
        let answers = batch
            .results
            .unwrap_or_else(|| vec![None; batch.tests.len()]);
        for answer in answers.into_iter().take(batch.tests.len()) {
            results.push(match answer {
                None => [0.0; 4],
                Some(LineHit {
                    owner: Some(owner), ..
                }) => {
                    op(world, LivingWorldOp::ResetDynamicObject(owner));
                    [0.0; 4]
                }
                Some(LineHit {
                    position,
                    owner: None,
                }) => position,
            });
        }
    }
    colliding.sort_unstable();
    for &index in &colliding {
        place_entry(world, index, &results);
    }
    if let Some(batch) = world.characters.line_batch.as_ref()
        && batch.results.is_none()
        && !batch.tests.is_empty()
    {
        let (serial, tests) = (batch.serial, batch.tests.clone());
        op(world, LivingWorldOp::LineTests { serial, tests });
    }
    world.characters.previous = colliding;
}

/// One colliding entry of `TestBBoxAgainstTree`.
fn place_entry(world: &mut World, index: u32, results: &[[f32; 4]]) {
    let Some(data) = world.characters.tree.entry(index).cloned() else {
        return;
    };
    if data.entity.is_some() {
        return;
    }
    let main = world.activities.main;
    let mut transform = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0; 4],
    ];
    let mut waypoint = None;
    let entity = match &data.info {
        SpawnInfo::Location {
            entity,
            location: name,
            alternate,
            ..
        } => {
            let name = if data.use_alternate_location && main == Some(data.activity) {
                alternate
            } else {
                name
            };
            let Some(orientation) = name.as_deref().and_then(|name| location(world, name)) else {
                return;
            };
            transform = rows(&orientation);
            *entity
        }
        SpawnInfo::Waypoint {
            entity,
            waypoint: group,
            ..
        } => {
            let Some(at) = waypoint_location(world, *group) else {
                return;
            };
            transform[3] = at;
            waypoint = Some(*group);
            *entity
        }
    };
    if entity.0 != LIVINGWORLD_ENTITIES {
        return;
    }
    let model = entity_model(world, entity);
    let marquee = model_category(world, model) == MARQUEE_CATEGORY;
    let mut allowed = is_allowed_to_spawn(world, &data, main);
    if allowed && world.game_mode == GM_PROFILE_FREE_SKATE {
        allowed = world.characters.security_setting
            && (entity.1 == hash64("security_nsz01") || entity.1 == hash64("security_nsz02"));
    }
    if allowed && marquee && world.characters.nis_scene_preloaded && !is_model_tracked(world, model)
    {
        allowed = false;
    }
    let mut data = data;
    if !allowed {
        data.line_test = -1;
    } else if is_zero(data.spawn_position) {
        let hit = usize::try_from(data.line_test)
            .ok()
            .and_then(|at| results.get(at))
            .copied();
        if let Some(hit) = hit.filter(|&hit| !is_zero(hit)) {
            data.spawn_position = hit;
            data.line_test = -1;
        } else {
            data.line_test = add_entity_to_line_test_batch(world, transform[3]);
            allowed = false;
        }
    }
    if allowed {
        transform[3] = data.spawn_position;
        if data.requested_model_load
            && data.model_load_token.is_none()
            && !is_model_tracked(world, model)
        {
            data.requested_model_load = false;
        }
        let load_model = !data.requested_model_load && marquee;
        let token = world.characters.next_token;
        world.characters.next_token = token.wrapping_add(1);
        if load_model {
            data.requested_model_load = true;
            data.model_load_token = Some(token);
        }
        let spawn = world.characters.tuning.enable_spawn;
        if spawn {
            data.entity = Some(CharacterRef::Pending(token));
            world
                .characters
                .characters
                .insert(CharacterRef::Pending(token), index);
        }
        let clear_alternate = main.is_none() && data.use_alternate_location;
        let challenge_flags =
            (data.use_alternate_location && !clear_alternate) || main == Some(data.activity);
        if spawn || load_model {
            world
                .characters
                .spawns
                .insert(token, (index, clear_alternate));
            op(
                world,
                LivingWorldOp::Spawn {
                    token,
                    entity: entity.1,
                    transform,
                    model,
                    load_model,
                    spawn,
                    waypoint,
                    challenge_flags,
                },
            );
        }
    }
    if let Some(slot) = world.characters.tree.entry_mut(index) {
        *slot = data;
    }
}

/// The session's answer to [`LivingWorldOp::Spawn`]: whether the model load was taken (when one
/// was asked for) and the spawned pedestrian with its recipe. A refused load or failed spawn
/// leaves the entry without a pedestrian (`EntityID` 0, `kInvalidAssetID`); a spawned one joins
/// `CharacterTreeMap` and `RecipeTreeMap`, and leaves `UseAlternateLocation` when no main
/// challenge ran as it was placed.
pub fn spawn_answered(
    world: &mut World,
    token: u32,
    model_loaded: Option<bool>,
    spawned: Option<(u32, u64)>,
) {
    let pending = CharacterRef::Pending(token);
    world.characters.characters.remove(&pending);
    let Some((index, clear_alternate)) = world.characters.spawns.remove(&token) else {
        return;
    };
    let Some(data) = world.characters.tree.entry_mut(index) else {
        return;
    };
    if data.model_load_token == Some(token) {
        data.model_load_token = None;
        if let Some(loaded) = model_loaded {
            data.requested_model_load = loaded;
        }
    }
    if data.entity != Some(pending) {
        return;
    }
    match spawned {
        Some((id, recipe)) => {
            data.entity = Some(CharacterRef::Spawned(id));
            data.recipe = recipe;
            if clear_alternate {
                data.use_alternate_location = false;
            }
            world
                .characters
                .characters
                .insert(CharacterRef::Spawned(id), index);
            let at = world
                .characters
                .recipes
                .iter()
                .rposition(|&(r, _)| r <= recipe)
                .map_or(0, |at| at + 1);
            world.characters.recipes.insert(at, (recipe, index));
        }
        None => {
            data.entity = None;
            data.recipe = crate::living_world_grind_services::dmo_models::INVALID_ASSET_ID;
        }
    }
}

/// The session's answer to [`LivingWorldOp::LineTests`], one result per test. A batch a later
/// `TestBBoxAgainstTree` of the same update has already read keeps no answer.
pub fn line_tests_answered(world: &mut World, serial: u32, results: Vec<Option<LineHit>>) {
    if let Some(batch) = world.characters.line_batch.as_mut()
        && batch.serial == serial
        && batch.results.is_none()
    {
        batch.results = Some(results);
    }
}

/// `RunMainActivity` (0x825a58c8): the challenge's own kept characters the tester's box holds
/// start taking pictures.
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    let previous = world.characters.previous.clone();
    for index in previous {
        let Some(data) = world.characters.tree.entry(index) else {
            continue;
        };
        if let (
            SpawnInfo::Location {
                on_start: on_start::KEEP,
                ..
            },
            true,
            Some(entity),
        ) = (&data.info, data.activity == activity, data.entity)
        {
            op(
                world,
                LivingWorldOp::StateFlags {
                    character: entity,
                    set: [0x40, 0],
                    clear: [0x20, 0],
                },
            );
        }
    }
}

/// `ForceCullChallengeSpawnedLivingWorld` (0x825a85f0), from the living world manager's
/// `RunMainActivity`: spawned characters the starting challenge disallows are released, the
/// others have their pro challenge started; its alternate-location characters are culled and
/// spawn again at the alternate location, as do those the tester's box holds.
pub(crate) fn force_cull_challenge_spawned_living_world(world: &mut World, activity: usize) {
    let spawned: Vec<(CharacterRef, u32)> = world
        .characters
        .characters
        .iter()
        .map(|(&e, &i)| (e, i))
        .collect();
    let mut culls = Vec::new();
    let mut alternates = Vec::new();
    for (entity, index) in spawned {
        let Some(data) = world.characters.tree.entry(index) else {
            continue;
        };
        if is_allowed_to_spawn(world, data, Some(activity)) {
            if data.activity == activity
                && matches!(
                    data.info,
                    SpawnInfo::Location {
                        on_start: on_start::ALTERNATE_LOCATION,
                        ..
                    }
                )
            {
                alternates.push(index);
            }
            op(
                world,
                LivingWorldOp::StateFlags {
                    character: entity,
                    set: [0, 0x40],
                    clear: [0, 0],
                },
            );
        } else {
            culls.push(index);
        }
    }
    for index in culls {
        release_spawn_data(world, index);
    }
    for index in alternates {
        let Some(entity) = world.characters.tree.entry(index).and_then(|d| d.entity) else {
            continue;
        };
        op(world, LivingWorldOp::RequestExplicitCull(entity));
        if let Some(data) = world.characters.tree.entry_mut(index) {
            data.use_alternate_location = true;
        }
        release_map_data(world, index);
    }
    let previous = world.characters.previous.clone();
    for index in previous {
        let Some(data) = world.characters.tree.entry_mut(index) else {
            continue;
        };
        if data.activity == activity
            && matches!(
                data.info,
                SpawnInfo::Location {
                    on_start: on_start::ALTERNATE_LOCATION,
                    ..
                }
            )
        {
            data.use_alternate_location = true;
            release_map_data(world, index);
        }
    }
}

/// `CompleteChallenge` (0x825a92d8) for the main challenge: `ResetAlternateLocationCharacters`
/// (0x825a8a30).
pub(crate) fn complete_challenge(world: &mut World, activity: usize, is_main: bool) {
    if is_main {
        reset_alternate_location_characters(world, activity);
    }
}

/// `ResetAlternateLocationCharacters` (0x825a8a30): the challenge's spawned characters stop
/// taking and reviewing pictures; those at their alternate location are culled to spawn at
/// their location again.
fn reset_alternate_location_characters(world: &mut World, activity: usize) {
    let spawned: Vec<(CharacterRef, u32)> = world
        .characters
        .characters
        .iter()
        .map(|(&e, &i)| (e, i))
        .collect();
    let mut alternates = Vec::new();
    for (entity, index) in spawned {
        let Some(data) = world.characters.tree.entry(index) else {
            continue;
        };
        if data.activity != activity {
            continue;
        }
        if data.use_alternate_location {
            alternates.push(index);
        } else {
            op(
                world,
                LivingWorldOp::StateFlags {
                    character: entity,
                    set: [0, 0],
                    clear: [0x60, 0],
                },
            );
        }
    }
    for index in alternates {
        if let Some(entity) = world.characters.tree.entry(index).and_then(|d| d.entity) {
            op(world, LivingWorldOp::RequestExplicitCull(entity));
            release_map_data(world, index);
        }
        if let Some(data) = world.characters.tree.entry_mut(index) {
            data.use_alternate_location = false;
        }
    }
}

/// `ReceiveWaypointGroupAddedMessage` (0x825a5798), `cMsgWaypointGroupAdded` on the simulation
/// broadcaster: the pending entries are tried on the next update, and a spawned character of
/// the group is put on its first waypoint.
pub fn receive_waypoint_group_added(world: &mut World, guid: u64) {
    world.characters.update_pending_next_frame = true;
    let Some(&index) = world.characters.waypoints.get(&guid) else {
        return;
    };
    if let Some(entity) = world.characters.tree.entry(index).and_then(|d| d.entity) {
        op(
            world,
            LivingWorldOp::SetCurrentWaypoint {
                character: entity,
                waypoint: guid,
            },
        );
    }
}

/// `ReceiveLocationAddedMessage` (0x825a3150), `cMsgLocationAdded`.
pub fn receive_location_added(world: &mut World) {
    world.characters.update_pending_next_frame = true;
}

/// `ReceiveDRegLWSchemaMsg` (0x825a8470), `cMsgAssetSimDRegLWSchema` from
/// `RecipeMan::UnloadLivingWorldModel` (0x82712918): the characters of the recipe are released.
pub fn receive_dreg_lw_schema(world: &mut World, recipe: u64) {
    let indices: Vec<u32> = world
        .characters
        .recipes
        .iter()
        .filter(|&&(r, _)| r == recipe)
        .map(|&(_, i)| i)
        .collect();
    for index in indices {
        release_spawn_data(world, index);
    }
}

/// `IsChallengeCharacter` (0x825a6760), which `LivingWorld::Manager::
/// IsPedestrianInvolvedInChallenges` (0x82703e48) answers with.
#[must_use]
pub fn is_challenge_character(world: &World, entity: u32) -> bool {
    world
        .characters
        .characters
        .contains_key(&CharacterRef::Spawned(entity))
}

#[cfg(test)]
#[path = "characters_tests.rs"]
mod tests;
