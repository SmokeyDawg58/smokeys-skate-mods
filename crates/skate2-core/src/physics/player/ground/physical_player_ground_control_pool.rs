//! Actual board constructor owner for Ground control a048..a074.
use super::{
    SimulationSkaterAssemblyHandle, SimulationSkaterAssemblyPool, SkateboardSimulationBackend,
};
use crate::{
    dynamic_assembly_session::skater_pool::board_entry::SkateboardEntrySessionError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    reckoning_numeric::ReckoningNumeric,
    skateboard_steering::physics_error::{PhysicsErrorSource, PhysicsErrorUpdateError},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum GroundControlError<N> {
    Owner(SkateboardEntrySessionError),
    Cvar(InputCvarError),
    Missing(&'static str),
    Physics(PhysicsErrorUpdateError<SkateboardEntrySessionError, N>),
}
impl<N: std::fmt::Debug> std::fmt::Display for GroundControlError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "Ground board control owner: {error}"),
            Self::Cvar(error) => write!(f, "Ground board control cvar: {error:?}"),
            Self::Missing(field) => write!(f, "Ground board control missing {field}"),
            Self::Physics(error) => write!(f, "{error}"),
        }
    }
}

pub(crate) struct Source<'a> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) board: &'a crate::skateboard_assembly::SkateboardAssemblySession,
    pub(crate) backend: &'a SkateboardSimulationBackend,
}
impl PhysicsErrorSource for Source<'_> {
    type Error = SkateboardEntrySessionError;
    fn orientation(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::ProcessedPoisoned)?
            .skateboard_orientation()
            .copied()
            .ok_or(Self::Error::MissingConstruction)
    }
    fn part_six(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.board
            .try_live_part_transform_typed(self.backend, 6)
            .map_err(Self::Error::Transform)
    }
}

impl SimulationSkaterAssemblyPool {
    pub(crate) fn try_ground_control_prefix<N: ReckoningNumeric>(
        &mut self,
        backend: &SkateboardSimulationBackend,
        board: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        cvars: &InputCvarReferences,
        numeric: &mut N,
    ) -> Result<f32, GroundControlError<N::Error>> {
        use GroundControlError as E;
        use SkateboardEntrySessionError as O;
        self.check_backend(backend)
            .map_err(|e| E::Owner(O::Pool(e)))?;
        let index = self.member_index(board).map_err(|e| E::Owner(O::Pool(e)))?;
        let candidate = self.members[index]
            .content
            .board_mut()
            .map_err(|e| E::Owner(O::Pool(e)))?;
        let actual = candidate
            .construction
            .as_ref()
            .and_then(|c| c.physical_owner())
            .ok_or(E::Missing("board constructor"))?;
        if !Arc::ptr_eq(actual, owner) {
            return Err(E::Owner(O::ForeignOwner));
        }
        {
            let retained = actual
                .reckoning()
                .lock()
                .map_err(|_| E::Owner(O::ReckoningPoisoned))?;
            if !Arc::ptr_eq(retained.processed(), processed) {
                return Err(E::Owner(O::ForeignOwner));
            }
        }
        // DebugDrawJoints23910 gets this actual body assembly then calls the
        // empty825187c0 twice. No debug singleton or geometry query occurs.
        // Both source branch callees are empty; preserve the live gate read.
        cvars.try_draw_board_joints().map_err(E::Cvar)?;
        let runtime = candidate
            .force_runtime
            .as_mut()
            .ok_or(E::Missing("outer board"))?;
        runtime
            .steering
            .try_update_physics_error(
                &mut Source {
                    processed,
                    board: &candidate.session,
                    backend,
                },
                numeric,
            )
            .map_err(E::Physics)?;
        // CalcTruckDamping23980 is exactly a fresh processed+944 scalar load.
        // It is returned in f1; SetTruckDamping is a later caller stage a118.
        let processed = processed
            .lock()
            .map_err(|_| E::Owner(O::ProcessedPoisoned))?;
        processed
            .prior_output()
            .z_speed()
            .map(|v| v.value)
            .ok_or(E::Missing("Z speed+944"))
    }
}
