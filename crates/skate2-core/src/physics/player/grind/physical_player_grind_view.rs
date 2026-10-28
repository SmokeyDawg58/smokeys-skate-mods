//! What the grind code reads from `ProcessedPhysIn` and the global attribute
//! collections (`physics_grinds_default` +0xa4, `physics_wipeout_default`
//! +0x108). Missing processed values are errors, never zeros; an absent
//! scalar attribute reads `Attrib::DefaultDataArea` (zero) as the native does.
use super::math::V4;
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarReferences, grind_states::GrindCvar},
    physics::{GrindType, PointGraph},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct GrindError(pub(crate) String);
impl std::fmt::Display for GrindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grind: {}", self.0)
    }
}
pub(crate) fn error(e: impl std::fmt::Debug) -> GrindError {
    GrindError(format!("{e:?}"))
}
pub(crate) fn need<T>(value: Option<T>, field: &'static str) -> Result<T, GrindError> {
    value.ok_or_else(|| GrindError(format!("missing {field}")))
}

/// Live `cVar` targets of the grind references.
pub(crate) struct GrindCvars<'a>(pub(crate) &'a InputCvarReferences);
impl GrindCvars<'_> {
    pub(crate) fn f(&self, cvar: GrindCvar) -> Result<f32, GrindError> {
        self.0.try_grind_float(cvar).map_err(error)
    }
    pub(crate) fn i(&self, cvar: GrindCvar) -> Result<i32, GrindError> {
        self.0.try_grind_int(cvar).map_err(error)
    }
    pub(crate) fn b(&self, cvar: GrindCvar) -> Result<bool, GrindError> {
        self.0.try_grind_bool(cvar).map_err(error)
    }
    pub(crate) fn e(&self, cvar: GrindCvar) -> Result<i32, GrindError> {
        self.0.try_grind_enum(cvar).map_err(error)
    }
}

/// The native vector's four lanes. Unknown W lanes are read as zero: the grind
/// code consumes them only through three-lane dot products and cross XYZ.
pub(crate) fn lanes(v: SkeletonRootVector) -> V4 {
    [v.xyz[0], v.xyz[1], v.xyz[2], v.w.unwrap_or(0.0)]
}
pub(crate) fn root(v: V4) -> SkeletonRootVector {
    SkeletonRootVector::known(v)
}

/// `physics_grinds` class and the `default` collection.
pub(crate) const PHYSICS_GRINDS_CLASS: u64 = 0x4f84_4d63_08a3_8b54;
const PHYSICS_WIPEOUT_CLASS: u64 = 0xeb84_4cfd_bf92_8efd;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
/// `DeckCenterToTruck`, also `GetGeometryCollisionOffset` 0x82cfabd8.
pub(crate) const DECK_CENTER_TO_TRUCK: u64 = 0xcbfe_ffea_12f4_cc27;
/// `TruckToWheel`.
pub(crate) const TRUCK_TO_WHEEL: u64 = 0x4f0f_e851_90a9_51a6;
/// Boardslide intersector depth below the top line.
pub(crate) const BOARDSLIDE_DEPTH: u64 = 0xa61c_6bb8_b346_1d47;
/// Boardslide intersector top-line lift.
pub(crate) const BOARDSLIDE_LIFT: u64 = 0x8ccf_40a1_a6dd_b2dc;
/// Head tracking lead time of `PhysState_Grind_Common::Update` 0x82d9b4f8.
pub(crate) const HEAD_TRACKING_LEAD: u64 = 0x8db7_0884_3e8b_c348;
/// `physics_wipeout` impact speed that ends an engaging grind.
pub(crate) const MAX_GRIND_IMPACT_SPEED: u64 = 0x3275_07d7_c7f8_eff6;

/// `PointNegGraphData4` / `PointNegGraphData8` / `PointGraphData4` fields of
/// the grinds layout (`Attrib::Gen::physics_grinds::_LayoutStruct`).
#[derive(Clone, Copy)]
pub(crate) enum GrindGraph {
    ExitLeanAngleVsTime,
    VertEngagementHelpVsVelY,
    TimeOfGravityReliefVsVerticalSpeed,
    GravityReliefTimeVsLinearSpeed,
    FrictionVsTime,
    ExitAssistVsLeanAngle,
    DvEntryThreshScalarVsSinSlope,
    GrabVsSlope,
}
impl GrindGraph {
    const fn offset(self) -> usize {
        match self {
            Self::ExitLeanAngleVsTime => 0x0,
            Self::VertEngagementHelpVsVelY => 0x80,
            Self::TimeOfGravityReliefVsVerticalSpeed => 0xb0,
            Self::GravityReliefTimeVsLinearSpeed => 0x110,
            Self::FrictionVsTime => 0x140,
            Self::ExitAssistVsLeanAngle => 0x1a0,
            Self::DvEntryThreshScalarVsSinSlope => 0x1d0,
            Self::GrabVsSlope => 0x220,
        }
    }
}

pub(crate) struct GrindAttributes {
    grinds: AttributeRuntimeInstance,
    wipeout: AttributeRuntimeInstance,
}
impl GrindAttributes {
    pub(crate) fn bind(vault: &Arc<AttributeRuntimeVault>) -> Self {
        Self {
            grinds: vault.instance(PHYSICS_GRINDS_CLASS, DEFAULT_COLLECTION),
            wipeout: vault.instance(PHYSICS_WIPEOUT_CLASS, DEFAULT_COLLECTION),
        }
    }
    fn read(instance: &AttributeRuntimeInstance, key: u64) -> Result<f32, GrindError> {
        let Some(data) = instance.try_get_data(key, 0).map_err(error)? else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data
            .bytes()
            .get(..4)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| GrindError(format!("attribute {key:016x} is not a scalar")))?;
        Ok(f32::from_be_bytes(bytes))
    }
    /// `Attrib::Instance::GetAttributePointer(key, 0)` on `physics_grinds`.
    pub(crate) fn scalar(&self, key: u64) -> Result<f32, GrindError> {
        Self::read(&self.grinds, key)
    }
    pub(crate) fn wipeout_scalar(&self, key: u64) -> Result<f32, GrindError> {
        Self::read(&self.wipeout, key)
    }
    fn floats<const N: usize>(&self, offset: usize) -> Result<[f32; N], GrindError> {
        let data = self
            .grinds
            .try_cached_layout_field(offset, N * 4)
            .map_err(error)?
            .ok_or_else(|| GrindError("physics_grinds default layout is absent".into()))?;
        let bytes = data.bytes();
        Ok(std::array::from_fn(|i| {
            f32::from_be_bytes(bytes[i * 4..i * 4 + 4].try_into().expect("layout field"))
        }))
    }
    /// `PointGraphEval::Evaluate` 0x824e2c40 on a layout graph: four points at
    /// +0x10/+0x20, eight at +0x10/+0x30, and the plain four-point graphs at
    /// +0/+0x10.
    pub(crate) fn graph(&self, graph: GrindGraph, x: f32) -> Result<f32, GrindError> {
        let base = graph.offset();
        match graph {
            GrindGraph::ExitLeanAngleVsTime => {
                let curve = PointGraph::<8> {
                    x: self.floats(base + 0x10)?,
                    y: self.floats(base + 0x30)?,
                };
                Ok(curve.evaluate(x))
            }
            GrindGraph::GrabVsSlope => {
                let curve = PointGraph::<4> {
                    x: self.floats(base)?,
                    y: self.floats(base + 0x10)?,
                };
                Ok(curve.evaluate(x))
            }
            _ => {
                let curve = PointGraph::<4> {
                    x: self.floats(base + 0x10)?,
                    y: self.floats(base + 0x20)?,
                };
                Ok(curve.evaluate(x))
            }
        }
    }
}

/// `eGrindType` values; `GRIND_TYPE_NONE` is -1.
pub(crate) const fn grind_type_raw(value: GrindType) -> i32 {
    match value {
        GrindType::None => -1,
        GrindType::FiftyFifty => 0,
        GrindType::Boardslide => 1,
        GrindType::Tipslide => 2,
        GrindType::FiveO => 3,
        GrindType::Backslash => 4,
    }
}
pub(crate) fn grind_type_from_raw(value: i32) -> Result<GrindType, GrindError> {
    Ok(match value {
        -1 => GrindType::None,
        0 => GrindType::FiftyFifty,
        1 => GrindType::Boardslide,
        2 => GrindType::Tipslide,
        3 => GrindType::FiveO,
        4 => GrindType::Backslash,
        other => return Err(GrindError(format!("grind type {other} out of range"))),
    })
}

/// `ProcessedPhysIn` fields read by the grind manager and states.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrindView {
    /// +8e4 (equal to +8e8 during Update1).
    pub(crate) state: i32,
    pub(crate) category: i32,
    pub(crate) processed_grind_type: i32,
    pub(crate) substate: i32,
    pub(crate) wheels: i32,
    pub(crate) dt: f32,
    pub(crate) deck_speed: f32,
    pub(crate) mass: f32,
    pub(crate) balance: f32,
    pub(crate) translation: f32,
    pub(crate) nudge: f32,
    pub(crate) up_down: f32,
    pub(crate) loose_end_height: f32,
    /// +40: X, Y, Z axes and translation of `mDeckTransform`.
    pub(crate) deck: [V4; 4],
    pub(crate) velocity: V4,
    pub(crate) ground_normal: V4,
    pub(crate) up: V4,
    pub(crate) com_velocity: V4,
    pub(crate) flags_8c4: (u32, u32),
    pub(crate) flags_8c8: u32,
    pub(crate) flags_8cc: u32,
    pub(crate) flags_8d0: u32,
}
impl GrindView {
    pub(crate) fn try_read(p: &SkeletonProcessedContact) -> Result<Self, GrindError> {
        let s = p.physics_state_fields();
        let a = p.animation_attribute_fields();
        let o = p.prior_output();
        let deck = need(p.deck_transform(), "Processed deck transform")?;
        Ok(Self {
            state: need(s.current, "Processed state")?,
            category: need(s.category, "Processed category")?,
            processed_grind_type: grind_type_raw(need(s.grind_type, "Processed grind type")?),
            substate: need(s.grind_substate, "Processed grind substate")?,
            wheels: need(s.num_wheels_in_contact, "Processed wheel count")?,
            dt: need(s.time_delta, "Processed time delta")?,
            deck_speed: need(o.deck_speed(), "Processed deck speed")?.value,
            mass: need(p.physical_mass(), "Processed skateboard mass")?,
            balance: need(s.balance, "Processed balance")?,
            translation: need(a.grind_translation, "Processed grind translation")?,
            nudge: need(a.grind_nudge, "Processed grind nudge")?,
            up_down: need(a.grind_up_down, "Processed grind up/down")?,
            loose_end_height: need(a.grind_loose_end_height, "Processed loose end height")?,
            deck: deck.columns().map(lanes),
            velocity: lanes(need(o.velocity(), "Processed velocity")?.value),
            ground_normal: lanes(need(o.ground_normal(), "Processed ground normal")?.value),
            up: lanes(need(o.reckoning_up(), "Processed up vector")?.value),
            com_velocity: lanes(need(o.centre_of_mass_velocity(), "Processed COM velocity")?.value),
            flags_8c4: p.known_flags_8c4(),
            flags_8c8: need(p.known_flags_8c8(), "Processed flags8c8")?,
            flags_8cc: need(s.flags_8cc, "Processed flags8cc")?,
            flags_8d0: need(s.flags_8d0, "Processed flags8d0")?,
        })
    }
    /// One known 8c4 bit; an unknown bit is an error, not a zero.
    pub(crate) fn bit_8c4(&self, mask: u32) -> Result<bool, GrindError> {
        if self.flags_8c4.1 & mask != mask {
            return Err(GrindError(format!(
                "Processed flags8c4 bit {mask:08x} unknown"
            )));
        }
        Ok(self.flags_8c4.0 & mask != 0)
    }
    pub(crate) fn deck_x(&self) -> V4 {
        self.deck[0]
    }
    pub(crate) fn deck_y(&self) -> V4 {
        self.deck[1]
    }
    pub(crate) fn deck_z(&self) -> V4 {
        self.deck[2]
    }
    pub(crate) fn deck_t(&self) -> V4 {
        self.deck[3]
    }
}
