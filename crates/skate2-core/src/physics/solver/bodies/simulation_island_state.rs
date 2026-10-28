//! Persistent signed-byte body labels used by the manager's island map.
//!
//! StartFrame labels (82e0bd80), HookPool exclusion, and the single-player
//! assembly-pair partition (82e0e2d0). Counter reset belongs to the frame owner.

use super::{SimulationRigidBodyIslandAssignment, SimulationRigidBodyIslandMapSnapshot};
use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation, SimulationRigidBodyPool,
    SimulationRigidBodyPoolAuthority, SimulationRigidBodyPoolError,
};

#[cfg(test)]
#[path = "simulation_island_state_tests.rs"]
mod tests;

/// Labels persist by native body SLOT, even when an allocation is removed and
/// that slot is reused. Occurrence qualification happens only when publishing
/// a snapshot of current live bodies; it must not silently clear native bytes.
pub(crate) struct SimulationRigidBodyIslandState {
    body_pool: SimulationRigidBodyPoolAuthority,
    labels: Box<[i8]>,
    frame: Option<SimulationRigidBodyFrameIdentity>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodyIslandStateError {
    Allocation,
    BodyPoolNotPristine,
    ForeignPool,
    FrameUnavailable,
    StaleFrame,
    Body(SimulationRigidBodyPoolError),
}

impl SimulationRigidBodyIslandState {
    /// PartitionAssembliesInIslands82e0e2d0. A repeated island number from the
    /// modulo counter is deliberate: merging replaces ALL of B's label, not
    /// just a union-find component. All Parts receive the label, even static
    /// or newly awakened bodies. Caller runs this after ThawAsNecessary.
    pub(crate) fn try_partition_assemblies(
        &mut self,
        pool: &SimulationRigidBodyPool,
        frame: &super::assembly_pairs::SimulationAssemblyPairFrame<'_>,
    ) -> Result<(), SimulationRigidBodyIslandStateError> {
        self.check_pool(pool)?;
        if self.frame.is_none() || self.frame != pool.frame_identity() {
            return Err(SimulationRigidBodyIslandStateError::StaleFrame);
        }
        let mut members = std::collections::BTreeMap::new();
        let mut labels = std::collections::BTreeMap::new();
        let mut next = 0_i8;
        for &[a, b] in frame.pairs() {
            for member in [a, b] {
                for &body in member.bodies() {
                    pool.try_validate_observation(body)
                        .map_err(SimulationRigidBodyIslandStateError::Body)?;
                }
                members.insert(member.key(), member);
            }
            partition_pair(&mut labels, a.key(), b.key(), &mut next);
        }
        // Native iterates its pointer-keyed map, then every Part in order.
        for (key, member) in members {
            for &body in member.bodies() {
                self.labels[body.identity().id().index() as usize] = labels[&key];
            }
        }
        Ok(())
    }

    /// Manager constructor `0x82e0c668..0x82e0c6cc` initializes every label -1
    /// BEFORE adding its static body. Reject mid-session reinitialization;
    /// constructing this owner does not claim that `StartFrame` has executed.
    pub(crate) fn try_new(
        pool: &SimulationRigidBodyPool,
    ) -> Result<Self, SimulationRigidBodyIslandStateError> {
        if !pool
            .try_is_manager_construction_pristine()
            .map_err(SimulationRigidBodyIslandStateError::Body)?
        {
            return Err(SimulationRigidBodyIslandStateError::BodyPoolNotPristine);
        }
        let mut labels = Vec::new();
        labels
            .try_reserve_exact(pool.slot_capacity())
            .map_err(|_| SimulationRigidBodyIslandStateError::Allocation)?;
        labels.resize(pool.slot_capacity(), -1);
        Ok(Self {
            body_pool: pool.authority(),
            labels: labels.into_boxed_slice(),
            frame: None,
        })
    }

    /// `StartFrame` stores at `0x82e0be0c` and `0x82e0be38`: Active right-link
    /// order gets zero, THEN the caller's retained manager static body gets -1.
    /// No other slots are reset. Does not advance the pool timestep/frame;
    /// call after that separate existing step. Repeated native stores are
    /// allowed, not turned into an invented source error.
    pub(crate) fn try_start_frame_labels(
        &mut self,
        pool: &SimulationRigidBodyPool,
        manager_static_body: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyIslandStateError> {
        self.check_pool(pool)?;
        let frame = pool
            .frame_identity()
            .ok_or(SimulationRigidBodyIslandStateError::FrameUnavailable)?;
        // Host preflight avoids partial labels on an invalid manager binding.
        pool.try_validate_observation(manager_static_body)
            .map_err(SimulationRigidBodyIslandStateError::Body)?;
        pool.try_for_each_active_observation(|body| {
            self.labels[body.identity().id().index() as usize] = 0;
        })
        .map_err(SimulationRigidBodyIslandStateError::Body)?;
        self.labels[manager_static_body.identity().id().index() as usize] = -1;
        self.frame = Some(frame);
        Ok(())
    }

    /// `SetAssemblyIsland(assembly,-1)`, 0x82e0b038: source `HookPool` membership
    /// supplies each current Part body, without filtering its state. This
    /// private store is not a snapshot-to-authority or world-index mapping.
    /// Host preflight preserves earlier members on a later stale-body error.
    pub(crate) fn try_exclude_hook_body(
        &mut self,
        pool: &SimulationRigidBodyPool,
        body: SimulationRigidBodyObservation,
    ) -> Result<(), SimulationRigidBodyIslandStateError> {
        self.check_pool(pool)?;
        if self.frame.is_none() || self.frame != pool.frame_identity() {
            return Err(SimulationRigidBodyIslandStateError::StaleFrame);
        }
        pool.try_validate_observation(body)
            .map_err(SimulationRigidBodyIslandStateError::Body)?;
        self.labels[body.identity().id().index() as usize] = -1;
        Ok(())
    }

    /// Captures current allocation occurrences without changing the retained
    /// native slot labels. Snapshot qualification rejects removed/reused stale
    /// observations in downstream compilers. Same-frame mutation chronology
    /// remains the scheduler's responsibility, as with existing map snapshots.
    pub(crate) fn try_snapshot(
        &self,
        pool: &SimulationRigidBodyPool,
    ) -> Result<SimulationRigidBodyIslandMapSnapshot, SimulationRigidBodyIslandStateError> {
        self.check_pool(pool)?;
        let frame = self
            .frame
            .ok_or(SimulationRigidBodyIslandStateError::StaleFrame)?;
        if pool.frame_identity() != Some(frame) {
            return Err(SimulationRigidBodyIslandStateError::StaleFrame);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(self.labels.len())
            .map_err(|_| SimulationRigidBodyIslandStateError::Allocation)?;
        slots.resize(self.labels.len(), None);
        pool.try_for_each_allocated_observation(|body| {
            let index = body.identity().id().index() as usize;
            slots[index] = Some(SimulationRigidBodyIslandAssignment {
                body,
                raw_label: self.labels[index],
            });
        })
        .map_err(SimulationRigidBodyIslandStateError::Body)?;
        Ok(SimulationRigidBodyIslandMapSnapshot {
            body_pool: self.body_pool,
            frame,
            slots: slots.into_boxed_slice(),
        })
    }

    fn check_pool(
        &self,
        pool: &SimulationRigidBodyPool,
    ) -> Result<(), SimulationRigidBodyIslandStateError> {
        if pool.authority() != self.body_pool || pool.slot_capacity() != self.labels.len() {
            return Err(SimulationRigidBodyIslandStateError::ForeignPool);
        }
        Ok(())
    }
}

fn partition_pair(
    labels: &mut std::collections::BTreeMap<usize, i8>,
    first: usize,
    second: usize,
    next: &mut i8,
) {
    match (labels.get(&first).copied(), labels.get(&second).copied()) {
        (None, None) => {
            labels.insert(first, *next);
            labels.insert(second, *next);
            *next += 1;
            // Step_Collision3 827b40b0..b8 sets 10 for FLAG_Both.
            if *next == super::SIMULATION_ISLAND_CAPACITY as i8 {
                *next = 0;
            }
        }
        (Some(label), None) => {
            labels.insert(second, label);
        }
        (None, Some(label)) => {
            labels.insert(first, label);
        }
        (Some(a), Some(b)) if a != b => {
            for label in labels.values_mut() {
                if *label == b {
                    *label = a;
                }
            }
        }
        _ => {}
    }
}
