//! Live borrowed `WallRide` collaborators; no owner construction or query fallback.
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeError},
    collision_line_collection::CollisionLineInput,
    collision_line_numeric::HostCollisionLineNumeric,
    simulation_assembly_backend::constraint_session::world_line_query::{
        SimulationWorldLineError, SimulationWorldLineQueries,
    },
    simulation_dmo_assembly_part_session::dynamic_pool::SimulationDmoAssemblyPool,
    skateboard_foot_attributes::{
        SkateboardFootAttributeError, SkateboardFootAttributePointer, SkateboardFootAttributeRead,
        SkateboardFootVaultAttributes, wallride::WallRideTrajectoryAttributes,
    },
    skateboard_force_runtime::wallride::{
        WallRideHit, WallRideScalar, WallRideSource, WallRideVector,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
    static_ground_pool::StaticGroundPool,
};
use std::sync::{Arc, Mutex};
pub(crate) trait WallRideDebug {
    type Error;
    type Target;
    fn capture(&mut self) -> Result<Self::Target, Self::Error>;
    fn load(&mut self, target: &Self::Target) -> Result<bool, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum WallRideRuntimeError<D> {
    Missing(&'static str),
    Poisoned,
    Feet(SkateboardFootAttributeError),
    Layout(AttributeRuntimeError),
    Query(SimulationWorldLineError<crate::reckoning_numeric::ReckoningHostNumericError>),
    Debug(D),
}
impl<D: std::fmt::Debug> std::fmt::Display for WallRideRuntimeError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "wallride missing {s}"),
            Self::Poisoned => f.write_str("wallride processed lock poisoned"),
            Self::Feet(e) => write!(f, "wallride feet: {e}"),
            Self::Layout(e) => write!(f, "wallride layout: {e}"),
            Self::Query(e) => write!(f, "wallride World query: {e:?}"),
            Self::Debug(e) => write!(f, "wallride debug: {e:?}"),
        }
    }
}
/// Outer session must obtain these borrows from one verified board/Skeleton/
/// Ground pair. Query owners are the existing initialized World fields.
pub(crate) struct WallRideRuntime<'a, D> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) jump_flag: &'a mut bool,
    pub(crate) jump_height: &'a mut f32,
    pub(crate) jump_velocity: &'a mut SkeletonRootVector,
    pub(crate) feet: &'a mut SkateboardFootVaultAttributes,
    pub(crate) trajectory: &'a WallRideTrajectoryAttributes,
    pub(crate) queries: Option<&'a mut SimulationWorldLineQueries>,
    pub(crate) dmos: Option<&'a SimulationDmoAssemblyPool>,
    pub(crate) static_ground: &'a StaticGroundPool,
    pub(crate) debug: &'a mut D,
}
impl<D: WallRideDebug> WallRideSource for WallRideRuntime<'_, D> {
    type Error = WallRideRuntimeError<D::Error>;
    type Scalar = SkateboardFootAttributePointer;
    type TrajectoryLayout = Option<AttributeRuntimeData>;
    type DebugTarget = D::Target;
    fn capture_feet(&mut self, key: u64) -> Result<Self::Scalar, Self::Error> {
        self.feet.lookup(key).map_err(Self::Error::Feet)
    }
    fn load_feet(&mut self, value: &Self::Scalar) -> Result<f32, Self::Error> {
        self.feet.float(value).map_err(Self::Error::Feet)
    }
    fn feet_graph(&mut self, time: f32) -> Result<f32, Self::Error> {
        self.feet
            .try_wallride_graph(time)
            .map_err(Self::Error::Layout)
    }
    fn capture_trajectory_layout(&mut self) -> Result<Self::TrajectoryLayout, Self::Error> {
        self.trajectory.capture().map_err(Self::Error::Layout)
    }
    fn trajectory_height(&mut self, layout: &Self::TrajectoryLayout) -> Result<f32, Self::Error> {
        Ok(WallRideTrajectoryAttributes::height(layout.as_ref()))
    }
    fn vector(&mut self, field: WallRideVector) -> Result<SkeletonRootVector, Self::Error> {
        let input = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let (value, name) = match field {
            WallRideVector::GroundNormal => (
                input.prior_output().ground_normal().map(|v| v.value),
                "ground normal1d0",
            ),
            WallRideVector::Up => (
                input.prior_output().reckoning_up().map(|v| v.value),
                "up220",
            ),
            WallRideVector::Velocity => (
                input.prior_output().velocity().map(|v| v.value),
                "velocity190",
            ),
            WallRideVector::DeckPosition => (
                input.deck_transform().map(|m| m.columns()[3]),
                "deck position70",
            ),
        };
        value.ok_or(Self::Error::Missing(name))
    }
    fn scalar(&mut self, field: WallRideScalar) -> Result<f32, Self::Error> {
        let input = self.processed.lock().map_err(|_| Self::Error::Poisoned)?;
        let (value, name) = match field {
            WallRideScalar::Mass => (input.physical_mass(), "board mass970"),
            WallRideScalar::Gravity => (input.gravity().map(|v| v.value), "gravity964"),
        };
        value.ok_or(Self::Error::Missing(name))
    }
    fn wheels(&mut self) -> Result<i32, Self::Error> {
        self.processed
            .lock()
            .map_err(|_| Self::Error::Poisoned)?
            .physics_state_fields()
            .num_wheels_in_contact
            .ok_or(Self::Error::Missing("wheels90c"))
    }
    fn capture_debug(&mut self) -> Result<Self::DebugTarget, Self::Error> {
        self.debug.capture().map_err(Self::Error::Debug)
    }
    fn debug_value(&mut self, target: &Self::DebugTarget) -> Result<bool, Self::Error> {
        self.debug.load(target).map_err(Self::Error::Debug)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.debug.require_overlay().map_err(Self::Error::Debug)
    }
    fn line(
        &mut self,
        start: SkeletonRootVector,
        end: SkeletonRootVector,
    ) -> Result<Option<WallRideHit>, Self::Error> {
        let queries = self
            .queries
            .as_mut()
            .ok_or(Self::Error::Missing("same World line queries"))?;
        let dmos = self
            .dmos
            .ok_or(Self::Error::Missing("same World DMO owner"))?;
        let result = queries
            .try_line_test(
                self.static_ground,
                dmos,
                CollisionLineInput {
                    start,
                    end,
                    fatness: 0.0,
                },
                "Skateboard::Toolkit_CalcWallRideForce",
                &mut HostCollisionLineNumeric,
            )
            .map_err(Self::Error::Query)?;
        Ok(result.map(|hit| WallRideHit {
            position: hit.position,
            normal: hit.normal,
        }))
    }
    fn clear_jump_flag(&mut self) -> Result<(), Self::Error> {
        *self.jump_flag = false;
        Ok(())
    }
    fn clear_jump_height(&mut self) -> Result<(), Self::Error> {
        *self.jump_height = 0.0;
        Ok(())
    }
    fn publish_jump_flag(&mut self) -> Result<(), Self::Error> {
        *self.jump_flag = true;
        Ok(())
    }
    fn publish_jump_velocity(&mut self, velocity: SkeletonRootVector) -> Result<(), Self::Error> {
        // Requires the approved SAME-field typed migration; no extra retained field.
        *self.jump_velocity = velocity;
        Ok(())
    }
}
