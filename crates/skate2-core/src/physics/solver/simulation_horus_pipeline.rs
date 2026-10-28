//! Checked reconstruction of the island-indirect Horus outer pass schedule.
//!
//! Raw `HorusPipelineIslandUsingIndirectJacobians` at
//! `0x82ae2858..0x82ae3190` snapshots one island's three table headers and the
//! runtime `Simulation::m_MaxIteration`, then executes Contact, Joint, and
//! Drive in that order for every pass. This module connects the three already
//! checked selected-island consumers. It also provides a deterministic serial
//! PC policy for the native all-island barrier without claiming that ascending
//! execution was the job system's runtime order.
//!
//! Contact batches are producer-neutral sealed manager data. Their historic
//! `NetworkContact` API names also admit CPU Assembly/ground records without
//! passing through a network queue. The shared consumer revalidates each
//! producer's table and RF body-side bindings before any solver effect.

use crate::simulation_contact_jacobian::network_preparation::{
    SimulationContactBatchIdentity, SimulationPreparedNetworkContactHorusSerialPlanError,
    SimulationPreparedNetworkContactIslandBatch, SimulationPreparedNetworkContactIslandHorusError,
    apply_prepared_network_contact_horus_serial_island,
    solve_prepared_network_contact_horus_record, try_prepare_network_contact_horus_island_sweep,
    try_prepare_network_contact_horus_serial_plan,
};
use crate::simulation_drive_jacobian::{
    SimulationPreparedDriveHorusSerialPlanError, SimulationPreparedDriveIslandBatch,
    SimulationPreparedDriveIslandHorusError, apply_prepared_drive_horus_serial_island,
    solve_prepared_drive_horus_island_record, try_prepare_drive_horus_island_sweep,
    try_prepare_drive_horus_serial_plan,
};
use crate::simulation_drive_pool::SimulationDrivePool;
use crate::simulation_island_map::SIMULATION_ISLAND_CAPACITY;
use crate::simulation_joint_preparation::{
    SimulationPreparedJointHorusSerialPlanError, SimulationPreparedJointIslandBatch,
    SimulationPreparedJointIslandHorusError, apply_prepared_joint_horus_serial_island,
    solve_prepared_joint_horus_record, try_prepare_joint_horus_island_sweep,
    try_prepare_joint_horus_serial_plan,
};
use crate::simulation_rigid_body_pool::{
    SimulationRigidBodyDynamicUpdatePhase, SimulationRigidBodyDynamicUpdatePhaseIdentity,
    SimulationRigidBodyFrameIdentity, SimulationRigidBodyPool, SimulationRigidBodyPoolAuthority,
};

/// One completed serial solver/integration boundary. The shared pool borrow
/// prevents freezing, a new frame, or body mutation before contact-spy export.
/// Construction is private to the actual solver path, not a caller assertion.
pub(crate) struct SimulationCompletedHorusFrame<'pool> {
    pool: &'pool SimulationRigidBodyPool,
    frame: SimulationRigidBodyFrameIdentity,
    phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
    had_active_bodies: bool,
    contact_batch: SimulationContactBatchIdentity,
}

impl SimulationCompletedHorusFrame<'_> {
    pub(crate) fn body_pool(&self) -> &SimulationRigidBodyPool {
        self.pool
    }

    pub(crate) fn matches_source(
        &self,
        pool: SimulationRigidBodyPoolAuthority,
        frame: SimulationRigidBodyFrameIdentity,
        phase: SimulationRigidBodyDynamicUpdatePhaseIdentity,
    ) -> bool {
        self.pool.authority() == pool && self.frame == frame && self.phase == phase
    }

    pub(crate) fn had_active_bodies(&self) -> bool {
        self.had_active_bodies
    }

    pub(crate) fn matches_contact_batch(&self, identity: &SimulationContactBatchIdentity) -> bool {
        self.contact_batch.same_as(identity)
    }
}

/// Typed pre-effect failure from one checked island-indirect Horus invocation.
///
/// The native solver assumes valid headers and has no returned-error path.
/// These failures are host ownership/allocation policy and all occur before the
/// first record or RF effect. Once execution starts it is infallible except for
/// panic; an exact earlier/current mutation prefix is deliberately retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedHorusIslandError {
    IslandOutOfRange { island: usize },
    Contact(SimulationPreparedNetworkContactIslandHorusError),
    Joint(SimulationPreparedJointIslandHorusError),
    Drive(SimulationPreparedDriveIslandHorusError),
}

/// Typed pre-effect refusal from the deterministic serial PC frame policy.
///
/// Native does not return these errors. They preserve host ownership and
/// allocation failures before every scheduled solver effect and before the
/// already-preflighted `DynamicUpdate` phase is consumed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SimulationPreparedHorusSerialFrameError {
    IslandCountOutOfRange { island_count: usize },
    Contact(SimulationPreparedNetworkContactHorusSerialPlanError),
    Joint(SimulationPreparedJointHorusSerialPlanError),
    Drive(SimulationPreparedDriveHorusSerialPlanError),
}

/// Executes one complete island-indirect Horus invocation.
///
/// `max_iterations` is the caller's one-time snapshot of the live unsigned
/// `Simulation::m_MaxIteration`; the shipped World initializes it to 25, but
/// the solver does not hard-code that value. After validating the fixed island
/// range, a zero value returns without inspecting any record/body occurrence,
/// matching the native post-header zero gate. For a nonzero value all three
/// families are fully validated and preallocated before pass zero. One
/// phase-owning transaction then reuses those exact indirect lists for every
/// Contact -> Joint -> Drive pass, reading then-live RF rows on every visit.
///
/// This function intentionally handles only one scheduled island job. Native
/// provides no dependency edge between island jobs, so cross-island ordering,
/// parallel RF ownership, the all-island barrier, and `DynamicUpdate` remain
/// separate evidence and ownership boundaries.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private checked Horus island integration boundary"
    )
)]
pub(crate) fn try_solve_prepared_horus_island(
    contact_batch: &mut SimulationPreparedNetworkContactIslandBatch<'_>,
    joint_batch: &mut SimulationPreparedJointIslandBatch<'_>,
    drive_batch: &mut SimulationPreparedDriveIslandBatch,
    island: usize,
    max_iterations: u32,
    drive_pool: &SimulationDrivePool,
    phase: &mut SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedHorusIslandError> {
    if island >= SIMULATION_ISLAND_CAPACITY {
        return Err(SimulationPreparedHorusIslandError::IslandOutOfRange { island });
    }
    if max_iterations == 0 {
        return Ok(());
    }

    let contact = try_prepare_network_contact_horus_island_sweep(contact_batch, island, phase)
        .map_err(SimulationPreparedHorusIslandError::Contact)?;
    let joint = try_prepare_joint_horus_island_sweep(joint_batch, island, phase)
        .map_err(SimulationPreparedHorusIslandError::Joint)?;
    let drive = try_prepare_drive_horus_island_sweep(drive_batch, island, drive_pool, phase)
        .map_err(SimulationPreparedHorusIslandError::Drive)?;

    phase
        .begin_horus_island_rf_transaction(contact, joint, drive)
        .execute_iterations(
            max_iterations,
            |_, prepared, rows| solve_prepared_network_contact_horus_record(prepared, rows),
            |_, prepared, rows| solve_prepared_joint_horus_record(prepared, rows),
            |_, prepared, rows| solve_prepared_drive_horus_island_record(prepared, rows),
        );
    Ok(())
}

/// Solves all scheduled islands under a deterministic serial PC policy and
/// then consumes the one already-preflighted `DynamicUpdate` phase.
///
/// Native `PopulateJobTree` (`0x82e0b410..0x82e0b67c`) constructs island jobs
/// in ascending index order, gives each the three compiler dependencies, and
/// makes `DynamicUpdate` depend on every island. It does not constrain runtime
/// island order or prove cross-island RF disjointness. This host boundary
/// therefore preflights every scheduled Contact/Joint/Drive occurrence before
/// effects, then executes islands serially in ascending construction-index
/// order. Within each island the recovered runtime-counted C -> J -> D order is
/// exact. Shared/repeated RF endpoints are preserved rather than rejected.
///
/// An empty Active phase follows the native zero-active job-tree bypass before
/// host island-count validation and performs no solver or integration work.
/// For an active phase, `1..=10` is a defensive host-policy range inferred
/// from observed public routes, not a native rejection contract. A zero host
/// iteration snapshot traverses no table entry and invokes no solver callback,
/// but still consumes `DynamicUpdate` once after the scheduled-island barrier.
/// Native reloads `m_MaxIteration` once per island invocation; using one stable
/// value for the complete host frame is an explicit snapshot policy.
///
/// All recoverable validation/allocation failures drop the phase inertly before
/// any record, RF, or body effect. Once execution begins there is no returned
/// failure. A solver panic retains its exact earlier/current mutation prefix
/// and skips `DynamicUpdate`; an integration panic retains its body/RF prefix.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "private prepared all-island Horus/DynamicUpdate boundary"
    )
)]
pub(crate) fn try_solve_prepared_horus_islands_serial_and_integrate_ordinary_finite(
    contact_batch: &mut SimulationPreparedNetworkContactIslandBatch<'_>,
    joint_batch: &mut SimulationPreparedJointIslandBatch<'_>,
    drive_batch: &mut SimulationPreparedDriveIslandBatch,
    island_count: usize,
    max_iterations: u32,
    drive_pool: &SimulationDrivePool,
    phase: SimulationRigidBodyDynamicUpdatePhase<'_>,
) -> Result<(), SimulationPreparedHorusSerialFrameError> {
    try_complete_prepared_horus_islands_serial(
        contact_batch,
        joint_batch,
        drive_batch,
        island_count,
        max_iterations,
        drive_pool,
        phase,
    )
    .map(|_| ())
}

/// Same recovered solver path, retaining a checked read-only post-integration
/// receipt for `ContactSpiesJob` (0x82e0b090). Native job dependency is at
/// 0x82e0b5e4. Empty Active phases perform no integration or spy work.
pub(crate) fn try_complete_prepared_horus_islands_serial<'pool>(
    contact_batch: &mut SimulationPreparedNetworkContactIslandBatch<'_>,
    joint_batch: &mut SimulationPreparedJointIslandBatch<'_>,
    drive_batch: &mut SimulationPreparedDriveIslandBatch,
    island_count: usize,
    max_iterations: u32,
    drive_pool: &SimulationDrivePool,
    mut phase: SimulationRigidBodyDynamicUpdatePhase<'pool>,
) -> Result<SimulationCompletedHorusFrame<'pool>, SimulationPreparedHorusSerialFrameError> {
    let frame_identity = phase.frame_identity();
    let phase_identity = phase.phase_identity();
    let had_active_bodies = phase.has_active_bodies();
    let contact_identity = contact_batch.identity();
    if !had_active_bodies {
        // Empty integration traverses no bodies and has no effects. It only
        // converts the already-exclusive phase into a read-only pool borrow.
        return Ok(SimulationCompletedHorusFrame {
            pool: phase.integrate_ordinary_finite(),
            frame: frame_identity,
            phase: phase_identity,
            had_active_bodies,
            contact_batch: contact_identity,
        });
    }
    if !(1..=SIMULATION_ISLAND_CAPACITY).contains(&island_count) {
        return Err(
            SimulationPreparedHorusSerialFrameError::IslandCountOutOfRange { island_count },
        );
    }
    if max_iterations == 0 {
        return Ok(SimulationCompletedHorusFrame {
            pool: phase.integrate_ordinary_finite(),
            frame: frame_identity,
            phase: phase_identity,
            had_active_bodies,
            contact_batch: contact_identity,
        });
    }

    let mut contact =
        try_prepare_network_contact_horus_serial_plan(contact_batch, island_count, &phase)
            .map_err(SimulationPreparedHorusSerialFrameError::Contact)?;
    let mut joint = try_prepare_joint_horus_serial_plan(joint_batch, island_count, &phase)
        .map_err(SimulationPreparedHorusSerialFrameError::Joint)?;
    let mut drive =
        try_prepare_drive_horus_serial_plan(drive_batch, island_count, drive_pool, &phase)
            .map_err(SimulationPreparedHorusSerialFrameError::Drive)?;

    for island in 0..island_count {
        for _pass in 0..max_iterations {
            apply_prepared_network_contact_horus_serial_island(&mut contact, island, &mut phase);
            apply_prepared_joint_horus_serial_island(&mut joint, island, &mut phase);
            apply_prepared_drive_horus_serial_island(&mut drive, island, &mut phase);
        }
    }
    Ok(SimulationCompletedHorusFrame {
        pool: phase.integrate_ordinary_finite(),
        frame: frame_identity,
        phase: phase_identity,
        had_active_bodies,
        contact_batch: contact_identity,
    })
}
