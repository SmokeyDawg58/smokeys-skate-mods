//! `GrindAirAdjust::Start` 0x82d991c0 and the active `UpdateGrindAdjust`
//! 0x82dd1518: `CalculateGrindTransform` 0x82dc6928, `ProjectPoints`
//! 0x82dc90c0, `SelectContactPoint` 0x82dc6d70, `SelectGrind` 0x82dcf6d0 with
//! `IsThisGrindTypeWithinRange` 0x82dc95e0, `UpdateOffset` 0x82dc9780 and
//! `UpdateAngle` 0x82dcf7d8; then the offset transform the skeleton builds from
//! the result in `Skeleton::UpdateGrindAirAdjust` 0x82b4cd80.
use super::{DEFAULT, GRIND_AIR_CLASS, GrindAirAdjustOwner, GrindAirConstructError};
use crate::{
    animation::AnimationMatrix,
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    input_cvar_runtime::{InputCvarReferences, grind_states::GrindCvar as C},
    physical_player_grind_manager::math::{GrindNum, abs1, fsel, neg, neg1, sub},
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_normalize_angle,
        try_normalize_safe, try_signed_angle,
    },
    reckoning_sin_cos::try_sin_cos,
};
use std::sync::Arc;

type V4 = [f32; 4];
type Frame = [V4; 4];

const ZERO: V4 = [0.0; 4];
const I: V4 = [1.0, 0.0, 0.0, 0.0];
const J: V4 = [0.0, 1.0, 0.0, 0.0];
const K: V4 = [0.0, 0.0, 1.0, 0.0];
/// `__real_c11ccccd`, the gravity `ProjectPoints` integrates.
const GRAVITY: f32 = f32::from_bits(0xc11c_cccd);
/// `SMALL_FLOAT` 0x823557f0.
const SMALL_FLOAT: f32 = f32::from_bits(0x3780_0000);
const DEGREES_TO_RADIANS: f32 = f32::from_bits(0x3c8e_fa35);
const RADIANS_TO_DEGREES: f32 = f32::from_bits(0x4265_2ee1);
const PI: f32 = f32::from_bits(0x4049_0fdb);
const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
/// `physics_grinds_air` scalar of `ProjectPoints`: the per-second velocity
/// change along the processed up vector.
const UP_ACCELERATION: u64 = 0x6aa4_b4ae_2e34_a5a5;
/// `UpdateOffset` limits: per-frame and total position adjustment lengths.
const MAX_POSITION_PER_FRAME: u64 = 0x06b0_96ea_e592_3b51;
const MAX_POSITION_TOTAL: u64 = 0x8f02_c2c1_5443_20bc;
/// `UpdateAngle` per-frame angle limit, in degrees.
const MAX_ANGLE_PER_FRAME: u64 = 0x1846_3541_b99b_6c11;

#[derive(Debug)]
pub(crate) enum GrindAirUpdateError {
    Attribute(String),
    Cvar(String),
    Numeric(String),
    Missing(&'static str),
    /// `kDrawGrindAssistInfo` needs the `DebugOverlay` singleton, which this
    /// ProcessData path does not reach.
    DebugOverlay,
    Index(&'static str, i64),
}
impl std::fmt::Display for GrindAirUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "GrindAirAdjust attribute: {e}"),
            Self::Cvar(e) => write!(f, "GrindAirAdjust cvar: {e}"),
            Self::Numeric(e) => write!(f, "GrindAirAdjust numeric: {e}"),
            Self::Missing(field) => write!(f, "GrindAirAdjust missing {field}"),
            Self::DebugOverlay => f.write_str("GrindAirAdjust debug drawing needs the overlay"),
            Self::Index(what, value) => write!(f, "GrindAirAdjust {what} index {value}"),
        }
    }
}
type E = GrindAirUpdateError;
fn numeric(error: impl std::fmt::Debug) -> E {
    E::Numeric(format!("{error:?}"))
}

/// `GrindAirAdjustIn` as `PhysState_KnownAir::Enter` 0x82d9f828 fills it: the
/// selector's grind primitive (start, end; its header is never written by the
/// selector) and the possible orientations. The six target orientation
/// matrices are the caller's unwritten stack and stay unmodeled.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrindAirAdjustIn {
    pub(crate) start: V4,
    pub(crate) end: V4,
    pub(crate) vertical_bit: u32,
    pub(crate) geometry_type: i32,
    pub(crate) is_garbage: bool,
    /// Boardslide, tipslide, backslash and high-side directions.
    pub(crate) directions: [V4; 4],
}

/// `GrindAirAdjustOut` (0x40). Fields the active update did not write are the
/// caller's unwritten stack.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrindAirAdjustOut {
    pub(crate) grind_direction: Option<V4>,
    pub(crate) offset: Option<V4>,
    pub(crate) orientation_euler: Option<V4>,
    pub(crate) adjustment_required: bool,
}

/// `ProcessedPhysIn` fields the active update reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrindAirProcessed {
    /// +40 `mDeckTransform` X, Y, Z, W.
    pub(crate) deck: Frame,
    /// +190.
    pub(crate) velocity: V4,
    /// +220.
    pub(crate) up: V4,
    /// +2d0 `mDeckAngularVelocity`.
    pub(crate) angular_velocity: V4,
    /// +93c.
    pub(crate) time_delta: f32,
}

/// Cvar and attribute collaborators of the update.
pub(crate) struct GrindAirServices<'a> {
    pub(crate) cvars: &'a InputCvarReferences,
    pub(crate) vault: &'a Arc<AttributeRuntimeVault>,
}
impl GrindAirServices<'_> {
    fn cvar(&self, cvar: C) -> Result<bool, E> {
        self.cvars
            .try_grind_bool(cvar)
            .map_err(|e| E::Cvar(format!("{e:?}")))
    }
    fn debug(&self) -> Result<(), E> {
        if self.cvar(C::kDrawGrindAssistInfo)? {
            return Err(E::DebugOverlay);
        }
        Ok(())
    }
    fn scalar(&self, key: u64) -> Result<f32, E> {
        let instance: AttributeRuntimeInstance = self.vault.instance(GRIND_AIR_CLASS, DEFAULT);
        let Some(data) = instance
            .try_get_data(key, 0)
            .map_err(|e| E::Attribute(format!("{e:?}")))?
        else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data
            .bytes()
            .get(..4)
            .and_then(|b| b.try_into().ok())
            .ok_or(E::Missing("four-byte grind-air scalar"))?;
        Ok(f32::from_be_bytes(bytes))
    }
}

fn splat_mul(a: V4, s: f32) -> V4 {
    a.map(|v| v * s)
}
/// `vmaddfp` with a splat multiplier: `v * s + acc`, one rounding per lane.
fn fma_splat(v: V4, s: f32, acc: V4) -> V4 {
    std::array::from_fn(|i| v[i].mul_add(s, acc[i]))
}
/// Row-vector transform of `p` by `rows`, starting from `x * row0` (or the
/// seed) with each later row fused in.
fn row_transform(p: V4, rows: &[V4; 3], seed: Option<V4>) -> V4 {
    let first = match seed {
        Some(seed) => fma_splat(rows[0], p[0], seed),
        None => splat_mul(rows[0], p[0]),
    };
    fma_splat(rows[2], p[2], fma_splat(rows[1], p[1], first))
}
/// The inline axis-angle rotation rows shared by `ProjectPoints` and the
/// skeleton: `vperm <0,5,0,0>` then `vrlimi` Z, so each W lane repeats X.
pub(crate) fn axis_angle_rows(axis: V4, sine: f32, cosine: f32) -> [V4; 3] {
    let (ax, ay, az) = (axis[0], axis[1], axis[2]);
    let t = 1.0 - cosine;
    let s_ax = sine * ax;
    let s_ay = sine * ay;
    let s_az = sine * az;
    let t_ay = t * ay;
    let t_az = t * az;
    let t_ax = t * ax;
    let yx = t_ay * ax;
    let zy = t_az * ay;
    let xz = t_ax * az;
    let m01 = t_ax.mul_add(ay, s_az);
    let m00 = t_ax.mul_add(ax, cosine);
    let m11 = t_ay.mul_add(ay, cosine);
    let m20 = t_az.mul_add(ax, s_ay);
    let m12 = t_ay.mul_add(az, s_ax);
    let m22 = t_az.mul_add(az, cosine);
    let m10 = yx - s_az;
    let m21 = zy - s_ax;
    let m02 = xz - s_ay;
    [
        [m00, m01, m02, m00],
        [m10, m11, m12, m10],
        [m20, m21, m22, m20],
    ]
}
fn length(v: V4, num: &mut GrindNum<'_>) -> Result<f32, E> {
    num.length(v).map_err(numeric)
}
fn dot(a: V4, b: V4, num: &mut GrindNum<'_>) -> Result<f32, E> {
    num.dot(a, b).map_err(numeric)
}
fn normalize_safe(v: V4, num: &mut GrindNum<'_>) -> Result<V4, E> {
    try_normalize_safe(v, ZERO, num.n).map_err(numeric)
}
fn cross(a: V4, b: V4) -> V4 {
    crate::reckoning_numeric::cross_full4(a, b)
}
fn index(value: i32, len: usize, what: &'static str) -> Result<usize, E> {
    usize::try_from(value)
        .ok()
        .filter(|&i| i < len)
        .ok_or(E::Index(what, i64::from(value)))
}
/// `fctiwz` of a frame count plus one half.
fn frame_index(frames: f32, num: &mut GrindNum<'_>) -> Result<usize, E> {
    let frame = num.n.truncate_i32(frames + 0.5).map_err(numeric)?;
    index(frame, 12, "predicted frame")
}

impl GrindAirAdjustOwner {
    /// `GrindAirAdjust::Start` 0x82d991c0: store the input, clear the chosen
    /// contact and adjustments, then rerun `InitTargets` 0x82dc9a70.
    pub(crate) fn try_start<R: crate::reckoning_sin_cos::ReckoningSinCosNumeric>(
        &mut self,
        input: &GrindAirAdjustIn,
        vault: &Arc<AttributeRuntimeVault>,
        rounding: &mut R,
    ) -> Result<(), GrindAirConstructError<R::Error>> {
        self.started = true;
        self.input_primitive.start = input.start;
        self.input_primitive.end = input.end;
        self.input_primitive.source = None;
        self.input_primitive.flag_word =
            (self.input_primitive.flag_word & 0x7fff_ffff) | (input.vertical_bit & 0x8000_0000);
        self.input_geometry_type = input.geometry_type;
        self.input_is_garbage = input.is_garbage;
        self.input_directions = input.directions;
        self.chosen_contact_valid = false;
        self.orientation_adjust_per_frame = ZERO;
        self.position_adjust_per_frame = ZERO;
        self.total_position_adjust = ZERO;
        self.total_orientation_adjust = ZERO;
        self.chosen_contact = -1;
        self.chosen_type = -1;
        self.last_chosen_type = -1;
        self.try_init_targets(vault, rounding)
    }

    /// `UpdateGrindAdjust` 0x82dd1518. The inactive branches read nothing
    /// beyond the activation byte, the assist cvar and the two processed flag
    /// words; `adjusting` always takes the result's flag.
    pub(crate) fn try_update_grind_adjust(
        &mut self,
        flag: &mut dyn FnMut(u32, u32) -> Result<bool, E>,
        processed: &mut dyn FnMut() -> Result<GrindAirProcessed, E>,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<GrindAirAdjustOut, E> {
        let mut out = GrindAirAdjustOut {
            grind_direction: None,
            offset: None,
            orientation_euler: None,
            adjustment_required: false,
        };
        if self.activated && services.cvar(C::kGrindAssistActive)? {
            // 8d0 bit 27, then the low bit of the 8c8 halfword (word bit 16).
            if flag(0x8d0, 0x0800_0000)? || flag(0x8c8, 0x0001_0000)? {
                self.total_position_adjust = ZERO;
                self.total_orientation_adjust = ZERO;
                out.adjustment_required = false;
            } else {
                self.chosen_contact_valid = false;
                self.last_chosen_type = self.chosen_type;
                for contact in 0..5 {
                    self.frames_to_plane[contact] = -1.0;
                    self.distances_to_grind[contact] = 0.0;
                }
                let p = processed()?;
                self.calculate_grind_transform(&p, num)?;
                self.project_points(&p, services, num)?;
                self.select_contact_point(num)?;
                self.select_grind(services, num)?;
                if self.chosen_contact_valid {
                    self.update_offset(&mut out, services, num)?;
                    self.update_angle(&mut out, services, num)?;
                }
            }
        }
        self.adjusting = out.adjustment_required;
        Ok(out)
    }

    /// `CalculateGrindTransform` 0x82dc6928: X across the rail from the up
    /// vector, Y the rail-normal up, Z the rail; the velocity frame takes X from
    /// the velocity instead. Both translations are zero.
    fn calculate_grind_transform(
        &mut self,
        p: &GrindAirProcessed,
        num: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        let rail = normalize_safe(
            sub(self.input_primitive.end, self.input_primitive.start),
            num,
        )?;
        let across = cross(p.up, rail);
        let upright = cross(rail, across);
        let x = normalize_safe(across, num)?;
        self.grind_transform[0] = x;
        let y = normalize_safe(upright, num)?;
        self.grind_transform[1] = y;
        self.grind_transform[2] = rail;
        self.grind_transform[3] = ZERO;
        let velocity_across = cross(p.velocity, rail);
        let velocity_x = normalize_safe(velocity_across, num)?;
        self.grind_velocity_transform[0] = velocity_x;
        let velocity_y = normalize_safe(upright, num)?;
        self.grind_velocity_transform[1] = velocity_y;
        self.grind_velocity_transform[2] = rail;
        self.grind_velocity_transform[3] = ZERO;
        Ok(())
    }

    /// `ProjectPoints` 0x82dc90c0: step the deck frame forward with the
    /// angular velocity and the velocity (gravity plus the up acceleration per
    /// frame), then the five contact points and the deck-to-rail angle of each
    /// predicted frame.
    fn project_points(
        &mut self,
        p: &GrindAirProcessed,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        let gravity_step = [0.0, p.time_delta * GRAVITY, 0.0, 0.0];
        let up_acceleration = services.scalar(UP_ACCELERATION)?;
        let omega = p.angular_velocity;
        let omega_squared = dot(omega, omega, num)?;
        self.predicted_decks[0] = p.deck;
        let up_scale = p.time_delta * up_acceleration;
        let mut velocity = p.velocity;
        let seed = num
            .n
            .reciprocal_sqrt_estimate(omega_squared)
            .map_err(numeric)?;
        let delta = fma_splat(p.up, up_scale, gravity_step);
        let speed = length_from_squared_and_estimate(omega_squared, seed);
        let angle = p.time_delta * speed;
        let rows = if speed < SMALL_FLOAT {
            [I, J, K]
        } else {
            let inverse = 1.0 / speed;
            let axis = splat_mul(omega, inverse);
            let (sine, cosine) = try_sin_cos(angle, num.s).map_err(numeric)?;
            axis_angle_rows(axis, sine, cosine)
        };
        let count = index(self.num_frames, 13, "frame count")?;
        for frame in 1..count.max(1) {
            let previous = self.predicted_decks[frame - 1];
            let z = row_transform(previous[2], &rows, None);
            let x = row_transform(previous[0], &rows, None);
            let y = row_transform(previous[1], &rows, None);
            velocity = std::array::from_fn(|i| velocity[i] + delta[i]);
            let dt = p.time_delta;
            let position = fma_splat(velocity, dt, previous[3]);
            self.predicted_decks[frame] = [x, y, z, position];
        }
        for frame in 0..count {
            let deck = self.predicted_decks[frame];
            for contact in 0..5 {
                let offset = self.contact_offsets[contact];
                let along_x = fma_splat(deck[0], offset[0], deck[3]);
                let along_y = fma_splat(deck[1], offset[1], along_x);
                self.predicted_contacts[frame][contact] = fma_splat(deck[2], offset[2], along_y);
            }
            self.deck_angles[frame] = self.calculate_angle_between_deck_z_and_grind(frame, num)?;
        }
        Ok(())
    }

    /// `CalculateAngleBetweenDeckZandGrind` 0x82dc6b18: the nose-to-tail
    /// contact line in the rail plane, flipped toward the rail, as a signed
    /// angle in degrees (negated). The `> 90` fold compares radians.
    fn calculate_angle_between_deck_z_and_grind(
        &self,
        frame: usize,
        num: &mut GrindNum<'_>,
    ) -> Result<f32, E> {
        if frame >= index(self.num_frames, 13, "frame count")? {
            return Err(E::Missing(
                "CalculateAngleBetweenDeckZandGrind frame assertion",
            ));
        }
        let contacts = self.predicted_contacts[frame];
        let long_axis = sub(contacts[1], contacts[4]);
        let y = self.grind_transform[1];
        let along_y = dot(y, long_axis, num)?;
        let in_plane = sub(long_axis, splat_mul(y, along_y));
        let projected = normalize_safe(in_plane, num)?;
        let rail = self.grind_transform[2];
        let facing = if dot(projected, rail, num)? < 0.0 {
            neg(projected)
        } else {
            projected
        };
        let cosine = dot(facing, rail, num)?;
        let lower = if -1.0 > cosine { -1.0 } else { cosine };
        let bounded = if 1.0 < lower { 1.0 } else { lower };
        let mut angle = abs1(num.acos(bounded).map_err(numeric)?);
        if angle > f32::from_bits(0x42b4_0000) {
            angle = f32::from_bits(0x4334_0000) - angle;
        }
        let turn = dot(y, cross(facing, rail), num)?;
        let sign = if 0.0 > turn { -1.0 } else { 1.0 };
        let signed = sign * angle;
        Ok(neg1(signed * RADIANS_TO_DEGREES))
    }

    /// `SelectContactPoint` 0x82dc6d70: per contact, the first predicted frame
    /// pair that crosses the plane through the rail start along the velocity
    /// frame's Y, the interpolated crossing, the nearest rail point and the
    /// offset and distance to it; the earliest crossing frame is kept.
    fn select_contact_point(&mut self, num: &mut GrindNum<'_>) -> Result<(), E> {
        #[expect(clippy::cast_precision_loss, reason = "native fcfid then frsp")]
        let mut closest = self.num_frames as f32;
        let count = index(self.num_frames, 13, "frame count")?;
        let start = self.input_primitive.start;
        for contact in 0..5 {
            self.frames_to_plane[contact] = -1.0;
            self.distances_to_grind[contact] = 0.0;
            let normal = self.grind_velocity_transform[1];
            for frame in 1..count.max(1) {
                let before = self.predicted_contacts[frame - 1][contact];
                let after = self.predicted_contacts[frame][contact];
                let side_before = dot(normal, sub(before, start), num)?;
                let side_after = dot(normal, sub(after, start), num)?;
                if !(side_before * side_after < 0.0) {
                    continue;
                }
                #[expect(clippy::cast_precision_loss, reason = "native fcfid then frsp")]
                let previous_frame = (frame - 1) as f32;
                let fraction = abs1(side_before / (side_before - side_after));
                let remaining = 1.0 - fraction;
                self.frames_to_plane[contact] = previous_frame + fraction;
                let crossing =
                    std::array::from_fn(|i| before[i].mul_add(remaining, after[i] * fraction));
                self.plane_intersections[contact] = crossing;
                let rail = self.grind_velocity_transform[2];
                let frames = self.frames_to_plane[contact];
                closest = fsel(closest - frames, frames, closest);
                let along = dot(sub(crossing, start), rail, num)?;
                let nearest = fma_splat(rail, along, start);
                self.nearest_points[contact] = nearest;
                let offset = sub(nearest, crossing);
                self.world_offsets_to_hit[contact] = offset;
                self.distances_to_grind[contact] = length(offset, num)?;
                break;
            }
        }
        self.closest_frame = closest;
        Ok(())
    }

    /// `IsThisGrindTypeWithinRange` 0x82dc95e0.
    fn is_grind_type_within_range(
        &self,
        grind_type: usize,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<bool, E> {
        let frames = self.frames_to_plane;
        if grind_type == 3 {
            let earlier = fsel(frames[2] - frames[3], frames[3], frames[2]);
            if earlier < 4.0 && abs1(frames[2] - frames[3]) > 2.0 {
                services.debug()?;
                return Ok(false);
            }
        } else if self.closest_frame < 5.0 {
            let contact = index(self.targets[grind_type].contact, 5, "target contact")?;
            if abs1(self.closest_frame - frames[contact]) > 3.0 {
                services.debug()?;
                return Ok(false);
            }
        }
        let target = self.targets[grind_type];
        let contact = index(target.contact, 5, "target contact")?;
        let frames = frames[contact];
        if !(frames > 0.0) || !(self.distances_to_grind[contact] < target.max_distance) {
            return Ok(false);
        }
        let frame = frame_index(frames, num)?;
        Ok(abs1(target.angle - abs1(self.deck_angles[frame])) < target.angle_range)
    }

    /// `SelectGrind` 0x82dcf6d0: the first in-range type below four wins; the
    /// truck-free types four and five compete on distance. A boardslide
    /// candidate keeps last frame's nose or tail slide while that stays in
    /// range.
    fn select_grind(
        &mut self,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        for candidate in 0..6 {
            if !self.is_grind_type_within_range(candidate, services, num)? {
                continue;
            }
            let mut grind_type = candidate;
            if candidate == 2 && matches!(self.last_chosen_type, 0 | 1) {
                let last = index(self.last_chosen_type, 6, "last grind type")?;
                if self.is_grind_type_within_range(last, services, num)? {
                    grind_type = last;
                    services.debug()?;
                }
            }
            let angle = self.deck_angles[grind_type];
            let target = self.targets[grind_type];
            let take = if self.chosen_contact_valid {
                let contact = index(target.contact, 5, "target contact")?;
                let chosen = index(self.chosen_contact, 5, "chosen contact")?;
                self.distances_to_grind[contact] < self.distances_to_grind[chosen]
            } else {
                true
            };
            if take {
                self.chosen_contact_valid = true;
                self.chosen_angle = angle;
                self.chosen_contact = target.contact;
                self.chosen_type = i32::try_from(grind_type).expect("six grind types");
            }
            if self.chosen_type < 4 {
                break;
            }
        }
        Ok(())
    }

    /// `UpdateOffset` 0x82dc9780: the per-frame adjustment gathers the chosen
    /// contact's offset over its frames to impact, both it and the total are
    /// length-limited, and the total is the output offset.
    fn update_offset(
        &mut self,
        out: &mut GrindAirAdjustOut,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        if !services.cvar(C::kGrindAssistApplyOffset)? {
            return Ok(());
        }
        let contact = index(self.chosen_contact, 5, "chosen contact")?;
        let frames = self.frames_to_plane[contact];
        let inverse = num.reciprocal(frames).map_err(numeric)?;
        let per_frame = fma_splat(
            self.world_offsets_to_hit[contact],
            inverse,
            self.position_adjust_per_frame,
        );
        self.position_adjust_per_frame = per_frame;
        let per_frame_length = length(per_frame, num)?;
        let per_frame_limit = services.scalar(MAX_POSITION_PER_FRAME)?;
        if per_frame_length > per_frame_limit {
            let ratio = per_frame_limit / per_frame_length;
            self.position_adjust_per_frame = splat_mul(self.position_adjust_per_frame, ratio);
        }
        self.total_position_adjust = std::array::from_fn(|i| {
            self.total_position_adjust[i] + self.position_adjust_per_frame[i]
        });
        let total_limit = services.scalar(MAX_POSITION_TOTAL)?;
        let total_length = length(self.total_position_adjust, num)?;
        if total_length > total_limit {
            let ratio = total_limit / total_length;
            self.total_position_adjust = splat_mul(self.total_position_adjust, ratio);
        }
        out.offset = Some(self.total_position_adjust);
        out.adjustment_required = true;
        Ok(())
    }

    /// `CalculateAngleBetweenDeckZAndDesiredLandingZ` 0x82dcf4f8: the contact
    /// line across the rail at the chosen contact's impact frame against the
    /// slide direction of the chosen type (nose/tail slides take the nearer of
    /// tipslide and backslash, boardslide its own, trucks none).
    fn calculate_angle_to_desired_landing_z(&self, num: &mut GrindNum<'_>) -> Result<f32, E> {
        let contact = index(self.chosen_contact, 5, "chosen contact")?;
        let frame = frame_index(self.frames_to_plane[contact], num)?;
        let contacts = self.predicted_contacts[frame];
        let rail = self.grind_transform[2];
        let long_axis = sub(contacts[1], contacts[4]);
        let along = dot(rail, long_axis, num)?;
        let across = sub(long_axis, splat_mul(rail, along));
        let mut direction = normalize_safe(across, num)?;
        let [boardslide, tipslide, backslash, _] = self.input_directions;
        match self.chosen_type {
            2 => {
                let angle =
                    try_signed_angle(direction, boardslide, rail, num.n).map_err(numeric)?;
                let mut angle = try_normalize_angle(angle, num.n).map_err(numeric)?;
                if angle > HALF_PI {
                    angle -= PI;
                }
                if angle < neg1(HALF_PI) {
                    angle += PI;
                }
                Ok(angle)
            }
            0 | 1 => {
                if self.chosen_type == 0 {
                    direction = neg(direction);
                }
                let tip = try_signed_angle(direction, tipslide, rail, num.n).map_err(numeric)?;
                let tip = try_normalize_angle(tip, num.n).map_err(numeric)?;
                let back = try_signed_angle(direction, backslash, rail, num.n).map_err(numeric)?;
                let back = try_normalize_angle(back, num.n).map_err(numeric)?;
                Ok(if abs1(tip) < abs1(back) { tip } else { back })
            }
            _ => Ok(0.0),
        }
    }

    /// `UpdateAngle` 0x82dcf7d8: Y turns toward the chosen type's target angle
    /// (sign follows the predicted angle) at the assist factor, X toward the
    /// desired landing axis, each per-frame value clamped to the attribute
    /// limit before joining the total.
    fn update_angle(
        &mut self,
        out: &mut GrindAirAdjustOut,
        services: &GrindAirServices<'_>,
        num: &mut GrindNum<'_>,
    ) -> Result<(), E> {
        if !self.chosen_contact_valid {
            return Err(E::Missing("UpdateAngle chosen contact assertion"));
        }
        if !services.cvar(C::kGrindAssistApplyAngleOffset)? {
            return Ok(());
        }
        out.grind_direction = Some(self.grind_transform[2]);
        let limit = services.scalar(MAX_ANGLE_PER_FRAME)?;
        let predicted = self.chosen_angle;
        let grind_type = index(self.chosen_type, 6, "chosen grind type")?;
        let target = self.targets[grind_type];
        let wanted = if predicted > 0.0 {
            target.angle
        } else {
            neg1(target.angle)
        };
        let contact = index(self.chosen_contact, 5, "chosen contact")?;
        let difference = wanted - predicted;
        let lower = neg1(limit);
        let frames = self.frames_to_plane[contact];
        let per_frame = difference / frames;
        let assisted = per_frame * target.angle_assist_factor;
        self.y_angle_distance = frames * per_frame;
        let current_degrees = self.orientation_adjust_per_frame[1] * RADIANS_TO_DEGREES;
        let floor = fsel(lower - assisted, lower, assisted);
        let step = fsel(limit - floor, floor, limit);
        let turned = step + current_degrees;
        let turned_floor = fsel(lower - turned, lower, turned);
        let turned = fsel(limit - turned_floor, turned_floor, limit);
        let limit_radians = limit * DEGREES_TO_RADIANS;
        self.orientation_adjust_per_frame[1] = turned * DEGREES_TO_RADIANS;
        let landing = self.calculate_angle_to_desired_landing_z(num)?;
        self.x_angle_distance = landing * RADIANS_TO_DEGREES;
        let lower = neg1(limit_radians);
        let frames = self.frames_to_plane[contact];
        let x_step = landing / frames;
        let x = self.orientation_adjust_per_frame[0] + x_step;
        let x_floor = fsel(lower - x, lower, x);
        self.orientation_adjust_per_frame[0] =
            fsel(limit_radians - x_floor, x_floor, limit_radians);
        self.total_orientation_adjust = std::array::from_fn(|i| {
            self.total_orientation_adjust[i] + self.orientation_adjust_per_frame[i]
        });
        out.orientation_euler = Some(self.total_orientation_adjust);
        out.adjustment_required = true;
        Ok(())
    }
}

/// `Skeleton::UpdateGrindAirAdjust` 0x82b4cd80 composition: the animation
/// frame turned about the grind direction by Euler X in world space, taken
/// back to animation space, pivoted about the mapped skateboard position,
/// turned by Euler Y and moved by the world offset.
pub(crate) fn grind_air_offset_transform(
    out: &GrindAirAdjustOut,
    animation_to_world: &Frame,
    world_to_animation: &AnimationMatrix,
    mapped_skateboard: &AnimationMatrix,
    num: &mut GrindNum<'_>,
) -> Result<AnimationMatrix, E> {
    let euler = out
        .orientation_euler
        .ok_or(E::Missing("unwritten GrindAirAdjustOut orientation"))?;
    let direction = out
        .grind_direction
        .ok_or(E::Missing("unwritten GrindAirAdjustOut direction"))?;
    let offset = out
        .offset
        .ok_or(E::Missing("unwritten GrindAirAdjustOut offset"))?;
    let (sine_y, cosine_y) = try_sin_cos(euler[1], num.s).map_err(numeric)?;
    let turn_y = [
        [cosine_y, 0.0, neg1(sine_y), cosine_y],
        [0.0, 1.0, 0.0, 0.0],
        [sine_y, 0.0, cosine_y, sine_y],
    ];
    let (sine_x, cosine_x) = try_sin_cos(euler[0], num.s).map_err(numeric)?;
    let turn_x = axis_angle_rows(direction, sine_x, cosine_x);
    let world = [
        row_transform(animation_to_world[0], &turn_x, None),
        row_transform(animation_to_world[1], &turn_x, None),
        row_transform(animation_to_world[2], &turn_x, None),
    ];
    let w2a: [V4; 4] =
        std::array::from_fn(|c| std::array::from_fn(|l| world_to_animation[c * 4 + l]));
    let w2a_rows = [w2a[0], w2a[1], w2a[2]];
    let pivot: V4 = std::array::from_fn(|l| mapped_skateboard[12 + l]);
    let offset_animation = row_transform(offset, &w2a_rows, None);
    let moved: V4 = std::array::from_fn(|l| offset_animation[l] + pivot[l]);
    let rotated = [
        row_transform(world[0], &w2a_rows, None),
        row_transform(world[1], &w2a_rows, None),
        row_transform(world[2], &w2a_rows, None),
    ];
    // The pivot matrix rows I, J, K, -pivot times the rotated rows, lane by lane.
    let unpivot = neg(pivot);
    let pivoted = [
        row_transform(I, &rotated, None),
        row_transform(J, &rotated, None),
        row_transform(K, &rotated, None),
        row_transform(unpivot, &rotated, Some(ZERO)),
    ];
    let turned = [
        row_transform(pivoted[0], &turn_y, None),
        row_transform(pivoted[1], &turn_y, None),
        row_transform(pivoted[2], &turn_y, None),
        row_transform(pivoted[3], &turn_y, Some(ZERO)),
    ];
    let identity = [I, J, K];
    let result = [
        row_transform(turned[0], &identity, None),
        row_transform(turned[1], &identity, None),
        row_transform(turned[2], &identity, None),
        row_transform(turned[3], &identity, Some(moved)),
    ];
    Ok(std::array::from_fn(|index| result[index / 4][index % 4]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_angle_rows_rotate_a_quarter_turn_about_y() {
        let rows = axis_angle_rows(J, 1.0, 0.0);
        let x = row_transform(I, &rows, None);
        assert!((x[0]).abs() < 1e-6 && (x[2] + 1.0).abs() < 1e-6);
        assert_eq!(rows[0][3], rows[0][0]);
    }

    #[test]
    fn identity_rows_leave_a_row_vector_in_place() {
        let v = [0.25, -2.0, 3.5, 7.0];
        let moved = row_transform(v, &[I, J, K], Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(moved, [1.25, -1.0, 4.5, 1.0]);
    }
}
