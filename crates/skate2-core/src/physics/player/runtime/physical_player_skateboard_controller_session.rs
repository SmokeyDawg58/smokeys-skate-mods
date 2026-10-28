//! Selected actual-pair controller construction/removal. The caller schedules
//! this after Skeleton Init; no full `HiLOD` constructor or offboard loop claim.
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SimulationSkaterPoolError,
        SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_remaining_constructor_owners::{
        LandingOnDeck, OffboardFeetIk, OffboardJumping, OffboardLocation,
    },
    simulation_drive_pool::{
        SimulationDriveDynamics, SimulationDriveFrames, SimulationDriveHandle,
        SimulationDrivePoolError,
    },
    simulation_rigid_body_pool::{SimulationRigidBodyHandle, SimulationRigidBodyPoolAuthority},
    skateboard_assembly::SkateboardSimulationBackend,
    skateboard_controller::{SkateboardControllerError, SkateboardControllerServices},
};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

/// Only the actual runtime cvar collaborator remains external to this bridge.
/// The original default is true; no default or cached bool is supplied here.
pub(crate) trait SkateboardControllerCvars {
    type Error;
    type Target;
    fn capture_use_hand_drives(&mut self) -> Result<Self::Target, Self::Error>;
    fn use_hand_drives(&mut self, target: &Self::Target) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerControllerServiceError<C> {
    Cvar(C),
    Part(SimulationSkaterPoolError),
    Drive(SimulationDrivePoolError),
    ForeignSimulation,
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerControllerServiceError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cvar(e) => write!(f, "controller cvar: {e:?}"),
            Self::Part(e) => write!(f, "controller Part body: {e}"),
            Self::Drive(e) => write!(f, "controller drive pool: {e}"),
            Self::ForeignSimulation => f.write_str("controller captured a different simulation"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerControllerSessionError<C> {
    MissingSkaterPool,
    Pair(SimulationSkeletonJumpPairError),
    PoisonedController,
    PoisonedRemaining,
    PoisonedReckoning,
    Controller(SkateboardControllerError<PhysicalPlayerControllerServiceError<C>>),
    DrivesRemain { slots: [bool; 2] },
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerControllerSessionError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSkaterPool => f.write_str("controller skater pool unavailable"),
            Self::Pair(e) => write!(f, "controller pair: {e}"),
            Self::PoisonedController => f.write_str("retained controller lock poisoned"),
            Self::PoisonedRemaining => f.write_str("remaining physical constructor lock poisoned"),
            Self::PoisonedReckoning => f.write_str("physical processed owner lock poisoned"),
            Self::Controller(e) => write!(f, "controller stage: {e}"),
            Self::DrivesRemain { slots } => {
                write!(f, "controller removal left live handles: {slots:?}")
            }
        }
    }
}

/// Private token created from the actual session backend, never from a caller's
/// selected Island ID. Pair validation establishes both members in this domain.
struct CapturedSimulation(SimulationRigidBodyPoolAuthority);
struct Services<'a, C> {
    pool: &'a SimulationSkaterAssemblyPool,
    backend: &'a mut SkateboardSimulationBackend,
    board: &'a SimulationSkaterAssemblyHandle,
    skeleton: &'a SimulationSkaterAssemblyHandle,
    cvars: &'a mut C,
}
impl<C: SkateboardControllerCvars> SkateboardControllerServices for Services<'_, C> {
    type Error = PhysicalPlayerControllerServiceError<C::Error>;
    type Cvar = C::Target;
    type Simulation = CapturedSimulation;
    type Body = SimulationRigidBodyHandle;
    type Drive = SimulationDriveHandle;
    fn capture_use_hand_drives(&mut self) -> Result<Self::Cvar, Self::Error> {
        self.cvars
            .capture_use_hand_drives()
            .map_err(PhysicalPlayerControllerServiceError::Cvar)
    }
    fn use_hand_drives(&mut self, target: &Self::Cvar) -> Result<bool, Self::Error> {
        self.cvars
            .use_hand_drives(target)
            .map_err(PhysicalPlayerControllerServiceError::Cvar)
    }
    fn capture_simulation(&mut self) -> Result<Self::Simulation, Self::Error> {
        Ok(CapturedSimulation(self.backend.authority()))
    }
    fn skeleton_body(&mut self, part: usize) -> Result<Self::Body, Self::Error> {
        self.pool
            .try_controller_part_body(self.backend, self.skeleton, part)
            .map_err(PhysicalPlayerControllerServiceError::Part)
    }
    fn deck_body(&mut self) -> Result<Self::Body, Self::Error> {
        self.pool
            .try_controller_part_body(self.backend, self.board, 6)
            .map_err(PhysicalPlayerControllerServiceError::Part)
    }
    fn add_drive(
        &mut self,
        simulation: &Self::Simulation,
        deck: Self::Body,
        hand: Self::Body,
        frames: &Arc<RwLock<SimulationDriveFrames>>,
        dynamics: &Arc<RwLock<SimulationDriveDynamics>>,
    ) -> Result<Option<Self::Drive>, Self::Error> {
        if simulation.0 != self.backend.authority() {
            return Err(PhysicalPlayerControllerServiceError::ForeignSimulation);
        }
        self.backend
            .try_add_controller_drive(deck, hand, frames, dynamics)
            .map_err(PhysicalPlayerControllerServiceError::Drive)
    }
    fn remove_drive(
        &mut self,
        simulation: &Self::Simulation,
        drive: &Self::Drive,
    ) -> Result<(), Self::Error> {
        if simulation.0 != self.backend.authority() {
            return Err(PhysicalPlayerControllerServiceError::ForeignSimulation);
        }
        self.backend
            .try_remove_controller_drive(drive)
            .map_err(PhysicalPlayerControllerServiceError::Drive)
    }
}

impl PhysicalAssemblyWorld {
    pub(crate) fn try_construct_pair_skateboard_controller<C: SkateboardControllerCvars>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerControllerSessionError<C::Error>> {
        use PhysicalPlayerControllerSessionError as E;
        let pool = self.skaters.as_mut().ok_or(E::MissingSkaterPool)?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut controller = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::PoisonedController)?;
        controller
            .try_construct(
                &owner.inputs().vault,
                &mut Services {
                    pool,
                    backend: &mut self.backend,
                    board,
                    skeleton,
                    cvars,
                },
            )
            .map_err(E::Controller)?;
        drop(controller);
        // HiLOD calls82e043b4/43f0/4424/4458 follow actual controller return.
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::PoisonedReckoning)?
                .processed(),
        );
        let mut remaining = owner.remaining().lock().map_err(|_| E::PoisonedRemaining)?;
        remaining.feet_ik = Some(OffboardFeetIk::new(
            &owner,
            Arc::clone(&processed),
            skeleton.clone(),
        ));
        remaining.landing = Some(LandingOnDeck::new(
            &owner,
            processed,
            skeleton.clone(),
            &owner.inputs().vault,
        ));
        remaining.location = Some(OffboardLocation::new(&owner));
        remaining.jumping = Some(OffboardJumping::new(&owner));
        Ok(())
    }

    /// Runs native removal before the enclosing pair teardown. A late false
    /// cvar may leave live handles; this host refusal prevents losing ownership.
    pub(crate) fn try_remove_pair_skateboard_controller_drives<C: SkateboardControllerCvars>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerControllerSessionError<C::Error>> {
        use PhysicalPlayerControllerSessionError as E;
        let pool = self.skaters.as_mut().ok_or(E::MissingSkaterPool)?;
        let owner = pool
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let mut controller = owner
            .skateboard_controller()
            .lock()
            .map_err(|_| E::PoisonedController)?;
        controller
            .try_remove_drives(&mut Services {
                pool,
                backend: &mut self.backend,
                board,
                skeleton,
                cvars,
            })
            .map_err(E::Controller)?;
        let slots = [controller.drive(0).is_some(), controller.drive(1).is_some()];
        if slots.iter().any(|present| *present) {
            return Err(E::DrivesRemain { slots });
        }
        Ok(())
    }
}
