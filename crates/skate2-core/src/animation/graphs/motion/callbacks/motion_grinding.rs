//! Grinding18 Motion behaviours: `CreateGrindAttributes` Update82ca1710,
//! `ControlGrindCrouch` Create82712148/Begin82cb1d10/Update82cc6e30 and
//! `GrindControlFade` Create82cbce08/Begin82cbcef0/Update82cc67f8. Every End is
//! the shared BLR 825187c0.
#[cfg(test)]
#[path = "motion_grinding_tests.rs"]
mod tests;
use crate::play_animation::{FastString30, FastString36};

/// `anim_motion` 0x3cce4d5548190472 at GlobalAttributeCollection+0x154.
pub(crate) const CROUCH_DRIVE_SPEED: u64 = 0x71cd_7678_af2a_7a5a;
pub(crate) const CROUCH_MAXIMUM: u64 = 0xa7c3_10ed_53f8_9af6;
pub(crate) const CROUCH_MINIMUM: u64 = 0x804b_5ece_c149_1fa2;
/// `anim_motion` 0x861925f635314abd at GlobalAttributeCollection+0x138.
pub(crate) const FADE_INTENT_RATE: u64 = 0x30ce_1d85_ce6b_2b42;
pub(crate) const FADE_BLEND: u64 = 0x68a7_3621_1512_65c0;
pub(crate) const FADE_DELTA_CHANGE: u64 = 0x3643_621c_cb9b_2578;
pub(crate) const FADE_DELTA_LIMIT: u64 = 0x8124_8d4d_1a96_309a;

/// `Sk8::GrindFacingForwardsAtt` / `GrindFacingBackwardsAtt` (82f82ae0/82f82af8).
const GRIND_FACING_FORWARDS: &str = "GrindFacingForwards";
const GRIND_FACING_BACKWARDS: &str = "GrindFacingBackwards";
/// `Sk8::Skater::CrouchIntent` (82f81b20).
const CROUCH_INTENT: &str = "Crouch";

/// ControlGrindCrouch Data (0x0c): Create zeroes +8, Begin seeds it with
/// `mCentreOfMassToDeck`, Update drives it toward the clamped crouch.
#[derive(Debug, Default)]
pub(crate) struct GrindCrouchInstance {
    pub(crate) distance: f32,
}

/// GrindControlFade Data (0x24). Create writes +8/+c/+10/+1c/+20; the adjusted
/// twist (+14) and the twist delta (+18) have no value until Begin.
#[derive(Debug)]
pub(crate) struct GrindControlFadeInstance {
    pub(crate) first_update: bool,
    pub(crate) time: f32,
    pub(crate) twist: f32,
    pub(crate) adjusted: Option<f32>,
    pub(crate) delta: Option<f32>,
    pub(crate) minimum: f32,
    pub(crate) maximum: f32,
}
impl GrindControlFadeInstance {
    pub(crate) fn after_allocation() -> Self {
        Self {
            first_update: true,
            time: 0.0,
            twist: 0.0,
            adjusted: None,
            delta: None,
            minimum: f32::from_bits(0xbfc9_0fdb),
            maximum: f32::from_bits(0x3fc9_0fdb),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Collection {
    GrindCrouch,
    ControlFade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GrindingCvar {
    UsePhysInducedGrindCrouch,
    ShowGrindCrouchText,
    ShowGrindControlFade,
}

/// Each callback's UCOM lookups, asserted together before the first use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Callback {
    /// IPhysical, ISkaterAnim, ISkaterMotionGraph.
    CreateGrindAttributesUpdate,
    /// IPhysical.
    GrindCrouchBegin,
    /// IPhysical, IAnimatable, IMotionGraph.
    GrindCrouchUpdate,
    /// IPhysical, IAnimatable, ISkaterAnim.
    ControlFadeBegin,
    /// IAnimatable, ISkaterAnim, IMotionGraph, ISkaterMotionGraph.
    ControlFadeUpdate,
}

pub(crate) struct ControlFadeNames {
    pub(crate) distance_board_to_cog: FastString30,
    pub(crate) twist: FastString30,
    pub(crate) twist_intent: FastString36,
}

pub(crate) trait GrindingSource {
    type Error;
    type Base;
    fn resolve(&mut self, callback: Callback) -> Result<(), Self::Error>;
    /// PhysOut Filtered+0x50.
    fn is_grinding(&mut self) -> Result<bool, Self::Error>;
    /// PhysOut Filtered+0x8.
    fn skating_grind_name(&mut self) -> Result<FastString30, Self::Error>;
    /// PhysOut Filtered+0x38.
    fn grind_crouch(&mut self) -> Result<f32, Self::Error>;
    /// PhysOut SkateboardMotion+0x111.
    fn skateboard_flipped(&mut self) -> Result<bool, Self::Error>;
    /// PhysOut SkateboardMotion+0x50.
    fn deck_velocity(&mut self) -> Result<[f32; 3], Self::Error>;
    /// PhysOut SkateboardReckoning+0x80.
    fn deck_nose_direction(&mut self) -> Result<[f32; 3], Self::Error>;
    /// PhysOut Animation+0x48.
    fn centre_of_mass_to_deck(&mut self) -> Result<f32, Self::Error>;
    /// PhysOut Skeleton+0x1f8.
    fn shoulder_twist(&mut self) -> Result<f32, Self::Error>;
    /// `SkaterAnim::GetIsMirrored`82cb7598 (ISkaterAnim +0x1c).
    fn mirrored(&mut self) -> Result<bool, Self::Error>;
    /// `SkaterMotionGraph::AddMgAttribute`825fe7a0 (ISkaterMotionGraph +0x10).
    fn motion_attribute(&mut self, name: FastString30, value: f32) -> Result<(), Self::Error>;
    /// `Intents::HasIntent` on IMotionGraph's intents; `None` leaves the 0.0 local.
    fn intent(&mut self, name: FastString36) -> Result<Option<f32>, Self::Error>;
    /// `GetAttributePointer(key, 0)`, `DefaultDataArea(4)` when absent.
    fn scalar(&mut self, collection: Collection, key: u64) -> Result<f32, Self::Error>;
    fn cvar(&mut self, cvar: GrindingCvar) -> Result<bool, Self::Error>;
    /// `HierarchicalController::GetDeltaTime`.
    fn dt(&mut self) -> Result<f32, Self::Error>;
    /// `Animatable::AddSettableAtt`82d7f5f0 (IAnimatable +0x48), sequence -1.
    fn settable(
        &mut self,
        name: FastString30,
        value: f32,
        normalized: bool,
    ) -> Result<(), Self::Error>;
    /// `SkaterAnim::SetAttributes` (IAnimatable +0x4c).
    fn set_attributes(&mut self) -> Result<(), Self::Error>;
    /// `Animatable::GetBaseAnim`82592b48 (IAnimatable +0x2c), `mBaseAnim` +0x4dbc.
    fn base_animation(&mut self) -> Result<Self::Base, Self::Error>;
    /// The base `Animation::GetAttribute` (vtable +0x34) with flags 0xf; the
    /// float at Attribute+0 when found.
    fn base_attribute(
        &mut self,
        base: &Self::Base,
        name: FastString30,
    ) -> Result<Option<f32>, Self::Error>;
    /// The `DebugOverlay` singleton read under a show cvar; its prints are 825187c0.
    fn debug_overlay(&mut self) -> Result<(), Self::Error>;
    fn assertion(&mut self, site: &'static str) -> Self::Error;
}

fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
/// `fsel(bound - value, bound, value)`.
fn lower(value: f32, bound: f32) -> f32 {
    select(bound - value, bound, value)
}
/// `fsel(bound - value, value, bound)`.
fn upper(value: f32, bound: f32) -> f32 {
    select(bound - value, value, bound)
}

pub(crate) fn create_grind_attributes_update<S: GrindingSource>(s: &mut S) -> Result<(), S::Error> {
    s.resolve(Callback::CreateGrindAttributesUpdate)?;
    if !s.is_grinding()? {
        return Ok(());
    }
    let name = s.skating_grind_name()?;
    s.motion_attribute(name, 1.0)?;
    let flipped = s.skateboard_flipped()?;
    let reversed = s.mirrored()? != flipped;
    let velocity = s.deck_velocity()?;
    let nose = s.deck_nose_direction()?;
    // vmulfp128 then vnmsubfp: y of cross(velocity, nose), the subtract fused.
    let product = velocity[2] * nose[0];
    let lateral = -(velocity[0].mul_add(nose[2], -product));
    // fcmpu/ble: an unordered y takes the not-positive branch.
    let backwards = if lateral > 0.0 { !reversed } else { reversed };
    let facing = if backwards {
        GRIND_FACING_BACKWARDS
    } else {
        GRIND_FACING_FORWARDS
    };
    s.motion_attribute(FastString30::encode(facing).expect("source literal"), 1.0)
}

pub(crate) fn grind_crouch_begin<S: GrindingSource>(
    s: &mut S,
    v: Option<&mut GrindCrouchInstance>,
) -> Result<(), S::Error> {
    s.resolve(Callback::GrindCrouchBegin)?;
    let distance = s.centre_of_mass_to_deck()?;
    // The store goes through the unchecked instance pointer.
    v.ok_or_else(|| s.assertion("ControlGrindCrouch instance"))?
        .distance = distance;
    Ok(())
}

pub(crate) fn grind_crouch_update<S: GrindingSource>(
    s: &mut S,
    v: Option<&mut GrindCrouchInstance>,
    drive_distance_to_com: FastString30,
) -> Result<(), S::Error> {
    s.resolve(Callback::GrindCrouchUpdate)?;
    let speed = s.scalar(Collection::GrindCrouch, CROUCH_DRIVE_SPEED)?;
    let maximum = s.scalar(Collection::GrindCrouch, CROUCH_MAXIMUM)?;
    let minimum = s.scalar(Collection::GrindCrouch, CROUCH_MINIMUM)?;
    let intent = s
        .intent(FastString36::encode(CROUCH_INTENT).expect("source literal"))?
        .unwrap_or(0.0);
    let input = upper(lower(1.0 - intent, minimum), maximum);
    let grind = s.grind_crouch()?;
    let physical = upper(lower(1.0 - grind, minimum), maximum);
    let target = if s.cvar(GrindingCvar::UsePhysInducedGrindCrouch)? {
        select(input - physical, physical, input)
    } else {
        input
    };
    let step = s.dt()? * speed;
    let v = v.ok_or_else(|| s.assertion("ControlGrindCrouch instance"))?;
    let current = v.distance;
    let distance = upper(lower(target, current - step), current + step);
    v.distance = distance;
    s.settable(drive_distance_to_com, distance, false)?;
    if s.cvar(GrindingCvar::ShowGrindCrouchText)? {
        s.debug_overlay()?;
    }
    Ok(())
}

pub(crate) fn control_fade_begin<S: GrindingSource>(
    s: &mut S,
    v: Option<&mut GrindControlFadeInstance>,
    names: &ControlFadeNames,
) -> Result<(), S::Error> {
    // +c and +8 are stored before any interface lookup.
    let v = v.ok_or_else(|| s.assertion("GrindControlFade instance"))?;
    v.time = 0.0;
    v.first_update = true;
    s.resolve(Callback::ControlFadeBegin)?;
    let distance = s.centre_of_mass_to_deck()?;
    let base = s.base_animation()?;
    s.settable(names.distance_board_to_cog, distance, false)?;
    s.settable(names.twist, 0.0, true)?;
    s.set_attributes()?;
    v.minimum = s
        .base_attribute(&base, names.twist)?
        .ok_or_else(|| s.assertion("GrindControlFade minimum twist attribute"))?;
    s.settable(names.twist, 1.0, true)?;
    s.set_attributes()?;
    v.maximum = s
        .base_attribute(&base, names.twist)?
        .ok_or_else(|| s.assertion("GrindControlFade maximum twist attribute"))?;
    let mut twist = s.shoulder_twist()?;
    if s.mirrored()? {
        twist = select(
            twist,
            std::f32::consts::PI - twist,
            -std::f32::consts::PI - twist,
        );
    }
    let twist = upper(lower(twist, v.minimum), v.maximum);
    s.settable(names.twist, twist, false)?;
    v.twist = twist;
    v.adjusted = Some(twist);
    v.delta = Some(0.0);
    if s.cvar(GrindingCvar::ShowGrindControlFade)? {
        s.debug_overlay()?;
    }
    Ok(())
}

pub(crate) fn control_fade_update<S: GrindingSource>(
    s: &mut S,
    v: Option<&mut GrindControlFadeInstance>,
    names: &ControlFadeNames,
) -> Result<(), S::Error> {
    let v = v.ok_or_else(|| s.assertion("GrindControlFade instance"))?;
    if v.first_update {
        v.first_update = false;
        return Ok(());
    }
    v.time = s.dt()? + v.time;
    s.resolve(Callback::ControlFadeUpdate)?;
    let intent = s.intent(names.twist_intent)?.unwrap_or(0.0);
    let rate = s.scalar(Collection::ControlFade, FADE_INTENT_RATE)?;
    let adjusted = v
        .adjusted
        .ok_or_else(|| s.assertion("GrindControlFade adjusted twist"))?;
    let adjusted = upper(lower(rate.mul_add(intent, adjusted), v.minimum), v.maximum);
    v.adjusted = Some(adjusted);
    let blend = s.scalar(Collection::ControlFade, FADE_BLEND)?;
    let blended = (1.0 - blend).mul_add(v.twist, adjusted * blend);
    let change = s.scalar(Collection::ControlFade, FADE_DELTA_CHANGE)?;
    let previous = v
        .delta
        .ok_or_else(|| s.assertion("GrindControlFade twist delta"))?;
    let delta = upper(
        lower(blended - v.twist, previous - change),
        previous + change,
    );
    let limit = s.scalar(Collection::ControlFade, FADE_DELTA_LIMIT)?;
    let delta = upper(lower(delta, -limit), limit);
    let twist = upper(lower(v.twist + delta, v.minimum), v.maximum);
    s.settable(names.twist, twist, false)?;
    if s.cvar(GrindingCvar::ShowGrindControlFade)? {
        s.debug_overlay()?;
    }
    v.twist = twist;
    v.delta = Some(delta);
    v.first_update = false;
    Ok(())
}
