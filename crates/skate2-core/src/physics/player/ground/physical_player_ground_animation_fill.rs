//! Same retained `GroundAnimation.FillPhysOut82d83fc0`.
use super::{PhysicalPlayerPhysOut, air_post_solve::AirFillError};
use crate::{
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::{
        PhysicalPlayerAssemblyOwner,
        remaining_states::{StateKind, StatePayload},
    },
    physical_player_physics_state::CurrentPhysicalState,
    simulation_assembly_backend::constraint_session::update5_state_publication::Update5StateRuntimeError,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;
type Error = AirFillError<Update5StateRuntimeError>;

impl PhysicalPlayerPhysOut {
    fn try_fill_ground_animation(
        &mut self,
        jumped: bool,
        velocity: impl FnOnce() -> Result<SkeletonRootVector, Update5StateRuntimeError>,
    ) -> Result<(), Error> {
        if !jumped {
            return Ok(());
        }
        self.air_fill_guard().map_err(Error::Output)?;
        self.concrete.air.as_mut().unwrap().state_triggered_jump = true;
        self.air_fill_guard().map_err(Error::Output)?;
        self.concrete.air.as_mut().unwrap().state_intended_jump_vel =
            velocity().map_err(Error::Source)?;
        Ok(())
    }
}
pub(crate) fn try_fill_current_ground_animation(
    owner: &Arc<PhysicalPlayerAssemblyOwner>,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
) -> Result<(), Error> {
    use Update5StateRuntimeError as E;
    let state = owner
        .physics_state()
        .lock()
        .map_err(|_| Error::Source(E::Poisoned))?;
    if state.current != CurrentPhysicalState::GroundAnimation {
        return Err(Error::Source(E::UnsupportedState(state.current)));
    }
    let states = owner
        .remaining_states()
        .lock()
        .map_err(|_| Error::Source(E::Poisoned))?;
    let Some(StatePayload::GroundAnimation(actual)) = states
        .states
        .get(&StateKind::GroundAnimation)
        .filter(|s| s.published)
        .and_then(|s| s.value.as_ref())
    else {
        return Err(Error::Source(E::Missing("GroundAnimation")));
    };
    let bindings = &actual.base.physical;
    if !bindings.board.same_as(board)
        || !bindings.skeleton.same_as(skeleton)
        || !bindings
            .owner
            .upgrade()
            .is_some_and(|v| Arc::ptr_eq(&v, owner))
    {
        return Err(Error::Source(E::ForeignOwner));
    }
    let jumped = actual
        .jumped_this_frame
        .ok_or(Error::Source(E::Missing("jumped this frame")))?;
    if !jumped {
        return Ok(());
    }
    owner
        .phys_out()
        .lock()
        .map_err(|_| Error::Source(E::Poisoned))?
        .try_fill_ground_animation(jumped, || {
            actual.jump_vel.ok_or(E::Missing("jump velocity"))
        })
}

#[cfg(test)]
mod tests {
    use super::super::PhysOutAllocation;
    use super::*;
    #[test]
    fn inactive_fill_needs_no_constructed_output_or_velocity() {
        PhysicalPlayerPhysOut::new()
            .try_fill_ground_animation(false, || panic!("inactive"))
            .unwrap();
    }
    #[test]
    fn late_velocity_failure_preserves_trigger_store() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        assert!(matches!(
            output.try_fill_ground_animation(true, || Err(Update5StateRuntimeError::Missing(
                "velocity"
            ))),
            Err(Error::Source(_))
        ));
        assert!(output.concrete.air.as_ref().unwrap().state_triggered_jump);
    }
    #[test]
    fn active_fill_copies_all_velocity_bits() {
        let mut output = PhysicalPlayerPhysOut::new();
        output
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        let bits = [0x8000_0000, 0x7fc1_2345, 0xbf80_0000, 0x3f12_3456];
        output
            .try_fill_ground_animation(true, || {
                Ok(SkeletonRootVector::known(bits.map(f32::from_bits)))
            })
            .unwrap();
        let copied = output
            .concrete
            .air
            .as_ref()
            .unwrap()
            .state_intended_jump_vel;
        assert_eq!(copied.xyz.map(f32::to_bits), bits[..3]);
        assert_eq!(copied.w.unwrap().to_bits(), bits[3]);
    }
}
