//! Ascending Part registration/cleanup bridge for the actual Island owner.
//! Skeleton Init reads Parts0..23 at 0x82b62188; board registration follows Set.
//! This module changes only the actual pool state prefix and returns an inert
//! observation. The caller retains/publishes the SAME Body/Entity/Part mapping;
//! no registry, Body identity, Part tag or native address is minted here.

use super::{
    DynamicAssemblyBodyOnlyPartSession, DynamicAssemblyPartAttachmentTransaction,
    DynamicAssemblySession,
};
use crate::{
    simulation_assembly_backend::SimulationAssemblyBackend,
    simulation_rigid_body_pool::{
        SimulationRigidBodyHandle, SimulationRigidBodyObservation, SimulationRigidBodyPoolError,
    },
};
use std::fmt;

#[cfg(test)]
#[path = "dynamic_assembly_part_contact_spy_tests.rs"]
mod tests;

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum DynamicAssemblyPartContactSpyError {
    NotAttached,
    BackendAuthorityMismatch,
    PartIndex { index: usize, count: usize },
    MissingPartBody { index: usize },
    Body(SimulationRigidBodyPoolError),
}

impl fmt::Display for DynamicAssemblyPartContactSpyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Part contact spy: {self:?}")
    }
}
impl std::error::Error for DynamicAssemblyPartContactSpyError {}

impl<M, JF: Copy, JL: Copy, C> DynamicAssemblySession<SimulationAssemblyBackend<M, JF, JL>, C> {
    pub(crate) fn part_contact_spy_handle(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyHandle, DynamicAssemblyPartContactSpyError> {
        part_handle(
            &self.part_attachment,
            self.attached_authority,
            backend,
            index,
        )
    }

    /// Read-only source ordinal to CURRENT actual body occurrence. This grants
    /// no removal/write capability and samples no inertia/motion payload.
    pub(crate) fn try_part_contact_spy_observation(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = self.part_contact_spy_handle(backend, index)?;
        backend
            .try_owned_body_observation(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }

    /// `RegisterSpyTarget` state |=8 (0x827b7874). Caller performs any checked
    /// host map reservation first, then stores Body/original Part tag AFTER this
    /// succeeds. Per-Part errors preserve earlier source-ordered registrations.
    pub(crate) fn try_register_part_contact_spy(
        &self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = self.part_contact_spy_handle(backend, index)?;
        backend
            .try_register_owned_body_contact_spy(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }

    /// `RemoveSpyTarget` state &=7 (0x827b5660), BEFORE caller target-map erasure.
    /// Enclosing cleanup must first apply its complete nonmutating detach guard,
    /// then visit source Part ordinals and finally remove the actual assembly.
    pub(crate) fn try_remove_part_contact_spy(
        &self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = self.part_contact_spy_handle(backend, index)?;
        backend
            .try_remove_owned_body_contact_spy(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }
}

fn part_handle<M, JF: Copy, JL: Copy>(
    attachment: &DynamicAssemblyPartAttachmentTransaction<SimulationAssemblyBackend<M, JF, JL>>,
    attached: Option<crate::simulation_rigid_body_pool::SimulationRigidBodyPoolAuthority>,
    backend: &SimulationAssemblyBackend<M, JF, JL>,
    index: usize,
) -> Result<SimulationRigidBodyHandle, DynamicAssemblyPartContactSpyError> {
    use DynamicAssemblyPartContactSpyError as Error;
    let authority = attached.ok_or(Error::NotAttached)?;
    if authority != backend.authority() {
        return Err(Error::BackendAuthorityMismatch);
    }
    let count = attachment.parts.len();
    let part = attachment
        .parts
        .get(index)
        .ok_or(Error::PartIndex { index, count })?;
    part.body.ok_or(Error::MissingPartBody { index })
}

// LWCollisionBody.AddToWorld82e50e18 visits these body-only Parts in the same
// ascending order. State writes use the existing backend kernels, not a second
// spy flag or inferred observation from the serialized publication.
impl<M, JF: Copy, JL: Copy>
    DynamicAssemblyBodyOnlyPartSession<SimulationAssemblyBackend<M, JF, JL>>
{
    pub(crate) fn try_part_contact_spy_observation(
        &self,
        backend: &SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = part_handle(&self.transaction, self.attached_authority, backend, index)?;
        backend
            .try_owned_body_observation(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }
    pub(crate) fn try_register_part_contact_spy(
        &self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = part_handle(&self.transaction, self.attached_authority, backend, index)?;
        backend
            .try_register_owned_body_contact_spy(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }
    pub(crate) fn try_remove_part_contact_spy(
        &self,
        backend: &mut SimulationAssemblyBackend<M, JF, JL>,
        index: usize,
    ) -> Result<SimulationRigidBodyObservation, DynamicAssemblyPartContactSpyError> {
        let body = part_handle(&self.transaction, self.attached_authority, backend, index)?;
        backend
            .try_remove_owned_body_contact_spy(body)
            .map_err(DynamicAssemblyPartContactSpyError::Body)
    }
}
