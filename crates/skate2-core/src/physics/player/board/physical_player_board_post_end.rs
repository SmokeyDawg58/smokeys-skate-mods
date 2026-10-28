//! Body timer -> SAME truck dynamics -> SAME truck frames3749c..374c8.
use super::physical_board_forces::PhysicalPlayerBoardForceError;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_construction::SimulationConstructedBoardError,
    },
    physical_player_assembly::PhysicalAssemblyWorld,
    skateboard_attributes::SkateboardAttributeError,
    skateboard_truck_drives::{SkateboardTruckDriveError, SkateboardTruckDriveNumeric},
};
use std::{convert::Infallible, fmt, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardPostEndError<N> {
    Owner(PhysicalPlayerBoardForceError),
    Pool(SimulationSkaterPoolError),
    Construction(SimulationConstructedBoardError),
    Refused(&'static str),
    Dynamics(SkateboardTruckDriveError<SkateboardAttributeError, Infallible, Infallible>),
    Frames(SkateboardTruckDriveError<Infallible, N, Infallible>),
}
impl<N: fmt::Debug> fmt::Display for PhysicalPlayerBoardPostEndError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "post end owner: {e:?}"),
            Self::Pool(e) => write!(f, "post end pool: {e}"),
            Self::Construction(e) => write!(f, "post end construction: {e}"),
            Self::Refused(e) => write!(f, "post end: {e}"),
            Self::Dynamics(e) => write!(f, "post end dynamics: {e}"),
            Self::Frames(e) => write!(f, "post end frames: {e:?}"),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_finish_board_postphysics<N: SkateboardTruckDriveNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerBoardPostEndError<N::Error>> {
        use PhysicalPlayerBoardPostEndError as E;
        let source = self
            .try_with_board_force_runtime(board, |runtime| Arc::clone(&runtime.contact_surface))
            .map_err(E::Owner)?;
        let frame = self
            .contact_spies
            .clone()
            .ok_or(E::Refused("no completed frame"))?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(|_| E::Refused("stale solved frame"))?;
        let mut state = source
            .lock()
            .map_err(|_| E::Refused("contact state poisoned"))?;
        if !state
            .normal_frame
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &frame))
        {
            return Err(E::Refused("normal endpoint not current"));
        }
        if state
            .body_post_attempted
            .as_ref()
            .is_some_and(|old| Arc::ptr_eq(old, &frame))
        {
            return Err(E::Refused("Body post suffix already attempted"));
        }
        let timestep = state
            .post_timestep
            .ok_or(E::Refused("missing original post timestep"))?;
        state.body_post_attempted = Some(Arc::clone(&frame));
        self.skaters
            .as_mut()
            .ok_or(E::Pool(SimulationSkaterPoolError::Uninitialized))?
            .with_board_post_fields(board, |fields| fields.advance_wheel_contact_timer(timestep))
            .map_err(E::Pool)?;
        self.try_with_constructed_board_truck_drives(board, |drives, attributes| {
            drives
                .try_update_dynamics(attributes)
                .map_err(E::Dynamics)?;
            drives.try_update_frames(numeric).map_err(E::Frames)
        })
        .map_err(E::Construction)??;
        state.body_post_completed = Some(frame);
        Ok(())
    }
}
