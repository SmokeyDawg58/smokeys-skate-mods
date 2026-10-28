//! DMO serialized admission through the actual constraint manager/backend.
//! Source image bindings remain explicit staged inputs. This does not yet
//! implement `DynamicPool`'s batch/mesh phases or outer Island Body registration.

use super::SimulationAssemblyConstraintSession;
use crate::{
    serialised_assembly_bbox::SerialisedAssemblyPartBBoxNumeric,
    simulation_assembly_backend::source::SimulationAssemblySourceInitError,
    simulation_dmo_assembly_part_session::dynamic_pool::{
        SimulationDmoAssemblyCandidate, SimulationDmoAssemblyHandle, SimulationDmoAssemblyPool,
        SimulationDmoInsertFailure, SimulationDmoPoolError,
    },
    simulation_normal_arena_plan::SimulationNormalArenaPlan,
    simulation_xenon_address_image::{
        SimulationArenaXenonAddressBinding, SimulationXenonAddressImage,
    },
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, sync::Arc};

#[derive(Debug)]
pub(crate) enum SimulationDmoPoolInitError {
    Pool(SimulationDmoPoolError<Infallible>),
    Source(SimulationAssemblySourceInitError),
}

impl std::fmt::Display for SimulationDmoPoolInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pool(error) => write!(f, "DMO pool initialization: {error}"),
            Self::Source(error) => write!(f, "DMO source initialization: {error}"),
        }
    }
}

impl<C> SimulationAssemblyConstraintSession<AssemblyPartMaterial, C> {
    /// Allocates the actual capacity50 owner, then binds the already constructed
    /// manager backend once. Checked setup finishes before publishing membership.
    pub(crate) fn try_initialize_dmo_pool(
        &mut self,
        image: SimulationXenonAddressImage,
        bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
        update_in_parallel: bool,
    ) -> Result<(), SimulationDmoPoolInitError> {
        if self.dmos.is_some() {
            return Err(SimulationDmoPoolInitError::Pool(
                SimulationDmoPoolError::AlreadyInitialized,
            ));
        }
        let pool =
            SimulationDmoAssemblyPool::try_new(&self.contacts, &self.backend, update_in_parallel)
                .map_err(SimulationDmoPoolInitError::Pool)?;
        self.backend
            .try_bind_source_arenas(image, bindings)
            .map_err(SimulationDmoPoolInitError::Source)?;
        self.dmos = Some(pool);
        Ok(())
    }

    /// Binds the source routes of an already constructed DMO pool (the World
    /// query owner creates it empty), once, before the first body is added.
    /// The addresses are host layout inputs, not native allocator results.
    pub(crate) fn try_bind_living_world_sources(
        &mut self,
        image: SimulationXenonAddressImage,
        bindings: Vec<(
            Arc<SimulationNormalArenaPlan>,
            Arc<SimulationArenaXenonAddressBinding>,
        )>,
    ) -> Result<(), SimulationDmoPoolInitError> {
        let pool = self
            .dmos
            .as_ref()
            .ok_or(SimulationDmoPoolInitError::Pool(SimulationDmoPoolError::Uninitialized))?;
        if pool.len() != 0 {
            return Err(SimulationDmoPoolInitError::Pool(
                SimulationDmoPoolError::AlreadyInitialized,
            ));
        }
        self.backend
            .try_bind_source_arenas(image, bindings)
            .map_err(SimulationDmoPoolInitError::Source)
    }

    pub(crate) fn dmo_pool(&self) -> Option<&SimulationDmoAssemblyPool> {
        self.dmos.as_ref()
    }

    pub(crate) fn try_insert_dmo<N: SerialisedAssemblyPartBBoxNumeric>(
        &mut self,
        candidate: SimulationDmoAssemblyCandidate,
        state: u32,
        numeric: &mut N,
    ) -> Result<SimulationDmoAssemblyHandle, Box<SimulationDmoInsertFailure<N::Error>>> {
        let Some(pool) = self.dmos.as_mut() else {
            return Err(Box::new(SimulationDmoInsertFailure {
                error: SimulationDmoPoolError::Uninitialized,
                candidate,
            }));
        };
        pool.try_insert(
            &mut self.contacts,
            &mut self.backend,
            candidate,
            state,
            numeric,
        )
    }

    /// Base serialized removal; source `DynamicPool` synchronization/mesh cleanup
    /// must enclose this phase once those retained owners are initialized.
    pub(crate) fn try_remove_dmo_serialized(
        &mut self,
        handle: &SimulationDmoAssemblyHandle,
    ) -> Result<SimulationDmoAssemblyCandidate, SimulationDmoPoolError<Infallible>> {
        self.dmos
            .as_mut()
            .ok_or(SimulationDmoPoolError::Uninitialized)?
            .try_remove_serialized(&mut self.backend, handle)
    }
}
