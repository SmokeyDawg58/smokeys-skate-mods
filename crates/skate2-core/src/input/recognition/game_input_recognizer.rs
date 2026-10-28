//! Same command-owned gameplay input to source recognizers826e18d8/826e2008.
//! Selected late construction and retained queue messages; no `Actor`/flush claim.
use super::{GameInputPlayer, GameInputPlayerError, GameInputReceiver};
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeVault},
    input_map::SelectedInputMap,
    input_pad::InputPad,
    input_pattern::{InputPattern, InputPatterns},
    input_pattern_matcher::{
        InputPatternCvars, InputPatternHistory, InputPatternMatch, InputPatternMatchError,
        InputPatternMatcher, InputPatternStick, InputPatternVaultAttributes,
    },
};
use std::{fmt, sync::Arc};
#[path = "../game_input_event_route.rs"]
pub(crate) mod event_route;

pub(crate) trait GameInputRecognizerServices: InputPatternCvars {
    type DeadzoneTarget;
    type DisplayTarget;
    type DebugInstance;
    fn capture_deadzone(&mut self) -> Result<Self::DeadzoneTarget, Self::Error>;
    fn deadzone(&mut self, target: &Self::DeadzoneTarget) -> Result<[f32; 2], Self::Error>;
    /// Process-shared once-copied LEFT and RIGHT display refs, not per-player.
    fn ensure_gesture_display_references(&mut self) -> Result<(), Self::Error>;
    fn gesture_display_value(&mut self, stick: InputPatternStick) -> Result<bool, Self::Error>;
    fn local_player_index(&mut self) -> Result<u32, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
    fn capture_debug_instance(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DebugInstance, Self::Error>;
    fn debug_lookup(
        &mut self,
        instance: &Self::DebugInstance,
        key: u64,
        default_bytes: usize,
    ) -> Result<(), Self::Error>;
    fn release_debug_instance(&mut self, instance: Self::DebugInstance) -> Result<(), Self::Error>;
    fn capture_input_display(
        &mut self,
        stick: InputPatternStick,
    ) -> Result<Self::DisplayTarget, Self::Error>;
    fn input_display_value(&mut self, target: &Self::DisplayTarget) -> Result<bool, Self::Error>;
}
#[derive(Clone, Debug)]
pub(crate) enum GameInputPatternMessage {
    InputEvent {
        input: GameInputReceiver,
        event: GameInputEvent,
        player_index: u32,
    },
    Held {
        pattern: Arc<InputPattern>,
        player_index: u32,
    },
    Match {
        input: GameInputReceiver,
        matched: InputPatternMatch,
        player_index: u32,
    },
}
/// Source eInputEvents0..4 and mInputEventName82fe5100. Debug controls are
/// generated separately from gameplay patterns, not mapped to skating tricks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[expect(
    clippy::enum_variant_names,
    reason = "exact original input-event names"
)]
pub(crate) enum GameInputEvent {
    DebugPrev,
    DebugNext,
    DebugPause,
    DebugReplaceSkateboard,
    DebugResetWorld,
}
impl GameInputEvent {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::DebugPrev => "DebugPrev",
            Self::DebugNext => "DebugNext",
            Self::DebugPause => "DebugPause",
            Self::DebugReplaceSkateboard => "DebugReplaceSkateboard",
            Self::DebugResetWorld => "DebugResetWorld",
        }
    }
}
#[derive(Debug)]
pub(crate) enum GameInputEventError {
    Player(GameInputPlayerError),
    Queue(i32),
}
/// Selected actual broadcaster storage; port identity is supplied by its owner.
/// Queue6 is checked at native enqueue sites. No implicit dispatch or flush.
pub(crate) struct GameInputPatternBroadcaster {
    port: u32,
    queue: i32,
    messages: Vec<(u32, GameInputPatternMessage)>,
}
impl GameInputPatternBroadcaster {
    pub(crate) const fn new(port: u32, queue: i32) -> Self {
        Self {
            port,
            queue,
            messages: Vec::new(),
        }
    }
    pub(crate) fn messages(&self) -> &[(u32, GameInputPatternMessage)] {
        &self.messages
    }
    fn enqueue(&mut self, message: GameInputPatternMessage) -> Result<(), i32> {
        if self.queue != 6 {
            return Err(self.queue);
        }
        self.messages.push((self.port, message));
        Ok(())
    }
}
struct StickState {
    history: InputPatternHistory,
    matchers: Vec<InputPatternMatcher<InputPatternVaultAttributes>>,
}
impl StickState {
    fn new(
        vault: &Arc<AttributeRuntimeVault>,
        stick: InputPatternStick,
        patterns: &[Arc<InputPatterns>],
    ) -> Self {
        let history = InputPatternHistory::new();
        let matchers = patterns
            .iter()
            .map(|patterns| {
                InputPatternMatcher::new(
                    patterns,
                    &history,
                    InputPatternVaultAttributes::bind(vault, stick),
                )
            })
            .collect();
        Self { history, matchers }
    }
}
pub(crate) struct GameInputRecognizer {
    left: StickState,
    right: StickState,
    patterns_added: bool,
}
#[derive(Debug)]
pub(crate) enum GameInputRecognizerError<E> {
    Player(GameInputPlayerError),
    AlreadyInitialized,
    InitializationRequired,
    Source(E),
    Match(InputPatternMatchError<AttributeRuntimeError, E>),
    Held(InputPatternMatchError<AttributeRuntimeError, std::convert::Infallible>),
    Queue(i32),
    MissingMatchedPattern,
}
impl<E: fmt::Debug> fmt::Display for GameInputRecognizerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player(e) => write!(f, "recognizer player: {e}"),
            Self::AlreadyInitialized => f.write_str("recognizer already initialized"),
            Self::InitializationRequired => f.write_str("recognizer initialization required"),
            Self::Source(e) => write!(f, "recognizer service: {e:?}"),
            Self::Match(e) => write!(f, "recognizer matching: {e:?}"),
            Self::Held(e) => write!(f, "recognizer held: {e:?}"),
            Self::Queue(q) => write!(f, "recognizer source queue must be6, received {q}"),
            Self::MissingMatchedPattern => {
                f.write_str("successful recognizer match has no pattern")
            }
        }
    }
}
impl GameInputPlayer {
    /// `PlayerInput2440` allocates both empty histories before its receiver.
    pub(crate) fn try_construct_recognizer_histories(
        &mut self,
    ) -> Result<(), GameInputRecognizerError<std::convert::Infallible>> {
        if self.recognizer.is_some() {
            return Err(GameInputRecognizerError::AlreadyInitialized);
        }
        let left = StickState {
            history: InputPatternHistory::new(),
            matchers: Vec::new(),
        };
        let right = StickState {
            history: InputPatternHistory::new(),
            matchers: Vec::new(),
        };
        self.recognizer = Some(GameInputRecognizer {
            left,
            right,
            patterns_added: false,
        });
        Ok(())
    }
    /// Manager Reset publishes the `PlayerInput` pointer before `AddPatterns`.
    pub(crate) fn try_add_recognizer_patterns(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        left: &[Arc<InputPatterns>],
        right: &[Arc<InputPatterns>],
    ) -> Result<(), GameInputRecognizerError<std::convert::Infallible>> {
        self.require_registered()
            .map_err(GameInputRecognizerError::Player)?;
        let state = self
            .recognizer
            .as_mut()
            .ok_or(GameInputRecognizerError::InitializationRequired)?;
        if state.patterns_added {
            return Err(GameInputRecognizerError::AlreadyInitialized);
        }
        state.patterns_added = true;
        for (state, stick, patterns) in [
            (&mut state.left, InputPatternStick::Left, left),
            (&mut state.right, InputPatternStick::Right, right),
        ] {
            for patterns in patterns {
                state.matchers.push(InputPatternMatcher::new(
                    patterns,
                    &state.history,
                    InputPatternVaultAttributes::bind(vault, stick),
                ));
            }
        }
        Ok(())
    }
    /// `GenerateGameInputEvents826dfdc0`. Caller schedules after `UpdateRecognizer`
    /// and before queue6 Flush; each mapped `GetState` is a fresh retained read.
    /// Earlier queued events survive any later map/queue refusal.
    pub(crate) fn try_generate_input_events(
        &mut self,
        broadcaster: &mut GameInputPatternBroadcaster,
    ) -> Result<(), GameInputEventError> {
        use GameInputEvent::{
            DebugNext, DebugPause, DebugPrev, DebugReplaceSkateboard, DebugResetWorld,
        };
        for (id, event) in [
            (59, DebugPrev),
            (60, DebugNext),
            (61, DebugPause),
            (62, DebugReplaceSkateboard),
            (63, DebugResetWorld),
        ] {
            if self.try_value(id).map_err(GameInputEventError::Player)? != 0.0 {
                broadcaster
                    .enqueue(GameInputPatternMessage::InputEvent {
                        input: self.receiver.clone(),
                        event,
                        player_index: self.player_index(),
                    })
                    .map_err(GameInputEventError::Queue)?;
            }
        }
        Ok(())
    }
    /// Selected late component construction: native histories precede receiver
    /// construction, while `AddPatterns` follows it. This bounded bridge does not
    /// claim the omitted full `PlayerInput` constructor chronology.
    pub(crate) fn try_initialize_recognizer(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        patterns: &[Arc<InputPatterns>; 7],
    ) -> Result<(), GameInputRecognizerError<std::convert::Infallible>> {
        self.require_registered()
            .map_err(GameInputRecognizerError::Player)?;
        if self.recognizer.is_some() {
            return Err(GameInputRecognizerError::AlreadyInitialized);
        }
        // Native AddPatterns order: left collections then right collections.
        let left = StickState::new(vault, InputPatternStick::Left, &patterns[5..]);
        let right = StickState::new(vault, InputPatternStick::Right, &patterns[..5]);
        self.recognizer = Some(GameInputRecognizer {
            left,
            right,
            patterns_added: true,
        });
        Ok(())
    }
    pub(crate) fn try_update_recognizer<S: GameInputRecognizerServices>(
        &mut self,
        services: &mut S,
        broadcaster: &mut GameInputPatternBroadcaster,
    ) -> Result<(), GameInputRecognizerError<S::Error>> {
        self.require_registered()
            .map_err(GameInputRecognizerError::Player)?;
        let state = self
            .recognizer
            .as_mut()
            .ok_or(GameInputRecognizerError::InitializationRequired)?;
        let player_index = self.receiver.id().wrapping_sub(10);
        let y = -read(&self.map, &mut self.pad, 78)?;
        let x = read(&self.map, &mut self.pad, 77)?;
        update_stick(
            &mut state.right,
            InputPatternStick::Right,
            [x, y],
            player_index,
            &self.receiver,
            services,
            broadcaster,
        )?;
        let y = -read(&self.map, &mut self.pad, 75)?;
        let x = read(&self.map, &mut self.pad, 74)?;
        update_stick(
            &mut state.left,
            InputPatternStick::Left,
            [x, y],
            player_index,
            &self.receiver,
            services,
            broadcaster,
        )?;
        display_input(InputPatternStick::Right, &self.map, &mut self.pad, services)?;
        display_input(InputPatternStick::Left, &self.map, &mut self.pad, services)?;
        Ok(())
    }
}
fn read<E>(
    map: &SelectedInputMap,
    pad: &mut InputPad,
    id: u32,
) -> Result<f32, GameInputRecognizerError<E>> {
    map.try_value(id, pad)
        .map_err(|e| GameInputRecognizerError::Player(GameInputPlayerError::Map(e)))
}
fn update_stick<S: GameInputRecognizerServices>(
    state: &mut StickState,
    stick: InputPatternStick,
    point: [f32; 2],
    player: u32,
    input: &GameInputReceiver,
    services: &mut S,
    broadcaster: &mut GameInputPatternBroadcaster,
) -> Result<(), GameInputRecognizerError<S::Error>> {
    use GameInputRecognizerError as E;
    let target = services.capture_deadzone().map_err(E::Source)?;
    let zone = services.deadzone(&target).map_err(E::Source)?;
    let filtered = [
        if zone[0] > point[0].abs() {
            0.0
        } else {
            point[0]
        },
        if zone[1] > point[1].abs() {
            0.0
        } else {
            point[1]
        },
    ];
    state.history.publish_selected_point(filtered);
    for matcher in &mut state.matchers {
        if let Some(pattern) = matcher.try_holding(&state.history).map_err(E::Held)? {
            broadcaster
                .enqueue(GameInputPatternMessage::Held {
                    pattern,
                    player_index: player,
                })
                .map_err(E::Queue)?;
        }
    }
    for matcher in &mut state.matchers {
        let mut matched = InputPatternMatch {
            pattern: None,
            normalized_speed: 0.0,
            error: 1_000_000.0,
            time: 0.0,
        };
        if matcher
            .try_find_match(&state.history, services, &mut matched)
            .map_err(E::Match)?
        {
            services
                .ensure_gesture_display_references()
                .map_err(E::Source)?;
            if matched.pattern.is_none() {
                return Err(E::MissingMatchedPattern);
            }
            display_match(stick, player, services)?;
            broadcaster
                .enqueue(GameInputPatternMessage::Match {
                    input: input.clone(),
                    matched,
                    player_index: player,
                })
                .map_err(E::Queue)?;
        }
    }
    Ok(())
}
fn display_match<S: GameInputRecognizerServices>(
    stick: InputPatternStick,
    player: u32,
    services: &mut S,
) -> Result<(), GameInputRecognizerError<S::Error>> {
    use GameInputRecognizerError as E;
    if !services.gesture_display_value(stick).map_err(E::Source)? {
        return Ok(());
    }
    if services.local_player_index().map_err(E::Source)? != player {
        return Ok(());
    }
    services.require_overlay().map_err(E::Source)?;
    let instance = services.capture_debug_instance(stick).map_err(E::Source)?;
    for (key, count, size) in [
        (0xcaaf_af92_d389_3b92, 2, 4),
        (0x06ec_96d7_ee3f_354f, 4, 16),
        (0x23ff_57fd_eddc_69ab, 4, 16),
        (0x0fec_b44c_9679_1635, 1, 4),
    ] {
        for _ in 0..count {
            services
                .debug_lookup(&instance, key, size)
                .map_err(E::Source)?;
        }
    }
    services.release_debug_instance(instance).map_err(E::Source)
}
fn display_input<S: GameInputRecognizerServices>(
    stick: InputPatternStick,
    map: &SelectedInputMap,
    pad: &mut InputPad,
    services: &mut S,
) -> Result<(), GameInputRecognizerError<S::Error>> {
    use GameInputRecognizerError as E;
    let target = services.capture_input_display(stick).map_err(E::Source)?;
    if !services.input_display_value(&target).map_err(E::Source)? {
        return Ok(());
    }
    let instance = services.capture_debug_instance(stick).map_err(E::Source)?;
    services
        .debug_lookup(&instance, 0x0fec_b44c_9679_1635, 4)
        .map_err(E::Source)?;
    let ids = if stick == InputPatternStick::Right {
        [78, 77]
    } else {
        [75, 74]
    };
    for id in ids {
        read::<S::Error>(map, pad, id)?;
    }
    services.require_overlay().map_err(E::Source)?;
    services.release_debug_instance(instance).map_err(E::Source)
}
#[cfg(test)]
#[path = "game_input_recognizer_tests.rs"]
mod tests;

impl fmt::Display for GameInputEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player(error) => write!(f, "input event player: {error:?}"),
            Self::Queue(queue) => write!(f, "input event requires queue6, received {queue}"),
        }
    }
}
