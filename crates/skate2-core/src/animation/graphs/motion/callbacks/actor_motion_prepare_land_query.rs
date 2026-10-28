//! Actual same-Actor `IsTimeToPrepareLand` query; `motion_runtime_session` child.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    input_cvar_runtime::{
        InputCvarReferences,
        body_tilt::{BodyTiltMotionAttributes, prepare_land::PrepareLandAttributes},
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::PhysOutSystemVector,
    physics::PointGraph,
    reckoning_numeric::ReckoningVector,
};
use std::sync::Arc;
#[path = "motion_prepare_land_query.rs"]
pub(crate) mod kernel;
use kernel::{Flag, PrepareLandSource, Tuning, Value};
pub(crate) struct PrepareLandRuntime<'a> {
    pub(crate) attributes: Option<&'a PrepareLandAttributes>,
    pub(crate) body_tilt: Option<&'a BodyTiltMotionAttributes>,
    pub(crate) cvars: Option<&'a InputCvarReferences>,
}
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    runtime: PrepareLandRuntime<'a>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("preland captured IPhysical"))
    }
    fn attributes(&self) -> Result<&PrepareLandAttributes, Error> {
        let attributes = self
            .runtime
            .attributes
            .ok_or(Error::MissingOwner("preland retained global attributes"))?;
        attributes.check_vault(&self.physical()?.inputs().vault)?;
        Ok(attributes)
    }
}
impl PrepareLandSource for Source<'_> {
    type Error = Error;
    fn resolve_physical(&mut self) -> Result<bool, Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        let owner = Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        );
        if self
            .physical
            .as_ref()
            .is_some_and(|previous| !Arc::ptr_eq(previous, &owner))
        {
            return Err(Error::ForeignOwner("preland resampled IPhysical"));
        }
        self.physical = Some(owner);
        Ok(true)
    }
    fn flag(&mut self, field: Flag) -> Result<bool, Error> {
        let output = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        Ok(match field {
            Flag::TargetingWallRide => {
                output
                    .try_air_output()
                    .map_err(Error::PhysOut)?
                    .is_targeting_wall_ride
            }
            Flag::CanLandOnBoard => {
                output
                    .try_offboard_output()
                    .map_err(Error::PhysOut)?
                    .can_land_on_skateboard
            }
            Flag::AirTrajectoryValid => {
                output
                    .try_air_output()
                    .map_err(Error::PhysOut)?
                    .is_traj_valid
            }
            Flag::OffboardTrajectoryReady => {
                output
                    .try_offboard_output()
                    .map_err(Error::PhysOut)?
                    .trajectory_ready
            }
        })
    }
    fn value(&mut self, field: Value) -> Result<f32, Error> {
        let output = self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?;
        Ok(match field {
            Value::LandingNormalY => output
                .try_air_output()
                .map_err(Error::PhysOut)?
                .landing_normal
                .operand()[1],
            Value::LastTiltX => output
                .try_animation_output()
                .map_err(Error::PhysOut)?
                .last_anim_tilt
                .operand()[0],
            Value::VelocityY => output
                .try_landing_system_vector(PhysOutSystemVector::CentreOfMassVelocity)
                .map_err(Error::PhysOut)?
                .operand()[1],
            Value::OffboardTime => {
                output
                    .try_offboard_output()
                    .map_err(Error::PhysOut)?
                    .time_to_land
            }
            Value::ComToDeck => {
                output
                    .try_animation_output()
                    .map_err(Error::PhysOut)?
                    .centre_of_mass_to_deck
            }
            Value::CollisionTime => {
                output
                    .try_air_output()
                    .map_err(Error::PhysOut)?
                    .traj_collision_time
            }
            Value::TrajectoryNormalY => output
                .try_air_output()
                .map_err(Error::PhysOut)?
                .traj_coll_normal
                .operand()[1],
        })
    }
    fn tuning(&mut self, field: Tuning) -> Result<f32, Error> {
        self.attributes()?.scalar(field, self.runtime.body_tilt)
    }
    fn landing_graph(&mut self) -> Result<PointGraph<8>, Error> {
        self.attributes()?.landing_graph()
    }
    fn offboard_preland_time(&mut self) -> Result<f32, Error> {
        self.runtime
            .cvars
            .ok_or(Error::MissingOwner("preland live cvar239"))?
            .try_offboard_air_preland_time()
            .map_err(Error::Cvar)
    }
}
impl ActorConstruction {
    /// `HasTiltToLargeForPreland`82c9c1c0 calls Motion virtual+1c/825f48d0.
    pub(crate) fn motion_has_tilt_too_large(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        runtime: PrepareLandRuntime<'_>,
    ) -> Result<bool, Error> {
        // Native condition returns false for a genuinely absent Motion interface.
        if context.entity() != Some(&self.authority.entity_handle()) {
            return Err(Error::ForeignOwner("preland condition entity"));
        }
        if self.prefix().map_err(Error::Actor)?.graph_interfaces[2].is_none() {
            return Ok(false);
        }
        self.check_motion_runtime_context(context)?;
        kernel::has_tilt_too_large(&mut Source {
            actor: self,
            context,
            runtime,
            physical: None,
        })
    }

    pub(crate) fn motion_time_to_prepare_land(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        runtime: PrepareLandRuntime<'_>,
    ) -> Result<bool, Error> {
        kernel::time_to_prepare_land(&mut Source {
            actor: self,
            context,
            runtime,
            physical: None,
        })
    }
}
