//! Same physical-board authority for source force stages.
use super::SimulationAssemblyConstraintSession;
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, forces::SimulationSkateboardForceError,
    },
    simulation_assembly_backend::constraint_session::physical_island::SimulationPhysicalBodyError,
    skateboard_foot_forces::SkateboardFootForce,
    skateboard_force_runtime::SkateboardForceRuntime,
    skateboard_hook::SkateboardHookDefinition,
};
use skate2_formats::collision::AssemblyPartMaterial;

#[path = "physical_player_board_wipeout_session.rs"]
pub(crate) mod board_wipeout;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardForceError {
    Owner(SimulationPhysicalBodyError),
    Forces(SimulationSkateboardForceError),
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerGroundForceError<I, D, N> {
    Owner(SimulationPhysicalBodyError),
    Forces(
        crate::dynamic_assembly_session::skater_pool::forces::SimulationGroundForceError<I, D, N>,
    ),
}
impl<I: std::fmt::Debug, D: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for PhysicalPlayerGroundForceError<I, D, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "physical grounded board owner: {error}"),
            Self::Forces(error) => write!(f, "{error}"),
        }
    }
}
impl std::fmt::Display for PhysicalPlayerBoardForceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "physical board force owner: {error}"),
            Self::Forces(error) => write!(f, "{error}"),
        }
    }
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    pub(crate) fn try_enable_board_collision_volumes(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_enable_board_collision_volumes(&self.backend, board)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    pub(crate) fn try_disable_board_collision_volumes(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_disable_board_collision_volumes(&self.backend, board)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    /// First actual board stage of PhysicsGround.Enter. This does not claim the
    /// remaining Skeleton/controller/manual/Wipeout entry sequence has run.
    pub(crate) fn try_set_board_standard(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_set_board_standard(&mut self.backend, board)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    /// `SetAngularDamping` (`Some`) / `SetStandardAngularDamping` (`None`).
    pub(crate) fn try_set_board_angular_damping(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        damping: Option<f32>,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_set_board_angular_damping(&mut self.backend, board, damping)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    #[cfg(test)]
    pub(crate) fn check_late_board_velocity_failure(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) {
        self.physical_body_owner_for_contacts(board).unwrap();
        self.skaters
            .as_mut()
            .unwrap()
            .check_late_board_velocity_failure(&mut self.backend, board);
    }
    /// Source `SetVelocity`, with host frame certificates invalidated on the first
    /// committed body write. Historical output values and native time survive.
    pub(crate) fn try_set_board_velocity(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        velocity: &crate::skeleton_root_transforms::SkeletonRootVector,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_set_board_velocity(&mut self.backend, board, velocity)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    #[expect(
        clippy::type_complexity,
        reason = "preserves physical authority and staged calculation error identities"
    )]
    pub(crate) fn try_update_board_grounded_force_tail<P, D, N>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        input: &mut P,
        debug: &mut D,
        numeric: &mut N,
    ) -> Result<
        (),
        PhysicalPlayerGroundForceError<
            <P as crate::skateboard_foot_forces::SkateboardFootInputRead>::Error,
            D::Error,
            N::Error,
        >,
    >
    where
        P: crate::skateboard_ground_force_tail::SkateboardGroundForceInput,
        D: crate::skateboard_foot_forces::SkateboardFootDebugRead,
        N: crate::reckoning_numeric::ReckoningNumeric,
    {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerGroundForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_update_board_grounded_force_tail(&mut self.backend, board, input, debug, numeric)
            .map_err(PhysicalPlayerGroundForceError::Forces)
    }
    pub(crate) fn try_set_board_linear_drag(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        drag: f32,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_set_board_linear_drag(&mut self.backend, board, drag)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
    pub(crate) fn try_with_board_force_runtime<T>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        operation: impl FnOnce(&mut SkateboardForceRuntime) -> T,
    ) -> Result<T, PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_with_board_force_runtime(board, operation)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }

    pub(crate) fn try_add_board_force(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        kind: u32,
        force: SkateboardFootForce,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_add_board_force(board, kind, force)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }

    /// Only `ApplyForces`, 0x82b35380. The preceding wheel-line stage and following
    /// postphysics work must still be scheduled by their actual native caller.
    pub(crate) fn try_apply_board_forces(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<(), PhysicalPlayerBoardForceError> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardForceError::Owner)?;
        self.skaters
            .as_mut()
            .expect("registered physical owner has skater pool")
            .try_apply_board_forces(&mut self.backend, board)
            .map_err(PhysicalPlayerBoardForceError::Forces)
    }
}

impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    /// Selected Update3 call82dfe820, after wheel Start and actual `ApplyForces`.
    pub(crate) fn try_store_board_force_displacement(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<crate::skeleton_root_transforms::SkeletonRootVector, crate::dynamic_assembly_session::skater_pool::forces::force_displacement::StoreForceDisplacementError>{
        use crate::dynamic_assembly_session::skater_pool::forces::force_displacement::StoreForceDisplacementError as E;
        self.physical_body_owner_for_contacts(board)
            .map_err(|error| E::Force(PhysicalPlayerBoardForceError::Owner(error)))?;
        self.skaters
            .as_mut()
            .expect("validated physical board pool")
            .try_store_force_displacement(&self.backend, board)
    }
}
