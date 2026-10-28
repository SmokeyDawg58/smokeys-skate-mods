//! Consecutive source `Update1_ProcessInput82e02174..242c`.
use crate::physics_state_ids::PHYSICS_CATEGORY_GROUND;
use super::{
    physical_update1_prefix::Update1AfterTeleportCheck,
    physical_wheel_material::PhysicalPlayerSurfaceRuntimeError,
};
use crate::{
    actor_construction::physical_mount::input_query::ActorPhysicalInputError,
    dynamic_assembly_session::skater_pool::SimulationSkaterAssemblyHandle,
    physical_player_assembly::PhysicalAssemblyWorld,
};

pub(crate) struct Update1BeforePriorCopy {
    pub(crate) continuation: Update1AfterTeleportCheck,
}
#[derive(Debug)]
pub(crate) enum ProcessInputControlsError {
    Missing(&'static str),
    Poisoned,
    Input(ActorPhysicalInputError),
    Surface(PhysicalPlayerSurfaceRuntimeError),
}
impl std::fmt::Display for ProcessInputControlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "physical input missing {field}"),
            Self::Poisoned => f.write_str("physical input owner lock poisoned"),
            Self::Input(error) => write!(f, "{error}"),
            Self::Surface(error) => write!(f, "{error}"),
        }
    }
}
fn transition_clamp(value: f32) -> f32 {
    let lower = if -1.0 - value >= 0.0 { -1.0 } else { value };
    if 1.0 - lower >= 0.0 { lower } else { 1.0 }
}
impl PhysicalAssemblyWorld {
    /// Consumes the actual prefix continuation; a failure retains preceding
    /// native writes, including those preceding Surface.Update at2268.
    #[expect(
        clippy::too_many_lines,
        reason = "consecutive native stores surround the midpoint surface call"
    )]
    pub(crate) fn try_continue_player_update1_controls(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        token: Update1AfterTeleportCheck,
    ) -> Result<Update1BeforePriorCopy, ProcessInputControlsError> {
        use ProcessInputControlsError as E;
        for handle in [board, skeleton] {
            let actual = self
                .physical_body_owner_for_contacts(handle)
                .map_err(|_| E::Missing("registered physical pair"))?;
            if !std::sync::Arc::ptr_eq(actual, &token.owner) {
                return Err(E::Missing("continuation belongs to another physical pair"));
            }
        }
        let owner = &token.owner;
        let mut processed = token.processed.lock().map_err(|_| E::Poisoned)?;
        let mut prefix_owner = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix_owner.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        if !processed.write_control_flag8c8(0x1000, prefix.non_collision_flags & 0x2000_0000 != 0) {
            return Err(E::Missing("processed8c8"));
        }
        let query = owner
            .actor_query
            .lock()
            .map_err(|_| E::Poisoned)?
            .as_ref()
            .cloned()
            .ok_or(E::Missing("IActorQuery"))?;
        if let Some(value) = query.try_input_value(75).map_err(E::Input)? {
            processed.input_controls_mut().transition = Some(transition_clamp(value));
        }
        if token.prior_category == PHYSICS_CATEGORY_GROUND {
            processed.write_control_flag8c8(0x8000, true);
            prefix.ground_state_count = prefix.ground_state_count.wrapping_add(1);
            #[expect(
                clippy::assign_op_pattern,
                reason = "native fadds2228 orders dt before the retained ground time"
            )]
            {
                prefix.time_on_ground = processed
                    .physics_state_fields()
                    .time_delta
                    .ok_or(E::Missing("processed dt"))?
                    + prefix.time_on_ground;
            }
            prefix.non_collision_flags = (prefix.non_collision_flags & 0x7fff_ffff)
                | if prefix.ground_state_count > 6 {
                    0x8000_0000
                } else {
                    0
                };
        } else {
            prefix.time_on_ground = 0.0;
            prefix.ground_state_count = 0;
        }
        processed.input_controls_mut().time_on_ground = Some(prefix.time_on_ground);
        // Do not hold history across the real surface call or preflight its
        // later reads before the published ground timer.
        drop(prefix_owner);
        self.try_dispatch_pair_surface(board, skeleton, &mut processed)
            .map_err(E::Surface)?;
        let input = token.input.lock().map_err(|_| E::Poisoned)?;
        let flags = processed
            .physics_state_fields_mut()
            .flags_8cc
            .as_mut()
            .ok_or(E::Missing("processed8cc"))?;
        *flags = (*flags & !8) | if input.flags.mirrored { 8 } else { 0 };
        let mut prefix_owner = owner.prefix().lock().map_err(|_| E::Poisoned)?;
        let prefix = prefix_owner.as_mut().ok_or(E::Missing("HiLOD prefix"))?;
        if input.flags.ai_input_active {
            processed.write_control_flag8c8(0x4000_0000, true);
            processed.copy_control_ai(&input.ai);
            if input.ai.flags & 0x0400_0000 != 0 {
                prefix.ai.assign_source(&input.ai);
            } else {
                processed.restore_control_ai_trajectory(&prefix.ai);
            }
        }
        processed.write_control_flag8c8(0x2000_0000, input.flags.under_ai_control);
        processed.write_control_flag8c4(0x2000, prefix.non_collision_flags & 0x8000_0000 != 0);
        let (flags, known) = processed.known_flags_8c4();
        if known & 8 == 0 {
            return Err(E::Missing("processed8c4 bit8"));
        }
        processed.write_control_flag8c4(0x0004_0000, flags & 8 != 0);
        prefix.frames_since_teleport = if prefix.non_collision_flags & 0x2000_0000 != 0 {
            0
        } else {
            prefix.frames_since_teleport.wrapping_add(1)
        };
        processed.input_controls_mut().frames_since_teleport = Some(prefix.frames_since_teleport);
        prefix.non_collision_flags &= !0x2000_0000;
        let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
        processed.input_controls_mut().state_count =
            Some(state.state_count.ok_or(E::Missing("HiLOD state count"))?);
        processed.physics_state_fields_mut().state_timer =
            Some(state.state_timer.ok_or(E::Missing("HiLOD state timer"))?);
        processed.input_controls_mut().update_count = Some(prefix.update_count);
        processed.write_control_flag8c8(0x4000, input.flags.weight_forwards);
        processed.write_control_flag8c4(2, input.flags.prevent_manual_respawn);
        processed.write_control_flag8c4(1, input.flags.ignore_respawn_reset_button);
        drop(state);
        drop(prefix_owner);
        drop(input);
        drop(processed);
        Ok(Update1BeforePriorCopy {
            continuation: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_construction::anim_out_phys_in::AnimationAiInput;
    #[test]
    fn native_transition_fsel_preserves_zero_and_selects_one_for_nan() {
        for (input, expected) in [
            (-0.0, -0.0),
            (0.0, 0.0),
            (-2.0, -1.0),
            (2.0, 1.0),
            (f32::NAN, 1.0),
        ] {
            assert_eq!(transition_clamp(input).to_bits(), f32::to_bits(expected));
        }
    }
    #[test]
    fn ai_assignment_preserves_unassigned_low_flags() {
        let mut retained = AnimationAiInput {
            flags: 0x01ab_cdef,
            ..AnimationAiInput::default()
        };
        let source = AnimationAiInput {
            flags: 0x8600_0000,
            trajectory_time: [3.0; 4],
            ..AnimationAiInput::default()
        };
        retained.assign_source(&source);
        assert_eq!(retained.flags, 0x87ab_cdef);
        assert_eq!(
            retained.trajectory_time.map(f32::to_bits),
            [3.0_f32.to_bits(); 4]
        );
    }
}
