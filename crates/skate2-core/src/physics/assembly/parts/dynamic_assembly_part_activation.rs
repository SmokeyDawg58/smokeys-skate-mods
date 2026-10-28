//! Actual ascending Part activation loop, `Assembly::Activate` 0x82ae09b8.
use super::{DynamicAssemblySession, DynamicAssemblySimulationBackend};
use std::fmt;

pub(crate) trait DynamicAssemblyActivationBackend: DynamicAssemblySimulationBackend {
    type ActivationError;
    fn try_activation_state(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<u32, Self::ActivationError>;
    fn try_activate_frozen(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<(), Self::ActivationError>;
    fn try_clear_cooldown(
        &mut self,
        body: &Self::RigidBodyHandle,
    ) -> Result<(), Self::ActivationError>;
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DynamicAssemblyActivationError<E> {
    NotAttached,
    ForeignBackend,
    MissingBody { ordinal: usize },
    Backend { ordinal: usize, source: E },
}
impl<E: fmt::Debug> fmt::Display for DynamicAssemblyActivationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Assembly activation: {self:?}")
    }
}
impl<B: DynamicAssemblySimulationBackend, C> DynamicAssemblySession<B, C> {
    /// Host authority validation precedes the native loop. Later body failures
    /// retain all earlier activation/cooldown effects; there is no rollback.
    pub(crate) fn try_activate(
        &self,
        backend: &mut B,
    ) -> Result<(), DynamicAssemblyActivationError<B::ActivationError>>
    where
        B: DynamicAssemblyActivationBackend,
    {
        use DynamicAssemblyActivationError as E;
        let authority = self.attached_authority.ok_or(E::NotAttached)?;
        if authority != backend.authority() {
            return Err(E::ForeignBackend);
        }
        for ordinal in 0..self.part_attachment.parts.len() {
            let body = self.part_attachment.parts[ordinal]
                .body
                .as_ref()
                .ok_or(E::MissingBody { ordinal })?;
            let state = backend
                .try_activation_state(body)
                .map_err(|source| E::Backend { ordinal, source })?;
            if state & 7 == 2 {
                backend
                    .try_activate_frozen(body)
                    .map_err(|source| E::Backend { ordinal, source })?;
            }
            // Native 0x82ae0a0c reloads the Part body after ActivateRigidBody.
            let body = self.part_attachment.parts[ordinal]
                .body
                .as_ref()
                .ok_or(E::MissingBody { ordinal })?;
            backend
                .try_clear_cooldown(body)
                .map_err(|source| E::Backend { ordinal, source })?;
        }
        Ok(())
    }
}
impl<B: DynamicAssemblySimulationBackend, C> DynamicAssemblySession<B, C> {
    /// (any Part Frozen, every Part Frozen) by the same `state & 7 == 2` test.
    /// A port addition for the PC Object Dropper's riderless boards.
    pub(crate) fn try_part_frozen_counts(
        &self,
        backend: &mut B,
    ) -> Result<(bool, bool), DynamicAssemblyActivationError<B::ActivationError>>
    where
        B: DynamicAssemblyActivationBackend,
    {
        use DynamicAssemblyActivationError as E;
        let authority = self.attached_authority.ok_or(E::NotAttached)?;
        if authority != backend.authority() {
            return Err(E::ForeignBackend);
        }
        let (mut any, mut all) = (false, true);
        for ordinal in 0..self.part_attachment.parts.len() {
            let body = self.part_attachment.parts[ordinal]
                .body
                .as_ref()
                .ok_or(E::MissingBody { ordinal })?;
            let frozen = backend
                .try_activation_state(body)
                .map_err(|source| E::Backend { ordinal, source })?
                & 7
                == 2;
            any |= frozen;
            all &= frozen;
        }
        Ok((any, all))
    }
}

#[cfg(test)]
#[path = "dynamic_assembly_part_activation_tests.rs"]
mod tests;
