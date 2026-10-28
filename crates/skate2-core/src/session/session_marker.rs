//! `Sk8::SessionMarker` (constructor 828adab8, Update 828acc90, SetSessionMarker
//! 828acf30, GetSessionMarker 828adc38) and its Verify rules. Game-layer answers
//! come from [`SessionMarkerRules`]. `IsSessionMarkerValid` and
//! `ClearSessionMarker` have only UI and message callers, none of them ported.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GROUND, PHYSICS_CATEGORY_OFFBOARD, PHYSICS_STATE_OFFBOARD_PUSHING,
    PHYSICS_STATE_SKITCHING,
};
use crate::skeleton_root_transforms::SkeletonRootMatrix;
use skate2_formats::attrib::string_hash64;

#[path = "session_marker_player_ui.rs"]
pub(crate) mod player_ui;
#[path = "session_marker_teleport.rs"]
pub(crate) mod teleport;

/// `ePhysicsSurfaceType_Stair`.
const SURFACE_STAIR: u32 = 8;
/// `CommonBackEnd::GameMode`: SinglePlayer, ProfileFreeSkate, DebugFE.
const GM_SINGLE_PLAYER: u32 = 1;
const GM_PROFILE_FREE_SKATE: u32 = 3;
const GM_DEBUG_FE: u32 = 7;
/// `WorldPainter::eSecurityGuardAreas`.
pub(crate) const SECURITY_GUARD_AREAS: usize = 6;

/// `Skater::FootForward`: RIGHT is 0, LEFT is 1.
pub(crate) type FootForward = u32;

/// The local Actor's const PhysOut fields read by the skating rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LocalSkaterOutput {
    /// `PhysOut_State+0x0c`.
    pub(crate) category: i32,
    /// `PhysOut_State+0x10`.
    pub(crate) state: i32,
    /// `PhysOut_State+0x32`.
    pub(crate) wiping_out: bool,
    /// `PhysOut_State+0x3e`.
    pub(crate) can_leave_wipeout: bool,
    /// `PhysOut_Collision+0x00`.
    pub(crate) wheels_in_contact: i32,
    /// `PhysOut_Collision+0x10`.
    pub(crate) surface: u32,
}

/// Game-layer owners queried by the session marker, in native call order.
pub(crate) trait SessionMarkerRules {
    type Error;
    /// `FEBackEnd::Data.mFEState == eFE_STATE_IN_GAME`.
    fn fe_in_game(&mut self) -> Result<bool, Self::Error>;
    /// `Simulation::IsNetworkGame`.
    fn network_game(&mut self) -> Result<bool, Self::Error>;
    /// `IsInCreateASpot`828ab1a8.
    fn in_create_a_spot(&mut self) -> Result<bool, Self::Error>;
    /// `kDisableSessionMarkerPreConditions`.
    fn disable_preconditions(&mut self) -> Result<bool, Self::Error>;
    /// `IsSecurityServiceRunning`828ab278.
    fn security_service_running(&mut self) -> Result<bool, Self::Error>;
    /// `CommonBackEnd::sGameInfo.gameMode`.
    fn game_mode(&mut self) -> Result<u32, Self::Error>;
    /// `CommonBackEnd::sGameInfo.securitySetting`.
    fn security_setting(&mut self) -> Result<bool, Self::Error>;
    /// `GetLocal`828ab060 is non-null.
    fn local_present(&mut self) -> Result<bool, Self::Error>;
    /// The present local Actor's const PhysOut.
    fn local_output(&mut self) -> Result<LocalSkaterOutput, Self::Error>;
    /// Network `VerifyLocalPlayerOkToSet`: `CalcRespawnTransform` then
    /// `IsPostionOk` on the local Actor's breadcrumbs.
    fn local_respawn_position_ok(&mut self) -> Result<bool, Self::Error>;
    /// `GetMainActivity`828ab0f0's challenge-data session marker byte, or
    /// `None` without a main activity.
    fn main_activity_allows(&mut self, use_marker: bool) -> Result<Option<bool>, Self::Error>;
    /// `cTeleportManager` set (or use) session marker overrides are empty.
    fn no_overrides(&mut self, use_marker: bool) -> Result<bool, Self::Error>;
    /// `ChaseMan::ChaseIsOn`826f3b48.
    fn chase_is_on(&mut self) -> Result<bool, Self::Error>;
    /// `ChallengeHOMManager+0x472` `mAllowUseSessionMarker`.
    fn hom_allows_use(&mut self) -> Result<bool, Self::Error>;
    /// `cSignPostMessageMan::GetMessageSetting`825cd6a8 with default true.
    fn message_setting(&mut self, name: &'static str) -> Result<bool, Self::Error>;
    /// `cChallengeSignUpManager::IsSigningUp`82da1ab0.
    fn signing_up(&mut self) -> Result<bool, Self::Error>;
    /// `LayerMan::GetAttribData(eSecurityGuardAreas, (x, z))`.
    fn security_area_key(&mut self, position: [f32; 2]) -> Result<u64, Self::Error>;
    /// `BreadCrumbManager::IsPostionOk`82ce1fc0: `_TestForCollisionConstraints`
    /// with the capsule test.
    fn position_ok(&mut self, marker: &SkeletonRootMatrix) -> Result<bool, Self::Error>;
}

/// The 0x80-byte allocation. The DMO locator table (+0x04) is only filled by
/// `cMsgAddDMOToSessionMarker`, whose senders are unported, so it stays empty
/// and its Clear/Reset calls are no-ops. `mInNoSkateZone` (+0x7b) is only
/// written by WorldPainter key-change messages (unported producer).
#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent native SessionMarker bytes"
)]
pub(crate) struct SessionMarker {
    pub(crate) props: props::MarkerProps,
    transform: SkeletonRootMatrix,
    foot_forward: FootForward,
    valid: bool,
    can_set: bool,
    can_use: bool,
    in_no_skate_zone: bool,
    on_board: bool,
}

/// `KeyIsNoSkateZoneKey`82cea848: HighAlert first, then NoSkateZone.
fn no_skate_zone_key(key: u64) -> bool {
    key == string_hash64("HighAlert_Area") || key == string_hash64("NoSkateZone_Area")
}

/// `GameBackEnd::IsSinglePlayerMode`8255b7a0.
fn single_player(game_mode: u32) -> bool {
    game_mode == GM_SINGLE_PLAYER || game_mode == GM_DEBUG_FE
}

/// `VerifySkatingSessionMarkerRules`828aa568.
fn skating_rules(local: LocalSkaterOutput) -> bool {
    if local.category != PHYSICS_CATEGORY_GROUND {
        return local.category == PHYSICS_CATEGORY_OFFBOARD
            && local.state != PHYSICS_STATE_OFFBOARD_PUSHING;
    }
    local.wheels_in_contact >= 2 && local.state != PHYSICS_STATE_SKITCHING
}

/// `VerifySkatingSessionMarkerRulesToUse`828aa608.
fn skating_rules_to_use(local: LocalSkaterOutput) -> bool {
    if local.wiping_out && !local.can_leave_wipeout {
        return false;
    }
    local.state != PHYSICS_STATE_SKITCHING && local.state != PHYSICS_STATE_OFFBOARD_PUSHING
}

/// `VerifyProgressionOkToSet/Use`828ab840/828ab940.
fn verify_progression<R: SessionMarkerRules>(
    rules: &mut R,
    name: &'static str,
) -> Result<bool, R::Error> {
    if !single_player(rules.game_mode()?) {
        return Ok(true);
    }
    rules.message_setting(name)
}

/// `VerifySignUpOkToUse`828aba40, used for both set and use.
fn verify_sign_up<R: SessionMarkerRules>(rules: &mut R) -> Result<bool, R::Error> {
    if !single_player(rules.game_mode()?) {
        return Ok(true);
    }
    Ok(!rules.signing_up()?)
}

/// `VerifyChallengeOkToSet`828ab5c0 / `VerifyChallengeOkToUse`828ab6d0.
fn verify_challenge<R: SessionMarkerRules>(
    rules: &mut R,
    use_marker: bool,
) -> Result<bool, R::Error> {
    if use_marker && rules.chase_is_on()? && !rules.security_service_running()? {
        return Ok(false);
    }
    if rules.main_activity_allows(use_marker)? == Some(false) {
        return Ok(false);
    }
    if !rules.no_overrides(use_marker)? {
        return Ok(false);
    }
    if use_marker {
        return rules.hom_allows_use();
    }
    Ok(true)
}

impl SessionMarker {
    /// ClearSessionMarker828aa550 invalidates the marker; the selected dynamic
    /// locator collection is empty, so its native clear has no additional work.
    pub(crate) fn clear_for_world_travel(&mut self) { self.valid = false; self.props.clear(); }
    /// Constructor 828adab8: identity axes over a zero translation row,
    /// stance RIGHT, on-board set and every other byte clear.
    pub(crate) fn construct() -> Self {
        Self {
            props: Default::default(),
            transform: SkeletonRootMatrix::known([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            foot_forward: 0,
            valid: false,
            can_set: false,
            can_use: false,
            in_no_skate_zone: false,
            on_board: true,
        }
    }

    /// `IsSetAllowed`828adc20.
    pub(crate) fn is_set_allowed(&self) -> bool {
        self.can_set
    }
    /// `IsUseAllowed`828adc28.
    pub(crate) fn is_use_allowed(&self) -> bool {
        self.can_use
    }

    /// `GetSessionMarker`828adc38: transform, stance and on-board when valid.
    pub(crate) fn get(&self) -> Option<(SkeletonRootMatrix, FootForward, bool)> {
        self.valid
            .then_some((self.transform, self.foot_forward, self.on_board))
    }

    /// `Update`828acc90, called by `GameModule::DoGameUpdate` before the UI.
    pub(crate) fn try_update<R: SessionMarkerRules>(
        &mut self,
        rules: &mut R,
    ) -> Result<(), R::Error> {
        if !rules.fe_in_game()? {
            self.can_use = false;
            self.can_set = false;
            return Ok(());
        }
        let mut a_spot = false;
        self.can_set = if rules.network_game()? {
            true
        } else {
            a_spot = rules.in_create_a_spot()?;
            a_spot
                || rules.disable_preconditions()?
                || !self.in_no_skate_zone
                || rules.security_service_running()?
                || !(rules.game_mode()? != GM_PROFILE_FREE_SKATE || rules.security_setting()?)
        };
        if self.can_set && !Self::verify_local_player_ok_to_set(rules)? {
            self.can_set = false;
        }
        if self.can_set && !verify_challenge(rules, false)? {
            self.can_set = false;
        }
        if self.can_set && !verify_progression(rules, "Set Session Marker Visible")? && !a_spot {
            self.can_set = false;
        }
        if self.can_set && !verify_sign_up(rules)? {
            self.can_set = false;
        }
        self.can_use = self.valid;
        if self.can_use && !Self::verify_local_player_ok_to_use(rules)? {
            self.can_use = false;
        }
        if self.can_use && !verify_challenge(rules, true)? {
            self.can_use = false;
        }
        if self.can_use && !verify_progression(rules, "Use Session Marker Visible")? && !a_spot {
            self.can_use = false;
        }
        if self.can_use && !verify_sign_up(rules)? {
            self.can_use = false;
        }
        // acee4..af20: standing on stairs forbids setting.
        if self.can_set && rules.local_present()? && rules.local_output()?.surface == SURFACE_STAIR
        {
            self.can_set = false;
        }
        Ok(())
    }

    /// `VerifyLocalPlayerOkToSet`828ab4a0: no local Actor is allowed.
    fn verify_local_player_ok_to_set<R: SessionMarkerRules>(
        rules: &mut R,
    ) -> Result<bool, R::Error> {
        if !rules.local_present()? {
            return Ok(true);
        }
        if !skating_rules(rules.local_output()?) {
            return Ok(false);
        }
        if !rules.network_game()? {
            return Ok(true);
        }
        rules.local_respawn_position_ok()
    }

    /// `VerifyLocalPlayerOkToUse`828ab568: no local Actor forbids use.
    fn verify_local_player_ok_to_use<R: SessionMarkerRules>(
        rules: &mut R,
    ) -> Result<bool, R::Error> {
        if !rules.local_present()? {
            return Ok(false);
        }
        Ok(skating_rules_to_use(rules.local_output()?))
    }

    /// `VerifyPosition`828ab328: the security layer at the marker's (x, z),
    /// then the local Actor's breadcrumb collision constraints.
    fn verify_position<R: SessionMarkerRules>(
        rules: &mut R,
        marker: &SkeletonRootMatrix,
    ) -> Result<bool, R::Error> {
        let translation = marker.columns()[3].xyz;
        let key = rules.security_area_key([translation[0], translation[2]])?;
        let restricted = rules.game_mode()? != GM_PROFILE_FREE_SKATE || rules.security_setting()?;
        if restricted
            && no_skate_zone_key(key)
            && !rules.network_game()?
            && !rules.in_create_a_spot()?
        {
            return Ok(false);
        }
        if !rules.local_present()? {
            return Ok(true);
        }
        rules.position_ok(marker)
    }

    /// `SetSessionMarker`828acf30. Unforced sets need IsSetAllowed plus a
    /// verified position, or IsSetAllowed while the security service runs.
    pub(crate) fn try_set<R: SessionMarkerRules>(
        &mut self,
        rules: &mut R,
        marker: &SkeletonRootMatrix,
        foot_forward: FootForward,
        force: bool,
        on_board: bool,
    ) -> Result<bool, R::Error> {
        if !force
            && (!self.is_set_allowed() || !Self::verify_position(rules, marker)?)
            && (!self.is_set_allowed() || !rules.security_service_running()?)
        {
            return Ok(false);
        }
        self.set_forced(marker, foot_forward, on_board);
        Ok(true)
    }

    /// `SetSessionMarker`828acf30 with `force`: the rules are not consulted.
    pub(crate) fn set_forced(&mut self, marker: &SkeletonRootMatrix, foot_forward: FootForward, on_board: bool) {
        self.transform = *marker;
        self.foot_forward = foot_forward;
        self.on_board = on_board;
        self.valid = true;
    }
}

#[cfg(test)]
#[path = "session_marker_tests.rs"]
mod tests;

#[path = "session_marker_props.rs"]
pub(crate) mod props;
