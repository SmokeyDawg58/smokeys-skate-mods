//! Selected controller construction (0x82dd02a0) and inactive Stop (0x82dd1710).
//! The enclosing owner binds the actual board/Skeleton/processed/Island pair.
//! Active release and offboard start remain explicit source-site continuations.

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "retained controller startup awaits full physical state scheduling"
    )
)]

use crate::physics_state_ids::{PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION};
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveDynamicsParams, SimulationDriveFrames,
        SimulationDriveHandle,
    },
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

#[cfg(test)]
#[path = "skateboard_controller_tests.rs"]
mod tests;

const CLASS: u64 = 0x7dea_05dc_70a1_ed7d;
const DEFAULT: u64 = 0xd7ed_bd36_2d7d_2152;
const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];
const NO_DRIVE: SimulationDriveDynamicsParams = SimulationDriveDynamicsParams {
    spring: 0.0,
    damping: 0.0,
    strength: 0.0,
    raw_type: 0,
};
const HARD: SimulationDriveDynamicsParams = SimulationDriveDynamicsParams {
    raw_type: 2,
    ..NO_DRIVE
};

/// Source `RetrievalProgressData::Reset` (0x82dc73e8), not sampled retrieval.
#[derive(Clone, Debug)]
pub(crate) struct ControllerRetrieval {
    pub(crate) start: SkeletonRootMatrix,
    pub(crate) target: SkeletonRootMatrix,
    pub(crate) current: SkeletonRootMatrix,
    pub(crate) time: f32,
    pub(crate) total_time: f32,
    pub(crate) parametric_time: f32,
    pub(crate) warped_parametric_time: f32,
}

/// Calls read the actual bound pair. Tokens capture source owners without
/// prefetching values. Admission must retain native capacity null distinctly
/// from host failure and insert these SAME mutable payload Arcs.
pub(crate) trait SkateboardControllerServices {
    type Error;
    type Cvar;
    type Simulation;
    type Body;
    type Drive;
    fn capture_use_hand_drives(&mut self) -> Result<Self::Cvar, Self::Error>;
    fn use_hand_drives(&mut self, target: &Self::Cvar) -> Result<bool, Self::Error>;
    fn capture_simulation(&mut self) -> Result<Self::Simulation, Self::Error>;
    fn skeleton_body(&mut self, part: usize) -> Result<Self::Body, Self::Error>;
    fn deck_body(&mut self) -> Result<Self::Body, Self::Error>;
    fn add_drive(
        &mut self,
        simulation: &Self::Simulation,
        deck: Self::Body,
        hand: Self::Body,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<Self::Drive>, Self::Error>;
    fn remove_drive(
        &mut self,
        simulation: &Self::Simulation,
        drive: &Self::Drive,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SkateboardControllerPhase {
    Fresh,
    AddDrives,
    InitHandDrives,
    Complete,
}

#[derive(Debug, PartialEq)]
pub(crate) enum SkateboardControllerError<E> {
    Service(E),
    NotFresh(SkateboardControllerPhase),
    NotComplete(SkateboardControllerPhase),
    ExistingDrive(usize),
    PoisonedDynamics(usize),
    HandInvariant { system_on: bool, hand: u32 },
    HandAlreadyHolding(u32),
    InvalidHandDrive(u32),
    SystemNotOn,
    InvalidState(u32),
    PendingLetGo,
    PendingStartController,
    MissingRetrieval,
}
impl<E: fmt::Display> fmt::Display for SkateboardControllerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Service(e) => write!(f, "controller service: {e}"),
            Self::NotFresh(p) => write!(f, "controller construction already started: {p:?}"),
            Self::NotComplete(p) => write!(f, "controller construction incomplete: {p:?}"),
            Self::ExistingDrive(i) => write!(f, "controller drive {i} already exists"),
            Self::PoisonedDynamics(i) => write!(f, "controller dynamics {i} poisoned"),
            Self::HandInvariant { system_on, hand } => write!(
                f,
                "controller hand invariant: system={system_on}, hand={hand}"
            ),
            Self::HandAlreadyHolding(hand) => {
                write!(f, "HoldSkateboard expected free hand sentinel, got {hand}")
            }
            Self::InvalidHandDrive(hand) => write!(f, "invalid selected hand drive {hand}"),
            Self::SystemNotOn => f.write_str("RequestDropBoard requires an active controller"),
            Self::InvalidState(state) => write!(f, "invalid board controller state {state}"),
            Self::PendingLetGo => f.write_str("actual LetGoOfSkateboard continuation unavailable"),
            Self::PendingStartController => {
                f.write_str("actual StartController continuation unavailable")
            }
            Self::MissingRetrieval => f.write_str("controller retrieval data not constructed"),
        }
    }
}

/// Allocation alone establishes no native controller fields. Construction is
/// attempted once; host failures retain their native prefix without rollback.
pub(crate) struct SkateboardController<D = SimulationDriveHandle> {
    occurrence: Arc<()>,
    phase: SkateboardControllerPhase,
    retrieval: Option<ControllerRetrieval>,
    attributes: Option<AttributeRuntimeInstance>,
    frames: [Option<Arc<RwLock<SimulationDriveFrames>>>; 2],
    dynamics: [Option<Arc<RwLock<SimulationDriveDynamics>>>; 2],
    drives: [Option<D>; 2],
    hand: Option<u32>,
    state: Option<u32>,
    system_on: Option<bool>,
}
impl<D> SkateboardController<D> {
    pub(crate) fn new() -> Self {
        Self {
            occurrence: Arc::new(()),
            phase: SkateboardControllerPhase::Fresh,
            retrieval: None,
            attributes: None,
            frames: [None, None],
            dynamics: [None, None],
            drives: [None, None],
            hand: None,
            state: None,
            system_on: None,
        }
    }
    pub(crate) fn occurrence(&self) -> &Arc<()> {
        &self.occurrence
    }
    pub(crate) fn phase(&self) -> SkateboardControllerPhase {
        self.phase
    }
    pub(crate) fn retrieval(&self) -> Option<&ControllerRetrieval> {
        self.retrieval.as_ref()
    }
    /// InitSkateboardHiding 0x82dc7770..77b8: reset precedes the live
    /// Processed deck read. Preserve that reset even if the host read fails.
    pub(crate) fn try_initialize_hiding_retrieval<E>(
        &mut self,
        deck: impl FnOnce() -> Result<SkeletonRootMatrix, E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        self.try_reset_retrieval()?;
        let deck = deck().map_err(SkateboardControllerError::Service)?;
        let retrieval = self.retrieval.as_mut().expect("reset validated retrieval");
        for index in 0..3 {
            retrieval.start.set_column(index, deck.columns()[index]);
        }
        retrieval.time = 0.0;
        retrieval.start.set_column(3, deck.columns()[3]);
        Ok(())
    }

    pub(crate) fn attributes(&self) -> Option<&AttributeRuntimeInstance> {
        self.attributes.as_ref()
    }
    pub(crate) fn frames(&self, slot: usize) -> Option<&Arc<RwLock<SimulationDriveFrames>>> {
        self.frames.get(slot).and_then(Option::as_ref)
    }
    pub(crate) fn dynamics(&self, slot: usize) -> Option<&Arc<RwLock<SimulationDriveDynamics>>> {
        self.dynamics.get(slot).and_then(Option::as_ref)
    }
    pub(crate) fn drive(&self, slot: usize) -> Option<&D> {
        self.drives.get(slot).and_then(Option::as_ref)
    }
    pub(crate) fn hand(&self) -> Option<u32> {
        self.hand
    }
    pub(crate) fn state(&self) -> Option<u32> {
        self.state
    }
    pub(crate) fn system_on(&self) -> Option<bool> {
        self.system_on
    }

    pub(crate) fn try_construct<S: SkateboardControllerServices<Drive = D>>(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        services: &mut S,
    ) -> Result<(), SkateboardControllerError<S::Error>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Fresh {
            return Err(Error::NotFresh(self.phase));
        }
        self.retrieval = Some(ControllerRetrieval {
            start: SkeletonRootMatrix::known(IDENTITY),
            target: SkeletonRootMatrix::known(IDENTITY),
            current: SkeletonRootMatrix::known(IDENTITY),
            time: 0.0,
            total_time: 0.0,
            parametric_time: 0.0,
            warped_parametric_time: 0.0,
        });
        self.attributes = Some(vault.instance(CLASS, DEFAULT));
        for slot in &mut self.frames {
            *slot = Some(Arc::new(RwLock::new(SimulationDriveFrames {
                quaternion_a: [0.0, 0.0, 0.0, 1.0],
                position_a: [0.0; 4],
                quaternion_b: [0.0, 0.0, 0.0, 1.0],
                position_b: [0.0; 4],
            })));
        }
        for slot in &mut self.dynamics {
            *slot = Some(Arc::new(RwLock::new(SimulationDriveDynamics {
                linear: NO_DRIVE,
                angular: NO_DRIVE,
            })));
        }
        self.hand = Some(2);
        // Actual pointer binding is owned by the enclosing pair/service owner.
        self.state = Some(0);
        self.system_on = Some(false);
        self.drives = [None, None];
        self.phase = SkateboardControllerPhase::AddDrives;
        let target = services.capture_use_hand_drives().map_err(Error::Service)?;
        if services.use_hand_drives(&target).map_err(Error::Service)? {
            for (slot, drive) in self.drives.iter().enumerate() {
                if drive.is_some() {
                    return Err(Error::ExistingDrive(slot));
                }
            }
            self.phase = SkateboardControllerPhase::InitHandDrives;
            let target = services.capture_use_hand_drives().map_err(Error::Service)?;
            if services.use_hand_drives(&target).map_err(Error::Service)? {
                let simulation = services.capture_simulation().map_err(Error::Service)?;
                for slot in 0..2 {
                    let mut dynamics = self.dynamics[slot]
                        .as_ref()
                        .expect("constructor dynamics")
                        .write()
                        .map_err(|_| Error::PoisonedDynamics(slot))?;
                    // 0x82dc7568..758c: angular first, then linear, then pointer.
                    dynamics.angular.spring = 0.0;
                    dynamics.angular.raw_type = 2;
                    dynamics.angular.damping = 0.0;
                    dynamics.angular.strength = 0.0;
                    dynamics.linear = HARD;
                    self.drives[slot] = None;
                }
                for (slot, part) in [3, 7].into_iter().enumerate() {
                    let hand = services.skeleton_body(part).map_err(Error::Service)?;
                    let deck = services.deck_body().map_err(Error::Service)?;
                    self.drives[slot] = services
                        .add_drive(
                            &simulation,
                            deck,
                            hand,
                            self.frames[slot].as_ref().expect("constructor frames"),
                            self.dynamics[slot].as_ref().expect("constructor dynamics"),
                        )
                        .map_err(Error::Service)?;
                }
            }
        }
        self.phase = SkateboardControllerPhase::Complete;
        Ok(())
    }

    /// `RemoveDrives` (0x82dc7490), also usable to clean a failed constructor's
    /// admitted prefix. The source cvar may skip removal; the enclosing teardown
    /// must check remaining handles before removing either body's ownership.
    /// This is not an automatic Rust destructor or implicit rollback.
    pub(crate) fn try_remove_drives<S: SkateboardControllerServices<Drive = D>>(
        &mut self,
        services: &mut S,
    ) -> Result<(), SkateboardControllerError<S::Error>> {
        use SkateboardControllerError as Error;
        if self.phase == SkateboardControllerPhase::Fresh {
            return Err(Error::NotComplete(self.phase));
        }
        let target = services.capture_use_hand_drives().map_err(Error::Service)?;
        if services.use_hand_drives(&target).map_err(Error::Service)? {
            let simulation = services.capture_simulation().map_err(Error::Service)?;
            for drive in &mut self.drives {
                if let Some(handle) = drive.as_ref() {
                    services
                        .remove_drive(&simulation, handle)
                        .map_err(Error::Service)?;
                    *drive = None;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn try_stop<E>(&mut self) -> Result<(), SkateboardControllerError<E>> {
        self.try_stop_with(|_| Err(SkateboardControllerError::PendingLetGo))
    }

    /// StopController82dd1710 calls LetGo while state/system are still live,
    /// then publishes state zero and system-off before its hand invariant.
    pub(crate) fn try_stop_with<E>(
        &mut self,
        let_go: impl FnOnce(&mut Self) -> Result<(), SkateboardControllerError<E>>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotComplete(self.phase));
        }
        if self.system_on.expect("constructed system flag") {
            if self.state.expect("constructed state") != 0 {
                let_go(self)?;
                self.state = Some(0);
            }
            self.system_on = Some(false);
        }
        let hand = self.hand.expect("constructed hand");
        let system_on = self.system_on.expect("constructed system flag");
        if system_on == (hand == 2) {
            return Err(Error::HandInvariant { system_on, hand });
        }
        Ok(())
    }

    /// RequestHideBoard82dd17a0: initialize on every state except HIDING.
    pub(crate) fn try_request_hide_with<E>(&mut self,
        initialize:impl FnOnce(&mut Self)->Result<(),SkateboardControllerError<E>>)
        ->Result<(),SkateboardControllerError<E>>{
        if self.phase!=SkateboardControllerPhase::Complete{return Err(SkateboardControllerError::NotComplete(self.phase));}
        if !self.system_on.expect("constructed system flag"){return Err(SkateboardControllerError::SystemNotOn);}
        if self.state!=Some(3){initialize(self)?;self.state=Some(3);}
        Ok(())
    }

    /// RequestDropBoard82dd1808: collaborators run before their state stores.
    /// One callback takes the branch kind to avoid borrowing the same World twice.
    pub(crate) fn try_request_drop_with<E>(
        &mut self,
        mut release: impl FnMut(&mut Self, bool) -> Result<(), SkateboardControllerError<E>>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotComplete(self.phase));
        }
        if !self.system_on.expect("constructed system flag") {
            return Err(Error::SystemNotOn);
        }
        match self.state.expect("constructed state") {
            0 | 1 => {
                release(self, false)?;
                self.state = Some(2);
            }
            2 | 3 => {}
            4 => {
                release(self, true)?;
                self.state = Some(3);
            }
            state => return Err(Error::InvalidState(state)),
        }
        Ok(())
    }

    pub(crate) fn try_enter_physics_state<E>(
        &mut self,
        state: i32,
    ) -> Result<(), SkateboardControllerError<E>> {
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(SkateboardControllerError::NotComplete(self.phase));
        }
        if (PHYSICS_STATE_OFFBOARD..=PHYSICS_STATE_OFFBOARD_LOCOMOTION).contains(&state) {
            Err(SkateboardControllerError::PendingStartController)
        } else {
            self.try_stop()
        }
    }

    /// `StartController` 0x82dd1678 and the retained stores in its Hold call
    /// 0x82dcc738. The callbacks keep board animation, SetStandard, collision,
    /// processed reads, and hand-drive effects with their actual owners.
    pub(crate) fn try_start_with<E>(
        &mut self,
        flags: impl FnMut() -> Result<u32, E>,
        prepare: impl FnMut() -> Result<(), E>,
        hands: impl FnMut() -> Result<u32, E>,
        enable: impl FnMut(&mut Self, u32) -> Result<(), E>,
        let_go: impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        self.try_start_or_hold_with(true, flags, prepare, hands, enable, let_go)
    }
    fn try_hold_with<E>(
        &mut self,
        mut prepare_hold: impl FnMut() -> Result<(), E>,
        mut processed_flags_8cc: impl FnMut() -> Result<u32, E>,
        mut enable_hand_drive: impl FnMut(&mut Self, u32) -> Result<(), E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        prepare_hold().map_err(Error::Service)?;
        if self.hand.expect("constructed hand") != 2 {
            return Err(Error::HandAlreadyHolding(
                self.hand.expect("constructed hand"),
            ));
        }
        let hand_flags = processed_flags_8cc().map_err(Error::Service)?;
        self.state = Some(1);
        let hand = (!hand_flags >> 3) & 1;
        self.hand = Some(hand);
        enable_hand_drive(self, hand).map_err(Error::Service)?;
        Ok(())
    }
    pub(crate) fn try_active_retrieval_mut<E>(
        &mut self,
    ) -> Result<&mut ControllerRetrieval, SkateboardControllerError<E>> {
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(SkateboardControllerError::NotComplete(self.phase));
        }
        self.retrieval
            .as_mut()
            .ok_or(SkateboardControllerError::MissingRetrieval)
    }
    pub(crate) fn try_store_active_state<E>(
        &mut self,
        state: u32,
    ) -> Result<(), SkateboardControllerError<E>> {
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(SkateboardControllerError::NotComplete(self.phase));
        }
        self.state = Some(state);
        Ok(())
    }
    pub(crate) fn try_reset_active_retrieval<E>(
        &mut self,
    ) -> Result<(), SkateboardControllerError<E>> {
        self.try_reset_retrieval()
    }
    pub(crate) fn try_start_or_hold_with<E>(
        &mut self,
        start: bool,
        mut processed_flags_8d0: impl FnMut() -> Result<u32, E>,
        prepare_hold: impl FnMut() -> Result<(), E>,
        processed_flags_8cc: impl FnMut() -> Result<u32, E>,
        enable_hand_drive: impl FnMut(&mut Self, u32) -> Result<(), E>,
        mut let_go_of_skateboard: impl FnMut(&mut Self) -> Result<(), E>,
    ) -> Result<(), SkateboardControllerError<E>> {
        use SkateboardControllerError as Error;
        if self.phase != SkateboardControllerPhase::Complete {
            return Err(Error::NotComplete(self.phase));
        }
        if !start {
            return self.try_hold_with(prepare_hold, processed_flags_8cc, enable_hand_drive);
        }
        if self.system_on.expect("constructed system flag") {
            return Ok(());
        }
        let flags = processed_flags_8d0().map_err(Error::Service)?;
        if flags & 0xc00 != 0 {
            if self.state.expect("constructed state") != 2 {
                let_go_of_skateboard(self).map_err(Error::Service)?;
                self.state = Some(2);
            }
        } else if self.state.expect("constructed state") != 1 {
            self.try_hold_with(prepare_hold, processed_flags_8cc, enable_hand_drive)?;
            // StartController stores state 1 again after HoldSkateboard returns.
            self.state = Some(1);
        }
        self.system_on = Some(true);
        Ok(())
    }

    /// SkateboardController::Reset 0x82dd1d18..34 calls StopController,
    /// then resets the existing RetrievalProgressData in place. An active
    /// nonzero state still stops at try_stop's typed LetGo boundary.
    pub(crate) fn try_reset_after_teleport<E>(
        &mut self,
    ) -> Result<(), SkateboardControllerError<E>> {
        self.try_stop()?;
        self.try_reset_retrieval()
    }

    fn try_reset_retrieval<E>(&mut self) -> Result<(), SkateboardControllerError<E>> {
        let retrieval = self
            .retrieval
            .as_mut()
            .ok_or(SkateboardControllerError::MissingRetrieval)?;
        retrieval.start = SkeletonRootMatrix::known(IDENTITY);
        retrieval.target = SkeletonRootMatrix::known(IDENTITY);
        retrieval.current = SkeletonRootMatrix::known(IDENTITY);
        retrieval.time = 0.0;
        retrieval.total_time = 0.0;
        retrieval.parametric_time = 0.0;
        retrieval.warped_parametric_time = 0.0;
        Ok(())
    }
}

#[path = "skateboard_controller_inactive_update.rs"]
pub(crate) mod inactive_update;

#[path = "skateboard_controller_hand_drive.rs"]
pub(crate) mod hand_drive;

#[path = "skateboard_controller_drive_frames.rs"]
pub(crate) mod drive_frames;

#[path = "skateboard_controller_righting.rs"]
pub(crate) mod righting;

#[path = "skateboard_controller_throw.rs"]
pub(crate) mod throw;
