//! Same-owner StoreLandingData.Update82ca1d00 adapter.
use super::{
    ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError as Error,
};
use crate::{
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physical_player_phys_out::PhysOutSystemVector,
    physics::FilteredPhysicsState,
    reckoning_numeric::{ReckoningVector, host_dot3},
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
#[path = "motion_store_landing_data.rs"]
pub(crate) mod kernel;
use kernel::LandingSource;
struct Source<'a> {
    actor: &'a ActorConstruction,
    context: &'a BoundMotionGraphContext<ActorIdentity>,
    physical: Option<Arc<PhysicalPlayerAssemblyOwner>>,
    motion: Option<Arc<()>>,
}
impl Source<'_> {
    fn physical(&self) -> Result<&Arc<PhysicalPlayerAssemblyOwner>, Error> {
        self.physical
            .as_ref()
            .ok_or(Error::MissingOwner("StoreLandingData IPhysical"))
    }
    fn system(&self, field: PhysOutSystemVector) -> Result<SkeletonRootVector, Error> {
        self.physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_landing_system_vector(field)
            .map_err(Error::PhysOut)
    }
}
impl LandingSource for Source<'_> {
    type Error = Error;
    fn resolve_interfaces(&mut self) -> Result<(), Error> {
        self.actor.check_motion_runtime_context(self.context)?;
        self.physical = Some(Arc::clone(
            self.actor
                .cached_physical_allocation()
                .map_err(Error::Physical)?
                .owner(),
        ));
        let actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_ref()
            .and_then(|g| g.motion())
            .filter(|_| actor.motion_graph_published)
            .ok_or(Error::MissingOwner(
                "StoreLandingData cached ISkaterMotionGraph",
            ))?;
        if !actor.graph_interfaces[2]
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("StoreLandingData ISkaterMotionGraph"));
        }
        self.motion = Some(Arc::clone(graph.occurrence()));
        Ok(())
    }
    fn is_air(&mut self) -> Result<bool, Error> {
        Ok(self
            .physical()?
            .phys_out()
            .lock()
            .map_err(|_| Error::Lock)?
            .try_filtered_output()
            .map_err(Error::PhysOut)?
            .filtered_physics_state
            == FilteredPhysicsState::Air)
    }
    fn missing_instance(&mut self) -> Error {
        Error::MissingOwner("StoreLandingData instance")
    }
    fn set_landing_velocity(&mut self, value: f32) -> Result<(), Error> {
        let mut actor = self.actor.prefix().map_err(Error::Actor)?;
        let graph = actor
            .graphs
            .as_mut()
            .and_then(|g| g.motion_mut())
            .ok_or(Error::MissingOwner("StoreLandingData captured Motion"))?;
        if !self
            .motion
            .as_ref()
            .is_some_and(|p| Arc::ptr_eq(p, graph.occurrence()))
        {
            return Err(Error::ForeignOwner("StoreLandingData captured Motion"));
        }
        graph
            .state
            .as_mut()
            .ok_or(Error::MissingOwner("Motion state"))?
            .last_good_landing_velocity = value;
        Ok(())
    }
    fn centre_velocity(&mut self) -> Result<SkeletonRootVector, Error> {
        self.system(PhysOutSystemVector::CentreOfMassVelocity)
    }
    fn up(&mut self) -> Result<SkeletonRootVector, Error> {
        self.system(PhysOutSystemVector::Up)
    }
    fn dot(&mut self, a: SkeletonRootVector, b: SkeletonRootVector) -> Result<f32, Error> {
        // Native vmsum3fp; shared host reduction policy is not Xenon bit parity.
        Ok(host_dot3(a.operand(), b.operand()))
    }
}
impl ActorConstruction {
    pub(crate) fn motion_runtime_store_landing_data(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        instance: Option<&mut kernel::LandingInstance>,
    ) -> Result<(), Error> {
        kernel::update(
            &mut Source {
                actor: self,
                context,
                physical: None,
                motion: None,
            },
            instance,
        )
    }
}
