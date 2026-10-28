//! Retained semantic payloads for one shared Assembly simulation backend.
//!
//! Native Parts retain definition inertia pointers (0x82ae9820), and Joints
//! retain definition limits pointers (0x82ae4358). Diagnostic immutable inputs
//! and runtime mutable board/skeleton inputs share the same pool identity.
//! Conversion retains the owner without sampling it. Writes remain private to
//! the existing occurrence- and authority-checked pool operations.

use crate::{
    simulation_joint_pool::{
        SimulationJointHandle, SimulationJointPool, SimulationJointPoolError,
        SimulationMutableJointLimits,
    },
    simulation_rigid_body_pool::{
        SimulationInertia, SimulationMutableInertia, SimulationRigidBodyHandle,
        SimulationRigidBodyPool, SimulationRigidBodyPoolError,
    },
};
use std::sync::Arc;

/// Retained Part inertia; cloning never copies a mutable payload value.
#[derive(Clone, Debug)]
pub struct SimulationAssemblyInertia(InertiaStorage);

#[derive(Clone, Debug)]
enum InertiaStorage {
    Immutable(Arc<SimulationInertia>),
    Mutable(SimulationMutableInertia),
    SourceRouted(crate::simulation_assembly_backend::SerialisedAssemblyRoutedInertia),
}

impl From<Arc<SimulationInertia>> for SimulationAssemblyInertia {
    fn from(value: Arc<SimulationInertia>) -> Self {
        Self(InertiaStorage::Immutable(value))
    }
}

impl SimulationAssemblyInertia {
    pub(crate) fn source_routed(
        value: crate::simulation_assembly_backend::SerialisedAssemblyRoutedInertia,
    ) -> Self {
        Self(InertiaStorage::SourceRouted(value))
    }

    pub(crate) fn as_source_routed(
        &self,
    ) -> Option<&crate::simulation_assembly_backend::SerialisedAssemblyRoutedInertia> {
        match &self.0 {
            InertiaStorage::SourceRouted(value) => Some(value),
            _ => None,
        }
    }
    pub(crate) fn mutable(value: SimulationMutableInertia) -> Self {
        Self(InertiaStorage::Mutable(value))
    }

    /// Original immutable allocation, when this payload uses immutable storage.
    #[must_use]
    pub fn as_immutable(&self) -> Option<&Arc<SimulationInertia>> {
        match &self.0 {
            InertiaStorage::Immutable(value) => Some(value),
            InertiaStorage::Mutable(_) | InertiaStorage::SourceRouted(_) => None,
        }
    }

    /// Read a current value without exposing a writable owner or lock guard.
    ///
    /// # Errors
    ///
    /// Returns `InertiaLockPoisoned` if mutable storage was poisoned.
    pub fn try_read(&self) -> Result<SimulationInertia, SimulationRigidBodyPoolError> {
        match &self.0 {
            InertiaStorage::Immutable(value) => Ok(**value),
            InertiaStorage::Mutable(value) => value.try_read(),
            InertiaStorage::SourceRouted(value) => Ok(**value.runtime_inertia().semantic()),
        }
    }

    pub(crate) fn try_add(
        &self,
        pool: &mut SimulationRigidBodyPool,
        transform: [f32; 16],
        body_state: u32,
    ) -> Result<Option<SimulationRigidBodyHandle>, SimulationRigidBodyPoolError> {
        match &self.0 {
            InertiaStorage::Immutable(value) => {
                pool.try_add_rigid_body(transform, Some(value), body_state)
            }
            InertiaStorage::Mutable(value) => {
                pool.try_add_rigid_body_mutable(transform, Some(value), body_state)
            }
            InertiaStorage::SourceRouted(_) => {
                unreachable!("source Add requires backend-owned route dispatch")
            }
        }
    }

    /// Current full tensor only when its producer supplied an explicit W lane.
    /// Immutable or generic XYZ-only owners return None, not an invented W.
    ///
    /// # Errors
    ///
    /// Returns `InertiaLockPoisoned` if mutable storage was poisoned.
    pub fn try_read_packed(&self) -> Result<Option<[f32; 4]>, SimulationRigidBodyPoolError> {
        match &self.0 {
            InertiaStorage::Immutable(_) | InertiaStorage::SourceRouted(_) => Ok(None),
            InertiaStorage::Mutable(value) => value.try_read_packed(),
        }
    }
}

/// Retained Joint limits with the same lifetime and read policy as Part inertia.
#[derive(Clone, Debug)]
pub struct SimulationAssemblyJointLimits<L>(JointLimitsStorage<L>);

#[derive(Clone, Debug)]
enum JointLimitsStorage<L> {
    Immutable(Arc<L>),
    Mutable(SimulationMutableJointLimits<L>),
}

impl<L> From<Arc<L>> for SimulationAssemblyJointLimits<L> {
    fn from(value: Arc<L>) -> Self {
        Self(JointLimitsStorage::Immutable(value))
    }
}

impl<L: Copy> SimulationAssemblyJointLimits<L> {
    pub(crate) fn mutable(value: SimulationMutableJointLimits<L>) -> Self {
        Self(JointLimitsStorage::Mutable(value))
    }

    /// Original immutable allocation, when this payload uses immutable storage.
    #[must_use]
    pub fn as_immutable(&self) -> Option<&Arc<L>> {
        match &self.0 {
            JointLimitsStorage::Immutable(value) => Some(value),
            JointLimitsStorage::Mutable(_) => None,
        }
    }

    /// Read a current value without exposing a writable owner or lock guard.
    ///
    /// # Errors
    ///
    /// Returns `LimitsLockPoisoned` if mutable storage was poisoned.
    pub fn try_read(&self) -> Result<L, SimulationJointPoolError> {
        match &self.0 {
            JointLimitsStorage::Immutable(value) => Ok(**value),
            JointLimitsStorage::Mutable(value) => value.try_read(),
        }
    }

    pub(crate) fn try_add<F: Copy>(
        &self,
        pool: &mut SimulationJointPool<F, L>,
        parent: SimulationRigidBodyHandle,
        child: SimulationRigidBodyHandle,
        frames: &Arc<F>,
    ) -> Result<Option<SimulationJointHandle>, SimulationJointPoolError> {
        match &self.0 {
            JointLimitsStorage::Immutable(value) => {
                pool.try_add_joint(parent, child, frames, value)
            }
            JointLimitsStorage::Mutable(value) => {
                pool.try_add_joint_mutable_limits(parent, child, frames, value)
            }
        }
    }
}
