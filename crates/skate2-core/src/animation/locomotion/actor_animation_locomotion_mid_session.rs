//! Selected SkaterManager::UpdateLocomotion825f53b0 at SimUpdate1 8289f934.
use super::{ActorConstruction, ActorConstructionError};
use crate::{
    actor_animation_construction::locomotion_mid::{MidLocomotionContinuation, MidLocomotionError},
    generated_motion_load::runtime::MotionUpdateError,
    input_cvar_runtime::InputCvarReferences,
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::HostReckoningNumeric,
    simulation_assembly_backend::constraint_session::step_selector::StepSelectorError,
    static_ground_pool::StaticGroundPool,
};
#[derive(Debug)]
pub(crate) enum ActorMidLocomotionError {
    Actor(ActorConstructionError),
    Interface(MotionUpdateError),
    Animation(MidLocomotionError),
    Step(StepSelectorError),
    Lock,
}
impl ActorConstruction {
    pub(crate) fn try_mid_physics_locomotion(
        &self,
        update_mask: u32,
        cvars: &InputCvarReferences,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
    ) -> Result<(), ActorMidLocomotionError> {
        use ActorMidLocomotionError as E;
        // Manager resolves the actual cached/registered ISkaterAnim before
        // checking Actor flags. No AI-controller exclusion in this callback.
        let animation = self.lifecycle_animation(2).map_err(E::Interface)?;
        let flags = self
            .prefix()
            .map_err(E::Actor)?
            .fields()
            .ok_or(E::Actor(ActorConstructionError::ActorFieldsUninitialized))?
            .flags;
        if flags & update_mask == 0 {
            return Ok(());
        }
        let continuation = animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_mid_locomotion_prefix(cvars)
            .map_err(E::Animation)?;
        let MidLocomotionContinuation::Active(launch) = continuation else {
            return Ok(());
        };
        if let Some(launch) = launch {
            // The real StepSelector takes the same animation mutex to mark the
            // second future pending. No animation/Actor guard survives here.
            world
                .try_launch_step_selector(
                    &animation,
                    launch.first,
                    launch.count,
                    launch.world_only,
                    ground,
                )
                .map_err(E::Step)?;
        }
        animation
            .lock()
            .map_err(|_| E::Lock)?
            .try_mid_locomotion_suffix(&mut HostReckoningNumeric)
            .map_err(E::Animation)
    }
}
