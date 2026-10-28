//! `TrajectorySelector` grind targeting on the retained onboard selector:
//! `CalculateGrindScore` 0x82dc5c38 with `GrindData::TrajectoryBoxTest`
//! 0x82cf9cf0 and `AnalyzeAndAdjustTrajectory` 0x82dc5160, and
//! `CalculateGrindPenalty` 0x82db9ee0.
//!
//! Debug drawing left in the build is its `DebugOverlay` singleton access;
//! every such site requires the overlay.
use super::{
    OnboardTrajectoryOwner, RetainedGrindTrajectoryResult, RetainedPredictionResult,
    UnwrittenGrindOrientations,
    selector_numeric::{DT, adjust, collision_position},
};
use crate::physical_player_footplant::RetainedTrajectory;
use crate::{
    attribute_runtime::{AttributeRuntimeData, AttributeRuntimeInstance},
    debug_overlay::DebugOverlaySlot,
    grind_data_octree::OctreeBounds,
    grind_data_runtime::{GrindDataPrimitive, GrindDataQueryResult},
    input_cvar_runtime::grind_states::GrindCvar as C,
    physical_player_grind_manager::{
        geometry::{GeometryLineBatch, calc_grind_geometry, calc_tilted_upmost_normal},
        math::{
            DEG_TO_RAD, GrindNum, HALF_PI, I, J, PI, V4, ZERO, abs1, add, cross, madd, neg, neg1,
            scale, sub,
        },
        types::{FAT_RAIL, GrindPrim, IMPOSSIBLE, LEDGE, THIN_RAIL},
        view::{GrindAttributes, GrindCvars, GrindError, error, lanes, root},
    },
    physics::PointGraph,
    reckoning::air::try_clamp_air_angle,
    reckoning_numeric::{
        ReckoningNumeric, length_from_squared_and_estimate, try_normalize_safe, try_signed_angle,
        try_unsigned_angle, wrap_turns_to_radians,
    },
    skateboard_deck::{EXTENT, LENGTH},
    skateboard_foot_attributes::wallride::WallRideTrajectoryAttributes,
};

/// `__real_426fffff`, the frame scale of the `ConsiderGrindPrimitive` index.
const FRAMES_PER_SECOND: f32 = f32::from_bits(0x426f_ffff);
/// `__real_38d1b717`.
const TINY: f32 = f32::from_bits(0x38d1_b717);
/// `g2PIReciprocal`.
const TWO_PI_RECIPROCAL: f32 = f32::from_bits(0x3e22_f983);
/// `Math::VECFLOAT_SMALL` (initializer 0x82f76340, `SMALL_FLOAT` 2^-16).
const VECFLOAT_SMALL: f32 = f32::from_bits(0x3780_0000);
/// Unnamed eight-point `physics_trajectory` graph of `FindBestGrind`: the
/// landing-angle gain a centre-locked candidate needs against its height gain.
const LOCKED_ANGLE_GAIN: u64 = 0x7dfc_d6f1_d65b_425c;

/// `ProcessedPhysIn` fields the chain reads: the `mDeckTransform`
/// translation +70, and the centre of mass +250 that only ledges read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SelectorGrindProcessed {
    pub(crate) deck_translation: V4,
    pub(crate) centre_of_mass: Option<V4>,
}

/// The chain's collaborators: `GrindData::Implementation::BoxTest` with
/// vertical results kept, the island batch of
/// `GrindGeometryAnalyzer::DoGeometryTest`, and the `VGlobalAttributeCollection`
/// instances +0x94 (`physicsdeck`), +0xa4 (`physics_grinds`) and +0xe8
/// (`physics_trajectory`).
pub(crate) struct SelectorGrindWorld<'a, B> {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) mod_grind_lock: Option<f32>,
    pub(crate) box_test:
        &'a mut dyn FnMut(OctreeBounds, &mut GrindDataQueryResult) -> Result<bool, GrindError>,
    pub(crate) lines: &'a mut B,
    pub(crate) grinds: &'a GrindAttributes,
    pub(crate) trajectory: &'a WallRideTrajectoryAttributes,
    /// The captured `physics_trajectory` layout.
    pub(crate) layout: Option<&'a AttributeRuntimeData>,
    pub(crate) deck: &'a AttributeRuntimeInstance,
    pub(crate) cvars: GrindCvars<'a>,
    pub(crate) overlay: &'a DebugOverlaySlot,
    pub(crate) processed: SelectorGrindProcessed,
}
impl<B> SelectorGrindWorld<'_, B> {
    fn layout(&self, offset: usize) -> f32 {
        #[cfg(feature = "skate3-runtime")]
        if offset == 0x218 { if let Some(distance) = self.mod_grind_lock { return distance; } }
        layout_word(self.layout, offset)
    }
    /// `GetAttributePointer` on `physicsdeck`; null reads `DefaultDataArea`.
    fn deck_scalar(&self, key: u64) -> Result<f32, GrindError> {
        let Some(data) = self.deck.try_get_data(key, 0).map_err(error)? else {
            return Ok(0.0);
        };
        let bytes: [u8; 4] = data
            .bytes()
            .get(..4)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| GrindError(format!("deck attribute {key:016x} is not a scalar")))?;
        Ok(f32::from_be_bytes(bytes))
    }
    /// `GetAttributePointer` then the 0x50-byte copy (`DefaultDataArea` when
    /// null) of an eight-point graph: X at +0x10, Y at +0x30.
    fn keyed_graph(&self, key: u64) -> Result<PointGraph<8>, GrindError> {
        let Some(data) = self.trajectory.try_data(key).map_err(error)? else {
            return Ok(PointGraph {
                x: [0.0; 8],
                y: [0.0; 8],
            });
        };
        let bytes = data.bytes();
        if bytes.len() < 0x50 {
            return Err(GrindError(format!(
                "trajectory graph {key:016x} has {} bytes",
                bytes.len()
            )));
        }
        let word = |offset: usize| {
            f32::from_be_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("eight-point graph"),
            )
        };
        Ok(PointGraph {
            x: std::array::from_fn(|i| word(0x10 + i * 4)),
            y: std::array::from_fn(|i| word(0x30 + i * 4)),
        })
    }
    fn debug(&self, cvar: C) -> Result<(), GrindError> {
        if self.cvars.b(cvar)? {
            self.overlay.require().map_err(error)?;
        }
        Ok(())
    }
    /// `kShowTrajectorySelect`.
    fn debug_select(&self) -> Result<(), GrindError> {
        if self.cvars.0.try_show_trajectory_select().map_err(error)? {
            self.overlay.require().map_err(error)?;
        }
        Ok(())
    }
    fn debug_times(&self, count: usize) -> Result<(), GrindError> {
        for _ in 0..count {
            self.overlay.require().map_err(error)?;
        }
        Ok(())
    }
}

fn layout_word(layout: Option<&AttributeRuntimeData>, offset: usize) -> f32 {
    layout.map_or(0.0, |data| {
        f32::from_be_bytes(
            data.bytes()[offset..offset + 4]
                .try_into()
                .expect("captured trajectory layout"),
        )
    })
}
/// `PointGraphEval::Evaluate` 0x824e2c40 on the eight-point layout graph at
/// `base`: X at +0x10, Y at +0x30.
fn layout_graph(layout: Option<&AttributeRuntimeData>, base: usize, x: f32) -> f32 {
    PointGraph::<8> {
        x: std::array::from_fn(|i| layout_word(layout, base + 0x10 + i * 4)),
        y: std::array::from_fn(|i| layout_word(layout, base + 0x30 + i * 4)),
    }
    .evaluate(x)
}

/// `vmaxfp`: the first NaN operand, quieted; +0 over -0.
pub(crate) fn vmax(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        f32::from_bits(a.to_bits() | 0x0040_0000)
    } else if b.is_nan() {
        f32::from_bits(b.to_bits() | 0x0040_0000)
    } else if a > b || (a == b && b.is_sign_negative()) {
        a
    } else {
        b
    }
}
/// `vminfp`: the first NaN operand, quieted; -0 under +0.
pub(crate) fn vmin(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        f32::from_bits(a.to_bits() | 0x0040_0000)
    } else if b.is_nan() {
        f32::from_bits(b.to_bits() | 0x0040_0000)
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}
/// `vcmpgefp.` of the result time against zero, all four lanes.
fn time_is_valid(r: &RetainedPredictionResult) -> bool {
    r.time.iter().all(|t| *t >= 0.0)
}

/// `GrindData::TrajectoryBoxTest` 0x82cf9cf0: the box query, then a primitive
/// with a parallel partner inside the pair window survives only as the pair's
/// lower rail nearer the takeoff.
#[expect(
    clippy::too_many_arguments,
    reason = "the native query, its cvars and overlay"
)]
pub(crate) fn trajectory_box_test(
    box_test: &mut dyn FnMut(OctreeBounds, &mut GrindDataQueryResult) -> Result<bool, GrindError>,
    cvars: &GrindCvars<'_>,
    overlay: &DebugOverlaySlot,
    minimum: V4,
    maximum: V4,
    takeoff: V4,
    out: &mut GrindDataQueryResult,
    num: &mut GrindNum<'_>,
) -> Result<bool, GrindError> {
    let mut local = GrindDataQueryResult::new();
    box_test(OctreeBounds { minimum, maximum }, &mut local)?;
    out.clear_count();
    let prims: Vec<GrindDataPrimitive> = local.results().cloned().collect();
    let count = prims.len();
    let mut dirs = Vec::with_capacity(count);
    for prim in &prims {
        dirs.push(num.normalize_unsafe(sub(prim.end, prim.start))?);
    }
    // Rows are filled below the diagonal with the row direction first, then
    // mirrored; the diagonal is never read.
    let mut parallel = vec![0.0_f32; count * count];
    for i in 0..count {
        for j in 0..i {
            let c = cross(dirs[i], dirs[j]);
            parallel[i * count + j] = num.dot(c, c)?;
        }
    }
    for i in 0..count {
        for j in i + 1..count {
            parallel[i * count + j] = parallel[j * count + i];
        }
    }
    for i in 0..count {
        let mut keep = true;
        for j in 0..count {
            if i == j
                || parallel[i * count + j].partial_cmp(&TINY) != Some(std::cmp::Ordering::Less)
            {
                continue;
            }
            let start_i = prims[i].start;
            let start_j = prims[j].start;
            let offset = sub(start_j, start_i);
            let minimum_pair = cvars.f(C::kMinPairDist)?;
            let along = num.dot(dirs[j], offset)?;
            let across = sub(offset, scale(dirs[j], along));
            let separation = num.dot(across, across)?;
            let paired = separation > minimum_pair * minimum_pair && {
                let maximum_pair = cvars.f(C::kMaxPairDist)?;
                separation < maximum_pair * maximum_pair
            };
            if !paired {
                continue;
            }
            let middle = scale(add(start_i, prims[i].end), 0.5);
            let to_start = sub(start_j, middle);
            let to_end = sub(prims[j].end, middle);
            if !(0.0 > num.dot(to_start, to_end)?) {
                continue;
            }
            let along_j = num.dot(dirs[j], to_start)?;
            let from_takeoff = sub(middle, takeoff);
            let foot = sub(to_start, scale(dirs[j], along_j));
            let middle_distance = num.dot(from_takeoff, from_takeoff)?;
            let side = cross(dirs[i], J);
            let closest = add(middle, foot);
            let to_closest = sub(closest, takeoff);
            let closest_distance = num.dot(to_closest, to_closest)?;
            let up = cross(side, dirs[i]);
            let up = try_normalize_safe(up, ZERO, num.n).map_err(error)?;
            let maximum_rise = cvars.f(C::kMaxPairDistY)?;
            let rise = num.dot(offset, up)?;
            if (rise > neg1(maximum_rise) && middle_distance > closest_distance)
                || rise > maximum_rise
            {
                keep = false;
                if cvars.b(C::kDrawGrindPairDetection)? {
                    overlay.require().map_err(error)?;
                }
            }
        }
        if keep {
            out.try_append(prims[i].clone());
        }
    }
    if cvars.b(C::kDrawGrindPairDetection)? {
        for _ in 0..out.count() {
            overlay.require().map_err(error)?;
        }
    }
    Ok(out.count() != 0)
}

/// `tTrajectory::CalcGreatestTimeToIntersectPlane` 0x82db7098 over
/// `_TrajectoryPlaneIntersection` 0x82db54e0: `n.(p0 + v0 t + g t^2/2 - P) = 0`;
/// the later of two roots, or a single root only when positive.
pub(crate) fn greatest_time_to_intersect_plane(
    trajectory: &RetainedTrajectory,
    point: V4,
    normal: V4,
    num: &mut GrindNum<'_>,
) -> Result<Option<f32>, GrindError> {
    let p0 = lanes(trajectory.position);
    let v0 = lanes(trajectory.velocity);
    let g = lanes(trajectory.gravity);
    let plane = num.dot(point, normal)?;
    let origin = num.dot(normal, p0)?;
    let half = num.reciprocal(2.0)?;
    let c = origin - plane;
    let b = num.dot(normal, v0)?;
    let a = num.dot(normal, scale(g, half))?;
    let discriminant = b * b - (4.0 * a) * c;
    if 0.0 > discriminant {
        return Ok(None);
    }
    let inverse = num.reciprocal(2.0 * a)?;
    if discriminant > 0.0 {
        let seed = num
            .n
            .reciprocal_sqrt_estimate(discriminant)
            .map_err(error)?;
        let root = length_from_squared_and_estimate(discriminant, seed);
        let later = inverse * (neg1(b) + root);
        let earlier = inverse * (neg1(b) - root);
        return Ok(Some(vmax(later, earlier)));
    }
    let only = inverse * neg1(b);
    Ok((only > 0.0).then_some(only))
}

/// `PhysicsUtility::ClosestAngleToXAxis` 0x82e48f10.
fn closest_angle_to_x_axis(angle: f32) -> f32 {
    let wrapped = wrap_turns_to_radians(angle * TWO_PI_RECIPROCAL);
    let sign = if wrapped > 0.0 { 1.0 } else { -1.0 };
    let mut magnitude = wrapped * sign;
    if magnitude > HALF_PI {
        magnitude -= PI;
    }
    sign * magnitude
}

/// `PhysicsUtility::CalcClosestPointOnLine` 0x82e48fb8: the projection onto
/// the segment, clamped to it; a segment no longer than `VECFLOAT_SMALL` keeps
/// its unnormalized direction.
pub(crate) fn closest_point_on_line(
    position: V4,
    start: V4,
    end: V4,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let delta = sub(end, start);
    let length = num.length(delta)?;
    let direction = if length > VECFLOAT_SMALL {
        scale(delta, num.reciprocal(length)?)
    } else {
        delta
    };
    let along = num.dot(direction, sub(position, start))?;
    let t = vmax(vmin(along, length), 0.0);
    Ok(madd(direction, t, start))
}

/// `PhysicsUtility::TrajLinePlaneIntersectTest` 0x82e49c40 outputs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TrajectoryLineHit {
    pub(crate) point_on_trajectory: V4,
    pub(crate) point_on_line: V4,
    pub(crate) line_normal: V4,
    pub(crate) distance: f32,
    pub(crate) time: f32,
}

/// `PhysicsUtility::TrajLinePlaneIntersectTest` 0x82e49c40: the trajectory's
/// latest crossing of the plane through the segment start whose normal is the
/// segment's upward perpendicular (J when degenerate), and its distance from
/// the segment.
pub(crate) fn traj_line_plane_intersect_test(
    trajectory: &RetainedTrajectory,
    start: V4,
    end: V4,
    num: &mut GrindNum<'_>,
) -> Result<Option<TrajectoryLineHit>, GrindError> {
    let delta = sub(end, start);
    let side = cross(delta, J);
    let normal = try_normalize_safe(cross(side, delta), J, num.n).map_err(error)?;
    let Some(t) = greatest_time_to_intersect_plane(trajectory, start, normal, num)? else {
        return Ok(None);
    };
    let p0 = lanes(trajectory.position);
    let v0 = lanes(trajectory.velocity);
    let g = lanes(trajectory.gravity);
    let linear: V4 = std::array::from_fn(|i| v0[i].mul_add(t, p0[i]));
    let point_on_trajectory: V4 = std::array::from_fn(|i| (g[i] * 0.5).mul_add(t * t, linear[i]));
    let point_on_line = closest_point_on_line(point_on_trajectory, start, end, num)?;
    let distance = num.length(sub(point_on_trajectory, point_on_line))?;
    Ok(Some(TrajectoryLineHit {
        point_on_trajectory,
        point_on_line,
        line_normal: normal,
        distance,
        time: t,
    }))
}

/// `ConsiderGrindPrimitive` 0x82dba150: where the prediction crosses the
/// plane raised above the rail by its radius plus the layout+0x204 offset.
fn consider_grind_primitive<B>(
    r: &RetainedPredictionResult,
    prim: &GrindDataPrimitive,
    world: &SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<Option<RetainedGrindTrajectoryResult>, GrindError> {
    let start = prim.start;
    let end = prim.end;
    let delta = sub(end, start);
    let side = cross(J, delta);
    let length = num.length(delta)?;
    let up = num.normalize_unsafe(cross(delta, side))?;
    // `ClampLength(delta, VECFLOAT_ONE)`.
    let direction = if length > 1.0 {
        scale(delta, num.reciprocal(length)? * 1.0)
    } else {
        delta
    };
    let offset = madd(up, r.radius, scale(up, world.layout(0x204)));
    let Some(t) = greatest_time_to_intersect_plane(&r.trajectory, add(start, offset), up, num)?
    else {
        return Ok(None);
    };
    let p0 = lanes(r.trajectory.position);
    let v0 = lanes(r.trajectory.velocity);
    let g = lanes(r.trajectory.gravity);
    let normal_speed = num.dot(up, v0)?;
    let linear: V4 = std::array::from_fn(|i| v0[i].mul_add(t, p0[i]));
    let crossing: V4 = std::array::from_fn(|i| (g[i] * 0.5).mul_add(t * t, linear[i]));
    let in_plane = sub(v0, scale(up, normal_speed));
    let collision = sub(crossing, offset);
    let unit = try_normalize_safe(direction, ZERO, num.n).map_err(error)?;
    let relative = sub(collision, start);
    let margin = scale(direction, 0.1);
    let along = num.dot(unit, relative)?;
    let extended_end = add(end, margin);
    let extended_start = sub(start, margin);
    let closest: V4 = std::array::from_fn(|i| unit[i].mul_add(along, start[i]));
    let bracket = num.dot(sub(closest, extended_start), sub(closest, extended_end))?;
    let distance = num.length(cross(relative, unit))?;
    if !(0.0 > bracket) {
        return Ok(None);
    }
    let penetration_index = num.n.truncate_i32(t * FRAMES_PER_SECOND).map_err(error)?;
    let direction_in_plane = try_normalize_safe(in_plane, ZERO, num.n).map_err(error)?;
    let mut approach = sub(closest, p0);
    approach[1] = 0.0;
    let angle = try_signed_angle(approach, unit, up, num.n).map_err(error)?;
    let landing_angle = abs1(closest_angle_to_x_axis(angle));
    if world.cvars.b(C::kShowGrindInfo)? {
        world.debug_times(3)?;
    }
    Ok(Some(RetainedGrindTrajectoryResult {
        closest_point: closest,
        collision_point: collision,
        grind_direction: unit,
        trajectory_direction_in_plane: direction_in_plane,
        distance_to_grind: distance,
        penetration_time: t,
        landing_angle,
        penetration_index,
        grind_index: 0,
        valid: true,
    }))
}

/// `CalculateGrindTrajData` 0x82dc4ec0: one list entry per accepted primitive.
fn calculate_grind_traj_data<B>(
    s: &mut OnboardTrajectoryOwner,
    index: usize,
    world: &SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<(), GrindError> {
    s.grind_trajectory_results.clear();
    let prims: Vec<GrindDataPrimitive> = s.grind_test_results.results().cloned().collect();
    s.num_grind_results = i32::try_from(prims.len()).map_err(error)?;
    let r = s.queries[index].result;
    for (i, prim) in prims.iter().enumerate() {
        let considered = consider_grind_primitive(&r, prim, world, num)?;
        if world.cvars.b(C::kShowGrindInfo)? {
            world.debug_times(2)?;
        }
        if let Some(mut result) = considered {
            if s.grind_trajectory_results.len() == 40 {
                return Err(GrindError("GrindTrajectoryResults list capacity".into()));
            }
            result.grind_index = i32::try_from(i).map_err(error)?;
            s.grind_trajectory_results.push(result);
        }
    }
    Ok(())
}

/// `FindBestGrind` 0x82dc2d08: the nearest candidate outside the centre-lock
/// distance, unless a centre-locked one gains enough landing angle for its
/// height; the chosen entry leaves the list.
fn find_best_grind<B>(
    s: &mut OnboardTrajectoryOwner,
    out: &mut RetainedGrindTrajectoryResult,
    world: &SelectorGrindWorld<'_, B>,
) -> Result<bool, GrindError> {
    let lock = world.layout(0x218);
    if s.grind_trajectory_results.is_empty() {
        return Ok(false);
    }
    let gain_graph = world.keyed_graph(LOCKED_ANGLE_GAIN)?;
    let mut best_distance = 1000.0_f32;
    let mut best_angle = PI;
    let mut best_height = -10_000.0_f32;
    let mut best = None;
    for (index, candidate) in s.grind_trajectory_results.iter().enumerate() {
        if !candidate.valid {
            return Err(GrindError(
                "GrindTrajectoryResults mIsValid assertion".into(),
            ));
        }
        let distance = candidate.distance_to_grind;
        if distance < lock {
            let gain = best_angle - candidate.landing_angle;
            let rise = candidate.closest_point[1] - best_height;
            if gain > gain_graph.evaluate(rise) {
                best_angle = candidate.landing_angle;
                best_distance = -1.0;
                best_height = candidate.closest_point[1];
                best = Some(index);
            }
        } else if distance < best_distance {
            best_distance = distance;
            best = Some(index);
        }
    }
    let Some(index) = best else {
        out.valid = false;
        return Ok(false);
    };
    *out = s.grind_trajectory_results.remove(index);
    Ok(true)
}

/// `GrindPrediction::FindBestGrindUpVectorOnLanding` 0x82cfca88; its landing
/// velocity and takeoff up arguments are unused. An impossible geometry
/// leaves the result unwritten.
fn find_best_grind_up_vector_on_landing<B: GeometryLineBatch>(
    prim: &GrindDataPrimitive,
    grind_position: V4,
    orientations: &mut UnwrittenGrindOrientations,
    world: &mut SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<Option<V4>, GrindError> {
    let geometry = calc_grind_geometry(
        &GrindPrim::from_query(prim),
        grind_position,
        world.grinds,
        &world.cvars,
        world.lines,
        num,
    )?;
    if geometry.geometry_type == IMPOSSIBLE {
        orientations.is_garbage = Some(true);
        orientations.geometry_type = Some(IMPOSSIBLE);
        return Ok(None);
    }
    if geometry.uncertain {
        orientations.is_garbage = Some(true);
        orientations.boardslide_direction = Some(I);
        orientations.geometry_type = Some(THIN_RAIL);
        orientations.high_side_direction = Some(I);
        orientations.tipslide_direction = Some(I);
        orientations.backslash_direction = Some(neg(I));
        return Ok(Some(J));
    }
    let result = calc_tilted_upmost_normal(&geometry, num)?;
    orientations.is_garbage = Some(false);
    orientations.geometry_type = Some(geometry.geometry_type);
    orientations.high_side_direction = Some(geometry.high_side_dir);
    // `GetTipslideTiltRad` / `GetBackslashTiltRad`.
    let tipslide_tilt = world.cvars.f(C::kTipslideTiltAngleDeg)? * DEG_TO_RAD;
    let backslash_tilt = world.cvars.f(C::kBackslashTiltAngleDeg)? * DEG_TO_RAD;
    let upmost = geometry.upmost_normal;
    let side = cross(upmost, geometry.primitive_dir);
    if geometry.geometry_type == THIN_RAIL {
        orientations.boardslide_direction = Some(side);
        let takeoff = world.processed.deck_translation;
        let toward = if num.dot(side, sub(takeoff, grind_position))? > 0.0 {
            side
        } else {
            neg(side)
        };
        let axis = cross(toward, upmost);
        orientations.tipslide_direction = Some(num.rotate(axis, tipslide_tilt, toward)?);
        orientations.backslash_direction = Some(num.rotate(axis, PI - backslash_tilt, toward)?);
        return Ok(Some(result));
    }
    let across = cross(side, upmost);
    orientations.boardslide_direction = Some(if geometry.geometry_type == FAT_RAIL {
        side
    } else {
        cross(result, geometry.primitive_dir)
    });
    let sign = if num.dot(side, geometry.high_side_dir)? > 0.0 {
        1.0
    } else {
        -1.0
    };
    orientations.backslash_direction =
        Some(num.rotate(scale(across, sign), backslash_tilt, scale(side, sign))?);
    let flipped = -sign;
    orientations.tipslide_direction =
        Some(num.rotate(scale(across, flipped), tipslide_tilt, scale(side, flipped))?);
    Ok(Some(result))
}

/// `CalculateGrindLandingNormal` 0x82dba758: up leaned against the velocity
/// across the rail, clamped within the layout+0x230 range of the landing up.
fn calculate_grind_landing_normal<B>(
    target: V4,
    direction: V4,
    velocity: V4,
    world: &SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let along = num.dot(direction, velocity)?;
    let across = neg(sub(velocity, scale(direction, along)));
    let lean = madd(across, world.layout(0x22c), J);
    let range = world.layout(0x230) * DEG_TO_RAD;
    let clamped =
        try_clamp_air_angle(root(lean), root(target), range, num.n, num.s).map_err(error)?;
    let normal = try_normalize_safe(lanes(clamped), ZERO, num.n).map_err(error)?;
    if world.cvars.b(C::kRenderGrindLandingUp)? {
        world.debug_times(4)?;
    }
    Ok(normal)
}

/// `CalculateGrindVerticalNormal` 0x82dba638: world up made perpendicular to
/// the rail and pointed up; up is the degenerate fallback.
fn calculate_grind_vertical_normal(
    direction: V4,
    num: &mut GrindNum<'_>,
) -> Result<V4, GrindError> {
    let across = cross(J, direction);
    let vertical = cross(direction, across);
    let sign = if 0.0 > vertical[1] { -1.0 } else { 1.0 };
    try_normalize_safe(scale(vertical, sign), J, num.n).map_err(error)
}

/// `AnalyzeAndAdjustTrajectory` 0x82dc5160: take candidates best first until
/// one passes the approach, geometry, centre-lock and redirect limits, then
/// bend the prediction onto it.
#[expect(clippy::too_many_lines, reason = "one native candidate loop")]
fn analyze_and_adjust_trajectory<B: GeometryLineBatch>(
    s: &mut OnboardTrajectoryOwner,
    index: usize,
    landing_velocity: V4,
    world: &mut SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<bool, GrindError> {
    if !time_is_valid(&s.queries[index].result) {
        return Err(GrindError(
            "AnalyzeAndAdjustTrajectory time assertion".into(),
        ));
    }
    calculate_grind_traj_data(s, index, world, num)?;
    let vertical_limit = world.layout(0x210);
    let ledge_lateral_limit = world.layout(0x208);
    let lateral_limit = world.layout(0x20c);
    let lock = world.layout(0x218);
    let lock_margin = world.layout(0x214);
    let low_behind = world.layout(0x228);
    let low_front = world.layout(0x224);
    let high_behind = world.layout(0x220);
    let high_front = world.layout(0x21c);
    let mut best = RetainedGrindTrajectoryResult {
        closest_point: ZERO,
        collision_point: ZERO,
        grind_direction: ZERO,
        trajectory_direction_in_plane: ZERO,
        distance_to_grind: 0.0,
        penetration_time: 0.0,
        landing_angle: 0.0,
        penetration_index: 0,
        grind_index: 0,
        valid: false,
    };
    if !find_best_grind(s, &mut best, world)? {
        return Ok(false);
    }
    let lock_distance = lock_margin + lock;
    loop {
        let prim = usize::try_from(best.grind_index)
            .ok()
            .and_then(|i| s.grind_test_results.results().nth(i))
            .cloned()
            .ok_or_else(|| GrindError("GrindDataQueryResult index assertion".into()))?;
        world.debug(C::kShowGrindInfo)?;
        s.grind_primitive.start = prim.start;
        s.grind_primitive.end = prim.end;
        let offset_from_rail = sub(best.collision_point, best.closest_point);
        let rail = try_normalize_safe(sub(prim.end, prim.start), ZERO, num.n).map_err(error)?;
        let along = num.dot(rail, landing_velocity)?;
        let rail_rise = abs1(rail[1]);
        let across = sub(landing_velocity, scale(rail, along));
        let mut lateral = across;
        lateral[1] = 0.0;
        let lateral_squared = num.dot(lateral, lateral)?;
        let vertical = across[1] * (1.0 - rail_rise);
        let accepted = 'candidate: {
            if abs1(vertical) > vertical_limit {
                world.debug_select()?;
                break 'candidate false;
            }
            world.debug_select()?;
            let orientations = s
                .possible_orientations
                .as_deref_mut()
                .ok_or_else(|| GrindError("PossibleGrindOrientations assertion".into()))?;
            let up = find_best_grind_up_vector_on_landing(
                &prim,
                best.closest_point,
                orientations,
                world,
                num,
            )?;
            let geometry_type = orientations
                .geometry_type
                .ok_or_else(|| GrindError("unwritten grind geometry type".into()))?;
            if geometry_type == IMPOSSIBLE {
                break 'candidate false;
            }
            let limit = if geometry_type == LEDGE {
                ledge_lateral_limit
            } else {
                lateral_limit
            };
            if lateral_squared > limit {
                world.debug_select()?;
                break 'candidate false;
            }
            let mut locked = lock_distance;
            if geometry_type == LEDGE {
                let high = orientations
                    .high_side_direction
                    .ok_or_else(|| GrindError("unwritten high side direction".into()))?;
                let centre_of_mass = world
                    .processed
                    .centre_of_mass
                    .ok_or_else(|| GrindError("missing Processed centre of mass".into()))?;
                let toward = num.dot(offset_from_rail, high)?;
                let body = num.dot(sub(centre_of_mass, prim.start), high)?;
                locked = if toward > 0.0 {
                    (if body > 0.0 { high_front } else { high_behind }) * lock
                } else {
                    (if body > 0.0 { low_front } else { low_behind }).mul_add(lock, lock_margin)
                };
                world.debug(C::kShowGrindInfo)?;
            }
            if !(best.distance_to_grind < locked) {
                break 'candidate false;
            }
            let relative = sub(best.collision_point, prim.start);
            let rail_along = num.dot(best.grind_direction, relative)?;
            let snap = neg(sub(relative, scale(best.grind_direction, rail_along)));
            let snap_length = num.length(snap)?;
            let half = world.deck_scalar(LENGTH)? * 0.5;
            let reach = world.deck_scalar(EXTENT)?.mul_add(0.5, half);
            let snap_scale = if snap_length < lock {
                1.0
            } else {
                if snap_length == 0.0 {
                    return Err(GrindError("grind snap length assertion".into()));
                }
                neg1(world.layout(0x200)).mul_add(reach, snap_length) / snap_length
            };
            let snap = scale(snap, snap_scale);
            let old_velocity = lanes(s.queries[index].result.trajectory.velocity);
            let old_squared = num.dot(old_velocity, old_velocity)?;
            let maximum_angle = world.layout(0x238) * DEG_TO_RAD;
            let inverse = num.reciprocal(best.penetration_time)?;
            let new_velocity: V4 =
                std::array::from_fn(|i| inverse.mul_add(snap[i], old_velocity[i]));
            let new_squared = num.dot(new_velocity, new_velocity)?;
            let angle = if old_squared * new_squared > TINY {
                try_unsigned_angle(old_velocity, new_velocity, num.n).map_err(error)?
            } else {
                0.0
            };
            if angle > maximum_angle {
                break 'candidate false;
            }
            // `TrajectorySelector::AdjustTrajectory` 0x82dba940.
            adjust::<(), _>(
                &mut s.queries[index].result.trajectory,
                best.penetration_index,
                root(snap),
                world.layout(0x1f4),
                num.n,
            )
            .map_err(error)?;
            s.queries[index].result.frame = best.penetration_index;
            #[expect(clippy::cast_precision_loss, reason = "native signed fcfid then frsp")]
            let frame_time = best.penetration_index as f32 * DT;
            s.queries[index].result.time = [frame_time; 4];
            let target = up.ok_or_else(|| GrindError("unwritten landing up vector".into()))?;
            let normal = calculate_grind_landing_normal(
                target,
                best.grind_direction,
                old_velocity,
                world,
                num,
            )?;
            s.queries[index].result.normal = root(normal);
            s.most_vertical_grind_normal =
                calculate_grind_vertical_normal(best.grind_direction, num)?;
            s.most_vertical_grind_normal_valid = true;
            if world.cvars.b(C::kRenderGrindLandings)? {
                let garbage = s
                    .possible_orientations
                    .as_deref()
                    .and_then(|o| o.is_garbage)
                    .ok_or_else(|| GrindError("unwritten orientation garbage flag".into()))?;
                if !garbage {
                    world.debug_times(3)?;
                }
            }
            world.debug(C::kShowGrindInfo)?;
            true
        };
        if accepted {
            return Ok(true);
        }
        world.debug(C::kShowGrindInfo)?;
        if !find_best_grind(s, &mut best, world)? {
            return Ok(false);
        }
    }
}

/// `CalculateGrindScore` 0x82dc5c38 for the selector's candidate `index`.
pub(crate) fn calculate_grind_score<B: GeometryLineBatch>(
    s: &mut OnboardTrajectoryOwner,
    index: usize,
    landing_velocity: V4,
    world: &mut SelectorGrindWorld<'_, B>,
    num: &mut GrindNum<'_>,
) -> Result<f32, GrindError> {
    if !time_is_valid(&s.queries[index].result) {
        return Err(GrindError("CalculateGrindScore time assertion".into()));
    }
    let landing = lanes(collision_position::<(), ()>(&s.queries[index].result).map_err(error)?);
    let half = world.layout(0x234);
    let extent = [half, half, half, 0.0];
    s.grind_test_results_valid = false;
    let takeoff = world.processed.deck_translation;
    trajectory_box_test(
        world.box_test,
        &world.cvars,
        world.overlay,
        sub(landing, extent),
        add(landing, extent),
        takeoff,
        &mut s.grind_test_results,
        num,
    )?;
    if s.grind_test_results.count() == 0 {
        return Ok(0.0);
    }
    s.grind_test_results_valid = true;
    if !analyze_and_adjust_trajectory(s, index, landing_velocity, world, num)? {
        return Ok(0.0);
    }
    s.grind_bonus_awarded = true;
    Ok(world.layout(0x1e0))
}

/// `CalculateGrindPenalty` 0x82db9ee0: the nearest tested rail through the
/// layout+0xa0 graph, never positive.
pub(crate) fn calculate_grind_penalty<N: ReckoningNumeric>(
    s: &OnboardTrajectoryOwner,
    index: usize,
    layout: Option<&AttributeRuntimeData>,
    n: &mut N,
) -> Result<f32, GrindError>
where
    N::Error: std::fmt::Debug,
{
    let mut distance = 1000.0;
    if s.grind_test_results_valid {
        let mut nearest = 1_000_000.0_f32;
        let r = &s.queries[index].result;
        for prim in s.grind_test_results.results() {
            let direction =
                try_normalize_safe(sub(prim.end, prim.start), ZERO, n).map_err(error)?;
            let point = if time_is_valid(r) {
                lanes(r.position)
            } else {
                ZERO
            };
            let from_start = sub(point, prim.start);
            let from_end = sub(point, prim.end);
            let along = n.dot3(direction, from_start).map_err(error)?;
            let start_squared = n.dot3(from_start, from_start).map_err(error)?;
            let end_squared = n.dot3(from_end, from_end).map_err(error)?;
            let off = sub(from_start, scale(direction, along));
            let off_squared = n.dot3(off, off).map_err(error)?;
            nearest = vmin(vmin(vmin(off_squared, start_squared), end_squared), nearest);
        }
        let seed = n.reciprocal_sqrt_estimate(nearest).map_err(error)?;
        distance = length_from_squared_and_estimate(nearest, seed);
    }
    let penalty = layout_graph(layout, 0xa0, distance);
    if penalty > 0.0 {
        return Err(GrindError("CalculateGrindPenalty sign assertion".into()));
    }
    Ok(penalty)
}
