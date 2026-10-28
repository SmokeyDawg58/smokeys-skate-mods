//! Same retained physical board, live Part transforms and selected World batch.
use super::{
    SimulationAssemblyConstraintSession,
    physical_island::SimulationPhysicalBodyError,
    world_line_query::{
        SimulationWorldLineBatch, SimulationWorldLineError, SimulationWorldLineLeaseError,
    },
};
use crate::{
    collision_line_collection::CollisionLineInput,
    dynamic_assembly_session::{
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkaterPoolError,
            board_construction::SimulationConstructedBoardError,
        },
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    skateboard_body_construction::{
        SkateboardRetainedWheelLines,
        wheel_lines::{
            SkateboardWheelLineError, SkateboardWheelLineHit, SkateboardWheelLineServices,
        },
    },
    skateboard_hook::SkateboardHookDefinition,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::sync::{Arc, Mutex};

type World = SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardPostPrefixError<N> {
    Owner(SimulationPhysicalBodyError),
    Prefix(
        crate::dynamic_assembly_session::skater_pool::postphysics::SimulationBoardPostPrefixError<
            N,
        >,
    ),
}
impl<N: std::fmt::Debug> std::fmt::Display for PhysicalPlayerBoardPostPrefixError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(e) => write!(f, "board post owner: {e}"),
            Self::Prefix(e) => write!(f, "{e}"),
        }
    }
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerWheelLineServiceError<E> {
    Physical(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Construction(SimulationConstructedBoardError),
    Part(DynamicAssemblyPartTransformError),
    MissingWorld,
    MissingDmo,
    Poisoned,
    Lease(SimulationWorldLineLeaseError),
    Query(SimulationWorldLineError<E>),
}
impl<E: std::fmt::Debug> std::fmt::Display for PhysicalPlayerWheelLineServiceError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Physical(e) => write!(f, "wheel physical owner: {e}"),
            Self::Pool(e) => write!(f, "wheel pool: {e}"),
            Self::Construction(e) => write!(f, "wheel construction: {e}"),
            Self::Part(e) => write!(f, "wheel Part: {e}"),
            Self::Lease(e) => write!(f, "wheel lease: {e:?}"),
            Self::Query(e) => write!(f, "wheel query: {e}"),
            other => write!(f, "wheel service: {other:?}"),
        }
    }
}
pub(crate) type PhysicalPlayerWheelLineError<E> =
    SkateboardWheelLineError<PhysicalPlayerWheelLineServiceError<E>>;

struct Services<'a, N> {
    world: &'a mut World,
    board: &'a SimulationSkaterAssemblyHandle,
    ground: &'a StaticGroundPool,
    numeric: &'a mut N,
}
impl<N: AssemblyGroundNumeric + ReckoningNumeric> SkateboardWheelLineServices for Services<'_, N> {
    type Batch = SimulationWorldLineBatch;
    type Error = PhysicalPlayerWheelLineServiceError<N::Error>;
    fn acquire(&mut self) -> Result<Self::Batch, Self::Error> {
        let dmos = self.world.dmos.as_ref().ok_or(Self::Error::MissingDmo)?;
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_acquire_line_batch(dmos, "SkateboardBody::StartWheelLineTest", 7)
            .map_err(Self::Error::Lease)
    }
    fn begin(&mut self, batch: &Self::Batch) -> Result<(), Self::Error> {
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_begin(batch)
            .map_err(Self::Error::Lease)
    }
    fn part_position(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        let matrix = self
            .world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_live_part_transform_host(&self.world.backend, part)
            .map_err(Self::Error::Part)?;
        // This attached-body host getter supplies semantic XYZ and artificial
        // affine W. Never advertise that affine lane as the native packed W.
        Ok(SkeletonRootVector {
            xyz: [matrix[12], matrix[13], matrix[14]],
            w: None,
        })
    }
    fn add(&mut self, batch: &Self::Batch, input: CollisionLineInput) -> Result<bool, Self::Error> {
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_add_line(batch, input)
            .map_err(Self::Error::Lease)
    }
    fn end(&mut self, batch: &Self::Batch) -> Result<(), Self::Error> {
        let dmos = self.world.dmos.as_ref().ok_or(Self::Error::MissingDmo)?;
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_end(batch, self.ground, dmos, self.numeric)
            .map_err(Self::Error::Query)
    }
    fn sync(&mut self, batch: &Self::Batch) -> Result<(), Self::Error> {
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_sync(batch)
            .map_err(Self::Error::Lease)
    }
    fn result(
        &mut self,
        batch: &Self::Batch,
        index: u32,
    ) -> Result<Option<SkateboardWheelLineHit>, Self::Error> {
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_get_result(batch, index)
            .map_err(Self::Error::Lease)
            .map(|hit| {
                hit.map(|hit| SkateboardWheelLineHit {
                    position: hit.position,
                    normal: hit.normal,
                    parameter: hit.line_parameter,
                    surface: u32::from(hit.surface_id),
                })
            })
    }
    fn release(&mut self, batch: &Self::Batch) -> Result<(), Self::Error> {
        self.world
            .world_line_queries
            .as_mut()
            .ok_or(Self::Error::MissingWorld)?
            .try_release(batch)
            .map_err(Self::Error::Lease)
    }
}
impl World {
    fn board_wheel_schedule<E>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<
        Arc<Mutex<crate::skateboard_body_construction::SkateboardWheelSchedule>>,
        PhysicalPlayerWheelLineServiceError<E>,
    > {
        use PhysicalPlayerWheelLineServiceError as Error;
        let construction =
            self.board_construction(board)
                .map_err(Error::Pool)?
                .ok_or(Error::Pool(
                    SimulationSkaterPoolError::MissingPhysicalConstruction,
                ))?;
        Ok(Arc::clone(construction.wheel_schedule()))
    }
    pub(crate) fn try_board_post_physics_prefix<N: ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        timestep: f32,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerBoardPostPrefixError<N::Error>> {
        self.physical_body_owner_for_contacts(board)
            .map_err(PhysicalPlayerBoardPostPrefixError::Owner)?;
        self.skaters
            .as_mut()
            .expect("physical owner has pool")
            .try_board_post_physics_prefix(&mut self.backend, board, timestep, numeric)
            .map_err(PhysicalPlayerBoardPostPrefixError::Prefix)
    }
    pub(crate) fn board_wheel_compressions(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<Option<[f32; 2]>, SimulationSkaterPoolError> {
        self.skaters
            .as_ref()
            .ok_or(SimulationSkaterPoolError::MissingPhysicalConstruction)?
            .board_wheel_compressions(board)
    }
    #[cfg(test)]
    pub(crate) fn wheel_line_inputs_for_test(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Vec<CollisionLineInput> {
        let retained = self
            .try_board_wheel_line_state::<std::convert::Infallible>(board)
            .unwrap();
        retained.lock().unwrap().inputs_for_test.clone()
    }
    #[cfg(test)]
    pub(crate) fn wheel_line_results_for_test(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Vec<Option<crate::simulation_assembly_backend::constraint_session::world_line_query::SimulationWorldLineHit>>{
        let retained = self
            .try_board_wheel_line_state::<std::convert::Infallible>(board)
            .unwrap();
        let state = retained.lock().unwrap();
        let batch = state.batch_for_test().unwrap();
        (0..5)
            .map(|index| {
                self.world_line_queries
                    .as_mut()
                    .unwrap()
                    .completed_line_for_test(batch, index)
            })
            .collect()
    }
    pub(crate) fn try_board_wheel_line_state<E>(
        &self,
        board: &SimulationSkaterAssemblyHandle,
    ) -> Result<Arc<Mutex<SkateboardRetainedWheelLines>>, PhysicalPlayerWheelLineServiceError<E>>
    {
        use PhysicalPlayerWheelLineServiceError as E;
        self.physical_body_owner_for_contacts(board)
            .map_err(E::Physical)?;
        self.skaters
            .as_ref()
            .ok_or(E::Pool(
                SimulationSkaterPoolError::MissingPhysicalConstruction,
            ))?
            .try_board_wheel_lines(&self.backend, board)
            .map_err(E::Construction)
    }
    /// Source by-value up was captured before entering Start. The outer caller
    /// must still schedule `ApplyForces` after this stage and End after the solve.
    pub(crate) fn try_start_board_wheel_lines<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        up: SkeletonRootVector,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerWheelLineError<N::Error>> {
        let retained = self
            .try_board_wheel_line_state(board)
            .map_err(SkateboardWheelLineError::Service)?;
        let schedule = self
            .board_wheel_schedule(board)
            .map_err(SkateboardWheelLineError::Service)?;
        schedule
            .lock()
            .map_err(|_| {
                SkateboardWheelLineError::Service(PhysicalPlayerWheelLineServiceError::Poisoned)
            })?
            .begin(self.contact_spies.as_ref());
        let mut state = retained.lock().map_err(|_| {
            SkateboardWheelLineError::Service(PhysicalPlayerWheelLineServiceError::Poisoned)
        })?;
        state.try_start(
            up,
            &mut Services {
                world: self,
                board,
                ground,
                numeric,
            },
        )?;
        schedule
            .lock()
            .map_err(|_| {
                SkateboardWheelLineError::Service(PhysicalPlayerWheelLineServiceError::Poisoned)
            })?
            .started();
        Ok(())
    }
    pub(crate) fn try_end_board_wheel_lines<N: AssemblyGroundNumeric + ReckoningNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<(), PhysicalPlayerWheelLineError<N::Error>> {
        let retained = self
            .try_board_wheel_line_state(board)
            .map_err(SkateboardWheelLineError::Service)?;
        let schedule = self
            .board_wheel_schedule(board)
            .map_err(SkateboardWheelLineError::Service)?;
        let mut state = retained.lock().map_err(|_| {
            SkateboardWheelLineError::Service(PhysicalPlayerWheelLineServiceError::Poisoned)
        })?;
        state.try_end(&mut Services {
            world: self,
            board,
            ground,
            numeric,
        })?;
        schedule
            .lock()
            .map_err(|_| {
                SkateboardWheelLineError::Service(PhysicalPlayerWheelLineServiceError::Poisoned)
            })?
            .end(self.contact_spies.as_ref());
        Ok(())
    }
}
