//! Same owner's final Update5 manager callback82e04f84; outer stores remain.
use super::{update5_controller::Update5BeforeWipeout, update5_managers::GrindManagerFillPolicy};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::wipeout_fill::{WipeoutFillError, WipeoutFillSource},
    physical_player_wipeout::WipeoutOwnerError,
    wipeout::WipeoutType,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum WipeoutFillRuntimeError {
    Poisoned,
    Owner(WipeoutOwnerError),
}
impl std::fmt::Display for WipeoutFillRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => f.write_str("Wipeout.Fill owner poisoned"),
            Self::Owner(error) => write!(f, "Wipeout.Fill request owner: {error:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum Update5WipeoutError {
    MissingPool,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Fill(WipeoutFillError<WipeoutFillRuntimeError>),
}
impl std::fmt::Display for Update5WipeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPool => f.write_str("Wipeout.Fill requires actual skater pool"),
            Self::Pair(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Wipeout.Fill continuation belongs to another pair"),
            Self::Fill(error) => error.fmt(f),
        }
    }
}
pub(crate) struct Update5AfterWipeoutFill {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) grind_policy: GrindManagerFillPolicy,
}
struct Source<'a>(&'a PhysicalPlayerAssemblyOwner);
impl WipeoutFillSource for Source<'_> {
    type Error = WipeoutFillRuntimeError;
    fn heavy_landing_requested(&mut self) -> Result<bool, Self::Error> {
        self.0
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_is_requested(WipeoutType::ALL[6])
            .map_err(Self::Error::Owner)
    }
    fn heavy_landing_speed(&mut self) -> Result<f32, Self::Error> {
        self.0
            .wipeout()
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .try_request_value(WipeoutType::ALL[6])
            .map_err(Self::Error::Owner)
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_continue_update5_wipeout(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update5BeforeWipeout,
    ) -> Result<Update5AfterWipeoutFill, Update5WipeoutError> {
        use Update5WipeoutError as E;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| {
                Arc::ptr_eq(owner, &before.owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        before
            .owner
            .phys_out()
            .lock()
            .map_err(|_| E::Fill(WipeoutFillError::Source(WipeoutFillRuntimeError::Poisoned)))?
            .try_fill_wipeout(&mut Source(&before.owner))
            .map_err(E::Fill)?;
        Ok(Update5AfterWipeoutFill {
            owner: before.owner,
            grind_policy: before.grind_policy,
        })
    }
}
