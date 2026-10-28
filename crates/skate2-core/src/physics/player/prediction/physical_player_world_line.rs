//! Actual selected-world query owner construction and scoped ground borrowing.
#[path = "../runtime/physical_player_camera_line_services.rs"]
pub(crate) mod camera;

use super::SimulationAssemblyConstraintSession;
use super::world_line_query::{
    SimulationWorldLineError, SimulationWorldLineInitError, SimulationWorldLineQueries,
    SimulationWorldLineQueryConfig,
};
use crate::simulation_dmo_assembly_part_session::dynamic_pool::{
    SimulationDmoAssemblyPool, SimulationDmoPoolError,
};
use skate2_formats::collision::AssemblyPartMaterial;

#[derive(Debug)]
pub(crate) enum PhysicalWorldLineInitError {
    AlreadyInitialized,
    Dmo(SimulationDmoPoolError<std::convert::Infallible>),
    Query(SimulationWorldLineInitError),
}
#[derive(Debug)]
pub(crate) enum PhysicalWorldLineError<E> {
    Uninitialized,
    MissingDmo,
    Query(SimulationWorldLineError<E>),
}
impl std::fmt::Display for PhysicalWorldLineInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => f.write_str("World query owner already initialized"),
            Self::Dmo(error) => write!(f, "World DMO constructor: {error}"),
            Self::Query(error) => write!(f, "World query constructor: {error}"),
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Display for PhysicalWorldLineError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uninitialized => f.write_str("World query owner is uninitialized"),
            Self::MissingDmo => f.write_str("World DMO owner is absent"),
            Self::Query(error) => write!(f, "{error}"),
        }
    }
}
impl<C> SimulationAssemblyConstraintSession<AssemblyPartMaterial, C> {
    /// InitWorld827b43b0 constructs a separate base AssemblyPool before the
    /// skater pool. Caller supplies the live pedestrian parallel cvar.
    pub(crate) fn try_initialize_pedestrian_pool(&mut self, update_in_parallel: bool)
        -> Result<(), SimulationDmoPoolError<std::convert::Infallible>> {
        if self.pedestrians.is_some() { return Err(SimulationDmoPoolError::AlreadyInitialized); }
        self.pedestrians = Some(
            crate::simulation_dmo_assembly_part_session::dynamic_pool::character::pedestrian_pool::PedestrianAssemblyPool::try_new(
                &self.contacts, &self.backend, self.island_flags, update_in_parallel)?);
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn last_world_line_input_for_test(
        &self,
    ) -> Option<crate::collision_line_collection::CollisionLineInput> {
        self.world_line_queries
            .as_ref()
            .and_then(SimulationWorldLineQueries::last_line_input_for_test)
    }
    /// Construct actual empty owners without binding invented source addresses.
    /// Existing DMO membership is preserved. No `BeginUpdate` or spawn is implied.
    pub(crate) fn try_initialize_world_line_queries(
        &mut self,
        config: SimulationWorldLineQueryConfig,
        dmo_update_in_parallel: bool,
    ) -> Result<(), PhysicalWorldLineInitError> {
        if self.world_line_queries.is_some() {
            return Err(PhysicalWorldLineInitError::AlreadyInitialized);
        }
        if self.dmos.is_none() {
            self.dmos = Some(
                SimulationDmoAssemblyPool::try_new(
                    &self.contacts,
                    &self.backend,
                    dmo_update_in_parallel,
                )
                .map_err(PhysicalWorldLineInitError::Dmo)?,
            );
        }
        self.world_line_queries = Some(
            SimulationWorldLineQueries::try_new(
                &self.contacts,
                &self.backend,
                self.dmos.as_ref().expect("constructed DMO"),
                self.island_flags,
                config,
            )
            .map_err(PhysicalWorldLineInitError::Query)?,
        );
        Ok(())
    }

    pub(crate) fn try_world_line_test<N>(
        &mut self,
        ground: &crate::static_ground_pool::StaticGroundPool,
        input: crate::collision_line_collection::CollisionLineInput,
        label: &str,
        numeric: &mut N,
    ) -> Result<
        Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineHit>,
        PhysicalWorldLineError<N::Error>,
    >
    where
        N: crate::simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric
            + crate::reckoning_numeric::ReckoningNumeric,
    {
        let queries = self
            .world_line_queries
            .as_mut()
            .ok_or(PhysicalWorldLineError::Uninitialized)?;
        let dmos = self
            .dmos
            .as_ref()
            .ok_or(PhysicalWorldLineError::MissingDmo)?;
        queries
            .try_line_test(ground, dmos, input, label, numeric)
            .map_err(PhysicalWorldLineError::Query)
    }

    /// The retained query owner and DMO pool, split-borrowed together for
    /// callers outside this session module (OffboardJumpingTest StartQuery).
    pub(crate) fn world_line_owners(
        &mut self,
    ) -> (
        Option<&mut SimulationWorldLineQueries>,
        Option<&SimulationDmoAssemblyPool>,
    ) {
        (self.world_line_queries.as_mut(), self.dmos.as_ref())
    }
}
