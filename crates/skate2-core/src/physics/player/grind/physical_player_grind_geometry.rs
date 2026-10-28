//! `GrindGeometryAnalyzer`: `CalcGrindGeometry` 0x82cfc2d0 with
//! `DoGeometryTest` 0x82cfac60, `IsStair` 0x82cf93d0, `IsCurb` 0x82cf9258 and
//! `ResolveSurfaceInfo` 0x82cfb380; `GrindPrediction::CalcTiltedUpmostNormal`
//! 0x82cf9440.
use super::{
    math::{
        DEG_TO_RAD, GrindNum, HALF_PI, I, K, PI, TWO_PI, V4, abs1, add, clamp_unit, cross, madd,
        neg, scale, sub,
    },
    types::{FAT_RAIL, Geometry, GrindPrim, IMPOSSIBLE, LEDGE, THIN_RAIL},
    view::{DECK_CENTER_TO_TRUCK, GrindAttributes, GrindCvars, GrindError},
};
use crate::input_cvar_runtime::grind_states::GrindCvar as C;

/// One `DoGeometryTest` line result (`CollResultsArray` entry, 0x40 bytes).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LineHit {
    pub(crate) hit: bool,
    pub(crate) fraction: f32,
    pub(crate) position: V4,
    pub(crate) normal: V4,
    pub(crate) surface: u32,
}

/// The five test lines, top to bottom: close1, close2, far1, far2, centre.
pub(crate) type GeometryLines = [(V4, V4); 5];

/// The world line batch of `DoGeometryTest` ("GrindGeometryAnalyzer::DoGeometryTest",
/// flags 7, fatness zero): all five lines added, then each result read.
pub(crate) trait GeometryLineBatch {
    fn try_test(&mut self, lines: &GeometryLines) -> Result<[Option<LineHit>; 5], GrindError>;
}

/// `CalcGrindGeometry` 0x82cfc2d0 for the investigated primitive and location.
#[expect(clippy::too_many_lines, reason = "one native classification sequence")]
pub(crate) fn calc_grind_geometry(
    prim: &GrindPrim,
    grind_position: V4,
    attributes: &GrindAttributes,
    cvars: &GrindCvars<'_>,
    batch: &mut impl GeometryLineBatch,
    num: &mut GrindNum<'_>,
) -> Result<Geometry, GrindError> {
    let mut out = Geometry::default();
    let direction = sub(prim.end, prim.start);
    let length = num.length(direction)?;
    let primitive_dir = scale(direction, num.reciprocal(length)?);
    out.primitive_dir = primitive_dir;
    let side_raw = cross(super::math::J, direction);
    let side = num.normalize_unsafe(side_raw)?;
    let up = cross(direction, side_raw);
    let up_length = num.length(up)?;
    let offset = attributes.scalar(DECK_CENTER_TO_TRUCK)?;
    if !(up_length > f32::from_bits(0x3727_c5ac)) {
        out.uncertain = true;
        out.hi_normal_limit_angle = 0.0;
        out.lo_normal_limit_angle = 0.0;
        let across = scale(I, offset);
        out.line1_hit = add(grind_position, across);
        out.line2_hit = sub(grind_position, across);
        out.upmost_normal = K;
        out.audio_surface = 3;
        out.physics_surface = 1;
        return Ok(out);
    }
    let along = num.dot(sub(grind_position, prim.start), primitive_dir)?;
    let projected = madd(primitive_dir, along, prim.start);
    let position = sub(grind_position, sub(grind_position, projected));
    let upmost = scale(up, num.reciprocal(up_length)?);
    out.upmost_normal = upmost;
    let close = cvars.f(C::kFineGeometryDist)?;
    let close_length = cvars.f(C::kFineGeometryDepth)?;
    let far_length = cvars.f(C::kTestHeightHeuristic)? * offset;
    let (results, far1_low, far2_low) = do_geometry_test(
        close,
        close_length,
        offset,
        far_length,
        position,
        upmost,
        primitive_dir,
        batch,
    )?;
    let coarse = |entry: &LineHit| -> Result<i32, GrindError> {
        Ok(
            if entry.hit && entry.fraction < cvars.f(C::kCoarseCurbParam)? {
                LEDGE
            } else {
                FAT_RAIL
            },
        )
    };
    if results[0].hit {
        if results[1].hit {
            out.geometry_type = IMPOSSIBLE;
            out.high_side_dir = side;
            out.is_apex = true;
        } else {
            out.geometry_type = coarse(&results[2])?;
            out.high_side_dir = side;
        }
    } else if results[1].hit {
        out.geometry_type = coarse(&results[3])?;
        out.high_side_dir = neg(side);
    } else {
        out.geometry_type = THIN_RAIL;
        out.high_side_dir = side;
    }
    out.is_stair = is_stair(&results);
    out.is_curb = is_curb(&results, upmost, position, out.geometry_type, cvars, num)?;
    let (audio, physics) = resolve_surface_info(&results, out.geometry_type, out.is_curb);
    out.audio_surface = audio;
    out.physics_surface = physics;
    out.line1_hit = far1_low;
    out.line2_hit = far2_low;
    let first = sub(far1_low, position);
    let second = sub(far2_low, position);
    let first_length = num.length(first)?;
    let second_length = num.length(second)?;
    // Debug assertion 0x82cfc7b0 (both lengths positive); shipped build continues.
    let clearance = cvars.f(C::kWheelClearAngleDeg)? * DEG_TO_RAD;
    let down = neg(upmost);
    let first_dir = scale(first, num.reciprocal(first_length)?);
    let first_dot = num.dot(down, first_dir)?;
    let first_angle = num.acos(clamp_unit(first_dot))?;
    out.lo_normal_limit_angle = (first_angle + HALF_PI) + clearance;
    let second_dir = scale(second, num.reciprocal(second_length)?);
    let second_dot = num.dot(down, second_dir)?;
    let second_angle = num.acos(clamp_unit(second_dot))?;
    out.hi_normal_limit_angle = ((TWO_PI - second_angle) - HALF_PI) - clearance;
    out.uncertain = false;
    Ok(out)
}

/// `DoGeometryTest` 0x82cfac60: two close lines either side, two far lines,
/// and the centre line, each from `normal * length` above to below.
#[expect(clippy::too_many_arguments, reason = "native argument list")]
fn do_geometry_test(
    close: f32,
    close_length: f32,
    far: f32,
    far_length: f32,
    position: V4,
    normal: V4,
    grind_dir: V4,
    batch: &mut impl GeometryLineBatch,
) -> Result<([LineHit; 5], V4, V4), GrindError> {
    let side = cross(normal, grind_dir);
    let close_side = scale(side, close);
    let close_up = scale(normal, close_length);
    let far_side = scale(side, far);
    let far_up = scale(normal, far_length);
    let close1 = add(position, close_side);
    let close2 = sub(position, close_side);
    let centre_top = add(position, close_up);
    let far1 = add(position, far_side);
    let far2 = sub(position, far_side);
    let centre_bottom = sub(position, close_up);
    let lines: GeometryLines = [
        (add(close1, close_up), sub(close1, close_up)),
        (add(close2, close_up), sub(close2, close_up)),
        (add(far1, far_up), sub(far1, far_up)),
        (add(far2, far_up), sub(far2, far_up)),
        (centre_top, centre_bottom),
    ];
    let hits = batch.try_test(&lines)?;
    let results: [LineHit; 5] = std::array::from_fn(|i| hits[i].unwrap_or_default());
    let far1_low = if results[2].hit {
        results[2].position
    } else {
        lines[2].1
    };
    let far2_low = if results[3].hit {
        results[3].position
    } else {
        lines[3].1
    };
    Ok((results, far1_low, far2_low))
}

/// `IsStair` 0x82cf93d0: either close line landed on a stair surface.
fn is_stair(results: &[LineHit; 5]) -> bool {
    let stair = |entry: &LineHit| entry.hit && entry.surface & 0xf80 == 0x400;
    stair(&results[0]) || stair(&results[1])
}

/// `IsCurb` 0x82cf9258: a ledge whose two far hits are both near-level and
/// within the curb height windows.
fn is_curb(
    results: &[LineHit; 5],
    upmost: V4,
    position: V4,
    geometry_type: i32,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<bool, GrindError> {
    if geometry_type != LEDGE || !results[2].hit || !results[3].hit {
        return Ok(false);
    }
    let likeness = cvars.f(C::kCurbNormalLikeness)?;
    if !(num.dot(upmost, results[2].normal)? > likeness) {
        return Ok(false);
    }
    if !(num.dot(upmost, results[3].normal)? > likeness) {
        return Ok(false);
    }
    let first = num.dot(sub(results[2].position, position), upmost)?;
    let second = num.dot(sub(results[3].position, position), upmost)?;
    let (low, high) = if first < second {
        (first, second)
    } else {
        (second, first)
    };
    let range = cvars.f(C::kCurbHighSideRange)?;
    if !(high > super::math::neg1(range)) || !(high < range) {
        return Ok(false);
    }
    let average = cvars.f(C::kAvgCurbHeight)?;
    let height_range = cvars.f(C::kCurbHeightRange)?;
    if !(low > super::math::neg1(average) - height_range) {
        return Ok(false);
    }
    Ok(low < height_range - average)
}

fn decode_surface(surface: u32) -> (u32, u32) {
    let low = surface & 0xffff;
    (low & 0x7f, (low >> 7) & 0x1f)
}

/// `ResolveSurfaceInfo` 0x82cfb380: (audio, physics) surface.
fn resolve_surface_info(results: &[LineHit; 5], geometry_type: i32, is_curb: bool) -> (u32, u32) {
    if is_curb {
        return (4, 2);
    }
    if geometry_type == THIN_RAIL {
        let audio = if results[4].hit {
            results[4].surface & 0x7f
        } else {
            0xb
        };
        return (audio, 4);
    }
    let higher = |a: usize, b: usize| {
        if results[a].position[1] > results[b].position[1] {
            a
        } else {
            b
        }
    };
    let chosen = if results[0].hit {
        if results[1].hit { higher(0, 1) } else { 0 }
    } else if results[1].hit {
        1
    } else if results[2].hit {
        if results[3].hit { higher(2, 3) } else { 2 }
    } else if results[3].hit {
        3
    } else if results[4].hit {
        4
    } else {
        return (3, 1);
    };
    decode_surface(results[chosen].surface)
}

/// `GrindPrediction::CalcTiltedUpmostNormal` 0x82cf9440: the upmost normal
/// tilted toward the free side of the geometry's clearance window.
pub(crate) fn calc_tilted_upmost_normal(
    geometry: &Geometry,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let lo = geometry.lo_normal_limit_angle;
    let hi = geometry.hi_normal_limit_angle;
    let down = neg(geometry.upmost_normal);
    if !(lo < hi) {
        let angle = (hi + lo) * 0.5;
        return num.rotate(geometry.primitive_dir, angle, down);
    }
    if lo < PI && hi > PI {
        return Ok(geometry.upmost_normal);
    }
    let lo_gap = abs1(PI - lo);
    let hi_gap = abs1(PI - hi);
    let angle = if lo_gap < hi_gap { lo } else { hi };
    num.rotate(geometry.primitive_dir, angle, down)
}
