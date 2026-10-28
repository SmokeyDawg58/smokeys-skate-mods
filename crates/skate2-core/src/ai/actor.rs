//! What an AI controller reads from and writes to its `Sk8::Actor` in a frame. The reads are
//! the Actor's `PhysOut` and transforms as the last physics step published them; the writes are
//! the Actor calls the AI makes, applied by the session in order after the AI step.
use super::vmx::V4;
use crate::physics_state_ids::PHYSICS_CATEGORY_OFFBOARD;

/// `AIClientType`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AiClientType {
    #[default]
    Ambient,
    Challenge,
    Test,
    TestScenario,
    Player,
}

impl AiClientType {
    /// `PlayerTypeFromClientType` 0x824b2c48: `BE::PlayerType`.
    pub(crate) const fn player_type(self) -> u32 {
        match self {
            Self::Ambient => 2,
            Self::Challenge => 3,
            Self::Test | Self::TestScenario => 4,
            Self::Player => 1,
        }
    }
}

/// `Actor+0x258` bits the AI reads or clears.
pub(crate) mod flags {
    /// Set by `SkaterManager::OnCommand` case 7: the input fill is skipped and `ForceBrake`
    /// requested.
    pub(crate) const PINNED: u32 = 0x0400_0000;
    /// An external reset (teleport) this frame.
    pub(crate) const EXTERNAL_RESET: u32 = 0x2000_0000;
    /// `ApplyAIPhysicsInput`'s mark.
    pub(crate) const AI_INPUT: u32 = 0x0800_0000;
}

/// The Actor and its `PhysOut` as the AI reads them.
#[derive(Clone, Debug, Default)]
pub(crate) struct AiActorView {
    pub(crate) player_id: i32,
    /// The teleport matrix's position (`Actor+0x21c`) while the external-reset flag waits:
    /// `GetClosestObstacleDistanceSquared` 0x824becb8 reads it for the Actor's position.
    pub(crate) teleport_destination: Option<V4>,
    /// `Actor+0x258`.
    pub(crate) flags: u32,
    /// `PhysOut_State`.
    pub(crate) physics_state_category: i32,
    pub(crate) physics_state: i32,
    pub(crate) is_wiping_out: bool,
    pub(crate) is_teleport_finished: bool,
    pub(crate) collide_with_world_only: bool,
    /// `PhysOut_Audio::mJustWipedOut`, `mJustRespawned`, and
    /// `PhysOut_Scoring2::mBankedPointChangeThisFrame`.
    pub(crate) just_wiped_out: bool,
    pub(crate) just_respawned: bool,
    pub(crate) banked_point_change_this_frame: i32,
    /// `PhysOut_Animation`.
    pub(crate) riding_fakie: bool,
    pub(crate) transitioning_on_off_board: bool,
    pub(crate) skater_off_board: bool,
    pub(crate) running_out: bool,
    /// `PhysOut_Air::mIsSkateboardAirborne`.
    pub(crate) skateboard_airborne: bool,
    /// `PhysOut_OffBoard`.
    pub(crate) mounting: bool,
    pub(crate) unmounting: bool,
    /// `PhysOut_Ground::mBelowHipsGroundDataIsValid`.
    pub(crate) below_hips_ground_valid: bool,
    /// `PhysOut_Skeleton`: `mEffectiveAnimZ`, `mSkeletonRootMatrix`, the wipeout teleport.
    pub(crate) effective_anim_z: V4,
    pub(crate) skeleton_root: [V4; 4],
    /// `mSkeletonHipsPos`, `mSkeletonHipsVel` and `mSkeletonHipsLook`.
    pub(crate) hips_position: V4,
    pub(crate) hips_velocity: V4,
    pub(crate) hips_look: V4,
    pub(crate) time_until_wipeout_teleport: f32,
    pub(crate) wipeout_teleport_pending: bool,
    /// `PhysOut_SkateboardReckoning::mDeckPosition`.
    pub(crate) deck_position: V4,
    /// `PhysOut_Scoring2`.
    pub(crate) hippy_jumping: bool,
    pub(crate) in_state_air: bool,
    pub(crate) air_spin_180_count: i32,
    /// The scorable of each active trick whose settings are valid, else -1.
    pub(crate) active_trick_scorables: Vec<i32>,
    /// `Actor::GetEffectiveDeckTransform`, `GetDeckTransform`, `GetTransform`, `GetVelocity`.
    pub(crate) effective_deck_transform: [V4; 4],
    pub(crate) deck_transform: [V4; 4],
    pub(crate) transform: [V4; 4],
    pub(crate) velocity: V4,
    /// `ISkaterAnim`'s stance comparison `GeneratePushIntents` makes: true pushes right.
    pub(crate) push_right: bool,
    /// `Actor::GetFadeOutTime`.
    pub(crate) fade_out_time: f32,
}

/// `Actor::CanSeeSkater` 0x825ed758: skeleton roots under 15 m apart.
pub(crate) fn can_see_skater(from: V4, to: V4) -> bool {
    let offset = super::vmx::sub(from, to);
    225.0 > super::vmx::dot3(offset, offset)
}

impl AiActorView {
    /// `AI::GetActorsPathRelativePosition` 0x824b47d8: the skeleton root off the board or
    /// wiping out, else the deck.
    pub(crate) fn path_relative_position(&self) -> V4 {
        if self.physics_state_category == PHYSICS_CATEGORY_OFFBOARD || self.is_wiping_out {
            self.skeleton_root[3]
        } else {
            self.deck_position
        }
    }

    /// `AI::GetPushDirection` 0x824b48b0: the effective deck's Z axis, reversed riding fakie.
    pub(crate) fn push_direction(&self) -> V4 {
        let z = self.effective_deck_transform[2];
        if self.riding_fakie { z.map(|lane| -lane) } else { z }
    }
}

/// `Physics::AIPhysicsInput`, with its flag word's high bits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AiPhysicsInput {
    pub(crate) desired_deck_transform: [V4; 4],
    pub(crate) desired_deck_velocity: V4,
    pub(crate) trajectory: super::path::Trajectory,
    pub(crate) trajectory_offset: V4,
    pub(crate) flags: u32,
}

/// The flag word's bits, highest first.
pub(crate) mod physics_flags {
    pub(crate) const DO_AI_ADJUST: u32 = 0x8000_0000;
    pub(crate) const DO_DECK_POSITION: u32 = 0x4000_0000;
    pub(crate) const DO_DECK_ORIENTATION: u32 = 0x2000_0000;
    pub(crate) const DO_DECK_VELOCITY: u32 = 0x1000_0000;
    pub(crate) const TRAJECTORY_ENTERS_GRIND: u32 = 0x0800_0000;
    pub(crate) const VALID_TRAJECTORY: u32 = 0x0400_0000;
    pub(crate) const LOCAL_WORLD_LOADED: u32 = 0x0200_0000;
}

impl AiPhysicsInput {
    /// The zeroed input `BlendToPath` and the settling path build: zero vectors, trajectory
    /// time -1, and the local-world bit.
    pub(crate) fn cleared(local_world_loaded: bool) -> Self {
        Self {
            desired_deck_transform: [[0.0; 4]; 4],
            desired_deck_velocity: [0.0; 4],
            trajectory: super::path::Trajectory { p0: [0.0; 4], v0: [0.0; 4], g: [0.0; 4], t: [-1.0; 4] },
            trajectory_offset: [0.0; 4],
            flags: if local_world_loaded { physics_flags::LOCAL_WORLD_LOADED } else { 0 },
        }
    }
}

/// An Actor call the AI made, in call order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AiActorCommand {
    /// `Actor::Teleport(matrix, true)`.
    Teleport([V4; 4]),
    /// `Actor::SetVelocityUnsafe`.
    SetVelocity(V4),
    /// `Actor::ApplyAIPhysicsInput`: the input, and `Actor+0x258 |= 0x08000000`.
    ApplyPhysicsInput(Box<AiPhysicsInput>),
    /// `Actor::SetFadingOut`.
    SetFadingOut(bool),
    /// `Actor::DisableNonWorldCollisionsForFadeout`.
    DisableNonWorldCollisionsForFadeout,
    /// The ambient pin clear at the top of `PathController::GenerateActionGraphIntents`.
    ClearPin,
}
