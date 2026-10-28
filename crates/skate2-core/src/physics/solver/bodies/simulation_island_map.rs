//! Shared occurrence-qualified body-island labels for constraint tables.
//!
//! Native code derives signed bytes by unchecked pointer subtraction. This
//! private host value keeps the recovered per-slot label semantics while
//! pairing every entry with the exact body occurrence and `StartFrame` identity.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared constraint map and persistent labels await the production session caller"
    )
)]

use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyFrameIdentity, SimulationRigidBodyObservation, SimulationRigidBodyPool,
    SimulationRigidBodyPoolAuthority,
};

pub(crate) const SIMULATION_ISLAND_CAPACITY: usize = 10;

#[path = "../../assembly/simulation_assembly_pairs.rs"]
pub(crate) mod assembly_pairs;
#[path = "simulation_island_state.rs"]
pub(crate) mod persistent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SimulationRigidBodyIslandAssignment {
    body: SimulationRigidBodyObservation,
    raw_label: i8,
}

/// Immutable occurrence-qualified body labels captured for one frame.
pub(crate) struct SimulationRigidBodyIslandMapSnapshot {
    body_pool: SimulationRigidBodyPoolAuthority,
    frame: SimulationRigidBodyFrameIdentity,
    slots: Box<[Option<SimulationRigidBodyIslandAssignment>]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationRigidBodyIslandMapBuildError {
    AllocationFailed {
        slot_count: usize,
    },
    ForeignBody {
        body: SimulationRigidBodyObservation,
    },
    SlotOutOfRange {
        body: SimulationRigidBodyObservation,
    },
    DuplicateSlot {
        body: SimulationRigidBodyObservation,
    },
}

impl SimulationRigidBodyIslandMapSnapshot {
    pub(crate) fn try_new(
        body_pool: &SimulationRigidBodyPool,
        frame: SimulationRigidBodyFrameIdentity,
        assignments: impl IntoIterator<Item = (SimulationRigidBodyObservation, i8)>,
    ) -> Result<Self, SimulationRigidBodyIslandMapBuildError> {
        let slot_count = body_pool.slot_capacity();
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(slot_count)
            .map_err(|_| SimulationRigidBodyIslandMapBuildError::AllocationFailed { slot_count })?;
        slots.resize(slot_count, None);
        for (body, raw_label) in assignments {
            if body.identity().authority() != body_pool.authority() {
                return Err(SimulationRigidBodyIslandMapBuildError::ForeignBody { body });
            }
            let index = body.identity().id().index() as usize;
            let Some(slot) = slots.get_mut(index) else {
                return Err(SimulationRigidBodyIslandMapBuildError::SlotOutOfRange { body });
            };
            if slot.is_some() {
                return Err(SimulationRigidBodyIslandMapBuildError::DuplicateSlot { body });
            }
            *slot = Some(SimulationRigidBodyIslandAssignment { body, raw_label });
        }
        Ok(Self {
            body_pool: body_pool.authority(),
            frame,
            slots: slots.into_boxed_slice(),
        })
    }

    #[must_use]
    pub(crate) const fn body_pool_authority(&self) -> SimulationRigidBodyPoolAuthority {
        self.body_pool
    }

    #[must_use]
    pub(crate) const fn frame_identity(&self) -> SimulationRigidBodyFrameIdentity {
        self.frame
    }

    #[must_use]
    pub(crate) fn assignment(
        &self,
        body: SimulationRigidBodyObservation,
    ) -> Option<(SimulationRigidBodyObservation, i8)> {
        let index = body.identity().id().index() as usize;
        self.slots
            .get(index)
            .copied()
            .flatten()
            .map(|assignment| (assignment.body, assignment.raw_label))
    }
}
