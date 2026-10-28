//! Actual ordinary Update3 suffix after Controller.Update82dfe7d8 returns.
use super::{
    physical_board_forces::PhysicalPlayerBoardForceError,
    physical_wheel_lines::PhysicalPlayerWheelLineError,
    update3_controller::Update3BeforePrePhysics,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
        forces::force_displacement::StoreForceDisplacementError,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum Update3PrePhysicsTailError<N> {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    ForeignOwner,
    Wheel(PhysicalPlayerWheelLineError<N>),
    Forces(PhysicalPlayerBoardForceError),
    Displacement(StoreForceDisplacementError),
}
impl<N: std::fmt::Debug> std::fmt::Display for Update3PrePhysicsTailError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "Update3 suffix missing {field}"),
            Self::Poisoned => f.write_str("Update3 suffix retained owner poisoned"),
            Self::Pair(error) => error.fmt(f),
            Self::ForeignOwner => f.write_str("Update3 suffix belongs to another pair"),
            Self::Wheel(error) => write!(f, "Update3 wheel lines: {error:?}"),
            Self::Forces(error) => error.fmt(f),
            Self::Displacement(error) => error.fmt(f),
        }
    }
}
/// Ordinary selected Update3 returned; solver/Update4/Update5 remain separate.
pub(crate) struct OrdinaryUpdate3Complete {
    pub(crate) owner: Arc<PhysicalPlayerAssemblyOwner>,
    pub(crate) expected_displacement: SkeletonRootVector,
    pub(crate) state_timer: f32,
}
impl PhysicalAssemblyWorld {
    ///82dfe7f0..e834, including actual `PrePhysics82b37cb8` and its ordered calls.
    pub(crate) fn try_finish_update3_prephysics<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        before: Update3BeforePrePhysics,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<OrdinaryUpdate3Complete, Update3PrePhysicsTailError<N::Error>> {
        use Update3PrePhysicsTailError as E;
        let owner = before.owner;
        let same = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, actual| {
                Arc::ptr_eq(actual, &owner)
            })
            .map_err(E::Pair)?;
        if !same {
            return Err(E::ForeignOwner);
        }
        // The shared native empty stub at82dfe7f0 has no effects.
        // PrePhysics captures this actual up vector by value before wheel Start.
        let up = owner
            .reckoning()
            .lock()
            .map_err(|_| E::Poisoned)?
            .reckoning()
            .ok_or(E::Missing("constructed Reckoning"))?
            .up();
        self.try_start_board_wheel_lines(board, ground, up, numeric)
            .map_err(E::Wheel)?;
        self.try_apply_board_forces(board).map_err(E::Forces)?;
        let expected_displacement = self
            .try_store_board_force_displacement(board)
            .map_err(E::Displacement)?;
        // e824 captures Processed, e828 reads old timer, e82c loads fresh DT.
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        let old_timer = state.state_timer.ok_or(E::Missing("HiLOD timer14c"))?;
        let dt = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("processed DT93c"))?;
        let state_timer = dt + old_timer; //e830 fadds DT, old timer
        state.state_timer = Some(state_timer); //e834; processed timer974 unchanged
        drop(state);
        Ok(OrdinaryUpdate3Complete {
            owner,
            expected_displacement,
            state_timer,
        })
    }
}
