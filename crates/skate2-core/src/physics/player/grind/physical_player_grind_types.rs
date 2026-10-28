//! PDB-shaped grind records: `GrindPrimitive`, `GrindGeometry` (112 bytes),
//! `GrindInvestigation` (400 bytes) and the collision test results.
//!
//! Bitfields are allocated from the least significant bit in declaration
//! order (verified against the 8c4 masks). Only the named bits are modeled:
//! `operator=` copies exactly those, and nothing reads the unnamed padding.
use super::math::{V4, ZERO};
use crate::grind_data_runtime::GrindDataPrimitive;

/// `GrindPrimitive` (48 bytes). `source` stands for `mpSplineHeader`: the
/// spline asset and segment the query copied the endpoints from.
#[derive(Clone, Debug, Default)]
pub(crate) struct GrindPrim {
    pub(crate) start: V4,
    pub(crate) end: V4,
    pub(crate) source: Option<GrindDataPrimitive>,
    /// `mIsVertical`, bit 31 of the flags word.
    pub(crate) vertical: bool,
}
impl GrindPrim {
    pub(crate) fn from_query(primitive: &GrindDataPrimitive) -> Self {
        Self {
            start: primitive.start,
            end: primitive.end,
            source: Some(primitive.clone()),
            vertical: primitive.vertical,
        }
    }
    /// The two leading `u64`s of `*mpSplineHeader` (GUID, local GUID); `None`
    /// for a null header pointer. A non-null offset outside the asset's inline
    /// headers is `Err`.
    pub(crate) fn spline_ids(&self) -> Result<Option<(u64, u64)>, &'static str> {
        let Some(source) = self.source.as_ref() else {
            return Ok(None);
        };
        let Some(offset) = source.spline_header_offset else {
            return Ok(None);
        };
        let header = source
            .source
            .spline_header(offset)
            .ok_or("grind primitive spline header outside its asset")?;
        Ok(Some((header.guid, header.local_guid)))
    }
}

/// `eGrindGeometryType`.
pub(crate) const THIN_RAIL: i32 = 0;
pub(crate) const FAT_RAIL: i32 = 1;
pub(crate) const LEDGE: i32 = 2;
pub(crate) const IMPOSSIBLE: i32 = 3;

/// `GrindGeometry`; the constructor 0x82b39068 zeroes all 112 bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Geometry {
    pub(crate) line1_hit: V4,
    pub(crate) line2_hit: V4,
    pub(crate) upmost_normal: V4,
    pub(crate) primitive_dir: V4,
    pub(crate) high_side_dir: V4,
    pub(crate) lo_normal_limit_angle: f32,
    pub(crate) hi_normal_limit_angle: f32,
    pub(crate) geometry_type: i32,
    pub(crate) audio_surface: u32,
    pub(crate) physics_surface: u32,
    pub(crate) is_stair: bool,
    pub(crate) is_apex: bool,
    pub(crate) is_curb: bool,
    pub(crate) uncertain: bool,
}

/// `eGrindType`.
pub(crate) const GRIND_5050: i32 = 0;
pub(crate) const BOARDSLIDE: i32 = 1;
pub(crate) const TIPSLIDE: i32 = 2;
pub(crate) const GRIND_5_O: i32 = 3;
pub(crate) const BACKSLASH: i32 = 4;

/// `eGrindEntryType`.
pub(crate) const RIDE_FROM_ABOVE: i32 = 0;
pub(crate) const RIDE_FROM_BELOW: i32 = 1;
pub(crate) const RIDE_INTO_COPING: i32 = 2;
pub(crate) const STAY_IN_GRIND: i32 = 3;
pub(crate) const GRIND_TO_GRIND: i32 = 4;
pub(crate) const AIR_TO_GRIND: i32 = 5;
pub(crate) const DROP_IN: i32 = 6;

/// `GrindInvestigation`; `Reset` 0x82b39088 is a 400-byte memset.
#[derive(Clone, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the seven named bits of the native flags word"
)]
pub(crate) struct Investigation {
    pub(crate) primitive_tangent: V4,
    pub(crate) primitive_location: V4,
    pub(crate) grind_dir: V4,
    pub(crate) grind_normal: V4,
    pub(crate) target_up_vector: V4,
    pub(crate) engagement_velocity: V4,
    pub(crate) front_primitive_location: V4,
    pub(crate) rear_primitive_location: V4,
    pub(crate) hang_primitive_location: V4,
    pub(crate) grind_type: i32,
    pub(crate) entry_type: i32,
    pub(crate) primitive: GrindPrim,
    pub(crate) hang_primitive: GrindPrim,
    pub(crate) geometry: Geometry,
    pub(crate) possible: bool,
    pub(crate) impact_speed: f32,
    pub(crate) friction_time_scalar: f32,
    pub(crate) exit_tilt_angle_deg: f32,
    pub(crate) truck_control_lr: f32,
    pub(crate) truck_control_ud: f32,
    pub(crate) gravity_relief_timer: f32,
    /// Bit 25.
    pub(crate) grab_tipslide: bool,
    /// Bit 26.
    pub(crate) hang_is_boardslide: bool,
    /// Bit 27.
    pub(crate) hang_exists: bool,
    /// Bit 28.
    pub(crate) loose_end_must_rise: bool,
    /// Bit 29.
    pub(crate) grind_on_front: bool,
    /// Bit 30.
    pub(crate) should_engage: bool,
    /// Bit 31.
    pub(crate) balance_requests_auto_exit: bool,
}
impl Investigation {
    pub(crate) fn reset() -> Self {
        Self::default()
    }
}

/// `CustomCollisionInfo` plus the test bookkeeping of `GrindCollTestResult`.
/// The index is only written with a hit; readers only use it after one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CollTestResult {
    pub(crate) performed: bool,
    pub(crate) point: V4,
    pub(crate) index: i32,
    pub(crate) hit: bool,
}
impl CollTestResult {
    pub(crate) const fn empty() -> Self {
        Self {
            performed: false,
            point: ZERO,
            index: -1,
            hit: false,
        }
    }
}
/// `GrindCollTestResultPair`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CollTestPair {
    pub(crate) performed: bool,
    pub(crate) front: CollTestResult,
    pub(crate) rear: CollTestResult,
}
impl CollTestPair {
    pub(crate) const fn empty() -> Self {
        Self {
            performed: false,
            front: CollTestResult::empty(),
            rear: CollTestResult::empty(),
        }
    }
}
