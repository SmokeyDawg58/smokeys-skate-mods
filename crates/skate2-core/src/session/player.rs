//! Native local-player selection and asynchronous load protocol.
//!
//! This is a readable reconstruction of the narrow `PlayerLoaderImpl` path
//! used before offline gameplay. Asset decoding and skater simulation live on
//! the other side of the commands emitted here; no console execution model is
//! represented by this module.

use std::fmt;

use skate2_formats::hash::rw_hash64_string;

/// Marquee selected when no valid profile-created character is available.
pub const DEFAULT_MARQUEE_ID: u64 = 0x2275_B900_5016_6969;
/// Host first-test spawn: an authored `BAM_Sim.loc` location name, not the
/// original `SpawnManager` choice. The initial Sim collection and the visual
/// seed are both derived from this one location.
pub const SELECTED_SPAWN_LOCATION: &str = "F_WF_Waterfront";
/// Optional host diagnostic selection from the SAME authored location table.
/// Does not alter SpawnManager, physics, or the default controller-test spawn.
#[must_use]
pub fn selected_spawn_location() -> &'static str {
    static LOCATION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    LOCATION.get_or_init(|| {
        std::env::var("SKATE2_DIAGNOSTIC_SPAWN_LOCATION")
            .unwrap_or_else(|_| SELECTED_SPAWN_LOCATION.to_owned())
    })
}
pub const LOCAL_PLAYER_INDEX: u32 = 0;
pub const NO_PEER_ID: i64 = -1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerSelection {
    /// Constructor state before the front end chooses a playable character.
    InitialState,
    Marquee {
        definition_id: u64,
        peer_id: i64,
    },
    CreateACharacter {
        peer_id: i64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarqueeDefinition {
    pub id: u64,
    /// Recipe/schema name stored at layout offset zero in
    /// `Attrib::Gen::characters_marquee`.
    pub schema_name: String,
}

impl MarqueeDefinition {
    #[must_use]
    pub fn schema_asset_id(&self) -> u64 {
        rw_hash64_string(&self.schema_name.to_ascii_lowercase())
    }

    #[must_use]
    pub fn binary_recipe_path(&self) -> String {
        format!(
            "data\\content\\recipe\\marquee\\{}.recipe",
            self.schema_name
        )
    }

    #[must_use]
    pub fn xml_recipe_path(&self) -> String {
        format!("data\\content\\recipe\\marquee\\{}.xml", self.schema_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterLoadCategory {
    Marquee,
    CreateACharacter,
}

/// Native equivalent of the useful fields in
/// `cMsgPresCharacterLoadRequest`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterLoadRequest {
    pub request_id: u32,
    pub category: CharacterLoadCategory,
    pub model_name: String,
    pub is_local_player: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CharacterRegistrationRequest {
    pub request_id: u32,
    pub character_id: u64,
    pub peer_id: i64,
    pub player_index: u32,
    pub ghost_system_enabled: bool,
    pub apply_attributes: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerLoadAction {
    Load(CharacterLoadRequest),
    Register(CharacterRegistrationRequest),
    /// The asset is retained; native `ApplyPlayerAttributes` must finish before Register.
    ApplyAttributes {
        character_id: u64,
        player_index: u32,
    },
    /// A completed load whose request is no longer current is immediately
    /// unloaded by the reference callback.
    UnloadStale {
        character_id: u64,
    },
    Complete {
        character_id: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerLoadPhase {
    Unloaded,
    LoadingAsset { request_id: u32 },
    ApplyingAttributes { request_id: u32, character_id: u64 },
    Registering { request_id: u32, character_id: u64 },
    Loaded { character_id: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerLoaderSlot {
    player_index: u32,
    requested: PlayerSelection,
    current: Option<PlayerSelection>,
    phase: PlayerLoadPhase,
}

impl PlayerLoaderSlot {
    #[must_use]
    pub const fn new(player_index: u32) -> Self {
        Self {
            player_index,
            requested: PlayerSelection::InitialState,
            // The original stores the out-of-enum value 3 here.
            current: None,
            phase: PlayerLoadPhase::Unloaded,
        }
    }

    #[must_use]
    pub const fn player_index(&self) -> u32 {
        self.player_index
    }

    #[must_use]
    pub const fn requested(&self) -> PlayerSelection {
        self.requested
    }

    #[must_use]
    pub const fn current(&self) -> Option<PlayerSelection> {
        self.current
    }

    #[must_use]
    pub const fn phase(&self) -> PlayerLoadPhase {
        self.phase
    }

    /// `PlayerLoader::IsLoaded` 0x828141c8: the loader holds a loaded character, whose id
    /// `OnSimCharacterLoaded` (0x82817fb4) stores before the attributes and the registration.
    #[must_use]
    pub const fn is_loaded(&self) -> bool {
        matches!(
            self.phase,
            PlayerLoadPhase::ApplyingAttributes { .. } | PlayerLoadPhase::Registering { .. } | PlayerLoadPhase::Loaded { .. }
        )
    }

    pub const fn set_marquee(&mut self, definition_id: u64, peer_id: i64) {
        self.requested = PlayerSelection::Marquee {
            definition_id,
            peer_id,
        };
    }

    pub const fn set_create_a_character(&mut self, peer_id: i64) {
        self.requested = PlayerSelection::CreateACharacter { peer_id };
    }

    /// Starts the verified marquee branch of `PlayerLoaderImpl::Load`.
    ///
    /// The current proof of concept deliberately accepts only an unloaded
    /// slot. Character replacement also has an asynchronous unload pipeline
    /// and will be added once that path is reconstructed end to end.
    ///
    /// # Errors
    ///
    /// Returns [`PlayerLoaderError`] for a non-marquee or mismatched selection,
    /// an invalid schema name, or a slot with incompatible in-flight work.
    pub fn begin_marquee_load(
        &mut self,
        definition: &MarqueeDefinition,
        request_id: u32,
    ) -> Result<PlayerLoadAction, PlayerLoaderError> {
        let selection = self.requested;
        let PlayerSelection::Marquee {
            definition_id,
            peer_id: _,
        } = selection
        else {
            return Err(PlayerLoaderError::SelectionIsNotMarquee(selection));
        };
        if definition_id != definition.id {
            return Err(PlayerLoaderError::WrongMarqueeDefinition {
                requested: definition_id,
                provided: definition.id,
            });
        }
        if definition.schema_name.is_empty() || definition.schema_name.len() >= 64 {
            return Err(PlayerLoaderError::InvalidSchemaName);
        }
        match self.phase {
            PlayerLoadPhase::Unloaded => {}
            PlayerLoadPhase::Loaded { character_id } if self.current == Some(selection) => {
                return Ok(PlayerLoadAction::Complete { character_id });
            }
            phase => return Err(PlayerLoaderError::SlotBusy(phase)),
        }

        self.current = Some(selection);
        self.phase = PlayerLoadPhase::LoadingAsset { request_id };
        // PlayerLoaderImpl::Load 0x8281aa80 passes true for every requested marquee, party
        // members included; only the invalid-CAS default marquee passes false.
        Ok(PlayerLoadAction::Load(CharacterLoadRequest {
            request_id,
            category: CharacterLoadCategory::Marquee,
            model_name: definition.schema_name.clone(),
            is_local_player: true,
        }))
    }

    /// PlayerLoaderImpl::Load8281abcc..ac24: valid CAC record byte stream. Its
    /// `SimCharacterManager::Load` 0x8281a358 takes no locality: every PlayerLoader
    /// member, the player's or a party member's, is a local player.
    pub(crate) fn begin_cac_load(&mut self, request_id:u32, model_name:&str)
        -> Result<PlayerLoadAction,PlayerLoaderError> {
        if !matches!(self.requested,PlayerSelection::CreateACharacter{..}) {
            return Err(PlayerLoaderError::MissingCurrentSelection);
        }
        if self.phase != PlayerLoadPhase::Unloaded { return Err(PlayerLoaderError::SlotBusy(self.phase)); }
        self.current=Some(self.requested);
        self.phase=PlayerLoadPhase::LoadingAsset{request_id};
        Ok(PlayerLoadAction::Load(CharacterLoadRequest{request_id, category:CharacterLoadCategory::CreateACharacter,
            model_name:model_name.to_owned(),is_local_player:true}))
    }

    /// Handles `PlayerLoaderImpl::OnSimCharacterLoaded`.
    ///
    /// # Errors
    ///
    /// Returns [`PlayerLoaderError`] if a matching load has no current marquee
    /// selection.
    pub fn character_loaded(
        &mut self,
        completed_request_id: u32,
        character_id: u64,
        registration_request_id: u32,
    ) -> Result<PlayerLoadAction, PlayerLoaderError> {
        let PlayerLoadPhase::LoadingAsset { request_id } = self.phase else {
            return Ok(PlayerLoadAction::UnloadStale { character_id });
        };
        if request_id != completed_request_id {
            return Ok(PlayerLoadAction::UnloadStale { character_id });
        }
        if self.current.is_none() {
            return Err(PlayerLoaderError::MissingCurrentSelection);
        }
        // Native82817fb4 stores the loaded asset BEFORE fallible attribute work.
        self.phase = PlayerLoadPhase::ApplyingAttributes {
            request_id: registration_request_id,
            character_id,
        };
        Ok(PlayerLoadAction::ApplyAttributes {
            character_id,
            player_index: self.player_index,
        })
    }

    // Only the actual core service continuation may publish Register. The public
    // protocol probe cannot acknowledge missing native attribute work as success.
    fn finish_loaded_attributes(
        &mut self,
        ghost_system_enabled: bool,
        apply_attributes: bool,
    ) -> Result<PlayerLoadAction, PlayerLoaderError> {
        let PlayerLoadPhase::ApplyingAttributes {
            request_id,
            character_id,
        } = self.phase
        else {
            return Err(PlayerLoaderError::AttributesNotPending);
        };
        let peer_id = match self.current {
            Some(PlayerSelection::Marquee{peer_id,..}|PlayerSelection::CreateACharacter{peer_id})=>peer_id,
            _=>return Err(PlayerLoaderError::MissingCurrentSelection),
        };
        self.phase = PlayerLoadPhase::Registering {
            request_id,
            character_id,
        };
        Ok(PlayerLoadAction::Register(CharacterRegistrationRequest {
            request_id,
            character_id,
            peer_id,
            player_index: self.player_index,
            ghost_system_enabled,
            apply_attributes,
        }))
    }

    /// Handles the matching branch of
    /// `PlayerLoaderImpl::OnSimCharacterRegistered`.
    ///
    /// # Errors
    ///
    /// Returns [`PlayerLoaderError`] when no registration is pending or its
    /// request/character identity does not match.
    pub fn character_registered(
        &mut self,
        completed_request_id: u32,
        character_id: u64,
    ) -> Result<PlayerLoadAction, PlayerLoaderError> {
        let PlayerLoadPhase::Registering {
            request_id,
            character_id: expected_character_id,
        } = self.phase
        else {
            return Err(PlayerLoaderError::UnexpectedRegistration);
        };
        if request_id != completed_request_id || expected_character_id != character_id {
            return Err(PlayerLoaderError::MismatchedRegistration {
                expected_request: request_id,
                completed_request: completed_request_id,
                expected_character: expected_character_id,
                completed_character: character_id,
            });
        }
        self.phase = PlayerLoadPhase::Loaded { character_id };
        Ok(PlayerLoadAction::Complete { character_id })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerLoaderError {
    SelectionIsNotMarquee(PlayerSelection),
    WrongMarqueeDefinition {
        requested: u64,
        provided: u64,
    },
    InvalidSchemaName,
    SlotBusy(PlayerLoadPhase),
    MissingCurrentSelection,
    UnexpectedRegistration,
    AttributesNotPending,
    MismatchedRegistration {
        expected_request: u32,
        completed_request: u32,
        expected_character: u64,
        completed_character: u64,
    },
}

impl fmt::Display for PlayerLoaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid player-loader transition: {self:?}")
    }
}

impl std::error::Error for PlayerLoaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_definition() -> MarqueeDefinition {
        MarqueeDefinition {
            id: DEFAULT_MARQUEE_ID,
            schema_name: "tutorial_male".to_owned(),
        }
    }

    #[test]
    fn constructor_matches_reference_sentinels() {
        let slot = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
        assert_eq!(slot.requested(), PlayerSelection::InitialState);
        assert_eq!(slot.current(), None);
        assert_eq!(slot.phase(), PlayerLoadPhase::Unloaded);
    }

    #[test]
    fn derives_default_marquee_asset_identity_and_paths() {
        let definition = default_definition();
        assert_eq!(definition.schema_asset_id(), 0x4D12_B500_299D_209D);
        assert_eq!(
            definition.binary_recipe_path(),
            "data\\content\\recipe\\marquee\\tutorial_male.recipe"
        );
        assert_eq!(
            definition.xml_recipe_path(),
            "data\\content\\recipe\\marquee\\tutorial_male.xml"
        );
    }

    #[test]
    fn drives_marquee_load_then_registration() {
        let mut slot = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
        slot.set_marquee(DEFAULT_MARQUEE_ID, NO_PEER_ID);
        assert_eq!(
            slot.begin_marquee_load(&default_definition(), 17).unwrap(),
            PlayerLoadAction::Load(CharacterLoadRequest {
                request_id: 17,
                category: CharacterLoadCategory::Marquee,
                model_name: "tutorial_male".to_owned(),
                is_local_player: true,
            })
        );
        assert_eq!(
            slot.character_loaded(17, 0x1234, 18).unwrap(),
            PlayerLoadAction::ApplyAttributes {
                character_id: 0x1234,
                player_index: LOCAL_PLAYER_INDEX
            }
        );
        assert_eq!(
            slot.character_registered(18, 0x1234),
            Err(PlayerLoaderError::UnexpectedRegistration)
        );
        assert_eq!(
            slot.finish_loaded_attributes(false, true).unwrap(),
            PlayerLoadAction::Register(CharacterRegistrationRequest {
                request_id: 18,
                character_id: 0x1234,
                peer_id: NO_PEER_ID,
                player_index: LOCAL_PLAYER_INDEX,
                ghost_system_enabled: false,
                apply_attributes: true,
            })
        );
        assert_eq!(
            slot.character_registered(18, 0x1234).unwrap(),
            PlayerLoadAction::Complete {
                character_id: 0x1234
            }
        );
        assert_eq!(
            slot.phase(),
            PlayerLoadPhase::Loaded {
                character_id: 0x1234
            }
        );
    }

    #[test]
    fn is_loaded_holds_from_the_loaded_asset_through_registration() {
        let mut slot = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
        slot.set_marquee(DEFAULT_MARQUEE_ID, NO_PEER_ID);
        assert!(!slot.is_loaded());
        slot.begin_marquee_load(&default_definition(), 17).unwrap();
        assert!(!slot.is_loaded());
        slot.character_loaded(17, 0x1234, 18).unwrap();
        assert!(slot.is_loaded());
        slot.finish_loaded_attributes(false, true).unwrap();
        assert!(slot.is_loaded());
        slot.character_registered(18, 0x1234).unwrap();
        assert!(slot.is_loaded());
    }

    #[test]
    fn stale_asset_completion_is_scheduled_for_unload() {
        let mut slot = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
        slot.set_marquee(DEFAULT_MARQUEE_ID, NO_PEER_ID);
        slot.begin_marquee_load(&default_definition(), 17).unwrap();
        assert_eq!(
            slot.character_loaded(16, 0x9999, 18).unwrap(),
            PlayerLoadAction::UnloadStale {
                character_id: 0x9999
            }
        );
        assert_eq!(
            slot.phase(),
            PlayerLoadPhase::LoadingAsset { request_id: 17 }
        );
    }

    #[test]
    fn same_loaded_selection_completes_without_reloading() {
        let mut slot = PlayerLoaderSlot::new(LOCAL_PLAYER_INDEX);
        slot.set_marquee(DEFAULT_MARQUEE_ID, NO_PEER_ID);
        slot.begin_marquee_load(&default_definition(), 17).unwrap();
        slot.character_loaded(17, 0x1234, 18).unwrap();
        slot.finish_loaded_attributes(false, true).unwrap();
        slot.character_registered(18, 0x1234).unwrap();
        assert_eq!(
            slot.begin_marquee_load(&default_definition(), 19).unwrap(),
            PlayerLoadAction::Complete {
                character_id: 0x1234
            }
        );
    }
}

#[path = "player_attributes_runtime.rs"]
pub(crate) mod attributes_runtime;

#[path = "player_mounted_recipe_load.rs"]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "mounted recipe producer is connected to original startup pending application composition"
    )
)]
pub(crate) mod mounted_recipe;
#[path = "sim_character_request_owner.rs"]
pub(crate) mod requests;
