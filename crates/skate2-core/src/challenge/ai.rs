//! `Sk8::Challenge::ChallengeAIManager` (0x7f8 bytes) with the AI half of
//! `cChallengeCompetitorsManager`: a challenge's competitors, the `ai_characters` picked for
//! them (`mSelectedCharacters`, by player) and their recipes, the challenge path ID and its
//! static path package, the round path filter, the AI respawn location and the release of the
//! challenge skaters once the main challenge completes.
//!
//! `AISkaterManager`, `AIPath::PathManager` and `cLocationManager` are singletons the natives
//! reach directly; here they are the session's, shared through [`AiLink`]. The AI
//! controllers' work goes out as [`Output::Ai`] in call order.

use super::{
    attrib_lua::{activity_of, attrib_location},
    binding::{get_bool, get_bool_or, get_int, int},
    keys::class,
    lua::{Lua, LuaResult, NativeCall, NativeFn, Value},
    players::GM_PARTY_PLAY,
    signup::challenge_type,
    skate::skater,
    system::{Output, World},
};
use crate::{
    ai::{
        path_controller::SharedFilter,
        path_id::{self, PathFilter},
        path_manager::PathManager,
        skater_manager::{AiSkaterManager, AmbientCommand},
        vmx::V4,
    },
    players_backend_settings::PlayersBackendSettings,
};
use skate2_formats::ai_path::AiPathId;
use std::sync::{Arc, Mutex, PoisonError};

type R = LuaResult<Vec<Value>>;

/// `ai_characters`.
const AI_CHARACTERS: u64 = 0xe574_78f7_96eb_c28c;
/// `ai_character_groups`.
const AI_CHARACTER_GROUPS: u64 = 0x3c7c_801b_f770_c11a;
/// The root collection `SelectCharacters` takes both classes' leaves under
/// (`StringToKey("default")`).
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;

/// The attributes the manager reads.
mod key {
    /// `challenges.Competitors` (`tCompetitorInfo[7]`).
    pub(super) const COMPETITORS: u64 = 0xfa18_5f4f_0928_02aa;
    /// `challenges` 0x4d8144913c73134a (`tCompetitorInfo[]`): the competitors facing a goofy
    /// local player in a team contest.
    pub(super) const GOOFY_COMPETITORS: u64 = 0x4d81_4491_3c73_134a;
    /// `challenges.ContestTeams`.
    pub(super) const CONTEST_TEAMS: u64 = 0x0e22_fd36_2785_9240;
    /// The two `challenges` flags that take the dynamically chosen competitors.
    pub(super) const DYNAMIC_COMPETITORS: [u64; 2] = [0x4263_ed15_8a50_063e, 0xf3a5_0805_8ae5_148d];
    /// `challenges` 0xf59b3a4d0f2009e6 and 0x25e41f010928bd59: the demo's paths and their
    /// index.
    pub(super) const DEMO_PATHS: u64 = 0xf59b_3a4d_0f20_09e6;
    pub(super) const DEMO_PATHS_INDEX: u64 = 0x25e4_1f01_0928_bd59;
    /// `challenge_local_data`'s path ID: branch, stage, part (`Text`) and index.
    pub(super) const PATH_BRANCH: u64 = 0xb65f_4ad3_c07b_72d4;
    pub(super) const PATH_STAGE: u64 = 0xb017_c514_4a4b_e44f;
    pub(super) const PATH_PART: u64 = 0x7c99_551d_913f_cb8f;
    pub(super) const PATH_INDEX: u64 = 0xed61_fad8_82b3_9551;
    /// `ai_character_groups.Characters` (`RefSpec[30]`).
    pub(super) const CHARACTERS: u64 = 0x36f9_4f67_8bda_08a3;
    /// `ai_characters` 0x4b60ff565334ccd6: the `characters_marquee` recipe.
    pub(super) const RECIPE: u64 = 0x4b60_ff56_5334_ccd6;
    /// `ai_characters` 0x1540c7101eaef5a6, the layout's first field: the `ai_skater_profiles`
    /// profile.
    pub(super) const PROFILE: u64 = 0x1540_c710_1eae_f5a6;
    /// `ai_characters.PercentageWillDoABDTrick`.
    pub(super) const PERCENTAGE_WILL_DO_ABD_TRICK: u64 = 0xccd0_1466_9dd0_9fcb;
    /// `ai_characters` flags `CheckCharacterValidity` reads for a best trick (8) and a death
    /// race (19) challenge.
    pub(super) const BEST_TRICK_FLAG: u64 = 0x9b98_c32f_acc5_eed4;
    pub(super) const DEATH_RACE_FLAG: u64 = 0x9cb3_d4cf_aa41_a187;
    /// `ai_characters.do_demo`, `do_jam` and `do_skate`.
    pub(super) const DO_DEMO: u64 = 0xbd5f_e241_ac8a_31b0;
    pub(super) const DO_JAM: u64 = 0x6bca_82b3_9858_75af;
    pub(super) const DO_SKATE: u64 = 0x68ed_2848_121a_8c82;
}

/// `eChallengeTypes` values the manager tests.
mod challenge {
    pub(super) const DEMO: i32 = 2;
    pub(super) const SKATE: i32 = 4;
    pub(super) const BEST_TRICK: i32 = 8;
    pub(super) const JAM: i32 = 15;
    pub(super) const DEATH_RACE: i32 = 19;
    pub(super) const MEGA_RAMP: i32 = 26;
    pub(super) const STREET_SKATING: i32 = 27;
}

/// `ai_characters` layout words: the short and long HAL names, the name `GetCharacterID`
/// matches and the character's name.
const LAYOUT_NAME: u32 = 8;
const LAYOUT_CHARACTER_NAME: u32 = 9;
/// `ai_characters` layout words of the short and long HAL names.
const LAYOUT_SHORT_HAL: u32 = 6;
const LAYOUT_LONG_HAL: u32 = 7;
/// `ai_characters` layout bytes a street skating (27) and a mega ramp (26) challenge read.
const LAYOUT_STREET_SKATING: u32 = 40;
const LAYOUT_MEGA_RAMP: u32 = 41;
/// `ai_character_groups` layout: the name (word 0) and the default flag (byte 4).
const GROUP_IS_DEFAULT: u32 = 4;

/// `PT_AI_Challenge`.
const PT_AI_CHALLENGE: i32 = 3;

/// `eReleaseAICharacterState_Complete`.
const RELEASE_COMPLETE: u32 = 4;
/// The frames `Update` waits after `ExitAiBasedChallenge`.
const RELEASE_WAIT_FRAMES: u32 = 6;
/// `DataSys::EraseAllButLastNFrames`' count while the challenge skaters go.
const RELEASE_REPLAY_FRAMES: i32 = 2;
/// `AIPathID::SetDescriptionToChar('*')` and the wildcard suffix of `mAIPathID`.
const ANY_DESCRIPTION: &[u8] = b"*";
const ANY_SUFFIX: u16 = 0xffff;

/// `cLocationManager::GetSpawnLocation(name, index)` (`0x82a8dce8`) of the session's location
/// manager.
pub(crate) trait SpawnLocations: Send + Sync {
    fn spawn_location(&self, name: &str, index: i32) -> Option<[f32; 16]>;
    /// `GetNumSpawnLocations` (`0x82a8e250`).
    fn num_spawn_locations(&self, name: &str) -> usize;
}

/// The singletons the challenge AI shares with the simulation, installed by the session.
#[derive(Clone)]
pub(crate) struct AiLink {
    pub(crate) manager: Arc<Mutex<AiSkaterManager>>,
    pub(crate) paths: Arc<Mutex<PathManager>>,
    pub(crate) settings: Arc<PlayersBackendSettings>,
    pub(crate) locations: Arc<dyn SpawnLocations>,
}

impl AiLink {
    fn manager(&self) -> std::sync::MutexGuard<'_, AiSkaterManager> {
        self.manager.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn paths(&self) -> std::sync::MutexGuard<'_, PathManager> {
        self.paths.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A challenge AI request of the simulation's AI skaters, applied after the update in call
/// order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiRequest {
    /// `AI.StartPath`: `SetBehavior<FollowPathBehavior>`, `SetPathID(mAIPathID)` and the branch
    /// mode (`BranchRandom` when `branch`, else `DoNotBranch`); with the filter active the
    /// manager's `mMultiFilter` follows, with the trick swap mode `isDynamicTrickSelection`
    /// gives (`RandomSwap`, else `DoNotSwap`).
    StartPath { player: i32, path_id: AiPathId, branch: bool, challenge_filter: bool, dynamic_trick_selection: bool },
    /// `AI.ClearBehavior`: `AIController::ClearBehavior`, then `ExternalReset`.
    ClearBehavior { player: i32 },
    /// `AI.StartLapPaths`: `AIController::SetChallengeRound`.
    SetChallengeRound { player: i32, round: i32 },
    /// `AI.TriggerPositiveGesture`/`AI.TriggerNegativeGesture`: `AIController::TriggerGesture`.
    Gesture { player: i32, positive: bool },
    /// `AI.SwitchOffFollowMeRubberBanding` of a challenge player.
    SwitchOffFollowMeRubberBanding { player: i32 },
    /// `AISkaterManager::ExitAiBasedChallenge(false)`: the challenge skaters become ambient
    /// ones or go, the excess models unload, and the manager leaves the challenge.
    ExitAiBasedChallenge,
    /// `LoadBinaryPathPackage`'s arena load of the static path package.
    LoadPathPackage { file: String },
    /// `AI.FollowTrickPattern`: `SetBehavior<FollowPathBehavior>`, then with a matching path the
    /// pattern (`SetTrickPattern`), `DoNotBranch`, `FollowTrickPattern` and its ID, otherwise
    /// the manager's `mAIPathID` (`filter`).
    FollowTrickPattern { player: i32, pattern: super::skate::TrickPattern, path_id: Option<AiPathId>, filter: AiPathId },
}

/// `ChallengeAIManager`'s state.
pub(crate) struct ChallengeAiManager {
    /// `ReleaseAICharacterState`: 0 once a main challenge completes, 1 after the next
    /// prerequisite pass, then 2, 3 while the released skaters settle, 4 when done.
    release_state: u32,
    /// `WaitingForUnspawnNumFramesRemaining`.
    frames_remaining: u32,
    /// `DoResetReplayBufferOnComplete`: the completed main challenge was an AI one.
    reset_replay_buffer: bool,
    /// `mAIPathID`, which `LoadPaths` sets.
    pub(crate) path_id: AiPathId,
    /// `mMultiFilter`, which a started path follows by reference while the filter is active.
    pub(crate) multi_filter: SharedFilter,
    /// `mRoundPathFilter.intAllowDynamic`.
    allow_dynamic: i32,
    /// `mIgnoreFilter`.
    ignore_filter: bool,
    /// `mRespawnLocation`'s location name and `mRespawnLocationChallenge`.
    pub(crate) respawn_location: Option<String>,
    respawn_challenge: Option<usize>,
    /// `mSelectedCharactersChallenge`.
    selected_challenge: Option<usize>,
    /// `mSelectedCharacters`: each player's `ai_characters` key, 0 for none.
    selected: [u64; 8],
}

impl Default for ChallengeAiManager {
    /// `ChallengeAIManager::ChallengeAIManager` 0x825bae10 and `Init` 0x825c1208.
    fn default() -> Self {
        Self {
            release_state: RELEASE_COMPLETE,
            frames_remaining: 0,
            reset_replay_buffer: false,
            path_id: AiPathId([0; 16]),
            multi_filter: SharedFilter::new(PathFilter::Multiple(Vec::new())),
            allow_dynamic: 0,
            ignore_filter: true,
            respawn_location: None,
            respawn_challenge: None,
            selected_challenge: None,
            selected: [0; 8],
        }
    }
}

fn script_error(world: &mut World, context: &str, error: impl std::fmt::Display) {
    world.output(Output::ScriptError { context: context.to_owned(), error: error.to_string() });
}

fn challenge_attr(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    world.db.attribute(class::CHALLENGES, world.activities.activities[activity].key, key)
}

fn challenge_bool(world: &World, activity: usize, key: u64) -> bool {
    challenge_attr(world, activity, key).and_then(|a| a.bool(0)).unwrap_or(false)
}

fn local_attr(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    let (class, collection) = world.activities.activities[activity].local_data?;
    world.db.attribute(class, collection, key)
}

fn uses_dynamic_competitors(world: &World, activity: usize) -> bool {
    key::DYNAMIC_COMPETITORS.iter().any(|&k| challenge_bool(world, activity, k))
}

fn party_play(world: &World) -> bool {
    world.game_mode == GM_PARTY_PLAY
}

fn index_of(player: i32) -> Option<usize> {
    usize::try_from(player).ok().filter(|&index| index < 8)
}

/// `cChallengeCompetitorsManager::GetNumCompetitors` (`0x825af3d0`): with a dynamic flag the
/// dynamically chosen competitors with a profile, else `Num_Competitors` (`0x825ad2a0`), the
/// `Competitors` array's length.
pub(crate) fn num_competitors(world: &World, activity: usize) -> u32 {
    if uses_dynamic_competitors(world, activity) {
        return world.competitors.infos.iter().flatten().filter(|i|i.profile_name.is_some()).count() as u32;
    }
    challenge_attr(world, activity, key::COMPETITORS).map_or(0, |a| u32::try_from(a.len()).unwrap_or(u32::MAX))
}

/// `tCompetitorInfo`: the profile (an `ai_characters` name or group name) and
/// `intRootCategory` (0 a character, 1 a random member of a group).
use super::competitors::CompetitorInfo;

/// `cChallengeCompetitorsManager::GetCompetitorInfo` (`0x825b4838`): a team contest takes the
/// goofy team's list when the local player is goofy; a missing entry reads zeroed.
fn competitor_info(world: &World, activity: usize, index: usize) -> Option<CompetitorInfo> {
    if uses_dynamic_competitors(world, activity) {
        return world.competitors.infos.get(index).cloned().flatten();
    }
    let goofy = challenge_bool(world, activity, key::CONTEST_TEAMS) && {
        let local = world.local_player;
        !world.players.as_deref().map_or(world.stance_regular, |players| players.is_stance_regular(local))
    };
    let list = if goofy { key::GOOFY_COMPETITORS } else { key::COMPETITORS };
    let Some(attribute) = challenge_attr(world, activity, list) else {
        return Some(CompetitorInfo { profile_name: None, root_category: 0 });
    };
    let root_category = attribute
        .value_bytes(index)
        .filter(|bytes| bytes.len() >= 8)
        .map_or(0, |bytes| i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]));
    Some(CompetitorInfo { profile_name: attribute.text(index).map(str::to_owned), root_category })
}

/// `AttribSysUtils::GetLeafCollections(class, root)` (`0x82ab98c8`): the class's leaves in key
/// order that descend from the root (`IsDescendantOf`).
fn leaf_collections_under(world: &World, class: u64, root: u64) -> Vec<u64> {
    world
        .db
        .leaf_collections(class)
        .into_iter()
        .filter(|&leaf| {
            let mut collection = leaf;
            for _ in 0..64 {
                if collection == root {
                    return true;
                }
                match world.db.parent(class, collection) {
                    Some(parent) if parent != 0 && parent != collection => collection = parent,
                    _ => return false,
                }
            }
            false
        })
        .collect()
}

/// `GetCharacterID(allCharacters, name)` (`0x825b2338`): the last character whose layout name
/// is the profile's, 0 for none.
fn character_id(world: &World, all_characters: &[u64], profile_name: Option<&str>) -> u64 {
    let mut found = 0;
    for &character in all_characters {
        if world.db.layout_text(AI_CHARACTERS, character, LAYOUT_NAME) == profile_name {
            found = character;
        }
    }
    found
}

/// `CheckCharacterValidity` (`0x825adf10`): the character's flag for the challenge's type;
/// other types take every character.
fn check_character_validity(world: &World, activity: usize, class: u64, character: u64) -> bool {
    let flag = |k: u64| world.db.attribute(class, character, k).and_then(|a| a.bool(0)).unwrap_or(false);
    match challenge_type(world, activity) {
        challenge::BEST_TRICK => flag(key::BEST_TRICK_FLAG),
        challenge::DEATH_RACE => flag(key::DEATH_RACE_FLAG),
        challenge::DEMO => flag(key::DO_DEMO),
        challenge::JAM => flag(key::DO_JAM),
        challenge::SKATE => flag(key::DO_SKATE),
        challenge::STREET_SKATING => world.db.layout_byte(class, character, LAYOUT_STREET_SKATING) != 0,
        challenge::MEGA_RAMP => world.db.layout_byte(class, character, LAYOUT_MEGA_RAMP) != 0,
        _ => true,
    }
}

/// The `Characters` of the last group the predicate takes, each as the referenced
/// collection.
fn group_characters(world: &World, groups: &[u64], mut take: impl FnMut(u64) -> bool) -> Vec<(u64, u64)> {
    let mut group = None;
    for &key in groups {
        if take(key) {
            group = Some(key);
        }
    }
    let Some(attribute) = group.and_then(|group| world.db.attribute(AI_CHARACTER_GROUPS, group, key::CHARACTERS)) else {
        return Vec::new();
    };
    (0..attribute.len()).map(|index| attribute.ref_spec(index).unwrap_or((0, 0))).collect()
}

/// `GetCharactersFromGroup` (`0x825bf590`): the named group's valid characters no competitor
/// already has.
fn characters_from_group(world: &World, activity: usize, groups: &[u64], name: Option<&str>, taken: &[u64]) -> Vec<u64> {
    group_characters(world, groups, |group| world.db.layout_text(AI_CHARACTER_GROUPS, group, 0) == name)
        .into_iter()
        .filter(|&(class, character)| check_character_validity(world, activity, class, character))
        .map(|(_, character)| character)
        .filter(|&character| !taken.iter().any(|&other| other as u32 == character as u32))
        .collect()
}

/// `GetCharactersFromDefaultGroup` (`0x825bf828`): the default group's characters no
/// competitor already has.
fn characters_from_default_group(world: &World, groups: &[u64], taken: &[u64]) -> Vec<u64> {
    group_characters(world, groups, |group| world.db.layout_byte(AI_CHARACTER_GROUPS, group, GROUP_IS_DEFAULT) != 0)
        .into_iter()
        .map(|(_, character)| character)
        .filter(|&character| !taken.iter().any(|&other| other as u32 == character as u32))
        .collect()
}

/// `GetRandomCharacterID` (`0x825bfa48`): the group's characters (the default group's when it
/// has none) and a Simulation draw. The native counts the draw down before it tests each
/// entry, so a draw `n` takes entry `n - 1` and a draw that is a multiple of the count takes
/// no character.
fn random_character_id(world: &mut World, activity: usize, groups: &[u64], name: Option<&str>, taken: &[u64]) -> Result<u64, String> {
    let mut characters = characters_from_group(world, activity, groups, name, taken);
    if characters.is_empty() {
        characters = characters_from_default_group(world, groups, taken);
    }
    let count = u32::try_from(characters.len()).map_err(|e| e.to_string())?;
    if count == 0 {
        return Err("GetRandomCharacterID: no character to choose (the native traps)".to_owned());
    }
    let random = world.random.as_mut().ok_or("GetRandomCharacterID: the Simulation RNG is not lent")?;
    let draw = random.random_value() % count;
    let Some(index) = draw.checked_sub(1) else { return Ok(0) };
    let character = characters[index as usize];
    Ok(if world.db.has_collection(AI_CHARACTERS, character) { character } else { 0 })
}

/// `CharacterRecipeFromCharacterKey` (`0x825b27e0`): the character's `characters_marquee`
/// collection; none for no character.
fn character_recipe_from_key(world: &World, character: u64) -> Option<u64> {
    if character == 0 || character == u64::MAX {
        return None;
    }
    let (class, collection) = world.db.attribute(AI_CHARACTERS, character, key::RECIPE).and_then(|a| a.ref_spec(0)).unwrap_or((0, 0));
    Some(if world.db.has_collection(class, collection) { collection } else { 0 })
}

/// `GetCharacterRecipe(player)` (`0x825b28a0`).
pub(crate) fn character_recipe(world: &World, player: i32) -> Option<u64> {
    let character = index_of(player).map_or(0, |index| world.ai_manager.selected[index]);
    character_recipe_from_key(world, character)
}

/// `GetCharacterProfile(player)` (`0x825b2920`): `default` without a character, else the
/// profile's layout name.
fn character_profile(world: &World, player: i32) -> Option<String> {
    let character = index_of(player).map_or(0, |index| world.ai_manager.selected[index]);
    if character == 0 {
        return Some("default".to_owned());
    }
    let (class, collection) = world.db.attribute(AI_CHARACTERS, character, key::PROFILE).and_then(|a| a.ref_spec(0))?;
    world.db.layout_text(class, collection, 0).map(str::to_owned)
}

/// `GetChallengeRecipeVector` (`0x825bfba0`): each selected character's recipe, once.
fn challenge_recipe_vector(world: &World) -> Vec<u64> {
    let mut recipes = Vec::new();
    for &character in &world.ai_manager.selected {
        if character as u32 == 0 {
            continue;
        }
        if let Some(recipe) = character_recipe_from_key(world, character)
            && !recipes.contains(&recipe)
        {
            recipes.push(recipe);
        }
    }
    recipes
}

/// The commands a manager call left, through the backend into the pending messages.
fn post(link: &AiLink, manager: &mut AiSkaterManager, commands: Vec<AmbientCommand>) -> Result<(), String> {
    for command in commands {
        manager.post(command, &link.settings)?;
    }
    Ok(())
}

/// `ChallengeAIManager::SelectCharacters` (`0x825c1420`): unless the manager already requested
/// a challenge's assets, each competitor's character (named, or a random member of its
/// group), in competitor order unless a team contest; `random_shuffle` with the base
/// `IRandom`, whose `GetRandomValue` is the shared `return 0` stub, rotates the last pick to the
/// front. The picks fill players 1 on, the manager enters the AI-based challenge, and the
/// characters' recipes load unless `no_asset_load`.
pub(crate) fn select_characters(world: &mut World, activity: usize, no_asset_load: bool) -> Result<(), String> {
    let link = world.ai_link.clone().ok_or("SelectCharacters: no AI link")?;
    if link.manager().has_challenge_asset_request_occurred() {
        return Ok(());
    }
    world.ai_manager.selected = [0; 8];
    let all_characters = leaf_collections_under(world, AI_CHARACTERS, DEFAULT_COLLECTION);
    let groups = leaf_collections_under(world, AI_CHARACTER_GROUPS, DEFAULT_COLLECTION);
    let mut picks: Vec<u64> = Vec::new();
    for index in 0..num_competitors(world, activity) as usize {
        let info = competitor_info(world, activity, index).ok_or("SelectCharacters: dynamically chosen competitor")?;
        let pick = match info.root_category {
            0 => character_id(world, &all_characters, info.profile_name.as_deref()),
            1 => random_character_id(world, activity, &groups, info.profile_name.as_deref(), &picks)?,
            _ => continue,
        };
        picks.push(pick);
    }
    if !challenge_bool(world, activity, key::CONTEST_TEAMS) && picks.len() > 1 {
        picks.rotate_right(1);
    }
    for (slot, &pick) in world.ai_manager.selected[1..].iter_mut().zip(&picks) {
        *slot = pick;
    }
    let recipes = challenge_recipe_vector(world);
    let party = party_play(world);
    let mut manager = link.manager();
    let commands = manager.enter_ai_based_challenge(party);
    post(&link, &mut manager, commands)?;
    if !no_asset_load {
        manager.load_ai_challenge_assets(&recipes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `ChallengeAIManager::SelectDemoCharacters(false, noAssetLoad)` (`0x825c1740`): unless the
/// manager already requested a challenge's assets, the characters of the demo sponsor's
/// competitors (`cSignPostSponsorMan::GetDemoCompetitorInfo`) by profile name, in order from
/// player 1; then as `SelectCharacters`. Its one caller never asks for the sponsor challenge's
/// single competitor (`GetSponsorCompetitorInfo`).
pub(crate) fn select_demo_characters(world: &mut World, no_asset_load: bool) -> Result<(), String> {
    let link = world.ai_link.clone().ok_or("SelectDemoCharacters: no AI link")?;
    if link.manager().has_challenge_asset_request_occurred() {
        return Ok(());
    }
    world.ai_manager.selected = [0; 8];
    let all_characters = leaf_collections_under(world, AI_CHARACTERS, DEFAULT_COLLECTION);
    let count = usize::try_from(super::sponsors::demo_competitor_count(world)).unwrap_or(0);
    let picks: Vec<u64> = (0..count)
        .map(|index| {
            let name = super::sponsors::competitor_profile_name(world, super::sponsors::demo_competitor_info(world, index));
            character_id(world, &all_characters, name.as_deref())
        })
        .collect();
    for (slot, &pick) in world.ai_manager.selected[1..].iter_mut().zip(&picks) {
        *slot = pick;
    }
    let recipes = challenge_recipe_vector(world);
    let party = party_play(world);
    let mut manager = link.manager();
    let commands = manager.enter_ai_based_challenge(party);
    post(&link, &mut manager, commands)?;
    if !no_asset_load {
        manager.load_ai_challenge_assets(&recipes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// `AISkaterManager::IsActivityAnAiChallenge` (`0x824b9718`): offline, outside Party Play,
/// with competitors.
pub(crate) fn is_activity_an_ai_challenge(world: &World, activity: usize) -> bool {
    let online = world.ai_link.as_ref().is_some_and(|link| link.manager().is_in_an_online_game());
    !online && !party_play(world) && num_competitors(world, activity) != 0
}

/// `ChallengeAIManager::AreAllAssetsLoaded` (`0x825b2c00`).
pub(crate) fn are_all_assets_loaded(world: &World) -> bool {
    world.ai_link.as_ref().is_none_or(|link| link.manager().are_all_assets_loaded())
}

/// `ChallengeAIManager::AreStaticAIPathsLoaded` (`0x825b2c50`).
pub(crate) fn are_static_paths_loaded(world: &World) -> bool {
    world.ai_link.as_ref().is_none_or(|link| link.paths().are_static_paths_loaded())
}

/// `ChallengeAIManager::LoadPaths(activity, false, -1)` (`0x825b1f68`): the path ID from the
/// activity's local data (branch, stage, the part's first letter, index) and the type
/// `gAIPathChallengeTypeMap` gives its challenge type, or a demo's (main branch, stage 0, part
/// `a`, the demo index); `mAIPathID` takes it with any description and suffix, and
/// `LoadStaticChallengePaths` requests its package.
pub(crate) fn load_paths(world: &mut World, activity: usize) -> Result<(), String> {
    let link = world.ai_link.clone().ok_or("LoadPaths: no AI link")?;
    let byte = |k: u64| local_attr(world, activity, k).and_then(|a| a.value_bytes(0)).and_then(|b| b.first().copied()).unwrap_or(0);
    let mut branch = local_attr(world, activity, key::PATH_BRANCH).and_then(|a| a.u32(0)).unwrap_or(0) as u8;
    let mut stage = byte(key::PATH_STAGE);
    let mut part = local_attr(world, activity, key::PATH_PART).and_then(|a| a.text(0)).and_then(|t| t.bytes().next()).unwrap_or(0);
    let mut index = byte(key::PATH_INDEX);
    let mapped = usize::try_from(challenge_type(world, activity)).ok().and_then(|t| path_id::CHALLENGE_TYPE_MAP.get(t)).copied().unwrap_or(-1);
    let mut kind = u8::try_from(mapped).map_err(|_| format!("LoadPaths: challenge type {} has no path type", challenge_type(world, activity)))?;
    if challenge_bool(world, activity, key::DEMO_PATHS) {
        branch = path_id::BRANCH_MAIN;
        stage = 0;
        part = b'a';
        index = challenge_attr(world, activity, key::DEMO_PATHS_INDEX).and_then(|a| a.value_bytes(0)).and_then(|b| b.first().copied()).unwrap_or(0);
        kind = path_id::CHALLENGE_TYPE_DEMO;
    }
    let mut id = world.ai_manager.path_id;
    path_id::set_progression_branch(&mut id, branch);
    id.0[1] = stage;
    id.0[2] = part;
    id.0[3] = kind;
    id.0[4] = index;
    id.0[5] = ANY_DESCRIPTION[0];
    id.0[6] = 0;
    id.0[14..16].copy_from_slice(&ANY_SUFFIX.to_be_bytes());
    world.ai_manager.path_id = id;
    let package = path_id::challenge_id(branch, stage, part, kind, index, b"", 0);
    let file = link.paths().load_static_challenge_paths(&package).map_err(|e| e.to_string())?;
    world.output(Output::Ai(AiRequest::LoadPathPackage { file }));
    Ok(())
}

/// `ChallengeAIManager::Update` (`0x825b1d60`): after a main challenge completes and the next
/// prerequisite pass, the manager leaves the AI-based challenge, then waits six updates;
/// while it does a completed AI challenge's replay keeps its last two frames.
pub(crate) fn update(world: &mut World) {
    let manager = &mut world.ai_manager;
    match manager.release_state {
        1 => manager.release_state = 2,
        2 => {
            manager.release_state = 3;
            let reset = manager.reset_replay_buffer;
            manager.frames_remaining = RELEASE_WAIT_FRAMES;
            world.output(Output::Ai(AiRequest::ExitAiBasedChallenge));
            if reset {
                world.replay.erase_all_but_last_n_frames(RELEASE_REPLAY_FRAMES);
            }
        }
        3 => {
            if manager.frames_remaining == 0 {
                manager.release_state = RELEASE_COMPLETE;
            } else {
                manager.frames_remaining -= 1;
            }
            if manager.reset_replay_buffer {
                world.replay.erase_all_but_last_n_frames(RELEASE_REPLAY_FRAMES);
            }
        }
        _ => {}
    }
}

/// `GetRespawnMatrix(i)` (`0x825ae1d8`) for each of `GetNumRespawnLocations` (`0x825ae178`), at
/// least the first: the respawn location's spawn points (none without a respawn location).
pub(crate) fn respawn_matrices(world: &World) -> Vec<[V4; 4]> {
    let (Some(name), Some(link)) = (world.ai_manager.respawn_location.as_deref(), world.ai_link.as_ref()) else {
        return Vec::new();
    };
    let count = link.locations.num_spawn_locations(name).max(1);
    (0..count)
        .filter_map(|index| link.locations.spawn_location(name, i32::try_from(index).ok()?))
        .map(|transform| std::array::from_fn(|row| std::array::from_fn(|column| transform[row * 4 + column])))
        .collect()
}

/// `ChallengeAIManager::CanStartMainChallenge` (`0x825addf8`): the last challenge's skaters
/// are released.
pub(crate) fn can_start_main_challenge(world: &World) -> bool {
    world.ai_manager.release_state == RELEASE_COMPLETE
}

/// `ChallengeAIManager::ReevaluateActivityPrereqs` (`0x825bb010`): a completed main challenge
/// starts the release. The AI lock-out pass over `mLockedOutActivities` is not ported.
pub(crate) fn reevaluate_activity_prereqs(world: &mut World) {
    if world.ai_manager.release_state == 0 {
        world.ai_manager.release_state = 1;
    }
}

/// `ChallengeAIManager::GameLoad` (`0x825b60a0`): the selected characters clear.
pub(crate) fn game_load(world: &mut World) {
    world.ai_manager.selected = [0; 8];
}

/// `ChallengeAIManager::RunMainActivity` (`0x825addc0`).
pub(crate) fn run_main_activity(world: &mut World, activity: usize) {
    world.ai_manager.selected_challenge = Some(activity);
}

/// `ChallengeAIManager::ShutdownActivity` (`0x825addc8`).
pub(crate) fn shutdown_activity(world: &mut World, activity: usize) {
    let manager = &mut world.ai_manager;
    if manager.selected_challenge == Some(activity) {
        manager.selected_challenge = None;
    }
    if manager.respawn_challenge == Some(activity) {
        manager.respawn_location = None;
        manager.respawn_challenge = None;
    }
}

/// `ChallengeAIManager::CompleteChallenge` (`0x825b1e68`): a main challenge starts the release,
/// noting whether it was an AI challenge (the distance callbacks it resets are not ported);
/// every completion drops the round filter.
pub(crate) fn complete_challenge(world: &mut World, activity: usize, is_main: bool) {
    if is_main {
        world.ai_manager.release_state = 0;
        world.ai_manager.reset_replay_buffer = is_activity_an_ai_challenge(world, activity);
        if let Some(mut manager) = world.ai_link.as_ref().and_then(|link| link.manager.lock().ok()) {
            manager.reset_distance_callbacks();
        }
    }
    world.ai_manager.ignore_filter = true;
    world.ai_manager.multi_filter.set(PathFilter::Multiple(Vec::new()));
}

fn link_or_error(world: &mut World, context: &str) -> Option<AiLink> {
    let link = world.ai_link.clone();
    if link.is_none() {
        script_error(world, context, "no AI link");
    }
    link
}

fn location_name(world: &World, value: &Value) -> Option<String> {
    let at = attrib_location(&world.pointers, value)?;
    world.db.text_at(at).map(str::to_owned)
}

/// `AI.SpawnAIAtLocation(player, location)` (`0x8288c620`): the location's spawn point for the
/// player, `SpawnAISkater(AC_CHALLENGE, GetCharacterProfile(player))` and the sequence
/// connectors' setting for the player (`cSequenceConnectorManager::mCurrentConnectorSetting`).
fn spawn_ai_at_location(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let Some(link) = link_or_error(world, "AI.SpawnAIAtLocation") else { return Ok(Vec::new()) };
    let Some(name) = location_name(world, call.arg(1)) else {
        script_error(world, "AI.SpawnAIAtLocation", "no location");
        return Ok(Vec::new());
    };
    let Some(transform) = link.locations.spawn_location(&name, player) else {
        script_error(world, "AI.SpawnAIAtLocation", format!("unknown location {name}"));
        return Ok(Vec::new());
    };
    let Some(slot) = usize::try_from(player).ok().filter(|&slot| slot <= 8) else {
        script_error(world, "AI.SpawnAIAtLocation", format!("player {player}"));
        return Ok(Vec::new());
    };
    let rows: [V4; 4] = std::array::from_fn(|row| std::array::from_fn(|column| transform[row * 4 + column]));
    let profile = character_profile(world, player).unwrap_or_default();
    let recipe = character_recipe(world, player);
    let result = {
        let mut manager = link.manager();
        manager
            .spawn_challenge_skater(slot, recipe, rows, profile)
            .map_err(|e| e.to_string())
            .and_then(|command| command.map_or(Ok(false), |command| manager.post(command, &link.settings).map(|()| true)))
    };
    match result {
        Ok(true) => {
            let (short, long) = ai_player_names(world, player);
            if let Ok(index) = u32::try_from(player)
                && let Err(error) = link.settings.try_set_ai_names(index, short.as_bytes(), long.as_deref().map(str::as_bytes))
            {
                script_error(world, "AI.SpawnAIAtLocation", format!("{error:?}"));
            }
        }
        Ok(false) => {}
        Err(error) => script_error(world, "AI.SpawnAIAtLocation", error),
    }
    if let Some(players) = world.players.as_deref() {
        players.set_manuals_are_connectors(player, world.sequence_connector.setting);
    }
    Ok(Vec::new())
}

/// `PlayersBackEnd::UpdatePlayerName` (`0x82563a88`) for a challenge AI player, which
/// `CreatePlayer` runs: without a main challenge, or for a character whose short HAL is empty or
/// `###GENERIC###` (`ChallengeAIManager::CheckGenericPlayerName` `0x825b2518`), the short name is
/// `DefaultName` and the long one keeps its reset; otherwise they are the localized short and
/// long HAL names (`GetCharacterShortHAL` `0x825b2670`, `GetCharacterLongHAL` `0x825b2490`:
/// `ai_characters` layout words 6 and 7) without the format prefix.
fn ai_player_names(world: &World, player: i32) -> (String, Option<String>) {
    let character = selected_character(world, player);
    let short_hal = world.db.layout_text(AI_CHARACTERS, character, LAYOUT_SHORT_HAL).unwrap_or_default();
    if world.activities.main.is_none() || short_hal.is_empty() || short_hal == "###GENERIC###" {
        return ("DefaultName".to_owned(), None);
    }
    let long_hal = world.db.layout_text(AI_CHARACTERS, character, LAYOUT_LONG_HAL).unwrap_or_default();
    let name = |hal: &str| world.host.localized(hal).unwrap_or_default();
    let names = (name(short_hal), Some(name(long_hal)));
    if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
        eprintln!("career: AI player {player} character {character:#x} names {short_hal:?}/{long_hal:?} => {names:?}");
    }
    names
}

/// `AI.UnspawnAI(player)` (`0x8288d0d0`): `AISkaterManager::UnspawnAISkater`.
fn unspawn_ai(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let Some(link) = link_or_error(world, "AI.UnspawnAI") else { return Ok(Vec::new()) };
    if let Ok(slot) = usize::try_from(player)
        && slot <= 8
    {
        let mut manager = link.manager();
        let commands = manager.unspawn_ai_skater_now(slot);
        if let Err(error) = post(&link, &mut manager, commands) {
            drop(manager);
            script_error(world, "AI.UnspawnAI", error);
        }
    }
    Ok(Vec::new())
}

/// `AI.GetPathCount()` (`0x8288c810`): `PathManager::mPathCount`.
fn get_path_count(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let count = world.ai_link.as_ref().map_or(0, |link| link.paths().path_count());
    Ok(vec![int(count)])
}

/// `AI.LoadPaths`, `AI.LoadRaceHeatPaths` and `AI.LoadSKATEProPaths` (`0x8288c8b8`,
/// `0x8288c9d8`, `0x8288cb08`): with `kbStaticPathBinaryLoading` the asset manager loads the
/// paths and these load nothing, so the path count they return the growth of stays put.
fn load_paths_native(_: &mut Lua<World>, _: &mut World, _: &NativeCall) -> R {
    Ok(vec![int(0)])
}

/// `AI.UnloadPaths()` (`0x8288cc28`): `PathManager::UnloadStaticPaths`.
fn unload_paths(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let Some(link) = link_or_error(world, "AI.UnloadPaths") else { return Ok(Vec::new()) };
    let result = link.paths().unload_static_paths();
    if let Err(error) = result {
        script_error(world, "AI.UnloadPaths", error);
    }
    Ok(Vec::new())
}

/// `AI.FilterPaths(data, roundPaths)` (`0x8288ccd8`): `ChallengeAIManager::SetFilter`
/// (`0x825ade48`): a disabled round ignores the filter; otherwise the round's dynamic flag
/// and each named path of its first 50 join `mMultiFilter`.
fn filter_paths(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.ai_manager.ignore_filter = false;
    let Some(at) = attrib_location(&world.pointers, call.arg(1)) else {
        script_error(world, "AI.FilterPaths", "no tRoundPaths");
        return Ok(Vec::new());
    };
    let word = |offset: u32| world.db.bytes(at.offset_by(offset), 4).map_or(0, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    if word(0x320) == 1 {
        world.ai_manager.ignore_filter = true;
        return Ok(Vec::new());
    }
    world.ai_manager.allow_dynamic = word(0x324);
    let names: Vec<String> = (0..50).filter_map(|i| world.db.text_at(at.offset_by(4 * i))).filter(|name| !name.is_empty()).map(str::to_owned).collect();
    let mut filter = match world.ai_manager.multi_filter.get() {
        PathFilter::Multiple(list) => list,
        PathFilter::Single(id) => vec![id],
    };
    for name in names {
        match path_id::parse(&name) {
            Some(id) => filter.push(id),
            None => script_error(world, "AI.FilterPaths", format!("path ID {name}")),
        }
    }
    world.ai_manager.multi_filter.set(PathFilter::Multiple(filter));
    Ok(Vec::new())
}

/// `AI.FilterReset()` (`0x8288cdb8`): `ChallengeAIManager::IgnoreFilter` (`0x825ade30`).
fn filter_reset(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    world.ai_manager.ignore_filter = true;
    world.ai_manager.multi_filter.set(PathFilter::Multiple(Vec::new()));
    Ok(Vec::new())
}

/// `AI.StartPath(player, branch = true)` (`0x8288ce68`).
fn start_path(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let branch = get_bool_or(call, 1, true);
    let manager = &world.ai_manager;
    let request = AiRequest::StartPath {
        player,
        path_id: manager.path_id,
        branch,
        challenge_filter: !manager.ignore_filter,
        dynamic_trick_selection: manager.allow_dynamic > 0,
    };
    world.output(Output::Ai(request));
    Ok(Vec::new())
}

/// `Skate.StartSkatePath(player, branch = true, clearInvalidTrickPatterns = true)` (`0x8288b340`):
/// `AI.StartPath`'s follow-path setup; the invalid trick patterns it also clears
/// (`FollowPathBehavior::SetInvalidTrickPatterns(nullptr, 0)` `0x824bd7f0`) have no other setter.
fn start_skate_path(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    start_path(lua, world, call)
}

/// The challenge database's `scoring_trick` records as `TrickScoringSettings` reads them: the
/// index among the sorted leaf collections, `mTrickType` (layout byte 112) and
/// `mSubstituteTrickGroup` (byte 124).
struct DbTricks<'a> {
    db: &'a super::attrib_db::AttribDb,
    leaves: Vec<u64>,
}

impl crate::ai::trick_pattern::TrickRecords for DbTricks<'_> {
    fn trick_index(&self, name: &str) -> Option<i32> {
        let key = super::keys::hash64(&name.to_ascii_lowercase());
        self.leaves.iter().position(|&k| k == key).and_then(|index| i32::try_from(index).ok())
    }

    fn trick_type_and_group(&self, index: i32) -> Option<(i32, u32)> {
        let key = *self.leaves.get(usize::try_from(index).ok()?)?;
        let class = super::skate::TRICK_CLASS;
        Some((self.db.layout_word(class, key, 28).cast_signed(), self.db.layout_word(class, key, 31)))
    }
}

/// `AI.FollowTrickPattern(player, spinCount)` (`0x8288d508`): a skater (`FindByIndex`) follows a
/// path inside the AI manager's `mAIPathID` that matches the S.K.A.T.E. pattern
/// (`cTrickPatternManager::GetTrickPattern(0)`, with trick substitution) and the spin count
/// (`PathManager::GetAIPathIDMatchingTrickPattern` `0x824c7f38`), through the pattern; without
/// one it follows the manager's path. Whether one matched. Without the skater nothing is pushed,
/// and the call answers the top of its stack, the last argument.
fn follow_trick_pattern(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    let spin_count = get_int(call, 1);
    if skater(world, player).is_none() {
        return Ok(vec![call.args.last().cloned().unwrap_or(Value::Nil)]);
    }
    let pattern = world.scoring.lock().unwrap_or_else(PoisonError::into_inner).patterns.pattern(0);
    let filter = world.ai_manager.path_id;
    let path_id = match world.ai_link.as_ref() {
        Some(link) => {
            let tricks = DbTricks { db: &world.db, leaves: world.db.leaf_collections(super::skate::TRICK_CLASS) };
            crate::ai::trick_pattern::path_id_matching_trick_pattern(&link.paths(), &pattern, spin_count, Some(&filter), true, &tricks)
                .map_err(|e| lua.error(format!("AI.FollowTrickPattern: {e:?}")))?
        }
        None => None,
    };
    world.output(Output::Ai(AiRequest::FollowTrickPattern { player, pattern, path_id, filter }));
    Ok(vec![Value::Boolean(path_id.is_some())])
}

/// `AI.StartLapPaths(player, round)` (`0x8288d010`).
fn start_lap_paths(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let request = AiRequest::SetChallengeRound { player: get_int(call, 0), round: get_int(call, 1) };
    world.output(Output::Ai(request));
    Ok(Vec::new())
}

/// `AI.ClearBehavior(player)` (`0x8288d458`).
fn clear_behavior(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::Ai(AiRequest::ClearBehavior { player: get_int(call, 0) }));
    Ok(Vec::new())
}

/// `AI.SelectCharacters(data, noAssetLoad = false)` (`0x8288d728`).
fn select_characters_native(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = activity_of(lua, call.arg(0)) else { return Ok(Vec::new()) };
    let no_asset_load = get_bool_or(call, 1, false);
    if let Err(error) = select_characters(world, activity, no_asset_load) {
        script_error(world, "AI.SelectCharacters", error);
    }
    Ok(Vec::new())
}

fn selected_character(world: &World, index: i32) -> u64 {
    index_of(index).map_or(0, |index| world.ai_manager.selected[index])
}

/// `AI.GetCharacterName(player)` (`0x8288d800`): `GetCharacterName(player - 1)`, the
/// character's layout name text.
fn get_character_name(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let character = selected_character(world, get_int(call, 0) - 1);
    let name = world.db.layout_text(AI_CHARACTERS, character, LAYOUT_CHARACTER_NAME);
    Ok(vec![name.map_or(Value::Nil, Value::from)])
}

/// `AI.GetCharacterModel(player)` (`0x8288d8b8`): `GetCharacterModel(player - 1)`, the recipe's
/// layout name; empty without a character.
fn get_character_model(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let character = selected_character(world, get_int(call, 0) - 1);
    if character == 0 {
        return Ok(vec![Value::from("")]);
    }
    let (class, collection) = world.db.attribute(AI_CHARACTERS, character, key::RECIPE).and_then(|a| a.ref_spec(0)).unwrap_or((0, 0));
    let model = world.db.layout_text(class, collection, 0);
    Ok(vec![model.map_or(Value::Nil, Value::from)])
}

/// `AI.GetCharacterPercentageWillDoABDTrick(player)` (`0x8288dc20`).
fn get_character_percentage_will_do_abd_trick(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let character = selected_character(world, get_int(call, 0));
    let percentage = world.db.attribute(AI_CHARACTERS, character, key::PERCENTAGE_WILL_DO_ABD_TRICK).and_then(|a| a.i16(0)).unwrap_or(0);
    Ok(vec![int(i32::from(percentage))])
}

/// `AI.EnterAiBasedChallenge()` (`0x8288d970`).
fn enter_ai_based_challenge(_: &mut Lua<World>, world: &mut World, _: &NativeCall) -> R {
    let Some(link) = link_or_error(world, "AI.EnterAiBasedChallenge") else { return Ok(Vec::new()) };
    let party = party_play(world);
    let mut manager = link.manager();
    let commands = manager.enter_ai_based_challenge(party);
    if let Err(error) = post(&link, &mut manager, commands) {
        drop(manager);
        script_error(world, "AI.EnterAiBasedChallenge", error);
    }
    Ok(Vec::new())
}

/// `AI.GetIfSkaterIsRegularStance(player)` (`0x8288dd88`): `Actor::GetNaturalStance` is
/// regular, true without the skater.
fn get_if_skater_is_regular_stance(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let regular = skater(world, get_int(call, 0)).is_none_or(|frame| frame.is_regular_stance);
    Ok(vec![Value::Boolean(regular)])
}

/// `AI.SetAIRespawnBehaviour(enable, location)` (`0x828931c8`): the running activity's AI
/// respawn location (`SetRespawnLocation` `0x825ae270`), or none.
fn set_ai_respawn_behaviour(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let location = if get_bool(call, 0) { location_name(world, call.arg(1)) } else { None };
    let manager = &mut world.ai_manager;
    if location.is_some() {
        manager.respawn_challenge = world.closure;
    } else {
        manager.respawn_challenge = None;
    }
    manager.respawn_location = location;
    Ok(Vec::new())
}

/// `AI.TriggerPositiveGesture(player)` (`0x8288d9c8`).
fn trigger_positive_gesture(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::Ai(AiRequest::Gesture { player: get_int(call, 0), positive: true }));
    Ok(Vec::new())
}

/// `AI.TriggerNegativeGesture(player)` (`0x8288da70`).
fn trigger_negative_gesture(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    world.output(Output::Ai(AiRequest::Gesture { player: get_int(call, 0), positive: false }));
    Ok(Vec::new())
}

/// `AI.SwitchOffFollowMeRubberBanding(player)` (`0x8288d398`): a `PT_AI_Challenge` player's.
fn switch_off_follow_me_rubber_banding(_: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let player = get_int(call, 0);
    if world.players.as_deref().is_some_and(|players| players.player_type(player) == PT_AI_CHALLENGE) {
        world.output(Output::Ai(AiRequest::SwitchOffFollowMeRubberBanding { player }));
    }
    Ok(Vec::new())
}

/// `AI.GetNumCompetitors(data)` (`0x8288dfa0`).
fn get_num_competitors(lua: &mut Lua<World>, world: &mut World, call: &NativeCall) -> R {
    let Some(activity) = activity_of(lua, call.arg(0)) else { return Ok(vec![int(0)]) };
    Ok(vec![super::binding::int_u32(num_competitors(world, activity))])
}

pub(crate) fn natives() -> Vec<(&'static str, &'static str, NativeFn<World>)> {
    vec![
        ("AI", "SpawnAIAtLocation", spawn_ai_at_location),
        ("AI", "UnspawnAI", unspawn_ai),
        ("AI", "GetPathCount", get_path_count),
        ("AI", "LoadPaths", load_paths_native),
        ("AI", "LoadRaceHeatPaths", load_paths_native),
        ("AI", "LoadSKATEProPaths", load_paths_native),
        ("AI", "UnloadPaths", unload_paths),
        ("AI", "FilterPaths", filter_paths),
        ("AI", "FilterReset", filter_reset),
        ("AI", "StartPath", start_path),
        ("AI", "FollowTrickPattern", follow_trick_pattern),
        ("Skate", "StartSkatePath", start_skate_path),
        ("AI", "StartLapPaths", start_lap_paths),
        ("AI", "ClearBehavior", clear_behavior),
        ("AI", "SelectCharacters", select_characters_native),
        ("AI", "GetCharacterName", get_character_name),
        ("AI", "GetCharacterModel", get_character_model),
        ("AI", "GetCharacterPercentageWillDoABDTrick", get_character_percentage_will_do_abd_trick),
        ("AI", "EnterAiBasedChallenge", enter_ai_based_challenge),
        ("AI", "GetIfSkaterIsRegularStance", get_if_skater_is_regular_stance),
        ("AI", "SetAIRespawnBehaviour", set_ai_respawn_behaviour),
        ("AI", "GetNumCompetitors", get_num_competitors),
        ("AI", "TriggerPositiveGesture", trigger_positive_gesture),
        ("AI", "TriggerNegativeGesture", trigger_negative_gesture),
        ("AI", "SwitchOffFollowMeRubberBanding", switch_off_follow_me_rubber_banding),
    ]
}
