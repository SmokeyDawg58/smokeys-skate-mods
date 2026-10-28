//! Native Update4 decision/dt reads and post-wipeout continuation through IK.
use crate::dynamic_assembly_session::skeleton_normal::post_wipeout_foot_live::ik_update::IkPostUpdateLiveDebug;
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    dynamic_assembly_session::{
        skater_pool::{SimulationSkaterAssemblyHandle, post_wipeout_live::PostWipeoutPoolError},
        skeleton_normal::{
            post_wipeout_foot_live::FootIkLiveDebug, post_wipeout_prefix::PostWipeoutBeforeIk,
        },
    },
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::{
        PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner,
        update5_decisions::{self, DecisionRuntimeError},
    },
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::ReckoningSinCosNumeric,
    skeleton_skateboard_wobble::post_wipeout_update::{WobbleAdjustment, WobbleUpdateDebug},
};
use std::sync::Arc;
#[derive(Debug)]
pub(crate) enum WobbleDebugError {
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    DrawingPending,
}
impl std::fmt::Display for WobbleDebugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
            Self::DrawingPending => f.write_str("enabled wobble drawing pending"),
        }
    }
}

struct Debug<'a> {
    cvars: &'a InputCvarReferences,
    overlay: &'a DebugOverlaySlot,
}
impl FootIkLiveDebug for Debug<'_> {
    type Error = WobbleDebugError;
    fn draw_post_feet(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_draw_post_feet_ik()
            .map_err(WobbleDebugError::Cvar)
    }
    fn draw_post_physics(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_draw_post_physics_ik()
            .map_err(WobbleDebugError::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(WobbleDebugError::Overlay)
    }
}
impl WobbleUpdateDebug for Debug<'_> {
    type Error = WobbleDebugError;
    fn debug_draw_wobble(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_debug_draw_skeleton_wobble()
            .map_err(WobbleDebugError::Cvar)
    }
    fn draw_wobble(&mut self, _: &WobbleAdjustment) -> Result<(), Self::Error> {
        self.overlay.require().map_err(WobbleDebugError::Overlay)?;
        Err(WobbleDebugError::DrawingPending)
    }
}
impl IkPostUpdateLiveDebug for Debug<'_> {
    fn do_new_post_physics(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_do_new_post_physics_ik()
            .map_err(WobbleDebugError::Cvar)
    }
    fn draw_new_post_physics(&mut self) -> Result<bool, Self::Error> {
        self.cvars
            .try_draw_new_post_physics_ik()
            .map_err(WobbleDebugError::Cvar)
    }
}
#[derive(Debug)]
pub(crate) enum PostWipeoutLiveError<N, S> {
    Missing(&'static str),
    Poisoned,
    Decision(DecisionRuntimeError<N>),
    Pool(PostWipeoutPoolError<N, WobbleDebugError, S>),
    Pose(crate::dynamic_assembly_session::skater_pool::pose_publication::PosePoolError<N, S>),
}
impl<N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display for PostWipeoutLiveError<N, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(v) => write!(f, "post-wipeout missing {v}"),
            Self::Poisoned => f.write_str("post-wipeout lock poisoned"),
            Self::Decision(e) => e.fmt(f),
            Self::Pool(e) => e.fmt(f),
            Self::Pose(e) => e.fmt(f),
        }
    }
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_begin_post_wipeout<N: ReckoningNumeric, S: ReckoningSinCosNumeric>(
        &mut self,
        board: &SimulationSkaterAssemblyHandle,
        skeleton: &SimulationSkaterAssemblyHandle,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        numeric: &mut N,
        rounding: &mut S,
    ) -> Result<PostWipeoutBeforeIk, PostWipeoutLiveError<N::Error, S::Error>> {
        use PostWipeoutLiveError as E;
        let starting =
            update5_decisions::try_is_wipeout_requested(owner, &self.debug_overlay, numeric)
                .map_err(E::Decision)?;
        let processed = Arc::clone(
            owner
                .reckoning()
                .lock()
                .map_err(|_| E::Poisoned)?
                .processed(),
        );
        // Mandatory fresh93c read follows the decision; this callee does not use dt.
        let _dt = processed
            .lock()
            .map_err(|_| E::Poisoned)?
            .physics_state_fields()
            .time_delta
            .ok_or(E::Missing("Processed93c dt"))?;
        let before_pose = self
            .skaters
            .as_mut()
            .ok_or(E::Missing("skater pool"))?
            .try_post_wipeout_through_ik(
                &mut self.backend,
                board,
                skeleton,
                owner,
                &processed,
                starting,
                numeric,
                &mut Debug {
                    cvars: &owner.inputs().cvars,
                    overlay: &self.debug_overlay,
                },
                rounding,
            )
            .map_err(E::Pool)?;
        if before_pose.update_pose {
            self.try_publish_skeleton_pose(board, skeleton, owner, numeric, rounding)
                .map_err(E::Pose)?;
        }
        Ok(before_pose.captured)
    }
}
