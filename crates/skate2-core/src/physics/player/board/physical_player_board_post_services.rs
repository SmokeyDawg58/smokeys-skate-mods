//! Actual retained-board reads for the existing `PostPhysics` contact stages.
//! Scheduling/frame validation stays with the enclosing World continuation.
use super::{
    SimulationAssemblyConstraintSession, physical_island::SimulationPhysicalBodyError,
    physical_wheel_lines::PhysicalPlayerWheelLineServiceError,
};
use crate::simulation_assembly_backend::constraint_session::physical_island::{
    SimulationIslandBodyHandle, contact_owners::SimulationContactEntityOwner,
};
use crate::{
    dynamic_assembly_session::{
        part_contact_spy::DynamicAssemblyPartContactSpyError,
        part_transform::DynamicAssemblyPartTransformError,
        skater_pool::{SimulationSkaterAssemblyHandle, SimulationSkaterPoolError},
    },
    reckoning_numeric::ReckoningNumeric,
    simulation_rigid_body_pool::SimulationRigidBodyPoolError,
    skateboard_contact_surface::{
        motion::SkateboardContactMotionServices, prefix::SkateboardPostContactServices,
    },
    skateboard_hook::SkateboardHookDefinition,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use skate2_formats::collision::AssemblyPartMaterial;
use std::{convert::Infallible, fmt};

type World = SimulationAssemblyConstraintSession<AssemblyPartMaterial, SkateboardHookDefinition>;

/// Targets are captured separately from their scalar loads. Implementations
/// retain the actual caller's cvar owners; this trait supplies no defaults.
pub(crate) trait SkateboardPostCvars {
    type Error;
    type DeckTarget;
    type LineTarget;
    type FreeWheelTarget;
    fn capture_deck_end(&mut self) -> Result<Self::DeckTarget, Self::Error>;
    fn deck_end(&mut self, target: &Self::DeckTarget) -> Result<f32, Self::Error>;
    fn capture_line_distance(&mut self) -> Result<Self::LineTarget, Self::Error>;
    fn line_distance(&mut self, target: &Self::LineTarget) -> Result<f32, Self::Error>;
    fn capture_free_wheel_drag(&mut self) -> Result<Self::FreeWheelTarget, Self::Error>;
    fn free_wheel_drag(&mut self, target: &Self::FreeWheelTarget) -> Result<f32, Self::Error>;
    fn forced_surface(&mut self) -> Result<u32, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum PhysicalPlayerBoardPostServiceError<N, C> {
    Numeric(N),
    Cvar(C),
    Owner(SimulationPhysicalBodyError),
    Pool(SimulationSkaterPoolError),
    Transform(DynamicAssemblyPartTransformError),
    Part(DynamicAssemblyPartContactSpyError),
    Body(SimulationRigidBodyPoolError),
    Wheel(PhysicalPlayerWheelLineServiceError<Infallible>),
    PoisonedWheelState,
    Missing(&'static str),
    InvalidWheel(usize),
}
impl<N: fmt::Debug, C: fmt::Debug> fmt::Display for PhysicalPlayerBoardPostServiceError<N, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(error) => write!(f, "board post numeric: {error:?}"),
            Self::Cvar(error) => write!(f, "board post cvar: {error:?}"),
            Self::Owner(error) => write!(f, "board post owner: {error}"),
            Self::Pool(error) => write!(f, "board post pool: {error}"),
            Self::Transform(error) => write!(f, "board post Body transform: {error}"),
            Self::Part(error) => write!(f, "board post Part: {error}"),
            Self::Body(error) => write!(f, "board post rigid body: {error}"),
            Self::Wheel(error) => write!(f, "board post wheel owner: {error}"),
            Self::PoisonedWheelState => f.write_str("board wheel state lock poisoned"),
            Self::Missing(field) => write!(f, "board post {field} is unready"),
            Self::InvalidWheel(index) => {
                write!(f, "board post wheel index {index} is outside four wheels")
            }
        }
    }
}

pub(crate) struct PhysicalPlayerBoardPostServices<'a, N, C> {
    pub(crate) world: &'a World,
    pub(crate) board: &'a SimulationSkaterAssemblyHandle,
    pub(crate) numeric: &'a mut N,
    pub(crate) cvars: &'a mut C,
}

impl<N: ReckoningNumeric, C: SkateboardPostCvars> ReckoningNumeric
    for PhysicalPlayerBoardPostServices<'_, N, C>
{
    type Error = PhysicalPlayerBoardPostServiceError<N::Error, C::Error>;
    fn dot3(&mut self, left: [f32; 4], right: [f32; 4]) -> Result<f32, Self::Error> {
        self.numeric.dot3(left, right).map_err(Self::Error::Numeric)
    }
    fn reciprocal_sqrt_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.numeric
            .reciprocal_sqrt_estimate(value)
            .map_err(Self::Error::Numeric)
    }
    fn reciprocal_estimate(&mut self, value: f32) -> Result<f32, Self::Error> {
        self.numeric
            .reciprocal_estimate(value)
            .map_err(Self::Error::Numeric)
    }
    fn truncate_i32(&mut self, value: f32) -> Result<i32, Self::Error> {
        self.numeric
            .truncate_i32(value)
            .map_err(Self::Error::Numeric)
    }
}

impl<N: ReckoningNumeric, C: SkateboardPostCvars> SkateboardPostContactServices
    for PhysicalPlayerBoardPostServices<'_, N, C>
{
    type DeckEndTarget = C::DeckTarget;
    fn owner(
        &mut self,
        body: &SimulationIslandBodyHandle,
    ) -> Result<SimulationContactEntityOwner, Self::Error> {
        self.world
            .island_body_owner_for_contacts(body)
            .map_err(Self::Error::Owner)
    }
    fn other_group(&mut self, body: &SimulationIslandBodyHandle) -> Result<u32, Self::Error> {
        self.world
            .island_body_group_for_contacts(body)
            .map_err(Self::Error::Owner)
    }
    fn deck_body_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_attached_part_body_transform(&self.world.backend, 6)
            .map_err(Self::Error::Transform)
    }
    fn capture_deck_end_target(&mut self) -> Result<Self::DeckEndTarget, Self::Error> {
        self.cvars.capture_deck_end().map_err(Self::Error::Cvar)
    }
    fn deck_end_value(&mut self, target: &Self::DeckEndTarget) -> Result<f32, Self::Error> {
        self.cvars.deck_end(target).map_err(Self::Error::Cvar)
    }
}

impl<N: ReckoningNumeric, C: SkateboardPostCvars> SkateboardContactMotionServices
    for PhysicalPlayerBoardPostServices<'_, N, C>
{
    type LineTarget = C::LineTarget;
    fn capture_line_threshold(&mut self) -> Result<Self::LineTarget, Self::Error> {
        self.cvars
            .capture_line_distance()
            .map_err(Self::Error::Cvar)
    }
    fn wheel_distance(&mut self, wheel: usize) -> Result<f32, Self::Error> {
        let retained = self
            .world
            .try_board_wheel_line_state::<Infallible>(self.board)
            .map_err(Self::Error::Wheel)?;
        let state = retained
            .lock()
            .map_err(|_| Self::Error::PoisonedWheelState)?;
        state
            .distances
            .get(wheel)
            .ok_or(Self::Error::InvalidWheel(wheel))?
            .ok_or(Self::Error::Missing("wheel distance"))
    }
    fn line_threshold(&mut self, target: &Self::LineTarget) -> Result<f32, Self::Error> {
        self.cvars.line_distance(target).map_err(Self::Error::Cvar)
    }
    fn wheel_normal(&mut self, wheel: usize) -> Result<SkeletonRootVector, Self::Error> {
        let retained = self
            .world
            .try_board_wheel_line_state::<Infallible>(self.board)
            .map_err(Self::Error::Wheel)?;
        let state = retained
            .lock()
            .map_err(|_| Self::Error::PoisonedWheelState)?;
        state
            .normals
            .get(wheel)
            .ok_or(Self::Error::InvalidWheel(wheel))?
            .ok_or(Self::Error::Missing("wheel normal"))
    }
    fn body_velocity(&mut self, part: usize) -> Result<SkeletonRootVector, Self::Error> {
        let observation = self
            .world
            .board(self.board)
            .map_err(Self::Error::Pool)?
            .try_part_contact_spy_observation(&self.world.backend, part)
            .map_err(Self::Error::Part)?;
        let xyz = self
            .world
            .backend
            .collision_body_pool()
            .try_contact_spy_linear_velocity(observation)
            .map_err(Self::Error::Body)?;
        Ok(SkeletonRootVector { xyz, w: None })
    }
    fn grip_enabled(&mut self) -> Result<bool, Self::Error> {
        Ok(self.post_fields()?.grip_enabled)
    }
    fn grip_cosine(&mut self) -> Result<f32, Self::Error> {
        Ok(self.post_fields()?.grip_cosine)
    }
    fn grip_vector(&mut self) -> Result<SkeletonRootVector, Self::Error> {
        Ok(self.post_fields()?.grip_vector)
    }
}

impl<N: ReckoningNumeric, C: SkateboardPostCvars> PhysicalPlayerBoardPostServices<'_, N, C> {
    fn post_fields(
        &self,
    ) -> Result<
        &crate::skateboard_body_construction::SkateboardBodyPostFields,
        PhysicalPlayerBoardPostServiceError<N::Error, C::Error>,
    > {
        self.world
            .board_construction(self.board)
            .map_err(PhysicalPlayerBoardPostServiceError::Pool)?
            .and_then(crate::skateboard_body_construction::SkateboardBodyConstruction::post_fields)
            .ok_or(PhysicalPlayerBoardPostServiceError::Missing(
                "body constructor fields",
            ))
    }
}
