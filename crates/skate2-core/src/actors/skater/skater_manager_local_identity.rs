//! Selected native local identity ownership; actor roster admission is separate.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected startup fields are exercised by Actor corpus; production bootstrap remains pending"
    )
)]
use crate::simulation_assembly_backend::constraint_session::world_services::{
    PhysicalGameInfo, PhysicalWorldServicesError,
};
use std::sync::{Arc, Mutex};
#[path = "../../session/stats_backend.rs"]
pub(crate) mod stats;

/// `CommonBackEnd` static byte83040430. Debug menu construction later copies a
/// live kbEnableGhostByDefault sample; its initializer82f65404 defaults false.
pub(crate) struct LocalBackendOwner {
    game_info: Arc<PhysicalGameInfo>,
    ghost_enabled: Mutex<bool>,
    game_state: Mutex<stats::GameState>,
    statistics: Mutex<stats::StatsBackend>,
    tricks_commands: Mutex<Vec<crate::ui::trick_backend::BackendCommand>>,
}
impl LocalBackendOwner {
    pub(crate) fn construct_static_fields(game_info: Arc<PhysicalGameInfo>) -> Arc<Self> {
        Arc::new(Self {
            game_info,
            ghost_enabled: Mutex::new(false),
            game_state: Mutex::new(stats::GameState::Loading),
            statistics: Mutex::new(stats::StatsBackend::initialize()),
            tricks_commands: Mutex::new(Vec::new()),
        })
    }
    /// cDebugFEMenu827412e8: caller must fetch the live cvar first.
    pub(crate) fn try_apply_debug_menu_ghost_sample(
        &self,
        sample: bool,
    ) -> Result<(), LocalIdentityError> {
        *self
            .ghost_enabled
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)? = sample;
        Ok(())
    }
    pub(crate) fn try_game_mode(&self) -> Result<u32, LocalIdentityError> {
        self.game_info
            .try_game_mode()
            .map_err(LocalIdentityError::GameInfo)
    }
    /// `sGameInfo`'s traffic, pedestrian and security settings.
    pub(crate) fn try_free_skate(&self) -> Result<(f32, f32, bool), LocalIdentityError> {
        self.game_info
            .try_free_skate()
            .map_err(LocalIdentityError::GameInfo)
    }
    pub(crate) fn try_is_network_game(&self) -> Result<bool, LocalIdentityError> {
        self.game_info
            .try_is_network_game()
            .map_err(LocalIdentityError::GameInfo)
    }
    pub(crate) fn try_ghost_enabled(&self) -> Result<bool, LocalIdentityError> {
        Ok(*self
            .ghost_enabled
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?)
    }
    pub(crate) fn shares_game_info(&self, game_info: &Arc<PhysicalGameInfo>) -> bool {
        Arc::ptr_eq(&self.game_info, game_info)
    }
    /// `CommonBackEnd::sGameInfo`.
    pub(crate) fn game_info(&self) -> &Arc<PhysicalGameInfo> {
        &self.game_info
    }
}

#[derive(Debug)]
pub(crate) enum LocalIdentityError {
    Poisoned,
    GameInfo(PhysicalWorldServicesError),
    PendingNetworkReset,
    PendingGhostProxyConstruction,
    AlreadyReset,
    PendingRosterAdmission,
    ActorFieldsUnavailable,
}
impl std::fmt::Display for LocalIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ActorFieldsUnavailable => f.write_str("roster Actor fields are unconstructed"),
            Self::Poisoned => f.write_str("SkaterManager identity lock poisoned"),
            Self::GameInfo(error) => error.fmt(f),
            Self::PendingNetworkReset => f.write_str("network reset local identity pending"),
            Self::PendingGhostProxyConstruction => {
                f.write_str("actual ghost proxy construction pending")
            }
            Self::AlreadyReset => f.write_str("first local identity reset already completed"),
            Self::PendingRosterAdmission => {
                f.write_str("selected first reset requires pre-spawn roster")
            }
        }
    }
}

/// Opaque native Ghost owner; construction/admission remains unrecovered.
struct NativeGhostOwner;

/// No recorder allocation is admitted in selected startup. Native constructor
/// `825fb35c/825fb474` publishes null at manager+3880.
enum NativeRecorderOwner {}

struct Fields {
    local_index: i32,
    skater_list: Vec<Arc<crate::actor_construction::ActorConstruction>>,
    local: Option<Arc<crate::actor_construction::ActorConstruction>>,
    reset: bool,
    ghost: Option<Arc<NativeGhostOwner>>,
    recorder: Option<NativeRecorderOwner>,
    observers: observers::SkaterObserverState,
}
/// `SkaterManager825fb300` initializes local_index=-1 and ghost/local/roster empty.
/// This selected owner remains before `SpawnPlayer` admission. No API manufactures
/// a local Actor, ghost, observer, or recorder; complete manager construction and
/// those publications must be integrated by the caller before broader use.
pub(crate) struct SkaterManagerLocalIdentity {
    backend: Arc<LocalBackendOwner>,
    fields: Mutex<Fields>,
}
#[derive(Debug)]
pub(crate) enum LocalActorQueryError<E> {
    Manager(LocalIdentityError),
    Actor(E),
}
impl<E: std::fmt::Debug> std::fmt::Display for LocalActorQueryError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manager(error) => error.fmt(f),
            Self::Actor(error) => write!(f, "local Actor query: {error:?}"),
        }
    }
}
impl SkaterManagerLocalIdentity {
    pub(crate) fn construct_local_fields(backend: Arc<LocalBackendOwner>) -> Arc<Self> {
        Arc::new(Self {
            backend,
            fields: Mutex::new(Fields {
                local_index: -1,
                skater_list: Vec::new(),
                local: None,
                reset: false,
                ghost: None,
                recorder: None,
                observers: observers::SkaterObserverState::construct_empty(),
            }),
        })
    }
    pub(crate) fn backend(&self) -> &Arc<LocalBackendOwner> {
        &self.backend
    }

    /// GetCount/GetActor order used by CollectNearbySkatersForScoring827aea20.
    /// Clone handles before querying PhysOut to avoid holding the manager lock
    /// across unrelated Actor/physics locks.
    pub(crate) fn try_scoring_roster(&self) -> Result<Vec<Arc<crate::actor_construction::ActorConstruction>>, LocalIdentityError> {
        Ok(self.fields.lock().map_err(|_| LocalIdentityError::Poisoned)?.skater_list.clone())
    }

    /// Selected first ResetGame8273a740..a91c. Offline produces index0 itself.
    /// `SetLocalIndex826003f8` then `UpdateLocalPlayer825fff1c` sees the actual
    /// constructor-empty list and unchanged null local pointer. Later actor
    /// admission/notifications are deliberately outside this method.
    pub(crate) fn try_reset_game_before_first_spawn(&self) -> Result<(), LocalIdentityError> {
        let mut fields = self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?;
        if fields.reset {
            return Err(LocalIdentityError::AlreadyReset);
        }
        if !fields.skater_list.is_empty() || fields.local.is_some() {
            return Err(LocalIdentityError::PendingRosterAdmission);
        }
        if self
            .backend
            .game_info
            .try_is_network_game()
            .map_err(LocalIdentityError::GameInfo)?
        {
            return Err(LocalIdentityError::PendingNetworkReset);
        }
        if *self
            .backend
            .ghost_enabled
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
        {
            return Err(LocalIdentityError::PendingGhostProxyConstruction);
        }
        // Native SetLocalIndex samples the same global again before its store.
        if *self
            .backend
            .ghost_enabled
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
        {
            return Err(LocalIdentityError::PendingGhostProxyConstruction);
        }
        fields.local_index = 0;
        // FindByIndex825f19c0 on this owned, constructor-empty roster.
        fields.local = None;
        fields.reset = true;
        Ok(())
    }
    /// `FindByIndex825f19c0` scans this manager's actual retained roster in order.
    pub(crate) fn try_find_by_index(
        &self,
        index: i32,
    ) -> Result<Option<Arc<crate::actor_construction::ActorConstruction>>, LocalIdentityError> {
        let fields = self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?;
        for actor in &fields.skater_list {
            let prefix = actor.prefix().map_err(|_| LocalIdentityError::Poisoned)?;
            let player = prefix
                .fields()
                .ok_or(LocalIdentityError::ActorFieldsUnavailable)?
                .player_index;
            if i32::from_ne_bytes(player.to_ne_bytes()) == index {
                return Ok(Some(Arc::clone(actor)));
            }
        }
        Ok(None)
    }
    /// `GetLocalIndex824859b0`, reads native +174.
    pub(crate) fn try_local_index(&self) -> Result<i32, LocalIdentityError> {
        Ok(self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
            .local_index)
    }
    /// `GetLocalSimIndex825eddb8`: no ghost has been admitted in this selected owner.
    pub(crate) fn try_local_sim_index(&self) -> Result<i32, LocalIdentityError> {
        self.try_local_index()
    }
    /// Actor.IsLocalPlayer825f0c60 gets manager index BEFORE Actor.GetPlayerID.
    pub(crate) fn try_is_local_player<E>(
        &self,
        actor_id: impl FnOnce() -> Result<i32, E>,
    ) -> Result<bool, LocalActorQueryError<E>> {
        let index = self
            .try_local_index()
            .map_err(LocalActorQueryError::Manager)?;
        Ok(index == actor_id().map_err(LocalActorQueryError::Actor)?)
    }
    /// Actor.IsLocalSimPlayer825f0d00 gets Actor.GetPlayerID BEFORE manager index.
    pub(crate) fn try_is_local_sim_player<E>(
        &self,
        actor_id: impl FnOnce() -> Result<i32, E>,
    ) -> Result<bool, LocalActorQueryError<E>> {
        let player = actor_id().map_err(LocalActorQueryError::Actor)?;
        Ok(player
            == self
                .try_local_sim_index()
                .map_err(LocalActorQueryError::Manager)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_ctor_then_offline_reset_produces_identity() {
        let game = PhysicalGameInfo::construct_network_field();
        let backend = LocalBackendOwner::construct_static_fields(Arc::clone(&game));
        assert!(backend.shares_game_info(&game));
        let manager = SkaterManagerLocalIdentity::construct_local_fields(backend);
        assert_eq!(manager.try_local_index().unwrap(), -1);
        manager.try_reset_game_before_first_spawn().unwrap();
        assert_eq!(manager.try_local_index().unwrap(), 0);
        assert_eq!(manager.try_local_sim_index().unwrap(), 0);
        assert!(matches!(
            manager.try_reset_game_before_first_spawn(),
            Err(LocalIdentityError::AlreadyReset)
        ));
    }
    #[test]
    fn live_ghost_sample_requires_actual_proxy_before_index_write() {
        let backend =
            LocalBackendOwner::construct_static_fields(PhysicalGameInfo::construct_network_field());
        backend.try_apply_debug_menu_ghost_sample(true).unwrap();
        let manager = SkaterManagerLocalIdentity::construct_local_fields(backend);
        assert!(matches!(
            manager.try_reset_game_before_first_spawn(),
            Err(LocalIdentityError::PendingGhostProxyConstruction)
        ));
        assert_eq!(manager.try_local_index().unwrap(), -1);
    }
}

impl SkaterManagerLocalIdentity {
    /// Native ctor825fb454 null; `GetGhost` reads this owner, not backend enablement.
    pub(crate) fn try_ghost_is_null(&self) -> Result<bool, LocalIdentityError> {
        Ok(self
            .fields
            .lock()
            .map_err(|_| LocalIdentityError::Poisoned)?
            .ghost
            .is_none())
    }
}

#[path = "skater_manager_roster.rs"]
pub(crate) mod roster;

#[path = "skater_manager_observers.rs"]
pub(crate) mod observers;

impl LocalBackendOwner {
    pub(crate) fn send_tricks_command(&self,command:crate::ui::trick_backend::BackendCommand)->Result<(),String>{
        self.tricks_commands.lock().map_err(|_|"TricksBackend command lock")?.push(command);Ok(())
    }
    pub(crate) fn take_tricks_commands(&self)->Result<Vec<crate::ui::trick_backend::BackendCommand>,String>{
        Ok(std::mem::take(&mut *self.tricks_commands.lock().map_err(|_|"TricksBackend command lock")?))
    }
}
