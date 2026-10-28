//! Borrow the already selected world for ForcePhysics.Begin; `constraint_session` child.
//! No shared-world lock, shadow Skeleton field, or inferred NULL is introduced.
use super::physical_island::SimulationPhysicalBodyError;
use crate::{
    actor_construction::motion_runtime_session::neutral_lifecycle::{
        ForcePhysicsWriter, WalkingWorldError,
    },
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, force_physics::MotionForcePhysicsError,
    },
    generated_motion_load::{construct::control::PhysicsAnimationMode, runtime::MotionUpdateError},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) enum ForcePhysicsCapabilityError {
    Owner(SimulationPhysicalBodyError),
    Pool(MotionForcePhysicsError),
    ForeignOwner,
    MissingPool,
}
impl std::fmt::Display for ForcePhysicsCapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owner(error) => write!(f, "ForcePhysics physical membership: {error}"),
            Self::Pool(error) => write!(f, "ForcePhysics Skeleton pool: {error}"),
            Self::ForeignOwner => {
                f.write_str("ForcePhysics Skeleton belongs to another physical owner")
            }
            Self::MissingPool => {
                f.write_str("ForcePhysics requires the selected world's Skeleton pool")
            }
        }
    }
}
impl std::error::Error for ForcePhysicsCapabilityError {}

/// A concrete selected Skeleton capability. Its handle is cloned from the
/// successful actual construction publication, never reconstructed from IDs.
/// New performs no lookup: source interface validation in the Actor caller must
/// precede the actual Skeleton operation and its possible failure.
pub(crate) struct MotionForcePhysicsWriter<'world, 'handle> {
    world: &'world mut PhysicalAssemblyWorld,
    skeleton: &'handle SimulationSkaterAssemblyHandle,
    ground: Option<&'handle crate::static_ground_pool::StaticGroundPool>,
}
impl<'world, 'handle> MotionForcePhysicsWriter<'world, 'handle> {
    pub(crate) fn new(
        world: &'world mut PhysicalAssemblyWorld,
        skeleton: &'handle SimulationSkaterAssemblyHandle,
    ) -> Self {
        Self {
            world,
            skeleton,
            ground: None,
        }
    }
    pub(crate) fn new_with_walking(
        world: &'world mut PhysicalAssemblyWorld,
        skeleton: &'handle SimulationSkaterAssemblyHandle,
        ground: &'handle crate::static_ground_pool::StaticGroundPool,
    ) -> Self {
        Self {
            world,
            skeleton,
            ground: Some(ground),
        }
    }
    fn write(
        &mut self,
        expected_owner: &Arc<PhysicalPlayerAssemblyOwner>,
        mode: PhysicsAnimationMode,
    ) -> Result<(), ForcePhysicsCapabilityError> {
        use ForcePhysicsCapabilityError as E;
        // Validate the actual manager's membership and physical owner relation.
        // Pool setter additionally validates backend, handle generation/kind and
        // the same owner before touching SkeletonNormalAssembly+3df0.
        let owner = self
            .world
            .physical_body_owner_for_contacts(self.skeleton)
            .map_err(E::Owner)?;
        if !Arc::ptr_eq(owner, expected_owner) {
            return Err(E::ForeignOwner);
        }
        self.world
            .skaters
            .as_mut()
            .ok_or(E::MissingPool)?
            .try_motion_force_physics(&self.world.backend, self.skeleton, expected_owner, mode)
            .map_err(E::Pool)
    }
}
impl ForcePhysicsWriter for MotionForcePhysicsWriter<'_, '_> {
    fn walking_debug_overlay(
        &mut self,
    ) -> Result<crate::debug_overlay::DebugOverlaySlot, WalkingWorldError> {
        Ok(self.world.debug_overlay.clone())
    }
    fn launch_walking_steps(
        &mut self,
        animation: &Arc<
            std::sync::Mutex<crate::actor_construction::animation_mount::MountedActorAnimation>,
        >,
        first: usize,
        count: usize,
        world_only: bool,
    ) -> Result<(), WalkingWorldError> {
        let ground = self
            .ground
            .ok_or(WalkingWorldError::Missing("selected static ground"))?;
        self.world
            .try_launch_step_selector(animation, first, count, world_only, ground)
            .map_err(WalkingWorldError::Step)
    }

    fn write_skeleton_mode(
        &mut self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        mode: PhysicsAnimationMode,
    ) -> Result<(), MotionUpdateError> {
        self.write(owner, mode)
            .map_err(|error| MotionUpdateError::ForcePhysics(Box::new(error)))
    }
}

// Native GetSkeleton_2==NULL is an allowed no-op, but the completed selected
// caller never supplies NULL: factory.skeleton() is a published concrete handle.
// This adapter intentionally cannot fabricate a nullable result from a missing
// pool, stale handle, wrong kind, failed membership check or wrong owner. A future
// partial-construction caller needs a separately issued nullable-owner proof.
