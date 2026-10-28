//! `AI::PathController` (0x290 bytes): the motion controller that rides a recorded path. Each
//! frame it tracks the skater's place on the path, looks ahead, and writes the speed, turn, trick,
//! gesture and off-board intents and the physics adjustment that keep the skater on it.
//!
//! The controller reads its Actor through [`AiActorView`] and returns its Actor calls as
//! [`AiActorCommand`]s in call order. `mPath` is a per-frame cache: `PreUpdate` resolves the path
//! ID and `GenerateActionGraphIntents` clears it at the end.
use super::{
    actor::{
        AiActorCommand, AiActorView, AiClientType, AiPhysicsInput, flags, physics_flags,
    },
    attributes::AiSkaterAttributes,
    intents::{self, OffBoardToggleStatus, ToggleOffboardState, name, off_board, on_board},
    obstacle_avoider::{AiOutAudio, AvoidDecision, AvoiderLocal, EntityObstacle, ObstacleAvoider, ObstacleSkater},
    path::{self, InterpolatedNode, NodeDescription, trick_type},
    path_id::PathFilter,
    path_manager::{self, LockedPathTest, PathManager, PathQueryType, PathRef, PathSkaterAbilities},
    profile::SkaterProfile,
    tricks::{self, TrickTable, category},
    vmx::{self, V4},
};
use crate::actor_graph_construction::ActorGraphIntents;
use crate::actor_graph_construction::ActorGraphIntentError;
use crate::input_cvar_runtime::{InputCvarError, InputCvarReferences};
use crate::physics_state_ids::{PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_SKITCHING};
use skate2_formats::ai_path::{AiPath, AiPathId, AiPathNode};
use std::sync::{Arc, Mutex};

/// `AI` cvars at their defaults (`startup.ini` sets none).
mod cvars {
    pub(super) const MAX_FRAMES_TRYING_TO_WIPE_OUT_FAILED_SO_RESPAWN: u32 = 360;
    pub(super) const NUM_FRAMES_STUCK_BEFORE_WIPEOUT: u32 = 300;
    pub(super) const NUM_FRAMES_STUCK_PRE_START_NODE_BEFORE_WIPEOUT: u32 = 150;
    pub(super) const NUM_FRAMES_STUCK_BEFORE_NAV_MESH: u32 = 420;
    pub(super) const NUMBER_OF_STUCK_FRAMES_BEFORE_TRYING_OFF_BOARD: u32 = 41;
    pub(super) const NUMBER_OF_STUCK_FRAMES_TO_STOP_TRYING_OFF_BOARD: u32 = 211;
    pub(super) const NUMBER_OF_STUCK_FRAMES_TO_TRY_AVOIDING_DMO: u32 = 240;
    pub(super) const NUMBER_OF_UN_STUCK_FRAMES_BEFORE_UNSTUCK: u32 = 25;
    pub(super) const MINIMUM_FRAME_ADVANCE_RATIO: f32 = 0.25;
    pub(super) const MINIMUM_FRAME_ADVANCE_FOR_NOT_STUCK_RATIO: f32 = 0.025;
    pub(super) const LOOK_AHEAD_SOON_FRAME_AMOUNT: f32 = 2.0;
    pub(super) const LOOK_AHEAD_FAR_FRAME_AMOUNT: f32 = 45.0;
    pub(super) const LOOK_AHEAD_SPEED_FRAME_AMOUNT: f32 = 30.0;
    pub(super) const LOOK_AHEAD_AVOIDANCE_FRAME_AMOUNT: f32 = 15.0;
    pub(super) const SIGNIFICANT_SPEEDUP_FRACTION: f32 = 0.94999999;
    pub(super) const SIGNIFICANT_SPEEDUP_FRACTION_TRICK_APPROACH: f32 = 0.30000001;
    pub(super) const SIGNIFICANT_SPEEDUP_FRACTION_OFF_BOARD_APPROACH: f32 = 0.30000001;
    pub(super) const SIGNIFICANT_SPEEDUP_FRACTION_STEEP_SCALAR: f32 = 0.30000001;
    pub(super) const SIGNIFICANT_BRAKING_FRACTION: f32 = 0.30000001;
    pub(super) const SIGNIFICANT_BRAKING_FRACTION_STEEP_SCALAR: f32 = 0.40000001;
    pub(super) const SIGNIFICANT_BRAKING_FRACTION_OFF_BOARD_APPROACH: f32 = 0.40000001;
    pub(super) const STEEP_UP_GRADIENT: f32 = 0.22;
    pub(super) const STEEP_DOWN_GRADIENT: f32 = -0.22;
    pub(super) const MIN_SPEED_FOR_APPROACHING_TRICK: f32 = 2.0;
    pub(super) const MAX_FRAMES_AWAY_FOR_APPROACHING_TRICK: i32 = 40;
    pub(super) const MAX_FRAMES_AWAY_FOR_APPROACHING_OFF_BOARD: i32 = 60;
    pub(super) const PATH_OFF_BOARD_MIN_MOVE_SPEED: f32 = 0.3001;
    pub(super) const COMFORTABLY_WITHIN_PATH_WIDTH_RATIO: f32 = 0.60000002;
    pub(super) const ALLOW_TRICK_SWAP_ON_ALL_AERIAL_TRICKS: bool = false;
    pub(super) const PREVENT_TRICK_SWAP_ON_ALL_AERIAL_TRICKS: bool = false;
    pub(super) const TELEPORT_BACK_TO_PATH_START: bool = false;
    pub(super) const ENABLE_PHYSICS_AI_ADJUST: bool = true;
    pub(super) const PREVENT_PHYSICS_AI_ADJUST_DURING_HIPPY_JUMP: bool = true;
    pub(super) const ENABLE_PHYSICS_AI_DECK_POSITION: bool = true;
    pub(super) const ENABLE_PHYSICS_AI_DECK_VELOCITY: bool = true;
    pub(super) const ENABLE_PHYSICS_AI_DECK_ORIENTATION: bool = true;
    pub(super) const ENABLE_ORIENTATION_WHEN_IN_PATH: bool = false;
    pub(super) const ENABLE_VELOCITY_WHEN_IN_PATH: bool = true;
    pub(super) const AMBIENT_AI_BAIL_DURATION: f32 = 5.0;
    pub(super) const SHORTEST_AI_BAIL_ALLOWED: f32 = 1.5;
    pub(super) const LONGEST_AI_BAIL_ALLOWED: f32 = 7.9000001;
    pub(super) const MIN_PATH_DISTANCE_TO_SWITCH_TO_NAVMESH_AT_END_OF_PATH: f32 = 6.0;
    pub(super) const MIN_PATH_DISTANCE_TO_SWITCH_TO_NAVMESH_WHEN_ON_PATH: f32 = 20.0;
    pub(super) const MAX_CACHED_TRANSFORM_DISTANCE: f32 = 3.0;
    pub(super) const ENABLE_NAV_MESH_CONTROLLER_SWAP: bool = false;
    pub(super) const RESPAWN_ON_PATH_VELOCITY_SCALAR_DEFAULT: f32 = 0.5;
    pub(super) const RESPAWN_ON_PATH_VELOCITY_SCALAR_RACE: f32 = 0.75;
    pub(super) const BAIL_ON_EVERY_LANDING: bool = false;
}

/// `PI`, `TWO_PI`, `EPSILON_0` and the other literals the controller compares against.
const PI: f32 = f32::from_bits(0x4049_0fdb);
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
const EPSILON_0: f32 = f32::from_bits(0x3400_0000);
const DEGREES_TO_RADIANS: f32 = 0.017453292;
const ONE_OVER_UPDATE_HZ: f32 = f32::from_bits(0x3c88_8889);
/// `PATH_USAGE_SCORE_SCALE`.
const PATH_USAGE_SCORE_SCALE: u32 = 1000;
/// `eChallengeType` values the controller tests.
pub(crate) mod challenge_type {
    pub(crate) const RACE_A: i32 = 19;
    pub(crate) const RACE_B: i32 = 20;
    pub(crate) const FOLLOW: i32 = 23;
    pub(crate) const NO_GESTURES: i32 = 4;
    /// `GetChallengeType` 0x824bf008 without a main challenge.
    pub(crate) const NONE: i32 = 52;
}

/// `PathFollowing::BranchMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum BranchMode {
    DoNotBranch,
    #[default]
    BranchRandom,
    AmbitionLevel,
}

/// `PathFollowing::TrickSwapMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TrickSwapMode {
    DoNotSwap,
    #[default]
    RandomSwap,
    FollowTrickPattern,
}

/// `AI::EGestureType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum GestureType {
    #[default]
    None,
    Positive,
    Negative,
}

/// An `IFilter` object others change in place: `FollowPathBehavior::mSingleFilter` or
/// `ChallengeAIManager::mMultiFilter`. Equal only to itself, as the native compares the
/// pointers.
#[derive(Clone, Debug)]
pub(crate) struct SharedFilter(Arc<Mutex<PathFilter>>);

impl SharedFilter {
    pub(crate) fn new(filter: PathFilter) -> Self {
        Self(Arc::new(Mutex::new(filter)))
    }

    pub(crate) fn set(&self, filter: PathFilter) {
        *self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = filter;
    }

    pub(crate) fn get(&self) -> PathFilter {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

impl PartialEq for SharedFilter {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The filter an influence carries; pointer identity is its variant and object.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum InfluenceFilter {
    #[default]
    None,
    /// `PathManager::mAmbientFilter`.
    Ambient,
    Shared(SharedFilter),
}

impl InfluenceFilter {
    fn resolve(&self, paths: &PathManager) -> Option<PathFilter> {
        match self {
            Self::None => None,
            Self::Ambient => Some(paths.ambient_filter().clone()),
            Self::Shared(filter) => Some(filter.get()),
        }
    }
}

/// `AI::PathFollowing::Influence` (0x44): the move's settings the controller follows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Influence {
    /// `mTrickPattern`: the S.K.A.T.E. pattern `TrickPatternTrickSwap` follows.
    pub(crate) trick_pattern: crate::challenge::skate::TrickPattern,
    pub(crate) filter: InfluenceFilter,
    pub(crate) skater_speed_adjustment: f32,
    pub(crate) ambition_level: f32,
    pub(crate) trick_swap_mode: TrickSwapMode,
    pub(crate) branch_mode: BranchMode,
}

impl Default for Influence {
    /// `Influence::Influence` 0x824b55c8.
    fn default() -> Self {
        Self {
            trick_pattern: crate::challenge::skate::TrickPattern::default(),
            filter: InfluenceFilter::None,
            skater_speed_adjustment: 0.0,
            ambition_level: 0.5,
            trick_swap_mode: TrickSwapMode::RandomSwap,
            branch_mode: BranchMode::BranchRandom,
        }
    }
}

/// Another skater's path following, for `Path::CalculateCurrentUsage` 0x824b8008.
#[derive(Clone, Debug)]
pub(crate) struct SkaterPathUsage {
    pub(crate) player_id: i32,
    /// The path and current node of an Actor whose active motion controller is its path
    /// controller.
    pub(crate) path: Option<(AiPathId, i32)>,
}

/// What the controller reads beyond its Actor.
pub(crate) struct PathControllerContext<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) paths: &'a PathManager,
    pub(crate) tricks: &'a TrickTable,
    pub(crate) attributes: &'a AiSkaterAttributes,
    pub(crate) profile: &'a SkaterProfile,
    pub(crate) abilities: PathSkaterAbilities,
    pub(crate) client_type: AiClientType,
    pub(crate) challenge_round: i32,
    /// `GetChallengeType`.
    pub(crate) challenge_type: i32,
    /// `FEBackEnd::Data.mFEState == eFE_STATE_IN_FE`.
    pub(crate) in_front_end: bool,
    /// Every skater, for path usage.
    pub(crate) skaters: &'a [SkaterPathUsage],
    /// Every skater, as the obstacle avoider reads it.
    pub(crate) obstacle_skaters: &'a [ObstacleSkater],
    /// The Living World obstacles near this skater (`living_world_obstacles`).
    pub(crate) obstacle_entities: &'a [EntityObstacle],
    /// The local human's path-relative position when one exists besides this Actor.
    pub(crate) local_player_position: Option<V4>,
    /// Whether that local player is wiping out.
    pub(crate) local_player_wiping_out: bool,
    /// `SkaterManager::GetLocal`'s player ID.
    pub(crate) local_player: Option<i32>,
    /// `cChallengeRaceManager::GetEstimatedTimeToFinish` of each racer.
    pub(crate) race_estimated_times: &'a [(i32, f32)],
    /// `ChallengeAIManager::GetRespawnMatrix` by index for its respawn location (empty without
    /// one).
    pub(crate) respawn_locations: &'a [[V4; 4]],
    /// `ChallengeAIManager::mRespawnLocation` is set (`AI.SetAIRespawnBehaviour(true, ...)`).
    pub(crate) respawn_location_set: bool,
    /// Every skater's player and its path-relative position (its pending teleport's position
    /// when one waits), for `GetClosestObstacleDistanceSquared`.
    pub(crate) skater_positions: &'a [(i32, V4)],
    pub(crate) locked: &'a mut LockedPathTest<'a>,
}

/// Live inputs to GetAppropriateRespawnTransform824ca430, including other
/// skaters' pending teleports when choosing among challenge spawn points.
pub(crate) struct RespawnContext<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) paths: &'a PathManager,
    pub(crate) client_type: AiClientType,
    pub(crate) challenge_type: i32,
    pub(crate) respawn_location_set: bool,
    pub(crate) respawn_locations: &'a [[V4; 4]],
    pub(crate) skater_positions: &'a [(i32, V4)],
    pub(crate) player: i32,
}

#[derive(Debug)]
pub(crate) enum PathControllerError {
    Cvar(InputCvarError),
    Intents(ActorGraphIntentError),
    Paths(path_manager::PathManagerError),
    Random(crate::native_skater_random::rw_math::RwRandomLockError),
    Attributes(crate::attribute_runtime::AttributeRuntimeError),
    /// A path the controller needs is gone mid-frame.
    MissingPath,
    /// Challenge-only behavior (respawn locators, rubber banding, trick patterns, the
    /// navmesh swap) reached by a skater it does not apply to.
    ChallengeOnly(&'static str),
}

impl std::fmt::Display for PathControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(error) => write!(f, "AI cvar: {error:?}"),
            Self::Intents(error) => write!(f, "AI intents: {error:?}"),
            Self::Paths(error) => write!(f, "AI paths: {error}"),
            Self::Random(error) => write!(f, "AI random: {error:?}"),
            Self::Attributes(error) => write!(f, "AI attributes: {error:?}"),
            Self::MissingPath => f.write_str("AI path missing"),
            Self::ChallengeOnly(what) => write!(f, "AI challenge-only path reached: {what}"),
        }
    }
}

impl std::error::Error for PathControllerError {}

impl From<InputCvarError> for PathControllerError {
    fn from(error: InputCvarError) -> Self { Self::Cvar(error) }
}

impl From<ActorGraphIntentError> for PathControllerError {
    fn from(error: ActorGraphIntentError) -> Self {
        Self::Intents(error)
    }
}

impl From<path_manager::PathManagerError> for PathControllerError {
    fn from(error: path_manager::PathManagerError) -> Self {
        Self::Paths(error)
    }
}

impl From<crate::attribute_runtime::AttributeRuntimeError> for PathControllerError {
    fn from(error: crate::attribute_runtime::AttributeRuntimeError) -> Self {
        Self::Attributes(error)
    }
}

type Result<T> = std::result::Result<T, PathControllerError>;

fn rw_random() -> Result<u32> {
    crate::native_skater_random::rw_math::global_next().map_err(PathControllerError::Random)
}

/// `AI::GetClosestObstacleDistanceSquared` 0x824becb8: the squared distance from the point to
/// the nearest other skater.
fn closest_obstacle_distance_squared(positions: &[(i32, V4)], this_player: i32, point: &[V4; 4]) -> f32 {
    let mut closest = f32::MAX;
    for &(player, position) in positions {
        if player == this_player {
            continue;
        }
        let offset = vmx::sub(point[3], position);
        let squared = vmx::dot3(offset, offset);
        if squared < closest {
            closest = squared;
        }
    }
    closest
}

/// `AI::CalculateGradient` 0x824be858: rise over the guarded horizontal run, 0 within 1 mm.
pub(crate) fn calculate_gradient(future: V4, current: V4) -> f32 {
    let dx = future[0] - current[0];
    let dz = future[2] - current[2];
    let dy = future[1] - current[1];
    let horizontal = vmx::sqrt_guarded(dz.mul_add(dz, dx * dx));
    if horizontal <= 0.001 { 0.0 } else { dy / horizontal }
}

/// `AI::RemoveComponentFromVector` 0x824b50c8: `v - c * (dot(v, c) / dot(c, c))`.
fn remove_component(vector: V4, component: V4) -> V4 {
    let along = vmx::dot3(vector, component) / vmx::dot3(component, component);
    vmx::sub(vector, vmx::scale(component, along))
}

/// `AI::DoTrickTypesRoughlyMatch` 0x824b4aa0: equal, or both aerial.
fn trick_types_roughly_match(a: i32, b: i32) -> bool {
    let aerial = |kind: i32| kind == trick_type::AIR || kind == trick_type::FLIP;
    a == b || (aerial(a) && aerial(b))
}

/// `AI::IsMultiFlipTrickSequence` 0x824b4be8.
fn is_multi_flip_trick_sequence(first: i32, second: i32) -> bool {
    let consumed = if tricks::is_valid_scorable(second) { tricks::consumed_scorable(second) } else { second };
    first == consumed
}

/// `AI::CalculateManualIntentValue` 0x824be378: forward for a nose manual, back for a tail
/// manual.
fn manual_intent_value(attributes: &AiSkaterAttributes, scorable: i32) -> f32 {
    match scorable {
        intents::scorable::NOSEMANUAL => attributes.manual_intent_value,
        intents::scorable::TAILMANUAL => -attributes.manual_intent_value,
        _ => 0.0,
    }
}

/// Whether the scorable's name starts with `N`/`n`: a nollie trick.
fn is_nollie_scorable(scorable: i32) -> bool {
    matches!(tricks::scorable_name(scorable).as_bytes().first(), Some(b'N' | b'n'))
}

/// `fsel(a - b, a, b)` clamps: `max(value, low)` then `min(.., high)`.
fn fsel_clamp(value: f32, low: f32, high: f32) -> f32 {
    let raised = if low - value >= 0.0 { low } else { value };
    if high - raised >= 0.0 { raised } else { high }
}

/// `AI::PathController`.
#[derive(Clone, Debug)]
pub(crate) struct PathController {
    pub(crate) is_active: bool,
    pub(crate) is_fading_out: bool,
    pub(crate) path_id: AiPathId,
    path: Option<PathRef>,
    filter: InfluenceFilter,
    trick_pattern_next_trick_index: i32,
    pub(crate) approximated_node: InterpolatedNode,
    predicted_node: InterpolatedNode,
    look_ahead_node_soon: AiPathNode,
    look_ahead_node_far: InterpolatedNode,
    look_ahead_node_speed: AiPathNode,
    landing_node: AiPathNode,
    object_blocking_path_info: NodeDescription,
    cached_deck_transform_for_physics: [V4; 4],
    position_last_frame: V4,
    velocity: V4,
    effective_actor_direction: V4,
    current_speed: f32,
    pub(crate) current_node_index: i32,
    last_frame_predicted_node_index: i32,
    deviation_from_path_in_radians: f32,
    distance_from_path: f32,
    current_trick_type: i32,
    manual_trick_value: f32,
    last_trick_index_started: i32,
    impending_trick_node_index: i32,
    impending_trick_node_frame_count: i32,
    impending_trick_desired_anticipation: f32,
    impending_trick_desired_anticipation_angle: f32,
    impending_trick_type: i32,
    impending_trick: i32,
    last_trick_scorable_id: i32,
    impending_off_board_index: i32,
    impending_off_board_frame_count: i32,
    board_adjust_angle: f32,
    board_adjust_magnitude: f32,
    path_width_ratio: f32,
    path_width_ratio_conservative: f32,
    rubber_band_speed_scalar: f32,
    most_recent_air_spin_180_count: i32,
    respawn_node_index: i32,
    max_wipeout_duration_in_seconds: f32,
    stuck_reset_count: u32,
    num_frames_stuck: u32,
    num_frames_stuck_before_start: u32,
    num_frames_not_stuck: u32,
    num_frames_left_until_settled: u32,
    num_frames_in_wipeout: u32,
    num_frames_since_on_ground: u32,
    num_frames_since_airborne: u32,
    num_frames_since_trick_node: u32,
    gesture_type: GestureType,
    has_joined_path: bool,
    has_ever_been_joined_to_this_path: bool,
    pub(crate) local_world_is_loaded: bool,
    impending_trick_node: bool,
    impending_trick_will_require_launching: bool,
    impending_trick_started: bool,
    start_impending_trick: bool,
    impending_off_board_node: bool,
    is_wiping_out: bool,
    is_off_board: bool,
    push_intent_used_this_frame: bool,
    player_is_flipped: bool,
    is_airborne: bool,
    is_scoring_in_air_state: bool,
    has_landing_node_been_found: bool,
    is_board_adjust_required: bool,
    is_within_path_width: bool,
    is_steering_to_avoid: bool,
    has_been_externally_reset: bool,
    requires_hold_pattern: bool,
    respawn_node_found: bool,
    is_physics_fixup_allowed_yet: bool,
    external_wipeout_request: bool,
    enable_follow_me_rubber_banding: bool,
    pub(crate) has_cached_deck_transform_for_physics: bool,
    is_swap_required: bool,
    stuck_so_try_off_board: bool,
    obstruction_so_try_running_forward: bool,
    obstruction_so_try_jumping_forward: bool,
    is_trying_to_skitch: bool,
    has_bailed_due_to_blocking_object: bool,
    pub(crate) toggle_offboard_state: ToggleOffboardState,
    pub(crate) obstacle_avoider: ObstacleAvoider,
}

/// One frame of the controller: its context, Actor and outputs.
struct Frame<'c, 'a, 'i> {
    context: &'c mut PathControllerContext<'a>,
    actor: &'c mut AiActorView,
    intents: &'i mut ActorGraphIntents,
    commands: &'c mut Vec<AiActorCommand>,
    influence: &'c Influence,
}

impl PathController {
    /// `PathController::PathController` 0x824cc3f8, ending in `FullReset`.
    pub(crate) fn new() -> Self {
        let interpolated = InterpolatedNode { node: path::constructed_node(), description: NodeDescription::default() };
        let mut controller = Self {
            is_active: false,
            is_fading_out: false,
            path_id: AiPathId([0, 0, 0, 0xff, 0, 0, 0, 0, 0, 0, 0, 0xff, 0, 0, 0, 0]),
            path: None,
            filter: InfluenceFilter::None,
            trick_pattern_next_trick_index: 0,
            approximated_node: interpolated,
            predicted_node: interpolated,
            look_ahead_node_soon: path::constructed_node(),
            look_ahead_node_far: interpolated,
            look_ahead_node_speed: path::constructed_node(),
            landing_node: path::constructed_node(),
            object_blocking_path_info: NodeDescription::default(),
            cached_deck_transform_for_physics: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0; 4],
            ],
            position_last_frame: [0.0; 4],
            velocity: [0.0; 4],
            effective_actor_direction: [0.0; 4],
            current_speed: 0.0,
            current_node_index: 0,
            last_frame_predicted_node_index: 0,
            deviation_from_path_in_radians: 0.0,
            distance_from_path: 0.0,
            current_trick_type: trick_type::NONE,
            manual_trick_value: 0.0,
            last_trick_index_started: -1,
            impending_trick_node_index: 0,
            impending_trick_node_frame_count: 0,
            impending_trick_desired_anticipation: 0.0,
            impending_trick_desired_anticipation_angle: 0.0,
            impending_trick_type: trick_type::NONE,
            impending_trick: -1,
            last_trick_scorable_id: -1,
            impending_off_board_index: -1,
            impending_off_board_frame_count: 0,
            board_adjust_angle: 0.0,
            board_adjust_magnitude: 0.0,
            path_width_ratio: 0.0,
            path_width_ratio_conservative: 0.0,
            rubber_band_speed_scalar: 0.0,
            most_recent_air_spin_180_count: 0,
            respawn_node_index: -1,
            max_wipeout_duration_in_seconds: 0.0,
            stuck_reset_count: 0,
            num_frames_stuck: 0,
            num_frames_stuck_before_start: 0,
            num_frames_not_stuck: 0,
            num_frames_left_until_settled: 0,
            num_frames_in_wipeout: 0,
            num_frames_since_on_ground: 0,
            num_frames_since_airborne: 0,
            num_frames_since_trick_node: 0,
            gesture_type: GestureType::None,
            has_joined_path: false,
            has_ever_been_joined_to_this_path: false,
            local_world_is_loaded: true,
            impending_trick_node: false,
            impending_trick_will_require_launching: false,
            impending_trick_started: false,
            start_impending_trick: false,
            impending_off_board_node: false,
            is_wiping_out: false,
            is_off_board: false,
            push_intent_used_this_frame: false,
            player_is_flipped: false,
            is_airborne: false,
            is_scoring_in_air_state: false,
            has_landing_node_been_found: false,
            is_board_adjust_required: false,
            is_within_path_width: false,
            is_steering_to_avoid: false,
            has_been_externally_reset: false,
            requires_hold_pattern: false,
            respawn_node_found: false,
            is_physics_fixup_allowed_yet: false,
            external_wipeout_request: false,
            enable_follow_me_rubber_banding: false,
            has_cached_deck_transform_for_physics: false,
            is_swap_required: false,
            stuck_so_try_off_board: false,
            obstruction_so_try_running_forward: false,
            obstruction_so_try_jumping_forward: false,
            is_trying_to_skitch: false,
            has_bailed_due_to_blocking_object: false,
            toggle_offboard_state: ToggleOffboardState::default(),
            obstacle_avoider: ObstacleAvoider::default(),
        };
        controller.full_reset();
        controller
    }

    fn path(&self) -> Result<&AiPath> {
        self.path.as_ref().map(PathRef::path).ok_or(PathControllerError::MissingPath)
    }

    /// The frame's path, held apart from the controller while it updates its own nodes.
    fn path_ref(&self) -> Result<PathRef> {
        self.path.clone().ok_or(PathControllerError::MissingPath)
    }

    /// `mVelocity`, which `AIController::GetActorVelocity` 0x824b3c88 reports while this is the
    /// active motion controller.
    pub(crate) fn velocity(&self) -> V4 {
        self.velocity
    }

    /// `ExternalWipeoutRequest` 0x824b4880.
    pub(crate) fn external_wipeout_request(&mut self) {
        if !self.is_wiping_out {
            self.external_wipeout_request = true;
        }
    }

    /// `TriggerGesture` 0x824bfd68: a positive gesture only on the profile's celebrate odds
    /// unless guaranteed.
    pub(crate) fn trigger_gesture(&mut self, profile: &SkaterProfile, gesture: GestureType, guaranteed: bool) {
        if guaranteed || gesture != GestureType::Positive || profile.random_should_celebrate() {
            self.gesture_type = gesture;
        }
    }

    /// `AIController::SwitchOffFollowMeRubberBanding` 0x824bdb20.
    pub(crate) fn switch_off_follow_me_rubber_banding(&mut self) {
        self.enable_follow_me_rubber_banding = false;
    }

    /// `MinimalReset` 0x824c8db8.
    fn minimal_reset(&mut self) {
        self.current_node_index = 0;
        self.predicted_node.description = NodeDescription::default();
        path::reset(&mut self.predicted_node.node);
        self.approximated_node.description = NodeDescription::default();
        path::reset(&mut self.approximated_node.node);
        self.toggle_offboard_state = ToggleOffboardState::default();
        self.deviation_from_path_in_radians = 0.0;
        self.distance_from_path = 0.0;
        self.last_frame_predicted_node_index = 0;
        self.manual_trick_value = 0.0;
        self.current_trick_type = trick_type::NONE;
        self.board_adjust_angle = 0.0;
        self.has_joined_path = false;
        self.board_adjust_magnitude = 0.0;
        self.has_ever_been_joined_to_this_path = false;
        self.last_trick_index_started = -1;
        self.impending_trick_node = false;
        self.impending_trick_started = false;
        self.start_impending_trick = false;
        self.impending_off_board_node = false;
        self.push_intent_used_this_frame = false;
        self.player_is_flipped = false;
        self.is_airborne = false;
        self.is_scoring_in_air_state = false;
        self.most_recent_air_spin_180_count = 0;
        self.num_frames_stuck = 0;
        self.num_frames_stuck_before_start = 0;
        self.stuck_reset_count = 0;
        self.num_frames_not_stuck = 0;
        self.num_frames_since_trick_node = 0;
        self.last_trick_scorable_id = -1;
        self.is_board_adjust_required = false;
        self.has_landing_node_been_found = false;
        self.stuck_so_try_off_board = false;
        self.requires_hold_pattern = false;
        self.num_frames_since_on_ground = 7200;
        self.num_frames_since_airborne = 7200;
        self.object_blocking_path_info = NodeDescription::default();
        self.has_bailed_due_to_blocking_object = false;
    }

    /// `FullReset` 0x824c8ed0.
    fn full_reset(&mut self) {
        self.approximated_node.description = NodeDescription::default();
        path::reset(&mut self.approximated_node.node);
        self.predicted_node.description = NodeDescription::default();
        path::reset(&mut self.predicted_node.node);
        path::reset(&mut self.look_ahead_node_soon);
        self.look_ahead_node_far.description = NodeDescription::default();
        path::reset(&mut self.look_ahead_node_far.node);
        path::reset(&mut self.look_ahead_node_speed);
        path::reset(&mut self.landing_node);
        self.position_last_frame = [0.0; 4];
        self.velocity = [0.0; 4];
        self.effective_actor_direction = [0.0; 4];
        self.num_frames_left_until_settled = 30;
        self.num_frames_in_wipeout = 0;
        self.current_speed = 0.0;
        self.is_within_path_width = false;
        self.path_width_ratio = 0.0;
        self.is_steering_to_avoid = false;
        self.path_width_ratio_conservative = 0.0;
        self.gesture_type = GestureType::None;
        self.rubber_band_speed_scalar = 0.0;
        self.trick_pattern_next_trick_index = 0;
        self.max_wipeout_duration_in_seconds = 2.0;
        self.respawn_node_index = -1;
        self.respawn_node_found = false;
        self.is_physics_fixup_allowed_yet = false;
        self.external_wipeout_request = false;
        self.enable_follow_me_rubber_banding = true;
        self.is_swap_required = false;
        self.obstruction_so_try_running_forward = false;
        self.obstruction_so_try_jumping_forward = false;
        self.is_trying_to_skitch = false;
        self.minimal_reset();
    }

    /// `ExternalReset` 0x824c8fe8.
    pub(crate) fn external_reset(&mut self) {
        self.full_reset();
        self.has_been_externally_reset = true;
    }

    /// `ChangePath` 0x824c9020.
    fn change_path(&mut self, paths: &PathManager, id: AiPathId, index: i32) {
        self.path_id = id;
        self.minimal_reset();
        self.current_node_index = index;
        self.path = paths.get_ai_path(&self.path_id);
    }

    /// `PathController::GenerateActionGraphIntents` 0x824d3e00.
    pub(crate) fn generate_action_graph_intents(
        &mut self,
        context: &mut PathControllerContext<'_>,
        influence: &Influence,
        actor: &mut AiActorView,
        intents: &mut ActorGraphIntents,
        audio: &mut AiOutAudio,
        commands: &mut Vec<AiActorCommand>,
    ) -> Result<()> {
        if context.client_type == AiClientType::Ambient && actor.flags & flags::PINNED != 0 {
            actor.flags &= !flags::PINNED;
            commands.push(AiActorCommand::ClearPin);
        }
        let mut frame = Frame { context, actor, intents, commands, influence };
        self.pre_update(&mut frame)?;
        self.handle_wipeouts(&mut frame)?;
        if !self.is_wiping_out && self.num_frames_left_until_settled == 0 {
            self.update(&mut frame, audio)?;
        } else {
            let mut input = AiPhysicsInput::cleared(self.local_world_is_loaded);
            input.desired_deck_transform = self.cached_deck_transform_for_physics(frame.actor);
            if !self.local_world_is_loaded {
                input.flags |= physics_flags::DO_AI_ADJUST
                    | physics_flags::DO_DECK_POSITION
                    | physics_flags::DO_DECK_ORIENTATION
                    | physics_flags::DO_DECK_VELOCITY;
            }
            frame.commands.push(AiActorCommand::ApplyPhysicsInput(Box::new(input)));
            if !self.is_wiping_out
                && self.path.is_some()
                && self.num_frames_left_until_settled == 1
                && self.should_respawn_on_path(frame.context)?
            {
                let node = *path::node(self.path()?, self.current_node_index);
                let race = matches!(frame.context.challenge_type, challenge_type::RACE_A | challenge_type::RACE_B);
                let scalar = if race {
                    cvars::RESPAWN_ON_PATH_VELOCITY_SCALAR_RACE
                } else {
                    cvars::RESPAWN_ON_PATH_VELOCITY_SCALAR_DEFAULT
                };
                frame.commands.push(AiActorCommand::SetVelocity(vmx::scale(path::velocity(&node), scalar)));
            }
        }
        self.generate_gesture_intents(&mut frame, audio)?;
        self.position_last_frame = frame.actor.path_relative_position();
        self.path = None;
        Ok(())
    }

    /// `PreUpdate` 0x824cd870.
    fn pre_update(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        if frame.actor.flags & flags::EXTERNAL_RESET != 0 {
            self.external_reset();
        }
        let filter_changed = frame.influence.filter != self.filter || self.has_been_externally_reset;
        self.filter = frame.influence.filter.clone();
        if filter_changed {
            self.find_new_path(frame, frame.actor.path_relative_position())?;
        }
        self.path = frame.context.paths.get_ai_path(&self.path_id);
        if self.has_been_externally_reset && frame.actor.flags & flags::EXTERNAL_RESET == 0 {
            let finished = frame.actor.is_teleport_finished;
            self.push_intent_used_this_frame = false;
            if !finished {
                self.has_been_externally_reset = false;
            }
        } else {
            self.push_intent_used_this_frame = false;
        }
        Ok(())
    }

    /// `HandleWipeouts` 0x824cd780.
    fn handle_wipeouts(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let was = self.is_wiping_out;
        let is = frame.actor.is_wiping_out;
        self.is_wiping_out = is;
        if is && !was {
            self.wipeout_begin(frame)?;
        } else if was && !is {
            frame.commands.push(AiActorCommand::SetFadingOut(false));
            self.is_fading_out = false;
            self.object_blocking_path_info = NodeDescription::default();
            self.has_bailed_due_to_blocking_object = false;
        }
        if self.is_wiping_out {
            self.update_wiping_out(frame)
        } else {
            self.update_settling(frame);
            Ok(())
        }
    }

    /// `WipeoutBegin` 0x824c76e8.
    fn wipeout_begin(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let client = frame.context.client_type;
        self.num_frames_in_wipeout = 0;
        self.max_wipeout_duration_in_seconds = self.max_wipeout_duration(frame.context);
        self.external_wipeout_request = false;
        if client == AiClientType::Ambient {
            self.max_wipeout_duration_in_seconds = fsel_clamp(
                cvars::AMBIENT_AI_BAIL_DURATION,
                cvars::SHORTEST_AI_BAIL_ALLOWED,
                cvars::LONGEST_AI_BAIL_ALLOWED,
            );
        }
        if self.path.is_some() && self.should_respawn_on_path(frame.context)? {
            let respawn = self.find_respawn_node(frame.context)?;
            self.respawn_node_index = respawn;
            self.respawn_node_found = true;
            if client != AiClientType::Ambient {
                let frames = path::node_frame_difference_unordered(
                    self.path()?,
                    self.approximated_node.description.index,
                    self.approximated_node.description.ratio,
                    respawn,
                    0.0,
                );
                let duration = frames * ONE_OVER_UPDATE_HZ + self.max_wipeout_duration_in_seconds;
                self.max_wipeout_duration_in_seconds =
                    fsel_clamp(duration, cvars::SHORTEST_AI_BAIL_ALLOWED, cvars::LONGEST_AI_BAIL_ALLOWED);
            }
        } else {
            self.respawn_node_found = false;
            self.respawn_node_index = -1;
        }
        Ok(())
    }

    /// `GetMaxWipeoutDurationInSeconds` 0x824bf220.
    fn max_wipeout_duration(&self, context: &PathControllerContext<'_>) -> f32 {
        match context.challenge_type {
            challenge_type::RACE_A | challenge_type::RACE_B => context.attributes.max_wipeout_duration_race,
            _ => context.attributes.max_wipeout_duration,
        }
    }

    /// `UpdateWipingOut` 0x824cd518.
    fn update_wiping_out(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        self.last_trick_scorable_id = -1;
        self.num_frames_in_wipeout += 1;
        let seconds = self.num_frames_in_wipeout as f32 * f32::from_bits(0x3c88_8889);
        let pending = frame.actor.wipeout_teleport_pending;
        let until_teleport = frame.actor.time_until_wipeout_teleport;
        let teleport_now = pending && until_teleport < 0.1;
        if seconds > self.max_wipeout_duration_in_seconds || teleport_now {
            if self.should_respawn_on_path(frame.context)? {
                if self.path.is_some() {
                    if !self.respawn_node_found {
                        self.respawn_node_index = self.find_respawn_node(frame.context)?;
                        self.respawn_node_found = true;
                    }
                    self.teleport_to_node(frame, self.respawn_node_index)?;
                    self.num_frames_left_until_settled = 8;
                } else {
                    self.full_reset();
                    self.num_frames_left_until_settled = 0;
                }
            } else {
                self.teleport_to_spawn_locator(frame)?;
            }
        } else {
            let fade = frame.actor.fade_out_time;
            let fade_teleport = pending && until_teleport < fade;
            let late = fade_teleport || seconds > self.max_wipeout_duration_in_seconds - fade;
            if late && !(seconds >= self.max_wipeout_duration_in_seconds) && !self.is_fading_out {
                frame.commands.push(AiActorCommand::SetFadingOut(true));
                frame.commands.push(AiActorCommand::DisableNonWorldCollisionsForFadeout);
                self.is_fading_out = true;
            }
        }
        Ok(())
    }

    /// `TeleportToSpawnLocator` 0x824cd4a0: to the challenge's respawn point, then a full reset
    /// and a new path from there.
    fn teleport_to_spawn_locator(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let matrix = self.spawn_location(frame)?;
        frame.commands.push(AiActorCommand::Teleport(matrix));
        self.full_reset();
        self.find_new_path(frame, matrix[3])
    }

    /// `GetSpawnLocation` 0x824bee38: the only respawn point, else the one from point 1 on whose
    /// nearest other skater is furthest (the native never weighs point 0 when there are several).
    /// Only reached with a respawn location (`ShouldRespawnOnPath`); one without spawn points
    /// leaves `GetRespawnMatrix` (0x825ae1d8) to its location manager's lookup.
    fn spawn_location(&self, frame: &Frame<'_, '_, '_>) -> Result<[V4; 4]> {
        Self::select_spawn_location(frame.context.respawn_locations, frame.context.skater_positions, frame.actor.player_id)
    }

    fn select_spawn_location(points: &[[V4; 4]], positions: &[(i32, V4)], player: i32) -> Result<[V4; 4]> {
        match points {
            [] => Err(PathControllerError::ChallengeOnly("TeleportToSpawnLocator without a respawn point")),
            [only] => Ok(*only),
            [_, first, rest @ ..] => {
                let mut best = *first;
                let mut furthest = closest_obstacle_distance_squared(positions, player, first);
                for point in rest {
                    let distance = closest_obstacle_distance_squared(positions, player, point);
                    if distance > furthest {
                        best = *point;
                        furthest = distance;
                    }
                }
                Ok(best)
            }
        }
    }

    /// `UpdateSettling` 0x824b5528.
    fn update_settling(&mut self, frame: &mut Frame<'_, '_, '_>) {
        if self.num_frames_left_until_settled != 0 {
            if !frame.actor.skateboard_airborne || !self.local_world_is_loaded {
                self.num_frames_left_until_settled -= 1;
            }
            self.velocity = frame.actor.velocity;
        }
    }

    /// `ShouldRespawnOnPath` 0x824bf1c0: every skater but a challenge one outside the
    /// path-respawn challenges.
    fn should_respawn_on_path(&self, context: &PathControllerContext<'_>) -> Result<bool> {
        Self::respawns_on_path(context.client_type, context.respawn_location_set, context.cvars)
    }

    pub(crate) fn respawns_on_path(client_type: AiClientType, respawn_location_set: bool, cvars: &InputCvarReferences) -> Result<bool> {
        // IsInPathRespawnChallengeType 0x824bf108: no respawn location is set.
        Ok(client_type != AiClientType::Challenge || cvars.try_ai_always_respawn_on_path()? || !respawn_location_set)
    }

    /// `ShouldRespawnAhead` 0x824bf318.
    fn should_respawn_ahead(challenge: i32, cvars: &InputCvarReferences) -> Result<bool> {
        if cvars.try_ai_respawn_behind_always()? {
            return Ok(false);
        }
        if !cvars.try_ai_respawn_behind_in_races()? {
            return Ok(true);
        }
        Ok(!matches!(challenge, challenge_type::RACE_A | challenge_type::RACE_B))
    }

    /// `FindRespawnNodeAhead` 0x824c75b0.
    fn find_respawn_node_ahead(&self, cvars: &InputCvarReferences) -> Result<i32> {
        let path = self.path()?;
        let mut index = (self.current_node_index + 2).min(path::node_count(path) - 1);
        if self.has_bailed_due_to_blocking_object {
            let advance = self.object_blocking_path_info.index + 1 - index;
            if advance > 0 && advance < cvars.try_ai_max_node_advance_for_blocking_object()? {
                index = self.object_blocking_path_info.index + 1;
            }
        }
        Ok(path::find_safe_node_index(path, index))
    }

    /// `FindRespawnNode` 0x824c7660.
    fn find_respawn_node(&self, context: &PathControllerContext<'_>) -> Result<i32> {
        self.find_respawn_node_for_challenge(context.challenge_type, context.cvars)
    }

    fn find_respawn_node_for_challenge(&self, challenge: i32, cvars: &InputCvarReferences) -> Result<i32> {
        if Self::should_respawn_ahead(challenge, cvars)? {
            return self.find_respawn_node_ahead(cvars);
        }
        Ok(path::find_safe_node_index_behind(self.path()?, self.approximated_node.description.index))
    }

    /// AIController.GetAppropriateRespawnTransform824cc208 delegates here
    /// (PathController824ca430). A temporarily resolved path is restored even
    /// when absent; this query neither resets the controller nor caches a newly
    /// found respawn node. Actor owns the subsequent on-board teleport.
    pub(crate) fn appropriate_respawn_transform(&mut self, context: &RespawnContext<'_>) -> Result<Option<[V4; 4]>> {
        if !Self::respawns_on_path(context.client_type, context.respawn_location_set, context.cvars)? {
            let matrix = Self::select_spawn_location(context.respawn_locations, context.skater_positions, context.player)?;
            if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                eprintln!("ai: appropriate respawn player={} locator at {:?}", context.player, matrix[3]);
            }
            return Ok(Some(matrix));
        }
        let retained = self.path.clone();
        if self.path.is_none() {
            self.path = context.paths.get_ai_path(&self.path_id);
        }
        let result = (|| {
            if self.path.is_none() {
                if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                    eprintln!("ai: appropriate respawn player={} missing path {:?}; breadcrumbs", context.player, self.path_id);
                }
                return Ok(None);
            }
            let index = if self.respawn_node_found { self.respawn_node_index }
                else { self.find_respawn_node_for_challenge(context.challenge_type, context.cvars)? };
            let matrix = path::node_matrix(path::node(self.path()?, index));
            if std::env::var_os("SKATE2_TRACE_AI").is_some() {
                eprintln!("ai: appropriate respawn player={} path {:?} node={index} cached={} at {:?}",
                    context.player, self.path_id, self.respawn_node_found, matrix[3]);
            }
            Ok(Some(matrix))
        })();
        self.path = retained;
        result
    }

    /// `TeleportToNode` 0x824ca3a0.
    fn teleport_to_node(&mut self, frame: &mut Frame<'_, '_, '_>, index: i32) -> Result<()> {
        let matrix = path::node_matrix(path::node(self.path()?, index));
        frame.commands.push(AiActorCommand::Teleport(matrix));
        self.full_reset();
        self.current_node_index = index;
        Ok(())
    }

    /// `GetCachedDeckTransformForPhysics` 0x824b53c0: the effective deck transform, recached
    /// whenever it moved more than `kMaxCachedTransformDistance` from the cache.
    fn cached_deck_transform_for_physics(&mut self, actor: &AiActorView) -> [V4; 4] {
        let deck = actor.effective_deck_transform;
        let recache = !self.has_cached_deck_transform_for_physics || {
            let offset = vmx::sub(self.cached_deck_transform_for_physics[3], deck[3]);
            let limit = cvars::MAX_CACHED_TRANSFORM_DISTANCE * cvars::MAX_CACHED_TRANSFORM_DISTANCE;
            vmx::dot3(offset, offset) > limit
        };
        if recache {
            self.cached_deck_transform_for_physics = deck;
            self.has_cached_deck_transform_for_physics = true;
        }
        self.cached_deck_transform_for_physics
    }

    /// `FindNewPath` 0x824cca68: the best of up to 16 paths within the radius, starts first.
    fn find_new_path(&mut self, frame: &mut Frame<'_, '_, '_>, position: V4) -> Result<()> {
        let context = &mut *frame.context;
        let filter = frame.influence.filter.resolve(context.paths);
        let results = context.paths.try_get_paths_in_radius(
            vmx::xyz(position),
            context.attributes.find_new_path_radius,
            16,
            PathQueryType::StartAllowFallback,
            filter.as_ref(),
            Some(context.abilities),
            Some(context.challenge_round),
            context.locked,
        )?;
        if results.is_empty() {
            return Ok(());
        }
        let best = if results.len() == 1 { 0 } else { self.pick_best_path(frame, &results)? };
        let paths = frame.context.paths;
        self.change_path(paths, results[best].path.path().id, results[best].node_index);
        Ok(())
    }

    /// `PickBestPath` 0x824c9368: the lowest score, the first on ties.
    fn pick_best_path(&self, frame: &Frame<'_, '_, '_>, results: &[path_manager::PathQueryResult]) -> Result<usize> {
        let mut best = 0;
        let mut best_score = u32::MAX;
        for (index, result) in results.iter().enumerate() {
            let score = self.calculate_path_score(frame, result.path.path(), result.node_index)?;
            if score < best_score {
                best = index;
                best_score = score;
            }
        }
        Ok(best)
    }

    /// `CalculatePathScore` 0x824c6e30: the heading change to the next node in tenths of a
    /// degree (paths more than 50 degrees off, or starting in the air or at an event, are
    /// unusable), a random 0..400, the other skaters near the node, and for ambient skaters in
    /// game the distance from the local player.
    fn calculate_path_score(&self, frame: &Frame<'_, '_, '_>, path: &AiPath, index: i32) -> Result<u32> {
        let actor = &*frame.actor;
        let position = actor.path_relative_position();
        let deck_z = actor.effective_deck_transform[2];
        let count = path::node_count(path);
        let next = index + 1;
        if next >= count {
            return Ok(u32::MAX);
        }
        let to_next = vmx::sub(vmx::v3(path::node(path, next).position), position);
        let mut angle = signed_angle(deck_z, to_next, actor.transform[1]);
        if angle > PI {
            angle -= TWO_PI;
        }
        let magnitude = angle.abs();
        if magnitude >= f32::from_bits(0x3f5f_66f3) {
            return Ok(u32::MAX);
        }
        let heading = (magnitude * f32::from_bits(0x440f_3d4d)) as i64 as u32;
        if index > 0 {
            for at in index - 1..=next {
                if (0..count).contains(&at) {
                    let node = path::node(path, at);
                    if node.bit_flags & path::NODE_AIRBORNE != 0 || node.event_type != 0 {
                        return Ok(u32::MAX);
                    }
                }
            }
        }
        let draw = rw_random()? % 100_000;
        let random = ((draw as f32 * f32::from_bits(0x3727_c5ac)) * 400.0) as i64 as u32;
        let usage = calculate_current_usage(frame.context.skaters, &path.id, index, 5, actor.player_id);
        let mut score = PATH_USAGE_SCORE_SCALE
            .wrapping_mul(usage)
            .wrapping_add(random)
            .wrapping_add(heading);
        if frame.context.client_type == AiClientType::Ambient
            && !frame.context.in_front_end
            && let Some(local) = frame.context.local_player_position
        {
            let squared = path::min_dist_squared_approx(path, vmx::xyz(local), index);
            let distance = vmx::sqrt_guarded(squared);
            let term = ((distance * 30.0) as i64 as u32).min(1500);
            score = score.wrapping_add(term);
        }
        if path.bit_flags & 7 == 7 {
            score = score.wrapping_add(1000);
        }
        Ok(score)
    }

    /// `PathController::Update` 0x824d3998.
    fn update(&mut self, frame: &mut Frame<'_, '_, '_>, audio: &mut AiOutAudio) -> Result<()> {
        if self.path.is_none() {
            self.find_new_path(frame, frame.actor.path_relative_position())?;
        }
        if self.external_wipeout_request {
            intents::set(frame.intents, name::WIPE_OUT_REQUEST, 1.0)?;
        }
        if self.path.is_none() {
            return Ok(());
        }
        let limit = cvars::MAX_FRAMES_TRYING_TO_WIPE_OUT_FAILED_SO_RESPAWN;
        if self.num_frames_stuck > limit || self.num_frames_stuck_before_start > limit {
            let respawn = self.find_respawn_node(frame.context)?;
            return self.teleport_to_node(frame, respawn);
        }
        let off_board = frame.actor.skater_off_board;
        let transitioning = frame.actor.transitioning_on_off_board;
        self.toggle_offboard_state.resolve_off_board_status(frame.intents, off_board, transitioning)?;
        self.update_off_board(frame.actor);
        self.update_velocity(frame.actor);
        self.update_rubber_banding(frame)?;
        let stuck = self.num_frames_stuck;
        if stuck > cvars::NUM_FRAMES_STUCK_BEFORE_WIPEOUT
            || self.num_frames_stuck_before_start > cvars::NUM_FRAMES_STUCK_PRE_START_NODE_BEFORE_WIPEOUT
        {
            intents::set(frame.intents, name::WIPE_OUT_REQUEST, 1.0)?;
        } else if stuck > cvars::NUM_FRAMES_STUCK_BEFORE_NAV_MESH {
            self.is_swap_required = true;
        }
        self.update_nodes(frame)?;
        self.obstacle_avoider.output_avoidance_decision(audio);
        self.update_airborne_status(frame)?;
        self.calculate_path_deviation(frame.actor, frame.context.attributes);
        if frame.actor.flags & flags::PINNED == 0 {
            if self.obstacle_avoider.avoid_decision == AvoidDecision::GiveUpAndBail && !self.is_airborne {
                self.object_blocking_path_info = self.obstacle_avoider.first_obstacle_node().description;
                self.has_bailed_due_to_blocking_object = true;
                intents::set(frame.intents, name::WIPE_OUT_REQUEST, 1.0)?;
            }
            if self.stuck_so_try_off_board
                || self.obstruction_so_try_running_forward
                || self.obstruction_so_try_jumping_forward
            {
                self.handle_any_obstructions(frame, off_board)?;
            } else {
                self.handle_skitching(frame.intents)?;
                let skitching = frame.actor.physics_state == PHYSICS_STATE_SKITCHING;
                let skitch = skitching || self.is_trying_to_skitch;
                if self.look_ahead_node_soon.bit_flags & path::NODE_OFF_BOARD == 0 || skitch {
                    self.toggle_offboard_state.request(OffBoardToggleStatus::AttemptToGetOnBoard);
                } else {
                    self.toggle_offboard_state.request(OffBoardToggleStatus::AttemptToGetOffBoard);
                }
                let fully_off = off_board && !transitioning;
                let fully_on = !off_board && !transitioning;
                if fully_on {
                    if !skitching {
                        self.generate_speed_intents(frame)?;
                    }
                    if !skitch {
                        self.generate_trick_intents(frame)?;
                    }
                } else if fully_off {
                    self.generate_move_intents(frame, 1.0)?;
                }
            }
        }
        self.generate_turn_intents(frame)?;
        self.blend_to_path(frame)?;
        if self.is_swap_required {
            if !cvars::ENABLE_NAV_MESH_CONTROLLER_SWAP {
                // AIController::SwapControllers: the navmesh controller declines activation.
                if self.num_frames_stuck > cvars::NUM_FRAMES_STUCK_BEFORE_NAV_MESH {
                    intents::set(frame.intents, name::WIPE_OUT_REQUEST, 1.0)?;
                }
                self.is_swap_required = false;
            } else {
                return Err(PathControllerError::ChallengeOnly("NavMeshController"));
            }
        }
        Ok(())
    }

    /// `UpdateOffBoard` 0x824b5368.
    fn update_off_board(&mut self, actor: &AiActorView) {
        self.is_off_board = actor.physics_state_category == PHYSICS_CATEGORY_OFFBOARD;
    }

    /// `UpdateVelocity` 0x824b5180: the position delta times `kfUpdateHz` (the Actor's
    /// velocity before a position is known), its signed length along the skater's facing.
    fn update_velocity(&mut self, actor: &AiActorView) {
        let last = self.position_last_frame;
        let moved = [last[0].abs(), last[1].abs(), last[2].abs(), last[0].abs()].iter().any(|lane| *lane > EPSILON_0);
        self.velocity = if moved {
            vmx::scale(vmx::sub(actor.path_relative_position(), last), path::UPDATE_HZ)
        } else {
            actor.velocity
        };
        self.current_speed = vmx::length3(self.velocity);
        self.effective_actor_direction =
            if self.is_off_board { actor.effective_anim_z } else { actor.effective_deck_transform[2] };
        if self.current_speed > 0.0099999998 && 0.0 > vmx::dot3(self.effective_actor_direction, self.velocity) {
            self.current_speed = -self.current_speed;
        }
    }

    /// `UpdateRubberBanding` 0x824bfa00: in a race or a follow challenge (or a challenge
    /// skater's tutorial) the speed bands against a local player other than this skater. The
    /// race's band needs `cChallengeRaceManager::GetEstimatedTimeToFinish`, which is not ported.
    fn update_rubber_banding(&mut self, frame: &Frame<'_, '_, '_>) -> Result<()> {
        self.rubber_band_speed_scalar = 0.0;
        let challenge = frame.context.challenge_type;
        let race = matches!(challenge, challenge_type::RACE_A | challenge_type::RACE_B);
        let follow = challenge == challenge_type::FOLLOW
            || (frame.context.client_type == AiClientType::Challenge && challenge == 0);
        if !(race || follow) {
            return Ok(());
        }
        let Some(local) = frame.context.local_player_position else { return Ok(()) };
        match frame.context.local_player {
            Some(local_player) if race => self.calculate_rubber_banding_for_race(frame, local_player),
            _ => self.calculate_rubber_banding_for_follow(frame, local),
        }
        Ok(())
    }

    /// `CalculateRubberBandingForRace` 0x824bf3a8: the gap between the local player's and this
    /// skater's estimated times to finish over the race window, negative when the player is
    /// behind; a wiping out player caps it.
    fn calculate_rubber_banding_for_race(&mut self, frame: &Frame<'_, '_, '_>, local_player: i32) {
        let times = frame.context.race_estimated_times;
        let time = |player: i32| times.iter().find(|(racer, _)| *racer == player).map_or(0.0, |(_, time)| *time);
        let gap = time(local_player) - time(frame.actor.player_id);
        let distance = gap.abs();
        let attributes = frame.context.attributes;
        let (low, high) = (attributes.race_rubber_band_min, attributes.race_rubber_band_max);
        if distance > low {
            self.rubber_band_speed_scalar = if distance >= high { 1.0 } else { (distance - low) / (high - low) };
        }
        if gap > 0.0 {
            self.rubber_band_speed_scalar = -self.rubber_band_speed_scalar;
        }
        if frame.context.local_player_wiping_out {
            let cap = attributes.race_rubber_band_wipeout;
            if self.rubber_band_speed_scalar - cap >= 0.0 {
                self.rubber_band_speed_scalar = cap;
            }
        }
    }

    /// `CalculateRubberBandingForFollow` 0x824bf6f8: in a follow challenge the distance to the
    /// player over the follow window, negative when the player is behind the push direction; a
    /// wiping out player caps it.
    fn calculate_rubber_banding_for_follow(&mut self, frame: &Frame<'_, '_, '_>, local: V4) {
        if !self.enable_follow_me_rubber_banding {
            self.rubber_band_speed_scalar = 0.0;
            return;
        }
        let attributes = frame.context.attributes;
        if frame.context.challenge_type == challenge_type::FOLLOW {
            let offset = vmx::sub(local, frame.actor.path_relative_position());
            let behind = 0.0 > vmx::dot3(frame.actor.push_direction(), offset);
            let distance = vmx::length3(offset);
            let (low, high) = (attributes.follow_rubber_band_min, attributes.follow_rubber_band_max);
            if distance > low {
                self.rubber_band_speed_scalar = if distance >= high { 1.0 } else { (distance - low) / (high - low) };
            }
            if behind {
                self.rubber_band_speed_scalar = -self.rubber_band_speed_scalar;
            }
        } else {
            self.rubber_band_speed_scalar = 0.0;
        }
        if frame.context.local_player_wiping_out {
            let cap = attributes.follow_rubber_band_wipeout;
            if self.rubber_band_speed_scalar - cap >= 0.0 {
                self.rubber_band_speed_scalar = cap;
            }
        }
    }

    /// `AdjustDesiredSpeed` 0x824bfb80: the influence's speed adjustment and the rubber band,
    /// then the avoider's speed window unless stuck.
    fn adjust_desired_speed(&self, frame: &Frame<'_, '_, '_>, desired: f32) -> (bool, f32) {
        let attributes = frame.context.attributes;
        let adjustment = ((attributes.speed_adjustment_scale * desired) * frame.influence.skater_speed_adjustment)
            + ((attributes.rubber_band_scale * desired) * self.rubber_band_speed_scalar);
        let adjusted = adjustment + desired;
        let changed = adjustment.abs() > 0.0099999998;
        if self.stuck_so_try_off_board || self.obstruction_so_try_running_forward || self.obstruction_so_try_jumping_forward
        {
            return (changed, adjusted);
        }
        let (avoided, out) = self.obstacle_avoider.adjust_desired_speed(adjusted);
        (changed || avoided, out)
    }

    /// `UpdateNodes` 0x824ccdf8.
    fn update_nodes(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        self.advance_and_branch(frame)?;
        self.update_approximated_node(frame)?;
        if !self.has_joined_path {
            self.attempt_to_join_path(frame.context.attributes);
        }
        if self.has_joined_path && !self.is_physics_fixup_allowed_yet {
            let description = self.approximated_node.description;
            self.is_physics_fixup_allowed_yet =
                description.index > 3 || (description.index == 3 && description.ratio > 0.000099999997);
        }
        self.update_predicted_node(frame)?;
        let approximated = self.approximated_node.description;
        let path_ref = self.path_ref()?;
        let (soon_index, soon_ratio) = self.future_from_approximated(cvars::LOOK_AHEAD_SOON_FRAME_AMOUNT)?;
        path::set_interpolated_node(path_ref.path(), &mut self.look_ahead_node_soon, soon_index, soon_ratio);
        let mut far = self.look_ahead_node_far;
        self.set_look_ahead_node(&mut far, approximated.index, approximated.ratio, cvars::LOOK_AHEAD_FAR_FRAME_AMOUNT)?;
        self.look_ahead_node_far = far;
        let (speed_index, speed_ratio) = self.future_from_approximated(cvars::LOOK_AHEAD_SPEED_FRAME_AMOUNT)?;
        path::set_interpolated_node(path_ref.path(), &mut self.look_ahead_node_speed, speed_index, speed_ratio);
        let soon_speed = vmx::length3(vmx::scale(vmx::v3(self.look_ahead_node_soon.direction), path::UPDATE_HZ));
        let launch = self.impending_trick_node && self.impending_trick_will_require_launching;
        let local = AvoiderLocal {
            player_id: frame.actor.player_id,
            position: frame.actor.skeleton_root[3],
            up: frame.actor.transform[1],
            path: path_ref.path(),
            velocity: self.velocity,
            effective_actor_direction: self.effective_actor_direction,
            current_node_index: self.current_node_index,
            path_width_ratio: self.path_width_ratio,
            is_off_board: self.is_off_board,
            has_joined_path: self.has_joined_path,
            collide_with_world_only: frame.actor.collide_with_world_only,
        };
        self.obstacle_avoider.update(
            &local,
            frame.context.obstacle_skaters,
            frame.context.obstacle_entities,
            frame.context.attributes,
            soon_speed,
            launch,
        );
        self.is_steering_to_avoid = false;
        let decision = self.obstacle_avoider.avoid_decision;
        if decision == AvoidDecision::RunOver {
            if !self.obstruction_so_try_running_forward && !self.obstruction_so_try_jumping_forward {
                self.obstruction_so_try_running_forward = true;
            }
        } else if decision == AvoidDecision::JumpOver && !self.obstruction_so_try_jumping_forward {
            self.obstruction_so_try_running_forward = true;
            self.obstruction_so_try_jumping_forward = false;
        }
        if self.obstruction_so_try_running_forward || self.obstruction_so_try_jumping_forward {
            self.current_trick_type = trick_type::NONE;
            return Ok(());
        }
        if matches!(decision, AvoidDecision::GoAround | AvoidDecision::Skitch | AvoidDecision::Blocked) {
            let mut avoidance = InterpolatedNode { node: path::constructed_node(), description: NodeDescription::default() };
            self.set_look_ahead_node(
                &mut avoidance,
                approximated.index,
                approximated.ratio,
                cvars::LOOK_AHEAD_AVOIDANCE_FRAME_AMOUNT,
            )?;
            let mut far = self.look_ahead_node_far;
            self.is_steering_to_avoid = self.obstacle_avoider.adjust_desired_destination_node(&mut far, &avoidance);
            self.look_ahead_node_far = far;
        }
        let avoider = &self.obstacle_avoider;
        if avoider.avoid_decision == AvoidDecision::GoBehind
            && !avoider.imminent_trick_launch
            && !(avoider.max_speed_to_pass_behind_of_all_obstacles >= 1.0)
            && !frame.actor.skateboard_airborne
            && matches!(self.current_trick_type, trick_type::GRAB | trick_type::MANUAL)
        {
            self.current_trick_type = trick_type::NONE;
        }
        if !self.is_airborne && matches!(decision, AvoidDecision::Skitch | AvoidDecision::Blocked) {
            self.current_trick_type = trick_type::NONE;
            self.check_for_impending_off_board()?;
        } else {
            self.check_for_impending_trick(frame.context.tricks)?;
            self.check_for_impending_off_board()?;
        }
        Ok(())
    }

    /// The look-ahead `UpdateNodes` builds inline: from the approximated description, `frames`
    /// ahead once it has left the path start.
    fn future_from_approximated(&self, frames: f32) -> Result<(i32, f32)> {
        let description = self.approximated_node.description;
        if description.index > 0 || description.ratio > 0.001 {
            let mut ratio = description.ratio;
            let index = path::future_node_index(self.path()?, description.index, &mut ratio, frames);
            Ok((index, ratio))
        } else {
            Ok((description.index, description.ratio))
        }
    }

    /// `SetLookAheadNode` 0x824c6da0.
    fn set_look_ahead_node(&self, node: &mut InterpolatedNode, index: i32, ratio: f32, frames: f32) -> Result<()> {
        let path = self.path()?;
        let (index, ratio) = if index > 0 || ratio > 0.001 {
            let mut ratio = ratio;
            (path::future_node_index(path, index, &mut ratio, frames), ratio)
        } else {
            (index, ratio)
        };
        node.description = NodeDescription { index, ratio };
        path::set_interpolated_node(path, &mut node.node, index, ratio);
        Ok(())
    }

    /// `AdvanceAndBranch` 0x824ccbc8.
    fn advance_and_branch(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let obstructed =
            self.stuck_so_try_off_board || self.obstruction_so_try_running_forward || self.obstruction_so_try_jumping_forward;
        if !obstructed && self.num_frames_stuck > 30 && self.stuck_reset_count < 2 {
            self.attempt_to_unstick_skater(frame.actor)?;
        }
        let previous = self.current_node_index;
        let position = frame.actor.path_relative_position();
        let nearest = path::nearest_node_index_in(self.path()?, vmx::xyz(position), previous, 4);
        let count = path::node_count(self.path()?);
        self.current_node_index = nearest;
        let at_end = if self.has_joined_path {
            !(self.predicted_node.description.index < count - 2)
        } else {
            !(nearest < count - 1)
        };
        if !at_end {
            if previous != nearest {
                return self.check_for_branch(frame);
            }
            if self.is_swap_required_at(frame.actor, cvars::MIN_PATH_DISTANCE_TO_SWITCH_TO_NAVMESH_WHEN_ON_PATH)? {
                self.is_swap_required = true;
            }
            return Ok(());
        }
        let predicted = self.predicted_node.description;
        let finished = !self.has_joined_path
            || predicted.index > count - 2
            || (predicted.index == count - 2 && predicted.ratio > 0.80000001);
        if finished {
            if cvars::TELEPORT_BACK_TO_PATH_START {
                return self.teleport_to_node(frame, 0);
            }
            let end = vmx::v3(path::node(self.path()?, count - 1).position);
            self.find_new_path(frame, end)?;
            if self.is_swap_required_at(frame.actor, cvars::MIN_PATH_DISTANCE_TO_SWITCH_TO_NAVMESH_AT_END_OF_PATH)? {
                self.is_swap_required = true;
            }
        }
        Ok(())
    }

    /// `IsSwapRequired` 0x824be140: off the path, farther than the distance from the current
    /// node, with the navmesh swap enabled.
    fn is_swap_required_at(&self, actor: &AiActorView, distance: f32) -> Result<bool> {
        if self.has_joined_path || !cvars::ENABLE_NAV_MESH_CONTROLLER_SWAP {
            return Ok(false);
        }
        let node = vmx::v3(path::node(self.path()?, self.current_node_index).position);
        let offset = vmx::sub(actor.path_relative_position(), node);
        Ok(vmx::dot3(offset, offset) > distance * distance)
    }

    /// `CheckForBranch` 0x824cc7c8: on the ground, outside an air or grind trick, at a branch
    /// group: the best of staying and each branch the skater may take.
    fn check_for_branch(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        if frame.influence.branch_mode == BranchMode::DoNotBranch || frame.actor.skateboard_airborne {
            return Ok(());
        }
        if matches!(self.current_trick_type, trick_type::GRIND | trick_type::AIR | trick_type::FLIP)
            || (self.impending_trick_node && self.impending_trick_will_require_launching)
            || !self.local_world_is_loaded
        {
            return Ok(());
        }
        let path_ref = self.path.clone().ok_or(PathControllerError::MissingPath)?;
        let Some(group) = path::branch_group(path_ref.path(), self.current_node_index) else {
            return Ok(());
        };
        if group.branches.is_empty() {
            return Ok(());
        }
        let mut results = vec![path_manager::PathQueryResult {
            path: path_ref.clone(),
            node_index: self.current_node_index,
            dist_sqr: 0.0,
        }];
        let filter = frame.influence.filter.resolve(frame.context.paths);
        for branch in &group.branches {
            let Some(destination) = frame.context.paths.get_ai_path(&branch.destination) else { continue };
            if !PathManager::should_skip_path(
                destination.path(),
                filter.as_ref(),
                Some(frame.context.abilities),
                Some(frame.context.challenge_round),
                frame.context.locked,
            ) {
                results.push(path_manager::PathQueryResult {
                    path: destination,
                    node_index: branch.destination_node_index,
                    dist_sqr: 0.0,
                });
            }
        }
        let best = self.pick_best_path(frame, &results)?;
        if best != 0 {
            let destination = &results[best];
            let target = destination.node_index;
            let mut start = target - 3;
            if start < 0 {
                start = 0;
            }
            let count = target - start;
            let index = if count > 0 {
                let position = frame.actor.path_relative_position();
                path::nearest_node_index_in(destination.path.path(), vmx::xyz(position), start, count)
            } else {
                start
            };
            let id = destination.path.path().id;
            let paths = frame.context.paths;
            self.change_path(paths, id, index);
        }
        Ok(())
    }

    /// `AttemptToUnstickSkater` 0x824c9ac0: up a steep stretch, restart past it where the path
    /// drops below the current node; else at the nearest node.
    fn attempt_to_unstick_skater(&mut self, actor: &AiActorView) -> Result<()> {
        let path = self.path()?;
        let count = path::node_count(path);
        let current = self.current_node_index;
        let mut restart = -1;
        if current + 2 < count {
            let near = vmx::v3(path::node(path, current).position);
            let far = vmx::v3(path::node(path, current + 2).position);
            if calculate_gradient(far, near) > 1.0 {
                let base = near[1];
                let mut frames = 0i32;
                let mut at = current + 1;
                while at < count {
                    let node = path::node(path, at);
                    frames += i32::from(node.frame_count_since_last_node);
                    if at > current + 2 {
                        if !(node.position[1] >= base) {
                            restart = at;
                            break;
                        }
                        if frames > 240 {
                            break;
                        }
                    }
                    at += 1;
                }
            }
        }
        let resets = self.stuck_reset_count;
        let index = if restart < 0 {
            path_manager::nearest_node_index(path, vmx::xyz(actor.path_relative_position()))?
        } else {
            restart
        };
        self.minimal_reset();
        self.current_node_index = index;
        self.stuck_reset_count = resets + 1;
        Ok(())
    }

    /// `UpdateApproximatedNode` 0x824c93f8.
    fn update_approximated_node(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let old = self.approximated_node.description;
        let position = frame.actor.path_relative_position();
        let path = self.path.clone().ok_or(PathControllerError::MissingPath)?;
        let path = path.path();
        let mut ratio = 0.0;
        let mut index = path::approximated_node_index(path, vmx::xyz(position), self.current_node_index, &mut ratio);
        let predicted_offset = vmx::sub(vmx::v3(self.predicted_node.node.position), position);
        let predicted_squared = vmx::dot3(predicted_offset, predicted_offset);
        let mut node = path::constructed_node();
        path::set_interpolated_node(path, &mut node, index, ratio);
        let approximated_offset = vmx::sub(vmx::v3(node.position), position);
        let approximated_squared = vmx::dot3(approximated_offset, approximated_offset);
        if !(predicted_squared >= approximated_squared + 0.0099999998) {
            index = self.predicted_node.description.index;
            ratio = self.predicted_node.description.ratio;
        }
        let predicted = self.predicted_node.description;
        let predicted_ahead = predicted.index > index || (predicted.index == index && predicted.ratio > ratio);
        let mut stuck_now = false;
        if predicted_ahead && (old.index > 0 || (old.index == 0 && old.ratio > 0.1)) {
            let advanced = path::node_frame_difference_unordered(path, old.index, old.ratio, index, ratio);
            let expected = path::node_frame_difference_unordered(path, old.index, old.ratio, predicted.index, predicted.ratio);
            let minimum = cvars::MINIMUM_FRAME_ADVANCE_RATIO * expected;
            if expected > 0.0 && advanced < minimum {
                let mut future_ratio = old.ratio;
                let future = path::future_node_index(path, old.index, &mut future_ratio, minimum);
                path::set_interpolated_node(path, &mut node, future, future_ratio);
                let mut offset = vmx::sub(vmx::v3(node.position), position);
                offset[1] = 0.0;
                if vmx::dot3(offset, offset) < 1.0 {
                    index = future;
                    ratio = future_ratio;
                }
                if advanced < cvars::MINIMUM_FRAME_ADVANCE_FOR_NOT_STUCK_RATIO * expected {
                    stuck_now = true;
                }
            }
        }
        self.approximated_node.description = NodeDescription { index, ratio };
        path::set_interpolated_node(path, &mut self.approximated_node.node, index, ratio);
        if self.approximated_node.description.index > self.current_node_index {
            self.current_node_index = self.approximated_node.description.index;
        }
        let actor = &*frame.actor;
        let off = actor.skater_off_board;
        let transitioning = actor.transitioning_on_off_board;
        let pinned = actor.flags & flags::PINNED != 0;
        let fully_off = off && !transitioning;
        let fully_on = !off && !transitioning;
        let rolling = fully_on && !pinned && !actor.skateboard_airborne;
        let walking = fully_off && !pinned && !actor.mounting && !actor.unmounting;
        let moving = rolling || walking;
        let obstructed =
            self.stuck_so_try_off_board || self.obstruction_so_try_running_forward || self.obstruction_so_try_jumping_forward;
        let was_trying_off_board = self.stuck_so_try_off_board;
        if obstructed {
            self.num_frames_stuck += 1;
        } else if stuck_now {
            if moving {
                let stuck = self.num_frames_stuck;
                if stuck <= cvars::NUMBER_OF_STUCK_FRAMES_BEFORE_TRYING_OFF_BOARD
                    || stuck >= cvars::NUMBER_OF_STUCK_FRAMES_TO_STOP_TRYING_OFF_BOARD
                {
                    if was_trying_off_board {
                        self.stuck_so_try_off_board = false;
                    }
                } else if !was_trying_off_board {
                    self.stuck_so_try_off_board = true;
                }
                if !actor.running_out {
                    self.num_frames_stuck += 1;
                }
                self.num_frames_not_stuck = 0;
            }
        } else {
            self.num_frames_not_stuck += 1;
            if self.num_frames_not_stuck > cvars::NUMBER_OF_UN_STUCK_FRAMES_BEFORE_UNSTUCK {
                self.num_frames_stuck = 0;
                self.stuck_reset_count = 0;
            }
        }
        let previous_distance = self.distance_from_path;
        self.calculate_path_width_ratio(actor, path);
        if !obstructed {
            let description = self.approximated_node.description;
            if description.index > 0 || (description.index == 0 && description.ratio > 0.1) {
                self.num_frames_stuck_before_start = 0;
            } else if moving && !(self.distance_from_path < previous_distance) {
                self.num_frames_stuck_before_start += 1;
            }
        }
        Ok(())
    }

    /// `CalculatePathWidthRatio` 0x824be940: the skater's distance from the approximated node
    /// over the path width on its side, and over the narrowest width of the next four nodes.
    fn calculate_path_width_ratio(&mut self, actor: &AiActorView, path: &AiPath) {
        let offset = vmx::sub(actor.path_relative_position(), vmx::v3(self.approximated_node.node.position));
        let distance = vmx::length3(offset);
        self.distance_from_path = distance;
        if !(distance > 0.000099999997) {
            self.is_within_path_width = true;
            self.path_width_ratio_conservative = 0.0;
            self.path_width_ratio = 0.0;
            return;
        }
        let direction = vmx::v3(self.approximated_node.node.direction);
        let inverse = vmx::rsqrt(vmx::dot3(direction, direction));
        let unit = vmx::scale(direction, inverse);
        let up = [0.0, 1.0, 0.0, 0.0];
        let side = vmx::cross(unit, up);
        let left = 0.0 > vmx::dot3(offset, side);
        let width_of = |node: &AiPathNode| f32::from(if left { node.width_left } else { node.width_right });
        let width = width_of(&self.approximated_node.node) * path::METRES_PER_NODE_WIDTH_UNIT;
        let start = self.approximated_node.description.index + 1;
        let end = (self.approximated_node.description.index + 5).min(path::node_count(path));
        let mut narrowest = width;
        for at in start..end {
            let ahead = width_of(path::node(path, at)) * path::METRES_PER_NODE_WIDTH_UNIT;
            if ahead < narrowest {
                narrowest = ahead;
            }
        }
        self.is_within_path_width = distance < width;
        if width <= 0.000099999997 {
            self.path_width_ratio = f32::MAX;
            return;
        }
        let ratio = distance * (1.0 / width);
        self.path_width_ratio = ratio;
        let conservative = if narrowest <= 0.000099999997 { 999.0 } else { distance / narrowest };
        self.path_width_ratio_conservative = conservative;
        if left {
            self.path_width_ratio_conservative = -conservative;
            self.path_width_ratio = -ratio;
        }
    }

    /// `AttemptToJoinPath` 0x824ca2b0.
    fn attempt_to_join_path(&mut self, attributes: &AiSkaterAttributes) {
        let description = self.approximated_node.description;
        if (description.index > 0 || (description.index == 0 && description.ratio > 0.000099999997))
            && (self.is_within_path_width || self.distance_from_path < attributes.join_path_distance)
        {
            let node = self.approximated_node;
            self.join_path(&node);
        }
    }

    /// `JoinPath` 0x824c7370.
    fn join_path(&mut self, node: &InterpolatedNode) {
        self.predicted_node.description = node.description;
        path::set_generic(&mut self.predicted_node.node, &node.node);
        self.predicted_node.node.frame_count_since_last_node = node.node.frame_count_since_last_node;
        self.has_joined_path = true;
        self.has_ever_been_joined_to_this_path = true;
    }

    /// `LeavePath` 0x824b54d0.
    fn leave_path(&mut self) {
        self.has_joined_path = false;
        self.current_trick_type = trick_type::NONE;
    }

    /// `UpdatePredictedNode` 0x824c73f8: the approximated node one adjusted frame ahead.
    fn update_predicted_node(&mut self, frame: &Frame<'_, '_, '_>) -> Result<()> {
        self.last_frame_predicted_node_index = self.predicted_node.description.index;
        let speed = vmx::length3(vmx::scale(vmx::v3(self.approximated_node.node.direction), path::UPDATE_HZ));
        let mut frames = 1.0;
        if speed > 0.001 {
            let (adjusted, out) = self.adjust_desired_speed(frame, speed);
            if adjusted {
                frames = out / speed;
            }
        }
        let description = self.approximated_node.description;
        let mut ratio = description.ratio;
        let mut index = description.index;
        if frames > 0.0000099999997 {
            index = path::future_node_index(self.path()?, description.index, &mut ratio, frames);
        }
        self.predicted_node.description = NodeDescription { index, ratio };
        let path = self.path.clone().ok_or(PathControllerError::MissingPath)?;
        path::set_interpolated_node(path.path(), &mut self.predicted_node.node, index, ratio);
        Ok(())
    }

    /// `CheckForImpendingTrick` 0x824be230: the next trick start within 60 frames while near
    /// the path.
    fn check_for_impending_trick(&mut self, tricks: &TrickTable) -> Result<()> {
        if !(self.distance_from_path <= 3.0) {
            self.impending_trick_node = false;
            return Ok(());
        }
        let description = self.approximated_node.description;
        let path_ref = self.path_ref()?;
        let path = path_ref.path();
        let mut frames = self.impending_trick_node_frame_count;
        let index = path::next_event_node_index(path, description.index, description.ratio, 60, &mut frames, &[path::EVENT_START_TRICK]);
        self.impending_trick_node_frame_count = frames;
        self.impending_trick_node_index = index;
        if index < 0 {
            self.start_impending_trick = false;
            self.impending_trick_node = false;
            return Ok(());
        }
        self.impending_trick_node = true;
        let node = *path::node(path, index);
        self.impending_trick = path::trick_scorable(&node);
        let kind = path::node_trick_type(&node);
        self.impending_trick_type = kind;
        let launching = kind == trick_type::AIR || kind == trick_type::FLIP;
        self.impending_trick_will_require_launching = launching;
        self.impending_trick_desired_anticipation = if launching { 1.0 } else { 0.0 };
        self.impending_trick_desired_anticipation_angle = 0.0;
        if self.impending_trick_desired_anticipation > 0.0 && self.next_trick_will_be_nollie_type(tricks, &node) {
            self.impending_trick_desired_anticipation_angle = PI;
        }
        Ok(())
    }

    /// `NextTrickWillBeNollieType` 0x824b4938.
    fn next_trick_will_be_nollie_type(&self, _tricks: &TrickTable, node: &AiPathNode) -> bool {
        is_nollie_scorable(path::trick_scorable(node))
    }

    /// `CheckForImpendingOffBoard` 0x824c6688.
    fn check_for_impending_off_board(&mut self) -> Result<()> {
        if !(self.distance_from_path <= 3.0) {
            self.impending_off_board_node = false;
            return Ok(());
        }
        let description = self.approximated_node.description;
        let mut frames = self.impending_off_board_frame_count;
        let index = path::next_flagged_node_index(
            self.path()?,
            description.index,
            description.ratio,
            cvars::MAX_FRAMES_AWAY_FOR_APPROACHING_OFF_BOARD,
            &mut frames,
            3,
        );
        self.impending_off_board_frame_count = frames;
        self.impending_off_board_index = index;
        if index >= 0 {
            self.impending_off_board_node = true;
        }
        Ok(())
    }

    /// `CalculatePathDeviation` 0x824c9090: the angle from the push direction to the path's
    /// heading soon, blended near the path with the angle toward the far look-ahead.
    fn calculate_path_deviation(&mut self, actor: &AiActorView, attributes: &AiSkaterAttributes) {
        let push = actor.push_direction();
        let deck_up = actor.effective_deck_transform[1];
        let mut heading = path::effective_board_orientation_matrix(&self.look_ahead_node_soon)[2];
        if self.has_joined_path && 0.0 > vmx::dot3(heading, push) {
            heading = heading.map(|lane| -lane);
        }
        let mut along = signed_angle(push, heading, actor.transform[1]);
        if along > PI {
            along -= TWO_PI;
        }
        let mut toward_far = vmx::v3(self.look_ahead_node_far.node.position);
        toward_far[3] = deck_up[3];
        let toward_far = remove_component(vmx::sub(toward_far, actor.path_relative_position()), deck_up);
        let toward = if vmx::dot3(toward_far, toward_far) > 0.0000099999997 {
            let mut angle = signed_angle(push, toward_far, actor.deck_transform[1]);
            if angle > PI {
                angle -= TWO_PI;
            }
            angle
        } else {
            along
        };
        self.deviation_from_path_in_radians =
            if self.is_steering_to_avoid || self.distance_from_path >= attributes.deviation_blend_distance {
                toward
            } else {
                (along + toward) * 0.5
            };
    }

    /// `UpdateAirborneStatus` 0x824d17b0.
    fn update_airborne_status(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let actor = &*frame.actor;
        let was_airborne = self.is_airborne;
        let was_scoring_air = self.is_scoring_in_air_state;
        self.is_airborne = actor.skateboard_airborne;
        self.is_scoring_in_air_state = actor.in_state_air;
        if actor.in_state_air {
            self.most_recent_air_spin_180_count = actor.air_spin_180_count;
        }
        let took_off = !was_airborne && self.is_airborne;
        let landed = was_airborne && !self.is_airborne;
        if (took_off || landed) && std::env::var_os("SKATE2_TRACE_AI_SPIN").is_some() {
            eprintln!(
                "ai spin: player {} {} spins {} trick {} node {} recorded spin {}",
                actor.player_id,
                if took_off { "took off" } else { "landed" },
                self.most_recent_air_spin_180_count,
                self.last_trick_scorable_id,
                self.approximated_node.description.index,
                path::air_spin_180_count(&self.approximated_node.node),
            );
        }
        let left_air_state = was_scoring_air && !self.is_scoring_in_air_state;
        if took_off {
            let skater_z = path::skater_orientation_matrix(&self.approximated_node.node)[2];
            self.player_is_flipped = 0.0 > vmx::dot3(skater_z, actor.skeleton_root[2]);
            self.num_frames_since_on_ground = 0;
            self.has_landing_node_been_found = false;
            let landing = path::next_node_on_ground_index(self.path()?, self.approximated_node.description.index);
            if landing >= 0 {
                let node = *path::node(self.path()?, landing);
                path::set_generic(&mut self.landing_node, &node);
                self.landing_node.frame_count_since_last_node = node.frame_count_since_last_node;
                self.calculate_required_board_adjust_for_landing(actor);
                self.has_landing_node_been_found = true;
            }
        } else {
            self.num_frames_since_on_ground += 1;
        }
        if landed {
            self.num_frames_since_airborne = 0;
        } else {
            self.num_frames_since_airborne += 1;
        }
        if (landed || left_air_state) && tricks::is_valid_scorable(self.last_trick_scorable_id) {
            let aerial = matches!(
                tricks::scorable_category(self.last_trick_scorable_id),
                category::GRABS | category::FLIPS | category::AIR
            );
            if aerial && self.last_trick_scorable_id != path::scorable::COFFIN {
                let spins = self.most_recent_air_spin_180_count.unsigned_abs();
                let wipe = cvars::BAIL_ON_EVERY_LANDING
                    || (frame.context.client_type != AiClientType::Ambient
                        && frame.context.profile.try_random_should_wipeout(
                            frame.context.tricks,
                            self.last_trick_scorable_id,
                            spins,
                        )?);
                if wipe {
                    intents::set(frame.intents, name::WIPE_OUT_REQUEST, 1.0)?;
                }
                self.last_trick_scorable_id = -1;
            }
        }
        if landed {
            self.requires_hold_pattern = false;
        }
        Ok(())
    }

    /// `CalculateRequiredBoardAdjustForLanding` 0x824be6f8: the board's heading against the
    /// skater's at the landing node, folded into a half turn either way.
    fn calculate_required_board_adjust_for_landing(&mut self, actor: &AiActorView) {
        let _ = actor;
        let board = path::board_orientation_matrix(&self.landing_node);
        let skater = path::skater_orientation_matrix(&self.landing_node);
        let up = [0.0, 1.0, 0.0, 0.0];
        let forward = [0.0, 0.0, 1.0, 0.0];
        let board_angle = signed_angle(board[2], forward, up);
        let skater_angle = signed_angle(skater[2], forward, up);
        let mut difference = board_angle - skater_angle;
        if difference > PI {
            difference -= TWO_PI;
        } else if difference < -PI {
            difference += TWO_PI;
        }
        if difference > HALF_PI {
            difference -= PI;
        } else if difference < -HALF_PI {
            difference += PI;
        }
        let magnitude = difference.abs();
        if magnitude <= f32::from_bits(0x3ea0_d97c) {
            self.is_board_adjust_required = false;
        } else {
            self.board_adjust_angle = if difference >= 0.0 { HALF_PI } else { -HALF_PI };
            self.is_board_adjust_required = true;
            self.board_adjust_magnitude = magnitude * f32::from_bits(0x3f22_f983);
        }
    }

    /// `HandleAnyObstructions` 0x824d0f80.
    fn handle_any_obstructions(&mut self, frame: &mut Frame<'_, '_, '_>, off_board: bool) -> Result<()> {
        if self.obstruction_so_try_running_forward {
            if self.num_frames_stuck >= cvars::NUMBER_OF_STUCK_FRAMES_TO_TRY_AVOIDING_DMO {
                self.obstruction_so_try_running_forward = false;
            } else {
                self.toggle_offboard_state.request(OffBoardToggleStatus::AttemptToGetOffBoard);
            }
        } else if self.obstruction_so_try_jumping_forward {
            if self.num_frames_stuck >= cvars::NUMBER_OF_STUCK_FRAMES_TO_TRY_AVOIDING_DMO {
                self.obstruction_so_try_jumping_forward = false;
            } else {
                self.toggle_offboard_state.request(OffBoardToggleStatus::AttemptToGetOffBoard);
            }
        } else if self.stuck_so_try_off_board {
            if self.num_frames_stuck >= cvars::NUMBER_OF_STUCK_FRAMES_TO_STOP_TRYING_OFF_BOARD {
                self.stuck_so_try_off_board = false;
            } else {
                self.toggle_offboard_state.request(OffBoardToggleStatus::AttemptToGetOffBoard);
            }
        }
        if off_board {
            self.generate_move_intents(frame, 0.69999999)?;
            self.generate_turn_intents_off_board(frame)?;
            if self.obstruction_so_try_jumping_forward {
                intents::insert(frame.intents, name::OB_JUMP, 1.0)?;
            }
        }
        Ok(())
    }

    /// `HandleSkitching` 0x824d22c8.
    fn handle_skitching(&mut self, intents: &mut ActorGraphIntents) -> Result<()> {
        if self.is_airborne || self.obstacle_avoider.avoid_decision != AvoidDecision::Skitch {
            self.is_trying_to_skitch = false;
        } else {
            intents::set(intents, name::GRAB_WORLD, 1.0)?;
            self.is_trying_to_skitch = true;
        }
        Ok(())
    }

    /// `GenerateMoveIntents` 0x824c6960: walk toward the far look-ahead at a speed that grows
    /// with the squared distance to it, once facing it.
    fn generate_move_intents(&mut self, frame: &mut Frame<'_, '_, '_>, max_speed: f32) -> Result<()> {
        const MIN_DOT_PROD_FOR_MOVE: f32 = 0.1;
        const FACING_THRESHOLD: f32 = 0.25;
        const MOVE_SPEED_THRESHOLD: f32 = 7.0;
        let offset = vmx::sub(vmx::v3(self.look_ahead_node_far.node.position), frame.actor.path_relative_position());
        let distance_squared = vmx::dot3(offset, offset);
        let direction = vmx::normalize_safe([offset[0], 0.0, offset[2], 0.0], [0.0; 4]);
        let anim = frame.actor.effective_anim_z;
        let facing = vmx::normalize_safe([anim[0], 0.0, anim[2], 0.0], [0.0; 4]);
        let dot = vmx::dot3(facing, direction);
        if dot >= MIN_DOT_PROD_FOR_MOVE {
            let fraction = if dot >= FACING_THRESHOLD { 1.0 } else { dot / FACING_THRESHOLD };
            let mut speed = (distance_squared * fraction) / MOVE_SPEED_THRESHOLD;
            if speed < cvars::PATH_OFF_BOARD_MIN_MOVE_SPEED {
                speed = cvars::PATH_OFF_BOARD_MIN_MOVE_SPEED;
            }
            if speed > max_speed {
                speed = max_speed;
            }
            off_board::generate_move_intents(frame.intents, speed)?;
        }
        Ok(())
    }

    /// `GenerateTurnIntentsOffBoard` 0x824c6b98.
    fn generate_turn_intents_off_board(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let full = frame.context.attributes.full_turn_deviation_degrees * DEGREES_TO_RADIANS;
        let offset = vmx::sub(vmx::v3(self.look_ahead_node_far.node.position), frame.actor.path_relative_position());
        let toward = [offset[0], 0.0, offset[2], 0.0];
        let anim = frame.actor.effective_anim_z;
        let facing = [anim[0], 0.0, anim[2], 0.0];
        let mut angle = signed_angle(toward, facing, [0.0, 1.0, 0.0, 0.0]);
        if angle > PI {
            angle -= TWO_PI;
        }
        let magnitude = angle.abs();
        let mut turn = if magnitude >= full { 1.0 } else { magnitude / full };
        if angle < 0.0 {
            turn = -turn;
        }
        off_board::generate_turn_intents(frame.intents, turn)?;
        Ok(())
    }

    /// `GenerateSpeedIntents` 0x824d1fc0: the look-ahead's speed (at least the off-path minimum
    /// when far off the path), adjusted, pushed toward or braked toward.
    fn generate_speed_intents(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        if frame.actor.skateboard_airborne {
            return Ok(());
        }
        let attributes = frame.context.attributes;
        let mut desired = vmx::length3(vmx::scale(vmx::v3(self.look_ahead_node_speed.direction), path::UPDATE_HZ));
        let gradient = calculate_gradient(
            vmx::v3(self.look_ahead_node_speed.position),
            vmx::v3(self.approximated_node.node.position),
        );
        if self.distance_from_path > attributes.off_path_distance_for_minimum_speed {
            let minimum = attributes.off_path_minimum_speed;
            desired = if desired - minimum >= 0.0 { desired } else { minimum };
        }
        let (_, adjusted) = self.adjust_desired_speed(frame, desired);
        if self.current_speed >= adjusted {
            self.generate_brake_intents(frame, adjusted, gradient)
        } else {
            self.generate_push_intents(frame, adjusted, gradient)
        }
    }

    /// `GeneratePushIntents` 0x824d1aa0.
    fn generate_push_intents(&mut self, frame: &mut Frame<'_, '_, '_>, desired: f32, gradient: f32) -> Result<()> {
        if matches!(
            self.current_trick_type,
            trick_type::BONELESS | trick_type::NO_COMPLY | trick_type::GRAB | trick_type::HIPPY_JUMP
        ) {
            return Ok(());
        }
        if self.stuck_so_try_off_board || self.obstruction_so_try_running_forward || self.obstruction_so_try_jumping_forward
        {
            return Ok(());
        }
        let push = frame.actor.push_direction();
        if 0.0 > vmx::dot3(vmx::v3(self.approximated_node.node.direction), push) {
            return Ok(());
        }
        let approaching_trick = self.impending_trick_node
            && self.impending_trick_node_frame_count < cvars::MAX_FRAMES_AWAY_FOR_APPROACHING_TRICK;
        let approaching_off_board = self.impending_off_board_node
            && self.impending_off_board_frame_count < cvars::MAX_FRAMES_AWAY_FOR_APPROACHING_OFF_BOARD;
        let mut fraction = if approaching_trick && self.current_speed > cvars::MIN_SPEED_FOR_APPROACHING_TRICK {
            cvars::SIGNIFICANT_SPEEDUP_FRACTION_TRICK_APPROACH
        } else {
            cvars::SIGNIFICANT_SPEEDUP_FRACTION
        };
        if approaching_off_board {
            fraction *= cvars::SIGNIFICANT_SPEEDUP_FRACTION_OFF_BOARD_APPROACH;
        }
        if gradient < cvars::STEEP_DOWN_GRADIENT {
            fraction *= cvars::SIGNIFICANT_SPEEDUP_FRACTION_STEEP_SCALAR;
        }
        if self.current_speed < fraction * desired
            && self.deviation_from_path_in_radians.abs()
                < frame.context.attributes.maximum_path_deviation_for_push * DEGREES_TO_RADIANS
        {
            let side = if frame.actor.push_right { name::RIGHT_PUSH } else { name::LEFT_PUSH };
            intents::set(frame.intents, side, 1.0)?;
            intents::set(frame.intents, name::NEW_PUSH, 1.0)?;
            intents::set(frame.intents, name::PUSHING, 1.0)?;
            self.push_intent_used_this_frame = true;
        }
        Ok(())
    }

    /// `GenerateBrakeIntents` 0x824d1df8.
    fn generate_brake_intents(&mut self, frame: &mut Frame<'_, '_, '_>, desired: f32, gradient: f32) -> Result<()> {
        if self.approximated_node.description.index < 1 {
            return Ok(());
        }
        let attributes = frame.context.attributes;
        let mut fraction = cvars::SIGNIFICANT_BRAKING_FRACTION;
        if self.impending_off_board_node
            && self.impending_off_board_frame_count < cvars::MAX_FRAMES_AWAY_FOR_APPROACHING_OFF_BOARD
        {
            fraction = cvars::SIGNIFICANT_BRAKING_FRACTION_OFF_BOARD_APPROACH * cvars::SIGNIFICANT_BRAKING_FRACTION;
        }
        if gradient > cvars::STEEP_UP_GRADIENT {
            fraction *= cvars::SIGNIFICANT_BRAKING_FRACTION_STEEP_SCALAR;
        }
        let braking = self.current_speed * fraction;
        if braking > desired
            && self.deviation_from_path_in_radians.abs()
                < attributes.maximum_path_deviation_for_brake * DEGREES_TO_RADIANS
        {
            let excess = braking - desired;
            let amount = if excess <= attributes.full_brake_threshold {
                excess / attributes.full_brake_threshold
            } else {
                1.0
            };
            let brake = if self.current_trick_type == trick_type::MANUAL { name::MANUAL_BRAKE } else { name::BRAKE };
            intents::set(frame.intents, brake, amount)?;
        }
        Ok(())
    }

    /// `GetTrickNodeCrossed` 0x824b4b08: the latest start or end node passed since last frame,
    /// later than `index`.
    fn trick_node_crossed(&self, index: &mut i32) -> Result<Option<AiPathNode>> {
        let current = self.predicted_node.description.index;
        let last = self.last_frame_predicted_node_index;
        let step = current - last;
        let plausible = step > 0 && if last > 0 { step < 4 } else { last == 0 && step < 2 };
        if !plausible || current <= last {
            *index = -1;
            return Ok(None);
        }
        let path = self.path()?;
        let mut at = current;
        while at > last {
            if at > *index {
                let node = path::node(path, at);
                if matches!(node.event_type, path::EVENT_START_TRICK | path::EVENT_END_TRICK) {
                    *index = at;
                    return Ok(Some(*node));
                }
            }
            at -= 1;
        }
        *index = -1;
        Ok(None)
    }

    /// The trick-start variants that take the node's scorable unless following a pattern.
    fn recorded_or_pattern_scorable(&mut self, influence: &Influence, tricks: &TrickTable, node: &AiPathNode) -> Result<i32> {
        match influence.trick_swap_mode {
            TrickSwapMode::DoNotSwap | TrickSwapMode::RandomSwap => Ok(path::trick_scorable(node)),
            TrickSwapMode::FollowTrickPattern => Ok(self.trick_pattern_trick_swap(influence, tricks, node)),
        }
    }

    /// `TrickPatternTrickSwap` 0x824b4c48: the pattern's next item, when the node's trick fits it.
    /// A trick type item of the current trick type takes the node's recorded trick; an exact trick
    /// item whose valid record roughly matches the current trick type
    /// (`DoTrickTypesRoughlyMatch`) takes its own scorable. Either moves the pattern on.
    fn trick_pattern_trick_swap(&mut self, influence: &Influence, tricks: &TrickTable, node: &AiPathNode) -> i32 {
        let pattern = &influence.trick_pattern;
        let next = self.trick_pattern_next_trick_index;
        if next >= pattern.length {
            return path::scorable::INVALID;
        }
        let Some(&(match_type, value)) = usize::try_from(next).ok().and_then(|index| pattern.items.get(index)) else {
            return path::scorable::INVALID;
        };
        if match_type != crate::challenge::skate::MATCH_EXACT_TRICK {
            if value == self.current_trick_type {
                self.trick_pattern_next_trick_index += 1;
                return path::trick_scorable(node);
            }
            return path::scorable::INVALID;
        }
        let Some(settings) = tricks.settings_for_index(value) else { return path::scorable::INVALID };
        if !trick_types_roughly_match(settings.trick_type, self.current_trick_type) {
            return path::scorable::INVALID;
        }
        self.trick_pattern_next_trick_index += 1;
        settings.scorable_id
    }

    /// `GenerateTrickIntents` 0x824d3198.
    fn generate_trick_intents(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let mut crossed_index = self.last_trick_index_started;
        let crossed = self.trick_node_crossed(&mut crossed_index)?;
        let (node, index) = if self.start_impending_trick && crossed.is_none() {
            let node = *path::node(self.path()?, self.impending_trick_node_index);
            (Some(node), self.impending_trick_node_index)
        } else {
            (crossed, crossed_index)
        };
        let starts = node.is_some_and(|node| node.event_type == path::EVENT_START_TRICK);
        let ends = node.is_some_and(|node| node.event_type == path::EVENT_END_TRICK);
        if let (true, Some(node)) = (starts, node) {
            self.num_frames_since_trick_node = 0;
            if self.start_impending_trick {
                self.start_impending_trick = false;
                self.impending_trick_started = true;
            } else {
                self.impending_trick_started = false;
            }
            self.last_trick_index_started = index;
            let kind = path::node_trick_type(&node);
            self.current_trick_type = kind;
            let started = match kind {
                trick_type::NONE => true,
                trick_type::AIR | trick_type::FLIP => self.start_aerial_trick(frame, &node, index)?,
                trick_type::FINGER_FLIP | trick_type::GRAB | trick_type::BONELESS => {
                    if frame.context.profile.does_data_tricks() {
                        self.start_data_trick(frame, &node)?
                    } else {
                        true
                    }
                }
                trick_type::GRIND | trick_type::FOOTPLANT => {
                    let scorable = self.recorded_or_pattern_scorable(frame.influence, frame.context.tricks, &node)?;
                    if tricks::is_valid_scorable(scorable) {
                        self.last_trick_scorable_id = scorable;
                        true
                    } else {
                        false
                    }
                }
                trick_type::HANDPLANT | trick_type::SLIDE => {
                    tricks::is_valid_scorable(self.recorded_or_pattern_scorable(frame.influence, frame.context.tricks, &node)?)
                }
                trick_type::NO_COMPLY | trick_type::HIPPY_JUMP => self.start_data_trick(frame, &node)?,
                trick_type::MANUAL => {
                    let scorable = self.recorded_or_pattern_scorable(frame.influence, frame.context.tricks, &node)?;
                    if tricks::is_valid_scorable(scorable) {
                        self.manual_trick_value = manual_intent_value(frame.context.attributes, scorable);
                        true
                    } else {
                        false
                    }
                }
                trick_type::REVERT => {
                    let scorable = self.recorded_or_pattern_scorable(frame.influence, frame.context.tricks, &node)?;
                    if tricks::is_valid_scorable(scorable) {
                        on_board::set_revert_intent(frame.intents, scorable)?;
                        true
                    } else {
                        false
                    }
                }
                _ => true,
            };
            if !started {
                self.requires_hold_pattern = false;
                self.current_trick_type = trick_type::NONE;
            }
        } else if ends {
            self.impending_trick_started = false;
            self.requires_hold_pattern = false;
            self.current_trick_type = trick_type::NONE;
        }
        self.generate_per_frame_trick_intents(frame)?;
        self.num_frames_since_trick_node += 1;
        Ok(())
    }

    /// `StartDataTrick` 0x824d3038.
    fn start_data_trick(&mut self, frame: &mut Frame<'_, '_, '_>, node: &AiPathNode) -> Result<bool> {
        let scorable = self.recorded_or_pattern_scorable(frame.influence, frame.context.tricks, node)?;
        if !tricks::is_valid_scorable(scorable) {
            return Ok(false);
        }
        self.last_trick_scorable_id = scorable;
        if self.current_trick_type == trick_type::BONELESS {
            on_board::set_trick_intents(
                frame.intents,
                frame.context.tricks,
                scorable,
                self.num_frames_since_trick_node as i32,
            )?;
        }
        Ok(true)
    }

    /// `StartAerialTrick` 0x824d1690: a variant of a base trick stays as recorded; otherwise
    /// the swap mode picks the trick.
    fn start_aerial_trick(&mut self, frame: &mut Frame<'_, '_, '_>, node: &AiPathNode, index: i32) -> Result<bool> {
        let recorded = path::trick_scorable(node);
        if tricks::is_valid_scorable(recorded) {
            let base = tricks::base_scorable(recorded);
            if base != recorded && base != path::scorable::OLLIE {
                return Ok(true);
            }
        }
        let scorable = match frame.influence.trick_swap_mode {
            TrickSwapMode::DoNotSwap => self.recorded_aerial_scorable(recorded, index)?,
            TrickSwapMode::RandomSwap => self.random_aerial_trick_swap(frame, node, index)?,
            TrickSwapMode::FollowTrickPattern => self.trick_pattern_trick_swap(frame.influence, frame.context.tricks, node),
        };
        if tricks::is_valid_scorable(scorable) {
            self.set_trick_intent(frame, scorable)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// `GetRecordedAerialScorable` 0x824b4f70: the last of a chain of start nodes each
    /// consuming the one before.
    fn recorded_aerial_scorable(&self, scorable: i32, index: i32) -> Result<i32> {
        if !tricks::is_valid_scorable(scorable) {
            return Ok(scorable);
        }
        let path = self.path()?;
        let mut current = scorable;
        let mut frames = 0;
        let mut next = path::next_event_node_index(path, index, 0.0, 120, &mut frames, &[path::EVENT_START_TRICK]);
        while next >= 0 {
            let node = path::node(path, next);
            let consumed = tricks::consumed_scorable(path::trick_scorable(node));
            if consumed != current {
                break;
            }
            current = consumed;
            next = path::next_event_node_index(path, next, 0.0, 120, &mut frames, &[path::EVENT_START_TRICK]);
        }
        Ok(current)
    }

    /// `RandomAerialTrickSwap` 0x824c68f8.
    fn random_aerial_trick_swap(&self, frame: &Frame<'_, '_, '_>, node: &AiPathNode, index: i32) -> Result<i32> {
        let recorded = path::trick_scorable(node);
        if self.is_safe_to_swap_aerial_trick(node, index)? {
            self.suitable_trick(frame, self.current_trick_type, recorded)
        } else {
            Ok(recorded)
        }
    }

    /// `IsSafeToSwapAerialTrick` 0x824c6708: no later trick within 300 frames that continues a
    /// flip sequence, nor a quick re-launch after landing.
    fn is_safe_to_swap_aerial_trick(&self, node: &AiPathNode, index: i32) -> Result<bool> {
        if cvars::ALLOW_TRICK_SWAP_ON_ALL_AERIAL_TRICKS {
            return Ok(true);
        }
        if cvars::PREVENT_TRICK_SWAP_ON_ALL_AERIAL_TRICKS {
            return Ok(false);
        }
        let path = self.path()?;
        let count = path::node_count(path);
        let recorded = path::trick_scorable(node);
        let mut frames = 0i32;
        let mut seen_air = false;
        let mut at = index + 1;
        while at < count {
            if frames >= 300 {
                return Ok(true);
            }
            let next = path::node(path, at);
            frames += i32::from(next.frame_count_since_last_node);
            if next.event_type == path::EVENT_START_TRICK {
                let scorable = path::trick_scorable(next);
                return Ok(match tricks::scorable_trick_type(scorable) {
                    trick_type::AIR | trick_type::FLIP => !is_multi_flip_trick_sequence(recorded, scorable),
                    _ => false,
                });
            }
            if seen_air && next.bit_flags & path::NODE_AIRBORNE == 0 {
                if at + 1 < count && path::node(path, at + 1).event_type == path::EVENT_START_TRICK {
                    return Ok(false);
                }
                return Ok(frames > 50);
            }
            seen_air |= next.bit_flags & path::NODE_AIRBORNE != 0;
            at += 1;
        }
        Ok(true)
    }

    /// `GetSuitableTrick` 0x824be428: an ollie or nollie trick from the profile's weights for
    /// air and flip tricks, else a random trick of the same type and substitute group.
    fn suitable_trick(&self, frame: &Frame<'_, '_, '_>, kind: i32, scorable: i32) -> Result<i32> {
        if !tricks::is_valid_scorable(scorable) {
            return Ok(scorable);
        }
        let Some(settings) = frame.context.tricks.settings(scorable) else {
            return Ok(scorable);
        };
        let group = settings.substitute_trick_group;
        let count = frame.context.tricks.trick_count(kind, group);
        if count == 0 {
            return Ok(scorable);
        }
        if kind == trick_type::AIR || kind == trick_type::FLIP {
            let profile = frame.context.profile;
            let weights = if is_nollie_scorable(scorable) { &profile.nollie_weights } else { &profile.ollie_weights };
            return Ok(weights.random_trick());
        }
        let draw = rw_random()?;
        Ok(frame.context.tricks.scorable_id(kind, draw % count, group))
    }

    /// `SetTrickIntent` 0x824d1400: `Trick`, the trick's name and `GestureSpeed`, plus height and
    /// no mirroring for air and flip tricks.
    fn set_trick_intent(&mut self, frame: &mut Frame<'_, '_, '_>, scorable: i32) -> Result<()> {
        let base = tricks::base_scorable(scorable);
        if !tricks::is_valid_scorable(base) {
            return Ok(());
        }
        let Some(settings) = frame.context.tricks.settings(base) else {
            return Ok(());
        };
        if !trick_types_roughly_match(settings.trick_type, self.current_trick_type) {
            return Ok(());
        }
        intents::set(frame.intents, name::TRICK, 1.0)?;
        intents::set(frame.intents, tricks::scorable_name(settings.scorable_id), 1.0)?;
        intents::set(frame.intents, name::GESTURE_SPEED, 1.0)?;
        if matches!(self.current_trick_type, trick_type::AIR | trick_type::FLIP) {
            intents::set(frame.intents, name::TRICK_HEIGHT, 1.0)?;
            intents::set(frame.intents, name::DONT_MIRROR_TRICK, 1.0)?;
        }
        self.requires_hold_pattern = base != scorable && base != path::scorable::OLLIE;
        self.last_trick_scorable_id = scorable;
        Ok(())
    }

    /// `SetPerFrameAerialTrickIntents` 0x824d1560: hold the pattern until the trick shows up
    /// among the active tricks.
    fn set_per_frame_aerial_trick_intents(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        if !self.requires_hold_pattern {
            return Ok(());
        }
        if frame.actor.active_trick_scorables.contains(&self.last_trick_scorable_id) {
            self.requires_hold_pattern = false;
        } else if !intents::has(frame.intents, name::TRICK) {
            intents::set(frame.intents, name::HOLD_PATTERN, 1.0)?;
        }
        Ok(())
    }

    /// `GeneratePerFrameTrickIntents` 0x824d2d08.
    fn generate_per_frame_trick_intents(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let mut anticipating = false;
        if self.impending_trick_node
            && self.impending_trick_desired_anticipation > 0.0
            && !self.push_intent_used_this_frame
            && self.deviation_from_path_in_radians.abs() < f32::from_bits(0x3f49_0fdb)
            && self.current_trick_type != trick_type::MANUAL
        {
            anticipating = true;
            intents::set(frame.intents, name::ANTIC_MAG, self.impending_trick_desired_anticipation)?;
            intents::set(frame.intents, name::ANTIC_ANGLE, self.impending_trick_desired_anticipation_angle)?;
        }
        if self.approximated_node.node.bit_flags & path::NODE_CROUCHED != 0
            && !self.push_intent_used_this_frame
            && !anticipating
            && self.current_trick_type == trick_type::NONE
        {
            intents::set(frame.intents, name::CROUCH, 1.0)?;
        }
        let recently_airborne = self.is_airborne || self.num_frames_since_airborne < 5;
        if self.impending_trick_node {
            if recently_airborne
                && self.current_trick_type != trick_type::MANUAL
                && self.impending_trick_type == trick_type::MANUAL
            {
                let value = manual_intent_value(frame.context.attributes, self.impending_trick);
                intents::set(frame.intents, name::MANUAL, value)?;
            }
            let maximum = frame.context.tricks.settings(self.impending_trick).map_or(0, |settings| settings.max_impending_trick_frames);
            if !self.impending_trick_started && maximum > 0 && self.impending_trick_node_frame_count < maximum {
                self.start_impending_trick = true;
            }
        }
        match self.current_trick_type {
            trick_type::AIR | trick_type::FLIP => self.set_per_frame_aerial_trick_intents(frame)?,
            trick_type::FINGER_FLIP
            | trick_type::GRAB
            | trick_type::GRIND
            | trick_type::NO_COMPLY
            | trick_type::HIPPY_JUMP
            | trick_type::BONELESS
            | trick_type::FOOTPLANT => on_board::set_trick_intents(
                frame.intents,
                frame.context.tricks,
                self.last_trick_scorable_id,
                self.num_frames_since_trick_node as i32,
            )?,
            trick_type::HANDPLANT => intents::set(frame.intents, name::GRAB_WORLD, 1.0)?,
            trick_type::MANUAL => intents::set(frame.intents, name::MANUAL, self.manual_trick_value)?,
            trick_type::SLIDE => {
                if self.has_joined_path {
                    on_board::set_slide_intent(frame.intents, self.deviation_from_path_in_radians)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `GenerateTurnIntents` 0x824d3880.
    fn generate_turn_intents(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let off = frame.actor.skater_off_board;
        let transitioning = frame.actor.transitioning_on_off_board;
        if off && !transitioning {
            self.generate_turn_intents_off_board(frame)
        } else if !off && !transitioning {
            if frame.actor.skateboard_airborne {
                self.generate_turn_intents_air(frame)
            } else {
                on_board::turn_on_ground(frame.intents, frame.context.attributes, self.deviation_from_path_in_radians)?;
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// `GenerateTurnIntentsAir` 0x824d3460: turn the skeleton toward the path's skater heading
    /// soon, flipped with the skater; prepare a manual or the landing's board adjustment.
    fn generate_turn_intents_air(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let mut skater = path::skater_orientation_matrix(&self.look_ahead_node_soon);
        if self.player_is_flipped {
            skater[0] = skater[0].map(|lane| -lane);
            skater[2] = skater[2].map(|lane| -lane);
        }
        let root = frame.actor.skeleton_root;
        let mut angle = signed_angle(root[2], skater[2], root[1]);
        if angle > PI {
            angle -= TWO_PI;
        }
        on_board::turn_in_air(frame.intents, frame.context.attributes, angle)?;
        if std::env::var_os("SKATE2_TRACE_AI_SPIN").is_some() {
            eprintln!(
                "ai spin: player {} air angle {angle:.3} node {} spin {} trick {} root z {:?} path z {:?}",
                frame.actor.player_id,
                self.look_ahead_node_soon.extended.is_some(),
                path::air_spin_180_count(&self.look_ahead_node_soon),
                self.last_trick_scorable_id,
                root[2],
                skater[2],
            );
        }
        if self.is_airborne
            && self.impending_trick_node
            && self.current_trick_type != trick_type::MANUAL
            && self.impending_trick_type == trick_type::MANUAL
        {
            let value = manual_intent_value(frame.context.attributes, self.impending_trick);
            let angle = if value <= 0.0 { f32::from_bits(0x3e83_9581) } else { f32::from_bits(0xc02b_53f8) };
            on_board::set_board_adjust(frame.intents, angle, value.abs())?;
        } else if self.has_landing_node_been_found && self.is_board_adjust_required {
            on_board::set_board_adjust(frame.intents, self.board_adjust_angle, self.board_adjust_magnitude)?;
        }
        Ok(())
    }

    /// `GenerateGestureIntents` 0x824d10f0: celebrate what the skater saw, then the gesture.
    fn generate_gesture_intents(&mut self, frame: &mut Frame<'_, '_, '_>, audio: &AiOutAudio) -> Result<()> {
        if self.gesture_type == GestureType::None
            && self.current_trick_type == trick_type::NONE
            && frame.context.challenge_type != challenge_type::NO_GESTURES
            && audio.words[9] & 0x0600_0000 != 0
        {
            self.trigger_gesture(frame.context.profile, GestureType::Positive, false);
        }
        match self.gesture_type {
            GestureType::None => {}
            GestureType::Positive => {
                let side = if rw_random()? & 1 != 0 { name::GESTURE_RIGHT } else { name::GESTURE_LEFT };
                intents::insert(frame.intents, side, 1.0)?;
            }
            GestureType::Negative => intents::insert(frame.intents, name::GESTURE_DOWN, 1.0)?,
        }
        self.gesture_type = GestureType::None;
        Ok(())
    }

    /// `BlendToPath` 0x824cd2a0: the trajectory ahead, and once on the path and allowed, the
    /// predicted node's transform and velocity as the physics adjustment.
    fn blend_to_path(&mut self, frame: &mut Frame<'_, '_, '_>) -> Result<()> {
        let mut input = AiPhysicsInput::cleared(self.local_world_is_loaded);
        self.add_trajectory(&mut input)?;
        let mut on_path = self.has_joined_path && !frame.actor.skateboard_airborne;
        if self.local_world_is_loaded
            && on_path
            && !self.is_within_path_width
            && self.distance_from_path > frame.context.attributes.leave_path_distance
        {
            self.leave_path();
            on_path = false;
        }
        let allowed = on_path
            && self.is_physics_fixup_allowed_yet
            && frame.actor.flags & flags::PINNED == 0
            && frame.actor.physics_state != PHYSICS_STATE_SKITCHING;
        if allowed || !self.local_world_is_loaded {
            let node = self.predicted_node;
            self.generate_physics_input(frame, &node, &mut input);
        }
        frame.commands.push(AiActorCommand::ApplyPhysicsInput(Box::new(input)));
        Ok(())
    }

    /// `AddTrajectory` 0x824c7290.
    fn add_trajectory(&self, input: &mut AiPhysicsInput) -> Result<()> {
        let Some(extended) = self.predicted_node.node.extended else {
            return Ok(());
        };
        if extended.bit_flags & path::EXTENDED_VALID_TRAJECTORY == 0 {
            return Ok(());
        }
        let start = (self.predicted_node.description.index - 1).max(0);
        input.flags |= physics_flags::VALID_TRAJECTORY;
        input.trajectory = path::trajectory(&extended);
        input.trajectory_offset = path::trajectory_offset(&self.predicted_node.node).unwrap_or([0.0; 4]);
        input.flags &= !physics_flags::TRAJECTORY_ENTERS_GRIND;
        if path::does_next_trajectory_lead_to_grind(self.path()?, start) {
            input.flags |= physics_flags::TRAJECTORY_ENTERS_GRIND;
        }
        Ok(())
    }

    /// `GeneratePhysicsInput` 0x824c9d40: the node's transform turned to the deck's heading and
    /// its velocity clamped to 99.9 and adjusted; the adjustment flags by how far off the path
    /// the skater is.
    fn generate_physics_input(&mut self, frame: &Frame<'_, '_, '_>, node: &InterpolatedNode, input: &mut AiPhysicsInput) {
        const MAX_SPEED: f32 = f32::from_bits(0x42c7_cccd);
        let mut matrix = path::node_matrix(&node.node);
        let deck_z = frame.actor.deck_transform[2];
        if 0.0 > vmx::dot3(matrix[2], deck_z) {
            matrix[0] = matrix[0].map(|lane| -lane);
            matrix[2] = matrix[2].map(|lane| -lane);
        }
        let mut velocity = vmx::scale(vmx::v3(node.node.direction), path::UPDATE_HZ);
        let squared = vmx::dot3(velocity, velocity);
        let mut speed = vmx::sqrt_guarded(squared);
        let mut adjust = true;
        if speed > MAX_SPEED {
            velocity = vmx::scale(vmx::scale(velocity, vmx::rsqrt(squared)), MAX_SPEED);
            speed = MAX_SPEED;
        } else if speed <= 0.001 {
            adjust = false;
        }
        if adjust {
            let (adjusted, out) = self.adjust_desired_speed(frame, speed);
            if adjusted {
                let out = if out > MAX_SPEED { MAX_SPEED } else { out };
                velocity = vmx::scale(velocity, out / speed);
            }
        }
        input.desired_deck_transform = [matrix[0], matrix[1], matrix[2], matrix[3]];
        input.desired_deck_velocity = velocity;
        if self.local_world_is_loaded {
            let comfortable = self.is_within_path_width
                && self.path_width_ratio_conservative.abs() < cvars::COMFORTABLY_WITHIN_PATH_WIDTH_RATIO;
            let hippy = cvars::PREVENT_PHYSICS_AI_ADJUST_DURING_HIPPY_JUMP && frame.actor.hippy_jumping;
            let steering = !hippy && !self.is_steering_to_avoid;
            let mut flags = input.flags & 0x0fff_ffff;
            if cvars::ENABLE_PHYSICS_AI_ADJUST {
                flags |= physics_flags::DO_AI_ADJUST;
            }
            if cvars::ENABLE_PHYSICS_AI_DECK_POSITION && steering && !comfortable {
                flags |= physics_flags::DO_DECK_POSITION;
            }
            if cvars::ENABLE_PHYSICS_AI_DECK_ORIENTATION
                && steering
                && (!comfortable || cvars::ENABLE_ORIENTATION_WHEN_IN_PATH)
            {
                flags |= physics_flags::DO_DECK_ORIENTATION;
            }
            if cvars::ENABLE_PHYSICS_AI_DECK_VELOCITY && steering && (!comfortable || cvars::ENABLE_VELOCITY_WHEN_IN_PATH)
            {
                flags |= physics_flags::DO_DECK_VELOCITY;
            }
            input.flags = flags;
        } else {
            if frame.context.client_type == AiClientType::Ambient && self.distance_from_path > 10.0 {
                input.desired_deck_transform = self.cached_deck_transform_for_physics(frame.actor);
                input.desired_deck_velocity = [0.0; 4];
            }
            input.flags |= 0xf000_0000;
        }
    }
}

/// `Math::AngleBetweenVectors(a, b, axis)` 0x829b0628: the angle from `a` to `b`, `2π - angle`
/// when it winds negatively about `axis`.
fn signed_angle(a: V4, b: V4, axis: V4) -> f32 {
    let mut numeric = crate::reckoning_numeric::HostReckoningNumeric;
    crate::reckoning_numeric::try_signed_angle(a, b, axis, &mut numeric).unwrap_or(0.0)
}

/// `Path::CalculateCurrentUsage` 0x824b8008: the other path-following skaters on the path
/// (words 4 and 12 of its ID) within `range` nodes.
pub(crate) fn calculate_current_usage(
    skaters: &[SkaterPathUsage],
    id: &AiPathId,
    index: i32,
    range: u32,
    ignore: i32,
) -> u32 {
    let mut count = 0;
    for skater in skaters {
        if skater.player_id == ignore {
            continue;
        }
        let Some((other, current)) = &skater.path else { continue };
        if other.word(4) == id.word(4) && other.word(12) == id.word(12) && (current - index).unsigned_abs() <= range {
            count += 1;
        }
    }
    count
}
