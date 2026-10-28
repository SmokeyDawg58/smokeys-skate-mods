//! Retained key subscriptions of `AttribSys::Monitor`; not the `NotifyAll` pipeline.
use std::sync::{Arc, Mutex, Weak, atomic::AtomicBool};

pub(crate) struct AttributeNotifications {
    entries: Mutex<Vec<(u64, Weak<AtomicBool>)>>,
}
#[derive(Debug)]
pub(crate) enum AttributeNotificationError {
    Poisoned,
    Allocation,
}
impl AttributeNotifications {
    /// Actual `AttribSys82735430` empty key map35450..47c. General listeners
    /// remain a separate unreconstructed subscription route.
    pub(crate) const fn constructed() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
    /// `AddKeyNotification82738090`: nonunique insertion; equal keys survive.
    /// The target is `CameraAttribManager`'s actual Changed flag callback owner.
    pub(crate) fn register_camera(
        &self,
        key: u64,
        changed: &Arc<AtomicBool>,
    ) -> Result<(), AttributeNotificationError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| AttributeNotificationError::Poisoned)?;
        entries
            .try_reserve(1)
            .map_err(|_| AttributeNotificationError::Allocation)?;
        let position = entries.partition_point(|(existing, _)| *existing <= key);
        entries.insert(position, (key, Arc::downgrade(changed)));
        Ok(())
    }
}
