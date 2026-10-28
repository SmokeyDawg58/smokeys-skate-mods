//! Selected serialized queue6 route over the SAME recognizer broadcaster.
//! Source buffer allocator/embedding and native `Actor` lifetime are not claimed.
use super::{GameInputPatternBroadcaster, GameInputPatternMessage};
use crate::action_graph_input_listener::{ActionGraphInputListener, ActionInputListenerError};
use std::sync::{Arc, Mutex, Weak};

const PATTERN: u32 = 0x6889_aa9b;
const HELD: u32 = 0x95d8_1395;
// Create825f3aa0 stores the same7381d2e7 kind as GenerateGameInputEvents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    InputEvent,
    Pattern,
    Held,
}
impl Kind {
    fn matches(self, kind: u32) -> bool {
        match self {
            Self::InputEvent => kind == 0x7381_d2e7,
            Self::Pattern => kind == PATTERN,
            Self::Held => kind == HELD,
        }
    }
}
struct Handler<K> {
    key: Arc<()>,
    listener: Weak<Mutex<ActionGraphInputListener<K>>>,
    port: u32,
    kind: Kind,
    id: u32,
    no_filter: bool,
}
#[derive(Clone)]
pub(crate) struct GameInputListenerRegistration {
    route: Arc<()>,
    keys: Vec<Arc<()>>,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum GameInputRouteError {
    Listener(ActionInputListenerError),
    ListenerLock,
    ExpiredListener,
    Allocation,
    DeliveryInProgress,
    ForeignRegistration,
}
pub(crate) struct GameInputEventRoute<K> {
    occurrence: Arc<()>,
    broadcaster: GameInputPatternBroadcaster,
    handlers: Vec<Handler<K>>,
    // Checked-host failure cursor: preceding handlers must not be redelivered.
    current_handler: Option<usize>,
    #[cfg(test)]
    fail_registration: Option<usize>,
}
impl<K> GameInputEventRoute<K> {
    #[cfg(test)]
    pub(crate) fn fail_registration_at_for_test(&mut self, index: usize) {
        self.fail_registration = Some(index);
    }
    #[cfg(test)]
    pub(crate) fn registered_handler_count_for_test(&self) -> usize {
        self.handlers.len()
    }
    pub(crate) fn new() -> Self {
        // Both original wrappers call hash32 with the SAME ABCDEF00 seed.
        let port = skate2_formats::state_graph::state_graph_string_hash32(b"GameInputBroadcaster");
        Self {
            occurrence: Arc::new(()),
            broadcaster: GameInputPatternBroadcaster::new(port, 6),
            handlers: Vec::new(),
            current_handler: None,
            #[cfg(test)]
            fail_registration: None,
        }
    }
    pub(crate) fn broadcaster_mut(&mut self) -> &mut GameInputPatternBroadcaster {
        &mut self.broadcaster
    }
    pub(crate) fn pending(&self) -> usize {
        self.broadcaster.messages.len()
    }
    pub(crate) fn try_construct_listener(
        &mut self,
        listener: &Arc<Mutex<ActionGraphInputListener<K>>>,
    ) -> Result<GameInputListenerRegistration, GameInputRouteError> {
        use GameInputRouteError as E;
        if self.current_handler.is_some() {
            return Err(E::DeliveryInProgress);
        }
        let mut owner = listener.lock().map_err(|_| E::ListenerLock)?;
        owner.begin_construction().map_err(E::Listener)?;
        owner.registration = Some(GameInputListenerRegistration {
            route: Arc::clone(&self.occurrence),
            keys: Vec::new(),
        });
        // No eager three-handler reserve: a late allocation refusal retains the
        // earlier actual registered handlers and the one-attempt owner prefix.
        for kind in [Kind::InputEvent, Kind::Pattern, Kind::Held] {
            #[cfg(test)]
            if self.fail_registration == Some(self.handlers.len()) {
                return Err(E::Allocation);
            }
            self.handlers.try_reserve(1).map_err(|_| E::Allocation)?;
            let registration = owner
                .registration
                .as_mut()
                .expect("construction registration");
            registration
                .keys
                .try_reserve(1)
                .map_err(|_| E::Allocation)?;
            let key = Arc::new(());
            self.handlers.push(Handler {
                key: Arc::clone(&key),
                listener: Arc::downgrade(listener),
                port: self.broadcaster.port,
                kind,
                id: 0,
                no_filter: false,
            });
            registration.keys.push(key);
        }
        owner.finish_construction();
        Ok(owner
            .registration
            .as_ref()
            .expect("construction registration")
            .clone())
    }
    pub(crate) fn try_unregister(
        &mut self,
        registration: &GameInputListenerRegistration,
    ) -> Result<(), GameInputRouteError> {
        if !Arc::ptr_eq(&registration.route, &self.occurrence) {
            return Err(GameInputRouteError::ForeignRegistration);
        }
        if self.current_handler.is_some() {
            return Err(GameInputRouteError::DeliveryInProgress);
        }
        self.handlers.retain(|handler| {
            !registration
                .keys
                .iter()
                .any(|key| Arc::ptr_eq(key, &handler.key))
        });
        Ok(())
    }
    /// Explicit checked-host recovery after a refused callback. Removes only
    /// the authorized subscription(s), preserves the current message, and
    /// adjusts the cursor so completed callbacks are never repeated. This is
    /// not a claim about native exceptions or general reentrant teardown.
    pub(crate) fn try_unregister_failed_delivery(
        &mut self,
        registration: &GameInputListenerRegistration,
    ) -> Result<(), GameInputRouteError> {
        if !Arc::ptr_eq(&registration.route, &self.occurrence) {
            return Err(GameInputRouteError::ForeignRegistration);
        }
        let Some(cursor) = self.current_handler else {
            return self.try_unregister(registration);
        };
        let mut old_index = 0;
        let mut retained_prefix = 0;
        self.handlers.retain(|handler| {
            let keep = !registration
                .keys
                .iter()
                .any(|key| Arc::ptr_eq(key, &handler.key));
            if old_index < cursor && keep {
                retained_prefix += 1;
            }
            old_index += 1;
            keep
        });
        self.current_handler = Some(retained_prefix);
        Ok(())
    }
    /// Live drain of selected pattern payloads. No `Drop` flush/removal. Rust's
    /// exclusive borrow excludes reentrant membership mutation; a checked
    /// delivery failure retains the current message and next callback cursor.
    pub(crate) fn try_flush(&mut self) -> Result<usize, GameInputRouteError> {
        let mut popped = 0;
        while let Some((port, message)) = self.broadcaster.messages.first() {
            let kind = match message {
                GameInputPatternMessage::InputEvent { .. } => 0x7381_d2e7,
                GameInputPatternMessage::Match { .. } => PATTERN,
                GameInputPatternMessage::Held { .. } => HELD,
            };
            // Native constructors of both selected payloads set mID=0.
            let id = 0;
            let mut cursor = self.current_handler.unwrap_or(0);
            self.current_handler = Some(cursor);
            while cursor < self.handlers.len() {
                let handler = &self.handlers[cursor];
                if handler.port == *port
                    && handler.kind.matches(kind)
                    && (handler.id == id || handler.no_filter)
                {
                    let listener = handler
                        .listener
                        .upgrade()
                        .ok_or(GameInputRouteError::ExpiredListener)?;
                    listener
                        .lock()
                        .map_err(|_| GameInputRouteError::ListenerLock)?
                        .receive(message)
                        .map_err(GameInputRouteError::Listener)?;
                }
                cursor += 1;
                self.current_handler = Some(cursor);
            }
            self.broadcaster.messages.remove(0);
            self.current_handler = None;
            popped += 1;
        }
        Ok(popped)
    }
}

#[cfg(test)]
#[path = "game_input_event_route_tests.rs"]
mod tests;
