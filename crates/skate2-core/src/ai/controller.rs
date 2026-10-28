//! `AI::AIController` (0xa8 bytes, `AIController::AIController` 0x824cedc8): one per AI Actor.
//! Its behaviour pushes moves; the top move feeds the shared `PathFollowing::Influence` and
//! hands the frame's intents to the active motion controller, the path controller (the navmesh
//! swap is disabled by `kbEnableNavMeshControllerSwap`).
use super::{
    actor::{AiActorCommand, AiActorView, AiClientType},
    obstacle_avoider::AiOutAudio,
    path_controller::{
        BranchMode, GestureType, Influence, InfluenceFilter, PathController, PathControllerContext,
        PathControllerError, SharedFilter, TrickSwapMode,
    },
    path_id::PathFilter,
    path_manager::PathSkaterAbilities,
    profile::SkaterProfile,
};
use crate::actor_graph_construction::ActorGraphIntents;
use crate::challenge::skate::TrickPattern;
use skate2_formats::ai_path::AiPathId;

/// `AI::FollowPathMove` (0x178, `FollowPathMove::FollowPathMove` 0x824b5630).
#[derive(Clone, Debug, PartialEq)]
struct FollowPathMove {
    /// `Move::mHasBegun`.
    has_begun: bool,
    filter: InfluenceFilter,
    /// `mTrickPattern`.
    trick_pattern: TrickPattern,
    branch_mode: BranchMode,
    trick_swap_mode: TrickSwapMode,
    speed_adjustment: f32,
    ambition_level: f32,
}

impl FollowPathMove {
    fn new() -> Self {
        Self {
            has_begun: false,
            filter: InfluenceFilter::None,
            trick_pattern: TrickPattern::default(),
            branch_mode: BranchMode::BranchRandom,
            trick_swap_mode: TrickSwapMode::RandomSwap,
            speed_adjustment: 0.0,
            ambition_level: 0.5,
        }
    }

    /// `UpdateActiveController` 0x824b5730: the move's settings into the influence. The invalid
    /// trick patterns it also copies have no setter but `Skate.StartSkatePath`'s clearing.
    fn update_active_controller(&self, influence: &mut Influence) {
        influence.skater_speed_adjustment = self.speed_adjustment;
        influence.branch_mode = self.branch_mode;
        if self.trick_swap_mode != influence.trick_swap_mode {
            influence.trick_swap_mode = self.trick_swap_mode;
        }
        influence.trick_pattern = self.trick_pattern;
        influence.filter = self.filter.clone();
        influence.ambition_level = self.ambition_level;
    }
}

/// `AI::Behavior`s.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Behavior {
    /// `WanderBehavior`: ambient path following.
    Wander,
    /// `FollowPathBehavior` (0x1c): a challenge's paths through its own `mSingleFilter`.
    FollowPath { single_filter: SharedFilter },
}

/// `AI::AIController`.
#[derive(Debug)]
pub(crate) struct AiController {
    pub(crate) skater_profile_name: String,
    pub(crate) recipe_id: u64,
    pub(crate) client_type: AiClientType,
    pub(crate) challenge_round: i32,
    pub(crate) local_world_loaded: bool,
    pub(crate) influence: Influence,
    pub(crate) path_controller: PathController,
    pub(crate) profile: SkaterProfile,
    pub(crate) abilities: PathSkaterAbilities,
    pub(crate) audio: AiOutAudio,
    moves: Vec<FollowPathMove>,
    behavior: Option<Behavior>,
}

impl AiController {
    /// `AIController::AIController`: a fresh influence and controllers, the path controller
    /// active, and the named profile (`default` without one).
    pub(crate) fn new(
        client_type: AiClientType,
        skater_profile_name: &str,
        recipe_id: u64,
        profile: SkaterProfile,
        abilities: PathSkaterAbilities,
    ) -> Self {
        let mut path_controller = PathController::new();
        path_controller.is_active = true;
        Self {
            skater_profile_name: skater_profile_name.to_owned(),
            recipe_id,
            client_type,
            challenge_round: -1,
            local_world_loaded: false,
            influence: Influence::default(),
            path_controller,
            profile,
            abilities,
            audio: AiOutAudio::default(),
            moves: Vec::new(),
            behavior: None,
        }
    }

    pub(crate) const fn behavior(&self) -> Option<&Behavior> {
        self.behavior.as_ref()
    }

    /// `SetBehavior<WanderBehavior>` 0x824b5cc0 through `SetBehavior` 0x824b3d68: the old
    /// behaviour ends, then `WanderBehavior::OnBegin` 0x824d1350 clears the moves and pushes a
    /// `FollowPathMove` on the ambient filter.
    pub(crate) fn set_wander_behavior(&mut self) {
        if self.behavior.take().is_some() {
            self.pop_move();
        }
        self.behavior = Some(Behavior::Wander);
        self.moves.clear();
        let mut wander = FollowPathMove::new();
        wander.branch_mode = BranchMode::BranchRandom;
        wander.filter = InfluenceFilter::Ambient;
        self.moves.push(wander);
    }

    /// `SetBehavior<FollowPathBehavior>` 0x824c0228: its `SingleFilter` copies the path
    /// manager's universal filter ID; the old behaviour ends (`OnEnd` 0x824c60c0 pops its move),
    /// then `FollowPathBehavior::OnBegin` 0x824d12f0 clears the moves and pushes a fresh
    /// `FollowPathMove`.
    pub(crate) fn set_follow_path_behavior(&mut self, universal_filter_id: AiPathId) {
        if self.behavior.take().is_some() {
            self.pop_move();
        }
        self.behavior = Some(Behavior::FollowPath { single_filter: SharedFilter::new(PathFilter::Single(universal_filter_id)) });
        self.moves.clear();
        self.moves.push(FollowPathMove::new());
    }

    /// `ClearBehavior` 0x824b3e00: the behaviour ends and goes.
    pub(crate) fn clear_behavior(&mut self) {
        if self.behavior.take().is_some() {
            self.pop_move();
        }
    }

    /// `FollowPathBehavior::SetPathID` 0x824bd8e8: the ID into the behaviour's single filter,
    /// which becomes the move's filter.
    pub(crate) fn set_path_id(&mut self, id: AiPathId) {
        let Some(Behavior::FollowPath { single_filter }) = &self.behavior else { return };
        single_filter.set(PathFilter::Single(id));
        let filter = InfluenceFilter::Shared(single_filter.clone());
        self.set_filter(filter);
    }

    /// `FollowPathBehavior::SetFilter` 0x824bd868: the top move's `mFilter`.
    pub(crate) fn set_filter(&mut self, filter: InfluenceFilter) {
        if let Some(top) = self.moves.last_mut() {
            top.filter = filter;
        }
    }

    /// `FollowPathBehavior::SetBranchMode` 0x824bd650.
    pub(crate) fn set_branch_mode(&mut self, mode: BranchMode) {
        if let Some(top) = self.moves.last_mut() {
            top.branch_mode = mode;
        }
    }

    /// `FollowPathBehavior::SetTrickSwapMode` 0x824bd6d0.
    pub(crate) fn set_trick_swap_mode(&mut self, mode: TrickSwapMode) {
        if let Some(top) = self.moves.last_mut() {
            top.trick_swap_mode = mode;
        }
    }

    /// `FollowPathBehavior::SetTrickPattern` 0x824bd750: the top move's `mTrickPattern`.
    pub(crate) fn set_trick_pattern(&mut self, pattern: TrickPattern) {
        if let Some(top) = self.moves.last_mut() {
            top.trick_pattern = pattern;
        }
    }

    /// `SetChallengeRound` 0x828830b0.
    pub(crate) fn set_challenge_round(&mut self, round: i32) {
        self.challenge_round = round;
    }

    /// `TriggerGesture` 0x824b3eb0: the active controller's, not guaranteed.
    pub(crate) fn trigger_gesture(&mut self, gesture: GestureType) {
        self.path_controller.trigger_gesture(&self.profile, gesture, false);
    }

    /// `SwitchOffFollowMeRubberBanding` 0x824bdb20.
    pub(crate) fn switch_off_follow_me_rubber_banding(&mut self) {
        self.path_controller.switch_off_follow_me_rubber_banding();
    }

    /// `PopMove` 0x824bda08; the move's `OnEnd` does nothing.
    fn pop_move(&mut self) {
        self.moves.pop();
    }

    /// `UpdateIsLocalWorldLoaded` 0x824b3cf8: ground below the hips means the world around
    /// the skater is loaded; losing it drops the cached deck transform.
    fn update_is_local_world_loaded(&mut self, actor: &AiActorView) {
        let loaded = actor.below_hips_ground_valid;
        self.local_world_loaded = loaded;
        let was = self.path_controller.local_world_is_loaded;
        self.path_controller.local_world_is_loaded = loaded;
        if was && !loaded {
            self.path_controller.has_cached_deck_transform_for_physics = false;
        }
    }

    /// `AIController::Update` 0x824c8be0, the first thing `Actor::Update` does: the local
    /// world, the behaviour (Wander's update does nothing) and `UpdateMove` 0x824c60c8: begin
    /// the top move (`Move::Begin` 0x824bd570 resets the active controller) and update it.
    pub(crate) fn update(&mut self, actor: &AiActorView) {
        self.update_is_local_world_loaded(actor);
        let Some(top) = self.moves.last_mut() else { return };
        if !top.has_begun {
            self.path_controller.external_reset();
            top.update_active_controller(&mut self.influence);
            top.has_begun = true;
        }
        top.update_active_controller(&mut self.influence);
    }

    /// `AIController::GenerateActionGraphIntents` 0x824bd950: a begun top move hands the frame
    /// to the path controller (`FollowPathMove::OnUpdateActionGraphIntent` 0x824b5718).
    pub(crate) fn generate_action_graph_intents(
        &mut self,
        context: &mut PathControllerContext<'_>,
        actor: &mut AiActorView,
        intents: &mut ActorGraphIntents,
        commands: &mut Vec<AiActorCommand>,
    ) -> Result<(), PathControllerError> {
        if !self.moves.last().is_some_and(|top| top.has_begun) {
            return Ok(());
        }
        self.path_controller.generate_action_graph_intents(
            context,
            &self.influence,
            actor,
            intents,
            &mut self.audio,
            commands,
        )
    }

    /// `AISkaterManager::PreUpdate` 0x824c80e0's reset of the controller's `AIOut_Audio`.
    pub(crate) fn clear_audio(&mut self) {
        let words = &mut self.audio.words;
        words[0] &= 0x0001_ffff;
        words[1..6].fill(u32::MAX);
        words[6..9].fill(0);
        words[9] &= 0x01ff_ffff;
    }

    /// `AIController::ExternalReset` 0x824b3e88.
    pub(crate) fn external_reset(&mut self) {
        self.path_controller.external_reset();
    }
}
