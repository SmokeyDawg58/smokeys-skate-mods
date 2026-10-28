//! Actual late debug side effects; the selected display call is native BLR.
use super::TrickIntents;
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    play_animation::FastString36,
};
use std::sync::{Arc, Mutex};
pub(crate) trait GestureMappingDebug {
    type Error;
    fn enabled(&mut self) -> Result<bool, Self::Error>;
    fn after_strings(&mut self, key: &FastString36, pair: &TrickIntents)
    -> Result<(), Self::Error>;
}
/// One application-global yPos830207d0 (.data initial400), shared across mappings.
#[derive(Clone)]
pub(crate) struct GestureMappingDebugPosition(Arc<Mutex<u32>>);
impl GestureMappingDebugPosition {
    pub(crate) fn new() -> Self {
        Self(Arc::new(Mutex::new(400)))
    }
}
#[derive(Debug)]
pub(crate) enum MappingDebugError<E> {
    Switch(E),
    Overlay(DebugOverlayError),
    Poisoned,
}
impl<E: std::fmt::Debug> std::fmt::Display for MappingDebugError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Switch(e) => write!(f, "gesture debug switch: {e:?}"),
            Self::Overlay(e) => write!(f, "gesture debug overlay: {e}"),
            Self::Poisoned => write!(f, "gesture debug position lock poisoned"),
        }
    }
}
/// The switch closure must sample the live source cVar reference at invocation.
pub(crate) struct LiveGestureMappingDebug<'a, F> {
    pub(crate) switch: F,
    pub(crate) overlay: &'a DebugOverlaySlot,
    pub(crate) position: &'a GestureMappingDebugPosition,
}
impl<E, F: FnMut() -> Result<bool, E>> GestureMappingDebug for LiveGestureMappingDebug<'_, F> {
    type Error = MappingDebugError<E>;
    fn enabled(&mut self) -> Result<bool, Self::Error> {
        (self.switch)().map_err(MappingDebugError::Switch)
    }
    fn after_strings(
        &mut self,
        _key: &FastString36,
        _pair: &TrickIntents,
    ) -> Result<(), Self::Error> {
        // ToUpperString writes only discarded stack strings; all authored keys valid.
        // Require holder BEFORE yPos load; call165c targets825187c0 word4e800020.
        let _overlay = self.overlay.require().map_err(MappingDebugError::Overlay)?;
        let mut position = self
            .position
            .0
            .lock()
            .map_err(|_| MappingDebugError::Poisoned)?;
        let updated = position.wrapping_add(20);
        *position = updated;
        if updated > 500 {
            *position = 400;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_graph_construction::ActorGraphIntents;
    #[test]
    fn shared_position_requires_overlay_before_mutation_and_wraps_after_500() {
        let overlay = DebugOverlaySlot::default();
        let position = GestureMappingDebugPosition::new();
        let alias = position.clone();
        let mut debug = LiveGestureMappingDebug {
            switch: || Ok::<_, ()>(true),
            overlay: &overlay,
            position: &position,
        };
        let name = ActorGraphIntents::native_name(b"Ollie");
        let pair = TrickIntents {
            ordinary: name,
            mirrored: name,
        };
        assert!(debug.after_strings(&name, &pair).is_err());
        assert_eq!(*alias.0.lock().unwrap(), 400);
        overlay.try_construct().unwrap();
        for expected in [420, 440, 460, 480, 500, 400] {
            debug.after_strings(&name, &pair).unwrap();
            assert_eq!(*alias.0.lock().unwrap(), expected);
        }
    }
}
