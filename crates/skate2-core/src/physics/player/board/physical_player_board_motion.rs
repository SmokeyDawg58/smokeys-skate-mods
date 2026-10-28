//! Scoped actual-body motion sampling/publication; not the preceding full
//! `FillPhysOut` position/effective-transform/cvar path or a complete frame.
use super::{
    SimulationAssemblyConstraintSession, physical_phys_out::PhysicalPlayerPhysOutOwnerError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
    },
    physical_player_phys_out::motion::PhysOutMotionError,
    simulation_contact_jacobian::contact_spy::SimulationContactSpyFrame,
    simulation_rigid_body_pool::{SimulationRigidBodyObservation, SimulationRigidBodyPoolError},
    skateboard_hook::SkateboardHookDefinition,
    skeleton_collision_numeric::SkeletonCollisionNumeric,
    skeleton_root_transforms::SkeletonRootVector,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::Arc;

/// Sealed early native body+0x20 sample, tied to this actual pair/body/frame.
/// Retention intentionally permits velocity changes before the late publication.
pub(crate) struct PhysicalPlayerBoardMotionSample {
    body: SimulationRigidBodyObservation,
    output: Arc<()>,
    frame: Option<Arc<SimulationContactSpyFrame>>,
    linear: SkeletonRootVector,
}
impl PhysicalPlayerBoardMotionSample {
    /// Copy of the sealed native f550 read; body/frame identity stays private.
    pub(crate) fn linear_velocity(&self) -> SkeletonRootVector {
        self.linear
    }
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardMotionError<N> {
    Pair(PhysicalPlayerPhysOutOwnerError),
    Pool(SimulationSkaterPoolError),
    Body(SimulationRigidBodyPoolError),
    WrongFrame,
    ForeignSample,
    Publication(PhysOutMotionError<SimulationRigidBodyPoolError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for PhysicalPlayerBoardMotionError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pair(e) => write!(f, "board motion pair: {e}"),
            Self::Pool(e) => write!(f, "board motion Part: {e}"),
            Self::Body(e) => write!(f, "board motion body: {e}"),
            Self::WrongFrame => f.write_str("board motion sample belongs to a noncurrent frame"),
            Self::ForeignSample => {
                f.write_str("board motion sample belongs to another body or output")
            }
            Self::Publication(e) => write!(f, "board motion publication: {e:?}"),
        }
    }
}
impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// f550 read from the body captured by native entry f43c. Caller supplies
    /// its already-held output occurrence, avoiding recursive output locking.
    pub(crate) fn try_sample_captured_board_motion(
        &self,
        body: SimulationRigidBodyObservation,
        output: Arc<()>,
    ) -> Result<
        PhysicalPlayerBoardMotionSample,
        PhysicalPlayerBoardMotionError<std::convert::Infallible>,
    > {
        use PhysicalPlayerBoardMotionError as E;
        let xyz = self
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .map_err(E::Body)?;
        Ok(PhysicalPlayerBoardMotionSample {
            body,
            output,
            frame: None,
            linear: SkeletonRootVector { xyz, w: None },
        })
    }
    /// Selected early read 0x82b2f550. Does not execute intervening Fill stages.
    pub(crate) fn try_sample_pair_board_motion(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        PhysicalPlayerBoardMotionSample,
        PhysicalPlayerBoardMotionError<std::convert::Infallible>,
    > {
        use PhysicalPlayerBoardMotionError as E;
        let frame = self.contact_spies.as_ref().ok_or(E::WrongFrame)?;
        frame
            .try_validate_current(&self.contacts, self.backend.collision_body_pool())
            .map_err(|_| E::WrongFrame)?;
        let frame = Arc::clone(frame);
        let body = self
            .board(board)
            .map_err(E::Pool)?
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(|e| E::Pool(SimulationSkaterPoolError::PhysicalPart(e)))?;
        let output = self
            .try_with_pair_phys_out(board, skeleton, |output| Arc::clone(output.occurrence()))
            .map_err(E::Pair)?;
        let xyz = self
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(body)
            .map_err(E::Body)?;
        Ok(PhysicalPlayerBoardMotionSample {
            body,
            output,
            frame: Some(frame),
            linear: SkeletonRootVector { xyz, w: None },
        })
    }
    /// Selected adjacent motion fields only. Consumes the sample so a completed
    /// publication cannot accidentally be replayed as a second native sample.
    pub(crate) fn try_publish_pair_board_motion<N: SkeletonCollisionNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        sample: PhysicalPlayerBoardMotionSample,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerBoardMotionError<N::Error>> {
        use PhysicalPlayerBoardMotionError as E;
        if let Some(sample_frame) = &sample.frame {
            let frame = self.contact_spies.as_ref().ok_or(E::WrongFrame)?;
            if !Arc::ptr_eq(frame, sample_frame) {
                return Err(E::WrongFrame);
            }
            frame
                .try_validate_current(&self.contacts, self.backend.collision_body_pool())
                .map_err(|_| E::WrongFrame)?;
        }
        let body = self
            .board(board)
            .map_err(E::Pool)?
            .try_part_contact_spy_observation(&self.backend, 6)
            .map_err(|e| E::Pool(SimulationSkaterPoolError::PhysicalPart(e)))?;
        if body != sample.body {
            return Err(E::ForeignSample);
        }
        // Reuse the same-pair bridge before taking an independent retained owner
        // borrow, allowing the angular getter to run inside the output stage.
        self.try_with_pair_phys_out(board, skeleton, |output| {
            Arc::ptr_eq(output.occurrence(), &sample.output)
        })
        .map_err(E::Pair)?
        .then_some(())
        .ok_or(E::ForeignSample)?;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(|e| E::Pair(PhysicalPlayerPhysOutOwnerError::Body(e)))?,
        );
        let mut output = owner
            .phys_out()
            .lock()
            .map_err(|_| E::Pair(PhysicalPlayerPhysOutOwnerError::Poisoned))?;
        output.set_motion_source_frame(None);
        output
            .try_publish_selected_board_motion(
                sample.linear,
                || {
                    self.backend
                        .collision_body_pool()
                        .try_phys_out_angular_velocity(sample.body)
                        .map(|xyz| SkeletonRootVector { xyz, w: None })
                },
                numeric,
            )
            .map_err(E::Publication)?;
        output.set_motion_source_frame(sample.frame);
        Ok(())
    }
}
