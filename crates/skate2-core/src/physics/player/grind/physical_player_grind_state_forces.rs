//! The world-space deck forces of the grind states, each returned for one
//! `Skateboard::ApplyWorldSpaceForceToDeck` at the Processed deck position in
//! the caller's native order. `None` is the native early return.
//! `ApplyFriction` 0x82d85578, `ApplyCopingAssistance` 0x82d857d8,
//! `ApplyPinForce` 0x82d858a0, `ApplyTruckControlCompensationForce`
//! 0x82d85940, `ApplyTranslationControl` 0x82d854f8, `ApplyExitNudge`
//! 0x82d85330, `ApplyAlignmentForce` 0x82d8fa70, `AssistExitLean` 0x82d8fd78,
//! Boardslide `ApplyTranslation` 0x82d90970, Tipslide `ApplyStabilityNudge`
//! 0x82d86208.
use crate::{
    input_cvar_runtime::grind_states::GrindCvar as C,
    physical_player_grind_manager::{
        math::{GrindNum, J, V4, abs1, cross, fsel, madd, neg, neg1, scale, sub},
        types::{FAT_RAIL, Investigation, LEDGE, THIN_RAIL},
        view::{GrindAttributes, GrindCvars, GrindError, GrindGraph, GrindView},
    },
};

/// The grind state's own vectors (+20 dir, +30 normal, +40 skew).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub(crate) dir: V4,
    pub(crate) normal: V4,
    pub(crate) skew: V4,
}

fn with_y(mut v: V4, y: f32) -> V4 {
    v[1] = y;
    v
}

/// `ApplyFriction(thin, fat, ledge)`: the geometry's friction, the forced or
/// detected physics surface, the curb scalar and the friction-vs-time scalar
/// against the velocity component along the grind.
#[expect(clippy::too_many_arguments, reason = "native argument list")]
pub(crate) fn friction(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    thin: f32,
    fat: f32,
    ledge: f32,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    let geometry = match inv.geometry.geometry_type {
        THIN_RAIL => thin,
        FAT_RAIL => fat,
        _ => ledge,
    };
    let forced = cvars.e(C::kForcedGrindSurface)?;
    let surface_type = if forced == 0 {
        i32::try_from(inv.geometry.physics_surface)
            .map_err(|_| GrindError("grind physics surface out of range".into()))?
    } else {
        forced
    };
    let mut surface = cvars.f(C::kGrindFriction_Smooth)?;
    let scaled = match surface_type {
        2 => Some(C::kGrindFriction_Rough),
        3 => Some(C::kGrindFriction_Slow),
        4 => Some(C::kGrindFriction_Slippery),
        5 => Some(C::kGrindFriction_VerySlow),
        6 => Some(C::kGrindFriction_Unrideable),
        _ => None,
    };
    if let Some(cvar) = scaled {
        surface = cvars.f(cvar)?;
    }
    let curb = if inv.geometry.is_curb {
        cvars.f(C::kCurbFrictionScalar)?
    } else {
        1.0
    };
    let along = num.dot(view.velocity, frame.normal)?;
    let tangential = sub(view.velocity, scale(frame.normal, along));
    let length = num.length(tangential)?;
    if !(length > f32::from_bits(0x3a83_126f)) {
        return Ok(None);
    }
    let up = num.dot(inv.geometry.upmost_normal, J)?;
    let inverse = -1.0 / length;
    let common = cvars.f(C::kGrindCommonFrictionScalar)?;
    let upward = fsel(neg1(up), 0.0, up);
    let magnitude =
        (((((upward * inverse) * common) * inv.friction_time_scalar) * curb) * surface) * geometry;
    Ok(Some(scale(tangential, magnitude)))
}

/// `ApplyCopingAssistance`: while the relief timer runs and the board moves
/// against the upmost normal, the coping support force straight up.
pub(crate) fn coping(
    view: &GrindView,
    inv: &Investigation,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    if !(inv.gravity_relief_timer > 0.0) {
        return Ok(None);
    }
    let toward = num.dot(inv.geometry.upmost_normal, view.velocity)?;
    if !(0.0 > toward) {
        return Ok(None);
    }
    Ok(Some([0.0, cvars.f(C::kCopingSupportForce)?, 0.0, 0.0]))
}

/// `ApplyPinForce`: while the relief timer runs, the grind normal times
/// `-kPinForce` with its Y lane replaced by zero.
pub(crate) fn pin(
    inv: &Investigation,
    frame: &Frame,
    cvars: &GrindCvars<'_>,
) -> Result<Option<V4>, GrindError> {
    if !(inv.gravity_relief_timer > 0.0) {
        return Ok(None);
    }
    let pin = neg1(cvars.f(C::kPinForce)?);
    Ok(Some(with_y(scale(frame.normal, pin), 0.0)))
}

/// `ApplyTruckControlCompensationForce`.
pub(crate) fn truck_compensation(
    inv: &Investigation,
    frame: &Frame,
    cvars: &GrindCvars<'_>,
) -> Result<V4, GrindError> {
    let magnitude = abs1(inv.truck_control_ud) * cvars.f(C::kTruckControlCompensationForce)?;
    Ok(scale(frame.normal, neg1(magnitude)))
}

/// `ApplyTranslationControl(strength)`.
pub(crate) fn translation_control(view: &GrindView, frame: &Frame, strength: f32) -> V4 {
    scale(frame.skew, view.translation * strength)
}

/// `ApplyExitNudge(strength, speed, lift, addVertical, useDeckSide)`.
#[expect(clippy::too_many_arguments, reason = "native argument list")]
pub(crate) fn exit_nudge(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    strength: f32,
    speed: f32,
    lift: f32,
    add_vertical: bool,
    use_deck_side: bool,
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    let dir = if !use_deck_side && inv.geometry.geometry_type != THIN_RAIL {
        neg(inv.geometry.high_side_dir)
    } else {
        let side = cross(frame.dir, frame.normal);
        let offset = sub(view.deck_t(), inv.primitive_location);
        if num.dot(side, offset)? > 0.0 {
            side
        } else {
            neg(side)
        }
    };
    // BGE skips on ordered >=; an unordered speed still nudges.
    if num.dot(view.velocity, dir)? >= speed {
        return Ok(None);
    }
    let mut force = scale(dir, strength);
    if add_vertical {
        let up = num.dot(frame.normal, J)?;
        let vertical = fsel(neg1(up), 0.0, up) * lift;
        force = madd(frame.normal, vertical, force);
    }
    Ok(Some(force))
}

/// `ApplyAlignmentForce(strength, deckDirDist, deckDownDist)`: pull the deck
/// point toward the primitive across the skew, damped by the velocity along
/// the pull, scaled by `GrabVsSlope` of the normal's slope without relief.
#[expect(clippy::too_many_arguments, reason = "native argument list")]
pub(crate) fn alignment(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    strength: f32,
    dir_dist: f32,
    down_dist: f32,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    let dir = if inv.grind_on_front {
        view.deck_z()
    } else {
        neg(view.deck_z())
    };
    let point = sub(
        madd(dir, dir_dist, view.deck_t()),
        scale(view.deck_y(), down_dist),
    );
    let across = num.dot(sub(inv.primitive_location, point), frame.skew)?;
    let force = scale(frame.skew, across * strength);
    let length = num.length(force)?;
    if !(length > 0.0) {
        return Ok(None);
    }
    let damping = cvars.f(C::kAlignmentForceDamping)? * strength;
    let unit = scale(force, num.reciprocal(length)?);
    let closing = damping * num.dot(view.velocity, unit)?;
    let damped = scale(unit, closing);
    let scalar = if inv.gravity_relief_timer > 0.0 {
        1.0
    } else {
        let slope = num.dot(frame.normal, inv.geometry.upmost_normal)?;
        attributes.graph(GrindGraph::GrabVsSlope, slope)?
    };
    Ok(Some(scale(sub(force, damped), scalar)))
}

/// `AssistExitLean`: against the ledge's high side by `ExitAssistVsLeanAngle`.
pub(crate) fn assist_exit_lean(
    inv: &Investigation,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
) -> Result<Option<V4>, GrindError> {
    let tilt = inv.exit_tilt_angle_deg;
    if !(tilt > 0.0) {
        return Ok(None);
    }
    let graph = attributes.graph(GrindGraph::ExitAssistVsLeanAngle, tilt)?;
    let magnitude = cvars.f(C::kExitAssistForce)? * graph;
    Ok(Some(scale(inv.geometry.high_side_dir, neg1(magnitude))))
}

/// One ordered effect of Boardslide `ApplyTranslation`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum TranslationStep {
    Force(V4),
    /// `ApplyTranslationControl(kBoardslideTranslationForce)`.
    TranslationControl,
    /// Inline `SingletonHolder<DebugOverlay>` lookup.
    Overlay,
}

/// Boardslide `ApplyTranslation` after the caller's 8c4 0x200000 latch of
/// `preparing`: the prepare-jump push, or the ledge translation, or the
/// truck-avoid push with its perpendicular damping, or the translation
/// control with the velocity damping.
pub(crate) fn boardslide_translation(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    preparing: bool,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<Vec<TranslationStep>, GrindError> {
    use TranslationStep as S;
    let mut steps = Vec::new();
    let offset = num.dot(sub(view.deck_t(), inv.primitive_location), frame.skew)?;
    let distance = abs1(offset);
    let away = |offset: f32| {
        if offset > 0.0 {
            neg(frame.skew)
        } else {
            frame.skew
        }
    };
    if preparing {
        if distance > cvars.f(C::kPrepareJumpClearance)? {
            steps.push(S::Force(scale(
                away(offset),
                cvars.f(C::kPrepareJumpForce)?,
            )));
            steps.push(S::Overlay);
        }
        return Ok(steps);
    }
    let show = cvars.b(C::kShowTruckAvoid)?;
    if show {
        steps.push(S::Overlay);
    }
    let along = num.dot(view.velocity, frame.dir)?;
    let perpendicular = sub(view.velocity, scale(frame.dir, along));
    if inv.geometry.geometry_type == LEDGE {
        steps.push(S::TranslationControl);
        return Ok(steps);
    }
    if distance > cvars.f(C::kTruckClearance)? {
        let dir = away(offset);
        steps.push(S::Force(scale(dir, cvars.f(C::kTruckAvoidForce)?)));
        if 0.0 > num.dot(dir, perpendicular)? {
            // kfUpdateHz 60.0 times the Processed skateboard mass.
            let damping = view.mass * f32::from_bits(0x4270_0000);
            steps.push(S::Force(with_y(scale(neg(perpendicular), damping), 0.0)));
        }
        if show {
            steps.push(S::Overlay);
        }
        return Ok(steps);
    }
    steps.push(S::TranslationControl);
    steps.push(S::Force(with_y(
        scale(neg(perpendicular), cvars.f(C::kVelDampingScalar)?),
        0.0,
    )));
    Ok(steps)
}

/// Tipslide `ApplyStabilityNudge`.
pub(crate) fn stability_nudge(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let offset = sub(view.deck_t(), inv.primitive_location);
    let dir = if num.dot(frame.skew, offset)? > 0.0 {
        neg(frame.skew)
    } else {
        frame.skew
    };
    if inv.grab_tipslide {
        return Ok(scale(dir, cvars.f(C::kTipslideGrabNudgeForce)?));
    }
    let side = cross(frame.dir, offset);
    let nudge = if num.dot(frame.normal, side)? > 0.0 {
        view.nudge
    } else {
        neg1(view.nudge)
    };
    Ok(scale(
        dir,
        cvars.f(C::kTipslideStabilityNudgeForce)? * nudge,
    ))
}
