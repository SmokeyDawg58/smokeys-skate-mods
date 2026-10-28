//! `SkaterPresentation::DoSimRestore`827e7298..73e0 render opacity for the
//! selected `SkaterPresEntity`: the post-blend opacity byte and flags that
//! `UpdateSkater`827da1d0..1f0 copies from the restored frame header,
//! `EvaluateProximityOpacity`827d3458, the `rendering_skaters` rate-limited
//! approach, then +0x61a `mIsGhosted` and +0x870 `i_params[0].x`.
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance, AttributeRuntimeVault},
    reckoning_numeric::{
        HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric,
        length_from_squared_and_estimate,
    },
    simulation_drive_jacobian::refine_drive_reciprocal,
};
use std::sync::Arc;

/// `Attrib::Gen::rendering` class key from `rendering::rendering`825821f8.
const RENDERING_CLASS: u64 = 0x1ffd_c8e3_aca0_7c1f;
/// GlobalAttributeCollection ctor828b3534 collection bound at +0x198.
const SKATERS_COLLECTION: u64 = 0xda92_275f_3d73_5e7d;
/// 0x3b808081 converts the post-blend opacity byte (827e72f8).
const BYTE_TO_UNIT: f32 = f32::from_bits(0x3b80_8081);
/// `Math::VECFLOAT_SMALL` used by `CalcClosestPointOnLine`82e48fe0.
const SMALL: f32 = f32::from_bits(0x3780_0000);

/// Header words of the restored frame, as `UpdateSkater` publishes them into
/// the post-blend flags, plus the two restored matrices DoSimRestore reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RenderOpacityInputs {
    /// Header word 0xbc bits 0xff00, the published `SetOpacity` value * 255.
    pub(crate) opacity_byte: u8,
    /// Header word 0xc0 bit 0x4000 (post-blend 0x02000000).
    pub(crate) collidable: bool,
    /// Header word 0xc0 bit 0x2000 (post-blend 0x01000000).
    pub(crate) main_camera_only: bool,
    /// `m_MatLtoW` (root * saved Hips) translation: the skater hips position.
    pub(crate) hips: [f32; 3],
    /// DataPostBlend root (+16) Y axis, record+0x40.
    pub(crate) up: [f32; 3],
    /// `Presentation::IsInReplay` 0x827eed90: the time controller's `IsInReplay`, true for
    /// a replay-controlled scene's `SceneReplayTimeController` (0x826c6d80) and false for
    /// the `RealTimeController` (0x82784530).
    pub(crate) in_replay: bool,
}

/// `SkaterPresEntity`+0x870 lane x and +0x61a. The constructor827e5948..5988
/// stores `i_params[0]` = (1, 0, 1, 0) and clears `mIsGhosted`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkaterRenderOpacity {
    pub(crate) opacity: f32,
    pub(crate) ghosted: bool,
}
impl SkaterRenderOpacity {
    pub(crate) const fn constructed() -> Self {
        Self {
            opacity: 1.0,
            ghosted: false,
        }
    }
}

#[derive(Debug)]
pub(crate) enum RenderOpacityError {
    /// No restored frame published its header and matrices this frame.
    MissingInputs,
    Attribute(AttributeRuntimeError),
    Numeric(ReckoningHostNumericError),
    LayoutField(usize),
}

/// GlobalAttributeCollection+0x198 `rendering_skaters`, constructed once.
pub(crate) fn rendering_skaters(vault: &Arc<AttributeRuntimeVault>) -> AttributeRuntimeInstance {
    vault.instance(RENDERING_CLASS, SKATERS_COLLECTION)
}

/// One `mLayoutPtr` float; a missing layout is `DefaultDataArea(0x78)` (zero).
fn layout(instance: &AttributeRuntimeInstance, offset: usize) -> Result<f32, RenderOpacityError> {
    match instance
        .try_cached_layout_field(offset, 4)
        .map_err(RenderOpacityError::Attribute)?
    {
        None => Ok(0.0),
        Some(data) => Ok(f32::from_be_bytes(
            data.bytes()
                .try_into()
                .map_err(|_| RenderOpacityError::LayoutField(offset))?,
        )),
    }
}

fn fsel(test: f32, nonnegative: f32, negative: f32) -> f32 {
    if test >= 0.0 { nonnegative } else { negative }
}

/// `vminfp`/`vmaxfp` propagate NaN; ordinary values compare as usual. Signed
/// zero tie selection is not claimed.
fn vmin(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else if a < b {
        a
    } else {
        b
    }
}
fn vmax(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
fn lanes(v: [f32; 3]) -> [f32; 4] {
    [v[0], v[1], v[2], 0.0]
}

/// `PhysicsUtility::CalcClosestPointOnLine`82e48fb8 (raw VX128 operands).
fn closest_point_on_line(
    position: [f32; 3],
    start: [f32; 3],
    end: [f32; 3],
    numeric: &mut HostReckoningNumeric,
) -> Result<[f32; 3], ReckoningHostNumericError> {
    let mut direction = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let squared = numeric.dot3(lanes(direction), lanes(direction))?;
    let estimate = numeric.reciprocal_sqrt_estimate(squared)?;
    // Two fused rsqrte refinements, squared * inverse, zero when squared == 0.
    let length = length_from_squared_and_estimate(squared, estimate);
    // vcmpgtfp128. every lane: only an ordered length above SMALL normalizes.
    if length > SMALL {
        let seed = numeric.reciprocal_estimate(length)?;
        let inverse = refine_drive_reciprocal(length, seed);
        direction = direction.map(|lane| inverse * lane);
    }
    let offset = [
        position[0] - start[0],
        position[1] - start[1],
        position[2] - start[2],
    ];
    let projection = numeric.dot3(lanes(direction), lanes(offset))?;
    let t = vmax(vmin(projection, length), 0.0);
    Ok([
        direction[0].mul_add(t, start[0]),
        direction[1].mul_add(t, start[1]),
        direction[2].mul_add(t, start[2]),
    ])
}

/// `SkaterPresentation::EvaluateProximityOpacity`827d3458.
fn proximity_opacity(
    rendering: &AttributeRuntimeInstance,
    hips: [f32; 3],
    up: [f32; 3],
    camera: [f32; 3],
    numeric: &mut HostReckoningNumeric,
) -> Result<f32, RenderOpacityError> {
    let near = layout(rendering, 0x3c)?;
    let near_squared = near * near;
    let far = layout(rendering, 0x44)?;
    let half_length = layout(rendering, 0x48)?;
    let near_opacity = layout(rendering, 0x38)?;
    let far_opacity = layout(rendering, 0x40)?;
    let inverse = 1.0 / far.mul_add(far, -near_squared);
    // p1 = hips - up * L (vmulfp128, vsubfp128); p2 = fma(up, L, hips).
    let start = [
        hips[0] - up[0] * half_length,
        hips[1] - up[1] * half_length,
        hips[2] - up[2] * half_length,
    ];
    let end = [
        up[0].mul_add(half_length, hips[0]),
        up[1].mul_add(half_length, hips[1]),
        up[2].mul_add(half_length, hips[2]),
    ];
    let closest =
        closest_point_on_line(camera, start, end, numeric).map_err(RenderOpacityError::Numeric)?;
    let offset = [
        closest[0] - camera[0],
        closest[1] - camera[1],
        closest[2] - camera[2],
    ];
    let squared = numeric
        .dot3(lanes(offset), lanes(offset))
        .map_err(RenderOpacityError::Numeric)?;
    let t = (squared - near_squared) * inverse;
    let t = fsel(-t, 0.0, t);
    let t = fsel(1.0 - t, t, 1.0);
    Ok(t.mul_add(far_opacity - near_opacity, near_opacity))
}

/// 827e7350..73ac: `current` approaches `proximity`, falling at `down` and rising at `up` a second
/// over `elapsed` (the rise is scaled before the ordered compare), is held in 0..1 and capped by
/// `target`.
fn approach(current: f32, proximity: f32, target: f32, down: f32, up: f32, elapsed: f32) -> f32 {
    let raise = up * elapsed;
    let approached = if current > proximity {
        let lowered = (-down).mul_add(elapsed, current);
        fsel(proximity - lowered, proximity, lowered)
    } else {
        let raised = current + raise;
        fsel(proximity - raised, raised, proximity)
    };
    let low = fsel(-approached, 0.0, approached);
    let clamped = fsel(1.0 - low, low, 1.0);
    fsel(target - clamped, clamped, target)
}

/// DoSimRestore827e7298..73e0 for one retained `SkaterPresEntity`. The original restores once a
/// presented frame, at most 60 a second, and moves the opacity by each rate times `kUpdateTime`
/// (827e7354): `elapsed` is the presentation time this restore stands for, `kUpdateTime` for one
/// such frame, so a restore drawn more often fades at the original's speed.
pub(crate) fn try_restore(
    state: &mut SkaterRenderOpacity,
    inputs: RenderOpacityInputs,
    camera: [f32; 3],
    rendering: &AttributeRuntimeInstance,
    numeric: &mut HostReckoningNumeric,
    elapsed: f32,
) -> Result<(), RenderOpacityError> {
    // 827e7298..72f8: the collidable bit selects the header opacity, unless a
    // replay shows a main-camera-only fade opaque; otherwise the ghost half.
    let target = if inputs.collidable {
        if inputs.main_camera_only && inputs.in_replay {
            1.0
        } else {
            f32::from(inputs.opacity_byte) * BYTE_TO_UNIT
        }
    } else if inputs.in_replay {
        1.0
    } else {
        0.5
    };
    let proximity = proximity_opacity(rendering, inputs.hips, inputs.up, camera, numeric)?;
    // rendering_skaters fades down at [19] and up at [20].
    let (down, up) = (layout(rendering, 0x4c)?, layout(rendering, 0x50)?);
    let result = approach(state.opacity, proximity, target, down, up, elapsed);
    // 827e73b0..73e0: blt 1.0 selects the ghost technique; lane x is replaced.
    state.ghosted = result < 1.0;
    state.opacity = result;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closest_point_clamps_to_the_segment_and_keeps_a_degenerate_start() {
        let mut numeric = HostReckoningNumeric;
        let hit = closest_point_on_line(
            [0.0, 5.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            &mut numeric,
        )
        .unwrap();
        assert!((hit[1] - 2.0).abs() < 1.0e-5 && hit[0] == 0.0 && hit[2] == 0.0);
        let below = closest_point_on_line(
            [0.0, -4.0, 1.0],
            [0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            &mut numeric,
        )
        .unwrap();
        assert_eq!(below, [0.0, 0.0, 0.0]);
        let point = closest_point_on_line(
            [3.0, 3.0, 3.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            &mut numeric,
        )
        .unwrap();
        assert_eq!(point, [1.0, 1.0, 1.0]);
    }
    #[test]
    fn a_fade_drawn_at_any_rate_keeps_the_original_speed() {
        // rendering_skaters: down 5 and up 15 a second, toward a proximity of 0.15.
        let frame = crate::simulation_sim_rate::UPDATE_TIME;
        let mut sixty = 1.0;
        for _ in 0..6 {
            sixty = approach(sixty, 0.15, 1.0, 5.0, 15.0, frame);
        }
        let mut faster = 1.0;
        for _ in 0..16 {
            faster = approach(faster, 0.15, 1.0, 5.0, 15.0, 6.0 * frame / 16.0);
        }
        assert!((sixty - 0.5).abs() < 1e-5 && (faster - sixty).abs() < 1e-5, "{sixty} {faster}");
        // Rising back is capped by the target the camera publishes.
        assert!((approach(0.5, 1.0, 0.6, 5.0, 15.0, frame) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn fsel_clamps_and_the_target_caps_the_approach() {
        assert_eq!(fsel(-0.25, 0.0, 0.25), 0.25);
        assert_eq!(fsel(0.0, 0.0, 0.25), 0.0);
        assert!(fsel(f32::NAN, 1.0, 2.0) == 2.0);
        assert!(vmin(f32::NAN, 1.0).is_nan() && vmax(1.0, f32::NAN).is_nan());
        assert_eq!(vmin(0.5, 1.0), 0.5);
        assert_eq!(vmax(-0.5, 0.0), 0.0);
    }
}
