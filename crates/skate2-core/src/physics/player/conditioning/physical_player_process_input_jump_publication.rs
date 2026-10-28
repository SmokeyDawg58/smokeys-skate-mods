//! Actual jump publication in `Update1`82e038f0..3a80, after the grind stage.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::skater_pool::{
        SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_phys_out::construct::prior_input::PriorInputReadError,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::{fmt, sync::Arc};
#[derive(Debug)]
pub(crate) enum JumpPublicationError {
    Missing(&'static str),
    Poisoned,
    Pair(SimulationSkeletonJumpPairError),
    Output(PriorInputReadError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl fmt::Display for JumpPublicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "jump publication missing {field}"),
            Self::Poisoned => f.write_str("jump publication owner lock poisoned"),
            Self::Pair(e) => write!(f, "jump publication pair: {e:?}"),
            Self::Output(e) => write!(f, "jump publication output: {e:?}"),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
fn need<T>(value: Option<T>, field: &'static str) -> Result<T, JumpPublicationError> {
    value.ok_or(JumpPublicationError::Missing(field))
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_publish_pair_jump_timing(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        cvars: &InputCvarReferences,
    ) -> Result<(), JumpPublicationError> {
        use JumpPublicationError as E;
        let owner = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |_, owner| Arc::clone(owner))
            .map_err(E::Pair)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let mut processed = processed.lock().map_err(|_| E::Poisoned)?;
        publish(&owner, &mut processed, cvars, &self.debug_overlay)
    }
}
#[expect(
    clippy::too_many_lines,
    reason = "ordered native reads preserve partial writes on late errors"
)]
fn publish(
    owner: &PhysicalPlayerAssemblyOwner,
    processed: &mut SkeletonProcessedContact,
    cvars: &InputCvarReferences,
    overlay: &DebugOverlaySlot,
) -> Result<(), JumpPublicationError> {
    use JumpPublicationError as E;
    let wall_exit = owner
        .phys_out()
        .lock()
        .map_err(|_| E::Poisoned)?
        .try_guarded_ground_output()
        .map_err(E::Output)?
        .wall_ride_exit;
    if wall_exit {
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .frames_since_jump = Some(0);
    }
    {
        let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        state.frames_since_jump =
            Some(need(state.frames_since_jump, "HiLOD framesSinceJump")?.wrapping_add(1));
    }
    // Native lhz+8d0 reads the HIGH halfword on big-endian PPC.
    let forced = need(processed.physics_state_fields().flags_8d0, "flags8d0")? & 0x0001_0000 != 0;
    let (flags, known) = processed.known_flags_8c4();
    if known & 0x0040_0000 == 0 {
        return Err(E::Missing("jump event flag8c4"));
    }
    let jump = flags & 0x0040_0000 != 0;
    let reset = if forced {
        true
    } else if jump {
        owner
            .prefix()
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .ok_or(E::Missing("HiLOD prefix"))?
            .non_collision_flags
            & 0x4000_0000
            != 0
    } else {
        false
    };
    if (jump || forced) && std::env::var_os("SKATE2_TRACE_KNOWN_AIR").is_some() {
        let current = owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .current;
        eprintln!(
            "known air: update1 state={current:?} start_jump={jump} foot_jump={forced} pop_edge={reset}"
        );
    }
    if reset {
        if cvars.try_draw_jump_debug_info().map_err(E::Cvar)? {
            overlay.require().map_err(E::Overlay)?;
        }
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .frames_since_jump = Some(0);
        owner
            .prefix()
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_mut()
            .ok_or(E::Missing("HiLOD prefix"))?
            .non_collision_flags &= !0x4000_0000;
    } else {
        let mut prefix = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        prefix.non_collision_flags =
            (prefix.non_collision_flags & !0x4000_0000) | if jump { 0 } else { 0x4000_0000 };
        processed.clear_consumed_jump_event();
    }
    processed.physics_state_fields_mut().frames_since_jump = Some(need(
        owner
            .physics_state()
            .lock()
            .map_err(|_| E::Poisoned)?
            .frames_since_jump,
        "HiLOD framesSinceJump",
    )?);
    let triggered = owner
        .phys_out()
        .lock()
        .map_err(|_| E::Poisoned)?
        .try_guarded_air_output()
        .map_err(E::Output)?
        .state_triggered_jump;
    if triggered {
        owner
            .prefix()
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_mut()
            .ok_or(E::Missing("HiLOD prefix"))?
            .frames_since_jump_correct = 1;
        // A second source guard precedes the full vector load at3a4c.
        let velocity = owner
            .phys_out()
            .lock()
            .map_err(|_| E::Poisoned)?
            .try_guarded_air_output()
            .map_err(E::Output)?
            .state_intended_jump_vel;
        owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .intended_jump_velocity = Some(velocity);
    } else {
        let mut prefix = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        prefix.frames_since_jump_correct = prefix.frames_since_jump_correct.wrapping_add(1);
    }
    processed
        .physics_state_fields_mut()
        .frames_since_jump_correct = Some(
        owner
            .prefix()
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .ok_or(E::Missing("HiLOD prefix"))?
            .frames_since_jump_correct,
    );
    processed
        .physics_state_fields_mut()
        .state_intended_jump_velocity = Some(need(
        owner
            .tail()
            .lock()
            .map_err(|_| E::Poisoned)?
            .intended_jump_velocity,
        "HiLOD intended jump velocity",
    )?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{JumpPublicationError, publish};
    use crate::{
        debug_overlay::DebugOverlaySlot,
        input_cvar_runtime::{InputCvarManager, InputCvarReferences},
        physical_player_assembly::{
            hilod_prefix::PhysicalHiLodPrefix, tests::issued_owner_for_grind_tests,
        },
        physical_player_phys_out::construct::PhysOutAllocation,
        skeleton_processed_contact::SkeletonProcessedContact,
        skeleton_root_transforms::SkeletonRootVector,
    };
    #[test]
    fn big_endian_flag_samples_debug_late_and_preserves_increment_on_error() {
        let (_world, owner) = issued_owner_for_grind_tests();
        owner
            .phys_out()
            .lock()
            .unwrap()
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        *owner.prefix().lock().unwrap() = Some(PhysicalHiLodPrefix::construct_in_sim_allocation());
        owner.physics_state().lock().unwrap().frames_since_jump = Some(7);
        owner
            .tail()
            .lock()
            .unwrap()
            .initialize_post_ground_jump_velocity();
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let refs = InputCvarReferences::new_static_initialized();
        let overlay = DebugOverlaySlot::default();
        // Low bit is outside the source lhz mask, so no debug target is read.
        processed.physics_state_fields_mut().flags_8d0 = Some(1);
        publish(&owner, &mut processed, &refs, &overlay).unwrap();
        assert_eq!(processed.physics_state_fields().frames_since_jump, Some(8));
        processed.physics_state_fields_mut().flags_8d0 = Some(0x0001_0000);
        assert!(matches!(
            publish(&owner, &mut processed, &refs, &overlay),
            Err(JumpPublicationError::Cvar(_))
        ));
        assert_eq!(
            owner.physics_state().lock().unwrap().frames_since_jump,
            Some(9)
        );
        assert_eq!(processed.physics_state_fields().frames_since_jump, Some(8));
        let mut manager = InputCvarManager::new();
        manager.try_link(&refs).unwrap();
        publish(&owner, &mut processed, &refs, &overlay).unwrap();
        assert_eq!(processed.physics_state_fields().frames_since_jump, Some(0));
    }
    #[test]
    fn actual_air_vector_preserves_known_and_unknown_w() {
        let (_world, owner) = issued_owner_for_grind_tests();
        owner
            .phys_out()
            .lock()
            .unwrap()
            .try_construct_complete(|_| Ok::<_, ()>(PhysOutAllocation::Host))
            .unwrap();
        *owner.prefix().lock().unwrap() = Some(PhysicalHiLodPrefix::construct_in_sim_allocation());
        owner.physics_state().lock().unwrap().frames_since_jump = Some(7);
        let mut processed = SkeletonProcessedContact::new();
        processed.reset();
        let refs = InputCvarReferences::new_static_initialized();
        for w in [Some(17.0), None] {
            let vector = SkeletonRootVector {
                xyz: [2.0, 3.0, 5.0],
                w,
            };
            owner
                .phys_out()
                .lock()
                .unwrap()
                .edit_prior_jump_test_outputs(|air, ground| {
                    air.state_triggered_jump = true;
                    air.state_intended_jump_vel = vector;
                    ground.wall_ride_exit = true;
                });
            publish(&owner, &mut processed, &refs, &DebugOverlaySlot::default()).unwrap();
            assert_eq!(processed.physics_state_fields().frames_since_jump, Some(1));
            assert_eq!(
                processed.physics_state_fields().frames_since_jump_correct,
                Some(1)
            );
            assert_eq!(
                processed
                    .physics_state_fields()
                    .state_intended_jump_velocity,
                Some(vector)
            );
            assert_eq!(
                owner.tail().lock().unwrap().intended_jump_velocity,
                Some(vector)
            );
        }
    }
}
