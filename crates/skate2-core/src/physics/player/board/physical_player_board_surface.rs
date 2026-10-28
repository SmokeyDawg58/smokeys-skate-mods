//! Actual board owner and sealed Island contact-list route for surface selection.
//! Separate output publication and next-frame dispatch remain caller stages.
use super::{
    SimulationAssemblyConstraintSession, physical_board_forces::PhysicalPlayerBoardForceError,
    physical_contact_list::SimulationPhysicalContactListError,
    physical_island::SimulationPhysicalBodyError,
};
use crate::simulation_assembly_backend::constraint_session::physical_island::{
    SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    skateboard_contact_surface::{
        CompletedSkateboardContacts, SkateboardContactFrameError, SkateboardContactSurfaceCvars,
        SkateboardContactSurfaceError, SkateboardContactSurfaceOwners,
    },
    skateboard_hook::SkateboardHookDefinition,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardSurfaceError<C> {
    Authority(PhysicalPlayerBoardForceError),
    MissingFrame,
    Frame(SkateboardContactFrameError),
    List(SimulationPhysicalContactListError),
    Poisoned,
    Selection(SkateboardContactSurfaceError<SimulationPhysicalBodyError, C>),
}
impl<C: fmt::Debug> fmt::Display for PhysicalPlayerBoardSurfaceError<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authority(error) => write!(f, "physical board surface authority: {error:?}"),
            Self::MissingFrame => f.write_str("physical board surface has no contact frame"),
            Self::Frame(error) => write!(f, "physical board surface frame: {error}"),
            Self::List(error) => write!(f, "physical board surface list: {error:?}"),
            Self::Poisoned => f.write_str("physical board surface state lock poisoned"),
            Self::Selection(error) => write!(f, "physical board surface selection: {error:?}"),
        }
    }
}

type World = SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;
struct Owners<'a>(&'a World);
impl SkateboardContactSurfaceOwners for Owners<'_> {
    type Error = SimulationPhysicalBodyError;
    fn owner(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, Self::Error> {
        self.0.island_body_owner_for_contacts(body)
    }
}

impl World {
    /// Source board postphysics winner/vote projection on the actual retained
    /// board runtime. Owner checks remain at the native per-contact read sites.
    pub(crate) fn try_update_board_contact_surface<C: SkateboardContactSurfaceCvars>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        cvars: &mut C,
    ) -> Result<(), PhysicalPlayerBoardSurfaceError<C::Error>> {
        use PhysicalPlayerBoardSurfaceError as E;
        // The runtime access also certifies this is the board member, including
        // empty frames where contact records cannot establish the body kind.
        let state = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Authority)?;
        let frame = Arc::clone(self.contact_spies.as_ref().ok_or(E::MissingFrame)?);
        // Validate completed/current even for empty input before the native
        // map[Body] side effect. The temporary borrow ends before GetCollisions.
        CompletedSkateboardContacts::try_new(
            &frame,
            &self.physical_contacts,
            &self.contacts,
            self.backend.collision_body_pool(),
            board,
        )
        .map_err(E::Frame)?;
        // 0x82b36ad0 obtains/creates the actual Body entry, including a valid
        // zero-spy frame where CreateContactSpyList produced no entry for it.
        self.try_get_physical_collisions(board).map_err(E::List)?;
        let completed = CompletedSkateboardContacts::try_new(
            &frame,
            &self.physical_contacts,
            &self.contacts,
            self.backend.collision_body_pool(),
            board,
        )
        .map_err(E::Frame)?;
        let mut owners = Owners(self);
        let mut state = state.lock().map_err(|_| E::Poisoned)?;
        state
            .try_update(&completed, &mut owners, cvars)
            .map_err(E::Selection)
    }
}
