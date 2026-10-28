//! `Sk8::Challenge::cChallengeLivingWorldManager` (0xb8 bytes): the census settings and living
//! world operations of the running challenge. Its starting challenge sets the census spawners
//! from `LivingWorldManagement`, culls or moves the challenge characters
//! (`ForceCullChallengeSpawnedLivingWorld`) and runs its `LivingWorldOperations`: cull every
//! census pedestrian and vehicle, clear an area of them, or also keep them out of it (the
//! avoid volumes the pedestrians' navigation reads through `GetWorldAvoidVolumes`). Its
//! completion turns every spawner back on.
//!
//! The census and trigger volumes are the session's: the changes go out as
//! [`LivingWorldOp`]s in the character manager's queue.

use super::{
    characters::{self, LivingWorldOp},
    keys::class,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    system::World,
};

/// The attributes the manager reads.
pub(crate) mod key {
    /// `challenges.LivingWorldManagement` (`tLivingWorldManagement`).
    pub(crate) const LIVING_WORLD_MANAGEMENT: u64 = 0x9154_a9b6_5bba_a7b9;
    /// `challenges.LivingWorldOperations` (`tLivingWorldManagementOperation[]`).
    pub(crate) const LIVING_WORLD_OPERATIONS: u64 = 0x2e5d_dc8a_3cbe_fe6e;
}

/// `eLivingWorldManagementOperations`.
pub(crate) mod operation {
    pub(crate) const CULL_ALL: u32 = 0;
    pub(crate) const CLEAR_AREA: u32 = 1;
    pub(crate) const AVOID_AREA: u32 = 2;
}

/// `CensusMan`'s enable byte (+0xa8) bits `SetLWManagementValues` sets from
/// `tLivingWorldManagement::Value` bits 0 to 5: `EnableCensusSpawner` (0x8271bfe0),
/// `EnablePedestrianSpawn`, `EnablePropDrivenPedestrianSpawn`, `EnableVehicleSpawn`,
/// `EnableDynamicObjectSpawn` and `EnablePropSpawn` (0x8271c058).
const CENSUS_BITS: [u8; 6] = [0x40, 0x20, 0x10, 0x08, 0x04, 0x02];

/// `cChallengeLivingWorldManager`'s state.
#[derive(Debug, Default)]
pub struct LivingWorldManager {
    /// `Activated`: career (4) and Party Play (16) games run the operations.
    activated: bool,
    /// `GameTypeFlags`.
    game_type_flags: u32,
    /// `SkipInitialSpawn`: a restart's operations ran, so `StateLoading::CharacterPrepareState`
    /// skips its initial census.
    skip_initial_spawn: bool,
    /// `AvoidVolumeList` (`fixed_vector<uint64, 16>`): trigger volume IDs.
    avoid_volumes: Vec<u64>,
}

impl LivingWorldManager {
    /// `AvoidVolumeList`.
    #[must_use]
    pub fn avoid_volumes(&self) -> &[u64] {
        &self.avoid_volumes
    }

    /// `SkipInitialSpawn`.
    #[must_use]
    pub const fn skip_initial_spawn(&self) -> bool {
        self.skip_initial_spawn
    }

    /// `StateLoading::CharacterPrepareState` (0x82760058) clears `SkipInitialSpawn` on entering.
    pub const fn clear_skip_initial_spawn(&mut self) {
        self.skip_initial_spawn = false;
    }
}

/// `GameLoad` (0x825ad758).
pub(crate) fn game_load(world: &mut World, game_type_flags: u32) {
    let m = &mut world.living_world;
    if game_type_flags == 4 || game_type_flags == 16 {
        m.activated = true;
    }
    m.game_type_flags = game_type_flags;
    m.skip_initial_spawn = false;
}

/// `GameUnload` (0x825b6080).
pub(crate) fn game_unload(world: &mut World) {
    let m = &mut world.living_world;
    m.activated = false;
    m.game_type_flags = 0;
    m.avoid_volumes.clear();
    m.skip_initial_spawn = false;
    characters::op(world, LivingWorldOp::AvoidVolumesCleared);
}

/// `SetLWManagementValues` (0x825ad780): the census spawners from the value's low six bits.
fn set_management_values(world: &mut World, value: u32) {
    let mut bits = 0;
    for (bit, census) in CENSUS_BITS.into_iter().enumerate() {
        if value >> bit & 1 != 0 {
            bits |= census;
        }
    }
    characters::op(
        world,
        LivingWorldOp::CensusEnable {
            mask: CENSUS_BITS.iter().fold(0, |m, b| m | b),
            value: bits,
        },
    );
}

/// `ReevaluateActivityPrereqs` (0x825ad840): `EnableCensusSpawner(true)`.
pub(crate) fn reevaluate_activity_prereqs(world: &mut World) {
    characters::op(
        world,
        LivingWorldOp::CensusEnable {
            mask: CENSUS_BITS[0],
            value: CENSUS_BITS[0],
        },
    );
}

/// `IsSignupAvailable` (0x825ad858), the manager's `ISignupSubController` answer: no chase.
#[must_use]
pub fn is_signup_available(world: &World) -> bool {
    !world
        .characters
        .facts()
        .is_some_and(|facts| facts.chase_is_on())
}

/// `RunMainActivity` (0x825c0da0): the challenge's `LivingWorldManagement` (`DefaultDataArea`:
/// zero when absent), its characters, then its operations.
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    let key = world.activities.activities[activity].key;
    let value = world
        .db
        .attribute(class::CHALLENGES, key, key::LIVING_WORLD_MANAGEMENT)
        .and_then(|a| a.u32(0))
        .unwrap_or(0);
    set_management_values(world, value);
    characters::force_cull_challenge_spawned_living_world(world, activity);
    execute_operations(world, activity, false);
}

/// `CompleteChallenge` (0x825b7640) of the main challenge: every spawner back on (`Value |= 0x3f`
/// of an uninitialised value: the six bits read are set) and no avoid volumes.
pub(crate) fn complete_challenge(world: &mut World, is_main: bool) {
    if is_main {
        set_management_values(world, 0x3f);
        world.living_world.avoid_volumes.clear();
        characters::op(world, LivingWorldOp::AvoidVolumesCleared);
    }
}

/// `RestartChallenge` (0x825bf220): the operations again, skipping the next initial spawn.
pub(crate) fn restart_challenge(world: &mut World, activity: usize) {
    execute_operations(world, activity, true);
}

/// `ExecuteLivingWorldOperations` (0x825bf148).
fn execute_operations(world: &mut World, activity: usize, skip_initial_spawn_on_clear: bool) {
    if !world.living_world.activated {
        return;
    }
    let key = world.activities.activities[activity].key;
    let db = std::sync::Arc::clone(&world.db);
    let Some(operations) = db.attribute(class::CHALLENGES, key, key::LIVING_WORLD_OPERATIONS)
    else {
        return;
    };
    let count = operations.len();
    for index in 0..count {
        let bytes = operations
            .data(index)
            .and_then(|at| db.bytes(at, 24))
            .map(<[u8]>::to_vec)
            .unwrap_or_else(|| vec![0; 24]);
        let kind = u32::from_be_bytes(bytes[0..4].try_into().expect("four bytes"));
        let volume = u64::from_be_bytes(bytes[16..24].try_into().expect("eight bytes"));
        execute_operation(world, kind, volume);
    }
    if count != 0 && skip_initial_spawn_on_clear {
        world.living_world.skip_initial_spawn = true;
    }
}

/// `ExecuteLivingWorldOperation` (0x825bde28) of a `tLivingWorldManagementOperation`: the type
/// and the volume's `VolumeID`. Clearing an area needs its trigger instance; avoiding one also
/// keeps the volume (full 64-bit match) in `AvoidVolumeList`.
pub(crate) fn execute_operation(world: &mut World, kind: u32, volume: u64) {
    let keep = world.characters.challenge_characters();
    match kind {
        operation::CULL_ALL => characters::op(world, LivingWorldOp::CensusManagedCullAll { keep }),
        operation::CLEAR_AREA => characters::op(
            world,
            LivingWorldOp::CensusManagedCullVolume { volume, keep },
        ),
        operation::AVOID_AREA => {
            characters::op(
                world,
                LivingWorldOp::CensusManagedCullVolume { volume, keep },
            );
            if !world.living_world.avoid_volumes.contains(&volume) {
                world.living_world.avoid_volumes.push(volume);
                characters::op(world, LivingWorldOp::AvoidVolumeAdded(volume));
            }
        }
        _ => {}
    }
}

/// `LivingWorld.ForceCullLivingWorld()` (0x82893ec0): the module's manager's
/// `LWManager->RequestCensusManagedCull()`.
fn force_cull_living_world(
    _: &mut Lua<World>,
    world: &mut World,
    _: &NativeCall,
) -> LuaResult<Vec<Value>> {
    let keep = world.characters.challenge_characters();
    characters::op(world, LivingWorldOp::CensusManagedCullAll { keep });
    Ok(Vec::new())
}

/// `LivingWorld.GetIfPlayerBeingChased()` (0x82883f90): `ChaseMan::ChaseIsOn`.
fn get_if_player_being_chased(
    _: &mut Lua<World>,
    world: &mut World,
    _: &NativeCall,
) -> LuaResult<Vec<Value>> {
    Ok(vec![Value::Boolean(
        world
            .characters
            .facts()
            .is_some_and(|facts| facts.chase_is_on()),
    )])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        (
            "LivingWorld",
            "ForceCullLivingWorld",
            force_cull_living_world,
        ),
        (
            "LivingWorld",
            "GetIfPlayerBeingChased",
            get_if_player_being_chased,
        ),
    ]
}
