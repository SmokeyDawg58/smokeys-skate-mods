//! Non-owning constructor service pointers for one Actor breadcrumb manager.
//! Three native Simulation interfaces share the selected World occurrence;
//! the LivingWorld Manager and its MeshStore remain distinct retained owners.

use crate::{
    living_world_grind_services::LivingWorldGrindServices,
    living_world_mesh_store::{MeshStore, MeshStoreError},
    physical_player_assembly::PhysicalAssemblyWorld,
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug)]
pub(crate) enum ActorBreadCrumbOwnerError {
    LivingWorldLock,
    Mesh(MeshStoreError),
    WorldExpired,
    LivingWorldExpired,
    MeshExpired,
    WorldLock,
    ForeignWorld,
    ForeignLivingWorld,
    ForeignMeshStore,
}

/// Native `Implementation` +0x50/+0x54/+0x58 all refer to the same Simulation
/// instance. This one weak World handle represents that shared identity, not
/// three fabricated interface implementations or a separate network flag.
#[derive(Clone)]
pub(crate) struct ActorBreadCrumbConstructorOwners {
    simulation: Weak<Mutex<PhysicalAssemblyWorld>>,
    living_world: Weak<Mutex<LivingWorldGrindServices>>,
    mesh_store: Weak<MeshStore>,
}

/// All handles are upgraded from the original constructor identities. Callers
/// may lock `simulation` only after `try_verify` has released its own guards.
pub(crate) struct VerifiedBreadCrumbOwners {
    pub(crate) simulation: Arc<Mutex<PhysicalAssemblyWorld>>,
    pub(crate) living_world: Arc<Mutex<LivingWorldGrindServices>>,
    pub(crate) mesh_store: Arc<MeshStore>,
}

impl ActorBreadCrumbConstructorOwners {
    /// No owners: every reached query reports an expired chain.
    #[cfg(test)]
    pub(crate) fn detached() -> Self {
        Self { simulation: Weak::new(), living_world: Weak::new(), mesh_store: Weak::new() }
    }

    /// Validate the Simulation interface identity without forcing the later
    /// MeshStore/LW owner checks before their native selector gates are reached.
    pub(crate) fn try_verify_world(
        &self,
        simulation: &Arc<Mutex<PhysicalAssemblyWorld>>,
    ) -> Result<(), ActorBreadCrumbOwnerError> {
        let retained = self
            .simulation
            .upgrade()
            .ok_or(ActorBreadCrumbOwnerError::WorldExpired)?;
        if !Arc::ptr_eq(&retained, simulation) {
            return Err(ActorBreadCrumbOwnerError::ForeignWorld);
        }
        Ok(())
    }

    /// The LW predicate needs the retained Manager, but not the MeshStore.
    pub(crate) fn try_verify_living_world_in_world(
        &self,
        simulation: &Arc<Mutex<PhysicalAssemblyWorld>>,
        world: &PhysicalAssemblyWorld,
    ) -> Result<Arc<Mutex<LivingWorldGrindServices>>, ActorBreadCrumbOwnerError> {
        use ActorBreadCrumbOwnerError as E;
        self.try_verify_world(simulation)?;
        let living_world = self.living_world.upgrade().ok_or(E::LivingWorldExpired)?;
        if !Arc::ptr_eq(world.grind_services.living_world(), &living_world) {
            return Err(E::ForeignLivingWorld);
        }
        Ok(living_world)
    }

    /// Capture at Actor::Actor 0x825ff350..438, after the physical constructor
    /// returns. The World guard is the selected caller's already-held World;
    /// it supplies its one constructed LivingWorld and that Manager's MeshStore.
    pub(crate) fn try_capture_selected(
        simulation: &Arc<Mutex<PhysicalAssemblyWorld>>,
        world: &PhysicalAssemblyWorld,
    ) -> Result<Self, ActorBreadCrumbOwnerError> {
        use ActorBreadCrumbOwnerError as E;
        let living_world = Arc::clone(world.grind_services.living_world());
        let mesh_store = living_world
            .lock()
            .map_err(|_| E::LivingWorldLock)?
            .try_mesh_store()
            .map_err(E::Mesh)?;
        Ok(Self {
            simulation: Arc::downgrade(simulation),
            living_world: Arc::downgrade(&living_world),
            mesh_store: Arc::downgrade(&mesh_store),
        })
    }

    /// Validate the owner chain at a reached query. A stale or foreign owner
    /// cannot silently substitute an empty MeshStore or another World.
    pub(crate) fn try_verify(&self) -> Result<VerifiedBreadCrumbOwners, ActorBreadCrumbOwnerError> {
        use ActorBreadCrumbOwnerError as E;
        let simulation = self.simulation.upgrade().ok_or(E::WorldExpired)?;
        let world = simulation.lock().map_err(|_| E::WorldLock)?;
        self.try_verify_in_world(&simulation, &world)
    }

    /// The selected pre-solve already holds this World's mutex. Verify the
    /// captured identity against that guard without locking World again.
    pub(crate) fn try_verify_in_world(
        &self,
        simulation: &Arc<Mutex<PhysicalAssemblyWorld>>,
        world: &PhysicalAssemblyWorld,
    ) -> Result<VerifiedBreadCrumbOwners, ActorBreadCrumbOwnerError> {
        use ActorBreadCrumbOwnerError as E;
        let living_world = self.try_verify_living_world_in_world(simulation, world)?;
        let retained = Arc::clone(simulation);
        let mesh_store = self.mesh_store.upgrade().ok_or(E::MeshExpired)?;
        let manager = living_world.lock().map_err(|_| E::LivingWorldLock)?;
        let current_mesh = manager.try_mesh_store().map_err(E::Mesh)?;
        if !Arc::ptr_eq(&current_mesh, &mesh_store) {
            return Err(E::ForeignMeshStore);
        }
        drop(manager);
        Ok(VerifiedBreadCrumbOwners {
            simulation: retained,
            living_world,
            mesh_store,
        })
    }
}
