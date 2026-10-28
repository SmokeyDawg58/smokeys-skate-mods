//! Live player selection for the existing Actor animation constructor.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "mounted Actor bootstrap integration is exercised by original corpus"
    )
)]
use crate::{
    actor_animation_construction::{
        ActorAnimationCvars, initialization::ActorAnimationInitializeSource,
    },
    actor_construction::{
        animation_mount::ActorAnimationSelection, animation_start::ActorAnimationStartSource,
    },
    actor_face_playback::{
        ActorFaceError, FaceCommandManager, FaceDatabaseManager, FacePlaybackManager,
    },
    animation_job_owners::AnimationJobManager,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    player::attributes_runtime::PlayerAttributeServices,
    players_backend_settings::PlayersSettingsError,
    simulation_animation_startup::SimulationAnimationStartup,
    skater_manager_local_identity::LocalIdentityError,
};
use std::sync::{Arc, Mutex};

/// All selection getters reach the SAME settings/local manager used by
/// `ApplyPlayerAttributes`; manager getters clone existing global startup owners.
pub(crate) struct ActorAnimationPlayerServices<'a> {
    pub(crate) players: &'a PlayerAttributeServices,
    pub(crate) startup: &'a SimulationAnimationStartup,
    pub(crate) cvars: &'a InputCvarReferences,
    jobs: Option<Arc<AnimationJobManager>>,
}
impl<'a> ActorAnimationPlayerServices<'a> {
    /// Bind the actual job capability before entering any buffered command lock.
    /// Resolving it inside `GetBindPose` would reacquire that same buffered owner.
    pub(crate) fn try_bind(
        players: &'a PlayerAttributeServices,
        startup: &'a SimulationAnimationStartup,
        cvars: &'a InputCvarReferences,
    ) -> Result<Self, PlayerAnimationError> {
        let jobs = startup
            .retained_job_manager()
            .map_err(PlayerAnimationError::Animation)?;
        Ok(Self {
            players,
            startup,
            cvars,
            jobs,
        })
    }
}
#[derive(Debug)]
pub(crate) enum PlayerAnimationError {
    Settings(PlayersSettingsError),
    Identity(LocalIdentityError),
    Cvar(InputCvarError),
    Animation(ActorFaceError),
    MissingJobManager,
}
impl std::fmt::Display for PlayerAnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Settings(e) => write!(f, "animation player settings: {e}"),
            Self::Identity(e) => write!(f, "animation player identity: {e}"),
            Self::Cvar(e) => write!(f, "animation constructor cvar: {e:?}"),
            Self::Animation(e) => write!(f, "animation startup owner: {e:?}"),
            Self::MissingJobManager => f.write_str("animation startup job manager is absent"),
        }
    }
}
impl ActorAnimationSelection for ActorAnimationPlayerServices<'_> {
    type Error = PlayerAnimationError;
    fn local_player_index(&mut self) -> Result<u32, Self::Error> {
        self.players
            .skaters
            .try_local_index()
            .map(|value| u32::from_ne_bytes(value.to_ne_bytes()))
            .map_err(PlayerAnimationError::Identity)
    }
    fn is_network_game(&mut self) -> Result<bool, Self::Error> {
        self.players
            .skaters
            .backend()
            .try_is_network_game()
            .map_err(PlayerAnimationError::Identity)
    }
    fn ghost_system_enabled(&mut self) -> Result<bool, Self::Error> {
        self.players
            .skaters
            .backend()
            .try_ghost_enabled()
            .map_err(PlayerAnimationError::Identity)
    }
    fn marquee_id(&mut self, player: u32) -> Result<u64, Self::Error> {
        self.players
            .settings
            .try_character(player)
            .map_err(PlayerAnimationError::Settings)
    }
    fn character_is_male(&mut self, player: u32) -> Result<bool, Self::Error> {
        self.players
            .settings
            .try_character_is_male(player)
            .map_err(PlayerAnimationError::Settings)
    }
    fn command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error> {
        Ok(Arc::clone(&self.startup.managers.command))
    }
    fn playback_manager(&mut self) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error> {
        Ok(Arc::clone(&self.startup.managers.playback))
    }
    fn database_manager(&mut self) -> Result<Arc<Mutex<FaceDatabaseManager>>, Self::Error> {
        Ok(Arc::clone(&self.startup.managers.database))
    }
}
impl ActorAnimationInitializeSource for ActorAnimationPlayerServices<'_> {
    type Error = PlayerAnimationError;
    fn read_use_jobs_for_skater_anim_update(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_use_jobs_for_skater_anim_update()
            .map_err(PlayerAnimationError::Cvar)
    }
    fn require_job_manager(&mut self) -> Result<Arc<AnimationJobManager>, Self::Error> {
        self.jobs
            .as_ref()
            .map(Arc::clone)
            .ok_or(PlayerAnimationError::MissingJobManager)
    }
}
impl ActorAnimationStartSource for ActorAnimationPlayerServices<'_> {
    fn is_stance_regular(&mut self, player: u32) -> Result<bool, Self::Error> {
        self.players
            .settings
            .try_stance_byte(player)
            .map(|value| value != 0)
            .map_err(PlayerAnimationError::Settings)
    }
    fn skater_id(&mut self, player: u32) -> Result<u64, Self::Error> {
        self.players
            .settings
            .try_character(player)
            .map_err(PlayerAnimationError::Settings)
    }
    fn player_gestures(&mut self, player: u32) -> Result<[u32; 4], Self::Error> {
        self.players
            .settings
            .try_gestures(player)
            .map_err(PlayerAnimationError::Settings)
    }
    fn player_style(&mut self, player: u32) -> Result<u32, Self::Error> {
        self.players
            .settings
            .try_style(player)
            .map_err(PlayerAnimationError::Settings)
    }
    fn command_manager(&mut self) -> Result<Arc<Mutex<FaceCommandManager>>, Self::Error> {
        Ok(Arc::clone(&self.startup.managers.command))
    }
    fn playback_manager(&mut self) -> Result<Arc<Mutex<FacePlaybackManager>>, Self::Error> {
        Ok(Arc::clone(&self.startup.managers.playback))
    }
}
impl ActorAnimationCvars for ActorAnimationPlayerServices<'_> {
    type Error = PlayerAnimationError;
    fn use_sprint_button(&mut self) -> Result<u8, Self::Error> {
        use crate::action_graph_input_listener::fill::{ActionInputBoolCvar, ActionInputFillCvars};
        let mut provider = self.cvars.fill_provider();
        provider
            .capture_boolean(ActionInputBoolCvar::UseSprintButtonInput)
            .map_err(PlayerAnimationError::Cvar)?;
        provider
            .boolean(ActionInputBoolCvar::UseSprintButtonInput)
            .map(u8::from)
            .map_err(PlayerAnimationError::Cvar)
    }
}

#[cfg(test)]
#[path = "actor_animation_player_services_tests.rs"]
mod tests;

/// Constructor samples from Actor825ff278..2a8, retained through physical allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PhysicalConstructorFlags {
    pub(crate) bouncy_ragdoll: u8,
    pub(crate) is_human: u8,
}
impl ActorAnimationPlayerServices<'_> {
    /// `IsSkaterBouncy825ed010`, then `GetPlayerType8255aa60` before index8 exception.
    pub(crate) fn try_physical_constructor_flags(
        &mut self,
        player: u32,
    ) -> Result<PhysicalConstructorFlags, PlayerAnimationError> {
        let bouncy = is_skater_bouncy(self.players, self.cvars, player)?;
        let player_type = self
            .players
            .settings
            .try_player_type(player)
            .map_err(PlayerAnimationError::Settings)?;
        Ok(PhysicalConstructorFlags {
            bouncy_ragdoll: u8::from(bouncy),
            is_human: u8::from(player_type == 1 || player == 8),
        })
    }
}

/// Actor::IsSkaterBouncy825ed010, including the full 64-bit comparison at
/// 825ed098..ac. Sampled at spawn, not on every frame or every bail.
pub(crate) fn is_skater_bouncy(
    players: &PlayerAttributeServices,
    cvars: &InputCvarReferences,
    player: u32,
) -> Result<bool, PlayerAnimationError> {
    Ok(cvars
        .try_bouncy_big_black_enabled()
        .map_err(PlayerAnimationError::Cvar)?
        && !players
            .skaters
            .backend()
            .try_is_network_game()
            .map_err(PlayerAnimationError::Identity)?
        && !players
            .skaters
            .backend()
            .try_ghost_enabled()
            .map_err(PlayerAnimationError::Identity)?
        && players.settings.try_character(player).map_err(PlayerAnimationError::Settings)?
            == 0x0376_a7b6_e3aa_dc5c)
}
