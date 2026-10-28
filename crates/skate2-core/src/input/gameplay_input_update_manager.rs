//! Native `GameInputManager`826e3308/3260/2c20/2578 retained ownership.
//! This is distinct from the command manager that owns receiver registrations.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "actual manager is exercised by the original-data Actor corpus; Simulation frame scheduling is pending"
    )
)]
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    game_input_manager::{GameInputCommandManager, GameInputManagerError},
    game_input_player::{
        GameInputPlayer, GameInputPlayerError,
        recognizer::{
            GameInputEventError, GameInputRecognizerError, GameInputRecognizerServices,
            event_route::{GameInputEventRoute, GameInputRouteError},
        },
    },
    input_map::SelectedInputMap,
    input_pattern::{InputPatternParseError, InputPatterns},
    input_pattern_matcher::InputPatternStick,
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
};

#[cfg(test)]
#[path = "gameplay_input_update_manager_tests.rs"]
mod tests;

const FILES: [(&str, InputPatternStick); 7] = [
    ("data/joystick/skater.pat", InputPatternStick::Right),
    ("data/joystick/skater90.pat", InputPatternStick::Right),
    ("data/joystick/skaterN90.pat", InputPatternStick::Right),
    ("data/joystick/skater_air.pat", InputPatternStick::Right),
    (
        "data/joystick/skater_fingerflip.pat",
        InputPatternStick::Right,
    ),
    ("data/joystick/skaterls.pat", InputPatternStick::Left),
    ("data/joystick/skaterstep.pat", InputPatternStick::Left),
];

#[derive(Debug)]
pub(crate) enum GameplayInputLifecycleError<E> {
    ConstructionAlreadyAttempted,
    ConstructionRequired,
    Load(E),
    Parse(InputPatternParseError),
    PlayerLock,
    Command(GameInputManagerError<Infallible>),
    Recognizer(GameInputRecognizerError<Infallible>),
}
#[derive(Debug)]
pub(crate) enum GameplayInputUpdateError<E> {
    Overlay(DebugOverlayError),
    PlayerLock,
    Player(GameInputPlayerError),
    Recognizer(GameInputRecognizerError<E>),
    Events(GameInputEventError),
    Flush(GameInputRouteError),
}
impl<E: std::fmt::Debug> std::fmt::Display for GameplayInputLifecycleError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstructionAlreadyAttempted => {
                f.write_str("game input manager construction already attempted")
            }
            Self::ConstructionRequired => f.write_str("game input manager construction required"),
            Self::Load(e) => write!(f, "input pattern load: {e:?}"),
            Self::Parse(e) => write!(f, "input pattern parse: {e:?}"),
            Self::PlayerLock => f.write_str("game input player lock poisoned"),
            Self::Command(e) => write!(f, "game input receiver lifecycle: {e}"),
            Self::Recognizer(e) => write!(f, "game input recognizer lifecycle: {e}"),
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for GameplayInputUpdateError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overlay(e) => write!(f, "game input update overlay: {e}"),
            Self::PlayerLock => f.write_str("game input update player lock poisoned"),
            Self::Player(e) => write!(f, "game input update player: {e}"),
            Self::Recognizer(e) => write!(f, "game input update recognizer: {e}"),
            Self::Events(e) => write!(f, "game input update events: {e:?}"),
            Self::Flush(e) => write!(f, "game input update flush: {e:?}"),
        }
    }
}

pub(crate) struct GameplayInputUpdateManager<K> {
    players: Vec<Arc<Mutex<GameInputPlayer>>>,
    route: GameInputEventRoute<K>,
    left_patterns: Vec<Arc<InputPatterns>>,
    right_patterns: Vec<Arc<InputPatterns>>,
    construction_attempted: bool,
    constructed: bool,
}
impl<K> GameplayInputUpdateManager<K> {
    /// Constructor826e3308 initializes the empty list and broadcaster BEFORE Load.
    /// Failed Load leaves its already-published pattern collections inspectable.
    pub(crate) fn new() -> Self {
        Self {
            players: Vec::with_capacity(9),
            route: GameInputEventRoute::new(),
            left_patterns: Vec::new(),
            right_patterns: Vec::new(),
            construction_attempted: false,
            constructed: false,
        }
    }
    pub(crate) fn try_construct<E>(
        &mut self,
        mut load: impl FnMut(&str) -> Result<Vec<u8>, E>,
    ) -> Result<(), GameplayInputLifecycleError<E>> {
        use GameplayInputLifecycleError as Error;
        if self.construction_attempted {
            return Err(Error::ConstructionAlreadyAttempted);
        }
        self.construction_attempted = true;
        for (path, stick) in FILES {
            let bytes = load(path).map_err(Error::Load)?;
            let patterns = InputPatterns::parse(&bytes).map_err(Error::Parse)?;
            match stick {
                InputPatternStick::Left => self.left_patterns.push(patterns),
                InputPatternStick::Right => self.right_patterns.push(patterns),
            }
        }
        self.constructed = true;
        Ok(())
    }
    /// Reset826e2c20 deletes every old player before clearing the list, then
    /// publishes each newly constructed player before left/right `AddPatterns`.
    pub(crate) fn try_reset(
        &mut self,
        map: &Arc<SelectedInputMap>,
        vault: &Arc<AttributeRuntimeVault>,
        commands: &mut GameInputCommandManager,
    ) -> Result<(), GameplayInputLifecycleError<Infallible>> {
        use GameplayInputLifecycleError as Error;
        if !self.constructed {
            return Err(Error::ConstructionRequired);
        }
        for player in &self.players {
            let mut player = player.lock().map_err(|_| Error::PlayerLock)?;
            // Histories/matchers precede the receiver's unregister callback.
            let receiver = player.begin_native_destruction();
            commands.try_unregister(&receiver).map_err(Error::Command)?;
            player.finish_native_destruction();
        }
        self.players.clear();
        for index in 0..9 {
            // The map is captured before BOTH histories, before GameInputSK8.
            let mut player = GameInputPlayer::new(index, Arc::clone(map));
            player
                .try_construct_recognizer_histories()
                .map_err(Error::Recognizer)?;
            let player = Arc::new(Mutex::new(player));
            commands
                .try_construct_player(&player)
                .map_err(Error::Command)?;
            self.players.push(Arc::clone(&player));
            player
                .lock()
                .map_err(|_| Error::PlayerLock)?
                .try_add_recognizer_patterns(vault, &self.left_patterns, &self.right_patterns)
                .map_err(Error::Recognizer)?;
        }
        Ok(())
    }
    pub(crate) fn player(&self, index: usize) -> Option<&Arc<Mutex<GameInputPlayer>>> {
        self.players.get(index)
    }
    pub(crate) fn loaded_patterns(&self, stick: InputPatternStick) -> &[Arc<InputPatterns>] {
        match stick {
            InputPatternStick::Left => &self.left_patterns,
            InputPatternStick::Right => &self.right_patterns,
        }
    }
    pub(crate) fn route_mut(&mut self) -> &mut GameInputEventRoute<K> {
        &mut self.route
    }

    /// Update826e2578. The pad count gate precedes flags; each player's complete
    /// recognizer/event prefix precedes the next player. Flush occurs once last.
    pub(crate) fn try_update<S: GameInputRecognizerServices>(
        &mut self,
        mask: u32,
        overlay: &DebugOverlaySlot,
        services: &mut S,
    ) -> Result<usize, GameplayInputUpdateError<S::Error>> {
        use GameplayInputUpdateError as Error;
        overlay.require().map_err(Error::Overlay)?;
        for player in &self.players {
            let mut player = player.lock().map_err(|_| Error::PlayerLock)?;
            if !player
                .try_native_update_eligible(mask)
                .map_err(Error::Player)?
            {
                continue;
            }
            player
                .try_update_recognizer(services, self.route.broadcaster_mut())
                .map_err(Error::Recognizer)?;
            player
                .try_generate_input_events(self.route.broadcaster_mut())
                .map_err(Error::Events)?;
        }
        self.route.try_flush().map_err(Error::Flush)
    }
}
