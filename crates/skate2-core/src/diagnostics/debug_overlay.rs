//! Concrete empty debug-class allocation and application singleton publication.
//! `Construct8272b300`, `InitApp827352a8..c4`, `Get82581e40`.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "selected application bootstrap awaits live app scheduling"
    )
)]
use std::{
    mem::MaybeUninit,
    sync::{Arc, Mutex},
    thread::ThreadId,
};

/// PDB class size1, no members. The source allocates without a constructor.
#[derive(Debug)]
pub(crate) struct DebugOverlay {
    _storage: MaybeUninit<u8>,
}

/// Application service retained by the host session, shared by every skater.
/// These holder fields are separate from the native one-byte object.
#[derive(Clone, Debug, Default)]
pub(crate) struct DebugOverlaySlot {
    holder: Arc<Mutex<DebugOverlayHolder>>,
}
#[derive(Debug, Default)]
struct DebugOverlayHolder {
    instance: Option<Arc<DebugOverlay>>,
    creation_thread: Option<ThreadId>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DebugOverlayError {
    AlreadyConstructed,
    Missing,
    Poisoned,
}
impl std::fmt::Display for DebugOverlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AlreadyConstructed => "debug overlay singleton is already constructed",
            Self::Missing => "debug overlay singleton has not been constructed",
            Self::Poisoned => "debug overlay singleton holder lock poisoned",
        })
    }
}

impl DebugOverlaySlot {
    /// Source category Singleton, name `Sk8::Debug::TheDebugOverlay`, size1,
    /// alignment1, flags0x101, offset0. Native allocator policy is host-owned.
    pub(crate) fn try_construct(&self) -> Result<(), DebugOverlayError> {
        let mut holder = self
            .holder
            .lock()
            .map_err(|_| DebugOverlayError::Poisoned)?;
        if holder.instance.is_some() {
            return Err(DebugOverlayError::AlreadyConstructed);
        }
        holder.instance = Some(Arc::new(DebugOverlay {
            _storage: MaybeUninit::uninit(),
        }));
        // InitApp samples the thread only AFTER the allocation is published.
        holder.creation_thread = Some(std::thread::current().id());
        Ok(())
    }

    /// Native Get checks the current instance; it does not check thread identity
    /// or create an instance on demand. No renderer service is implied.
    pub(crate) fn require(&self) -> Result<Arc<DebugOverlay>, DebugOverlayError> {
        self.holder
            .lock()
            .map_err(|_| DebugOverlayError::Poisoned)?
            .instance
            .clone()
            .ok_or(DebugOverlayError::Missing)
    }
}

impl<M, C>
    crate::simulation_assembly_backend::constraint_session::SimulationAssemblyConstraintSession<
        M,
        C,
    >
{
    pub(crate) fn try_initialize_debug_overlay(&mut self) -> Result<(), DebugOverlayError> {
        self.debug_overlay.try_construct()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_empty_allocation_is_required_and_duplicate_creation_preserves_identity() {
        assert_eq!(std::mem::size_of::<DebugOverlay>(), 1);
        let slot = DebugOverlaySlot::default();
        let captured_holder = slot.clone();
        assert!(matches!(slot.require(), Err(DebugOverlayError::Missing)));
        assert_eq!(slot.holder.lock().unwrap().creation_thread, None);
        slot.try_construct().unwrap();
        let original = captured_holder.require().unwrap();
        assert_eq!(
            slot.holder.lock().unwrap().creation_thread,
            Some(std::thread::current().id())
        );
        assert_eq!(
            slot.try_construct(),
            Err(DebugOverlayError::AlreadyConstructed)
        );
        assert!(Arc::ptr_eq(&original, &slot.require().unwrap()));
    }
}
