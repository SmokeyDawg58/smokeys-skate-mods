//! `PlayerUI::UpdateSessionMarker`828af3f0: LB+DPad Down sets the marker and
//! a held LB+DPad Up requests `Flow::StateMessage_TeleportToPosition`. The
//! crackle messages feed the same VisualDirector record as out-of-bounds.
#![allow(clippy::neg_cmp_op_on_partial_ord)]
use super::{FootForward, SessionMarker, SessionMarkerRules};
use crate::skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector};

/// `kUpdateTime`82163190 (1/60): the held clock advances per update, not by dt.
const UPDATE_TIME: f32 = f32::from_bits(0x3c88_8889);

/// `Input::eInputStates` read through `cInputMap::GetState`8298c8c8.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionMarkerInput {
    /// `kIS_SessionMarkerSet`52: `LBumper.held && DPadD.pressed`.
    Set,
    /// `kIS_SessionMarkerUse`53: `LBumper.held && DPadU.held`.
    Use,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionMarkerCvar {
    MinDistanceForTeleport,
    MinTimeHeld,
    MaxTimeHeld,
    MinDistanceForMinTime,
    MaxDistanceForMaxTime,
}

/// `Flow::StateMessage_TeleportToPosition` (ctor 825b11a0) as sent by the stage.
/// Its `largeLoadRadius` (false here) only feeds `TeleportRequiresLoadScreen`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TeleportToPosition {
    pub(crate) transform: SkeletonRootMatrix,
    pub(crate) foot_forward: FootForward,
    pub(crate) on_board: bool,
    pub(crate) session_marker: bool,
}

/// PlayerUI owners read by the stage, beyond the SessionMarker rules.
pub(crate) trait PlayerUiSessionMarkerSource: SessionMarkerRules {
    /// `GetLocalActor`828aa8b0(playerIndex): `GetGhost`, else `FindByIndex`.
    fn player_actor_present(&mut self) -> Result<bool, Self::Error>;
    /// `cInputMap::GetState` on the SIM input of this PlayerUI's controller.
    fn input_state(&mut self, input: SessionMarkerInput) -> Result<bool, Self::Error>;
    /// `Actor::IsOffboard`825ed600: PhysOut `State+0x41`.
    fn is_offboard(&mut self) -> Result<bool, Self::Error>;
    /// `Actor::GetFootForwardWithFakie`825ed560.
    fn foot_forward_with_fakie(&mut self) -> Result<FootForward, Self::Error>;
    /// `Actor::CalcRespawnTransform`825f09a8.
    fn respawn_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error>;
    /// Magnitude of `PhysOut.mSkeleton+0x1a0` minus `point` (vmsum3fp, then the
    /// refined reciprocal square root, zero for a zero square).
    fn distance_to(&mut self, point: SkeletonRootVector) -> Result<f32, Self::Error>;
    /// `cTeleportManager::TeleportRequiresLoadScreen`828a9420 (large radius false).
    fn teleport_requires_load_screen(
        &mut self,
        transform: &SkeletonRootMatrix,
    ) -> Result<bool, Self::Error>;
    /// `IsSafeToUseSessionMarker`828aa130.
    fn safe_to_use(&mut self) -> Result<bool, Self::Error>;
    /// `kDebugSessionTeleport`.
    fn debug_session_teleport(&mut self) -> Result<bool, Self::Error>;
    fn cvar(&mut self, cvar: SessionMarkerCvar) -> Result<f32, Self::Error>;
    /// `SendMsg<cMsgSetMarker>` with this PlayerUI's `mPlayerIndex`.
    fn send_set_marker(&mut self) -> Result<(), Self::Error>;
    /// The security-service dismissal popup (`OpenPopup`0xd9 + FRONTEND pause)
    /// is front-end owned; reaching it is an explicit boundary.
    fn dismissal_popup(&mut self) -> Self::Error;
}

/// The hold time ramps linearly between the two distances (828af7c4..800).
fn hold_time(
    distance: f32,
    min_time: f32,
    min_distance: f32,
    max_time: f32,
    max_distance: f32,
) -> f32 {
    let slope = (max_time - min_time) / (max_distance - min_distance);
    // af7e4 BLE and af7f0 BGE: unordered distances keep the current bound.
    if !(distance > min_distance) {
        return min_time;
    }
    if !(distance < max_distance) {
        return max_time;
    }
    // af7fc FNMSUBS then af800 FMADDS, each fused.
    distance.mul_add(slope, (-min_distance).mul_add(slope, min_time))
}

/// PlayerUI fields written by the stage. The constructor 828ae0f0 zeroes them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct PlayerUiSessionMarker {
    /// Last queued `cMsgOutOfBoundaryAmount`0xa5be2e3b. The native VDWriter
    /// receiver overwrites earlier messages in FIFO delivery order.
    out_of_boundary_amount: Option<f32>,
    /// Completed native OnTrySetSessionMarker(true) notifications for the HUD.
    pub(crate) successful_sets: u64,
    /// +0xac `mUseSessionMarker`.
    use_session_marker: bool,
    /// +0xad `mHasToldFESessionMarkerCantBeUsed`.
    told_front_end_cant_use: bool,
    /// +0xb0 `mExtraFramesOfSnow`.
    extra_frames_of_snow: i32,
    /// +0xb4 `mUpdatesUseSessionMarkerHeld`.
    updates_held: f32,
}

impl PlayerUiSessionMarker {
    /// `VDWriter::WriteData`827ffb2c/827fff9c publishes a present nonnegative
    /// record once, then clears it. No message means no effect on that frame.
    pub(crate) fn take_out_of_boundary_amount(&mut self) -> Option<f32> {
        self.out_of_boundary_amount
            .take()
            .filter(|amount| *amount >= 0.0)
    }
    /// `UpdateSessionMarker`828af3f0, reached from `PlayerUI::Update` only with
    /// a SessionMarker and focus. Returns the Flow message it sends, if any.
    pub(crate) fn try_update<S: PlayerUiSessionMarkerSource>(
        &mut self,
        marker: &mut SessionMarker,
        source: &mut S,
    ) -> Result<Option<TeleportToPosition>, S::Error> {
        // af44c: GetLocalActor(playerIndex) precedes both input states.
        let local = source.player_actor_present()?;
        if source.input_state(SessionMarkerInput::Set)? && marker.is_set_allowed() && local {
            let on_board = !source.is_offboard()?;
            let foot_forward = source.foot_forward_with_fakie()?;
            let transform = source.respawn_transform()?;
            // af52c ClearDynamicObjectLocatorsInSessionMarker has an empty
            // table; OnTrySetSessionMarker(true) is the HUD's count.
            if marker.try_set(source, &transform, foot_forward, false, on_board)? {
                marker.props.clear();
                source.send_set_marker()?;
                self.successful_sets = self.successful_sets.wrapping_add(1);
            }
        }
        let mut request = None;
        if !source.input_state(SessionMarkerInput::Use)? {
            // af968: releasing the input resets the use latch and clock.
            self.use_session_marker = false;
            self.told_front_end_cant_use = false;
            self.updates_held = 0.0;
        } else if !self.use_session_marker {
            if (source.debug_session_teleport()? || marker.is_use_allowed()) && local {
                if let Some((transform, foot_forward, on_board)) = marker.get() {
                    self.updates_held += UPDATE_TIME;
                    let minimum = source.cvar(SessionMarkerCvar::MinDistanceForTeleport)?;
                    let distance = source.distance_to(transform.columns()[3])?;
                    // af6d4 BLE: unordered distance does nothing more.
                    if distance > minimum {
                        // af6f0..af73c: network, then load screen, then security.
                        // TeleportRequiresLoadScreen needs the unported
                        // IsAreaLoaded; both calls are pure, so the port asks
                        // for the security service first and only then for it.
                        if !source.network_game()?
                            && source.security_service_running()?
                            && source.teleport_requires_load_screen(&transform)?
                        {
                            return Err(source.dismissal_popup());
                        }
                        let hold = hold_time(
                            distance,
                            source.cvar(SessionMarkerCvar::MinTimeHeld)?,
                            source.cvar(SessionMarkerCvar::MinDistanceForMinTime)?,
                            source.cvar(SessionMarkerCvar::MaxTimeHeld)?,
                            source.cvar(SessionMarkerCvar::MaxDistanceForMaxTime)?,
                        );
                        // af830 BLE: the held clock must strictly exceed the hold.
                        if source.safe_to_use()?
                            && (source.debug_session_teleport()? || self.updates_held > hold)
                        {
                            self.use_session_marker = true;
                            self.extra_frames_of_snow = 3;
                            // af854 ResetDynamicObjectLocatorsInSessionMarker
                            // walks the same empty table.
                            request = Some(TeleportToPosition {
                                transform,
                                foot_forward,
                                on_board,
                                session_marker: true,
                            });
                        }
                        // af8e0..af930: cMsgOutOfBoundaryAmount on eEQGameSim.
                        let amount = self.updates_held / hold;
                        self.out_of_boundary_amount = Some(if !(amount >= 0.0) {
                            0.0
                        } else if amount > 1.0 {
                            1.0
                        } else {
                            amount
                        });
                    }
                }
            } else if !self.told_front_end_cant_use {
                // af944: latch once; OnTryUseSessionMarker itself is UI.
                self.told_front_end_cant_use = true;
            }
        }
        // af97c..af9dc: this later message overrides the ramp that update.
        if self.extra_frames_of_snow > 0 {
            self.out_of_boundary_amount = Some(1.0);
            self.extra_frames_of_snow -= 1;
        }
        Ok(request)
    }
}

#[cfg(test)]
#[path = "session_marker_player_ui_tests.rs"]
mod tests;
