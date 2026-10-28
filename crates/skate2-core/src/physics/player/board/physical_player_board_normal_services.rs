//! Actual retained board fields and late per-wheel inertia writes for the
//! contact-normal tail. Frame validation and tail scheduling belong to its caller.
use super::board_post_services::SkateboardPostCvars;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_contact_surface::normal_tail::SkateboardContactNormalServices,
};
use std::fmt;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardNormalServiceError<C> {
    Pool(SimulationSkaterPoolError),
    Body(SimulationRigidBodyPoolError),
    Cvar(C),
    InvalidWheel(usize),
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerBoardNormalServiceError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "board normal owner: {error}"),
            Self::Body(error) => write!(f, "board normal inertia: {error}"),
            Self::Cvar(error) => write!(f, "board normal cvar: {error:?}"),
            Self::InvalidWheel(wheel) => write!(f, "board normal Part {wheel} is not a wheel"),
        }
    }
}

pub(crate) struct PhysicalPlayerBoardNormalServices<'a, C> {
    world: &'a mut PhysicalAssemblyWorld,
    board: &'a SimulationSkaterAssemblyHandle,
    cvars: &'a mut C,
}
impl<'a, C> PhysicalPlayerBoardNormalServices<'a, C> {
    /// Borrows the already-selected actual world/board. No four-body preflight,
    /// constructor scheduling or eager cvar capture occurs here.
    pub(crate) fn new(
        world: &'a mut PhysicalAssemblyWorld,
        board: &'a SimulationSkaterAssemblyHandle,
        cvars: &'a mut C,
    ) -> Self {
        Self {
            world,
            board,
            cvars,
        }
    }
}
impl<C: SkateboardPostCvars> SkateboardContactNormalServices
    for PhysicalPlayerBoardNormalServices<'_, C>
{
    type Target = C::FreeWheelTarget;
    type Error = PhysicalPlayerBoardNormalServiceError<C::Error>;
    fn read_free_wheel_enabled(&mut self) -> Result<bool, Self::Error> {
        let construction = self
            .world
            .board_construction(self.board)
            .map_err(Self::Error::Pool)?;
        let fields = construction
            .and_then(|construction| construction.post_fields())
            .ok_or(Self::Error::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?;
        Ok(fields.free_wheel_enabled)
    }
    fn capture_free_wheel_drag(&mut self) -> Result<Self::Target, Self::Error> {
        self.cvars
            .capture_free_wheel_drag()
            .map_err(Self::Error::Cvar)
    }
    fn free_wheel_drag(&mut self, target: &Self::Target) -> Result<f32, Self::Error> {
        self.cvars
            .free_wheel_drag(target)
            .map_err(Self::Error::Cvar)
    }
    fn store_num_parts(&mut self, count: u8) -> Result<(), Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .with_board_post_fields(self.board, |fields| fields.num_parts = count)
            .map_err(Self::Error::Pool)
    }
    fn store_num_wheels(&mut self, count: u8) -> Result<(), Self::Error> {
        self.world
            .skaters
            .as_mut()
            .ok_or(Self::Error::Pool(SimulationSkaterPoolError::Uninitialized))?
            .with_board_post_fields(self.board, |fields| fields.num_wheels = count)
            .map_err(Self::Error::Pool)
    }
    fn set_wheel_angular_drag(&mut self, wheel: usize, value: f32) -> Result<(), Self::Error> {
        if wheel >= 4 {
            return Err(Self::Error::InvalidWheel(wheel));
        }
        // The caller has already rounded dragInput * 426fffff. Resolve only
        // this actual Part at the source site, then write its retained inertia.
        let body = self
            .world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_part_contact_spy_observation(self.world.backend(), wheel)
            .map_err(|error| Self::Error::Pool(SimulationSkaterPoolError::PhysicalPart(error)))?;
        self.world
            .backend
            .try_set_part_angular_drag(body, value)
            .map_err(Self::Error::Body)
    }
}
