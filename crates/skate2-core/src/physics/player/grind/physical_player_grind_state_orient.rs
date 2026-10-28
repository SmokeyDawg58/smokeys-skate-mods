//! Grind state board orientation targets and the `ReorientSkateboard`
//! arithmetic: `CalcOrientationControlResult` 0x82d859e0, the per-type
//! `OrientBoard` targets (5050 0x82d9b918, Tipslide 0x82d9bac0, 5_O
//! 0x82d9bcf8, Backslash 0x82d9bd88, Boardslide `OrientBoardToNormal`
//! 0x82d9b818), the Gram-Schmidt of `ReorientSkateboard` 0x82d9b580 and the
//! Euler noise of `ApplyNoise` 0x82d8fe78. Matrices are the native rows
//! (X, Y, Z, T), which the port's `SkeletonRootMatrix` calls columns.
use super::forces::Frame;
use crate::{
    input_cvar_runtime::grind_states::GrindCvar as C,
    physical_player_grind_manager::{
        math::{DEG_TO_RAD, GrindNum, V4, ZERO, abs1, cross, fsel, neg, neg1, scale, sub},
        types::Investigation,
        view::{GrindCvars, GrindError, GrindView},
    },
};

pub(crate) type Rows = [V4; 4];

/// `CalcOrientationControlResult(from, heading, headingRate, pitch,
/// pitchRate)`: Z rotated about the grind normal by `-heading*rate`, X the
/// normalised normal-cross-Z (a near-zero X returns `from` unchanged), Z then
/// pitched about X. The result's T row is never initialised natively and no
/// caller reads it.
#[expect(clippy::too_many_arguments, reason = "native argument list")]
pub(crate) fn orientation_control(
    from: &Rows,
    normal: V4,
    heading: f32,
    heading_rate: f32,
    pitch: f32,
    pitch_rate: f32,
    pitch_flipped: bool,
    num: &mut GrindNum<'_>,
) -> Result<Rows, GrindError> {
    let turn = neg1(heading * heading_rate);
    let z = if abs1(turn) > 0.0 {
        num.rotate(normal, turn, from[2])?
    } else {
        from[2]
    };
    let raw = cross(normal, z);
    let length = num.length(raw)?;
    if !(length > f32::from_bits(0x3a83_126f)) {
        return Ok(*from);
    }
    let x = scale(raw, num.reciprocal(length)?);
    let signed = if pitch_flipped { neg1(pitch) } else { pitch };
    let lift = signed * pitch_rate;
    if abs1(lift) > 0.0 {
        let z = num.rotate(x, lift, z)?;
        return Ok([x, cross(z, x), z, ZERO]);
    }
    Ok([x, cross(z, x), z, ZERO])
}

/// 5050 `OrientBoard` without truck control: the grind direction facing the
/// deck's Z, the grind normal as Y.
pub(crate) fn target_5050(
    view: &GrindView,
    frame: &Frame,
    num: &mut GrindNum<'_>,
) -> Result<Rows, GrindError> {
    let z = if num.dot(frame.dir, view.deck_z())? > 0.0 {
        frame.dir
    } else {
        neg(frame.dir)
    };
    Ok([cross(frame.normal, z), frame.normal, z, view.deck_t()])
}

/// Tipslide and Backslash `OrientBoard`: the normal tilted about the grind
/// direction away from the primitive by the state's tilt, Z along the grind
/// direction facing the deck's X.
pub(crate) fn target_tilted(
    view: &GrindView,
    inv: &Investigation,
    frame: &Frame,
    tilt_deg: f32,
    num: &mut GrindNum<'_>,
) -> Result<Rows, GrindError> {
    let side = cross(frame.dir, frame.normal);
    let offset = sub(view.deck_t(), inv.primitive_location);
    let axis = if num.dot(side, offset)? > 0.0 {
        neg(frame.dir)
    } else {
        frame.dir
    };
    let facing = num.dot(frame.dir, view.deck_x())?;
    let normal = num.rotate(axis, tilt_deg * DEG_TO_RAD, frame.normal)?;
    let z = if facing > 0.0 {
        frame.dir
    } else {
        neg(frame.dir)
    };
    Ok([z, normal, cross(z, normal), view.deck_t()])
}

/// Boardslide `OrientBoardToNormal`: X along the grind direction facing the
/// deck's X, the grind normal as Y.
pub(crate) fn target_boardslide(
    view: &GrindView,
    frame: &Frame,
    num: &mut GrindNum<'_>,
) -> Result<Rows, GrindError> {
    let x = if num.dot(frame.dir, view.deck_x())? > 0.0 {
        frame.dir
    } else {
        neg(frame.dir)
    };
    Ok([x, frame.normal, cross(x, frame.normal), view.deck_t()])
}

/// The classical Gram-Schmidt of `ReorientSkateboard` (Z, then Y against Z,
/// then X against Z and Y, each dot against the ORIGINAL row), every row
/// normalised by the unguarded refined reciprocal square root; T is zero.
pub(crate) fn orthonormalize(rows: &Rows, num: &mut GrindNum<'_>) -> Result<Rows, GrindError> {
    let z = num.normalize_unsafe(rows[2])?;
    let z_dot_y = num.dot(z, rows[1])?;
    let y = num.normalize_unsafe(sub(rows[1], scale(z, z_dot_y)))?;
    let x_first = sub(rows[0], scale(z, num.dot(z, rows[0])?));
    let y_dot_x = num.dot(y, rows[0])?;
    let x = num.normalize_unsafe(sub(x_first, scale(y, y_dot_x)))?;
    Ok([x, y, z, ZERO])
}

/// `ApplyNoise(transform, amount)` after its three `GetRandomInt` draws: each
/// draw's `% 100000` scaled into `[-A, A]` with `A` the speed-scaled noise,
/// the Euler rotation of the three angles, then that rotation times
/// `transform` with the translation carried.
pub(crate) fn noise(
    transform: &Rows,
    draws: [u32; 3],
    deck_speed: f32,
    amount: f32,
    cvars: &GrindCvars<'_>,
    num: &mut GrindNum<'_>,
) -> Result<Rows, GrindError> {
    let ratio = deck_speed / cvars.f(C::kNoiseFullSpeed)?;
    let clamped = fsel(1.0 - ratio, ratio, 1.0);
    let amplitude = (clamped * cvars.f(C::kNoiseScalar)?) * amount;
    let twice = amplitude * 2.0;
    let unit = f32::from_bits(0x3727_c5ac);
    // Each remainder is below 2^24: the fcfid/frsp conversion is exact.
    #[expect(clippy::cast_precision_loss, reason = "remainder below 100000")]
    let angle = |draw: u32| ((draw % 100_000) as f32 * unit).mul_add(twice, neg1(amplitude));
    let (s0, c0) = num.sin_cos(angle(draws[0]))?;
    let (s1, c1) = num.sin_cos(angle(draws[1]))?;
    let (s2, c2) = num.sin_cos(angle(draws[2]))?;
    let s0c2 = s0 * c2;
    let c0s2 = c0 * s2;
    let s0s2 = s0 * s2;
    let c0c2 = c0 * c2;
    let rotation = [
        [c1 * c2, c1 * s2, neg1(s1)],
        [s1 * s0c2 - c0s2, s1.mul_add(s0s2, c0c2), c1 * s0],
        [s1.mul_add(c0c2, s0s2), s1 * c0s2 - s0c2, c1 * c0],
    ];
    let [tx, ty, tz, t] = *transform;
    let row = |r: [f32; 3]| -> V4 {
        std::array::from_fn(|i| r[2].mul_add(tz[i], r[1].mul_add(ty[i], r[0] * tx[i])))
    };
    // The rotation's translation row is zero: three fused 0*row + T.
    let translation: V4 = std::array::from_fn(|i| {
        0.0_f32.mul_add(tz[i], 0.0_f32.mul_add(ty[i], 0.0_f32.mul_add(tx[i], t[i])))
    });
    Ok([
        row(rotation[0]),
        row(rotation[1]),
        row(rotation[2]),
        translation,
    ])
}
