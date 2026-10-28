//! `Sk8::Challenge::cVisualIndicatorManager` (ctor `0x82db4048`): the `cVisualIndicatorGroup`s
//! of the activity scripts, the sign-up manager and the objectives, under their owner's unique
//! ID (`VisualIndicatorGroups`, a multimap), and the `cFixedMemStore` banks every group's
//! location, spline, follow and trigger indicators come from. `VisualDirector::VDWriter`
//! reads them through `GetVisible*Indicators` (`visual_director`).

use std::collections::BTreeMap;

use super::{
    attrib_db::{AttribDb, Location},
    attrib_lua::{ObjectKind, Pointee, REF_SPEC_METATABLE, RefSpec, attrib_location},
    binding::{get_bool, get_float, get_int, get_int_or},
    keys::{hash64, hash_value32},
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value, stdlib},
    objectives,
    system::World,
};

type R = LuaResult<Vec<Value>>;

/// `eSk8ChallengeIndicators_Unknown`: the type of a bank's unused trigger indicator.
const INDICATOR_UNKNOWN: i32 = 13;

/// `cFixedMemStore<tLocationIndicator, 300>`, `<tSplineIndicator, 100>`,
/// `<tFollowIndicator, 10>` and `<tTriggerIndicator, 200>`.
const LOCATION_BANK: u32 = 300;
const SPLINE_BANK: u32 = 100;
const FOLLOW_BANK: u32 = 10;
const TRIGGER_BANK: u32 = 200;

/// `cLocationManager::DefaultLocationOrientation` (ctor `0x82a8ef20`): the I, J and K axes
/// and a zero w row.
const DEFAULT_LOCATION_ORIENTATION: [f32; 16] =
    [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

/// `tVisualEditorData` (`0x50` bytes): its `type` and the fields each type reads.
mod editor_data {
    pub const TYPE: u32 = 0x0;
    /// `Indicator` (`eSk8ChallengeIndicators`).
    pub const INDICATOR: u32 = 0x4;
    /// `splineData.SplineID`.
    pub const SPLINE_ID: u32 = 0x10;
    /// `LocData.Location`.
    pub const LOCATION: u32 = 0x18;
    /// `LocData.SplineBankObject.RibbonIndicator.mCollectionKey`.
    pub const RIBBON_INDICATOR: u32 = 0x28;
    /// `LocData.SplineBankObject.RibbonColour.mCollectionKey`.
    pub const RIBBON_COLOUR: u32 = 0x40;
    /// `eChallengeIndicatorType_Location`.
    pub const LOCATION_TYPE: u32 = 0;
    /// `eChallengeIndicatorType_Spline`.
    pub const SPLINE_TYPE: u32 = 1;
}

/// `tSplineBankObject` (`0x30` bytes): its `RibbonIndicator` and `RibbonColour` collection keys.
mod spline_bank_object {
    pub const RIBBON_INDICATOR: u32 = 0x8;
    pub const RIBBON_COLOUR: u32 = 0x20;
}

/// `tLocationIndicator`.
#[derive(Clone, Debug, PartialEq)]
pub struct LocationIndicator {
    /// `Transform`: `cLocationManager::GetLocationOrientation` (`0x82a8e3b0`) of the location.
    pub transform: [f32; 16],
    pub ribbon_indicator_key: u64,
    pub ribbon_colour_key: u64,
    pub enabled: bool,
}

/// `tSplineIndicator`, held under its `AssetID`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplineIndicator {
    /// `IndicatorData` (`eSk8ChallengeIndicators`).
    pub indicator: i32,
    pub enabled: bool,
}

/// `tFollowIndicator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FollowIndicator {
    pub player_id: i32,
    pub enabled: bool,
}

/// `tTriggerIndicator`, held under its trigger volume ID; its `TriggerData` is that volume's
/// `cTriggerVolumeManager::GetTriggerInstance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerIndicator {
    /// `IndicatorData` (`eSk8ChallengeIndicators`).
    pub indicator: i32,
    pub ribbon_indicator_key: u64,
    pub ribbon_colour_key: u64,
    pub enabled: bool,
}

/// `cVisualIndicatorGroup` (`0x1f28` bytes).
#[derive(Clone, Debug)]
pub struct IndicatorGroup {
    pub id: u32,
    /// `mKey`: the owner's unique ID.
    pub key: u64,
    /// `mEnabled`.
    pub enabled: bool,
    /// `LocationIndicators`, under `GetHashValue32` of the location name.
    pub locations: BTreeMap<u32, LocationIndicator>,
    /// `SplineIndicators`, under the spline asset ID.
    pub splines: BTreeMap<u64, SplineIndicator>,
    /// `FollowIndicators`.
    pub follows: Vec<FollowIndicator>,
    /// `TriggerIndicators`, in trigger volume ID order.
    pub triggers: Vec<(f32, TriggerIndicator)>,
}

/// The free entries of the four banks.
#[derive(Clone, Copy, Debug)]
struct Banks {
    locations: u32,
    splines: u32,
    follows: u32,
    triggers: u32,
}

/// `cFixedMemStore::GetBlock`: an entry while the bank has one.
fn take(free: &mut u32) -> bool {
    let available = *free > 0;
    if available {
        *free -= 1;
    }
    available
}

#[expect(clippy::cast_possible_truncation, reason = "the banks hold at most 300 entries")]
fn count(len: usize) -> u32 {
    len as u32
}

#[derive(Debug)]
pub struct IndicatorManager {
    /// `VisualIndicatorGroups`: a group goes after the groups under keys not above its own.
    pub groups: Vec<IndicatorGroup>,
    next_id: u32,
    free: Banks,
}

impl Default for IndicatorManager {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            next_id: 0,
            free: Banks { locations: LOCATION_BANK, splines: SPLINE_BANK, follows: FOLLOW_BANK, triggers: TRIGGER_BANK },
        }
    }
}

impl IndicatorManager {
    /// `CreateGroup` (`0x82dae978`): a new, enabled group under the key.
    pub fn create_group(&mut self, key: u64) -> u32 {
        self.next_id += 1;
        let id = self.next_id;
        let at = self.groups.partition_point(|g| g.key <= key);
        self.groups.insert(
            at,
            IndicatorGroup {
                id,
                key,
                enabled: true,
                locations: BTreeMap::new(),
                splines: BTreeMap::new(),
                follows: Vec::new(),
                triggers: Vec::new(),
            },
        );
        id
    }

    #[must_use]
    pub fn group(&self, id: u32) -> Option<&IndicatorGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn group_mut(&mut self, id: u32) -> Option<&mut IndicatorGroup> {
        self.groups.iter_mut().find(|g| g.id == id)
    }

    /// `GetFirstGroup` (`0x82da44a8`): the first group under the key.
    #[must_use]
    pub fn first_group(&self, key: u64) -> Option<u32> {
        self.groups.iter().find(|g| g.key == key).map(|g| g.id)
    }

    /// `cVisualIndicatorGroup::ClearAll` (`0x82da5eb0`): every indicator back to its bank.
    pub fn clear_all(&mut self, id: u32) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        free.locations += count(g.locations.len());
        free.splines += count(g.splines.len());
        free.follows += count(g.follows.len());
        free.triggers += count(g.triggers.len());
        g.locations.clear();
        g.splines.clear();
        g.follows.clear();
        g.triggers.clear();
    }

    /// `DeleteGroup` (`0x82dab3b0`).
    pub fn delete_group(&mut self, id: u32) {
        if self.group(id).is_some() {
            self.clear_all(id);
            self.groups.retain(|g| g.id != id);
        }
    }

    /// `DeleteGroups` (`0x82dab498`): every group under the key.
    pub fn delete_groups(&mut self, key: u64) {
        let ids: Vec<u32> = self.groups.iter().filter(|g| g.key == key).map(|g| g.id).collect();
        for id in ids {
            self.delete_group(id);
        }
    }

    /// `cVisualIndicatorGroup::AddLocationIndicator` (`0x82db25d0`): a location the group does
    /// not hold yet, at its orientation; a colour key of -1 is the `default` colour.
    pub fn add_location(&mut self, id: u32, location: u32, transform: [f32; 16], ribbon_indicator_key: u64, ribbon_colour_key: u64) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        if g.locations.contains_key(&location) || !take(&mut free.locations) {
            return;
        }
        let ribbon_colour_key = if ribbon_colour_key == u64::MAX { hash64("default") } else { ribbon_colour_key };
        g.locations.insert(location, LocationIndicator { transform, ribbon_indicator_key, ribbon_colour_key, enabled: true });
    }

    /// `cVisualIndicatorGroup::RemoveLocationIndicator` (`0x82da5d08`).
    pub fn remove_location(&mut self, id: u32, location: u32) {
        let Self { groups, free, .. } = self;
        if let Some(g) = groups.iter_mut().find(|g| g.id == id)
            && g.locations.remove(&location).is_some()
        {
            free.locations += 1;
        }
    }

    /// `cVisualIndicatorGroup::AddSplineIndicator` (`0x82db2778`): the spline's indicator, from
    /// the bank when the group does not hold it.
    pub fn add_spline(&mut self, id: u32, asset: u64, indicator: i32) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        if !g.splines.contains_key(&asset) && !take(&mut free.splines) {
            return;
        }
        g.splines.insert(asset, SplineIndicator { indicator, enabled: true });
    }

    /// `cVisualIndicatorGroup::RemoveSplineIndicator` (`0x82da5d90`).
    pub fn remove_spline(&mut self, id: u32, asset: u64) {
        let Self { groups, free, .. } = self;
        if let Some(g) = groups.iter_mut().find(|g| g.id == id)
            && g.splines.remove(&asset).is_some()
        {
            free.splines += 1;
        }
    }

    /// `cVisualIndicatorGroup::AddFollowTarget` (`0x82daea38`): the player's indicator, after
    /// any it had.
    pub fn add_follow(&mut self, id: u32, player: i32) {
        self.remove_follow(id, player);
        let Self { groups, free, .. } = self;
        if let Some(g) = groups.iter_mut().find(|g| g.id == id)
            && take(&mut free.follows)
        {
            g.follows.push(FollowIndicator { player_id: player, enabled: true });
        }
    }

    /// `cVisualIndicatorGroup::RemoveFollowTarget` (`0x82dab5e8`): the player's first indicator.
    pub fn remove_follow(&mut self, id: u32, player: i32) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        if let Some(at) = g.follows.iter().position(|f| f.player_id == player) {
            g.follows.remove(at);
            free.follows += 1;
        }
    }

    /// `cVisualIndicatorGroup::AddTriggerIndicator` (`0x82db26b8`): the volume's indicator (a
    /// new one from the bank has no ribbon), retyped and enabled, with the spline bank object's
    /// ribbon when one is given.
    pub fn add_trigger(&mut self, id: u32, volume: f32, indicator: i32, ribbon: Option<(u64, u64)>) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        let at = match g.triggers.iter().position(|(v, _)| *v == volume) {
            Some(at) => at,
            None => {
                if !take(&mut free.triggers) {
                    return;
                }
                let at = g.triggers.partition_point(|(v, _)| *v < volume);
                let unused = TriggerIndicator {
                    indicator: INDICATOR_UNKNOWN,
                    ribbon_indicator_key: u64::MAX,
                    ribbon_colour_key: u64::MAX,
                    enabled: false,
                };
                g.triggers.insert(at, (volume, unused));
                at
            }
        };
        let trigger = &mut g.triggers[at].1;
        trigger.indicator = indicator;
        trigger.enabled = true;
        if let Some((ribbon_indicator_key, ribbon_colour_key)) = ribbon {
            trigger.ribbon_indicator_key = ribbon_indicator_key;
            trigger.ribbon_colour_key = ribbon_colour_key;
        }
    }

    /// `VisualIndicators.RemoveTriggerIndicator`'s (`0x82da6228`) erase of the volume's indicator.
    pub fn remove_trigger(&mut self, id: u32, volume: f32) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        if let Some(at) = g.triggers.iter().position(|(v, _)| *v == volume) {
            g.triggers.remove(at);
            free.triggers += 1;
        }
    }

    /// `cVisualIndicatorGroup::RemoveByTypeTriggers` (`0x82da5e00`).
    pub fn remove_triggers_of_type(&mut self, id: u32, indicator: i32) {
        let Self { groups, free, .. } = self;
        let Some(g) = groups.iter_mut().find(|g| g.id == id) else { return };
        let before = g.triggers.len();
        g.triggers.retain(|(_, t)| t.indicator != indicator);
        free.triggers += count(before - g.triggers.len());
    }

    /// `GetVisibleIndicators<T>` (`0x82db2210`, `0x82da8a20`, `0x82da8b68`, `0x82da8cb0`): the
    /// groups under the main challenge's unique ID (-1 without one), then every group under
    /// another key, each in multimap order.
    fn visible_order(&self, main: Option<u64>) -> impl Iterator<Item = &IndicatorGroup> {
        let key = main.unwrap_or(u64::MAX);
        self.groups
            .iter()
            .filter(move |g| g.key == key)
            .chain(self.groups.iter().filter(move |g| g.key != key))
    }

    /// `GetVisibleLocationIndicators` (`0x82db2568`): each group's visible locations until
    /// `max` are held.
    #[must_use]
    pub fn visible_locations(
        &self,
        main: Option<u64>,
        point: [f32; 3],
        range: f32,
        max: usize,
    ) -> Vec<&LocationIndicator> {
        let mut visible = Vec::new();
        for group in self.visible_order(main) {
            let more = group.visible_locations(point, range, max - visible.len());
            visible.extend(more);
            if visible.len() >= max {
                break;
            }
        }
        visible
    }

    /// `GetVisibleSplineIndicators` (`0x82da93f8`): `(AssetID, IndicatorData)`.
    #[must_use]
    pub fn visible_splines(&self, main: Option<u64>, max: usize) -> Vec<(u64, i32)> {
        let mut visible = Vec::new();
        for group in self.visible_order(main) {
            if group.enabled {
                // AddIndicatorsFromGroup<tSplineIndicator> 0x82da4518: no distance test.
                let remaining = max - visible.len();
                visible.extend(
                    group
                        .splines
                        .iter()
                        .filter(|(_, s)| s.enabled)
                        .map(|(&asset, s)| (asset, s.indicator))
                        .take(remaining),
                );
            }
            if visible.len() >= max {
                break;
            }
        }
        visible
    }

    /// `GetVisibleFollowIndicators` (`0x82da9460`): the followed players.
    #[must_use]
    pub fn visible_follows(&self, main: Option<u64>, max: usize) -> Vec<i32> {
        let mut visible = Vec::new();
        for group in self.visible_order(main) {
            if group.enabled {
                // AddIndicatorsFromGroup<tFollowIndicator> 0x82da2b00.
                let remaining = max - visible.len();
                visible.extend(
                    group
                        .follows
                        .iter()
                        .filter(|f| f.enabled)
                        .map(|f| f.player_id)
                        .take(remaining),
                );
            }
            if visible.len() >= max {
                break;
            }
        }
        visible
    }

    /// `GetVisibleTriggerIndicators` (`0x82da94c8`): `(volume, indicator)`.
    #[must_use]
    pub fn visible_triggers(&self, main: Option<u64>, max: usize) -> Vec<(f32, TriggerIndicator)> {
        let mut visible = Vec::new();
        for group in self.visible_order(main) {
            if group.enabled {
                // AddIndicatorsFromGroup<tTriggerIndicator> 0x82da45a8: types Obstacle (1) to
                // ChallengeBoundary (8).
                let remaining = max - visible.len();
                let shown = group
                    .triggers
                    .iter()
                    .filter(|(_, t)| t.enabled && (1..=8).contains(&t.indicator));
                visible.extend(shown.copied().take(remaining));
            }
            if visible.len() >= max {
                break;
            }
        }
        visible
    }
}

impl IndicatorGroup {
    /// `cVisualIndicatorGroup::AddIndicatorsFromGroup<tLocationIndicator>` (`0x82db0c30`):
    /// the enabled locations strictly within `range` of the point, nearest first
    /// (`quick_sort` with `SortLocationIndicatorOnDistanceToPointOfInterest`), at most
    /// `remaining` of them.
    fn visible_locations(
        &self,
        point: [f32; 3],
        range: f32,
        remaining: usize,
    ) -> Vec<&LocationIndicator> {
        if !self.enabled {
            return Vec::new();
        }
        let distance = |l: &LocationIndicator| {
            let d = [
                l.transform[12] - point[0],
                l.transform[13] - point[1],
                l.transform[14] - point[2],
                0.0,
            ];
            crate::reckoning_numeric::host_dot3(d, d)
        };
        let range_squared = range * range;
        let mut held: Vec<&LocationIndicator> = self
            .locations
            .values()
            .filter(|l| l.enabled && distance(l) - range_squared < 0.0)
            .collect();
        // The comparator is `fsel(d(b) - d(a), 1, 0) != 0`: equal distances count as ordered.
        let _ = crate::native_execution_sort::sort(&mut held, &mut |a, b| {
            Ok::<_, std::convert::Infallible>(distance(b) - distance(a) >= 0.0)
        });
        held.truncate(remaining);
        held
    }
}

fn data_u32(db: &AttribDb, data: Location, offset: u32) -> u32 {
    db.bytes(data.offset_by(offset), 4).map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn data_u64(db: &AttribDb, data: Location, offset: u32) -> u64 {
    db.bytes(data.offset_by(offset), 8).map_or(0, |b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// `cLocationManager::GetLocationOrientation` (`0x82a8e3b0`): the host's registered location
/// (a challenge stream's locators register with its arenas), else the loaded location
/// table's, else `DefaultLocationOrientation`.
fn location_orientation(world: &mut World, location: u32) -> [f32; 16] {
    world
        .host
        .location_orientation(location)
        .or_else(|| world.locations.get(&location).copied())
        .unwrap_or(DEFAULT_LOCATION_ORIENTATION)
}

/// `cVisualIndicatorGroup::AddLocationIndicator` (`0x82db25d0`) of a named location.
pub(crate) fn add_location_indicator(world: &mut World, group: u32, name: &str, ribbon_indicator_key: u64, ribbon_colour_key: u64) {
    let location = hash_value32(name);
    let transform = location_orientation(world, location);
    world.indicators.add_location(group, location, transform, ribbon_indicator_key, ribbon_colour_key);
}

/// `cVisualIndicatorGroup::AddVisualIndicator` (`0x82db2808`): a `tVisualEditorData`'s
/// location or spline indicator.
pub(crate) fn add_visual_indicator(world: &mut World, group: u32, data: Location) {
    let db = std::sync::Arc::clone(&world.db);
    match data_u32(&db, data, editor_data::TYPE) {
        editor_data::LOCATION_TYPE => {
            let Some(name) = db.text_at(data.offset_by(editor_data::LOCATION)) else { return };
            let indicator = data_u64(&db, data, editor_data::RIBBON_INDICATOR);
            let colour = data_u64(&db, data, editor_data::RIBBON_COLOUR);
            add_location_indicator(world, group, name, indicator, colour);
        }
        editor_data::SPLINE_TYPE => {
            #[expect(clippy::cast_possible_wrap, reason = "eSk8ChallengeIndicators")]
            let indicator = data_u32(&db, data, editor_data::INDICATOR) as i32;
            world.indicators.add_spline(group, data_u64(&db, data, editor_data::SPLINE_ID), indicator);
        }
        _ => {}
    }
}

/// `cVisualIndicatorGroup::RemoveVisualIndicator` (`0x82da60a0`).
pub(crate) fn remove_visual_indicator(world: &mut World, group: u32, data: Location) {
    let db = std::sync::Arc::clone(&world.db);
    match data_u32(&db, data, editor_data::TYPE) {
        editor_data::LOCATION_TYPE => {
            if let Some(name) = db.text_at(data.offset_by(editor_data::LOCATION)) {
                world.indicators.remove_location(group, hash_value32(name));
            }
        }
        editor_data::SPLINE_TYPE => world.indicators.remove_spline(group, data_u64(&db, data, editor_data::SPLINE_ID)),
        _ => {}
    }
}

/// `cVisualIndicatorManager::CompleteChallenge` (`0x82dad738`): the main activity's groups go.
pub(crate) fn complete_challenge(world: &mut World, activity: usize, is_main: bool) {
    if is_main {
        let key = world.activities.activities[activity].key;
        world.indicators.delete_groups(key);
    }
}

/// `cVisualIndicatorManager::ShutdownActivity` (`0x82dad780`).
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    let key = world.activities.activities[activity].key;
    world.indicators.delete_groups(key);
}

fn group_arg(world: &World, value: &Value) -> Option<u32> {
    world.pointers.object(value, ObjectKind::VisualIndicatorGroup)
}

/// `tLocationID` behind a `lightuserdata`: its `LocationName`.
fn location_arg(world: &World, value: &Value) -> Option<String> {
    let at = attrib_location(&world.pointers, value)?;
    world.db.text_at(at).map(str::to_owned)
}

/// `luaL_checkudata(index, "RefSpecMetaTable")`.
fn check_ref_spec(lua: &Lua<World>, call: &NativeCall, n: usize, function: &str) -> LuaResult<RefSpec> {
    lua.userdata_payload::<RefSpec>(call.arg(n - 1))
        .copied()
        .ok_or_else(|| stdlib::type_error(lua, call, n, function, REF_SPEC_METATABLE))
}

/// The calling activity's unique ID (`cActivity::GetUniqueID` of the closure's activity).
fn calling_activity_key(world: &World) -> Option<u64> {
    world.closure.map(|activity| world.activities.activities[activity].key)
}

/// `VisualIndicators.CreateGroup()` (`0x82daead8`): a group under the calling activity.
fn create_group(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let Some(key) = calling_activity_key(world) else { return Ok(vec![Value::Nil]) };
    let id = world.indicators.create_group(key);
    Ok(vec![Value::LightUserData(world.pointers.handle(Pointee::Object { kind: ObjectKind::VisualIndicatorGroup, id }))])
}

/// `VisualIndicators.DeleteGroup(group)` (`0x82dab688`).
fn delete_group(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = group_arg(world, call.arg(0)) {
        world.indicators.delete_group(group);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.AddIndicator(group, tVisualEditorData)` (`0x82db2870`).
fn add_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let (Some(group), Some(data)) = (group_arg(world, call.arg(0)), attrib_location(&world.pointers, call.arg(1))) {
        add_visual_indicator(world, group, data);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.RemoveIndicator(group, tVisualEditorData)` (`0x82da60c8`).
fn remove_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let (Some(group), Some(data)) = (group_arg(world, call.arg(0)), attrib_location(&world.pointers, call.arg(1))) {
        remove_visual_indicator(world, group, data);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.AddLocationIndicator(group, tLocationID, RefSpec)` (`0x82db2948`): the
/// location under the ribbon, in the default colour.
fn add_location_indicator_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = group_arg(world, call.arg(0));
    let location = location_arg(world, call.arg(1));
    let ribbon = check_ref_spec(lua, call, 3, "AddLocationIndicator")?;
    if let (Some(group), Some(location)) = (group, location) {
        add_location_indicator(world, group, &location, ribbon.collection, u64::MAX);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.RemoveLocationIndicator(group, tLocationID)` (`0x82da6190`).
fn remove_location_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let (Some(group), Some(location)) = (group_arg(world, call.arg(0)), location_arg(world, call.arg(1))) {
        world.indicators.remove_location(group, hash_value32(&location));
    }
    Ok(Vec::new())
}

/// `VisualIndicators.ChangeIndicatorColour(group, tVisualEditorData, RefSpec)` (`0x82da4650`):
/// a held location indicator's colour.
fn change_indicator_colour(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let group = group_arg(world, call.arg(0));
    let data = attrib_location(&world.pointers, call.arg(1));
    let colour = check_ref_spec(lua, call, 3, "ChangeIndicatorColour")?;
    let (Some(group), Some(data)) = (group, data) else { return Ok(Vec::new()) };
    if data_u32(&world.db, data, editor_data::TYPE) != editor_data::LOCATION_TYPE {
        return Ok(Vec::new());
    }
    let Some(location) = world.db.text_at(data.offset_by(editor_data::LOCATION)).map(hash_value32) else { return Ok(Vec::new()) };
    if let Some(indicator) = world.indicators.group_mut(group).and_then(|g| g.locations.get_mut(&location)) {
        indicator.ribbon_colour_key = colour.collection;
    }
    Ok(Vec::new())
}

/// `VisualIndicators.AddTriggerIndicator(group, volume, type, tSplineBankObject)` (`0x82db2a00`).
fn add_trigger_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(group) = group_arg(world, call.arg(0)) else { return Ok(Vec::new()) };
    let volume = get_float(call, 1);
    let indicator = get_int_or(call, 2, 0);
    let ribbon = attrib_location(&world.pointers, call.arg(3)).map(|at| {
        (data_u64(&world.db, at, spline_bank_object::RIBBON_INDICATOR), data_u64(&world.db, at, spline_bank_object::RIBBON_COLOUR))
    });
    world.indicators.add_trigger(group, volume, indicator, ribbon);
    Ok(Vec::new())
}

/// `VisualIndicators.RemoveTriggerIndicator(group, volume)` (`0x82da6228`).
fn remove_trigger_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = group_arg(world, call.arg(0)) {
        world.indicators.remove_trigger(group, get_float(call, 1));
    }
    Ok(Vec::new())
}

/// `VisualIndicators.RemoveByType(group, type)` (`0x82da62e0`): type `None` removes nothing.
fn remove_by_type(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let indicator = get_int_or(call, 1, 0);
    if let Some(group) = group_arg(world, call.arg(0))
        && indicator != 0
    {
        world.indicators.remove_triggers_of_type(group, indicator);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.Enable(group, enabled)` (`0x82da2b78`): `mEnabled`.
fn enable(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let enabled = get_bool(call, 1);
    if let Some(group) = group_arg(world, call.arg(0)).and_then(|id| world.indicators.group_mut(id)) {
        group.enabled = enabled;
    }
    Ok(Vec::new())
}

/// `VisualIndicators.AddFollowIndicator(group, player)` (`0x82daeb78`).
fn add_follow_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = group_arg(world, call.arg(0)) {
        world.indicators.add_follow(group, get_int(call, 1));
    }
    Ok(Vec::new())
}

/// `VisualIndicators.RemoveFollowIndicator(group, player)` (`0x82dab728`).
fn remove_follow_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = group_arg(world, call.arg(0)) {
        world.indicators.remove_follow(group, get_int(call, 1));
    }
    Ok(Vec::new())
}

/// `VisualIndicators.ClearAllIndicators(group)` (`0x82da6370`).
fn clear_all_indicators(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let Some(group) = group_arg(world, call.arg(0)) {
        world.indicators.clear_all(group);
    }
    Ok(Vec::new())
}

/// `VisualIndicators.SetObjectiveVisualIndicator(objectiveGroup, index, show)` (`0x82da2bf8`):
/// `cChallengeObjectiveGroup::SetObjectiveVisualIndicator` (`0x82d43510`).
fn set_objective_visual_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let index = get_int(call, 1);
    let show = get_bool(call, 2);
    if let (Some(group), Ok(index)) = (world.pointers.object(call.arg(0), ObjectKind::ObjectiveGroup), usize::try_from(index)) {
        objectives::override_visual_indicator_status(world, group, index, show);
    }
    Ok(Vec::new())
}

/// `Challenge.AddVisualIndicator(tVisualEditorData)` (`0x82892bf8`): into the calling
/// activity's first group, created when it has none.
fn challenge_add_visual_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let (Some(key), Some(data)) = (calling_activity_key(world), attrib_location(&world.pointers, call.arg(0))) else {
        return Ok(Vec::new());
    };
    let group = world.indicators.first_group(key).unwrap_or_else(|| world.indicators.create_group(key));
    add_visual_indicator(world, group, data);
    Ok(Vec::new())
}

/// `Challenge.RemoveVisualIndicator(tVisualEditorData)` (`0x82892d20`): from the calling
/// activity's first group.
fn challenge_remove_visual_indicator(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    if let (Some(key), Some(data)) = (calling_activity_key(world), attrib_location(&world.pointers, call.arg(0)))
        && let Some(group) = world.indicators.first_group(key)
    {
        remove_visual_indicator(world, group, data);
    }
    Ok(Vec::new())
}

pub fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("VisualIndicators", "CreateGroup", create_group),
        ("VisualIndicators", "DeleteGroup", delete_group),
        ("VisualIndicators", "AddIndicator", add_indicator),
        ("VisualIndicators", "RemoveIndicator", remove_indicator),
        ("VisualIndicators", "AddLocationIndicator", add_location_indicator_native),
        ("VisualIndicators", "RemoveLocationIndicator", remove_location_indicator),
        ("VisualIndicators", "ChangeIndicatorColour", change_indicator_colour),
        ("VisualIndicators", "AddTriggerIndicator", add_trigger_indicator),
        ("VisualIndicators", "RemoveTriggerIndicator", remove_trigger_indicator),
        ("VisualIndicators", "RemoveByType", remove_by_type),
        ("VisualIndicators", "Enable", enable),
        ("VisualIndicators", "AddFollowIndicator", add_follow_indicator),
        ("VisualIndicators", "RemoveFollowIndicator", remove_follow_indicator),
        ("VisualIndicators", "ClearAllIndicators", clear_all_indicators),
        ("VisualIndicators", "SetObjectiveVisualIndicator", set_objective_visual_indicator),
        ("Challenge", "AddVisualIndicator", challenge_add_visual_indicator),
        ("Challenge", "RemoveVisualIndicator", challenge_remove_visual_indicator),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banks_bound_every_group_together() {
        let mut manager = IndicatorManager::default();
        let a = manager.create_group(7);
        let b = manager.create_group(7);
        for location in 0..LOCATION_BANK {
            manager.add_location(if location % 2 == 0 { a } else { b }, location, DEFAULT_LOCATION_ORIENTATION, 1, u64::MAX);
        }
        manager.add_location(a, LOCATION_BANK, DEFAULT_LOCATION_ORIENTATION, 1, u64::MAX);
        assert!(!manager.group(a).unwrap().locations.contains_key(&LOCATION_BANK));
        manager.delete_groups(7);
        assert!(manager.groups.is_empty());
        let c = manager.create_group(0);
        manager.add_location(c, LOCATION_BANK, DEFAULT_LOCATION_ORIENTATION, 1, 5);
        let held = &manager.group(c).unwrap().locations[&LOCATION_BANK];
        assert_eq!((held.ribbon_indicator_key, held.ribbon_colour_key, held.enabled), (1, 5, true));
    }

    #[test]
    fn a_location_is_added_once_and_a_minus_one_colour_is_the_default() {
        let mut manager = IndicatorManager::default();
        let g = manager.create_group(1);
        manager.add_location(g, 9, DEFAULT_LOCATION_ORIENTATION, 2, u64::MAX);
        manager.add_location(g, 9, DEFAULT_LOCATION_ORIENTATION, 3, 4);
        let held = &manager.group(g).unwrap().locations[&9];
        assert_eq!((held.ribbon_indicator_key, held.ribbon_colour_key), (2, hash64("default")));
    }

    #[test]
    fn trigger_indicators_keep_their_ribbon_and_go_by_type() {
        let mut manager = IndicatorManager::default();
        let g = manager.create_group(1);
        manager.add_trigger(g, 3.0, 2, None);
        manager.add_trigger(g, 1.0, 3, Some((10, 11)));
        manager.add_trigger(g, 1.0, 4, None);
        let triggers = &manager.group(g).unwrap().triggers;
        assert_eq!(triggers.iter().map(|(v, _)| *v).collect::<Vec<_>>(), [1.0, 3.0]);
        assert_eq!(triggers[0].1, TriggerIndicator { indicator: 4, ribbon_indicator_key: 10, ribbon_colour_key: 11, enabled: true });
        assert_eq!(triggers[1].1.ribbon_indicator_key, u64::MAX);
        manager.remove_triggers_of_type(g, 2);
        assert_eq!(manager.group(g).unwrap().triggers.len(), 1);
    }

    #[test]
    fn groups_follow_the_groups_under_lower_or_equal_keys() {
        let mut manager = IndicatorManager::default();
        let a = manager.create_group(5);
        let b = manager.create_group(0);
        let c = manager.create_group(5);
        assert_eq!(manager.groups.iter().map(|g| g.id).collect::<Vec<_>>(), [b, a, c]);
        assert_eq!(manager.first_group(5), Some(a));
    }
}
