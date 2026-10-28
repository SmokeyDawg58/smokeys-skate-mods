//! `GrindInvestigator`: `Update` 0x82de1718 and `CheckForLocalGrind`
//! 0x82ddf678 with the per-type checks `CheckFor5050` 0x82ddb0a0,
//! `CheckFor5_O` 0x82ddb768, `CheckForTipslide` 0x82ddb358 and
//! `CheckForBoardSlide` 0x82ddabd0, plus their shared entry/distance tests.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_GRIND_5050,
    PHYSICS_STATE_GRIND_5_O, PHYSICS_STATE_GRIND_BACKSLASH, PHYSICS_STATE_GRIND_BOARDSLIDE,
    PHYSICS_STATE_GRIND_TIPSLIDE, PHYSICS_STATE_NONSPECIFIC, PHYSICS_STATE_PHYSICS_GROUND,
};
use super::{
    collision,
    fields::GrindInvestigator,
    math::{DEG_TO_RAD, GrindNum, I, V4, abs1, add, cross, fsel, neg, neg1, scale, sub},
    types::{
        AIR_TO_GRIND, BOARDSLIDE, CollTestPair, CollTestResult, DROP_IN, GRIND_5_O, GRIND_5050,
        GRIND_TO_GRIND, GrindPrim, Investigation, RIDE_FROM_ABOVE, RIDE_FROM_BELOW,
        RIDE_INTO_COPING, STAY_IN_GRIND, TIPSLIDE,
    },
    view::{
        DECK_CENTER_TO_TRUCK, GrindAttributes, GrindCvars, GrindError, GrindGraph, GrindView,
        TRUCK_TO_WHEEL, error,
    },
};
use crate::{
    debug_overlay::DebugOverlaySlot, grind_data_runtime::GrindDataPrimitive,
    input_cvar_runtime::grind_states::GrindCvar as C,
};

pub(crate) struct InvestigatorContext<'a, 'n> {
    pub(crate) view: &'a GrindView,
    pub(crate) attributes: &'a GrindAttributes,
    pub(crate) cvars: &'a GrindCvars<'a>,
    /// The `GrindCollision` query result, in its native slot order.
    pub(crate) prims: &'a [GrindDataPrimitive],
    pub(crate) overlay: &'a DebugOverlaySlot,
    pub(crate) num: &'a mut GrindNum<'n>,
}
impl InvestigatorContext<'_, '_> {
    fn prim(&self, index: i32) -> Result<&GrindDataPrimitive, GrindError> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.prims.get(i))
            .ok_or_else(|| GrindError(format!("grind primitive {index} outside the query")))
    }
    fn overlay(&self) -> Result<(), GrindError> {
        self.overlay.require().map(|_| ()).map_err(error)
    }
}

/// The per-call collision caches of `CheckForLocalGrind`: each intersector is
/// computed at most once per investigation and reused by the hang search.
#[derive(Default)]
struct Cached {
    truck: Option<CollTestPair>,
    tip: Option<CollTestPair>,
    board: Option<CollTestResult>,
}

impl GrindInvestigator {
    /// `Update` 0x82de1718.
    pub(crate) fn try_update_investigation(
        &mut self,
        ctx: &mut InvestigatorContext<'_, '_>,
        forbidden: bool,
    ) -> Result<Investigation, GrindError> {
        let mut out = self.try_check_for_local_grind(ctx, forbidden)?;
        if out.possible {
            out.grind_dir = calc_grind_dir(ctx, &out.primitive)?;
        }
        // DebugDraw 0x82dd7368: one DebugOverlay lookup per nearby primitive.
        if ctx.cvars.b(C::kDrawNearbyGrindPrimatives)? {
            for _ in ctx.prims {
                ctx.overlay()?;
            }
        }
        self.last_velocity = ctx.view.velocity;
        Ok(out)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one native priority order with shared collision caches"
    )]
    fn try_check_for_local_grind(
        &mut self,
        ctx: &mut InvestigatorContext<'_, '_>,
        forbidden: bool,
    ) -> Result<Investigation, GrindError> {
        let view = ctx.view;
        let state = view.state;
        let boardslide_timer = self.hang_primitive_existed
            && state == PHYSICS_STATE_PHYSICS_GROUND
            && view.wheels < 2
            && view.deck_speed < ctx.cvars.f(C::kGroundToBoardslideSpeed)?;
        self.ground_to_boardslide_timer = if boardslide_timer {
            self.ground_to_boardslide_timer.wrapping_add(1)
        } else {
            0
        };
        let exiting = view.substate == 2;
        self.avoid_grind_frames = if exiting {
            ctx.cvars.i(C::kGrindExitFrames)?
        } else {
            self.avoid_grind_frames.wrapping_sub(1).max(0)
        };
        self.avoid_five_o_frames = self.avoid_five_o_frames.wrapping_sub(1).max(0);
        let on_ground = view.category == PHYSICS_CATEGORY_GROUND;
        self.ground_frames = if on_ground {
            self.ground_frames.wrapping_add(1)
        } else {
            0
        };
        self.non_ground_frames = if on_ground {
            0
        } else {
            self.non_ground_frames.wrapping_add(1)
        };
        let mut local = Investigation::reset();
        local.hang_exists = false;
        local.possible = false;
        if ctx.prims.is_empty() {
            return Ok(self.finish_local(local));
        }
        let mut cache = Cached::default();
        if !forbidden && (self.avoid_grind_frames == 0 || exiting) {
            if let Some(found) = self.try_checks(ctx, &mut cache, state)? {
                return Ok(found);
            }
        }
        // Hang primitive search: truck ends, then the boardslide quad, then tips.
        let mut hang_is_boardslide = false;
        let mut found: Option<(i32, V4)> = None;
        let truck = match cache.truck {
            Some(pair) => pair,
            None => collision::truck_collisions(view, ctx.attributes, ctx.prims, ctx.num)?,
        };
        let truck_candidate = if truck.front.hit {
            Some((truck.front.index, truck.front.point))
        } else if truck.rear.hit {
            Some((truck.rear.index, truck.rear.point))
        } else {
            None
        };
        if let Some((index, point)) = truck_candidate {
            if index != -1 {
                found = Some((index, point));
            }
        }
        if found.is_none() {
            let board = match cache.board {
                Some(result) => result,
                None => collision::boardslide_collision(view, ctx.attributes, ctx.prims, ctx.num)?,
            };
            if board.hit {
                hang_is_boardslide = true;
                if board.index != -1 {
                    found = Some((board.index, board.point));
                }
            }
        }
        if found.is_none() {
            let tip = match cache.tip {
                Some(pair) => pair,
                None => collision::tip_collisions(view, ctx.attributes, ctx.prims, ctx.num)?,
            };
            let candidate = if tip.front.hit {
                Some((tip.front.index, tip.front.point))
            } else if tip.rear.hit {
                Some((tip.rear.index, tip.rear.point))
            } else {
                None
            };
            if let Some((index, point)) = candidate {
                if index > -1 {
                    found = Some((index, point));
                }
            }
        }
        if let Some((index, point)) = found {
            if index > -1 {
                let prim = ctx.prim(index)?;
                local.hang_primitive = GrindPrim::from_query(prim);
                local.hang_primitive_location = point;
                local.hang_is_boardslide = hang_is_boardslide;
                local.hang_exists = true;
            }
        }
        Ok(self.finish_local(local))
    }

    fn finish_local(&mut self, mut local: Investigation) -> Investigation {
        local.possible = false;
        self.hang_primitive_existed = local.hang_exists;
        local
    }

    /// The state-priority check, then the default order. Returns the first
    /// possible investigation; `CheckFor5_O` runs again in the default order.
    fn try_checks(
        &self,
        ctx: &mut InvestigatorContext<'_, '_>,
        cache: &mut Cached,
        state: i32,
    ) -> Result<Option<Investigation>, GrindError> {
        let mut five_fifty = Investigation::reset();
        let mut tip_result = Investigation::reset();
        let mut board_result = Investigation::reset();
        match state {
            PHYSICS_STATE_GRIND_5050 | PHYSICS_STATE_GRIND_5_O => {
                let pair =
                    collision::truck_collisions(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
                cache.truck = Some(pair);
                five_fifty = self.check_for_5050(ctx, &pair)?;
                if five_fifty.possible {
                    return Ok(Some(five_fifty));
                }
                let five_o = self.check_for_5_o(ctx, &pair)?;
                if five_o.possible {
                    return Ok(Some(five_o));
                }
            }
            PHYSICS_STATE_GRIND_TIPSLIDE | PHYSICS_STATE_GRIND_BACKSLASH => {
                let pair = collision::tip_collisions(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
                cache.tip = Some(pair);
                tip_result = self.check_for_tipslide(ctx, &pair)?;
                if tip_result.possible {
                    return Ok(Some(tip_result));
                }
            }
            PHYSICS_STATE_GRIND_BOARDSLIDE => {
                let result =
                    collision::boardslide_collision(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
                cache.board = Some(result);
                board_result = self.check_for_boardslide(ctx, &result)?;
                if board_result.possible {
                    return Ok(Some(board_result));
                }
            }
            _ => {}
        }
        let truck = if let Some(pair) = cache.truck {
            pair
        } else {
            let pair = collision::truck_collisions(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
            cache.truck = Some(pair);
            five_fifty = self.check_for_5050(ctx, &pair)?;
            pair
        };
        if five_fifty.possible {
            return Ok(Some(five_fifty));
        }
        let five_o = self.check_for_5_o(ctx, &truck)?;
        if five_o.possible {
            return Ok(Some(five_o));
        }
        if cache.tip.is_none() {
            let pair = collision::tip_collisions(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
            cache.tip = Some(pair);
            tip_result = self.check_for_tipslide(ctx, &pair)?;
        }
        if tip_result.possible {
            return Ok(Some(tip_result));
        }
        if cache.board.is_none() {
            let result =
                collision::boardslide_collision(ctx.view, ctx.attributes, ctx.prims, ctx.num)?;
            cache.board = Some(result);
            board_result = self.check_for_boardslide(ctx, &result)?;
        }
        if board_result.possible {
            return Ok(Some(board_result));
        }
        Ok(None)
    }

    /// `CheckFor5050` 0x82ddb0a0: both trucks on the same line.
    fn check_for_5050(
        &self,
        ctx: &mut InvestigatorContext<'_, '_>,
        pair: &CollTestPair,
    ) -> Result<Investigation, GrindError> {
        let view = ctx.view;
        let mut out = Investigation::reset();
        let translation = abs1(view.translation);
        let nudge = abs1(view.nudge);
        let mut test_entry = false;
        if pair.front.hit && pair.rear.hit {
            let steer = fsel(nudge - translation, nudge, translation);
            if steer < ctx.cvars.f(C::k5050MaxSteer)? {
                let rear = pair.rear.point;
                let front = pair.front.point;
                let contact_dir = sub(front, rear);
                let t = view.deck_t();
                let above_front = sub(t, front);
                let y = view.deck_y();
                let above_rear = sub(t, rear);
                let centre = scale(add(front, rear), 0.5);
                let front_height = ctx.num.dot(above_front, y)?;
                let rear_height = ctx.num.dot(above_rear, y)?;
                let tangent = ctx.num.normalize_unsafe(contact_dir)?;
                let height = fsel(front_height - rear_height, front_height, rear_height);
                if height < ctx.cvars.f(C::k5050ContactDist)? {
                    let mut separated = false;
                    let prim = ctx.prim(pair.front.index)?.clone();
                    if pair.front.index != pair.rear.index {
                        let dir = ctx.num.grind_dir(prim.start, prim.end)?;
                        let up = ctx.num.upmost(dir)?;
                        let side = cross(up, dir);
                        let separation = ctx.num.dot(contact_dir, side)?;
                        if separation > ctx.cvars.f(C::k5050MaxSeparation)? {
                            separated = true;
                        }
                    }
                    if !separated {
                        out.possible = true;
                        out.grind_type = GRIND_5050;
                        out.primitive_location = centre;
                        out.front_primitive_location = front;
                        out.rear_primitive_location = rear;
                        out.primitive_tangent = tangent;
                        out.primitive = GrindPrim::from_query(&prim);
                    }
                    test_entry = true;
                }
            }
        }
        if test_entry {
            let tangent = out.primitive_tangent;
            let (acute, entry) = is_entry_angle_acute_enough(ctx, PHYSICS_STATE_GRIND_5050, tangent, false)?;
            out.entry_type = entry;
            if !acute {
                out.possible = false;
            }
        } else {
            out.possible = false;
        }
        Ok(out)
    }

    /// `CheckFor5_O` 0x82ddb768: one truck, leading or trailing by weight.
    #[expect(clippy::too_many_lines, reason = "native lead/trail selection order")]
    fn check_for_5_o(
        &self,
        ctx: &mut InvestigatorContext<'_, '_>,
        pair: &CollTestPair,
    ) -> Result<Investigation, GrindError> {
        let view = ctx.view;
        let mut out = Investigation::reset();
        out.possible = false;
        out.grind_type = GRIND_5_O;
        if !pair.front.hit && !pair.rear.hit {
            return Ok(out);
        }
        if view.flags_8c8 & 1 != 0 {
            return Ok(out);
        }
        let mut front_entry = AIR_TO_GRIND;
        let mut rear_entry = AIR_TO_GRIND;
        let mut front_tangent = I;
        let mut rear_tangent = I;
        let contact = ctx.cvars.f(C::k5_OContactDist)?;
        let mut front_ok = false;
        let mut rear_ok = false;
        if pair.front.hit {
            front_ok = is_within_dist_to_deck(ctx, pair.front.point, contact)?;
        }
        if pair.rear.hit {
            rear_ok = is_within_dist_to_deck(ctx, pair.rear.point, contact)?;
        }
        if front_ok {
            let prim = ctx.prim(pair.front.index)?;
            let (start, end) = (prim.start, prim.end);
            front_tangent = ctx.num.normalize_unsafe(sub(end, start))?;
            let (acute, entry) = is_entry_angle_acute_enough(ctx, PHYSICS_STATE_GRIND_5_O, front_tangent, true)?;
            front_entry = entry;
            front_ok = acute && is_5_o_orientation_ok(ctx, front_tangent)?;
        }
        if rear_ok {
            let prim = ctx.prim(pair.rear.index)?;
            let (start, end) = (prim.start, prim.end);
            rear_tangent = ctx.num.normalize_unsafe(sub(end, start))?;
            let (acute, entry) = is_entry_angle_acute_enough(ctx, PHYSICS_STATE_GRIND_5_O, rear_tangent, true)?;
            rear_entry = entry;
            rear_ok = acute && is_5_o_orientation_ok(ctx, rear_tangent)?;
        }
        let forward = ctx.num.dot(view.deck_z(), view.velocity)? > 0.0;
        let lead_ok = if forward { front_ok } else { rear_ok };
        let trail_ok = if forward { rear_ok } else { front_ok };
        let weight_frames = self.ground_frames > ctx.cvars.i(C::kAnimWeightGroundFrames)?;
        let weight_forwards = view.flags_8c8 & 0x4000 != 0;
        let mut choose_lead = false;
        let mut choose_trail = false;
        if lead_ok && trail_ok {
            let lean_forward = if weight_frames {
                weight_forwards
            } else {
                view.translation > ctx.cvars.f(C::k5_OLeanForward)?
            };
            if lean_forward {
                choose_lead = true;
            } else {
                choose_trail = true;
            }
        } else if trail_ok {
            choose_trail = true;
        } else if lead_ok {
            choose_lead = if weight_frames { weight_forwards } else { true };
        }
        if choose_lead && choose_trail {
            // Debug assertion 0x82ddbb0c; the shipped build continues.
        }
        let (use_rear, use_front) = if forward {
            (choose_trail, choose_lead)
        } else {
            (choose_lead, choose_trail)
        };
        if use_rear {
            out.primitive_tangent = rear_tangent;
            out.primitive_location = pair.rear.point;
            out.front_primitive_location = pair.rear.point;
            out.primitive = GrindPrim::from_query(ctx.prim(pair.rear.index)?);
            out.grind_on_front = false;
            out.possible = true;
            out.entry_type = rear_entry;
        } else if use_front {
            out.primitive_tangent = front_tangent;
            out.primitive_location = pair.front.point;
            out.front_primitive_location = pair.front.point;
            out.primitive = GrindPrim::from_query(ctx.prim(pair.front.index)?);
            out.grind_on_front = true;
            out.possible = true;
            out.entry_type = front_entry;
        }
        Ok(out)
    }

    /// `CheckForTipslide` 0x82ddb358.
    fn check_for_tipslide(
        &self,
        ctx: &mut InvestigatorContext<'_, '_>,
        pair: &CollTestPair,
    ) -> Result<Investigation, GrindError> {
        let view = ctx.view;
        let mut out = Investigation::reset();
        let any = pair.front.hit || pair.rear.hit;
        out.possible = false;
        if view.flags_8c8 & 1 != 0 || !any {
            return Ok(out);
        }
        let (side, on_front) = if pair.front.hit {
            (&pair.front, true)
        } else {
            (&pair.rear, false)
        };
        let prim = ctx.prim(side.index)?;
        let direction = sub(prim.end, prim.start);
        out.primitive_location = side.point;
        out.primitive = GrindPrim::from_query(prim);
        out.grind_on_front = on_front;
        out.primitive_tangent = ctx.num.normalize_unsafe(direction)?;
        let (acute, entry) = is_entry_angle_acute_enough(ctx, PHYSICS_STATE_GRIND_TIPSLIDE, out.primitive_tangent, true)?;
        out.entry_type = entry;
        if !acute {
            return Ok(out);
        }
        let mut allowed = true;
        let drop_in_test = entry == DROP_IN || (view.velocity[1] < 0.0 && view.category != PHYSICS_CATEGORY_GRIND);
        if drop_in_test {
            let rise = view.deck_t()[1] - out.primitive_location[1];
            allowed = rise > ctx.cvars.f(C::kMinDeckDZTipslideDropIn)?;
        }
        if !allowed {
            // "tilted drop in tipslide rejected" DebugOverlay print.
            ctx.overlay()?;
            return Ok(out);
        }
        if !is_within_dist_to_deck(
            ctx,
            out.primitive_location,
            ctx.cvars.f(C::kContactDist_Tipslide)?,
        )? {
            return Ok(out);
        }
        let tangent = out.primitive_tangent;
        let along = ctx.num.dot(tangent, view.velocity)?;
        let perpendicular = sub(view.velocity, scale(tangent, along));
        let perpendicular_squared = ctx.num.dot(perpendicular, perpendicular)?;
        let ignore = ctx.cvars.f(C::kMinSpeedWeightIgnore)?;
        let mut accept = true;
        if perpendicular_squared < ignore * ignore {
            if abs1(view.balance) > 0.0 {
                let offset = sub(out.primitive_location, view.deck_t());
                accept = 0.0 > ctx.num.dot(offset, view.ground_normal)?;
            } else if self.ground_frames > ctx.cvars.i(C::kAnimWeightGroundFrames)? {
                let offset = sub(out.primitive_location, view.deck_t());
                let ahead = ctx.num.dot(offset, view.velocity)? > 0.0;
                let weight_forwards = view.flags_8c8 & 0x4000 != 0;
                accept = if ahead {
                    weight_forwards
                } else {
                    !weight_forwards
                };
            }
        }
        if accept {
            out.grind_type = TIPSLIDE;
            out.possible = true;
        }
        Ok(out)
    }

    /// `CheckForBoardSlide` 0x82ddabd0.
    fn check_for_boardslide(
        &self,
        ctx: &mut InvestigatorContext<'_, '_>,
        result: &CollTestResult,
    ) -> Result<Investigation, GrindError> {
        let view = ctx.view;
        let fail = Investigation::reset;
        if view.flags_8c8 & 1 != 0 || !result.hit {
            return Ok(fail());
        }
        let prim = ctx.prim(result.index)?.clone();
        let direction = sub(prim.end, prim.start);
        let y = view.deck_y();
        let grind_dir = ctx.num.normalize_by_length(direction)?;
        let up = ctx.num.upmost(grind_dir)?;
        if ctx.num.dot(up, y)? <= ctx.cvars.f(C::kMinBoardslideDeckTilt)? {
            return Ok(fail());
        }
        let (acute, entry) = is_entry_angle_acute_enough(ctx, PHYSICS_STATE_GRIND_BOARDSLIDE, grind_dir, false)?;
        if !acute {
            return Ok(fail());
        }
        if !is_within_dist_to_deck(ctx, result.point, ctx.cvars.f(C::kContactDist_Boardslide)?)? {
            return Ok(fail());
        }
        let along = abs1(ctx.num.dot(view.velocity, grind_dir)?);
        if view.category == PHYSICS_CATEGORY_GROUND
            && !(along > ctx.cvars.f(C::kMinGrindSpeedBoardslide)?)
            && self.ground_to_boardslide_timer <= ctx.cvars.i(C::kGroundToBoardslideFrames)?
        {
            return Ok(fail());
        }
        let rise = ctx.num.dot(direction, y)?;
        let flat = sub(direction, scale(y, rise));
        let flat_length = ctx.num.length(flat)?;
        if !(flat_length > 0.0) {
            return Ok(fail());
        }
        let offset = sub(result.point, view.deck_t());
        let offset_rise = ctx.num.dot(offset, y)?;
        let in_plane = sub(offset, scale(y, offset_rise));
        let unit = scale(flat, ctx.num.reciprocal(flat_length)?);
        let cos_z = ctx.num.dot(unit, view.deck_z())?;
        let in_plane_length = ctx.num.length(in_plane)?;
        let abs_cos = fsel(cos_z, cos_z, neg1(cos_z));
        let sine_squared = (-abs_cos).mul_add(abs_cos, 1.0);
        let sine = sqrt_guarded(ctx.num, sine_squared)?;
        let to_truck = ctx.attributes.scalar(DECK_CENTER_TO_TRUCK)?;
        let to_wheel = ctx.attributes.scalar(TRUCK_TO_WHEEL)?;
        let reach = if abs_cos > 0.0 {
            ((to_truck - in_plane_length) / abs_cos) * sine
        } else {
            f32::from_bits(0x4479_c000)
        };
        if !(reach > to_wheel) {
            return Ok(fail());
        }
        let mut out = Investigation::reset();
        out.possible = true;
        out.primitive_tangent = grind_dir;
        out.primitive = GrindPrim::from_query(&prim);
        out.grind_type = BOARDSLIDE;
        out.entry_type = entry;
        out.primitive_location = result.point;
        Ok(out)
    }

    /// `TweakForGeometry` 0x82dd6e38, after `CalcGrindGeometry`.
    pub(crate) fn try_tweak_for_geometry(
        &mut self,
        ctx: &mut InvestigatorContext<'_, '_>,
        inv: &mut Investigation,
        exit_tilt_deg: f32,
        balance_high_side: V4,
    ) -> Result<(), GrindError> {
        let view = ctx.view;
        if inv.geometry.geometry_type == super::types::IMPOSSIBLE {
            inv.possible = false;
            ctx.overlay()?;
        }
        if inv.grind_type == TIPSLIDE
            && inv.geometry.is_stair
            && view.category != PHYSICS_CATEGORY_GRIND
            && self.non_ground_frames < ctx.cvars.i(C::kMinNonGroundStairFrames)?
        {
            inv.possible = false;
            ctx.overlay()?;
        }
        if inv.grind_type == BOARDSLIDE && inv.geometry.is_stair {
            inv.possible = false;
            ctx.overlay()?;
        }
        if inv.possible && is_tipslide_a_backslash(ctx, inv)? {
            inv.grind_type = super::types::BACKSLASH;
        }
        if inv.possible && exit_tilt_deg > 0.0 && inv.geometry.geometry_type != super::types::LEDGE
        {
            let dot = ctx.num.dot(balance_high_side, inv.geometry.high_side_dir)?;
            if 0.0 > dot {
                inv.possible = false;
                ctx.overlay()?;
            }
        }
        if inv.possible && inv.geometry.geometry_type != super::types::THIN_RAIL {
            let velocity = view.velocity;
            let along = ctx.num.dot(inv.grind_dir, velocity)?;
            let mut across = sub(scale(inv.grind_dir, along), velocity);
            across[1] = 0.0;
            let squared = ctx.num.dot(across, across)?;
            let avoid = ctx.cvars.f(C::kHorizontalGrindAvoidSpeed)?;
            if squared > avoid * avoid {
                let toward = ctx.num.dot(velocity, inv.geometry.high_side_dir)?;
                if 0.0 > toward {
                    inv.possible = false;
                    ctx.overlay()?;
                }
            }
        }
        if inv.grind_type == GRIND_5_O && inv.geometry.geometry_type == super::types::LEDGE {
            if self.avoid_five_o_frames > 0 {
                inv.possible = false;
                return Ok(());
            }
            let mut dir = view.deck_z();
            if inv.grind_on_front {
                dir = scale(dir, -1.0);
            }
            if ctx.num.dot(dir, inv.geometry.high_side_dir)? > 0.0 {
                let rise = ctx.num.dot(dir, inv.geometry.upmost_normal)?;
                inv.loose_end_must_rise = rise < ctx.cvars.f(C::kLooseEndRiseThresh)?;
                if rise < 0.0 {
                    self.avoid_five_o_frames = 30;
                    inv.possible = false;
                    ctx.overlay()?;
                }
            }
        }
        Ok(())
    }
}

/// Guarded square root of a scalar the native computes with the replicated
/// `vrsqrtefp` chain: `x * rsqrt(x)`, zero for an exactly zero input.
fn sqrt_guarded(num: &mut GrindNum<'_>, value: f32) -> Result<f32, GrindError> {
    use crate::reckoning_numeric::{ReckoningNumeric, length_from_squared_and_estimate};
    let estimate = num.n.reciprocal_sqrt_estimate(value).map_err(error)?;
    Ok(length_from_squared_and_estimate(value, estimate))
}

/// `CalcGrindDir` 0x82dd2e80: the primitive direction against the velocity.
pub(crate) fn calc_grind_dir(
    ctx: &mut InvestigatorContext<'_, '_>,
    prim: &GrindPrim,
) -> Result<V4, GrindError> {
    let dir = ctx.num.grind_dir(prim.start, prim.end)?;
    if ctx.num.dot(dir, ctx.view.velocity)? < 0.0 {
        return Ok(scale(dir, -1.0));
    }
    Ok(dir)
}

/// `IsWithinDistToDeck` 0x82dd30a0: the deck-normal distance only.
fn is_within_dist_to_deck(
    ctx: &mut InvestigatorContext<'_, '_>,
    location: V4,
    distance: f32,
) -> Result<bool, GrindError> {
    let y = ctx.view.deck_y();
    let offset = sub(ctx.view.deck_t(), location);
    let height = ctx.num.dot(offset, y)?;
    let normal = scale(y, height);
    Ok(ctx.num.dot(normal, normal)? < distance * distance)
}

/// `Is5_OOrientationOK` 0x82dd30f0.
fn is_5_o_orientation_ok(
    ctx: &mut InvestigatorContext<'_, '_>,
    tangent: V4,
) -> Result<bool, GrindError> {
    let across = abs1(ctx.num.dot(ctx.view.deck_x(), tangent)?);
    if !(across < ctx.cvars.f(C::kMaxCos_5_O)?) {
        return Ok(false);
    }
    let tilt = abs1(ctx.num.dot(tangent, ctx.view.deck_y())?);
    Ok(tilt < ctx.cvars.f(C::kMaxTiltFor_5_O)?)
}

/// `EntryAngleTest` 0x82dd71c0.
fn entry_angle_test(
    ctx: &mut InvestigatorContext<'_, '_>,
    tangent: V4,
    angle_deg: f32,
) -> Result<bool, GrindError> {
    let up = ctx.num.upmost(tangent)?;
    let velocity = ctx.view.velocity;
    let rise = ctx.num.dot(velocity, up)?;
    let flat = sub(velocity, scale(up, rise));
    let length = ctx.num.length(flat)?;
    if length <= f32::from_bits(0x3a83_126f) {
        return Ok(true);
    }
    let unit = scale(flat, ctx.num.reciprocal(length)?);
    let mut cosine = ctx.num.dot(unit, tangent)?;
    if cosine < 0.0 {
        cosine = ctx.num.dot(unit, neg(tangent))?;
    }
    let limit = ctx.num.cosine(angle_deg * DEG_TO_RAD)?;
    Ok(cosine > limit)
}

/// `IsEntryAngleAcuteEnough` 0x82dda748: classifies the entry and applies the
/// entry-type test; returns the pass flag and the written entry type.
pub(crate) fn is_entry_angle_acute_enough(
    ctx: &mut InvestigatorContext<'_, '_>,
    testing: i32,
    tangent: V4,
    drop_in: bool,
) -> Result<(bool, i32), GrindError> {
    let view = ctx.view;
    let entry = if view.category == PHYSICS_CATEGORY_GRIND || view.state == PHYSICS_STATE_NONSPECIFIC {
        if testing == view.state || (testing == PHYSICS_STATE_GRIND_TIPSLIDE && view.state == PHYSICS_STATE_GRIND_BACKSLASH) {
            STAY_IN_GRIND
        } else {
            GRIND_TO_GRIND
        }
    } else if view.category == PHYSICS_CATEGORY_GROUND {
        if drop_in && view.deck_speed < ctx.cvars.f(C::kMaxDropInSpeed)? {
            DROP_IN
        } else {
            let slope = ctx.num.approach_slope_sine(tangent, view.velocity)?;
            if slope > f32::from_bits(0x3f75_c28f) {
                RIDE_INTO_COPING
            } else if slope > 0.25 {
                RIDE_FROM_BELOW
            } else {
                RIDE_FROM_ABOVE
            }
        }
    } else {
        AIR_TO_GRIND
    };
    let pass = match entry {
        AIR_TO_GRIND | DROP_IN => true,
        STAY_IN_GRIND => entry_angle_test(ctx, tangent, ctx.cvars.f(C::kStayEntryAngle)?)?,
        GRIND_TO_GRIND => entry_angle_test(ctx, tangent, ctx.cvars.f(C::kChangeEntryAngle)?)?,
        RIDE_FROM_ABOVE => entry_angle_test(ctx, tangent, ctx.cvars.f(C::kAboveEntryAngle)?)?,
        _ => {
            let velocity = view.velocity;
            let along = ctx.num.dot(velocity, tangent)?;
            let speed_squared = ctx.num.dot(velocity, velocity)?;
            let across = sub(scale(tangent, along), velocity);
            let across_squared = ctx.num.dot(across, across)?;
            let sine = if speed_squared > f32::from_bits(0x3b03_126f) {
                sqrt_guarded(ctx.num, across_squared / speed_squared)?
            } else {
                1.0
            };
            let dv1 = ctx.cvars.f(C::kMaxCopingDV1)?;
            let dv0 = ctx.cvars.f(C::kMaxCopingDV0)?;
            let slope = ctx.num.approach_slope_sine(tangent, velocity)?;
            let scalar = ctx
                .attributes
                .graph(GrindGraph::DvEntryThreshScalarVsSinSlope, slope)?;
            let limit = (dv0 - dv1).mul_add(1.0 - sine, dv1) * scalar;
            // "sine slope %f scalar %f : %s" DebugOverlay print.
            ctx.overlay()?;
            across_squared < limit * limit
        }
    };
    if ctx.cvars.b(C::kSpamGrindEntryAngle)? {
        ctx.overlay()?;
    }
    Ok((pass, entry))
}

/// `IsTipslideABackslash` 0x82dd2f28.
fn is_tipslide_a_backslash(
    ctx: &mut InvestigatorContext<'_, '_>,
    inv: &Investigation,
) -> Result<bool, GrindError> {
    if inv.grind_type != TIPSLIDE || ctx.view.state == PHYSICS_STATE_GRIND_TIPSLIDE || inv.geometry.uncertain {
        return Ok(false);
    }
    let location = inv.primitive_location;
    let first = sub(inv.geometry.line1_hit, location);
    let second = sub(inv.geometry.line2_hit, location);
    let up = inv.geometry.upmost_normal;
    let first_height = ctx.num.dot(first, up)?;
    let second_height = ctx.num.dot(second, up)?;
    let difference = abs1(first_height - second_height);
    let offset = ctx.attributes.scalar(DECK_CENTER_TO_TRUCK)?;
    if !(difference > offset * f32::from_bits(0x3dcc_cccd)) {
        return Ok(false);
    }
    let higher = if first_height > second_height {
        first
    } else {
        second
    };
    let deck = sub(ctx.view.deck_t(), location);
    Ok(ctx.num.dot(deck, higher)? > 0.0)
}
