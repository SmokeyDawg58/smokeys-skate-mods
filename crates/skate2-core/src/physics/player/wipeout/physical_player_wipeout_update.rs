//! Actual `WipeoutGround::Update` 0x82d9aff8 through its actual prediction submission.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    dynamic_assembly_session::{
        skater_pool::{
            SimulationSkaterAssemblyHandle, SimulationSkeletonJumpPairError,
            board_construction::SimulationConstructedSteeringError,
            wipeout::{WipeoutSkeletonError, WipeoutSkeletonOperation},
        },
        skeleton_normal::{
            drive_owner::hook_soft::HipHookSoftKind,
            general_owner::wipeout::WipeoutSkeletonUpdateError,
        },
    },
    input_cvar_runtime::{
        InputCvarError,
        ground_skateboard::{GroundSkateboardCvarError, GroundSkateboardCvars},
        wipeout_control::WipeoutDragFloat,
    },
    physical_player_assembly::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner},
    physical_player_physics_state::CurrentPhysicalState,
    reckoning_numeric::{ReckoningHostNumericError, ReckoningNumeric},
    simulation_assembly_backend::constraint_session::physical_grounded::wipeout_control::WipeoutControlError,
    simulation_assembly_backend::constraint_session::{
        ground_control_complete::SteeringInput, ground_pre_solve::HostGroundPreparationNumeric,
        physical_skeleton_controller::hip_hook_soft::PhysicalHipHookSoftError,
    },
    skeleton_drives::ragdoll_update::RagdollDriveWeights,
    static_ground_pool::StaticGroundPool,
};
use std::{convert::Infallible, sync::Arc};
type NumericError = ReckoningHostNumericError;
type ServicesError = super::GroundedWorldServiceError<GroundSkateboardCvarError>;
#[derive(Debug)]
pub(crate) enum WipeoutUpdateError {
    Missing(&'static str),
    Tail(tail::WipeoutTailError),
    Prediction(crate::simulation_assembly_backend::constraint_session::prediction::runtime::PredictionRuntimeError<NumericError>),
    Poisoned,
    ForeignState,
    Pair(SimulationSkeletonJumpPairError),
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    Overlay(crate::debug_overlay::DebugOverlayError),
    Control(WipeoutControlError<NumericError>),
    Drag(WipeoutSkeletonError<NumericError>),
    Hook(PhysicalHipHookSoftError<NumericError>),
    Steering(SimulationConstructedSteeringError<&'static str>),
    Skeleton(WipeoutSkeletonUpdateError<NumericError, Infallible, ServicesError>),
}
impl std::fmt::Display for WipeoutUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prediction(e) => write!(f, "wipeout prediction: {e}"),
            Self::Tail(e) => write!(f, "wipeout update tail: {e}"),
            Self::Missing(e) => write!(f, "wipeout update missing: {e}"),
            Self::Poisoned => f.write_str("wipeout update poisoned"),
            Self::ForeignState => f.write_str("wipeout update foreign state"),
            Self::Pair(e) => write!(f, "wipeout update pair: {e}"),
            Self::Cvar(e) => write!(f, "wipeout update cvar: {e:?}"),
            Self::Attribute(e) => write!(f, "wipeout update attribute: {e}"),
            Self::Overlay(e) => write!(f, "wipeout update overlay: {e:?}"),
            Self::Control(e) => write!(f, "wipeout update control: {e}"),
            Self::Drag(e) => write!(f, "wipeout update drag: {e}"),
            Self::Hook(e) => write!(f, "wipeout update hook: {e}"),
            Self::Steering(e) => write!(f, "wipeout update steering: {e:?}"),
            Self::Skeleton(e) => write!(f, "wipeout update skeleton: {e}"),
        }
    }
}
/// Issued only after actual selected Wipeout Update and prediction submission.
pub(crate) struct WipeoutUpdateComplete {
    owner: Arc<PhysicalPlayerAssemblyOwner>,
}
impl WipeoutUpdateComplete {
    pub(crate) fn owner(&self) -> &Arc<PhysicalPlayerAssemblyOwner> {
        &self.owner
    }
}
fn clamp_source(value: f32, maximum: f32) -> f32 {
    let lower = if -value >= 0.0 { 0.0 } else { value };
    if maximum - lower >= 0.0 {
        lower
    } else {
        maximum
    }
}
fn fade(duration: f32, time: f32) -> f32 {
    let divisor = if duration == 0.0 {
        f32::from_bits(0x3c23_d70a)
    } else {
        duration
    };
    clamp_source((duration - time) / divisor, 1.0)
}
impl PhysicalAssemblyWorld {
    #[expect(
        clippy::too_many_lines,
        reason = "ordered native Update prefix preserves partial publications on failure"
    )]
    #[expect(
        clippy::assign_op_pattern,
        reason = "retain native dt-first floating-point operand order"
    )]
    pub(crate) fn try_update_pair_wipeout(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        ground: &StaticGroundPool,
        numeric: &mut HostGroundPreparationNumeric,
    ) -> Result<WipeoutUpdateComplete, WipeoutUpdateError> {
        use WipeoutUpdateError as E;
        let (owner, hit) = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_with_jump_pair(&self.backend, board, skeleton, |normal, owner| {
                (Arc::clone(owner), normal.state().collision.hit_world != 0)
            })
            .map_err(E::Pair)?;
        let (slow, over, settled) = {
            let state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            if state.current != CurrentPhysicalState::WipeoutGround {
                return Err(E::ForeignState);
            }
            let w = state
                .wipeout_ground
                .as_ref()
                .ok_or(E::Missing("WipeoutGround"))?;
            if !w.bindings.board.same_as(board)
                || !w.bindings.skeleton.same_as(skeleton)
                || !w
                    .bindings
                    .owner
                    .upgrade()
                    .is_some_and(|v| Arc::ptr_eq(&v, &owner))
            {
                return Err(E::ForeignState);
            }
            (w.fields.slow, w.fields.over, w.fields.time_settled)
        };
        let refs = &owner.inputs().cvars;
        let drag = if slow && hit {
            let start = refs
                .try_capture_wipeout_drag_float(WipeoutDragFloat::Start)
                .map_err(E::Cvar)?
                .read()
                .map_err(E::Cvar)?;
            if over {
                let maximum = refs
                    .try_capture_wipeout_drag_float(WipeoutDragFloat::Maximum)
                    .map_err(E::Cvar)?;
                let duration = refs
                    .try_capture_wipeout_drag_float(WipeoutDragFloat::FadeTime)
                    .map_err(E::Cvar)?;
                let maximum = maximum.read().map_err(E::Cvar)?;
                let difference = maximum - start;
                let duration = duration.read().map_err(E::Cvar)?;
                clamp_source((difference / duration).mul_add(settled, start), maximum)
            } else {
                start
            }
        } else {
            0.0
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_wipeout_skeleton_operation(
                &mut self.backend,
                board,
                skeleton,
                WipeoutSkeletonOperation::LinearDrag(drag),
                &mut numeric.reckoning,
            )
            .map_err(E::Drag)?;
        if slow && hit && refs.try_wipeout_debug_display().map_err(E::Cvar)? {
            self.debug_overlay.require().map_err(E::Overlay)?;
        }
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        let dt = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("DT93c"))?;
        let end = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let w = state
                .wipeout_ground
                .as_mut()
                .ok_or(E::Missing("WipeoutGround"))?;
            w.fields.time_in_state = dt + w.fields.time_in_state;
            let delta = w
                .try_update_scalar(0x4f75_7127_c9a9_8c27)
                .map_err(E::Attribute)?;
            let delta = if hit { delta } else { -delta };
            let value = clamp_source(delta + w.fields.end_drive_weight, 1.0);
            w.fields.end_drive_weight = value;
            value
        };
        let controlled = self
            .try_update_pair_wipeout_control(
                board,
                skeleton,
                hit,
                &self.debug_overlay.clone(),
                &mut numeric.reckoning,
            )
            .map_err(E::Control)?;
        let (hook_soft, weights) = {
            let mut state = owner.physics_state().lock().map_err(|_| E::Poisoned)?;
            let w = state
                .wipeout_ground
                .as_mut()
                .ok_or(E::Missing("WipeoutGround"))?;
            let hook_duration = w
                .try_update_scalar(0xfbc4_ff75_229e_9f75)
                .map_err(E::Attribute)?;
            let hook_soft = fade(hook_duration, w.fields.time_in_state);
            let start_duration = w
                .try_update_scalar(0x9021_d945_ee80_5730)
                .map_err(E::Attribute)?;
            let start_fade = fade(start_duration, w.fields.time_in_state);
            let start_fade = start_fade * start_fade;
            let delta = w
                .try_update_scalar(0x191a_4c6e_b5ff_1a23)
                .map_err(E::Attribute)?;
            let control_active = controlled
                || processed
                    .lock()
                    .map_err(|_| E::Poisoned)?
                    .known_flags_8c8()
                    .ok_or(E::Missing("Processed8c8"))?
                    & 0x0010_0000
                    != 0;
            let delta = if control_active { delta } else { -delta };
            let start = (1.0 - end) * start_fade;
            w.fields.controlled_drive_weight =
                clamp_source(w.fields.controlled_drive_weight + delta, 1.0);
            let controlled = ((1.0 - end) - start) * w.fields.controlled_drive_weight;
            (
                hook_soft,
                RagdollDriveWeights {
                    start,
                    end,
                    controlled,
                },
            )
        };
        self.try_update_constructed_board_steering(board, 0.0, &mut SteeringInput(&processed))
            .map_err(E::Steering)?;
        self.try_pair_hip_hook_soft(
            board,
            skeleton,
            HipHookSoftKind::Angular,
            hook_soft,
            &mut |v| numeric.reckoning.reciprocal_sqrt_estimate(v),
        )
        .map_err(E::Hook)?;
        let mut cvars = GroundSkateboardCvars::new(refs, self.debug_overlay.clone());
        let mut services = super::services::GroundedWorldServices {
            cvars: &mut cvars,
            queries: self.world_line_queries.as_mut(),
            dmos: self.dmos.as_ref(),
            ground,
        };
        self.skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_update_paired_skeleton_wipeout(
                &mut self.backend,
                &mut self.hooks,
                board,
                skeleton,
                &owner,
                &processed,
                weights,
                refs,
                &mut numeric.reckoning,
                &mut numeric.sin_cos,
                &mut services,
            )
            .map_err(E::Skeleton)?;
        self.try_finish_pair_wipeout_recovery(board, skeleton)
            .map_err(E::Tail)?;
        self.try_finish_wipeout_prediction(&owner, ground, &mut numeric.reckoning)
            .map_err(E::Prediction)?;
        Ok(WipeoutUpdateComplete { owner })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_fsel_and_zero_duration_fades_preserve_unordered_and_signed_zero() {
        assert_eq!(clamp_source(f32::NAN, 1.0).to_bits(), 1.0_f32.to_bits());
        assert_eq!(clamp_source(-0.0, 1.0).to_bits(), 0);
        assert_eq!(fade(0.0, 0.0).to_bits(), 0);
        assert_eq!(fade(2.0, 1.0).to_bits(), 0.5_f32.to_bits());
    }
}

#[path = "physical_player_wipeout_tail.rs"]
pub(crate) mod tail;

#[path = "physical_player_wipeout_prediction.rs"]
pub(crate) mod prediction;

#[path = "physical_player_wipeout_post_solve.rs"]
pub(crate) mod post_solve;
