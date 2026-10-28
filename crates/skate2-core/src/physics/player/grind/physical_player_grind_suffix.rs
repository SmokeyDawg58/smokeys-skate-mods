//! The `GrindManager::Update` suffix after the investigator:
//! `UpdateFrictionVsTime` 0x82dd7678, `ManageCopingAssistance` 0x82ddbc48,
//! `GrindBalance` (`UpdateTargetUpVector` 0x82dd2168, `UpdateExitLean`
//! 0x82dd64d8, `UpdateForceExit` 0x82dd26c0), `GrindEngagement::Update`
//! 0x82ddf5f8, `GrindControls::UpdateControls` 0x82dd3270 and
//! `GrindJumper::Update` 0x82dcb350 / `LaunchSkateboard` 0x82dcb3a8.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_AIR, PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_GRIND_5_O,
    PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_TIPSLIDE, PHYSICS_STATE_NONSPECIFIC,
};
use super::{
    fields::{GrindBalance, GrindControls, GrindEngagement, GrindJumper},
    geometry::{LineHit, calc_tilted_upmost_normal},
    math::{
        DEG_TO_RAD, GrindNum, V4, abs1, add, clamp_unit, cross, fsel, madd, neg, neg1,
        quaternion_rotate, scale, sub,
    },
    types::{Investigation, LEDGE, THIN_RAIL, TIPSLIDE},
    view::{
        GrindAttributes, GrindCvars, GrindError, GrindGraph, GrindView, MAX_GRIND_IMPACT_SPEED,
        need,
    },
};
use crate::input_cvar_runtime::grind_states::GrindCvar as C;

/// Side effects the native writes into other owners during the suffix.
#[derive(Default, Debug)]
pub(crate) struct SuffixEffects {
    /// `ProcessedPhysIn` 8c4 0x40000 (`mIsWipingOut`) set by the engagement.
    pub(crate) wiping_out: bool,
    /// `Wipeout::mWipeoutRequest` indices, each with value 0 and one counter step.
    pub(crate) wipeout_requests: Vec<usize>,
    /// `ProcessedPhysIn` 8cc 0x08000000 (`mLowEnergyJumpAvoidGrinds`).
    pub(crate) avoid_grinds: bool,
    /// Inline `SingletonHolder<DebugOverlay>::Get` assertions reached.
    pub(crate) overlay_lookups: usize,
}

/// `UpdateFrictionVsTime` 0x82dd7678.
pub(crate) fn update_friction_vs_time(
    inv: &mut Investigation,
    time_in_grind: f32,
    attributes: &GrindAttributes,
) -> Result<(), GrindError> {
    inv.friction_time_scalar = attributes.graph(GrindGraph::FrictionVsTime, time_in_grind)?;
    Ok(())
}

/// `ManageCopingAssistance` 0x82ddbc48; returns the manager's relief timer.
pub(crate) fn manage_coping_assistance(
    inv: &mut Investigation,
    mut timer: f32,
    view: &GrindView,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<f32, GrindError> {
    if inv.possible && view.category == PHYSICS_CATEGORY_GROUND {
        let vertical = num.vertical_approach(inv.primitive_tangent, view.velocity)?;
        if vertical > cvars.f(C::kMinCopingVert)? {
            let along = abs1(num.dot(inv.primitive_tangent, view.velocity)?);
            let by_vertical = attributes.graph(
                GrindGraph::TimeOfGravityReliefVsVerticalSpeed,
                view.velocity[1],
            )?;
            let by_linear = attributes.graph(GrindGraph::GravityReliefTimeVsLinearSpeed, along)?;
            timer = by_linear * by_vertical;
        }
    }
    let remaining = timer - view.dt;
    timer = fsel(neg1(remaining), 0.0, remaining);
    inv.gravity_relief_timer = timer;
    Ok(timer)
}

impl GrindBalance {
    /// `UpdateTargetUpVector` 0x82dd2168.
    pub(crate) fn try_update_target_up(
        &mut self,
        inv: &mut Investigation,
        view: &GrindView,
        cvars: &GrindCvars<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        if !inv.possible {
            return Ok(());
        }
        let tilted = calc_tilted_upmost_normal(&inv.geometry, num)?;
        let wanted = if view.category == PHYSICS_CATEGORY_GRIND {
            if matches!(view.state, PHYSICS_STATE_GRIND_TIPSLIDE | PHYSICS_STATE_GRIND_BACKSLASH) {
                let k = cvars.f(C::kTipslideNormalSmoothness)?;
                let rest = scale(inv.geometry.upmost_normal, 1.0 - k);
                madd(self.grind_normal, k, rest)
            } else {
                let k = cvars.f(C::kNormalSmoothness)?;
                let rest = scale(tilted, 1.0 - k);
                madd(self.grind_normal, k, rest)
            }
        } else {
            view.up
        };
        let across = cross(inv.grind_dir, wanted);
        let perpendicular = cross(across, inv.grind_dir);
        let length = num.length(perpendicular)?;
        let normal = if length > f32::from_bits(0x3a83_126f) {
            scale(perpendicular, num.reciprocal(length)?)
        } else {
            inv.geometry.upmost_normal
        };
        inv.grind_normal = normal;
        self.grind_normal = normal;
        let k = cvars.f(C::kUpVectorSmoothness)?;
        let rest = scale(inv.geometry.upmost_normal, 1.0 - k);
        let target = madd(view.up, k, rest);
        inv.target_up_vector = target;
        let length = num.length(target)?;
        // Debug assertion 0x82dd24b4 (positive length); shipped build continues.
        let target = scale(target, num.reciprocal(length)?);
        inv.target_up_vector = target;
        let dot = num.dot(normal, target)?;
        let angle = num.acos(clamp_unit(dot))?;
        let limit = cvars.f(C::kMaxBalanceAngleDeg)? * DEG_TO_RAD;
        if angle > limit {
            let axis = cross(normal, target);
            let axis_length = num.length(axis)?;
            if axis_length > f32::from_bits(0x3a83_126f) {
                let axis = scale(axis, num.reciprocal(axis_length)?);
                inv.target_up_vector = num.rotate(axis, limit, normal)?;
            } else {
                inv.target_up_vector = normal;
            }
        }
        Ok(())
    }

    /// `UpdateExitLean` 0x82dd64d8.
    #[expect(clippy::too_many_lines, reason = "one native timer and lean sequence")]
    pub(crate) fn try_update_exit_lean(
        &mut self,
        inv: &mut Investigation,
        view: &GrindView,
        attributes: &GrindAttributes,
        cvars: &GrindCvars<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let in_grind = view.category == PHYSICS_CATEGORY_GRIND || view.state == PHYSICS_STATE_NONSPECIFIC || inv.possible;
        let non_grind = if in_grind {
            0
        } else {
            need(self.non_grind_frames, "GrindBalance non-grind frames")?.wrapping_add(1)
        };
        self.non_grind_frames = Some(non_grind);
        let lapsed = non_grind
            > cvars
                .0
                .try_lean_min_non_grind_frames()
                .map_err(super::view::error)?;
        let droppable = if inv.grind_type == TIPSLIDE {
            in_grind && inv.geometry.geometry_type != THIN_RAIL
        } else {
            inv.geometry.geometry_type == LEDGE
        };
        let mut started = false;
        if lapsed {
            self.exit_tilt_angle = 0.0;
            self.time_in_droppable_grind = 0.0;
            self.exit_start_time = Some(
                cvars
                    .0
                    .try_curb_exit_start_time()
                    .map_err(super::view::error)?,
            );
        } else {
            let time = self.time_in_droppable_grind;
            let active = time > 0.0 || (in_grind && droppable && inv.possible);
            if active {
                if time == 0.0 {
                    self.exit_start_time = Some(match inv.entry_type {
                        1 => cvars.f(C::kFairlySteepExitStartTime)?,
                        2 => cvars.f(C::kCopingExitStartTime)?,
                        _ => cvars
                            .0
                            .try_curb_exit_start_time()
                            .map_err(super::view::error)?,
                    });
                    started = true;
                }
                if view.substate != 2 {
                    self.time_in_droppable_grind = view.dt + time;
                }
            }
        }
        let time = self.time_in_droppable_grind;
        if time > 0.0 {
            let since = time - need(self.exit_start_time, "GrindBalance exit start time")?;
            self.exit_tilt_angle = if since > 0.0 {
                attributes.graph(GrindGraph::ExitLeanAngleVsTime, since)?
            } else {
                0.0
            };
            if inv.possible {
                if inv.geometry.geometry_type == THIN_RAIL {
                    // Debug assertion 0x82dd6708 (not started this frame).
                    let side = cross(inv.geometry.upmost_normal, inv.primitive_tangent);
                    let same = num.dot(self.high_side_direction, side)? > 0.0;
                    self.high_side_direction = if same { side } else { neg(side) };
                } else if started {
                    self.high_side_direction = inv.geometry.high_side_dir;
                } else {
                    let same = num.dot(self.high_side_direction, inv.geometry.high_side_dir)? > 0.0;
                    self.high_side_direction = if same {
                        inv.geometry.high_side_dir
                    } else {
                        neg(inv.geometry.high_side_dir)
                    };
                }
            }
            if inv.possible && self.exit_tilt_angle > 0.0 {
                let turn = cross(inv.grind_dir, self.high_side_direction);
                let axis = if turn[1] > 0.0 {
                    inv.grind_dir
                } else {
                    neg(inv.grind_dir)
                };
                let q = num.quaternion(axis, self.exit_tilt_angle * DEG_TO_RAD)?;
                inv.target_up_vector = quaternion_rotate(q, inv.target_up_vector);
                inv.grind_normal = quaternion_rotate(q, inv.grind_normal);
            }
        }
        inv.exit_tilt_angle_deg = self.exit_tilt_angle;
        Ok(())
    }

    /// `UpdateForceExit` 0x82dd26c0: tilted past roll-out, or past ride-out
    /// with nothing under the high side, requests the automatic exit.
    pub(crate) fn try_update_force_exit(
        &self,
        inv: &mut Investigation,
        cvars: &GrindCvars<'_>,
        line: &mut impl FnMut(V4, V4) -> Result<Option<LineHit>, GrindError>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let tilt = self.exit_tilt_angle;
        if tilt > cvars.f(C::kForceExitRollOutTiltDeg)? {
            inv.balance_requests_auto_exit = true;
            return Ok(());
        }
        if !(tilt > cvars.f(C::kForceExitRideOutTiltDeg)?) {
            inv.balance_requests_auto_exit = false;
            return Ok(());
        }
        let out = cvars.f(C::kForceExitTestOut)?;
        let start = sub(inv.primitive_location, scale(self.high_side_direction, out));
        let down = [0.0, neg1(cvars.f(C::kForceExitTestDown)?), 0.0, 0.0];
        let end = add(start, down);
        let Some(hit) = line(start, end)? else {
            inv.balance_requests_auto_exit = true;
            return Ok(());
        };
        let facing = num.dot(self.high_side_direction, hit.normal)?;
        inv.balance_requests_auto_exit = facing > cvars.f(C::kForceExitNormalThresh)?;
        Ok(())
    }
}

impl GrindEngagement {
    /// `Update` 0x82ddf5f8.
    pub(crate) fn try_update(
        &mut self,
        inv: &mut Investigation,
        view: &GrindView,
        attributes: &GrindAttributes,
        cvars: &GrindCvars<'_>,
        num: &mut GrindNum<'_>,
        effects: &mut SuffixEffects,
    ) -> Result<(), GrindError> {
        inv.impact_speed = 0.0;
        self.consider_grabbing_tipslide(inv, view, cvars)?;
        if inv.possible {
            let change = decide_engagement(inv, view, attributes, cvars, num, effects)?;
            if inv.possible {
                measure_impact(inv, change, view, attributes, cvars, num, effects)?;
            }
        }
        Ok(())
    }

    /// `ConsiderGrabbingTipslide` 0x82dd6be0.
    fn consider_grabbing_tipslide(
        &mut self,
        inv: &mut Investigation,
        view: &GrindView,
        cvars: &GrindCvars<'_>,
    ) -> Result<(), GrindError> {
        let grab = inv.possible
            && inv.grind_type == TIPSLIDE
            && matches!(view.state, PHYSICS_STATE_NONSPECIFIC | PHYSICS_STATE_GRIND_5_O)
            && view.deck_speed > cvars.f(C::kTipslideGrabSlow)?;
        self.tipslide_grab_timer = if grab {
            cvars.i(C::kTipslideGrabFrames)?
        } else {
            self.tipslide_grab_timer.wrapping_sub(1).max(0)
        };
        inv.grab_tipslide = self.tipslide_grab_timer > 0;
        Ok(())
    }
}

/// `DecideEngagement` 0x82dda290: the engagement velocity and its change.
fn decide_engagement(
    inv: &mut Investigation,
    view: &GrindView,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
    effects: &mut SuffixEffects,
) -> Result<V4, GrindError> {
    inv.should_engage = false;
    let ground = view.category == PHYSICS_CATEGORY_GROUND;
    if !ground && view.category != PHYSICS_CATEGORY_AIR {
        return Ok([0.0; 4]);
    }
    let tilt = abs1(num.dot(view.up, inv.grind_dir)?);
    if tilt > cvars.f(C::kOffBalanceGrindThresh)? {
        if !ground {
            inv.possible = false;
            effects.wiping_out = true;
            effects.wipeout_requests.push(13);
        }
        inv.possible = false;
        return Ok([0.0; 4]);
    }
    let side = cross(inv.grind_normal, inv.grind_dir);
    inv.should_engage = true;
    if ground {
        let drop_in = |cvars: &GrindCvars<'_>| -> Result<bool, GrindError> {
            Ok(view.balance != 0.0 && view.deck_speed < cvars.f(C::kMaxDropInSpeed)?)
        };
        let help = match inv.grind_type {
            0 => cvars.f(C::kGroundEngagement5050)?,
            1 => cvars.f(C::kGroundEngagementBoardslide)?,
            3 => cvars.f(C::kGroundEngagement5_O)?,
            4 => {
                if drop_in(cvars)? {
                    1.0
                } else {
                    cvars.f(C::kGroundEngagementBackslash)?
                }
            }
            2 => {
                if drop_in(cvars)? {
                    1.0
                } else {
                    cvars.f(C::kGroundEngagementTipslide)?
                }
            }
            _ => 0.0,
        };
        let vertical = num.vertical_approach(inv.grind_dir, view.velocity)?;
        let assist = attributes.graph(GrindGraph::VertEngagementHelpVsVelY, vertical)?;
        let along = num.dot(view.velocity, inv.grind_dir)?;
        let projected = scale(inv.grind_dir, along);
        let blend = assist.mul_add(1.0 - help, help);
        let blend = fsel(1.0 - blend, blend, 1.0);
        let keep = scale(view.velocity, 1.0 - blend);
        let engagement = madd(projected, blend, keep);
        inv.engagement_velocity = engagement;
        return Ok(sub(engagement, view.velocity));
    }
    let help = match inv.grind_type {
        0 => cvars.f(C::kAirEngagement5050)?,
        1 => cvars.f(C::kAirEngagementBoardslide)?,
        2 => cvars.f(C::kAirEngagementTipslide)?,
        3 => cvars.f(C::kAirEngagement5_O)?,
        4 => cvars.f(C::kAirEngagementBackslash)?,
        _ => 0.0,
    };
    let com = view.com_velocity;
    let sideways = num.dot(com, side)?;
    let without_side = sub(com, scale(side, sideways));
    let keep = scale(com, 1.0 - help);
    let engagement = madd(without_side, help, keep);
    inv.engagement_velocity = engagement;
    Ok(sub(engagement, com))
}

/// `MeasureImpact` 0x82dd69d0.
fn measure_impact(
    inv: &mut Investigation,
    change: V4,
    view: &GrindView,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
    effects: &mut SuffixEffects,
) -> Result<(), GrindError> {
    if !inv.should_engage {
        return Ok(());
    }
    inv.impact_speed = num.length(change)?;
    if inv.impact_speed > attributes.wipeout_scalar(MAX_GRIND_IMPACT_SPEED)? {
        inv.possible = false;
        effects.wiping_out = true;
        effects.wipeout_requests.push(9);
    }
    let along = num.dot(inv.grind_dir, view.velocity)?;
    let mut across = sub(scale(inv.grind_dir, along), view.velocity);
    across[1] = 0.0;
    let squared = num.dot(across, across)?;
    let limit = cvars.f(C::kHorizontalGrindWipeoutSpeed)?;
    if squared > limit * limit {
        let toward_high = inv.geometry.geometry_type == THIN_RAIL
            || num.dot(view.velocity, inv.geometry.high_side_dir)? > 0.0;
        if toward_high {
            inv.possible = false;
            effects.wiping_out = true;
            effects.wipeout_requests.push(14);
        }
    }
    Ok(())
}

impl GrindControls {
    /// `UpdateControls` 0x82dd3270: smoothed cubic truck controls for 5050
    /// (up/down) and 5-0 (pitch with deck limits, lateral twist).
    #[expect(clippy::too_many_lines, reason = "one native control sequence")]
    pub(crate) fn try_update_controls(
        &mut self,
        inv: &mut Investigation,
        view: &GrindView,
        cvars: &GrindCvars<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), GrindError> {
        let is_5050 = inv.grind_type == 0;
        let mut clamped = false;
        let (target_lr, target_ud) = if !inv.possible || !(is_5050 || inv.grind_type == 3) {
            (0.0, 0.0)
        } else {
            let (lr, ud) = if is_5050 {
                (0.0, view.up_down)
            } else {
                let flipped = view.bit_8c4(0x0010_0000)?;
                let on_front = inv.grind_on_front;
                let effective_front = if flipped { !on_front } else { on_front };
                let tilt = num.dot(inv.grind_normal, view.deck_z())?;
                let signed = if effective_front { neg1(tilt) } else { tilt };
                let flip_sign = if flipped { -1.0 } else { 1.0 };
                let pitch = flip_sign * signed;
                let up_down = view.up_down;
                let mut wanted = up_down;
                let max_down = cvars.f(C::k5_OMaxDeckDown)?;
                if pitch > 0.0 && !(abs1(up_down) > f32::from_bits(0x3c23_d70a)) && pitch > max_down
                {
                    let gravity = cvars.f(C::k5_OGravity)?;
                    wanted = if effective_front {
                        neg1(gravity)
                    } else {
                        gravity
                    };
                }
                let mut result = wanted;
                let max_up = cvars.f(C::k5_OMaxDeckUp)?;
                if abs1(tilt) > max_up {
                    result = if effective_front {
                        fsel(neg1(wanted), wanted, 0.0)
                    } else {
                        fsel(neg1(wanted), 0.0, wanted)
                    };
                }
                if pitch < max_down {
                    result = if effective_front {
                        fsel(neg1(wanted), 0.0, wanted)
                    } else {
                        fsel(neg1(wanted), wanted, 0.0)
                    };
                }
                let loose_end = view.loose_end_height;
                if loose_end > 0.0 && !(pitch > loose_end) {
                    let flip = if flipped { -1.0 } else { 1.0 };
                    let side = if on_front { 1.0 } else { -1.0 };
                    let sign = side * flip;
                    let hold = cvars.f(C::k5_OGrabGrindHoldUpMultiplier)?;
                    let push = ((loose_end - pitch) * hold) * sign;
                    result = if sign > 0.0 {
                        fsel(sign - push, push, sign)
                    } else {
                        fsel(sign - push, sign, push)
                    };
                }
                if inv.loose_end_must_rise {
                    let flip = if flipped { -1.0 } else { 1.0 };
                    let side = if on_front { 1.0 } else { -1.0 };
                    let sign = side * flip;
                    let pry = cvars.f(C::k5_OPryOffOffset)?;
                    result = if sign > 0.0 {
                        let raised = pry + result;
                        fsel(1.0 - raised, raised, 1.0)
                    } else {
                        let lowered = result - pry;
                        fsel(-1.0 - lowered, -1.0, lowered)
                    };
                }
                clamped = result != wanted;
                let dead_zone = cvars.f(C::kDeckControlLRDeadzone)?;
                let twist = cvars.f(C::kBodyTwistLRDeckControl)?;
                let body = super::math::adjust_for_dead_zone(view.nudge, dead_zone) * twist;
                let mirror = if flipped { on_front } else { !on_front };
                let facing = if mirror { -1.0 } else { 1.0 };
                let board_scale = cvars.f(C::kBoardControlLRDeckControl)?;
                let board =
                    super::math::adjust_for_dead_zone(view.translation, dead_zone) * board_scale;
                let limit = cvars.f(C::kBoardControlLRDeckControl)?;
                let sum = board * facing + body;
                let low = fsel(neg1(limit) - sum, neg1(limit), sum);
                let lr = fsel(limit - low, low, limit);
                (lr, result)
            };
            let lr_cube = (lr * lr) * lr;
            let ud_cube = (ud * ud) * ud;
            let flip = if cvars.b(C::kFlipTruckGrindControl)? {
                -1.0
            } else {
                1.0
            };
            (flip * lr_cube, ud_cube)
        };
        let lr_smooth = cvars.f(C::kTruckGrindControlSmoothness)?;
        let ud_smooth = if clamped {
            cvars.f(C::kTruckGrindControlClampedSmoothness)?
        } else {
            cvars.f(C::kTruckGrindControlSmoothness)?
        };
        let lr = (1.0 - lr_smooth).mul_add(target_lr, lr_smooth * self.lr);
        let ud = (1.0 - ud_smooth).mul_add(target_ud, self.ud * ud_smooth);
        let tiny = f32::from_bits(0x38d1_b717);
        self.lr = if abs1(lr) < tiny { 0.0 } else { lr };
        self.ud = if abs1(ud) < tiny { 0.0 } else { ud };
        inv.truck_control_lr = self.lr;
        inv.truck_control_ud = self.ud;
        Ok(())
    }
}

impl GrindJumper {
    /// `Update` 0x82dcb350 with `Update_GrindExits` 0x82dcb2a8.
    pub(crate) fn try_update(
        &mut self,
        inv: &Investigation,
        view: &GrindView,
        cvars: &GrindCvars<'_>,
        effects: &mut SuffixEffects,
    ) -> Result<(), GrindError> {
        self.just_jumped = false;
        self.frames_avoiding = self.frames_avoiding.wrapping_sub(1).max(0);
        if self.frames_avoiding > 0 {
            effects.avoid_grinds = true;
        }
        if inv.possible {
            self.geometry_type = inv.geometry.geometry_type;
            self.most_vertical_normal = inv.geometry.upmost_normal;
            self.high_side_direction = inv.geometry.high_side_dir;
            self.grind_normal = inv.grind_normal;
            self.grind_direction = inv.grind_dir;
            self.primitive_location = inv.primitive_location;
        }
        if view.processed_grind_type != -1 {
            self.last_grind_type = view.processed_grind_type;
        }
        let energy = cvars.f(C::kJumpEnergyRegeneration)? + self.energy_fraction;
        self.energy_fraction = fsel(1.0 - energy, energy, 1.0);
        Ok(())
    }

    /// `LaunchSkateboard` 0x82dcb3a8: the jump velocity out of a grind. Native
    /// ends with `Skateboard::SetVelocity` at 0x82dcb8f0; this owner has no board
    /// access, so the caller performs that board write right after.
    pub(crate) fn try_launch_skateboard(
        &mut self,
        side_scalar: f32,
        vertical: f32,
        view: &GrindView,
        cvars: &GrindCvars<'_>,
        num: &mut GrindNum<'_>,
        effects: &mut SuffixEffects,
    ) -> Result<V4, GrindError> {
        let mut result = view.velocity;
        let is_backslash = self.last_grind_type == 4;
        let normal = self.grind_normal;
        let mut up = if self.last_grind_type != 2 && !is_backslash {
            let k = cvars.f(C::kJumpNormalVertFraction)?;
            let scaled = scale(normal, 1.0 - k);
            madd(self.most_vertical_normal, k, scaled)
        } else {
            let across = cross(normal, self.grind_direction);
            let offset = sub(view.deck_t(), self.primitive_location);
            let away = if num.dot(across, offset)? > 0.0 {
                across
            } else {
                neg(across)
            };
            let push = if is_backslash {
                scale(away, neg1(cvars.f(C::kBackslashSideJump)?))
            } else {
                scale(away, cvars.f(C::kTipslideSideJump)?)
            };
            add(normal, push)
        };
        let length = num.length(up)?;
        up = if length > f32::from_bits(0x3dcc_cccd) {
            scale(up, num.reciprocal(length)?)
        } else {
            normal
        };
        let along = num.dot(result, up)?;
        result = sub(result, scale(up, along));
        let lift = cvars.f(C::kGrindCommonJumpVertScalar)? * vertical;
        result = madd(up, lift, result);
        let side_axis = cross(self.grind_direction, up);
        let mut nudge = view.nudge;
        if self.energy_fraction < cvars.f(C::kSideJumpEnergy)? {
            let minimum = cvars.f(C::kMinSideJumpLowEnergy)?;
            if nudge < 0.0 {
                let negative = neg1(minimum);
                nudge = fsel(nudge - negative, negative, nudge);
            } else if nudge > 0.0 {
                nudge = fsel(nudge - minimum, nudge, minimum);
            } else {
                let offset = sub(view.deck_t(), self.primitive_location);
                nudge = if num.dot(offset, side_axis)? > 0.0 {
                    minimum
                } else {
                    neg1(minimum)
                };
            }
            self.frames_avoiding = cvars.i(C::kTrajSelectorAvoidGrindFrames)?;
            if self.frames_avoiding > 0 {
                effects.avoid_grinds = true;
            }
            effects.overlay_lookups += 1;
        }
        let jump_side = cvars.f(C::kGrindCommonJumpSide)?;
        let mut side = scale(side_axis, jump_side * nudge);
        let geometry_scalar = if self.geometry_type == THIN_RAIL {
            0.0
        } else {
            side_scalar
        };
        side = sub(side, scale(self.high_side_direction, geometry_scalar));
        let side_length = num.length(side)?;
        if side_length > jump_side {
            side = scale(side, jump_side / side_length);
        }
        result = add(result, side);
        self.just_jumped = true;
        let energy = self.energy_fraction - cvars.f(C::kJumpEnergyCost)?;
        self.energy_fraction = fsel(neg1(energy), 0.0, energy);
        Ok(result)
    }
}
