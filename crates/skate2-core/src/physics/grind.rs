//! Runtime-owned grind rails reconstructed from authored Pegasus splines.

use crate::physics::{
    BallisticTrajectory, PHYSICS_STEP_SECONDS, PointGraph, TrajectorySelectionTuning,
};
use skate2_formats::pegasus::SplineData;
use std::f32::consts::PI;

/// `SMALL_FLOAT_150` at `0x82319320`, used by the reference constructor to
/// classify a spline segment whose endpoints have no meaningful XZ movement.
pub const VERTICAL_GRIND_HORIZONTAL_EPSILON: f32 = f32::from_bits(0x3780_0000);
pub const MAX_GRIND_QUERY_RESULTS: usize = 40;
pub const MAX_LIVING_WORLD_GRIND_SPLINES_PER_BODY: usize = 64;
pub const LIVING_WORLD_DYNAMIC_OBJECT_MAX_SIZE_DEFAULT: f32 = 15.0;
pub const LIVING_WORLD_VEHICLE_MAX_SIZE_DEFAULT: f32 = 15.0;
pub const GRIND_PAIR_PARALLEL_ERROR_SQUARED: f32 = 0.000_1;
pub const GRIND_REDIRECTION_MAGNITUDE_PRODUCT_EPSILON: f32 = 0.000_1;

/// Defaults installed by the four grind-pair cvar initializers at
/// `0x82f8a6b0..0x82f8a7d0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindPairFilterTuning {
    pub minimum_distance: f32,
    pub maximum_distance: f32,
    pub maximum_vertical_distance: f32,
}

impl Default for GrindPairFilterTuning {
    fn default() -> Self {
        Self {
            minimum_distance: 0.01,
            maximum_distance: 0.1,
            maximum_vertical_distance: 0.02,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindBounds {
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
}

impl GrindBounds {
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        (0..3).all(|axis| {
            self.minimum[axis] <= other.maximum[axis] && other.minimum[axis] <= self.maximum[axis]
        })
    }
}

/// Human-readable equivalent of the PDB-proven 48-byte `GrindPrimitive`.
/// Serialized pointers become stable source offsets and indices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindPrimitive {
    pub start_point: [f32; 3],
    pub end_point: [f32; 3],
    pub spline_header_offset: Option<u32>,
    pub is_vertical: bool,
    pub source_asset_id: u64,
    pub source_dictionary_index: u32,
    pub source_segment_index: usize,
}

/// Runtime-owned grind splines for one object resident in a living-world
/// manager. Vector order stands in for the reference manager's hash-table and
/// per-object spline-map iteration order.
#[derive(Clone, Debug, PartialEq)]
pub struct LivingWorldGrindBody {
    pub transform: [f32; 16],
    pub grind_assets: Vec<GrindAsset>,
}

/// Reference cvars installed by the initializers at `0x82f8c1b0`,
/// `0x82f8cff8`, and `0x82f8b5a8`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingWorldGrindTuning {
    pub feed_dynamic_object_splines: bool,
    pub maximum_dynamic_object_size: f32,
    pub maximum_vehicle_size: f32,
}

impl Default for LivingWorldGrindTuning {
    fn default() -> Self {
        Self {
            feed_dynamic_object_splines: true,
            maximum_dynamic_object_size: LIVING_WORLD_DYNAMIC_OBJECT_MAX_SIZE_DEFAULT,
            maximum_vehicle_size: LIVING_WORLD_VEHICLE_MAX_SIZE_DEFAULT,
        }
    }
}

/// Trace-fed values for a retail-code defect in both living-world providers.
/// They copy the result's high-bit flag from an uninitialized stack word once
/// per manager call. Keeping that input explicit makes captures reproducible;
/// the deterministic native default is clear (both bits unset).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LivingWorldLegacyGrindFlags {
    pub dynamic_objects_are_vertical: bool,
    pub vehicles_are_vertical: bool,
}

/// Native ownership boundary corresponding to the registered
/// `LivingWorld::Manager` grind provider. Only manager-resident bodies belong
/// in these vectors; removed/inactive objects are excluded by their owner.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LivingWorldGrindScene {
    pub tuning: LivingWorldGrindTuning,
    pub legacy_flags: LivingWorldLegacyGrindFlags,
    pub dynamic_objects: Vec<LivingWorldGrindBody>,
    pub vehicles: Vec<LivingWorldGrindBody>,
}

/// Applies the live body matrix used by the DMO/vehicle dynamic-grind
/// providers and recomputes verticality from the transformed endpoints.
#[must_use]
pub fn transform_dynamic_grind_primitive(
    primitive: GrindPrimitive,
    body_transform: [f32; 16],
) -> GrindPrimitive {
    let start_point = transform_affine_point(primitive.start_point, body_transform);
    let end_point = transform_affine_point(primitive.end_point, body_transform);
    let horizontal_delta = [end_point[0] - start_point[0], end_point[2] - start_point[2]];
    let is_vertical = !horizontal_delta
        .into_iter()
        .any(|component| component.abs() > VERTICAL_GRIND_HORIZONTAL_EPSILON);
    GrindPrimitive {
        start_point,
        end_point,
        is_vertical,
        ..primitive
    }
}

fn transform_affine_point(point: [f32; 3], transform: [f32; 16]) -> [f32; 3] {
    std::array::from_fn(|component| {
        transform[8 + component].mul_add(
            point[2],
            transform[4 + component].mul_add(
                point[1],
                transform[component].mul_add(point[0], transform[12 + component]),
            ),
        )
    })
}

/// Source-level form of the PDB-proven 96-byte
/// `TrajectorySelector::GrindTrajectoryResults` record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindTrajectoryResult {
    pub closest_point_on_primitive: [f32; 3],
    pub collision_point: [f32; 3],
    pub grind_direction: [f32; 3],
    pub trajectory_direction_in_grind_plane: [f32; 3],
    pub distance_to_grind: f32,
    pub grind_plane_penetration_time: f32,
    pub landing_angle: f32,
    pub grind_plane_penetration_index: i32,
    pub grind_index: usize,
}

/// PDB-proven `eGrindGeometryType` values used by trajectory acceptance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrindGeometryType {
    ThinRail,
    FatRail,
    Ledge,
    Impossible,
}

/// Velocity components constructed by `AnalyzeAndAdjustTrajectory` before its
/// physics-island grind-geometry query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindApproachMetrics {
    pub vertical_speed: f32,
    pub lateral_speed_squared: f32,
}

/// Geometry information returned by the still-separate physics-island grind
/// analyzer. Ledge lock distance depends on high/low-side board orientation;
/// ordinary rails use the trajectory tuning's centre-plus-tip distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrindGeometryAssessment {
    ThinRail,
    FatRail,
    Ledge { maximum_lock_distance: f32 },
    Impossible,
}

impl GrindGeometryAssessment {
    #[must_use]
    pub fn geometry_type(self) -> GrindGeometryType {
        match self {
            Self::ThinRail => GrindGeometryType::ThinRail,
            Self::FatRail => GrindGeometryType::FatRail,
            Self::Ledge { .. } => GrindGeometryType::Ledge,
            Self::Impossible => GrindGeometryType::Impossible,
        }
    }
}

/// Source-level subset of the PDB-proven 112-byte `GrindGeometry` produced by
/// `GrindGeometryAnalyzer::CalcGrindGeometry`. Surface classification and hit
/// positions remain with the island query; these are the fields consumed by
/// grind-orientation selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalyzedGrindGeometry {
    pub upmost_normal: [f32; 3],
    pub primitive_direction: [f32; 3],
    pub high_side_direction: [f32; 3],
    pub low_normal_limit_angle: f32,
    pub high_normal_limit_angle: f32,
    pub geometry_type: GrindGeometryType,
    pub uncertain: bool,
}

/// PDB-proven output fields of `PossibleGrindOrientations`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PossibleGrindOrientations {
    pub geometry_type: GrindGeometryType,
    pub is_garbage: bool,
    pub boardslide_direction: [f32; 3],
    pub tipslide_direction: [f32; 3],
    pub backslash_direction: [f32; 3],
    pub high_side_direction: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindLandingOrientation {
    pub selected_up_normal: [f32; 3],
    pub possible: PossibleGrindOrientations,
}

/// Defaults installed by the named grind-state cvar initializers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindOrientationTuning {
    pub tipslide_tilt_degrees: f32,
    pub backslash_tilt_degrees: f32,
}

impl Default for GrindOrientationTuning {
    fn default() -> Self {
        Self {
            tipslide_tilt_degrees: 20.0,
            backslash_tilt_degrees: 42.0,
        }
    }
}

/// Geometry-test values consumed by `GrindGeometryAnalyzer`. The collision
/// offset comes from `physics_grinds.default`; the remaining defaults are
/// installed by named cvar initializers in the executable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindGeometryTuning {
    pub collision_offset: f32,
    pub test_height_heuristic: f32,
    pub fine_distance: f32,
    pub fine_depth: f32,
    pub coarse_curb_parameter: f32,
    pub wheel_clear_angle_degrees: f32,
    pub curb_normal_likeness: f32,
    pub average_curb_height: f32,
    pub curb_height_range: f32,
    pub curb_high_side_range: f32,
}

impl Default for GrindGeometryTuning {
    fn default() -> Self {
        Self {
            collision_offset: 0.0,
            test_height_heuristic: 1.06,
            fine_distance: 0.09,
            fine_depth: 0.07,
            coarse_curb_parameter: 0.65,
            wheel_clear_angle_degrees: 16.0,
            curb_normal_likeness: 0.99,
            average_curb_height: 0.22,
            curb_height_range: 0.02,
            curb_high_side_range: 0.035,
        }
    }
}

/// Source-level form of the PDB-proven 64-byte
/// `PhysicsUtility::CollTestResult` used by the five geometry probes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindGeometryCollision {
    pub hit: bool,
    pub line_parameter: f32,
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub surface_id: u32,
}

/// `DoGeometryTest` requests `BatchFlags::BF_Default`: ground, dynamic-object,
/// and vehicle collision pools.
pub const GRIND_GEOMETRY_BATCH_FLAGS: u8 = 1 | 2 | 4;

/// Reconstructs `AggregateLineBatch::GetResult` for the five grind probes.
/// Pools must be supplied in island order (ground, DMO, then vehicle). A
/// strictly nearer hit replaces the current result, so equal parameters keep
/// the earlier pool's collision.
#[must_use]
pub fn merge_grind_geometry_collision_batches<'a>(
    batches: impl IntoIterator<Item = &'a [GrindGeometryCollision; 5]>,
) -> [GrindGeometryCollision; 5] {
    let mut merged = [GrindGeometryCollision::default(); 5];
    for batch in batches {
        for (current, candidate) in merged.iter_mut().zip(batch) {
            if candidate.hit && (!current.hit || candidate.line_parameter < current.line_parameter)
            {
                *current = *candidate;
            }
        }
    }
    merged
}

/// One of the five high-to-low lines submitted by `DoGeometryTest`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindGeometryProbeLine {
    pub high_point: [f32; 3],
    pub low_point: [f32; 3],
}

/// Lines are ordered by the PDB result enum: close one/two, far one/two,
/// centre. The two fallback points are the low endpoints of the far lines.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindGeometryProbeBatch {
    pub lines: [GrindGeometryProbeLine; 5],
    pub far_low_fallbacks: [[f32; 3]; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindGeometryProbeSetup {
    pub position: [f32; 3],
    pub upmost_normal: [f32; 3],
    pub primitive_direction: [f32; 3],
    pub high_side_direction: [f32; 3],
    pub batch: GrindGeometryProbeBatch,
}

/// The near-vertical escape in `CalcGrindGeometry` marks its result uncertain
/// and never gives the constructor's intentionally invalid type to consumers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UncertainGrindGeometry {
    pub line_positions: [[f32; 3]; 2],
    pub upmost_normal: [f32; 3],
    pub primitive_direction: [f32; 3],
    pub surface_info: GrindSurfaceInfo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreparedGrindGeometry {
    Query(GrindGeometryProbeSetup),
    Uncertain(UncertainGrindGeometry),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrindGeometryCalculation {
    Certain(CalculatedGrindGeometry),
    Uncertain(UncertainGrindGeometry),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalculatedGrindGeometry {
    pub line_positions: [[f32; 3]; 2],
    pub analysis: AnalyzedGrindGeometry,
    pub surface_info: GrindSurfaceInfo,
    pub is_stair: bool,
    pub is_apex: bool,
    pub is_curb: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindGeometryLandingAssessment {
    pub assessment: GrindGeometryAssessment,
    pub landing_orientation: GrindLandingOrientation,
    pub surface_info: GrindSurfaceInfo,
    pub is_stair: bool,
    pub is_apex: bool,
    pub is_curb: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrindGeometryHitClassification {
    pub geometry_type: GrindGeometryType,
    pub reverse_high_side_direction: bool,
    pub is_apex: bool,
}

/// PDB-proven eight-byte `GrindSurfaceInfo`. Raw enum values are retained so
/// every packed surface ID has the same lossless behavior as the reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GrindSurfaceInfo {
    pub audio_surface_raw: u8,
    pub physics_surface_raw: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcceptedGrindCandidate {
    pub trajectory_result: GrindTrajectoryResult,
    pub primitive: GrindPrimitive,
    pub geometry: GrindGeometryAssessment,
    pub approach: GrindApproachMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcceptedAnalyzedGrindCandidate {
    pub accepted: AcceptedGrindCandidate,
    pub geometry: GrindGeometryLandingAssessment,
}

/// Recovered trajectory, geometry, and orientation values consumed together
/// by the analyzer-backed grind-selection loop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindAdjustmentAnalysisTuning<'a> {
    pub trajectory: &'a TrajectorySelectionTuning,
    pub geometry: &'a GrindGeometryTuning,
    pub orientation: GrindOrientationTuning,
}

/// A grind candidate that survived every trajectory-selector gate, together
/// with the displacement passed to `tTrajectory::AdjustTrajectory`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindAdjustmentPlan {
    pub accepted_candidate: AcceptedGrindCandidate,
    pub snap_displacement: [f32; 3],
    pub velocity_redirection_angle: f32,
}

/// Final trajectory and collision outputs written after a grind adjustment is
/// accepted. The surrounding prediction owner stores these in its native
/// collision/result structures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AppliedGrindAdjustment {
    pub collision_frame: i32,
    pub collision_time: f32,
    pub landing_normal: [f32; 3],
    pub most_vertical_normal: [f32; 3],
}

/// Board footprint used when the trajectory selector aims a distant grind at
/// the deck tip. The reference derives this from two unnamed
/// `physicsdeck.default` scalar attributes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GrindDeckTuning {
    pub contact_footprint_radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GrindAsset {
    pub asset_id: u64,
    pub source_dictionary_index: u32,
    pub bounds: Option<GrindBounds>,
    pub primitives: Vec<GrindPrimitive>,
    primitive_bounds: Vec<GrindBounds>,
}

impl GrindAsset {
    /// Reconstructs `GrindData::Implementation::AssetRecord` at `0x82cfb640`
    /// and `GrindPrimitive::GrindPrimitive` at `0x82cfb538`.
    #[must_use]
    pub fn from_spline_data(
        asset_id: u64,
        source_dictionary_index: u32,
        data: &SplineData,
    ) -> Self {
        Self::from_segment_indices(
            asset_id,
            source_dictionary_index,
            data,
            0..data.segments.len(),
        )
    }

    /// Builds the one-header asset installed by `Vehicle::InitSplines` and
    /// `DynamicObject::InitSplines`. Their query loops walk from `m_Head`
    /// through `m_Tail` in contiguous 144-byte segment steps.
    #[must_use]
    pub fn from_spline_header(
        asset_id: u64,
        source_dictionary_index: u32,
        data: &SplineData,
        header_index: usize,
    ) -> Option<Self> {
        const SEGMENT_STRIDE: u32 = 144;
        let header = data.splines.get(header_index)?;
        let head_relative = header.head_offset?.checked_sub(data.segments_offset)?;
        let tail_relative = header.tail_offset?.checked_sub(data.segments_offset)?;
        if head_relative % SEGMENT_STRIDE != 0 || tail_relative % SEGMENT_STRIDE != 0 {
            return None;
        }
        let first = usize::try_from(head_relative / SEGMENT_STRIDE).ok()?;
        let last = usize::try_from(tail_relative / SEGMENT_STRIDE).ok()?;
        if first > last || last >= data.segments.len() {
            return None;
        }
        Some(Self::from_segment_indices(
            asset_id,
            source_dictionary_index,
            data,
            first..=last,
        ))
    }

    fn from_segment_indices(
        asset_id: u64,
        source_dictionary_index: u32,
        data: &SplineData,
        segment_indices: impl IntoIterator<Item = usize>,
    ) -> Self {
        let mut primitives = Vec::with_capacity(data.segments.len());
        let mut primitive_bounds = Vec::with_capacity(data.segments.len());
        let mut bounds: Option<GrindBounds> = None;

        for source_segment_index in segment_indices {
            let segment = &data.segments[source_segment_index];
            let coefficient = |vector: usize, axis: usize| segment.basis_matrix[vector * 4 + axis];
            // `SplineEval_Zero` returns D. `SplineEval_One` uses the exact VPU
            // grouping (C + B) + (A + D), which matters for f32 rounding.
            let start_point = std::array::from_fn(|axis| coefficient(3, axis));
            let end_point = std::array::from_fn(|axis| {
                (coefficient(2, axis) + coefficient(1, axis))
                    + (coefficient(0, axis) + coefficient(3, axis))
            });
            let horizontal_delta = [end_point[0] - start_point[0], end_point[2] - start_point[2]];
            let is_vertical = !horizontal_delta
                .into_iter()
                .any(|component| component.abs() > VERTICAL_GRIND_HORIZONTAL_EPSILON);
            let segment_bounds = GrindBounds {
                minimum: [
                    segment.bounding_box_min[0],
                    segment.bounding_box_min[1],
                    segment.bounding_box_min[2],
                ],
                maximum: [
                    segment.bounding_box_max[0],
                    segment.bounding_box_max[1],
                    segment.bounding_box_max[2],
                ],
            };
            bounds = Some(match bounds {
                Some(current) => GrindBounds {
                    minimum: std::array::from_fn(|axis| {
                        current.minimum[axis].min(segment_bounds.minimum[axis])
                    }),
                    maximum: std::array::from_fn(|axis| {
                        current.maximum[axis].max(segment_bounds.maximum[axis])
                    }),
                },
                None => segment_bounds,
            });
            primitive_bounds.push(segment_bounds);
            primitives.push(GrindPrimitive {
                start_point,
                end_point,
                spline_header_offset: segment.spline_offset,
                is_vertical,
                source_asset_id: asset_id,
                source_dictionary_index,
                source_segment_index,
            });
        }

        // The reference expands only the asset-level octree extent by one;
        // each inserted/query-tested segment retains its authored AABB.
        let bounds = bounds.map(|bounds| GrindBounds {
            minimum: std::array::from_fn(|axis| bounds.minimum[axis] - 1.0),
            maximum: std::array::from_fn(|axis| bounds.maximum[axis] + 1.0),
        });
        Self {
            asset_id,
            source_dictionary_index,
            bounds,
            primitives,
            primitive_bounds,
        }
    }

    fn append_box_results(
        &self,
        query: GrindBounds,
        ignore_vertical: bool,
        results: &mut Vec<GrindPrimitive>,
    ) {
        if self.bounds.is_none_or(|bounds| !bounds.overlaps(query)) {
            return;
        }
        for (primitive, bounds) in self.primitives.iter().zip(&self.primitive_bounds) {
            if results.len() == MAX_GRIND_QUERY_RESULTS {
                return;
            }
            if bounds.overlaps(query) && (!ignore_vertical || !primitive.is_vertical) {
                results.push(*primitive);
            }
        }
    }

    fn append_transformed_box_results(
        &self,
        query: GrindBounds,
        body_transform: [f32; 16],
        forced_vertical_flag: Option<bool>,
        results: &mut Vec<GrindPrimitive>,
    ) -> bool {
        for (primitive, bounds) in self.primitives.iter().zip(&self.primitive_bounds) {
            if transform_dynamic_grind_bounds(*bounds, body_transform).overlaps(query) {
                if results.len() == MAX_GRIND_QUERY_RESULTS {
                    return false;
                }
                let mut transformed = transform_dynamic_grind_primitive(*primitive, body_transform);
                if let Some(is_vertical) = forced_vertical_flag {
                    transformed.is_vertical = is_vertical;
                }
                results.push(transformed);
            }
        }
        true
    }
}

impl LivingWorldGrindBody {
    /// Reconstructs the fixed-map population performed from a model instance's
    /// embedded grind-spline pointer array. The retail map silently ignores
    /// entries after its 64-header capacity.
    #[must_use]
    pub fn from_spline_headers(
        transform: [f32; 16],
        asset_id: u64,
        source_dictionary_index: u32,
        data: &SplineData,
        header_indices: impl IntoIterator<Item = usize>,
    ) -> Self {
        let grind_assets = header_indices
            .into_iter()
            .take(MAX_LIVING_WORLD_GRIND_SPLINES_PER_BODY)
            .filter_map(|header_index| {
                GrindAsset::from_spline_header(
                    asset_id,
                    source_dictionary_index,
                    data,
                    header_index,
                )
            })
            .collect();
        Self {
            transform,
            grind_assets,
        }
    }
}

impl LivingWorldGrindScene {
    /// Appends the registered provider's DMO results followed by vehicle
    /// results. A provider stops at the first overlapping segment encountered
    /// after the shared result record has reached its 40-entry capacity.
    pub fn append_box_results(&self, query: GrindBounds, results: &mut Vec<GrindPrimitive>) {
        if self.tuning.feed_dynamic_object_splines {
            append_living_world_manager_results(
                &self.dynamic_objects,
                query,
                self.tuning.maximum_dynamic_object_size,
                self.legacy_flags.dynamic_objects_are_vertical,
                results,
            );
        }
        append_living_world_manager_results(
            &self.vehicles,
            query,
            self.tuning.maximum_vehicle_size,
            self.legacy_flags.vehicles_are_vertical,
            results,
        );
    }

    #[must_use]
    pub fn box_test(&self, query: GrindBounds) -> Vec<GrindPrimitive> {
        let mut results = Vec::with_capacity(MAX_GRIND_QUERY_RESULTS);
        self.append_box_results(query, &mut results);
        results
    }
}

#[allow(clippy::manual_midpoint)] // The reference adds then multiplies by 0.5.
fn append_living_world_manager_results(
    bodies: &[LivingWorldGrindBody],
    query: GrindBounds,
    maximum_object_size: f32,
    forced_vertical_flag: bool,
    results: &mut Vec<GrindPrimitive>,
) -> bool {
    let query_centre: [f32; 3] =
        std::array::from_fn(|axis| (query.minimum[axis] + query.maximum[axis]) * 0.5);
    let maximum_distance_squared = maximum_object_size * maximum_object_size;
    for body in bodies {
        let delta: [f32; 3] =
            std::array::from_fn(|axis| body.transform[12 + axis] - query_centre[axis]);
        let distance_squared =
            delta[0].mul_add(delta[0], delta[1].mul_add(delta[1], delta[2] * delta[2]));
        // The VMX provider uses a greater-than-or-equal comparison, so an
        // object exactly on the configured radius is rejected. Unordered NaN
        // comparisons fall through and retain the object.
        if distance_squared >= maximum_distance_squared {
            continue;
        }
        for asset in body
            .grind_assets
            .iter()
            .take(MAX_LIVING_WORLD_GRIND_SPLINES_PER_BODY)
        {
            if !asset.append_transformed_box_results(
                query,
                body.transform,
                Some(forced_vertical_flag),
                results,
            ) {
                return false;
            }
        }
    }
    true
}

/// Transforms an authored AABB with the center/absolute-extent algorithm used
/// by the living-world grind providers.
#[must_use]
#[allow(clippy::manual_midpoint)] // The reference adds then multiplies by 0.5.
pub fn transform_dynamic_grind_bounds(
    bounds: GrindBounds,
    body_transform: [f32; 16],
) -> GrindBounds {
    let centre: [f32; 3] =
        std::array::from_fn(|axis| (bounds.minimum[axis] + bounds.maximum[axis]) * 0.5);
    let half_extent: [f32; 3] =
        std::array::from_fn(|axis| (bounds.maximum[axis] - bounds.minimum[axis]) * 0.5);
    let world_centre = transform_affine_point(centre, body_transform);
    let world_half_extent: [f32; 3] = std::array::from_fn(|component| {
        body_transform[8 + component].abs().mul_add(
            half_extent[2],
            body_transform[component].abs().mul_add(
                half_extent[0],
                body_transform[4 + component].abs() * half_extent[1],
            ),
        )
    });
    GrindBounds {
        minimum: std::array::from_fn(|axis| world_centre[axis] - world_half_extent[axis]),
        maximum: std::array::from_fn(|axis| world_centre[axis] + world_half_extent[axis]),
    }
}

/// World-space segment query used behind the registered living-world dynamic
/// grind provider. Callers supply only active DMO/vehicle asset+body pairs in
/// their manager iteration order; result capping occurs when these are
/// appended to the authored query.
#[must_use]
pub fn transformed_dynamic_grind_box_test<'a>(
    assets: impl IntoIterator<Item = (&'a GrindAsset, [f32; 16])>,
    query: GrindBounds,
) -> Vec<GrindPrimitive> {
    let mut results = Vec::with_capacity(MAX_GRIND_QUERY_RESULTS);
    for (asset, body_transform) in assets {
        if !asset.append_transformed_box_results(query, body_transform, None, &mut results) {
            break;
        }
    }
    results
}

/// Reconstructs the static-asset portion of
/// `GrindData::Implementation::BoxTest` at `0x82cf97e0`.
///
/// The reference octree performs the same final per-segment AABB overlap test.
/// Its traversal order remains a differential-test item for queries containing
/// more than the 40-result cap.
#[must_use]
pub fn grind_box_test<'a>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    ignore_vertical: bool,
) -> Vec<GrindPrimitive> {
    let diagonal = std::array::from_fn::<_, 3, _>(|axis| minimum[axis] - maximum[axis]);
    let extent_squared = diagonal.into_iter().map(|value| value * value).sum::<f32>();
    if extent_squared <= VERTICAL_GRIND_HORIZONTAL_EPSILON {
        return Vec::new();
    }

    let query = GrindBounds { minimum, maximum };
    let mut results = Vec::with_capacity(MAX_GRIND_QUERY_RESULTS);
    for asset in assets {
        asset.append_box_results(query, ignore_vertical, &mut results);
        if results.len() == MAX_GRIND_QUERY_RESULTS {
            break;
        }
    }
    results
}

/// Complete `GrindData::Implementation::BoxTest` ordering around the runtime
/// dynamic-spline provider. Static authored results are appended first. The
/// provider is still queried when those results fill the 40-entry output, and
/// its interface has no vertical-filter argument, so dynamic results are not
/// filtered by `ignore_vertical`.
#[must_use]
pub fn grind_box_test_with_dynamic<'a, I>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    ignore_vertical: bool,
    query_dynamic: impl FnOnce(GrindBounds) -> I,
) -> Vec<GrindPrimitive>
where
    I: IntoIterator<Item = GrindPrimitive>,
{
    let diagonal = std::array::from_fn::<_, 3, _>(|axis| minimum[axis] - maximum[axis]);
    let extent_squared = diagonal.into_iter().map(|value| value * value).sum::<f32>();
    if extent_squared <= VERTICAL_GRIND_HORIZONTAL_EPSILON {
        return Vec::new();
    }

    let query = GrindBounds { minimum, maximum };
    let mut results = grind_box_test(assets, minimum, maximum, ignore_vertical);
    for primitive in query_dynamic(query) {
        if results.len() < MAX_GRIND_QUERY_RESULTS {
            results.push(primitive);
        }
    }
    results
}

/// Complete static plus registered living-world provider query. Unlike the
/// generic provider seam, this path owns the reference DMO/vehicle cvars,
/// manager ordering, body-distance rejection, and provider-local cap exits.
#[must_use]
pub fn grind_box_test_with_living_world<'a>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    ignore_vertical: bool,
    living_world: &LivingWorldGrindScene,
) -> Vec<GrindPrimitive> {
    let diagonal = std::array::from_fn::<_, 3, _>(|axis| minimum[axis] - maximum[axis]);
    let extent_squared = diagonal.into_iter().map(|value| value * value).sum::<f32>();
    if extent_squared <= VERTICAL_GRIND_HORIZONTAL_EPSILON {
        return Vec::new();
    }

    let mut results = grind_box_test(assets, minimum, maximum, ignore_vertical);
    living_world.append_box_results(GrindBounds { minimum, maximum }, &mut results);
    results
}

/// Reconstructs the paired-rail filtering stage of
/// `GrindData::TrajectoryBoxTest` at `0x82cf9cf0`.
///
/// The input is the capped, load-ordered result of `BoxTest`. Output order is
/// unchanged. Invalid zero-length directions cannot satisfy the reference's
/// ordered floating-point comparisons and are therefore retained.
#[must_use]
pub fn filter_trajectory_grind_pairs(
    primitives: &[GrindPrimitive],
    takeoff_position: [f32; 3],
    tuning: GrindPairFilterTuning,
) -> Vec<GrindPrimitive> {
    let directions = primitives
        .iter()
        .map(|primitive| normalize(subtract(primitive.end_point, primitive.start_point)))
        .collect::<Vec<_>>();
    let minimum_distance_squared = tuning.minimum_distance * tuning.minimum_distance;
    let maximum_distance_squared = tuning.maximum_distance * tuning.maximum_distance;

    primitives
        .iter()
        .enumerate()
        .filter_map(|(current_index, current)| {
            let Some(current_direction) = directions[current_index] else {
                return Some(*current);
            };
            let current_midpoint = scale(add(current.start_point, current.end_point), 0.5);
            let current_takeoff_distance_squared =
                length_squared(subtract(current_midpoint, takeoff_position));
            let Some(current_up) = normalize(cross(
                cross(current_direction, [0.0, 1.0, 0.0]),
                current_direction,
            )) else {
                return Some(*current);
            };

            for (other_index, other) in primitives.iter().enumerate() {
                if current_index == other_index {
                    continue;
                }
                let Some(other_direction) = directions[other_index] else {
                    continue;
                };
                if length_squared(cross(current_direction, other_direction))
                    >= GRIND_PAIR_PARALLEL_ERROR_SQUARED
                {
                    continue;
                }

                let starts_delta = subtract(other.start_point, current.start_point);
                let perpendicular_delta = subtract(
                    starts_delta,
                    scale(other_direction, dot(other_direction, starts_delta)),
                );
                let pair_distance_squared = length_squared(perpendicular_delta);
                if pair_distance_squared <= minimum_distance_squared
                    || pair_distance_squared >= maximum_distance_squared
                {
                    continue;
                }

                let midpoint_to_other_start = subtract(other.start_point, current_midpoint);
                let midpoint_to_other_end = subtract(other.end_point, current_midpoint);
                if dot(midpoint_to_other_start, midpoint_to_other_end) >= 0.0 {
                    continue;
                }

                let other_point_paired_to_midpoint = subtract(
                    other.start_point,
                    scale(
                        other_direction,
                        dot(other_direction, midpoint_to_other_start),
                    ),
                );
                let other_takeoff_distance_squared =
                    length_squared(subtract(other_point_paired_to_midpoint, takeoff_position));
                let vertical_separation = dot(starts_delta, current_up);
                if vertical_separation > tuning.maximum_vertical_distance
                    || (vertical_separation > -tuning.maximum_vertical_distance
                        && current_takeoff_distance_squared > other_takeoff_distance_squared)
                {
                    return None;
                }
            }
            Some(*current)
        })
        .collect()
}

/// Full static authored-rail query used by `GrindData::TrajectoryBoxTest`.
#[must_use]
pub fn trajectory_grind_box_test<'a>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    takeoff_position: [f32; 3],
    pair_tuning: GrindPairFilterTuning,
) -> Vec<GrindPrimitive> {
    let primitives = grind_box_test(assets, minimum, maximum, false);
    filter_trajectory_grind_pairs(&primitives, takeoff_position, pair_tuning)
}

/// Dynamic-provider form of [`trajectory_grind_box_test`]. Pair filtering is
/// applied after static and transformed dynamic rails share the capped result,
/// matching `TrajectoryBoxTest`.
#[must_use]
pub fn trajectory_grind_box_test_with_dynamic<'a, I>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    takeoff_position: [f32; 3],
    pair_tuning: GrindPairFilterTuning,
    query_dynamic: impl FnOnce(GrindBounds) -> I,
) -> Vec<GrindPrimitive>
where
    I: IntoIterator<Item = GrindPrimitive>,
{
    let primitives = grind_box_test_with_dynamic(assets, minimum, maximum, false, query_dynamic);
    filter_trajectory_grind_pairs(&primitives, takeoff_position, pair_tuning)
}

/// Living-world form of [`trajectory_grind_box_test`]. Pair filtering runs
/// after static, DMO, and vehicle rails share the reference result record.
#[must_use]
pub fn trajectory_grind_box_test_with_living_world<'a>(
    assets: impl IntoIterator<Item = &'a GrindAsset>,
    minimum: [f32; 3],
    maximum: [f32; 3],
    takeoff_position: [f32; 3],
    pair_tuning: GrindPairFilterTuning,
    living_world: &LivingWorldGrindScene,
) -> Vec<GrindPrimitive> {
    let primitives =
        grind_box_test_with_living_world(assets, minimum, maximum, false, living_world);
    filter_trajectory_grind_pairs(&primitives, takeoff_position, pair_tuning)
}

/// Reconstructs the non-debug geometry in
/// `TrajectorySelector::ConsiderGrindPrimitive` at `0x82dba150`.
///
/// The candidate plane is vertical, so gravity has zero projection onto its
/// normal and the reference trajectory/plane solver takes its linear branch.
#[must_use]
pub fn consider_grind_primitive(
    trajectory: &BallisticTrajectory,
    prediction_radius: f32,
    grind_offset: f32,
    primitive: GrindPrimitive,
    grind_index: usize,
) -> Option<GrindTrajectoryResult> {
    let primitive_delta = subtract(primitive.end_point, primitive.start_point);
    let grind_direction = normalize(primitive_delta)?;
    let plane_normal = normalize(cross([0.0, 1.0, 0.0], primitive_delta))?;
    let plane_displacement = scale(plane_normal, grind_offset + prediction_radius);
    let plane_point = add(primitive.start_point, plane_displacement);
    let denominator = dot(trajectory.initial_velocity, plane_normal);
    if denominator == 0.0 {
        return None;
    }
    let penetration_time = dot(
        subtract(plane_point, trajectory.initial_position),
        plane_normal,
    ) / denominator;
    if penetration_time <= 0.0 {
        return None;
    }

    // Intersect the offset prediction plane, then translate back to the rail's
    // actual vertical plane before measuring the primitive.
    let collision_point = subtract(
        trajectory.position_at_time(penetration_time),
        plane_displacement,
    );
    let along_primitive = dot(
        grind_direction,
        subtract(collision_point, primitive.start_point),
    );
    let closest_point_on_primitive = add(
        primitive.start_point,
        scale(grind_direction, along_primitive),
    );
    let endpoint_margin = scale(primitive_delta, 0.1);
    let extended_start = subtract(primitive.start_point, endpoint_margin);
    let extended_end = add(primitive.end_point, endpoint_margin);
    if dot(
        subtract(closest_point_on_primitive, extended_start),
        subtract(closest_point_on_primitive, extended_end),
    ) >= 0.0
    {
        return None;
    }

    let distance_to_grind = length(subtract(collision_point, closest_point_on_primitive));
    let trajectory_direction_in_grind_plane = normalize(subtract(
        trajectory.initial_velocity,
        scale(plane_normal, dot(plane_normal, trajectory.initial_velocity)),
    ))
    .unwrap_or([0.0; 3]);
    let mut horizontal_approach = subtract(closest_point_on_primitive, trajectory.initial_position);
    horizontal_approach[1] = 0.0;
    let landing_angle = normalize(horizontal_approach).map_or(0.0, |approach| {
        let angle = dot(approach, grind_direction).clamp(-1.0, 1.0).acos();
        angle.min(PI - angle).abs()
    });

    #[allow(clippy::cast_possible_truncation)]
    let grind_plane_penetration_index = (penetration_time * PHYSICS_STEP_SECONDS.recip()) as i32;
    Some(GrindTrajectoryResult {
        closest_point_on_primitive,
        collision_point,
        grind_direction,
        trajectory_direction_in_grind_plane,
        distance_to_grind,
        grind_plane_penetration_time: penetration_time,
        landing_angle,
        grind_plane_penetration_index,
        grind_index,
    })
}

/// Selects and removes one candidate using `TrajectorySelector::FindBestGrind`
/// at `0x82dc2d08`.
///
/// Candidates outside the centre-lock distance compete by distance. Once a
/// centre-locked candidate wins, only another locked candidate with enough
/// landing-angle improvement for its height difference may replace it. The
/// graph's original schema name is absent from the PDB; its 64-bit Vault key is
/// retained by the archive loader instead of inventing a name.
#[must_use]
pub fn find_best_grind(
    candidates: &mut Vec<GrindTrajectoryResult>,
    centre_lock_distance: f32,
    minimum_angle_improvement_vs_height_delta: &PointGraph<8>,
) -> Option<GrindTrajectoryResult> {
    let mut best_index = None;
    let mut best_distance = 1_000.0_f32;
    let mut best_landing_angle = PI;
    let mut selected_height = -10_000.0_f32;

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.distance_to_grind >= centre_lock_distance {
            if candidate.distance_to_grind < best_distance {
                best_distance = candidate.distance_to_grind;
                best_index = Some(index);
            }
            continue;
        }

        let angle_improvement = best_landing_angle - candidate.landing_angle;
        let height_delta = candidate.closest_point_on_primitive[1] - selected_height;
        if angle_improvement > minimum_angle_improvement_vs_height_delta.evaluate(height_delta) {
            best_landing_angle = candidate.landing_angle;
            best_distance = -1.0;
            selected_height = candidate.closest_point_on_primitive[1];
            best_index = Some(index);
        }
    }

    best_index.map(|index| candidates.remove(index))
}

/// Reconstructs the landing-velocity decomposition in
/// `TrajectorySelector::AnalyzeAndAdjustTrajectory` at `0x82dc5160`.
#[must_use]
pub fn grind_approach_metrics(
    landing_velocity: [f32; 3],
    grind_direction: [f32; 3],
) -> GrindApproachMetrics {
    let perpendicular_velocity = subtract(
        landing_velocity,
        scale(grind_direction, dot(grind_direction, landing_velocity)),
    );
    let vertical_speed = perpendicular_velocity[1] * (1.0 - grind_direction[1].abs());
    let mut lateral_velocity = perpendicular_velocity;
    lateral_velocity[1] = 0.0;
    GrindApproachMetrics {
        vertical_speed,
        lateral_speed_squared: length_squared(lateral_velocity),
    }
}

/// Applies the PDB-named grind approach speed gates. The reference performs
/// both comparisons as inclusive acceptance (`<=`).
#[must_use]
pub fn grind_approach_speed_is_acceptable(
    metrics: GrindApproachMetrics,
    geometry_type: GrindGeometryType,
    maximum_vertical_speed: f32,
    maximum_lateral_speed_squared_into_ledge: f32,
    maximum_lateral_speed_squared_into_grind: f32,
) -> bool {
    if metrics.vertical_speed.abs() > maximum_vertical_speed {
        return false;
    }
    let maximum_lateral_speed_squared = match geometry_type {
        GrindGeometryType::Ledge => maximum_lateral_speed_squared_into_ledge,
        GrindGeometryType::ThinRail | GrindGeometryType::FatRail => {
            maximum_lateral_speed_squared_into_grind
        }
        GrindGeometryType::Impossible => return false,
    };
    metrics.lateral_speed_squared <= maximum_lateral_speed_squared
}

/// Classifies the close/far hit pattern in `CalcGrindGeometry`. Result indices
/// follow the PDB enum: close 1/2, far 1/2, then centre.
#[must_use]
pub fn classify_grind_geometry_hits(
    collisions: &[GrindGeometryCollision; 5],
    tuning: &GrindGeometryTuning,
) -> GrindGeometryHitClassification {
    let close_one = collisions[0].hit;
    let close_two = collisions[1].hit;
    if close_one && close_two {
        return GrindGeometryHitClassification {
            geometry_type: GrindGeometryType::Impossible,
            reverse_high_side_direction: false,
            is_apex: true,
        };
    }
    if close_one {
        let geometry_type =
            if collisions[2].hit && collisions[2].normal[1] < tuning.coarse_curb_parameter {
                GrindGeometryType::Ledge
            } else {
                GrindGeometryType::FatRail
            };
        return GrindGeometryHitClassification {
            geometry_type,
            reverse_high_side_direction: false,
            is_apex: false,
        };
    }
    if close_two {
        let geometry_type =
            if collisions[3].hit && collisions[3].normal[1] < tuning.coarse_curb_parameter {
                GrindGeometryType::Ledge
            } else {
                GrindGeometryType::FatRail
            };
        return GrindGeometryHitClassification {
            geometry_type,
            reverse_high_side_direction: true,
            is_apex: false,
        };
    }
    GrindGeometryHitClassification {
        geometry_type: GrindGeometryType::ThinRail,
        reverse_high_side_direction: false,
        is_apex: false,
    }
}

/// `GrindGeometryAnalyzer::IsStair` at `0x82cf93d0` recognizes the packed
/// stair surface on either close-side result.
#[must_use]
pub fn grind_geometry_is_stair(collisions: &[GrindGeometryCollision; 5]) -> bool {
    collisions[..2]
        .iter()
        .any(|collision| collision.hit && collision.surface_id & 0xF80 == 0x400)
}

/// `GrindGeometryAnalyzer::IsCurb` at `0x82cf9258`. Only a ledge with both far
/// hits can be a curb; both normals and the signed top/drop height windows use
/// strict comparisons in the reference.
#[must_use]
pub fn grind_geometry_is_curb(
    collisions: &[GrindGeometryCollision; 5],
    upmost_normal: [f32; 3],
    grind_position: [f32; 3],
    geometry_type: GrindGeometryType,
    tuning: &GrindGeometryTuning,
) -> bool {
    if geometry_type != GrindGeometryType::Ledge || !collisions[2].hit || !collisions[3].hit {
        return false;
    }
    if dot(upmost_normal, collisions[2].normal) <= tuning.curb_normal_likeness
        || dot(upmost_normal, collisions[3].normal) <= tuning.curb_normal_likeness
    {
        return false;
    }

    let first_height = dot(
        subtract(collisions[2].position, grind_position),
        upmost_normal,
    );
    let second_height = dot(
        subtract(collisions[3].position, grind_position),
        upmost_normal,
    );
    let lower_height = first_height.min(second_height);
    let higher_height = first_height.max(second_height);
    higher_height > -tuning.curb_high_side_range
        && higher_height < tuning.curb_high_side_range
        && lower_height > -tuning.average_curb_height - tuning.curb_height_range
        && lower_height < tuning.curb_height_range - tuning.average_curb_height
}

/// Reconstructs the basis and line-test preparation at the front of
/// `GrindGeometryAnalyzer::CalcGrindGeometry` (`0x82cfc2d0`).
#[must_use]
pub fn prepare_grind_geometry(
    primitive: &GrindPrimitive,
    grind_position: [f32; 3],
    tuning: &GrindGeometryTuning,
) -> Option<PreparedGrindGeometry> {
    let primitive_direction = normalize(subtract(primitive.end_point, primitive.start_point))?;
    let projected_up = subtract(
        [0.0, 1.0, 0.0],
        scale(primitive_direction, primitive_direction[1]),
    );
    let projected_up_magnitude = length(projected_up);
    if projected_up_magnitude <= 0.000_01 {
        let offset = [tuning.collision_offset, 0.0, 0.0];
        return Some(PreparedGrindGeometry::Uncertain(UncertainGrindGeometry {
            line_positions: [
                add(grind_position, offset),
                subtract(grind_position, offset),
            ],
            upmost_normal: [0.0, 0.0, 1.0],
            primitive_direction,
            surface_info: GrindSurfaceInfo {
                audio_surface_raw: 3,
                physics_surface_raw: 1,
            },
        }));
    }

    let upmost_normal = scale(projected_up, projected_up_magnitude.recip());
    let high_side_direction = cross(primitive_direction, upmost_normal);
    let from_start = subtract(grind_position, primitive.start_point);
    let position = add(
        primitive.start_point,
        scale(primitive_direction, dot(from_start, primitive_direction)),
    );
    let batch = build_grind_geometry_probe_batch(
        tuning.fine_distance,
        tuning.fine_depth,
        tuning.collision_offset,
        tuning.test_height_heuristic * tuning.collision_offset,
        position,
        upmost_normal,
        primitive_direction,
    )?;
    Some(PreparedGrindGeometry::Query(GrindGeometryProbeSetup {
        position,
        upmost_normal,
        primitive_direction,
        high_side_direction,
        batch,
    }))
}

/// Completes every pure post-query branch in `CalcGrindGeometry` after an
/// island provider has returned the five collision records.
#[must_use]
pub fn finish_grind_geometry(
    setup: &GrindGeometryProbeSetup,
    collisions: &[GrindGeometryCollision; 5],
    tuning: &GrindGeometryTuning,
) -> Option<CalculatedGrindGeometry> {
    let classification = classify_grind_geometry_hits(collisions, tuning);
    let high_side_direction = if classification.reverse_high_side_direction {
        scale(setup.high_side_direction, -1.0)
    } else {
        setup.high_side_direction
    };
    let is_stair = grind_geometry_is_stair(collisions);
    let is_curb = grind_geometry_is_curb(
        collisions,
        setup.upmost_normal,
        setup.position,
        classification.geometry_type,
        tuning,
    );
    let surface_info =
        resolve_grind_surface_info(collisions, classification.geometry_type, is_curb);
    let line_positions = resolve_grind_far_low_points(&setup.batch, collisions);
    let (low_normal_limit_angle, high_normal_limit_angle) = calculate_grind_normal_limit_angles(
        setup.position,
        setup.upmost_normal,
        line_positions,
        tuning.wheel_clear_angle_degrees,
    )?;

    Some(CalculatedGrindGeometry {
        line_positions,
        analysis: AnalyzedGrindGeometry {
            upmost_normal: setup.upmost_normal,
            primitive_direction: setup.primitive_direction,
            high_side_direction,
            low_normal_limit_angle,
            high_normal_limit_angle,
            geometry_type: classification.geometry_type,
            uncertain: false,
        },
        surface_info,
        is_stair,
        is_apex: classification.is_apex,
        is_curb,
    })
}

/// Full readable `CalcGrindGeometry` orchestration around a typed batched
/// island-line provider. `None` represents invalid input or the reference's
/// asserted zero-length normal-limit vector.
///
/// # Errors
///
/// Returns the provider's error without substituting collision results.
pub fn calculate_grind_geometry<E>(
    primitive: &GrindPrimitive,
    grind_position: [f32; 3],
    tuning: &GrindGeometryTuning,
    run_batch: impl FnOnce(&GrindGeometryProbeBatch) -> Result<[GrindGeometryCollision; 5], E>,
) -> Result<Option<GrindGeometryCalculation>, E> {
    match prepare_grind_geometry(primitive, grind_position, tuning) {
        None => Ok(None),
        Some(PreparedGrindGeometry::Uncertain(geometry)) => {
            Ok(Some(GrindGeometryCalculation::Uncertain(geometry)))
        }
        Some(PreparedGrindGeometry::Query(setup)) => {
            let collisions = run_batch(&setup.batch)?;
            Ok(finish_grind_geometry(&setup, &collisions, tuning)
                .map(GrindGeometryCalculation::Certain))
        }
    }
}

/// Converts the analyzer result into the exact geometry/lock input consumed by
/// `AnalyzeAndAdjustTrajectory`, while retaining the landing orientations and
/// surface flags needed by the owning prediction state.
#[must_use]
pub fn assess_grind_geometry_on_landing(
    geometry: &GrindGeometryCalculation,
    takeoff_position: [f32; 3],
    grind_position: [f32; 3],
    trajectory_tuning: &TrajectorySelectionTuning,
    orientation_tuning: GrindOrientationTuning,
) -> GrindGeometryLandingAssessment {
    match geometry {
        GrindGeometryCalculation::Certain(calculated) => {
            let landing_orientation = grind_orientations_on_landing(
                &calculated.analysis,
                takeoff_position,
                grind_position,
                orientation_tuning,
            );
            let assessment = match calculated.analysis.geometry_type {
                GrindGeometryType::ThinRail => GrindGeometryAssessment::ThinRail,
                GrindGeometryType::FatRail => GrindGeometryAssessment::FatRail,
                GrindGeometryType::Impossible => GrindGeometryAssessment::Impossible,
                GrindGeometryType::Ledge => {
                    let signed_high_side_position = dot(
                        calculated.analysis.high_side_direction,
                        subtract(takeoff_position, grind_position),
                    );
                    let landing_up_dot_high_side = dot(
                        landing_orientation.selected_up_normal,
                        calculated.analysis.high_side_direction,
                    );
                    GrindGeometryAssessment::Ledge {
                        maximum_lock_distance: grind_ledge_maximum_lock_distance(
                            signed_high_side_position,
                            landing_up_dot_high_side,
                            trajectory_tuning,
                        ),
                    }
                }
            };
            GrindGeometryLandingAssessment {
                assessment,
                landing_orientation,
                surface_info: calculated.surface_info,
                is_stair: calculated.is_stair,
                is_apex: calculated.is_apex,
                is_curb: calculated.is_curb,
            }
        }
        GrindGeometryCalculation::Uncertain(uncertain) => {
            let fallback = AnalyzedGrindGeometry {
                upmost_normal: uncertain.upmost_normal,
                primitive_direction: uncertain.primitive_direction,
                high_side_direction: [0.0; 3],
                low_normal_limit_angle: 0.0,
                high_normal_limit_angle: 0.0,
                geometry_type: GrindGeometryType::ThinRail,
                uncertain: true,
            };
            GrindGeometryLandingAssessment {
                assessment: GrindGeometryAssessment::ThinRail,
                landing_orientation: grind_orientations_on_landing(
                    &fallback,
                    takeoff_position,
                    grind_position,
                    orientation_tuning,
                ),
                surface_info: uncertain.surface_info,
                is_stair: false,
                is_apex: false,
                is_curb: false,
            }
        }
    }
}

/// Reconstructs the line construction in `GrindGeometryAnalyzer::DoGeometryTest`
/// at `0x82cfac60`. The debug reference asserts that both directions have
/// magnitudes strictly inside `.98..1.02` and that both lengths are positive.
#[must_use]
pub fn build_grind_geometry_probe_batch(
    close_distance: f32,
    close_length: f32,
    far_distance: f32,
    far_length: f32,
    position: [f32; 3],
    normal: [f32; 3],
    grind_direction: [f32; 3],
) -> Option<GrindGeometryProbeBatch> {
    let normal_magnitude = length_squared(normal).sqrt();
    let direction_magnitude = length_squared(grind_direction).sqrt();
    if !(normal_magnitude > 0.98
        && normal_magnitude < 1.02
        && direction_magnitude > 0.98
        && direction_magnitude < 1.02
        && close_length > 0.0
        && far_length > 0.0)
    {
        return None;
    }

    let high_side = cross(grind_direction, normal);
    let close_one_base = add(position, scale(high_side, close_distance));
    let close_two_base = subtract(position, scale(high_side, close_distance));
    let far_one_base = add(position, scale(high_side, far_distance));
    let far_two_base = subtract(position, scale(high_side, far_distance));
    let close_vertical = scale(normal, close_length);
    let far_vertical = scale(normal, far_length);
    let line = |base, vertical| GrindGeometryProbeLine {
        high_point: add(base, vertical),
        low_point: subtract(base, vertical),
    };
    let far_one = line(far_one_base, far_vertical);
    let far_two = line(far_two_base, far_vertical);

    Some(GrindGeometryProbeBatch {
        lines: [
            line(close_one_base, close_vertical),
            line(close_two_base, close_vertical),
            far_one,
            far_two,
            line(position, close_vertical),
        ],
        far_low_fallbacks: [far_one.low_point, far_two.low_point],
    })
}

/// The reference replaces each far-line low fallback with its collision
/// position when that far probe hit.
#[must_use]
pub fn resolve_grind_far_low_points(
    batch: &GrindGeometryProbeBatch,
    collisions: &[GrindGeometryCollision; 5],
) -> [[f32; 3]; 2] {
    [
        if collisions[2].hit {
            collisions[2].position
        } else {
            batch.far_low_fallbacks[0]
        },
        if collisions[3].hit {
            collisions[3].position
        } else {
            batch.far_low_fallbacks[1]
        },
    ]
}

/// Reconstructs the final normal-limit calculation in `CalcGrindGeometry`.
/// The first far-low vector bounds the high-side half of the interval; the
/// second bounds the opposite half. Wheel clearance narrows both sides.
#[must_use]
pub fn calculate_grind_normal_limit_angles(
    position: [f32; 3],
    upmost_normal: [f32; 3],
    far_low_points: [[f32; 3]; 2],
    wheel_clear_angle_degrees: f32,
) -> Option<(f32, f32)> {
    let first = subtract(far_low_points[0], position);
    let second = subtract(far_low_points[1], position);
    let first_magnitude = length_squared(first).sqrt();
    let second_magnitude = length_squared(second).sqrt();
    if !(first_magnitude > 0.0 && second_magnitude > 0.0) {
        return None;
    }

    let opposite_up = scale(upmost_normal, -1.0);
    let first_cosine = dot(opposite_up, scale(first, first_magnitude.recip())).clamp(-1.0, 1.0);
    let second_cosine = dot(opposite_up, scale(second, second_magnitude.recip())).clamp(-1.0, 1.0);
    let wheel_clear_angle = wheel_clear_angle_degrees.to_radians();
    Some((
        first_cosine.acos() + PI * 0.5 + wheel_clear_angle,
        PI * 2.0 - second_cosine.acos() - PI * 0.5 - wheel_clear_angle,
    ))
}

/// `GrindGeometryAnalyzer::ResolveSurfaceInfo` at `0x82cfb380`.
///
/// Curb and thin-rail paths use authored overrides. Other geometry searches
/// close, far, then centre results; when both side results hit, the higher Y
/// wins and an exact tie selects the second result.
#[must_use]
pub fn resolve_grind_surface_info(
    collisions: &[GrindGeometryCollision; 5],
    geometry_type: GrindGeometryType,
    is_curb: bool,
) -> GrindSurfaceInfo {
    const CONCRETE_POLISHED: u8 = 3;
    const CONCRETE_ROUGH: u8 = 4;
    const METAL_SOLID_ROUND_ONE: u8 = 11;
    const PHYSICS_SMOOTH: u8 = 1;
    const PHYSICS_ROUGH: u8 = 2;
    const PHYSICS_SLIPPERY: u8 = 4;

    if is_curb {
        return GrindSurfaceInfo {
            audio_surface_raw: CONCRETE_ROUGH,
            physics_surface_raw: PHYSICS_ROUGH,
        };
    }

    if geometry_type == GrindGeometryType::ThinRail {
        return GrindSurfaceInfo {
            audio_surface_raw: if collisions[4].hit {
                packed_grind_surface_info(collisions[4].surface_id).audio_surface_raw
            } else {
                METAL_SOLID_ROUND_ONE
            },
            physics_surface_raw: PHYSICS_SLIPPERY,
        };
    }

    let selected = select_higher_grind_collision(collisions, 0, 1)
        .or_else(|| select_higher_grind_collision(collisions, 2, 3))
        .or_else(|| collisions[4].hit.then_some(&collisions[4]));
    selected.map_or(
        GrindSurfaceInfo {
            audio_surface_raw: CONCRETE_POLISHED,
            physics_surface_raw: PHYSICS_SMOOTH,
        },
        |collision| packed_grind_surface_info(collision.surface_id),
    )
}

fn packed_grind_surface_info(surface_id: u32) -> GrindSurfaceInfo {
    GrindSurfaceInfo {
        audio_surface_raw: (surface_id & 0x7F) as u8,
        physics_surface_raw: ((surface_id >> 7) & 0x1F) as u8,
    }
}

fn select_higher_grind_collision(
    collisions: &[GrindGeometryCollision; 5],
    first: usize,
    second: usize,
) -> Option<&GrindGeometryCollision> {
    match (collisions[first].hit, collisions[second].hit) {
        (true, true) => Some(
            if collisions[first].position[1] > collisions[second].position[1] {
                &collisions[first]
            } else {
                &collisions[second]
            },
        ),
        (true, false) => Some(&collisions[first]),
        (false, true) => Some(&collisions[second]),
        (false, false) => None,
    }
}

/// Reconstructs the four ledge lock-distance branches in
/// `TrajectorySelector::AnalyzeAndAdjustTrajectory` at `0x82dc5160`.
/// The first sign locates the board centre of mass relative to the ledge's
/// high-side direction; the second describes the selected landing up vector.
#[must_use]
pub fn grind_ledge_maximum_lock_distance(
    signed_high_side_position: f32,
    landing_up_dot_high_side: f32,
    tuning: &TrajectorySelectionTuning,
) -> f32 {
    if signed_high_side_position > 0.0 {
        let scalar = if landing_up_dot_high_side > 0.0 {
            tuning.grind_lock_ledge_scalar_squared
        } else {
            tuning.grind_lock_ledge_scalar
        };
        scalar * tuning.grind_lock_to_tip_distance
    } else {
        let scalar = if landing_up_dot_high_side > 0.0 {
            tuning.grind_lock_ledge_low_side_scalar_squared
        } else {
            tuning.grind_lock_ledge_low_side_scalar
        };
        scalar.mul_add(
            tuning.grind_lock_to_tip_distance,
            tuning.grind_lock_to_centre_distance,
        )
    }
}

/// Reconstructs the candidate-removal and retry gates at the front of
/// `TrajectorySelector::AnalyzeAndAdjustTrajectory` (`0x82dc5160`).
///
/// The callback is the explicit boundary for `CalcGrindGeometry` and the
/// ledge-side lock calculation that depend on a physics island and board pose.
#[must_use]
pub fn select_grind_for_adjustment<F>(
    candidates: &mut Vec<GrindTrajectoryResult>,
    primitives: &[GrindPrimitive],
    landing_velocity: [f32; 3],
    tuning: &TrajectorySelectionTuning,
    mut analyze_geometry: F,
) -> Option<AcceptedGrindCandidate>
where
    F: FnMut(&GrindPrimitive, &GrindTrajectoryResult) -> GrindGeometryAssessment,
{
    loop {
        let selected = find_best_grind(
            candidates,
            tuning.grind_lock_to_centre_distance,
            &tuning.grind_minimum_angle_improvement_vs_height_delta,
        )?;
        let primitive = *primitives.get(selected.grind_index)?;
        let approach = grind_approach_metrics(landing_velocity, selected.grind_direction);
        if approach.vertical_speed.abs() > tuning.grind_maximum_speed_down_onto_grind {
            continue;
        }

        let geometry = analyze_geometry(&primitive, &selected);
        let geometry_type = geometry.geometry_type();
        if !grind_approach_speed_is_acceptable(
            approach,
            geometry_type,
            tuning.grind_maximum_speed_down_onto_grind,
            tuning.grind_maximum_speed_squared_into_ledge,
            tuning.grind_maximum_speed_squared_into_grind,
        ) {
            continue;
        }
        let maximum_lock_distance = match geometry {
            GrindGeometryAssessment::Ledge {
                maximum_lock_distance,
            } => maximum_lock_distance,
            GrindGeometryAssessment::ThinRail | GrindGeometryAssessment::FatRail => {
                tuning.grind_lock_to_tip_distance + tuning.grind_lock_to_centre_distance
            }
            GrindGeometryAssessment::Impossible => continue,
        };
        if selected.distance_to_grind >= maximum_lock_distance {
            continue;
        }
        return Some(AcceptedGrindCandidate {
            trajectory_result: selected,
            primitive,
            geometry,
            approach,
        });
    }
}

/// Native analyzer-backed form of `AnalyzeAndAdjustTrajectory`'s selection
/// loop. Rejected candidates remain removed, while island batch failures are
/// propagated without fabricating geometry.
///
/// # Errors
///
/// Returns the line-batch provider's error.
pub fn select_grind_for_adjustment_with_geometry<E>(
    candidates: &mut Vec<GrindTrajectoryResult>,
    primitives: &[GrindPrimitive],
    landing_velocity: [f32; 3],
    takeoff_position: [f32; 3],
    tuning: GrindAdjustmentAnalysisTuning<'_>,
    mut run_batch: impl FnMut(
        &GrindPrimitive,
        &GrindGeometryProbeBatch,
    ) -> Result<[GrindGeometryCollision; 5], E>,
) -> Result<Option<AcceptedAnalyzedGrindCandidate>, E> {
    loop {
        let Some(selected) = find_best_grind(
            candidates,
            tuning.trajectory.grind_lock_to_centre_distance,
            &tuning
                .trajectory
                .grind_minimum_angle_improvement_vs_height_delta,
        ) else {
            return Ok(None);
        };
        let Some(&primitive) = primitives.get(selected.grind_index) else {
            return Ok(None);
        };
        let approach = grind_approach_metrics(landing_velocity, selected.grind_direction);
        if approach.vertical_speed.abs() > tuning.trajectory.grind_maximum_speed_down_onto_grind {
            continue;
        }

        let calculation = calculate_grind_geometry(
            &primitive,
            selected.closest_point_on_primitive,
            tuning.geometry,
            |batch| run_batch(&primitive, batch),
        )?;
        let Some(calculation) = calculation else {
            continue;
        };
        let geometry = assess_grind_geometry_on_landing(
            &calculation,
            takeoff_position,
            selected.closest_point_on_primitive,
            tuning.trajectory,
            tuning.orientation,
        );
        let geometry_type = geometry.assessment.geometry_type();
        if !grind_approach_speed_is_acceptable(
            approach,
            geometry_type,
            tuning.trajectory.grind_maximum_speed_down_onto_grind,
            tuning.trajectory.grind_maximum_speed_squared_into_ledge,
            tuning.trajectory.grind_maximum_speed_squared_into_grind,
        ) {
            continue;
        }
        let maximum_lock_distance = match geometry.assessment {
            GrindGeometryAssessment::Ledge {
                maximum_lock_distance,
            } => maximum_lock_distance,
            GrindGeometryAssessment::ThinRail | GrindGeometryAssessment::FatRail => {
                tuning.trajectory.grind_lock_to_tip_distance
                    + tuning.trajectory.grind_lock_to_centre_distance
            }
            GrindGeometryAssessment::Impossible => continue,
        };
        if selected.distance_to_grind >= maximum_lock_distance {
            continue;
        }
        return Ok(Some(AcceptedAnalyzedGrindCandidate {
            accepted: AcceptedGrindCandidate {
                trajectory_result: selected,
                primitive,
                geometry: geometry.assessment,
                approach,
            },
            geometry,
        }));
    }
}

/// Returns the displacement that the reference feeds to its trajectory
/// adjustment. Inside the centre lock it aims the collision point directly at
/// the rail; outside it leaves `GrindTipScalar * deck footprint` for tip
/// contact.
#[must_use]
pub fn grind_snap_displacement(
    selected: &GrindTrajectoryResult,
    trajectory_tuning: &TrajectorySelectionTuning,
    deck_tuning: GrindDeckTuning,
) -> [f32; 3] {
    let displacement = subtract(
        selected.closest_point_on_primitive,
        selected.collision_point,
    );
    let distance = length(displacement);
    if distance < trajectory_tuning.grind_lock_to_centre_distance {
        return displacement;
    }
    debug_assert!(distance != 0.0, "centre-external grind distance is zero");
    let retained_tip_distance =
        trajectory_tuning.grind_tip_scalar * deck_tuning.contact_footprint_radius;
    scale(displacement, (distance - retained_tip_distance) / distance)
}

/// Returns the angle by which a grind snap would redirect the launch
/// velocity. `AnalyzeAndAdjustTrajectory` divides by the candidate's
/// continuous plane-penetration time for this gate, even though the accepted
/// displacement is subsequently applied at the discrete penetration frame.
///
/// The reference treats a near-zero product of the two squared velocity
/// magnitudes as a zero angle instead of calling `AngleBetweenVectors`.
#[must_use]
pub fn grind_snap_velocity_redirection_angle(
    trajectory: &BallisticTrajectory,
    selected: &GrindTrajectoryResult,
    snap_displacement: [f32; 3],
) -> f32 {
    let redirected_velocity = add(
        trajectory.initial_velocity,
        scale(
            snap_displacement,
            selected.grind_plane_penetration_time.recip(),
        ),
    );
    let magnitude_product =
        length_squared(trajectory.initial_velocity) * length_squared(redirected_velocity);
    if magnitude_product <= GRIND_REDIRECTION_MAGNITUDE_PRODUCT_EPSILON {
        return 0.0;
    }
    let cosine = dot(trajectory.initial_velocity, redirected_velocity) / magnitude_product.sqrt();
    cosine.clamp(-1.0, 1.0).acos()
}

/// Reconstructs `GrindPrediction::CalcTiltedUpmostNormal` at `0x82cf9440`.
/// The geometry stores a circular interval around the rail with world-up
/// represented at PI. A wrapping interval uses its midpoint, a one-sided
/// interval uses the endpoint nearest PI, and an interval containing PI keeps
/// the unmodified upmost normal.
#[must_use]
#[allow(clippy::manual_midpoint)] // The reference performs `(low + high) * 0.5`.
pub fn tilted_upmost_grind_normal(geometry: &AnalyzedGrindGeometry) -> [f32; 3] {
    let low = geometry.low_normal_limit_angle;
    let high = geometry.high_normal_limit_angle;
    if low >= high {
        return rotate_vector_about_unit_axis(
            scale(geometry.upmost_normal, -1.0),
            geometry.primitive_direction,
            (low + high) * 0.5,
        );
    }
    if low >= PI || high <= PI {
        let nearest_limit = if (PI - low).abs() >= (PI - high).abs() {
            high
        } else {
            low
        };
        return rotate_vector_about_unit_axis(
            scale(geometry.upmost_normal, -1.0),
            geometry.primitive_direction,
            nearest_limit,
        );
    }
    geometry.upmost_normal
}

/// Reconstructs the post-island portion of
/// `GrindPrediction::FindBestGrindUpVectorOnLanding` at `0x82cfca88`.
/// `CalcGrindGeometry` remains the explicit query boundary; once its typed
/// result is available, these orientation outputs require no island access.
#[must_use]
pub fn grind_orientations_on_landing(
    geometry: &AnalyzedGrindGeometry,
    takeoff_position: [f32; 3],
    grind_position: [f32; 3],
    tuning: GrindOrientationTuning,
) -> GrindLandingOrientation {
    if geometry.geometry_type == GrindGeometryType::Impossible {
        return GrindLandingOrientation {
            selected_up_normal: [0.0; 3],
            possible: PossibleGrindOrientations {
                geometry_type: GrindGeometryType::Impossible,
                is_garbage: true,
                boardslide_direction: [0.0; 3],
                tipslide_direction: [0.0; 3],
                backslash_direction: [0.0; 3],
                high_side_direction: [0.0; 3],
            },
        };
    }
    if geometry.uncertain {
        return GrindLandingOrientation {
            selected_up_normal: [0.0, 1.0, 0.0],
            possible: PossibleGrindOrientations {
                geometry_type: GrindGeometryType::ThinRail,
                is_garbage: true,
                boardslide_direction: [1.0, 0.0, 0.0],
                tipslide_direction: [1.0, 0.0, 0.0],
                backslash_direction: [-1.0, 0.0, 0.0],
                high_side_direction: [1.0, 0.0, 0.0],
            },
        };
    }

    let selected_up_normal = tilted_upmost_grind_normal(geometry);
    let side_direction = cross(geometry.primitive_direction, geometry.upmost_normal);
    let tipslide_tilt = tuning.tipslide_tilt_degrees.to_radians();
    let backslash_tilt = tuning.backslash_tilt_degrees.to_radians();

    let (boardslide_direction, tipslide_direction, backslash_direction) =
        if geometry.geometry_type == GrindGeometryType::ThinRail {
            let takeoff_delta = subtract(takeoff_position, grind_position);
            let takeoff_sign = if dot(side_direction, takeoff_delta) > 0.0 {
                1.0
            } else {
                -1.0
            };
            let takeoff_side = scale(side_direction, takeoff_sign);
            let rotation_axis = cross(takeoff_side, geometry.upmost_normal);
            (
                side_direction,
                rotate_vector_about_unit_axis(takeoff_side, rotation_axis, tipslide_tilt),
                rotate_vector_about_unit_axis(takeoff_side, rotation_axis, PI - backslash_tilt),
            )
        } else {
            let rotation_axis = cross(side_direction, geometry.upmost_normal);
            let high_side_sign = if dot(side_direction, geometry.high_side_direction) > 0.0 {
                1.0
            } else {
                -1.0
            };
            let backslash_side = scale(side_direction, high_side_sign);
            let backslash_axis = scale(rotation_axis, high_side_sign);
            let tipslide_side = scale(side_direction, -high_side_sign);
            let tipslide_axis = scale(rotation_axis, -high_side_sign);
            let boardslide = if geometry.geometry_type == GrindGeometryType::FatRail {
                side_direction
            } else {
                cross(geometry.primitive_direction, selected_up_normal)
            };
            (
                boardslide,
                rotate_vector_about_unit_axis(tipslide_side, tipslide_axis, tipslide_tilt),
                rotate_vector_about_unit_axis(backslash_side, backslash_axis, backslash_tilt),
            )
        };

    GrindLandingOrientation {
        selected_up_normal,
        possible: PossibleGrindOrientations {
            geometry_type: geometry.geometry_type,
            is_garbage: false,
            boardslide_direction,
            tipslide_direction,
            backslash_direction,
            high_side_direction: geometry.high_side_direction,
        },
    }
}

/// Semantic equivalent of `PhysicsUtility::RotateVector` at `0x826fc598`.
fn rotate_vector_about_unit_axis(vector: [f32; 3], axis: [f32; 3], angle_radians: f32) -> [f32; 3] {
    let axis_length = length(axis);
    debug_assert!(
        axis_length > 0.98 && axis_length < 1.02,
        "reference RotateVector requires a unit axis"
    );
    let (sine, cosine) = angle_radians.sin_cos();
    let parallel_scale = dot(axis, vector) * (1.0 - cosine);
    add(
        add(scale(vector, cosine), scale(cross(axis, vector), sine)),
        scale(axis, parallel_scale),
    )
}

/// Reconstructs `TrajectorySelector::CalculateGrindVerticalNormal` at
/// `0x82dba638`: project world up perpendicular to the rail, orient the result
/// upward, and normalize with world up as the degenerate vertical-rail
/// fallback.
#[must_use]
pub fn grind_vertical_normal(grind_direction: [f32; 3]) -> [f32; 3] {
    const WORLD_UP: [f32; 3] = [0.0, 1.0, 0.0];

    let mut vertical_normal = cross(cross(grind_direction, WORLD_UP), grind_direction);
    if vertical_normal[1] < 0.0 {
        vertical_normal = scale(vertical_normal, -1.0);
    }
    normalize(vertical_normal).unwrap_or(WORLD_UP)
}

/// Reconstructs `TrajectorySelector::CalculateGrindLandingNormal` at
/// `0x82dba758`. The velocity component perpendicular to the rail leans the
/// desired up vector against the approach, then the shared physics utility
/// limits that vector to `GrindLandingMaxAngle` around the geometry normal.
#[must_use]
pub fn grind_landing_normal(
    grind_normal: [f32; 3],
    grind_direction: [f32; 3],
    trajectory_velocity: [f32; 3],
    landing_velocity_scalar: f32,
    landing_maximum_angle_degrees: f32,
) -> [f32; 3] {
    const WORLD_UP: [f32; 3] = [0.0, 1.0, 0.0];

    let perpendicular_velocity = subtract(
        trajectory_velocity,
        scale(grind_direction, dot(grind_direction, trajectory_velocity)),
    );
    let desired_normal = add(
        WORLD_UP,
        scale(perpendicular_velocity, -landing_velocity_scalar),
    );
    let clamped = clamp_vector_within_angle_range(
        desired_normal,
        grind_normal,
        landing_maximum_angle_degrees.to_radians(),
    );
    normalize(clamped).unwrap_or([0.0; 3])
}

/// Semantic reconstruction of
/// `PhysicsUtility::ClampVectorWithinAngleRange` at `0x82e49fd0`.
fn clamp_vector_within_angle_range(vector: [f32; 3], target: [f32; 3], range: f32) -> [f32; 3] {
    let Some(vector_direction) = normalize(vector) else {
        return vector;
    };
    let Some(target_direction) = normalize(target) else {
        return vector;
    };
    let cosine = dot(vector_direction, target_direction).clamp(-1.0, 1.0);
    let angle = cosine.acos();
    if angle.abs() < range {
        return vector;
    }

    let perpendicular = subtract(vector_direction, scale(target_direction, cosine));
    if length_squared(perpendicular) < 1.0 / 65_536.0 {
        return vector;
    }
    let perpendicular_direction = normalize(perpendicular).unwrap_or([0.0; 3]);
    let (sine, cosine) = range.sin_cos();
    let clamped_direction = add(
        scale(target_direction, cosine),
        scale(perpendicular_direction, sine),
    );
    scale(clamped_direction, length(vector))
}

/// Completes the candidate-removal loop in
/// `TrajectorySelector::AnalyzeAndAdjustTrajectory` at `0x82dc5160`.
/// Candidates rejected by the final velocity-redirection angle gate stay
/// removed, so the next `FindBestGrind` winner is considered.
#[must_use]
pub fn plan_grind_adjustment<F>(
    candidates: &mut Vec<GrindTrajectoryResult>,
    primitives: &[GrindPrimitive],
    trajectory: &BallisticTrajectory,
    landing_velocity: [f32; 3],
    trajectory_tuning: &TrajectorySelectionTuning,
    deck_tuning: GrindDeckTuning,
    mut analyze_geometry: F,
) -> Option<GrindAdjustmentPlan>
where
    F: FnMut(&GrindPrimitive, &GrindTrajectoryResult) -> GrindGeometryAssessment,
{
    loop {
        let accepted_candidate = select_grind_for_adjustment(
            candidates,
            primitives,
            landing_velocity,
            trajectory_tuning,
            &mut analyze_geometry,
        )?;
        let snap_displacement = grind_snap_displacement(
            &accepted_candidate.trajectory_result,
            trajectory_tuning,
            deck_tuning,
        );
        let velocity_redirection_angle = grind_snap_velocity_redirection_angle(
            trajectory,
            &accepted_candidate.trajectory_result,
            snap_displacement,
        );
        if velocity_redirection_angle
            > trajectory_tuning
                .grind_adjustment_maximum_angle
                .to_radians()
        {
            continue;
        }
        return Some(GrindAdjustmentPlan {
            accepted_candidate,
            snap_displacement,
            velocity_redirection_angle,
        });
    }
}

/// Applies the accepted tail of `AnalyzeAndAdjustTrajectory`. Landing-normal
/// steering uses the original trajectory velocity at the discrete collision
/// frame; the velocity correction is applied afterward by the recovered
/// `tTrajectory::AdjustTrajectory` routine.
#[allow(clippy::cast_precision_loss)] // The reference converts its signed frame to f32.
pub fn apply_grind_adjustment(
    trajectory: &mut BallisticTrajectory,
    plan: &GrindAdjustmentPlan,
    grind_normal: [f32; 3],
    tuning: &TrajectorySelectionTuning,
) -> AppliedGrindAdjustment {
    let selected = &plan.accepted_candidate.trajectory_result;
    let collision_time = selected.grind_plane_penetration_index as f32 * PHYSICS_STEP_SECONDS;
    let trajectory_velocity = add(
        trajectory.initial_velocity,
        scale(trajectory.gravity, collision_time),
    );
    let landing_normal = grind_landing_normal(
        grind_normal,
        selected.grind_direction,
        trajectory_velocity,
        tuning.grind_landing_velocity_scalar,
        tuning.grind_landing_maximum_angle,
    );
    let most_vertical_normal = grind_vertical_normal(selected.grind_direction);
    trajectory.adjust(
        selected.grind_plane_penetration_index,
        plan.snap_displacement,
        tuning.maximum_trajectory_adjustment,
    );
    AppliedGrindAdjustment {
        collision_frame: selected.grind_plane_penetration_index,
        collision_time,
        landing_normal,
        most_vertical_normal,
    }
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] + right[axis])
}

fn subtract(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}

fn scale(vector: [f32; 3], scalar: f32) -> [f32; 3] {
    vector.map(|component| component * scalar)
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1].mul_add(right[2], -(left[2] * right[1])),
        left[2].mul_add(right[0], -(left[0] * right[2])),
        left[0].mul_add(right[1], -(left[1] * right[0])),
    ]
}

fn length(vector: [f32; 3]) -> f32 {
    length_squared(vector).sqrt()
}

fn length_squared(vector: [f32; 3]) -> f32 {
    dot(vector, vector)
}

fn normalize(vector: [f32; 3]) -> Option<[f32; 3]> {
    let magnitude = length(vector);
    (magnitude > 0.0).then(|| scale(vector, magnitude.recip()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::pegasus::{ObjectId, SplineHeader, SplineSegment};

    fn segment(
        coefficients: [[f32; 4]; 4],
        minimum: [f32; 4],
        maximum: [f32; 4],
        spline_offset: u32,
    ) -> SplineSegment {
        SplineSegment {
            basis_matrix: coefficients.concat().try_into().unwrap(),
            inverse: [0.0; 4],
            bounding_box_min: minimum,
            bounding_box_max: maximum,
            length: 0.0,
            distance: 0.0,
            spline_offset: Some(spline_offset),
            previous_offset: None,
            next_offset: None,
        }
    }

    fn grind_result(distance: f32, height: f32, angle: f32, index: usize) -> GrindTrajectoryResult {
        GrindTrajectoryResult {
            closest_point_on_primitive: [0.0, height, 0.0],
            collision_point: [0.0; 3],
            grind_direction: [1.0, 0.0, 0.0],
            trajectory_direction_in_grind_plane: [1.0, 0.0, 0.0],
            distance_to_grind: distance,
            grind_plane_penetration_time: 0.0,
            landing_angle: angle,
            grind_plane_penetration_index: 0,
            grind_index: index,
        }
    }

    fn grind_primitive(start_point: [f32; 3], end_point: [f32; 3], index: usize) -> GrindPrimitive {
        GrindPrimitive {
            start_point,
            end_point,
            spline_header_offset: None,
            is_vertical: false,
            source_asset_id: 1,
            source_dictionary_index: 2,
            source_segment_index: index,
        }
    }

    fn translated_transform(translation: [f32; 3]) -> [f32; 16] {
        [
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            translation[0],
            translation[1],
            translation[2],
            1.0,
        ]
    }

    fn dynamic_asset(
        asset_id: u64,
        start: [f32; 3],
        end: [f32; 3],
        bounds: GrindBounds,
    ) -> GrindAsset {
        let delta = subtract(end, start);
        GrindAsset::from_spline_data(
            asset_id,
            0,
            &SplineData {
                splines_offset: 16,
                segments_offset: 48,
                splines: vec![],
                segments: vec![segment(
                    [
                        [0.0; 4],
                        [0.0; 4],
                        [delta[0], delta[1], delta[2], 0.0],
                        [start[0], start[1], start[2], 1.0],
                    ],
                    [bounds.minimum[0], bounds.minimum[1], bounds.minimum[2], 0.0],
                    [bounds.maximum[0], bounds.maximum[1], bounds.maximum[2], 0.0],
                    16,
                )],
            },
        )
    }

    fn analyzed_geometry(geometry_type: GrindGeometryType) -> AnalyzedGrindGeometry {
        AnalyzedGrindGeometry {
            upmost_normal: [0.0, 1.0, 0.0],
            primitive_direction: [1.0, 0.0, 0.0],
            high_side_direction: [0.0, 0.0, 1.0],
            low_normal_limit_angle: 0.0,
            high_normal_limit_angle: PI * 2.0,
            geometry_type,
            uncertain: false,
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn converts_spline_coefficients_with_reference_endpoint_grouping() {
        let data = SplineData {
            splines_offset: 16,
            segments_offset: 48,
            splines: vec![SplineHeader {
                guid: 1,
                local_guid: 2,
                instance: ObjectId(0),
                head_offset: Some(48),
                tail_offset: Some(48),
            }],
            segments: vec![segment(
                [
                    [1.0, 2.0, 3.0, 0.0],
                    [4.0, 5.0, 6.0, 0.0],
                    [7.0, 8.0, 9.0, 0.0],
                    [10.0, 11.0, 12.0, 1.0],
                ],
                [9.0, 10.0, 11.0, 0.0],
                [23.0, 27.0, 31.0, 0.0],
                16,
            )],
        };
        let asset = GrindAsset::from_spline_data(0x1234, 7, &data);
        assert_eq!(asset.bounds.unwrap().minimum, [8.0, 9.0, 10.0]);
        assert_eq!(asset.bounds.unwrap().maximum, [24.0, 28.0, 32.0]);
        assert_eq!(asset.primitives[0].start_point, [10.0, 11.0, 12.0]);
        assert_eq!(asset.primitives[0].end_point, [22.0, 26.0, 30.0]);
        assert_eq!(asset.primitives[0].spline_header_offset, Some(16));
        assert!(!asset.primitives[0].is_vertical);
    }

    #[test]
    fn box_test_filters_vertical_segments_and_caps_results() {
        let mut data = SplineData {
            splines_offset: 16,
            segments_offset: 48,
            splines: vec![],
            segments: vec![],
        };
        for index in 0..45_u16 {
            let x = f32::from(index);
            data.segments.push(segment(
                [[0.0; 4], [0.0; 4], [0.0, 1.0, 0.0, 0.0], [x, 0.0, 0.0, 1.0]],
                [x - 0.1, -0.1, -0.1, 0.0],
                [x + 0.1, 1.1, 0.1, 0.0],
                16,
            ));
        }
        let asset = GrindAsset::from_spline_data(1, 2, &data);
        assert!(
            asset
                .primitives
                .iter()
                .all(|primitive| primitive.is_vertical)
        );
        assert_eq!(
            grind_box_test(
                std::slice::from_ref(&asset),
                [-1.0; 3],
                [50.0, 2.0, 1.0],
                false,
            )
            .len(),
            MAX_GRIND_QUERY_RESULTS
        );
        assert!(grind_box_test(&[asset], [-1.0; 3], [50.0, 2.0, 1.0], true).is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)] // The provider receives the original AABB exactly.
    fn dynamic_grind_provider_appends_after_static_without_vertical_filtering() {
        let mut dynamic = grind_primitive([0.0; 3], [0.0, 1.0, 0.0], 9);
        dynamic.is_vertical = true;
        dynamic.source_asset_id = 99;
        let results = grind_box_test_with_dynamic(
            std::iter::empty::<&GrindAsset>(),
            [-1.0; 3],
            [1.0; 3],
            true,
            |bounds| {
                assert_eq!(bounds.minimum, [-1.0; 3]);
                [dynamic]
            },
        );
        assert_eq!(results, [dynamic]);

        let mut queried = false;
        let results = grind_box_test_with_dynamic(
            std::iter::empty::<&GrindAsset>(),
            [0.0; 3],
            [0.0; 3],
            false,
            |_| {
                queried = true;
                [dynamic]
            },
        );
        assert!(results.is_empty());
        assert!(!queried);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact affine lane ordering is the contract under test.
    fn dynamic_grind_transform_uses_body_columns_and_recomputes_verticality() {
        let primitive = grind_primitive([1.0, 2.0, 3.0], [3.0, 2.0, 3.0], 7);
        let transform = [
            0.0, 1.0, 0.0, 0.0, // x axis
            -1.0, 0.0, 0.0, 0.0, // y axis
            0.0, 0.0, 1.0, 0.0, // z axis
            10.0, 20.0, 30.0, 1.0, // translation
        ];

        let transformed = transform_dynamic_grind_primitive(primitive, transform);
        assert_eq!(transformed.start_point, [8.0, 21.0, 33.0]);
        assert_eq!(transformed.end_point, [8.0, 23.0, 33.0]);
        assert!(transformed.is_vertical);
        assert_eq!(transformed.source_segment_index, 7);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact affine AABB/endpoint lanes are under test.
    fn transformed_dynamic_grind_query_uses_world_segment_bounds() {
        let transform = [
            0.0, 1.0, 0.0, 0.0, // x axis
            -1.0, 0.0, 0.0, 0.0, // y axis
            0.0, 0.0, 1.0, 0.0, // z axis
            10.0, 20.0, 30.0, 1.0, // translation
        ];
        assert_eq!(
            transform_dynamic_grind_bounds(
                GrindBounds {
                    minimum: [1.0, 2.0, 3.0],
                    maximum: [3.0, 4.0, 5.0],
                },
                transform,
            ),
            GrindBounds {
                minimum: [6.0, 21.0, 33.0],
                maximum: [8.0, 23.0, 35.0],
            }
        );

        let data = SplineData {
            splines_offset: 16,
            segments_offset: 48,
            splines: vec![],
            segments: vec![segment(
                [
                    [2.0, 0.0, 0.0, 0.0],
                    [0.0; 4],
                    [0.0; 4],
                    [1.0, 2.0, 3.0, 1.0],
                ],
                [0.9, 1.9, 2.9, 0.0],
                [3.1, 2.1, 3.1, 0.0],
                16,
            )],
        };
        let asset = GrindAsset::from_spline_data(5, 6, &data);
        let results = transformed_dynamic_grind_box_test(
            [(&asset, transform)],
            GrindBounds {
                minimum: [7.8, 20.8, 32.8],
                maximum: [8.2, 23.2, 33.2],
            },
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].start_point, [8.0, 21.0, 33.0]);
        assert_eq!(results[0].end_point, [8.0, 23.0, 33.0]);
    }

    #[test]
    fn living_world_grind_defaults_match_cvar_initializers() {
        assert_eq!(
            LivingWorldGrindTuning::default(),
            LivingWorldGrindTuning {
                feed_dynamic_object_splines: true,
                maximum_dynamic_object_size: 15.0,
                maximum_vehicle_size: 15.0,
            }
        );
        assert_eq!(
            LivingWorldLegacyGrindFlags::default(),
            LivingWorldLegacyGrindFlags {
                dynamic_objects_are_vertical: false,
                vehicles_are_vertical: false,
            }
        );
    }

    #[test]
    fn living_world_body_owns_only_requested_header_ranges_in_array_order() {
        let segment_bounds = ([-0.5, -0.5, -0.5, 0.0], [0.5, 0.5, 0.5, 0.0]);
        let make_segment = |x: f32, spline_offset| {
            segment(
                [
                    [0.0; 4],
                    [0.0; 4],
                    [0.25, 0.0, 0.0, 0.0],
                    [x, 0.0, 0.0, 1.0],
                ],
                segment_bounds.0,
                segment_bounds.1,
                spline_offset,
            )
        };
        let data = SplineData {
            splines_offset: 16,
            segments_offset: 80,
            splines: vec![
                SplineHeader {
                    guid: 1,
                    local_guid: 1,
                    instance: ObjectId(1),
                    head_offset: Some(80),
                    tail_offset: Some(224),
                },
                SplineHeader {
                    guid: 2,
                    local_guid: 2,
                    instance: ObjectId(1),
                    head_offset: Some(368),
                    tail_offset: Some(368),
                },
            ],
            segments: vec![
                make_segment(0.0, 16),
                make_segment(0.1, 16),
                make_segment(0.2, 48),
            ],
        };
        let body = LivingWorldGrindBody::from_spline_headers(
            translated_transform([0.0; 3]),
            55,
            7,
            &data,
            [1, 0],
        );

        assert_eq!(body.grind_assets.len(), 2);
        assert_eq!(
            body.grind_assets
                .iter()
                .flat_map(|asset| &asset.primitives)
                .map(|primitive| primitive.source_segment_index)
                .collect::<Vec<_>>(),
            [2, 0, 1]
        );
    }

    #[test]
    fn living_world_body_silently_caps_embedded_headers_at_sixty_four() {
        let segments_offset = 16 + 65 * 32;
        let mut splines = Vec::new();
        let mut segments = Vec::new();
        for index in 0..65_u32 {
            let segment_offset = segments_offset + index * 144;
            splines.push(SplineHeader {
                guid: u64::from(index),
                local_guid: u64::from(index),
                instance: ObjectId(1),
                head_offset: Some(segment_offset),
                tail_offset: Some(segment_offset),
            });
            segments.push(segment(
                [[0.0; 4], [0.0; 4], [0.25, 0.0, 0.0, 0.0], [0.0; 4]],
                [-0.5; 4],
                [0.5; 4],
                16 + index * 32,
            ));
        }
        let data = SplineData {
            splines_offset: 16,
            segments_offset,
            splines,
            segments,
        };
        let body = LivingWorldGrindBody::from_spline_headers(
            translated_transform([0.0; 3]),
            1,
            2,
            &data,
            0..65,
        );
        assert_eq!(
            body.grind_assets.len(),
            MAX_LIVING_WORLD_GRIND_SPLINES_PER_BODY
        );
        assert_eq!(
            body.grind_assets.last().unwrap().primitives[0].source_segment_index,
            63
        );
    }

    #[test]
    fn living_world_query_orders_managers_and_uses_trace_fed_legacy_bits() {
        let bounds = GrindBounds {
            minimum: [-0.5; 3],
            maximum: [0.5; 3],
        };
        let dynamic_object = LivingWorldGrindBody {
            transform: translated_transform([0.0; 3]),
            grind_assets: vec![dynamic_asset(
                10,
                [-0.25, 0.0, 0.0],
                [0.25, 0.0, 0.0],
                bounds,
            )],
        };
        let vehicle = LivingWorldGrindBody {
            transform: translated_transform([0.0; 3]),
            grind_assets: vec![dynamic_asset(
                20,
                [-0.25, 0.0, 0.0],
                [0.25, 0.0, 0.0],
                bounds,
            )],
        };
        let scene = LivingWorldGrindScene {
            legacy_flags: LivingWorldLegacyGrindFlags {
                dynamic_objects_are_vertical: true,
                vehicles_are_vertical: false,
            },
            dynamic_objects: vec![dynamic_object],
            vehicles: vec![vehicle],
            ..LivingWorldGrindScene::default()
        };

        let results = scene.box_test(GrindBounds {
            minimum: [-1.0; 3],
            maximum: [1.0; 3],
        });
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source_asset_id, 10);
        assert!(results[0].is_vertical);
        assert_eq!(results[1].source_asset_id, 20);
        assert!(!results[1].is_vertical);
    }

    #[test]
    fn living_world_query_honors_gate_and_strict_radius_boundary() {
        let local_bounds = GrindBounds {
            minimum: [-15.5, -0.5, -0.5],
            maximum: [-14.5, 0.5, 0.5],
        };
        let boundary_object = LivingWorldGrindBody {
            transform: translated_transform([15.0, 0.0, 0.0]),
            grind_assets: vec![dynamic_asset(
                10,
                [-15.25, 0.0, 0.0],
                [-14.75, 0.0, 0.0],
                local_bounds,
            )],
        };
        let vehicle = LivingWorldGrindBody {
            transform: translated_transform([0.0; 3]),
            grind_assets: vec![dynamic_asset(
                20,
                [-0.25, 0.0, 0.0],
                [0.25, 0.0, 0.0],
                GrindBounds {
                    minimum: [-0.5; 3],
                    maximum: [0.5; 3],
                },
            )],
        };
        let scene = LivingWorldGrindScene {
            tuning: LivingWorldGrindTuning {
                feed_dynamic_object_splines: false,
                ..LivingWorldGrindTuning::default()
            },
            dynamic_objects: vec![boundary_object],
            vehicles: vec![vehicle],
            ..LivingWorldGrindScene::default()
        };

        let results = scene.box_test(GrindBounds {
            minimum: [-1.0; 3],
            maximum: [1.0; 3],
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_asset_id, 20);

        let mut enabled = scene;
        enabled.tuning.feed_dynamic_object_splines = true;
        let results = enabled.box_test(GrindBounds {
            minimum: [-1.0; 3],
            maximum: [1.0; 3],
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_asset_id, 20);
    }

    #[test]
    fn transformed_provider_stops_at_shared_result_capacity() {
        let mut data = SplineData {
            splines_offset: 16,
            segments_offset: 48,
            splines: vec![],
            segments: vec![],
        };
        for index in 0..45_u16 {
            let offset = f32::from(index) * 0.001;
            data.segments.push(segment(
                [
                    [0.0; 4],
                    [0.0; 4],
                    [0.5, 0.0, 0.0, 0.0],
                    [offset, 0.0, 0.0, 1.0],
                ],
                [-0.1, -0.1, -0.1, 0.0],
                [0.7, 0.1, 0.1, 0.0],
                16,
            ));
        }
        let asset = GrindAsset::from_spline_data(30, 0, &data);
        let results = transformed_dynamic_grind_box_test(
            [(&asset, translated_transform([0.0; 3]))],
            GrindBounds {
                minimum: [-1.0; 3],
                maximum: [1.0; 3],
            },
        );
        assert_eq!(results.len(), MAX_GRIND_QUERY_RESULTS);
        assert_eq!(results.last().unwrap().source_segment_index, 39);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn considers_linear_grind_plane_and_extended_primitive_bounds() {
        let trajectory = BallisticTrajectory {
            initial_position: [1.0, 1.0, 5.0],
            initial_velocity: [-1.0, 0.0, 0.0],
            gravity: [0.0, -9.8, 0.0],
            duration: 4.0,
        };
        let primitive = GrindPrimitive {
            start_point: [0.0, 0.0, 0.0],
            end_point: [0.0, 0.0, 10.0],
            spline_header_offset: Some(16),
            is_vertical: false,
            source_asset_id: 1,
            source_dictionary_index: 2,
            source_segment_index: 3,
        };
        let result = consider_grind_primitive(&trajectory, 0.2, -0.2, primitive, 7).unwrap();
        assert_eq!(result.closest_point_on_primitive, [0.0, 0.0, 5.0]);
        assert_eq!(result.collision_point, [0.0, -3.9, 5.0]);
        assert_eq!(result.grind_direction, [0.0, 0.0, 1.0]);
        assert_eq!(result.trajectory_direction_in_grind_plane, [0.0; 3]);
        assert!((result.distance_to_grind - 3.9).abs() < 0.000_001);
        assert_eq!(result.grind_plane_penetration_time, 1.0);
        assert_eq!(result.grind_plane_penetration_index, 59);
        assert!((result.landing_angle - PI * 0.5).abs() < 0.000_001);
        assert_eq!(result.grind_index, 7);

        let outside = GrindPrimitive {
            start_point: [0.0, 0.0, -20.0],
            end_point: [0.0, 0.0, -10.0],
            ..primitive
        };
        assert!(consider_grind_primitive(&trajectory, 0.2, -0.2, outside, 0).is_none());
    }

    #[test]
    fn best_grind_prefers_first_nearest_unlocked_candidate_and_removes_it() {
        let mut candidates = vec![
            grind_result(2.0, 0.0, 0.5, 10),
            grind_result(1.0, 0.0, 0.4, 11),
            grind_result(1.0, 0.0, 0.1, 12),
        ];
        let selected = find_best_grind(&mut candidates, 0.9, &PointGraph::default()).unwrap();

        assert_eq!(selected.grind_index, 11);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.grind_index)
                .collect::<Vec<_>>(),
            vec![10, 12]
        );
    }

    #[test]
    fn paired_grinds_at_the_same_height_keep_the_one_closest_to_takeoff() {
        let primitives = [
            grind_primitive([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], 0),
            grind_primitive([0.0, 0.0, 0.05], [10.0, 0.0, 0.05], 1),
        ];
        let filtered = filter_trajectory_grind_pairs(
            &primitives,
            [5.0, 0.0, 0.0],
            GrindPairFilterTuning::default(),
        );

        assert_eq!(filtered, vec![primitives[0]]);
    }

    #[test]
    fn paired_grinds_more_than_vertical_tolerance_keep_the_upper_rail() {
        let primitives = [
            grind_primitive([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], 0),
            grind_primitive([0.0, 0.05, 0.05], [10.0, 0.05, 0.05], 1),
        ];
        let filtered = filter_trajectory_grind_pairs(
            &primitives,
            [5.0, 0.0, 0.0],
            GrindPairFilterTuning::default(),
        );

        assert_eq!(filtered, vec![primitives[1]]);
    }

    #[test]
    fn grind_pair_filter_preserves_nonparallel_and_boundary_separations() {
        let primitives = [
            grind_primitive([0.0, 0.0, 0.0], [10.0, 0.0, 0.0], 0),
            grind_primitive([0.0, 0.0, 0.01], [10.0, 0.0, 0.01], 1),
            grind_primitive([5.0, 0.0, -5.0], [5.0, 0.0, 5.0], 2),
        ];
        let filtered = filter_trajectory_grind_pairs(
            &primitives,
            [5.0, 0.0, 0.0],
            GrindPairFilterTuning::default(),
        );

        assert_eq!(filtered, primitives);
    }

    #[test]
    fn locked_grind_replacement_uses_height_graph_and_strict_angle_improvement() {
        let graph = PointGraph {
            x: [-2.0, -1.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            y: [0.25; 8],
        };
        let mut candidates = vec![
            grind_result(0.5, 0.0, 1.0, 20),
            // Exactly the graph threshold does not replace due to strict `>`.
            grind_result(0.4, 1.0, 0.75, 21),
            grind_result(0.8, -1.0, 0.4, 22),
            // An unlocked candidate cannot replace a locked winner.
            grind_result(1.0, 0.0, 0.0, 23),
        ];
        let selected = find_best_grind(&mut candidates, 0.9, &graph).unwrap();

        assert_eq!(selected.grind_index, 22);
        assert_eq!(candidates.len(), 3);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn grind_approach_metrics_remove_rail_parallel_and_vertical_components() {
        let horizontal = grind_approach_metrics([3.0, -4.0, 5.0], [1.0, 0.0, 0.0]);
        assert_eq!(horizontal.vertical_speed, -4.0);
        assert_eq!(horizontal.lateral_speed_squared, 25.0);

        let vertical = grind_approach_metrics([3.0, -4.0, 5.0], [0.0, 1.0, 0.0]);
        assert_eq!(vertical.vertical_speed, 0.0);
        assert_eq!(vertical.lateral_speed_squared, 34.0);
    }

    #[test]
    #[allow(clippy::float_cmp)] // The interval-containing branch returns the input exactly.
    fn tilted_upmost_normal_uses_containing_nearest_and_wrapped_intervals() {
        let mut geometry = analyzed_geometry(GrindGeometryType::ThinRail);
        assert_eq!(tilted_upmost_grind_normal(&geometry), [0.0, 1.0, 0.0]);

        geometry.low_normal_limit_angle = PI * 0.75;
        geometry.high_normal_limit_angle = PI * 0.9;
        let nearest = tilted_upmost_grind_normal(&geometry);
        assert!((nearest[1] - (PI * 0.1).cos()).abs() < 0.000_001);
        assert!((nearest[2] + (PI * 0.1).sin()).abs() < 0.000_001);

        geometry.low_normal_limit_angle = PI;
        geometry.high_normal_limit_angle = PI;
        let wrapped = tilted_upmost_grind_normal(&geometry);
        assert!(wrapped[0].abs() < 0.000_001);
        assert!((wrapped[1] - 1.0).abs() < 0.000_001);
        assert!(wrapped[2].abs() < 0.000_001);
    }

    #[test]
    #[allow(clippy::float_cmp)] // The uncertain fallback is constant reference data.
    fn uncertain_grind_geometry_uses_reference_garbage_fallback() {
        let mut geometry = analyzed_geometry(GrindGeometryType::Ledge);
        geometry.uncertain = true;
        let result = grind_orientations_on_landing(
            &geometry,
            [0.0; 3],
            [0.0; 3],
            GrindOrientationTuning::default(),
        );

        assert_eq!(result.selected_up_normal, [0.0, 1.0, 0.0]);
        assert!(result.possible.is_garbage);
        assert_eq!(result.possible.geometry_type, GrindGeometryType::ThinRail);
        assert_eq!(result.possible.boardslide_direction, [1.0, 0.0, 0.0]);
        assert_eq!(result.possible.tipslide_direction, [1.0, 0.0, 0.0]);
        assert_eq!(result.possible.backslash_direction, [-1.0, 0.0, 0.0]);
        assert_eq!(result.possible.high_side_direction, [1.0, 0.0, 0.0]);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned components remain exact.
    fn thin_grind_orientations_use_takeoff_side_and_authored_tilts() {
        let geometry = analyzed_geometry(GrindGeometryType::ThinRail);
        let result = grind_orientations_on_landing(
            &geometry,
            [0.0, 0.0, 2.0],
            [0.0; 3],
            GrindOrientationTuning::default(),
        );
        let tipslide = 20.0_f32.to_radians();
        let backslash = 42.0_f32.to_radians();

        assert_eq!(result.possible.boardslide_direction, [0.0, 0.0, 1.0]);
        assert!(result.possible.tipslide_direction[0].abs() < 0.000_001);
        assert!((result.possible.tipslide_direction[1] - tipslide.sin()).abs() < 0.000_001);
        assert!((result.possible.tipslide_direction[2] - tipslide.cos()).abs() < 0.000_001);
        assert!(result.possible.backslash_direction[0].abs() < 0.000_001);
        assert!((result.possible.backslash_direction[1] - backslash.sin()).abs() < 0.000_001);
        assert!((result.possible.backslash_direction[2] + backslash.cos()).abs() < 0.000_001);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned components remain exact.
    fn fat_grind_orientations_split_tipslide_and_backslash_across_high_side() {
        let geometry = analyzed_geometry(GrindGeometryType::FatRail);
        let result = grind_orientations_on_landing(
            &geometry,
            [0.0; 3],
            [0.0; 3],
            GrindOrientationTuning::default(),
        );
        let tipslide = 20.0_f32.to_radians();
        let backslash = 42.0_f32.to_radians();

        assert_eq!(result.possible.boardslide_direction, [0.0, 0.0, 1.0]);
        assert!((result.possible.tipslide_direction[1] - tipslide.sin()).abs() < 0.000_001);
        assert!((result.possible.tipslide_direction[2] + tipslide.cos()).abs() < 0.000_001);
        assert!((result.possible.backslash_direction[1] - backslash.sin()).abs() < 0.000_001);
        assert!((result.possible.backslash_direction[2] - backslash.cos()).abs() < 0.000_001);
    }

    #[test]
    fn grind_geometry_hit_patterns_preserve_reference_classification_boundaries() {
        let tuning = GrindGeometryTuning::default();
        let mut collisions = [GrindGeometryCollision::default(); 5];

        assert_eq!(
            classify_grind_geometry_hits(&collisions, &tuning),
            GrindGeometryHitClassification {
                geometry_type: GrindGeometryType::ThinRail,
                reverse_high_side_direction: false,
                is_apex: false,
            }
        );

        collisions[0].hit = true;
        collisions[2].hit = true;
        collisions[2].normal[1] = 0.649_999_9;
        assert_eq!(
            classify_grind_geometry_hits(&collisions, &tuning).geometry_type,
            GrindGeometryType::Ledge
        );
        collisions[2].normal[1] = tuning.coarse_curb_parameter;
        assert_eq!(
            classify_grind_geometry_hits(&collisions, &tuning).geometry_type,
            GrindGeometryType::FatRail
        );

        collisions = [GrindGeometryCollision::default(); 5];
        collisions[1].hit = true;
        let second_side = classify_grind_geometry_hits(&collisions, &tuning);
        assert_eq!(second_side.geometry_type, GrindGeometryType::FatRail);
        assert!(second_side.reverse_high_side_direction);
        assert!(!second_side.is_apex);

        collisions[0].hit = true;
        assert_eq!(
            classify_grind_geometry_hits(&collisions, &tuning),
            GrindGeometryHitClassification {
                geometry_type: GrindGeometryType::Impossible,
                reverse_high_side_direction: false,
                is_apex: true,
            }
        );
    }

    #[test]
    fn stair_and_curb_recognition_preserve_packed_surface_and_strict_windows() {
        let tuning = GrindGeometryTuning::default();
        let mut collisions = [GrindGeometryCollision::default(); 5];

        collisions[0].hit = true;
        collisions[0].surface_id = 0x400;
        assert!(grind_geometry_is_stair(&collisions));
        collisions[0].surface_id = 0x380;
        assert!(!grind_geometry_is_stair(&collisions));

        collisions[0] = GrindGeometryCollision::default();
        collisions[2].hit = true;
        collisions[2].normal = [0.0, 1.0, 0.0];
        collisions[2].position = [0.0, 0.0, 0.0];
        collisions[3].hit = true;
        collisions[3].normal = [0.0, 1.0, 0.0];
        collisions[3].position = [0.0, -0.22, 0.0];
        assert!(grind_geometry_is_curb(
            &collisions,
            [0.0, 1.0, 0.0],
            [0.0; 3],
            GrindGeometryType::Ledge,
            &tuning,
        ));

        collisions[2].normal[1] = tuning.curb_normal_likeness;
        assert!(!grind_geometry_is_curb(
            &collisions,
            [0.0, 1.0, 0.0],
            [0.0; 3],
            GrindGeometryType::Ledge,
            &tuning,
        ));
    }

    #[test]
    fn grind_surface_resolution_preserves_overrides_priority_and_ties() {
        let mut collisions = [GrindGeometryCollision::default(); 5];
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::FatRail, false),
            GrindSurfaceInfo {
                audio_surface_raw: 3,
                physics_surface_raw: 1,
            }
        );
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::Ledge, true),
            GrindSurfaceInfo {
                audio_surface_raw: 4,
                physics_surface_raw: 2,
            }
        );
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::ThinRail, false),
            GrindSurfaceInfo {
                audio_surface_raw: 11,
                physics_surface_raw: 4,
            }
        );

        collisions[4].hit = true;
        collisions[4].surface_id = 0x4D | (9 << 7);
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::ThinRail, false),
            GrindSurfaceInfo {
                audio_surface_raw: 77,
                physics_surface_raw: 4,
            }
        );

        collisions[0].hit = true;
        collisions[0].position[1] = 2.0;
        collisions[0].surface_id = 0x0C | (6 << 7);
        collisions[1].hit = true;
        collisions[1].position[1] = 1.0;
        collisions[1].surface_id = 0x1F | (8 << 7);
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::FatRail, false),
            GrindSurfaceInfo {
                audio_surface_raw: 12,
                physics_surface_raw: 6,
            }
        );

        collisions[1].position[1] = 2.0;
        assert_eq!(
            resolve_grind_surface_info(&collisions, GrindGeometryType::FatRail, false),
            GrindSurfaceInfo {
                audio_surface_raw: 31,
                physics_surface_raw: 8,
            }
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned authored arithmetic is intentional.
    fn grind_geometry_probe_batch_preserves_line_order_signs_and_far_fallbacks() {
        let batch = build_grind_geometry_probe_batch(
            0.1,
            0.2,
            0.3,
            0.4,
            [1.0, 2.0, 3.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0],
        )
        .unwrap();

        assert_eq!(
            batch.lines,
            [
                GrindGeometryProbeLine {
                    high_point: [1.0, 2.2, 3.1],
                    low_point: [1.0, 1.8, 3.1],
                },
                GrindGeometryProbeLine {
                    high_point: [1.0, 2.2, 2.9],
                    low_point: [1.0, 1.8, 2.9],
                },
                GrindGeometryProbeLine {
                    high_point: [1.0, 2.4, 3.3],
                    low_point: [1.0, 1.6, 3.3],
                },
                GrindGeometryProbeLine {
                    high_point: [1.0, 2.4, 2.7],
                    low_point: [1.0, 1.6, 2.7],
                },
                GrindGeometryProbeLine {
                    high_point: [1.0, 2.2, 3.0],
                    low_point: [1.0, 1.8, 3.0],
                },
            ]
        );

        let mut collisions = [GrindGeometryCollision::default(); 5];
        collisions[2].hit = true;
        collisions[2].position = [7.0, 8.0, 9.0];
        assert_eq!(
            resolve_grind_far_low_points(&batch, &collisions),
            [[7.0, 8.0, 9.0], [1.0, 1.6, 2.7]]
        );
    }

    #[test]
    fn island_batch_merge_keeps_strictly_nearest_hit_and_earlier_ties() {
        assert_eq!(GRIND_GEOMETRY_BATCH_FLAGS, 7);
        let mut ground = [GrindGeometryCollision::default(); 5];
        ground[0] = GrindGeometryCollision {
            hit: true,
            line_parameter: 0.4,
            surface_id: 10,
            ..GrindGeometryCollision::default()
        };
        ground[1] = GrindGeometryCollision {
            hit: true,
            line_parameter: 0.3,
            surface_id: 11,
            ..GrindGeometryCollision::default()
        };
        let mut dynamic_objects = [GrindGeometryCollision::default(); 5];
        dynamic_objects[0] = GrindGeometryCollision {
            hit: true,
            line_parameter: 0.2,
            surface_id: 20,
            ..GrindGeometryCollision::default()
        };
        dynamic_objects[1] = GrindGeometryCollision {
            hit: true,
            line_parameter: 0.3,
            surface_id: 21,
            ..GrindGeometryCollision::default()
        };
        let mut vehicles = [GrindGeometryCollision::default(); 5];
        vehicles[0] = GrindGeometryCollision {
            hit: true,
            line_parameter: 0.25,
            surface_id: 30,
            ..GrindGeometryCollision::default()
        };

        let merged = merge_grind_geometry_collision_batches([&ground, &dynamic_objects, &vehicles]);
        assert_eq!(merged[0].surface_id, 20);
        assert_eq!(merged[1].surface_id, 11);
        assert!(!merged[2].hit);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned projection is exact.
    fn grind_geometry_preparation_builds_projected_basis_and_uncertain_escape() {
        let tuning = GrindGeometryTuning {
            collision_offset: 0.243,
            ..GrindGeometryTuning::default()
        };
        let horizontal = grind_primitive([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0);
        let PreparedGrindGeometry::Query(query) =
            prepare_grind_geometry(&horizontal, [1.0, 2.0, 0.0], &tuning).unwrap()
        else {
            panic!("horizontal rail must produce a query");
        };
        assert_eq!(query.position, [1.0, 0.0, 0.0]);
        assert_eq!(query.upmost_normal, [0.0, 1.0, 0.0]);
        assert_eq!(query.primitive_direction, [1.0, 0.0, 0.0]);
        assert_eq!(query.high_side_direction, [0.0, 0.0, 1.0]);
        assert_eq!(query.batch.lines[0].high_point, [1.0, 0.07, 0.09]);
        assert_eq!(query.batch.lines[2].high_point, [1.0, 1.06 * 0.243, 0.243]);

        let vertical = grind_primitive([0.0, 0.0, 0.0], [0.0, 2.0, 0.0], 1);
        let PreparedGrindGeometry::Uncertain(uncertain) =
            prepare_grind_geometry(&vertical, [4.0, 5.0, 6.0], &tuning).unwrap()
        else {
            panic!("vertical rail must use the uncertain escape");
        };
        assert_eq!(
            uncertain.line_positions,
            [[4.243, 5.0, 6.0], [3.757, 5.0, 6.0]]
        );
        assert_eq!(uncertain.upmost_normal, [0.0, 0.0, 1.0]);
        assert_eq!(
            uncertain.surface_info,
            GrindSurfaceInfo {
                audio_surface_raw: 3,
                physics_surface_raw: 1,
            }
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned basis vectors remain exact.
    fn grind_geometry_finish_composes_thin_defaults_and_normal_interval() {
        let tuning = GrindGeometryTuning {
            collision_offset: 0.243,
            ..GrindGeometryTuning::default()
        };
        let primitive = grind_primitive([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0);
        let PreparedGrindGeometry::Query(query) =
            prepare_grind_geometry(&primitive, [1.0, 0.0, 0.0], &tuning).unwrap()
        else {
            panic!("horizontal rail must produce a query");
        };
        let geometry =
            finish_grind_geometry(&query, &[GrindGeometryCollision::default(); 5], &tuning)
                .unwrap();

        assert_eq!(geometry.analysis.geometry_type, GrindGeometryType::ThinRail);
        assert_eq!(geometry.analysis.high_side_direction, [0.0, 0.0, 1.0]);
        assert!(!geometry.is_stair);
        assert!(!geometry.is_apex);
        assert!(!geometry.is_curb);
        assert_eq!(
            geometry.surface_info,
            GrindSurfaceInfo {
                audio_surface_raw: 11,
                physics_surface_raw: 4,
            }
        );
        assert!(geometry.analysis.low_normal_limit_angle < PI);
        assert!(geometry.analysis.high_normal_limit_angle > PI);
    }

    #[test]
    fn grind_geometry_orchestration_preserves_batched_query_and_uncertain_bypass() {
        let tuning = GrindGeometryTuning {
            collision_offset: 0.243,
            ..GrindGeometryTuning::default()
        };
        let horizontal = grind_primitive([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0);
        let calculated = calculate_grind_geometry(
            &horizontal,
            [1.0, 0.0, 0.0],
            &tuning,
            |batch| -> Result<_, ()> {
                assert_eq!(batch.lines.len(), 5);
                Ok([GrindGeometryCollision::default(); 5])
            },
        )
        .unwrap()
        .unwrap();
        assert!(matches!(calculated, GrindGeometryCalculation::Certain(_)));

        let vertical = grind_primitive([0.0, 0.0, 0.0], [0.0, 2.0, 0.0], 1);
        let uncertain = calculate_grind_geometry(
            &vertical,
            [0.0; 3],
            &tuning,
            |_| -> Result<[GrindGeometryCollision; 5], ()> {
                panic!("uncertain geometry must not submit a line batch")
            },
        )
        .unwrap()
        .unwrap();
        assert!(matches!(uncertain, GrindGeometryCalculation::Uncertain(_)));
    }

    #[test]
    fn landing_assessment_connects_ledge_lock_and_uncertain_fallback() {
        let trajectory_tuning = TrajectorySelectionTuning {
            grind_lock_to_tip_distance: 2.0,
            grind_lock_to_centre_distance: 0.9,
            grind_lock_ledge_scalar_squared: 0.35,
            grind_lock_ledge_scalar: 0.5,
            grind_lock_ledge_low_side_scalar_squared: 0.65,
            grind_lock_ledge_low_side_scalar: 1.0,
            ..TrajectorySelectionTuning::default()
        };
        let certain = GrindGeometryCalculation::Certain(CalculatedGrindGeometry {
            line_positions: [[0.0; 3]; 2],
            analysis: analyzed_geometry(GrindGeometryType::Ledge),
            surface_info: GrindSurfaceInfo {
                audio_surface_raw: 7,
                physics_surface_raw: 2,
            },
            is_stair: true,
            is_apex: false,
            is_curb: true,
        });
        let ledge = assess_grind_geometry_on_landing(
            &certain,
            [0.0, 0.0, 1.0],
            [0.0; 3],
            &trajectory_tuning,
            GrindOrientationTuning::default(),
        );
        assert_eq!(
            ledge.assessment,
            GrindGeometryAssessment::Ledge {
                maximum_lock_distance: 1.0,
            }
        );
        assert!(ledge.is_stair);
        assert!(ledge.is_curb);

        let uncertain = GrindGeometryCalculation::Uncertain(UncertainGrindGeometry {
            line_positions: [[0.0; 3]; 2],
            upmost_normal: [0.0, 0.0, 1.0],
            primitive_direction: [0.0, 1.0, 0.0],
            surface_info: GrindSurfaceInfo {
                audio_surface_raw: 3,
                physics_surface_raw: 1,
            },
        });
        let fallback = assess_grind_geometry_on_landing(
            &uncertain,
            [0.0; 3],
            [0.0; 3],
            &trajectory_tuning,
            GrindOrientationTuning::default(),
        );
        assert_eq!(fallback.assessment, GrindGeometryAssessment::ThinRail);
        assert!(fallback.landing_orientation.possible.is_garbage);
        assert_eq!(
            fallback.landing_orientation.possible.geometry_type,
            GrindGeometryType::ThinRail
        );
    }

    #[test]
    fn grind_geometry_probe_batch_preserves_strict_debug_preconditions() {
        assert!(
            build_grind_geometry_probe_batch(
                0.1,
                0.2,
                0.3,
                0.4,
                [0.0; 3],
                [0.98, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            )
            .is_none()
        );
        assert!(
            build_grind_geometry_probe_batch(
                0.1,
                0.2,
                0.3,
                0.0,
                [0.0; 3],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            )
            .is_none()
        );
    }

    #[test]
    fn grind_normal_limits_use_far_low_vectors_and_wheel_clearance() {
        let wheel = 16.0_f32.to_radians();
        let base = (1.0_f32 / 2.0_f32.sqrt()).acos() + PI * 0.5;
        let limits = calculate_grind_normal_limit_angles(
            [0.0; 3],
            [0.0, 1.0, 0.0],
            [[0.0, -1.0, 1.0], [0.0, -1.0, -1.0]],
            16.0,
        )
        .unwrap();

        assert!((limits.0 - (base + wheel)).abs() < 0.000_001);
        assert!((limits.1 - (PI * 2.0 - base - wheel)).abs() < 0.000_001);
        assert!(limits.0 < PI);
        assert!(limits.1 > PI);
        assert!(
            calculate_grind_normal_limit_angles(
                [0.0; 3],
                [0.0, 1.0, 0.0],
                [[0.0; 3], [0.0, -1.0, 0.0]],
                16.0,
            )
            .is_none()
        );
    }

    #[test]
    fn grind_approach_speed_gates_preserve_geometry_limits_and_boundaries() {
        let ledge_boundary = GrindApproachMetrics {
            vertical_speed: -7.0,
            lateral_speed_squared: 36.0,
        };
        assert!(grind_approach_speed_is_acceptable(
            ledge_boundary,
            GrindGeometryType::Ledge,
            7.0,
            36.0,
            64.0,
        ));
        assert!(!grind_approach_speed_is_acceptable(
            GrindApproachMetrics {
                vertical_speed: 7.000_1,
                ..ledge_boundary
            },
            GrindGeometryType::Ledge,
            7.0,
            36.0,
            64.0,
        ));
        assert!(grind_approach_speed_is_acceptable(
            GrindApproachMetrics {
                lateral_speed_squared: 64.0,
                ..ledge_boundary
            },
            GrindGeometryType::FatRail,
            7.0,
            36.0,
            64.0,
        ));
        assert!(!grind_approach_speed_is_acceptable(
            ledge_boundary,
            GrindGeometryType::Impossible,
            7.0,
            36.0,
            64.0,
        ));
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact branches and authored f32 arithmetic are intentional.
    fn ledge_lock_distance_preserves_high_side_and_landing_up_sign_branches() {
        let tuning = TrajectorySelectionTuning {
            grind_lock_to_tip_distance: 2.0,
            grind_lock_to_centre_distance: 0.9,
            grind_lock_ledge_scalar_squared: 0.35,
            grind_lock_ledge_scalar: 0.5,
            grind_lock_ledge_low_side_scalar_squared: 0.65,
            grind_lock_ledge_low_side_scalar: 1.0,
            ..TrajectorySelectionTuning::default()
        };

        assert_eq!(grind_ledge_maximum_lock_distance(1.0, 1.0, &tuning), 0.7);
        assert_eq!(grind_ledge_maximum_lock_distance(1.0, 0.0, &tuning), 1.0);
        assert_eq!(
            grind_ledge_maximum_lock_distance(0.0, 1.0, &tuning),
            0.65_f32.mul_add(2.0, 0.9)
        );
        assert_eq!(grind_ledge_maximum_lock_distance(0.0, 0.0, &tuning), 2.9);
    }

    #[test]
    fn grind_adjustment_selection_removes_rejections_in_reference_order() {
        let mut candidates = vec![
            grind_result(1.0, 0.0, 0.1, 0),
            grind_result(2.0, 0.0, 0.1, 1),
            grind_result(3.0, 0.0, 0.1, 2),
            grind_result(4.0, 0.0, 0.1, 3),
        ];
        for candidate in &mut candidates[1..] {
            candidate.grind_direction = [0.0, 1.0, 0.0];
        }
        let primitives = (0..4)
            .map(|index| grind_primitive([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], index))
            .collect::<Vec<_>>();
        let tuning = TrajectorySelectionTuning {
            grind_lock_to_centre_distance: 0.1,
            grind_lock_to_tip_distance: 10.0,
            grind_maximum_speed_down_onto_grind: 7.0,
            grind_maximum_speed_squared_into_ledge: 36.0,
            grind_maximum_speed_squared_into_grind: 64.0,
            ..TrajectorySelectionTuning::default()
        };
        let mut analyzed = Vec::new();
        let accepted = select_grind_for_adjustment(
            &mut candidates,
            &primitives,
            [5.0, -8.0, 0.0],
            &tuning,
            |_, result| {
                analyzed.push(result.grind_index);
                match result.grind_index {
                    1 => GrindGeometryAssessment::Impossible,
                    2 => GrindGeometryAssessment::Ledge {
                        maximum_lock_distance: 2.0,
                    },
                    _ => GrindGeometryAssessment::ThinRail,
                }
            },
        )
        .unwrap();

        assert_eq!(accepted.trajectory_result.grind_index, 3);
        assert_eq!(analyzed, vec![1, 2, 3]);
        assert!(candidates.is_empty());
    }

    #[test]
    fn analyzer_backed_selection_retries_impossible_geometry_and_retains_details() {
        let mut candidates = vec![
            grind_result(0.1, 0.0, 0.2, 0),
            grind_result(0.2, 0.0, 0.3, 1),
        ];
        let primitives = [
            grind_primitive([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0),
            grind_primitive([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1),
        ];
        let trajectory_tuning = TrajectorySelectionTuning {
            grind_lock_to_centre_distance: 0.05,
            grind_lock_to_tip_distance: 1.0,
            grind_maximum_speed_down_onto_grind: 7.0,
            grind_maximum_speed_squared_into_ledge: 36.0,
            grind_maximum_speed_squared_into_grind: 64.0,
            ..TrajectorySelectionTuning::default()
        };
        let geometry_tuning = GrindGeometryTuning {
            collision_offset: 0.243,
            ..GrindGeometryTuning::default()
        };
        let mut queried = Vec::new();
        let accepted = select_grind_for_adjustment_with_geometry(
            &mut candidates,
            &primitives,
            [0.0; 3],
            [0.0, 0.0, 1.0],
            GrindAdjustmentAnalysisTuning {
                trajectory: &trajectory_tuning,
                geometry: &geometry_tuning,
                orientation: GrindOrientationTuning::default(),
            },
            |primitive, _| -> Result<_, ()> {
                queried.push(primitive.source_segment_index);
                let mut collisions = [GrindGeometryCollision::default(); 5];
                if primitive.source_segment_index == 0 {
                    collisions[0].hit = true;
                    collisions[1].hit = true;
                }
                Ok(collisions)
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(queried, [0, 1]);
        assert_eq!(accepted.accepted.trajectory_result.grind_index, 1);
        assert_eq!(
            accepted.accepted.geometry,
            GrindGeometryAssessment::ThinRail
        );
        assert_eq!(accepted.geometry.surface_info.physics_surface_raw, 4);
        assert!(!accepted.geometry.landing_orientation.possible.is_garbage);
        assert!(candidates.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn grind_snap_aims_centre_or_retains_the_authored_deck_tip_distance() {
        let tuning = TrajectorySelectionTuning {
            grind_lock_to_centre_distance: 0.9,
            grind_tip_scalar: 0.7,
            ..TrajectorySelectionTuning::default()
        };
        let deck = GrindDeckTuning {
            contact_footprint_radius: 0.3775,
        };
        let mut selected = grind_result(0.0, 0.0, 0.0, 0);
        selected.closest_point_on_primitive = [0.0, 0.0, 0.0];
        selected.collision_point = [0.8, 0.0, 0.0];
        assert_eq!(
            grind_snap_displacement(&selected, &tuning, deck),
            [-0.8, 0.0, 0.0]
        );

        selected.collision_point = [1.0, 0.0, 0.0];
        assert_eq!(
            grind_snap_displacement(&selected, &tuning, deck),
            [-0.73575, 0.0, 0.0]
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // The reference's zero-angle fallback is exact.
    fn grind_snap_angle_uses_continuous_penetration_time_and_zero_fallback() {
        let trajectory = BallisticTrajectory {
            initial_position: [0.0; 3],
            initial_velocity: [10.0, 0.0, 0.0],
            gravity: [0.0, -9.8, 0.0],
            duration: 2.0,
        };
        let mut selected = grind_result(1.0, 0.0, 0.0, 0);
        selected.grind_plane_penetration_time = 0.5;
        selected.grind_plane_penetration_index = 12;
        let angle = grind_snap_velocity_redirection_angle(&trajectory, &selected, [0.0, 1.0, 0.0]);
        assert!((angle - 0.2_f32.atan()).abs() < 0.000_001);

        let stopped = BallisticTrajectory {
            initial_velocity: [0.0; 3],
            ..trajectory
        };
        assert_eq!(
            grind_snap_velocity_redirection_angle(&stopped, &selected, [0.0, 1.0, 0.0]),
            0.0
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact snap components are part of the recovered contract.
    fn grind_adjustment_retries_after_angle_rejection() {
        let mut trajectory = BallisticTrajectory {
            initial_position: [0.0; 3],
            initial_velocity: [10.0, 0.0, 0.0],
            gravity: [0.0, -9.8, 0.0],
            duration: 2.0,
        };
        let mut candidates = vec![
            grind_result(0.2, 1.0, 0.0, 0),
            grind_result(0.3, 0.1, 0.0, 1),
        ];
        candidates[0].grind_plane_penetration_time = 0.1;
        candidates[1].grind_plane_penetration_time = 1.0;
        candidates[1].grind_plane_penetration_index = 60;
        let primitives = [
            grind_primitive([0.0; 3], [1.0, 0.0, 0.0], 0),
            grind_primitive([0.0; 3], [1.0, 0.0, 0.0], 1),
        ];
        let tuning = TrajectorySelectionTuning {
            grind_lock_to_centre_distance: 0.1,
            grind_lock_to_tip_distance: 10.0,
            grind_maximum_speed_down_onto_grind: 7.0,
            grind_maximum_speed_squared_into_ledge: 36.0,
            grind_maximum_speed_squared_into_grind: 64.0,
            grind_adjustment_maximum_angle: 11.0,
            grind_landing_velocity_scalar: 0.1,
            grind_landing_maximum_angle: 20.0,
            maximum_trajectory_adjustment: 1.0,
            ..TrajectorySelectionTuning::default()
        };
        let mut analyzed = Vec::new();
        let plan = plan_grind_adjustment(
            &mut candidates,
            &primitives,
            &trajectory,
            [0.0; 3],
            &tuning,
            GrindDeckTuning::default(),
            |_, selected| {
                analyzed.push(selected.grind_index);
                GrindGeometryAssessment::ThinRail
            },
        )
        .unwrap();

        assert_eq!(plan.accepted_candidate.trajectory_result.grind_index, 1);
        assert_eq!(plan.snap_displacement, [0.0, 0.1, 0.0]);
        assert!(plan.velocity_redirection_angle < 11.0_f32.to_radians());
        assert_eq!(analyzed, vec![0, 1]);
        assert!(candidates.is_empty());

        let applied = apply_grind_adjustment(&mut trajectory, &plan, [0.0, 1.0, 0.0], &tuning);
        assert_eq!(trajectory.initial_velocity, [10.0, 0.1, 0.0]);
        assert_eq!(applied.collision_frame, 60);
        assert_eq!(applied.collision_time, 1.0);
        assert!(applied.landing_normal[0].abs() < 0.000_001);
        assert!((applied.landing_normal[1] - 1.0).abs() < 0.000_001);
        assert!(applied.landing_normal[2].abs() < 0.000_001);
        assert_eq!(applied.most_vertical_normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Axis-aligned reference outputs are exact.
    fn grind_vertical_normal_tracks_rail_slope_and_falls_back_for_vertical_rail() {
        let inverse_sqrt_two = 0.5_f32.sqrt();
        assert_eq!(grind_vertical_normal([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
        let sloped = grind_vertical_normal([inverse_sqrt_two, inverse_sqrt_two, 0.0]);
        assert!((sloped[0] + inverse_sqrt_two).abs() < 0.000_001);
        assert!((sloped[1] - inverse_sqrt_two).abs() < 0.000_001);
        assert_eq!(sloped[2], 0.0);
        assert_eq!(grind_vertical_normal([0.0, 1.0, 0.0]), [0.0, 1.0, 0.0]);
    }

    #[test]
    #[allow(clippy::float_cmp)] // The rejected rail-parallel component is exactly zero.
    fn grind_landing_normal_leans_against_approach_and_clamps_to_authored_angle() {
        let normal = grind_landing_normal(
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [10.0, 0.0, 0.0],
            0.1,
            20.0,
        );
        let maximum_angle = 20.0_f32.to_radians();
        assert!((normal[0] + maximum_angle.sin()).abs() < 0.000_001);
        assert!((normal[1] - maximum_angle.cos()).abs() < 0.000_001);
        assert_eq!(normal[2], 0.0);

        let within_limit =
            grind_landing_normal([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 0.1, 20.0);
        let inverse_length = 1.01_f32.sqrt().recip();
        assert!((within_limit[0] + 0.1 * inverse_length).abs() < 0.000_001);
        assert!((within_limit[1] - inverse_length).abs() < 0.000_001);
    }
}
