//! Actual pair `SurfacePhysics` owner and four native wheel Part pointer stores.
use super::{
    SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError,
    physical_phys_out::PhysicalPlayerPhysOutOwnerError,
};
use crate::{
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
        board_construction::SimulationConstructedMaterialError,
    },
    physical_player_phys_out::PhysOutSurfaceError,
    skateboard_hook::SkateboardHookDefinition,
    skateboard_processed_surface::SkateboardSelectedMaterial,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum PhysicalPlayerSurfaceRuntimeError {
    Physical(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Output(PhysicalPlayerPhysOutOwnerError),
    MissingSurface,
    Poisoned,
    ForeignOutput,
    ForeignMaterial,
    Dispatch(PhysOutSurfaceError<Infallible>),
}
impl std::fmt::Display for PhysicalPlayerSurfaceRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Physical(e) => write!(f, "surface physical owner: {e}"),
            Self::Pool(e) => write!(f, "surface pool: {e}"),
            Self::Output(e) => write!(f, "surface output: {e}"),
            Self::Dispatch(e) => write!(f, "surface dispatch: {e:?}"),
            other => write!(f, "surface runtime: {other:?}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerWheelMaterialError<I> {
    Owner(PhysicalPlayerSurfaceRuntimeError),
    Input(I),
    Part(SimulationConstructedMaterialError),
}
impl<I: std::fmt::Debug> std::fmt::Display for PhysicalPlayerWheelMaterialError<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "{e}"),
            Self::Input(e) => write!(f, "selected material input: {e:?}"),
            Self::Part(e) => write!(f, "{e}"),
        }
    }
}
impl SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition> {
    #[cfg(test)]
    pub(crate) fn with_board_materials_for_test<T>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
        callback: impl FnOnce(
            &crate::skateboard_assembly::SkateboardAssemblySession,
            &crate::dynamic_assembly_session::skateboard_publication::SkateboardCollisionPublication,
            u64,
        ) -> T,
    ) -> T {
        self.skaters
            .as_ref()
            .unwrap()
            .with_board_materials_for_test(board, callback)
    }
    pub(crate) fn try_dispatch_pair_surface(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        processed: &mut SkeletonProcessedContact,
    ) -> Result<(), PhysicalPlayerSurfaceRuntimeError> {
        use PhysicalPlayerSurfaceRuntimeError as E;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(E::Physical)?,
        );
        self.try_with_pair_phys_out(board, skeleton, |output| {
            if !processed.begin_surface_dispatch(output) {
                return Err(E::ForeignOutput);
            }
            let retained = owner.surface_physics().lock().map_err(|_| E::Poisoned)?;
            output
                .try_dispatch_surface(retained.as_ref().ok_or(E::MissingSurface)?, processed)
                .map_err(E::Dispatch)
        })
        .map_err(E::Output)?
    }
    /// `SetWheelMaterialForSurface` 0x82b29fb8. Capture one selected pointer
    /// after actual Assembly resolution, then perform four separate Part stores.
    pub(crate) fn try_set_board_wheel_material<I>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        capture: impl FnOnce() -> Result<SkateboardSelectedMaterial, I>,
    ) -> Result<(), PhysicalPlayerWheelMaterialError<I>> {
        use PhysicalPlayerSurfaceRuntimeError as O;
        use PhysicalPlayerWheelMaterialError as E;
        self.board(board).map_err(|e| E::Owner(O::Pool(e)))?;
        let owner = Arc::clone(
            self.physical_body_owner_for_contacts(board)
                .map_err(|e| E::Owner(O::Physical(e)))?,
        );
        let selected = capture().map_err(E::Input)?;
        {
            let retained = owner
                .surface_physics()
                .lock()
                .map_err(|_| E::Owner(O::Poisoned))?;
            let surface = retained.as_ref().ok_or(E::Owner(O::MissingSurface))?;
            if !surface.owns_material(&selected) {
                return Err(E::Owner(O::ForeignMaterial));
            }
        }
        for part in 0..4 {
            self.skaters
                .as_mut()
                .expect("validated physical board has pool")
                .try_set_constructed_part_material(
                    &self.backend,
                    board,
                    part,
                    Arc::clone(selected.material_arc()),
                )
                .map_err(E::Part)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn check_actual_board_material_publication(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) {
        self.skaters
            .as_mut()
            .unwrap()
            .check_actual_board_material_publication(&self.backend, board);
    }
}
