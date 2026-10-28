//! Actual Actor `GetInput825f11a8`, then retained map/pad control evaluation.
use super::ActorPhysicalQueryBinding;
use crate::game_input_player::GameInputPlayerError;
use std::sync::{Arc, Mutex};

pub(crate) struct ActorCapturedPhysicalInput {
    gameplay: Arc<Mutex<crate::game_input_player::GameInputPlayer>>,
}
impl ActorCapturedPhysicalInput {
    pub(crate) fn try_value(&self, control: u32) -> Result<f32, ActorPhysicalInputError> {
        self.gameplay
            .lock()
            .map_err(|_| ActorPhysicalInputError::Poisoned)?
            .try_value(control)
            .map_err(ActorPhysicalInputError::Input)
    }
}

#[derive(Debug)]
pub(crate) enum ActorPhysicalInputError {
    Missing(&'static str),
    Poisoned,
    Input(GameInputPlayerError),
}
impl std::fmt::Display for ActorPhysicalInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Actor input missing {field}"),
            Self::Poisoned => f.write_str("Actor input owner lock poisoned"),
            Self::Input(error) => write!(f, "Actor input: {error}"),
        }
    }
}
impl ActorPhysicalQueryBinding {
    /// IPhysical.GetFlags/GetPlayerID and `IsNonPlayerAISkater`825ed100: an AI
    /// controller whose client type is not `AI_CLIENT_PLAYER`.
    pub(crate) fn try_scoring_identity(&self) -> Result<(u32, u32, bool), ActorPhysicalInputError> {
        use ActorPhysicalInputError as E;
        let state = self.state.upgrade().ok_or(E::Missing("live Actor"))?;
        let actor = state.lock().map_err(|_| E::Poisoned)?;
        let fields = actor.fields().ok_or(E::Missing("Actor initial fields"))?;
        let non_player_ai = actor
            .physics_auxiliary
            .as_ref()
            .ok_or(E::Missing("Actor AI pointer"))?
            .ai_controller
            .as_ref()
            .is_some_and(|client| **client != crate::ai::actor::AiClientType::Player);
        Ok((fields.player_index, fields.flags, non_player_ai))
    }
    /// `PhysicalPlayerHiLOD::GetFlags` 0x82dffd60: the Actor query's flags.
    pub(crate) fn try_flags(&self) -> Result<u32, ActorPhysicalInputError> {
        use ActorPhysicalInputError as E;
        let state = self.state.upgrade().ok_or(E::Missing("live Actor"))?;
        let actor = state.lock().map_err(|_| E::Poisoned)?;
        Ok(actor.fields().ok_or(E::Missing("Actor initial fields"))?.flags)
    }
    /// IActor.GetPlayerIndex: read the retained Actor's initialized fields.
    pub(crate) fn try_player_index(&self) -> Result<u32, ActorPhysicalInputError> {
        use ActorPhysicalInputError as E;
        let state = self.state.upgrade().ok_or(E::Missing("live Actor"))?;
        let actor = state.lock().map_err(|_| E::Poisoned)?;
        Ok(actor
            .fields()
            .ok_or(E::Missing("Actor initial fields"))?
            .player_index)
    }
    pub(crate) fn try_input_value(
        &self,
        control: u32,
    ) -> Result<Option<f32>, ActorPhysicalInputError> {
        self.try_capture_input()?
            .map(|input| input.try_value(control))
            .transpose()
    }
    /// `GetInput` is evaluated once; later values reread the captured object's
    /// current map/pad without repeating the nullable Actor controller query.
    pub(crate) fn try_capture_input(
        &self,
    ) -> Result<Option<ActorCapturedPhysicalInput>, ActorPhysicalInputError> {
        use ActorPhysicalInputError as E;
        let state = self.state.upgrade().ok_or(E::Missing("live Actor"))?;
        let actor = state.lock().map_err(|_| E::Poisoned)?;
        if actor
            .physics_auxiliary
            .as_ref()
            .ok_or(E::Missing("Actor AI controller field"))?
            .ai_controller
            .is_some()
        {
            return Ok(None);
        }
        drop(actor);
        // The actual registered gameplay receiver retains the selected player's
        // map and pad. Unsupported global expressions are refused by that map;
        // no synthetic InputSK8 global values are supplied here.
        Ok(Some(ActorCapturedPhysicalInput {
            gameplay: Arc::clone(self.entity.key().gameplay()),
        }))
    }
}
