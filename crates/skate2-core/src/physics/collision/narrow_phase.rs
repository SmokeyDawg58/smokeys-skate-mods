//! Typed reconstruction of `RenderWare` geometric-primitive collision data.
//!
//! The reconstruction is exposed in evidence-complete slices: primitive
//! construction and generic separation, sphere/capsule/triangle support
//! features, all point-involved prisms, and complete edge/edge and face/edge
//! clipping plus the generic face/face tail and box/cylinder maximum features.
//! The specialized box/triangle, box/box, and cylinder separation paths are
//! reconstructed separately from the generic candidate builder.

use skate2_formats::collision::{Volume, VolumeType};

const EDGE_CROSS_MINIMUM_LENGTH_SQUARED: f32 = 0.001;
const DIRECTION_MINIMUM_LENGTH_SQUARED: f32 = f32::EPSILON;
const FEATURE_SIMPLIFICATION_THRESHOLD: f32 = 0.05;
const TRIANGLE_FACE_MINIMUM_ALIGNMENT: f32 = 0.95;
const TRIANGLE_FIXUP_ALIGNMENT_CUTOFF: f32 = f32::from_bits(0x3f7f_f62b);
const TRIANGLE_MINIMUM_CLIPPING_ANGLE: f32 = f32::from_bits(0x3a83_126f);
const FLAG_TRIANGLE_ONE_SIDED: u32 = 0x010;
const FLAG_TRIANGLE_EDGE_01_CONVEX: u32 = 0x020;
const FLAG_TRIANGLE_EDGE_12_CONVEX: u32 = 0x040;
const FLAG_TRIANGLE_EDGE_20_CONVEX: u32 = 0x080;
const FLAG_TRIANGLE_USE_EDGE_COS: u32 = 0x100;
const FLAG_TRIANGLE_VERTEX_0_DISABLE: u32 = 0x200;
const FLAG_TRIANGLE_VERTEX_1_DISABLE: u32 = 0x400;
const FLAG_TRIANGLE_VERTEX_2_DISABLE: u32 = 0x800;
const BOX_FACE_ALIGNMENT_THRESHOLD: f32 = 0.2;
const CYLINDER_CAP_ALIGNMENT_THRESHOLD: f32 = f32::from_bits(0x3f7f_f2e5);
const CYLINDER_EDGE_ALIGNMENT_THRESHOLD: f32 = f32::from_bits(0x3ca3_d70a);
const SQRT_HALF: f32 = f32::from_bits(0x3f35_04f3);
const BOX_OTHER_AXES: [[usize; 2]; 3] = [[1, 2], [0, 2], [0, 1]];
const BOX_FACE_CORNER_SIGNS: [[f32; 2]; 4] = [[1.0, 1.0], [1.0, -1.0], [-1.0, -1.0], [-1.0, 1.0]];
const BOX_VERTEX_SIGNS: [[f32; 3]; 8] = [
    [-1.0, 1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [1.0, 1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, -1.0, 1.0],
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpPrimitiveType {
    Sphere,
    Capsule,
    Triangle,
    Box,
    Cylinder,
}

/// Region returned by `ComputeTriangleFeatureTypeFromNormal` at `0x82acb278`.
///
/// The three edge bits compose the intervening vertex values: for example,
/// vertex 1 is the junction of edges 01 and 12 and therefore has value `3`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TriangleFeatureType {
    Face = 0,
    Edge01 = 1,
    Edge12 = 2,
    Vertex1 = 3,
    Edge20 = 4,
    Vertex0 = 5,
    Vertex2 = 6,
}

/// Human-readable counterpart of the 192-byte `rw::collision::GPInstance`.
///
/// `points[0]` is the position for analytic primitives. A triangle uses all
/// three points. Fixed-size direction arrays preserve the reference storage
/// and ordering, including populated cylinder slots beyond its active count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpPrimitive {
    pub primitive_type: GpPrimitiveType,
    pub points: [[f32; 3]; 3],
    pub face_normals: [[f32; 3]; 3],
    pub face_normal_count: u8,
    pub edge_directions: [[f32; 3]; 3],
    pub edge_direction_count: u8,
    pub dimensions: [f32; 3],
    pub fatness: f32,
    pub edge_data: [f32; 3],
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectionInterval {
    pub minimum: f32,
    pub maximum: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeparatingDirection {
    /// Oriented from the first primitive toward the second primitive.
    pub direction: [f32; 3],
    /// Unpadded core-shape separation. Fatness is applied by `Overlap` later.
    pub separation: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpFeatureEdge {
    pub start: [f32; 3],
    pub direction: [f32; 3],
    pub plane_normal: [f32; 3],
    pub length: f32,
}

impl Default for GpFeatureEdge {
    fn default() -> Self {
        Self {
            start: [0.0; 3],
            direction: [0.0; 3],
            plane_normal: [0.0; 3],
            length: 0.0,
        }
    }
}

impl GpFeatureEdge {
    #[must_use]
    pub fn from_endpoints(start: [f32; 3], end: [f32; 3]) -> Self {
        let delta = subtract(end, start);
        let squared = length_squared(delta);
        if squared > DIRECTION_MINIMUM_LENGTH_SQUARED {
            let length = squared.sqrt();
            Self {
                start,
                direction: delta.map(|component| component / length),
                plane_normal: [0.0; 3],
                length,
            }
        } else {
            Self {
                start,
                ..Self::default()
            }
        }
    }

    /// Clamps a point to the edge and returns the reference region code:
    /// start `1`, interior (including exact endpoints) `2`, or end `3`.
    pub fn constrain_point(&self, point: &mut [f32; 3]) -> u32 {
        let parameter = dot(subtract(*point, self.start), self.direction);
        if parameter < 0.0 {
            *point = self.start;
            1
        } else if parameter > self.length {
            *point = add(self.start, scale(self.direction, self.length));
            3
        } else {
            *point = add(self.start, scale(self.direction, parameter));
            2
        }
    }
}

/// Typed counterpart of the 576-byte `rw::collision::Feature` record.
#[derive(Clone, Debug, PartialEq)]
pub struct GpFeature {
    pub region: u32,
    pub edges: [GpFeatureEdge; 8],
    pub own_normal: [f32; 3],
    pub point: [f32; 3],
    pub edge_count: i32,
}

/// Human-readable counterpart of the 544-byte
/// `rwc_FeatureIntersectionPrism` contact record.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureIntersectionPrism {
    pub points_on_first: [[f32; 3]; 16],
    pub points_on_second: [[f32; 3]; 16],
    pub normal: [f32; 3],
    pub point_count: i32,
    pub normal_override: u32,
}

/// Proven tag words carried by one native `GPInstance`.
///
/// Geometry and occurrence identity remain separate because the human-readable
/// [`GpPrimitive`] deliberately has no pointer-shaped native header fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
pub(super) struct GpInstanceTags {
    volume: u32,
    user: u32,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
impl GpInstanceTags {
    pub(super) const fn new(volume: u32, user: u32) -> Self {
        Self { volume, user }
    }
}

/// One fatness-adjusted pair in `GPInstance::ContactPoints`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
pub(super) struct Sk8ContactPointPair {
    first: [f32; 3],
    second: [f32; 3],
}

/// Accepted semantic output of `Sk8ComputeContactPoints`.
///
/// PDB storage is 560 bytes with sixteen 32-byte point pairs. This value keeps
/// represented XYZ and exact integer words; vector W lanes remain outside the
/// current numeric claim.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
pub(super) struct Sk8ContactPoints {
    first: GpInstanceTags,
    second: GpInstanceTags,
    point_count: usize,
    normal: [f32; 3],
    point_pairs: [Sk8ContactPointPair; 16],
}

/// Internal return-plus-scratch state of the native function.
///
/// Triangle fix-up rejection still packs the scratch value and only changes
/// the returned count to zero. The public-to-this-crate accepted seam below
/// deliberately does not expose that rejected scratch to Contact production.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
struct Sk8ContactPointsComputation {
    returned_count: usize,
    scratch: Sk8ContactPoints,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
impl Sk8ContactPoints {
    pub(super) const fn first_tags(&self) -> GpInstanceTags {
        self.first
    }

    pub(super) const fn second_tags(&self) -> GpInstanceTags {
        self.second
    }

    pub(super) const fn normal(&self) -> [f32; 3] {
        self.normal
    }

    pub(super) fn point_pairs(&self) -> &[Sk8ContactPointPair] {
        &self.point_pairs[..self.point_count]
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
impl Sk8ContactPointPair {
    pub(super) const fn first(&self) -> [f32; 3] {
        self.first
    }

    pub(super) const fn second(&self) -> [f32; 3] {
        self.second
    }
}

impl Default for FeatureIntersectionPrism {
    fn default() -> Self {
        Self {
            points_on_first: [[0.0; 3]; 16],
            points_on_second: [[0.0; 3]; 16],
            normal: [0.0; 3],
            point_count: 0,
            normal_override: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PreTriangleFixupCandidate {
    separating: SeparatingDirection,
    prism: FeatureIntersectionPrism,
    result_normal: [f32; 3],
}

// The 544-byte native prism is stack-local. Boxing this internal discriminator
// would add an allocation to every accepted primitive/triangle pair solely to
// equalize the deliberately data-free separated variant.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
enum PreTriangleFixupOutcome {
    Separated,
    Candidate(PreTriangleFixupCandidate),
}

/// Outputs written by `ClipEdgeWithFacePrism` at `0x82ac9b00`.
///
/// A failed clip still returns the reference's clamped interval and, for the
/// parallel-constraint case, the last face edge that rejected the input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FacePrismEdgeClip {
    pub intersects: bool,
    pub parameter_start: f32,
    pub parameter_end: f32,
    pub constraining_base: [f32; 3],
    pub constraining_direction: [f32; 3],
}

impl Default for GpFeature {
    fn default() -> Self {
        Self {
            region: 0,
            edges: [GpFeatureEdge::default(); 8],
            own_normal: [0.0; 3],
            point: [0.0; 3],
            edge_count: 0,
        }
    }
}

impl GpFeature {
    /// Reconstructs `Feature::BuildEdgePlanes` at `0x82ac8810`.
    ///
    /// The reference reverses the extrusion direction for clockwise features
    /// and only normalizes crosses whose squared length is strictly greater
    /// than `VEC_EPSILON`.
    pub fn build_edge_planes(&mut self, counter_clockwise: bool, extrusion_direction: [f32; 3]) {
        let extrusion_direction = if counter_clockwise {
            extrusion_direction
        } else {
            negate(extrusion_direction)
        };
        let edge_count = usize::try_from(self.edge_count)
            .unwrap_or(0)
            .min(self.edges.len());
        for edge in &mut self.edges[..edge_count] {
            edge.plane_normal = normalize_above_or_zero(
                cross(edge.direction, extrusion_direction),
                DIRECTION_MINIMUM_LENGTH_SQUARED,
            );
        }
    }
}

/// Exact point/point branch of `FindFeatureIntersectionPrism` at
/// `0x82ad2350`.
#[must_use]
pub fn point_point_feature_prism(
    first: &GpFeature,
    second: &GpFeature,
) -> FeatureIntersectionPrism {
    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = first.point;
    result.points_on_second[0] = second.point;
    result
}

/// Exact `FindEdgePointPrism` path at `0x82ac90d8`. Returned points are
/// ordered as edge then point, matching the named helper.
pub fn edge_point_feature_prism(
    edge_feature: &mut GpFeature,
    point_feature: &GpFeature,
) -> Option<FeatureIntersectionPrism> {
    if edge_feature.edge_count != 1 || point_feature.edge_count != 0 {
        return None;
    }
    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = point_feature.point;
    result.points_on_second[0] = point_feature.point;
    let edge_region = edge_feature.edges[0].constrain_point(&mut result.points_on_first[0]);
    edge_feature.region = edge_feature
        .region
        .wrapping_sub(edge_region)
        .wrapping_add(2);
    Some(result)
}

/// Exact `FindFacePointPrism` path at `0x82ac9848`. Returned points are
/// ordered as face then point, matching the named helper.
pub fn face_point_feature_prism(
    face_feature: &mut GpFeature,
    point_feature: &GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    let edge_count = usize::try_from(face_feature.edge_count)
        .ok()?
        .min(face_feature.edges.len());
    if edge_count <= 1 || point_feature.edge_count != 0 {
        return None;
    }

    let plane_origin = face_feature.edges[0].start;
    let plane_distance = dot(
        separation_direction,
        subtract(plane_origin, point_feature.point),
    );
    let mut projected_point = add(
        point_feature.point,
        scale(separation_direction, plane_distance),
    );

    let mut furthest_edge = 0;
    let mut furthest_distance = dot(
        face_feature.edges[0].plane_normal,
        subtract(projected_point, face_feature.edges[0].start),
    );
    for (index, edge) in face_feature.edges[..edge_count].iter().enumerate().skip(1) {
        let distance = dot(edge.plane_normal, subtract(projected_point, edge.start));
        if distance > furthest_distance {
            furthest_distance = distance;
            furthest_edge = index;
        }
    }
    if furthest_distance > 0.0 {
        let edge = &face_feature.edges[furthest_edge];
        projected_point = subtract(projected_point, scale(edge.plane_normal, furthest_distance));
        let edge_region = edge.constrain_point(&mut projected_point);
        let edge_offset = u32::try_from(furthest_edge).ok()?.wrapping_mul(2);
        face_feature.region = face_feature
            .region
            .wrapping_add(edge_region)
            .wrapping_add(edge_offset);
    }

    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = projected_point;
    result.points_on_second[0] = point_feature.point;
    Some(result)
}

/// Reconstructs every point-involved branch of
/// `FindFeatureIntersectionPrism` at `0x82ad2350`, preserving caller argument
/// order in the two output arrays. Returns `None` when neither feature is a
/// point or when a typed feature count is invalid for these branches.
pub fn point_involved_feature_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    match (first.edge_count, second.edge_count) {
        (0, 0) => Some(point_point_feature_prism(first, second)),
        (1, 0) => edge_point_feature_prism(first, second),
        (first_edges, 0) if first_edges > 1 => {
            face_point_feature_prism(first, second, separation_direction)
        }
        (0, 1) => {
            let mut result = edge_point_feature_prism(second, first)?;
            std::mem::swap(&mut result.points_on_first, &mut result.points_on_second);
            Some(result)
        }
        (0, second_edges) if second_edges > 1 => {
            let mut result = face_point_feature_prism(second, first, separation_direction)?;
            std::mem::swap(&mut result.points_on_first, &mut result.points_on_second);
            Some(result)
        }
        _ => None,
    }
}

/// Reconstructs the non-parallel branch of `FindEdgeEdgePrism` at
/// `0x82ac9140`. `None` preserves the reference branch boundary: squared
/// cross-direction length at or below f32 epsilon requires the separate
/// parallel-overlap path.
pub fn nonparallel_edge_edge_feature_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
) -> Option<FeatureIntersectionPrism> {
    if first.edge_count != 1 || second.edge_count != 1 {
        return None;
    }
    let first_edge = first.edges[0];
    let second_edge = second.edges[0];
    let cross_direction = cross(first_edge.direction, second_edge.direction);
    let cross_length_squared = length_squared(cross_direction);
    if cross_length_squared <= DIRECTION_MINIMUM_LENGTH_SQUARED {
        return None;
    }

    let start_delta = subtract(second_edge.start, first_edge.start);
    let mut first_parameter =
        dot(cross(start_delta, second_edge.direction), cross_direction) / cross_length_squared;
    if first_parameter < 0.0 {
        first_parameter = 0.0;
    }
    if first_parameter > first_edge.length {
        first_parameter = first_edge.length;
    }

    let candidate = add(
        first_edge.start,
        scale(first_edge.direction, first_parameter),
    );
    let mut point_on_second = candidate;
    let second_region = second_edge.constrain_point(&mut point_on_second);
    second.region = second.region.wrapping_sub(second_region).wrapping_add(2);
    let mut point_on_first = point_on_second;
    let first_region = first_edge.constrain_point(&mut point_on_first);
    first.region = first.region.wrapping_sub(first_region).wrapping_add(2);

    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = point_on_first;
    result.points_on_second[0] = point_on_second;
    Some(result)
}

/// Reconstructs the parallel interval branch of `FindEdgeEdgePrism` at
/// `0x82ac9140`. The caller must supply edge directions whose squared cross is
/// at or below f32 epsilon. Positive scalar overlap emits two paired line
/// points; touching or disjoint intervals emit the nearest endpoint pair and
/// mutate each feature region toward the selected endpoint.
pub fn parallel_edge_edge_feature_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    if first.edge_count != 1 || second.edge_count != 1 {
        return None;
    }
    let first_edge = first.edges[0];
    let second_edge = second.edges[0];
    if length_squared(cross(first_edge.direction, second_edge.direction))
        > DIRECTION_MINIMUM_LENGTH_SQUARED
    {
        return None;
    }

    let first_alignment = dot(separation_direction, first_edge.direction).abs();
    let second_alignment = dot(separation_direction, second_edge.direction).abs();
    let interval_direction = if first_alignment < second_alignment {
        first_edge.direction
    } else {
        second_edge.direction
    };

    let first_end = add(
        first_edge.start,
        scale(first_edge.direction, first_edge.length),
    );
    let second_end = add(
        second_edge.start,
        scale(second_edge.direction, second_edge.length),
    );
    let first_start_projection = dot(interval_direction, first_edge.start);
    let first_end_projection = dot(interval_direction, first_end);
    let second_start_projection = dot(interval_direction, second_edge.start);
    let second_end_projection = dot(interval_direction, second_end);

    let (first_minimum, first_maximum) = if first_end_projection > first_start_projection {
        (first_start_projection, first_end_projection)
    } else {
        (first_end_projection, first_start_projection)
    };
    let (second_minimum, second_maximum) = if second_end_projection > second_start_projection {
        (second_start_projection, second_end_projection)
    } else {
        (second_end_projection, second_start_projection)
    };
    let overlap_start = if first_minimum > second_minimum {
        first_minimum
    } else {
        second_minimum
    };
    let overlap_end = if second_maximum > first_maximum {
        first_maximum
    } else {
        second_maximum
    };

    if overlap_end > overlap_start {
        let mut result = FeatureIntersectionPrism {
            point_count: 2,
            ..FeatureIntersectionPrism::default()
        };
        for (index, interval_parameter) in [overlap_start, overlap_end].into_iter().enumerate() {
            result.points_on_first[index] = parallel_edge_point_at_projection(
                first_edge,
                separation_direction,
                interval_direction,
                interval_parameter,
            );
            result.points_on_second[index] = parallel_edge_point_at_projection(
                second_edge,
                separation_direction,
                interval_direction,
                interval_parameter,
            );
        }
        return Some(result);
    }

    let first_interval_is_lower = second_minimum > first_minimum;
    let (first_point, first_region_delta) = parallel_edge_nearest_endpoint(
        first_edge,
        first_start_projection,
        first_end_projection,
        first_interval_is_lower,
    );
    let (second_point, second_region_delta) = parallel_edge_nearest_endpoint(
        second_edge,
        second_start_projection,
        second_end_projection,
        !first_interval_is_lower,
    );
    first.region = first.region.wrapping_add_signed(first_region_delta);
    second.region = second.region.wrapping_add_signed(second_region_delta);

    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = first_point;
    result.points_on_second[0] = second_point;
    Some(result)
}

fn parallel_edge_nearest_endpoint(
    edge: GpFeatureEdge,
    start_projection: f32,
    end_projection: f32,
    select_maximum: bool,
) -> ([f32; 3], i32) {
    let select_end = if select_maximum {
        start_projection <= end_projection
    } else {
        start_projection >= end_projection
    };
    if select_end {
        (add(edge.start, scale(edge.direction, edge.length)), 1_i32)
    } else {
        (edge.start, -1_i32)
    }
}

fn parallel_edge_point_at_projection(
    edge: GpFeatureEdge,
    separation_direction: [f32; 3],
    interval_direction: [f32; 3],
    interval_parameter: f32,
) -> [f32; 3] {
    let target = scale(interval_direction, interval_parameter);
    let line_plane_normal = cross(cross(edge.direction, separation_direction), edge.direction);
    let line_plane_normal = scale(
        line_plane_normal,
        length_squared(line_plane_normal).sqrt().recip(),
    );
    let separation_parameter = dot(subtract(edge.start, target), line_plane_normal)
        / dot(separation_direction, line_plane_normal);
    let candidate = add(target, scale(separation_direction, separation_parameter));
    add(
        edge.start,
        scale(
            edge.direction,
            dot(edge.direction, subtract(candidate, edge.start)),
        ),
    )
}

/// Complete `FindEdgeEdgePrism` dispatcher at `0x82ac9140`.
pub fn edge_edge_feature_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    if first.edge_count != 1 || second.edge_count != 1 {
        return None;
    }
    if length_squared(cross(first.edges[0].direction, second.edges[0].direction))
        > DIRECTION_MINIMUM_LENGTH_SQUARED
    {
        nonparallel_edge_edge_feature_prism(first, second)
    } else {
        parallel_edge_edge_feature_prism(first, second, separation_direction)
    }
}

/// Reconstructs `ClipEdgeWithFacePrism` at `0x82ac9b00`.
///
/// The helper clips the edge's distance parameter against the face prism. It
/// skips constraint crosses whose squared length is strictly below `.001`,
/// treats denominators with magnitude strictly below `.05` as parallel, and
/// preserves the executable's strict interval comparisons and failure-bound
/// clamping. `None` denotes an invalid typed feature shape rather than a
/// geometric rejection.
#[must_use]
pub fn clip_edge_with_face_prism(
    face: &GpFeature,
    edge: &GpFeature,
    prism_direction: [f32; 3],
) -> Option<FacePrismEdgeClip> {
    let face_edge_count = usize::try_from(face.edge_count).ok()?;
    if !(2..=face.edges.len()).contains(&face_edge_count) || edge.edge_count != 1 {
        return None;
    }

    let clipped_edge = edge.edges[0];
    let mut result = FacePrismEdgeClip {
        intersects: true,
        parameter_start: 0.0,
        parameter_end: clipped_edge.length,
        constraining_base: [0.0; 3],
        constraining_direction: [0.0; 3],
    };
    let mut rejected_by_parallel_constraint = false;

    for face_edge in &face.edges[..face_edge_count] {
        let constraint_cross = cross(prism_direction, face_edge.direction);
        let cross_length_squared = length_squared(constraint_cross);
        if cross_length_squared < EDGE_CROSS_MINIMUM_LENGTH_SQUARED {
            continue;
        }
        let constraint_direction = scale(constraint_cross, cross_length_squared.sqrt().recip());
        let denominator = dot(clipped_edge.direction, constraint_direction);
        let numerator = dot(
            subtract(face_edge.start, clipped_edge.start),
            constraint_direction,
        );

        if denominator.abs() < FEATURE_SIMPLIFICATION_THRESHOLD {
            if numerator > 0.0 {
                result.constraining_base = face_edge.start;
                result.constraining_direction = constraint_direction;
                rejected_by_parallel_constraint = true;
            }
            continue;
        }

        let parameter = numerator / denominator;
        if denominator > 0.0 {
            if parameter > result.parameter_start {
                result.parameter_start = parameter;
            }
            if result.parameter_start > result.parameter_end {
                result.parameter_start = result.parameter_end;
                result.intersects = false;
                return Some(result);
            }
        } else {
            if parameter < result.parameter_end {
                result.parameter_end = parameter;
            }
            if result.parameter_end < result.parameter_start {
                result.parameter_end = result.parameter_start;
                result.intersects = false;
                return Some(result);
            }
        }
    }

    result.intersects = !rejected_by_parallel_constraint;
    Some(result)
}

/// Reconstructs `FaceConstrainPoint` at `0x82ac9980`.
///
/// The furthest strictly positive face-prism plane is selected, the point is
/// projected to that plane, and then clamped to its finite edge. The returned
/// value is the executable's edge/vertex region code including the face-edge
/// index offset. `None` is reserved for an impossible typed edge count.
pub fn constrain_point_to_face_prism(
    face: &GpFeature,
    prism_direction: [f32; 3],
    point: &mut [f32; 3],
) -> Option<u32> {
    if face.edge_count <= 0 {
        return Some(0);
    }
    let face_edge_count = usize::try_from(face.edge_count).ok()?;
    if face_edge_count > face.edges.len() {
        return None;
    }

    let mut furthest_index = 0;
    let first_edge = face.edges[0];
    let first_cross = cross(first_edge.direction, prism_direction);
    let mut furthest_direction = if length_squared(first_cross) > DIRECTION_MINIMUM_LENGTH_SQUARED {
        scale(first_cross, length_squared(first_cross).sqrt().recip())
    } else {
        first_cross
    };
    let mut furthest_distance = dot(furthest_direction, subtract(*point, first_edge.start));

    for (index, face_edge) in face.edges[..face_edge_count].iter().enumerate().skip(1) {
        let constraint_cross = cross(face_edge.direction, prism_direction);
        let cross_length_squared = length_squared(constraint_cross);
        let constraint_direction = if cross_length_squared > DIRECTION_MINIMUM_LENGTH_SQUARED {
            scale(constraint_cross, cross_length_squared.sqrt().recip())
        } else {
            constraint_cross
        };
        let distance = dot(constraint_direction, subtract(*point, face_edge.start));
        if distance > furthest_distance {
            furthest_index = index;
            furthest_direction = constraint_direction;
            furthest_distance = distance;
        }
    }

    if furthest_distance > 0.0 {
        *point = subtract(*point, scale(furthest_direction, furthest_distance));
        let edge_region = face.edges[furthest_index].constrain_point(point);
        Some(edge_region.wrapping_add(u32::try_from(furthest_index).ok()?.wrapping_mul(2)))
    } else {
        Some(0)
    }
}

/// Evidence-complete successful-clip branch of `FindFaceEdgePrism` at
/// `0x82ad0cc0`. Returned points are ordered face then edge, matching the
/// named helper. A rejected interval belongs to the still-isolated fallback
/// branch and returns `None`.
#[must_use]
pub fn clipped_face_edge_feature_prism(
    face: &GpFeature,
    edge: &GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    let clip = clip_edge_with_face_prism(face, edge, separation_direction)?;
    if !clip.intersects {
        return None;
    }

    Some(clipped_face_edge_feature_prism_from_clip(
        face,
        edge,
        separation_direction,
        clip,
    ))
}

fn clipped_face_edge_feature_prism_from_clip(
    face: &GpFeature,
    edge: &GpFeature,
    separation_direction: [f32; 3],
    clip: FacePrismEdgeClip,
) -> FeatureIntersectionPrism {
    let clipped_edge = edge.edges[0];
    let plane_origin = face.edges[0].start;
    let projection_denominator = dot(separation_direction, face.own_normal);
    let mut result = FeatureIntersectionPrism {
        point_count: 2,
        ..FeatureIntersectionPrism::default()
    };
    for (index, parameter) in [clip.parameter_start, clip.parameter_end]
        .into_iter()
        .enumerate()
    {
        let point_on_edge = add(clipped_edge.start, scale(clipped_edge.direction, parameter));
        let projection_parameter =
            dot(subtract(plane_origin, point_on_edge), face.own_normal) / projection_denominator;
        result.points_on_second[index] = point_on_edge;
        result.points_on_first[index] = add(
            point_on_edge,
            scale(separation_direction, projection_parameter),
        );
    }
    result
}

/// Complete reconstruction of `FindFaceEdgePrism` at `0x82ad0cc0`.
/// Returned points are ordered face then edge. Besides the ordinary two-point
/// interval, this preserves the collapsed one-point fallback, the parallel
/// constraining-plane fallback, both reference region mutations, and the
/// conditional replacement contact normal.
pub fn face_edge_feature_prism(
    face: &mut GpFeature,
    edge: &mut GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    let clip = clip_edge_with_face_prism(face, edge, separation_direction)?;
    if clip.intersects {
        return Some(clipped_face_edge_feature_prism_from_clip(
            face,
            edge,
            separation_direction,
            clip,
        ));
    }

    let clipped_edge = edge.edges[0];
    if clip.parameter_end - clip.parameter_start <= EDGE_CROSS_MINIMUM_LENGTH_SQUARED {
        let point_on_edge = add(
            clipped_edge.start,
            scale(clipped_edge.direction, clip.parameter_start),
        );
        let projection_parameter = dot(
            separation_direction,
            subtract(face.edges[0].start, point_on_edge),
        );
        let mut point_on_face = add(
            point_on_edge,
            scale(separation_direction, projection_parameter),
        );
        let face_region =
            constrain_point_to_face_prism(face, separation_direction, &mut point_on_face)?;
        face.region = face.region.wrapping_add(face_region);

        let mut constrained_edge_point = point_on_face;
        let edge_region = clipped_edge.constrain_point(&mut constrained_edge_point);
        edge.region = edge.region.wrapping_sub(edge_region).wrapping_add(2);

        let mut result = FeatureIntersectionPrism {
            point_count: 1,
            ..FeatureIntersectionPrism::default()
        };
        result.points_on_first[0] = point_on_face;
        result.points_on_second[0] = constrained_edge_point;
        return Some(result);
    }

    let interval_length = clip.parameter_end - clip.parameter_start;
    let face_projection_parameter = dot(
        separation_direction,
        subtract(face.edges[0].start, clipped_edge.start),
    );
    let projected_to_face = add(
        clipped_edge.start,
        scale(separation_direction, face_projection_parameter),
    );
    let constraint_projection_parameter = dot(
        clip.constraining_direction,
        subtract(clip.constraining_base, projected_to_face),
    );
    let projected_to_constraint = add(
        projected_to_face,
        scale(clip.constraining_direction, constraint_projection_parameter),
    );
    let mut first_face_point = add(
        projected_to_constraint,
        scale(clipped_edge.direction, clip.parameter_start),
    );
    let mut second_face_point = add(
        first_face_point,
        scale(clipped_edge.direction, interval_length),
    );
    let first_face_region =
        constrain_point_to_face_prism(face, separation_direction, &mut first_face_point)?;
    let second_face_region =
        constrain_point_to_face_prism(face, separation_direction, &mut second_face_point)?;
    face.region = face
        .region
        .wrapping_add(first_face_region.max(second_face_region) & !1);

    let mut first_edge_point = first_face_point;
    let mut second_edge_point = second_face_point;
    clipped_edge.constrain_point(&mut first_edge_point);
    clipped_edge.constrain_point(&mut second_edge_point);

    let mut result = FeatureIntersectionPrism {
        point_count: 2,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = first_face_point;
    result.points_on_first[1] = second_face_point;
    result.points_on_second[0] = first_edge_point;
    result.points_on_second[1] = second_edge_point;

    let point_delta = subtract(first_face_point, first_edge_point);
    if length_squared(point_delta).sqrt() > f32::MIN_POSITIVE {
        let replacement_normal = cross(
            clipped_edge.direction,
            cross(point_delta, clipped_edge.direction),
        );
        result.normal = scale(
            replacement_normal,
            length_squared(replacement_normal).sqrt().recip(),
        );
        result.normal_override = 1;
    }
    Some(result)
}

/// Reconstructs the positive polygon-intersection portion of the generic
/// face/face tail in `FindFeatureIntersectionPrism` at `0x82ad2510`.
///
/// Contact order is face-two vertices inside face one, paired boundary
/// intersections in face-two-edge/face-one-edge order, then face-one vertices
/// inside face two. `None` preserves the separate nearest-edge fallback when
/// the projected polygons produce no contacts.
#[must_use]
pub fn intersecting_face_face_feature_prism(
    first: &GpFeature,
    second: &GpFeature,
) -> Option<FeatureIntersectionPrism> {
    let first_edge_count = usize::try_from(first.edge_count).ok()?;
    let second_edge_count = usize::try_from(second.edge_count).ok()?;
    if !(2..=first.edges.len()).contains(&first_edge_count)
        || !(2..=second.edges.len()).contains(&second_edge_count)
    {
        return None;
    }

    let mut second_outside = [false; 8];
    let mut second_edge_crossings = [0_u32; 8];
    for (first_plane_index, first_edge) in first.edges[..first_edge_count].iter().enumerate() {
        let mut previous_second_index = second_edge_count - 1;
        let mut previous_distance =
            face_edge_plane_distance(first_edge, second.edges[previous_second_index].start);
        for (second_index, is_outside) in second_outside[..second_edge_count].iter_mut().enumerate()
        {
            let distance = face_edge_plane_distance(first_edge, second.edges[second_index].start);
            if distance > 0.0 {
                *is_outside = true;
            }
            if distance * previous_distance <= 0.0 {
                second_edge_crossings[previous_second_index] |= 1_u32 << first_plane_index;
            }
            previous_second_index = second_index;
            previous_distance = distance;
        }
    }

    let mut result = FeatureIntersectionPrism::default();
    for (second_index, is_outside) in second_outside[..second_edge_count].iter().enumerate() {
        if !is_outside {
            push_face_pair_from_second(&mut result, first, second.edges[second_index].start)?;
        }
    }

    if usize::try_from(result.point_count).ok()? != second_edge_count {
        let mut first_outside = [false; 8];
        for (second_edge_index, second_edge) in second.edges[..second_edge_count].iter().enumerate()
        {
            let mut previous_first_index = first_edge_count - 1;
            let mut previous_distance =
                face_edge_plane_distance(second_edge, first.edges[previous_first_index].start);
            for (first_index, is_outside) in
                first_outside[..first_edge_count].iter_mut().enumerate()
            {
                let distance =
                    face_edge_plane_distance(second_edge, first.edges[first_index].start);
                if distance > 0.0 {
                    *is_outside = true;
                }
                let crosses_both_boundaries = distance * previous_distance <= 0.0
                    && second_edge_crossings[second_edge_index] & (1_u32 << previous_first_index)
                        != 0;
                let distance_delta = previous_distance - distance;
                if crosses_both_boundaries && distance_delta.abs() > f32::MIN_POSITIVE {
                    let crossing_edge = first.edges[previous_first_index];
                    let edge_parameter =
                        (crossing_edge.length * previous_distance) * distance_delta.recip();
                    let point_on_first = add(
                        crossing_edge.start,
                        scale(crossing_edge.direction, edge_parameter),
                    );
                    push_face_pair_from_first(&mut result, second, point_on_first)?;
                }
                previous_first_index = first_index;
                previous_distance = distance;
            }
        }

        for (first_index, is_outside) in first_outside[..first_edge_count].iter().enumerate() {
            if !is_outside {
                push_face_pair_from_first(&mut result, second, first.edges[first_index].start)?;
            }
        }
    }

    (result.point_count != 0).then_some(result)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FaceFaceNonoverlapCandidate {
    SecondVertexFirstEdge {
        second_vertex_index: usize,
        first_edge_index: usize,
    },
    FirstVertexSecondEdge {
        first_vertex_index: usize,
        second_edge_index: usize,
    },
}

/// Reconstructs the generic face/face tail of
/// `FindFeatureIntersectionPrism` at `0x82ad2510`.
///
/// Polygon contacts use [`intersecting_face_face_feature_prism`]. If none are
/// produced, the reference maximizes each face plane's minimum distance to an
/// opposing vertex, then passes that vertex/edge pair to
/// [`face_face_nonoverlap_feature_prism`]. Strict comparisons preserve both
/// the first-plane global tie and the last-vertex initialization used within
/// each plane.
#[must_use]
pub fn face_face_feature_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
) -> Option<FeatureIntersectionPrism> {
    if let Some(result) = intersecting_face_face_feature_prism(first, second) {
        return Some(result);
    }

    let first_edge_count = usize::try_from(first.edge_count).ok()?;
    let second_edge_count = usize::try_from(second.edge_count).ok()?;
    if !(2..=first.edges.len()).contains(&first_edge_count)
        || !(2..=second.edges.len()).contains(&second_edge_count)
    {
        return None;
    }

    let mut best_minimum_distance = -f32::MAX;
    let mut candidate = None;
    for (first_edge_index, first_edge) in first.edges[..first_edge_count].iter().enumerate() {
        let (second_vertex_index, minimum_distance) =
            minimum_face_plane_vertex(first_edge, &second.edges[..second_edge_count]);
        if minimum_distance > best_minimum_distance {
            best_minimum_distance = minimum_distance;
            candidate = Some(FaceFaceNonoverlapCandidate::SecondVertexFirstEdge {
                second_vertex_index,
                first_edge_index,
            });
        }
    }
    for (second_edge_index, second_edge) in second.edges[..second_edge_count].iter().enumerate() {
        let (first_vertex_index, minimum_distance) =
            minimum_face_plane_vertex(second_edge, &first.edges[..first_edge_count]);
        if minimum_distance > best_minimum_distance {
            best_minimum_distance = minimum_distance;
            candidate = Some(FaceFaceNonoverlapCandidate::FirstVertexSecondEdge {
                first_vertex_index,
                second_edge_index,
            });
        }
    }

    let mut result = match candidate? {
        FaceFaceNonoverlapCandidate::SecondVertexFirstEdge {
            second_vertex_index,
            first_edge_index,
        } => {
            let reversed = face_face_nonoverlap_feature_prism(
                second,
                first,
                second_vertex_index,
                first_edge_index,
            )?;
            FeatureIntersectionPrism {
                points_on_first: reversed.points_on_second,
                points_on_second: reversed.points_on_first,
                normal: reversed.normal,
                point_count: reversed.point_count,
                normal_override: reversed.normal_override,
            }
        }
        FaceFaceNonoverlapCandidate::FirstVertexSecondEdge {
            first_vertex_index,
            second_edge_index,
        } => face_face_nonoverlap_feature_prism(
            first,
            second,
            first_vertex_index,
            second_edge_index,
        )?,
    };

    let point_delta = subtract(result.points_on_second[0], result.points_on_first[0]);
    let point_distance = length_squared(point_delta).sqrt();
    if point_distance > f32::MIN_POSITIVE {
        result.normal = scale(point_delta, point_distance.recip());
        result.normal_override = 1;
    }
    Some(result)
}

fn minimum_face_plane_vertex(plane: &GpFeatureEdge, vertices: &[GpFeatureEdge]) -> (usize, f32) {
    let mut minimum_vertex_index = vertices.len() - 1;
    let mut minimum_distance =
        face_edge_plane_distance(plane, vertices[minimum_vertex_index].start);
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        let distance = face_edge_plane_distance(plane, vertex.start);
        if distance < minimum_distance {
            minimum_vertex_index = vertex_index;
            minimum_distance = distance;
        }
    }
    (minimum_vertex_index, minimum_distance)
}

/// Reconstructs `FindFaceFaceNonOverlapPrism` at `0x82ac9cf8` for a selected
/// vertex on the first face and edge on the second.
///
/// The previous first-face edge is preferred when its plane-normal alignment
/// with the second edge is strictly below `0.05`; the current edge is tested
/// second. That path treats the two directed edges as opposing intervals and
/// emits either their two-point overlap or the nearest endpoint pair. When
/// neither adjacent edge qualifies, the selected vertex is projected and
/// clamped to the second edge.
#[must_use]
pub fn face_face_nonoverlap_feature_prism(
    first_face: &mut GpFeature,
    second_face: &mut GpFeature,
    first_vertex_index: usize,
    second_edge_index: usize,
) -> Option<FeatureIntersectionPrism> {
    let first_edge_count = usize::try_from(first_face.edge_count).ok()?;
    let second_edge_count = usize::try_from(second_face.edge_count).ok()?;
    if !(2..=first_face.edges.len()).contains(&first_edge_count)
        || !(2..=second_face.edges.len()).contains(&second_edge_count)
        || first_vertex_index >= first_edge_count
        || second_edge_index >= second_edge_count
    {
        return None;
    }

    let previous_edge_index = if first_vertex_index == 0 {
        first_edge_count - 1
    } else {
        first_vertex_index - 1
    };
    let second_edge = second_face.edges[second_edge_index];
    let previous_alignment = dot(
        first_face.edges[previous_edge_index].plane_normal,
        second_edge.direction,
    )
    .abs();
    let current_alignment = dot(
        first_face.edges[first_vertex_index].plane_normal,
        second_edge.direction,
    )
    .abs();
    let adjacent_edge_index = if previous_alignment < FEATURE_SIMPLIFICATION_THRESHOLD {
        Some(previous_edge_index)
    } else if current_alignment < FEATURE_SIMPLIFICATION_THRESHOLD {
        Some(first_vertex_index)
    } else {
        None
    };

    if let Some(first_edge_index) = adjacent_edge_index {
        return adjacent_face_edges_nonoverlap_prism(
            first_face,
            second_face,
            first_edge_index,
            second_edge_index,
        );
    }

    face_vertex_edge_nonoverlap_prism(
        first_face,
        second_face,
        first_vertex_index,
        second_edge_index,
    )
}

fn face_vertex_edge_nonoverlap_prism(
    first_face: &mut GpFeature,
    second_face: &mut GpFeature,
    first_vertex_index: usize,
    second_edge_index: usize,
) -> Option<FeatureIntersectionPrism> {
    first_face.region = u32::try_from(2 * first_vertex_index + 1).ok()?;
    let second_edge_region = u32::try_from(2 * (second_edge_index + 1)).ok()?;
    second_face.region = second_edge_region;

    let second_edge = second_face.edges[second_edge_index];
    let point_on_first = first_face.edges[first_vertex_index].start;
    let projection_parameter = dot(
        second_edge.direction,
        subtract(point_on_first, second_edge.start),
    );
    let clamped_parameter = if projection_parameter < f32::MIN_POSITIVE {
        second_face.region = second_edge_region - 1;
        0.0
    } else if projection_parameter > second_edge.length {
        second_face.region = second_edge_region + 1;
        second_edge.length
    } else {
        projection_parameter
    };

    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = point_on_first;
    result.points_on_second[0] = add(
        second_edge.start,
        scale(second_edge.direction, clamped_parameter),
    );
    Some(result)
}

fn adjacent_face_edges_nonoverlap_prism(
    first_face: &mut GpFeature,
    second_face: &mut GpFeature,
    first_edge_index: usize,
    second_edge_index: usize,
) -> Option<FeatureIntersectionPrism> {
    let first_edge = first_face.edges[first_edge_index];
    let second_edge = second_face.edges[second_edge_index];
    first_face.region = u32::try_from(2 * (first_edge_index + 1)).ok()?;
    second_face.region = u32::try_from(2 * (second_edge_index + 1)).ok()?;

    let projection_parameter = dot(
        first_edge.direction,
        subtract(second_edge.start, first_edge.start),
    );
    if projection_parameter < f32::MIN_POSITIVE {
        first_face.region -= 1;
        second_face.region -= 1;
        return Some(single_feature_pair(first_edge.start, second_edge.start));
    }

    let interval_remainder = first_edge.length + second_edge.length - projection_parameter;
    if interval_remainder < f32::MIN_POSITIVE {
        first_face.region += 1;
        second_face.region += 1;
        return Some(single_feature_pair(
            add(
                first_edge.start,
                scale(first_edge.direction, first_edge.length),
            ),
            add(
                second_edge.start,
                scale(second_edge.direction, second_edge.length),
            ),
        ));
    }

    let mut result = FeatureIntersectionPrism {
        point_count: 2,
        ..FeatureIntersectionPrism::default()
    };
    let first_end_remainder = first_edge.length - projection_parameter;
    if first_end_remainder < f32::MIN_POSITIVE {
        result.points_on_first[0] = add(
            first_edge.start,
            scale(first_edge.direction, first_edge.length),
        );
        result.points_on_second[0] = subtract(
            second_edge.start,
            scale(second_edge.direction, first_end_remainder),
        );
    } else {
        result.points_on_first[0] = add(
            first_edge.start,
            scale(first_edge.direction, projection_parameter),
        );
        result.points_on_second[0] = second_edge.start;
    }

    let second_end_remainder = second_edge.length - projection_parameter;
    if second_end_remainder < f32::MIN_POSITIVE {
        result.points_on_first[1] = subtract(
            first_edge.start,
            scale(first_edge.direction, second_end_remainder),
        );
        result.points_on_second[1] = add(
            second_edge.start,
            scale(second_edge.direction, second_edge.length),
        );
    } else {
        result.points_on_first[1] = first_edge.start;
        result.points_on_second[1] = add(
            second_edge.start,
            scale(second_edge.direction, projection_parameter),
        );
    }
    Some(result)
}

fn single_feature_pair(
    point_on_first: [f32; 3],
    point_on_second: [f32; 3],
) -> FeatureIntersectionPrism {
    let mut result = FeatureIntersectionPrism {
        point_count: 1,
        ..FeatureIntersectionPrism::default()
    };
    result.points_on_first[0] = point_on_first;
    result.points_on_second[0] = point_on_second;
    result
}

fn face_edge_plane_distance(edge: &GpFeatureEdge, point: [f32; 3]) -> f32 {
    dot(edge.plane_normal, subtract(point, edge.start))
}

fn project_point_onto_feature_face(face: &GpFeature, point: [f32; 3]) -> [f32; 3] {
    add(
        point,
        scale(
            face.own_normal,
            dot(face.own_normal, subtract(face.edges[0].start, point)),
        ),
    )
}

fn push_face_pair_from_second(
    result: &mut FeatureIntersectionPrism,
    first: &GpFeature,
    point_on_second: [f32; 3],
) -> Option<()> {
    let index = usize::try_from(result.point_count).ok()?;
    if index >= result.points_on_first.len() {
        return None;
    }
    result.points_on_second[index] = point_on_second;
    result.points_on_first[index] = project_point_onto_feature_face(first, point_on_second);
    result.point_count += 1;
    Some(())
}

fn push_face_pair_from_first(
    result: &mut FeatureIntersectionPrism,
    second: &GpFeature,
    point_on_first: [f32; 3],
) -> Option<()> {
    let index = usize::try_from(result.point_count).ok()?;
    if index >= result.points_on_first.len() {
        return None;
    }
    result.points_on_first[index] = point_on_first;
    result.points_on_second[index] = project_point_onto_feature_face(second, point_on_first);
    result.point_count += 1;
    Some(())
}

impl GpPrimitive {
    /// Reconstructs each volume's `CreateGPInstance` method. `parent_transform`
    /// is the optional affine argument passed by `RenderWare`; `None` preserves
    /// the volume's local data.
    #[must_use]
    pub fn from_volume(volume: &Volume, parent_transform: Option<[f32; 16]>) -> Option<Self> {
        let primitive_type = match volume.volume_type()? {
            VolumeType::Sphere => GpPrimitiveType::Sphere,
            VolumeType::Capsule => GpPrimitiveType::Capsule,
            VolumeType::Triangle => GpPrimitiveType::Triangle,
            VolumeType::Box => GpPrimitiveType::Box,
            VolumeType::Cylinder => GpPrimitiveType::Cylinder,
            VolumeType::Null | VolumeType::Aggregate => return None,
        };
        let mut primitive = Self {
            primitive_type,
            points: [[0.0; 3]; 3],
            face_normals: [[0.0; 3]; 3],
            face_normal_count: 0,
            edge_directions: [[0.0; 3]; 3],
            edge_direction_count: 0,
            dimensions: [0.0; 3],
            fatness: volume.radius,
            edge_data: [0.0; 3],
            flags: volume.flags,
        };

        if primitive_type == GpPrimitiveType::Triangle {
            primitive.populate_triangle(volume, parent_transform);
            return Some(primitive);
        }

        let transform = parent_transform.map_or(volume.transform, |parent| {
            compose_affine(parent, volume.transform)
        });
        primitive.points[0] = affine_translation(transform);
        let axes = affine_axes(transform);
        match primitive_type {
            GpPrimitiveType::Sphere => {}
            GpPrimitiveType::Capsule => {
                primitive.edge_directions[0] = axes[2];
                primitive.edge_direction_count = 1;
                primitive.dimensions[0] = f32::from_bits(volume.shape_words[0]);
            }
            GpPrimitiveType::Box => {
                primitive.face_normals = axes;
                primitive.edge_directions = axes;
                primitive.face_normal_count = 3;
                primitive.edge_direction_count = 3;
                primitive.dimensions = volume.shape_words.map(f32::from_bits);
            }
            GpPrimitiveType::Cylinder => {
                // The specialized cylinder routines consume the two inactive
                // face-normal slots even though the generic count is one.
                primitive.face_normals = [axes[2], axes[1], axes[0]];
                primitive.face_normal_count = 1;
                primitive.edge_directions[0] = axes[2];
                primitive.edge_direction_count = 1;
                primitive.dimensions[0] = f32::from_bits(volume.shape_words[0]);
                primitive.dimensions[1] = f32::from_bits(volume.shape_words[1]);
            }
            GpPrimitiveType::Triangle => unreachable!(),
        }
        Some(primitive)
    }

    /// Builds the triangle GP instance used when a clustered-mesh leaf is
    /// expanded for a volume/volume overlap query.
    ///
    /// The point and cyclic-edge layout matches `TriangleVolume` construction:
    /// points remain `0,1,2`, while directed edges are `0->2`, `2->1`, and
    /// `1->0`. Cluster triangles have no separate rounded fatness.
    #[must_use]
    pub fn from_triangle_points(points: [[f32; 3]; 3]) -> Self {
        let normal = normalize_or_zero(cross(
            subtract(points[1], points[0]),
            subtract(points[2], points[0]),
        ));
        let edge_deltas = [
            subtract(points[2], points[0]),
            subtract(points[1], points[2]),
            subtract(points[0], points[1]),
        ];
        let mut edge_directions = [[0.0; 3]; 3];
        let mut dimensions = [0.0; 3];
        for (index, delta) in edge_deltas.into_iter().enumerate() {
            let length = length_squared(delta).sqrt();
            dimensions[index] = length;
            if length != 0.0 {
                edge_directions[index] = scale(delta, length.recip());
            }
        }
        Self {
            primitive_type: GpPrimitiveType::Triangle,
            points,
            face_normals: [normal, [0.0; 3], [0.0; 3]],
            face_normal_count: 1,
            edge_directions,
            edge_direction_count: 3,
            dimensions,
            fatness: 0.0,
            edge_data: [0.0; 3],
            flags: 2,
        }
    }

    fn populate_triangle(&mut self, volume: &Volume, parent_transform: Option<[f32; 16]>) {
        let local_points = std::array::from_fn(|column| affine_column(volume.transform, column));
        self.points = parent_transform.map_or(local_points, |parent| {
            local_points.map(|point| transform_point(parent, point))
        });

        let local_normal = if volume.flags & 2 == 0 {
            affine_translation(volume.transform)
        } else {
            normalize_or_zero(cross(
                subtract(local_points[1], local_points[0]),
                subtract(local_points[2], local_points[0]),
            ))
        };
        self.face_normals[0] = parent_transform.map_or(local_normal, |parent| {
            transform_direction(parent, local_normal)
        });
        self.face_normal_count = 1;

        // CreateGPInstance uses this exact cyclic order and stores the three
        // corresponding edge lengths in mDimensions at +0x70.
        let edge_deltas = [
            subtract(self.points[2], self.points[0]),
            subtract(self.points[1], self.points[2]),
            subtract(self.points[0], self.points[1]),
        ];
        for (index, delta) in edge_deltas.into_iter().enumerate() {
            let length = length_squared(delta).sqrt();
            self.dimensions[index] = length;
            self.edge_directions[index] = if length == 0.0 {
                [0.0; 3]
            } else {
                scale(delta, length.recip())
            };
        }
        self.edge_direction_count = 3;
        self.edge_data = volume.shape_words.map(f32::from_bits);
    }

    #[must_use]
    pub fn position(&self) -> [f32; 3] {
        self.points[0]
    }

    /// Reconstructs the shape-specific `GetInterval` method. Fatness is not
    /// included here; the reference adds both primitive radii and padding only
    /// after choosing the best separating direction.
    #[must_use]
    pub fn projection_interval(&self, direction: [f32; 3]) -> ProjectionInterval {
        match self.primitive_type {
            GpPrimitiveType::Sphere => point_interval(self.position(), direction),
            GpPrimitiveType::Capsule => {
                let center = dot(self.position(), direction);
                let extent = dot(self.edge_directions[0], direction).abs() * self.dimensions[0];
                ProjectionInterval {
                    minimum: center - extent,
                    maximum: center + extent,
                }
            }
            GpPrimitiveType::Triangle => {
                let projected = self.points.map(|point| dot(point, direction));
                ProjectionInterval {
                    minimum: projected[0].min(projected[1]).min(projected[2]),
                    maximum: projected[0].max(projected[1]).max(projected[2]),
                }
            }
            GpPrimitiveType::Box => {
                let center = dot(self.position(), direction);
                let extent = (0..3)
                    .map(|axis| {
                        dot(self.face_normals[axis], direction).abs() * self.dimensions[axis]
                    })
                    .sum::<f32>();
                ProjectionInterval {
                    minimum: center - extent,
                    maximum: center + extent,
                }
            }
            GpPrimitiveType::Cylinder => {
                let center = dot(self.position(), direction);
                let axial_projection = dot(self.edge_directions[0], direction);
                let radial_projection = (1.0 - axial_projection * axial_projection).abs().sqrt();
                let extent = axial_projection.abs() * self.dimensions[0]
                    + radial_projection * self.dimensions[1];
                ProjectionInterval {
                    minimum: center - extent,
                    maximum: center + extent,
                }
            }
        }
    }

    /// Reconstructs maximum features that do not need an explicit winding.
    /// Triangles, boxes, and cylinders use their explicit-winding methods.
    #[must_use]
    pub fn maximum_feature(&self, direction: [f32; 3]) -> Option<GpFeature> {
        match self.primitive_type {
            GpPrimitiveType::Sphere => Some(GpFeature {
                point: self.position(),
                ..GpFeature::default()
            }),
            GpPrimitiveType::Capsule => {
                let axis = self.edge_directions[0];
                let half_height = self.dimensions[0];
                let axis_projection = dot(axis, direction);
                if axis_projection.abs() >= FEATURE_SIMPLIFICATION_THRESHOLD {
                    let signed_height = if axis_projection > 0.0 {
                        half_height
                    } else {
                        -half_height
                    };
                    Some(GpFeature {
                        point: add(self.position(), scale(axis, signed_height)),
                        ..GpFeature::default()
                    })
                } else {
                    let positive_end = add(self.position(), scale(axis, half_height));
                    let negative_end = subtract(self.position(), scale(axis, half_height));
                    let mut feature = GpFeature::default();
                    feature.edges[0] = GpFeatureEdge::from_endpoints(positive_end, negative_end);
                    feature.edge_count = 1;
                    Some(feature)
                }
            }
            GpPrimitiveType::Triangle | GpPrimitiveType::Box | GpPrimitiveType::Cylinder => None,
        }
    }

    /// Reconstructs `GPBox::GetMaximumFeature` at `0x82acc360`.
    ///
    /// Axis alignment is classified with the reference's inclusive `0.2`
    /// boundary. Two low-alignment axes produce a face, one produces its
    /// parallel edge, and zero or three produce a vertex. Corner enumeration
    /// comes from `axis_corner_signs`; strict projection improvements retain
    /// the first corner on ties.
    #[must_use]
    pub fn box_maximum_feature(
        &self,
        counter_clockwise: bool,
        direction: [f32; 3],
    ) -> Option<GpFeature> {
        if self.primitive_type != GpPrimitiveType::Box {
            return None;
        }
        let axis_projection = self.face_normals.map(|axis| dot(axis, direction));
        let mut low_alignment_count = 0;
        let mut last_high_axis = 0;
        let mut last_low_axis = 0;
        for (axis_index, projection) in axis_projection.into_iter().enumerate() {
            if projection.abs() >= BOX_FACE_ALIGNMENT_THRESHOLD {
                last_high_axis = axis_index;
            } else {
                low_alignment_count += 1;
                last_low_axis = axis_index;
            }
        }

        match low_alignment_count {
            2 => Some(self.box_face_feature(
                counter_clockwise,
                direction,
                last_high_axis,
                axis_projection[last_high_axis],
            )),
            1 => Some(self.box_edge_feature(direction, last_low_axis)),
            _ => Some(self.box_vertex_feature(direction)),
        }
    }

    fn box_face_feature(
        &self,
        counter_clockwise: bool,
        direction: [f32; 3],
        face_axis: usize,
        face_projection: f32,
    ) -> GpFeature {
        let face_sign = if face_projection <= 0.0 { -1.0 } else { 1.0 };
        let face_center = add(
            self.position(),
            scale(
                self.face_normals[face_axis],
                self.dimensions[face_axis] * face_sign,
            ),
        );
        let other_axes = BOX_OTHER_AXES[face_axis];
        let corners = BOX_FACE_CORNER_SIGNS.map(|signs| {
            let first_offset = scale(
                self.face_normals[other_axes[0]],
                self.dimensions[other_axes[0]] * signs[0],
            );
            let second_offset = scale(
                self.face_normals[other_axes[1]],
                self.dimensions[other_axes[1]] * signs[1],
            );
            add(add(face_center, first_offset), second_offset)
        });
        let adjusted_sign = if face_axis == 1 {
            -face_sign
        } else {
            face_sign
        };
        let corner_order = if (adjusted_sign < 0.0) == counter_clockwise {
            [0, 1, 2, 3]
        } else {
            [0, 3, 2, 1]
        };

        let mut feature = GpFeature {
            own_normal: scale(self.face_normals[face_axis], face_sign),
            edge_count: 4,
            ..GpFeature::default()
        };
        for edge_index in 0..4 {
            let corner_index = corner_order[edge_index];
            let next_corner_index = corner_order[(edge_index + 1) % 4];
            let current_signs = BOX_FACE_CORNER_SIGNS[corner_index];
            let next_signs = BOX_FACE_CORNER_SIGNS[next_corner_index];
            let changed_slot = usize::from(current_signs[0].to_bits() == next_signs[0].to_bits());
            let edge_axis = other_axes[changed_slot];
            let edge_sign = (next_signs[changed_slot] - current_signs[changed_slot]) * 0.5;
            feature.edges[edge_index] = stored_feature_edge(
                corners[corner_index],
                scale(self.face_normals[edge_axis], edge_sign),
                self.dimensions[edge_axis] * 2.0,
            );
        }
        feature.build_edge_planes(counter_clockwise, direction);
        feature
    }

    fn box_edge_feature(&self, direction: [f32; 3], edge_axis: usize) -> GpFeature {
        let other_axes = BOX_OTHER_AXES[edge_axis];
        let midpoint = select_projected_box_point(
            self.position(),
            &self.face_normals,
            &self.dimensions,
            direction,
            &BOX_FACE_CORNER_SIGNS,
            other_axes,
        );
        let mut feature = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        feature.edges[0] = stored_feature_edge(
            subtract(
                midpoint,
                scale(self.face_normals[edge_axis], self.dimensions[edge_axis]),
            ),
            self.face_normals[edge_axis],
            self.dimensions[edge_axis] * 2.0,
        );
        feature
    }

    fn box_vertex_feature(&self, direction: [f32; 3]) -> GpFeature {
        let point = select_projected_box_vertex(
            self.position(),
            &self.face_normals,
            &self.dimensions,
            direction,
        );
        GpFeature {
            point,
            ..GpFeature::default()
        }
    }

    /// Reconstructs `GPCylinder::GetMaximumFeature` at `0x82add438`.
    ///
    /// The reference approximates a cap by a fixed eight-edge ring when the
    /// axial alignment is strictly above `0.99980003`. Alignment at or below
    /// `0.02` produces the axial support edge; the intervening range produces
    /// one rim point. The cap reverses both winding and axial position for a
    /// negative query direction.
    #[must_use]
    pub fn cylinder_maximum_feature(
        &self,
        mut counter_clockwise: bool,
        direction: [f32; 3],
    ) -> Option<GpFeature> {
        if self.primitive_type != GpPrimitiveType::Cylinder {
            return None;
        }

        let axis_x = self.face_normals[2];
        let axis_y = self.face_normals[1];
        let axis_z = self.face_normals[0];
        let local_direction = [
            dot(axis_x, direction),
            dot(axis_y, direction),
            dot(axis_z, direction),
        ];
        let half_height = self.dimensions[0];
        let radius = self.dimensions[1];
        let axial_alignment = local_direction[2];

        let mut feature = if axial_alignment.abs() > CYLINDER_CAP_ALIGNMENT_THRESHOLD {
            let axial_position = if axial_alignment < 0.0 {
                counter_clockwise = !counter_clockwise;
                -half_height
            } else {
                half_height
            };
            let diagonal = radius * SQRT_HALF;
            let ring = [
                [0.0, radius, axial_position],
                [diagonal, diagonal, axial_position],
                [-radius, 0.0, axial_position],
                [-diagonal, diagonal, axial_position],
                [radius, 0.0, axial_position],
                [diagonal, -diagonal, axial_position],
                [0.0, -radius, axial_position],
                [-diagonal, -diagonal, axial_position],
            ];
            let order = if counter_clockwise {
                [3, 2, 7, 6, 5, 4, 1, 0]
            } else {
                [0, 1, 4, 5, 6, 7, 2, 3]
            };
            let mut feature = GpFeature {
                own_normal: [0.0, 0.0, axial_position.signum()],
                edge_count: 8,
                ..GpFeature::default()
            };
            for edge_index in 0..8 {
                feature.edges[edge_index] = GpFeatureEdge::from_endpoints(
                    ring[order[edge_index]],
                    ring[order[(edge_index + 1) % 8]],
                );
            }
            // The inlined edge-plane builder uses the canonical local cylinder
            // axis here, not the near-axial query direction.
            feature.build_edge_planes(counter_clockwise, [0.0, 0.0, 1.0]);
            feature
        } else if axial_alignment.abs() <= CYLINDER_EDGE_ALIGNMENT_THRESHOLD {
            // Unlike the point path below, the executable deliberately leaves
            // the small axial component in this normalization before replacing
            // the endpoint Z coordinates with the two cap heights.
            let support_direction = normalize_or_zero(local_direction);
            let radial_x = support_direction[0] * radius;
            let radial_y = support_direction[1] * radius;
            let mut feature = GpFeature {
                own_normal: local_direction,
                edge_count: 1,
                ..GpFeature::default()
            };
            feature.edges[0] = GpFeatureEdge::from_endpoints(
                [radial_x, radial_y, half_height],
                [radial_x, radial_y, -half_height],
            );
            feature
        } else {
            let radial_direction = normalize_or_zero([local_direction[0], local_direction[1], 0.0]);
            GpFeature {
                own_normal: local_direction,
                point: [
                    radial_direction[0] * radius,
                    radial_direction[1] * radius,
                    if axial_alignment < 0.0 {
                        -half_height
                    } else {
                        half_height
                    },
                ],
                ..GpFeature::default()
            }
        };

        feature.own_normal =
            cylinder_local_direction_to_world(axis_x, axis_y, axis_z, feature.own_normal);
        feature.point =
            cylinder_local_point_to_world(self.position(), axis_x, axis_y, axis_z, feature.point);
        let edge_count = usize::try_from(feature.edge_count)
            .unwrap_or(0)
            .min(feature.edges.len());
        for edge in &mut feature.edges[..edge_count] {
            edge.start =
                cylinder_local_point_to_world(self.position(), axis_x, axis_y, axis_z, edge.start);
            edge.direction =
                cylinder_local_direction_to_world(axis_x, axis_y, axis_z, edge.direction);
            edge.plane_normal =
                cylinder_local_direction_to_world(axis_x, axis_y, axis_z, edge.plane_normal);
        }
        Some(feature)
    }

    /// Reconstructs only the face-aligned branch of
    /// `GPTriangle::GetMaximumFeature` at `0x82ace110`.
    ///
    /// `None` means either that this is not a triangle or that the direction
    /// requires the still-untranslated edge/vertex classification path. Face
    /// selection is strict: an absolute normal projection of exactly `0.95`
    /// does not enter this branch.
    #[must_use]
    pub fn triangle_face_feature(
        &self,
        counter_clockwise: bool,
        direction: [f32; 3],
    ) -> Option<GpFeature> {
        if self.primitive_type != GpPrimitiveType::Triangle {
            return None;
        }
        let normal_projection = dot(self.face_normals[0], direction);
        if normal_projection.abs() <= TRIANGLE_FACE_MINIMUM_ALIGNMENT {
            return None;
        }

        let mut feature = GpFeature {
            own_normal: self.face_normals[0],
            edge_count: 3,
            ..GpFeature::default()
        };
        if (normal_projection < 0.0) == counter_clockwise {
            feature.region = 8;
            feature.edges[0] =
                stored_feature_edge(self.points[0], self.edge_directions[0], self.dimensions[0]);
            feature.edges[1] =
                stored_feature_edge(self.points[2], self.edge_directions[1], self.dimensions[1]);
            feature.edges[2] =
                stored_feature_edge(self.points[1], self.edge_directions[2], self.dimensions[2]);
        } else {
            feature.edges[0] = stored_feature_edge(
                self.points[0],
                negate(self.edge_directions[2]),
                self.dimensions[2],
            );
            feature.edges[1] = stored_feature_edge(
                self.points[1],
                negate(self.edge_directions[1]),
                self.dimensions[1],
            );
            feature.edges[2] = stored_feature_edge(
                self.points[2],
                negate(self.edge_directions[0]),
                self.dimensions[0],
            );
        }
        feature.build_edge_planes(counter_clockwise, direction);
        Some(feature)
    }

    /// Reconstructs all branches of `GPTriangle::GetMaximumFeature` at
    /// `0x82ace110`, including its projected-vertex tie ordering and strict
    /// edge simplification tests.
    #[must_use]
    pub fn triangle_maximum_feature(
        &self,
        counter_clockwise: bool,
        direction: [f32; 3],
    ) -> Option<GpFeature> {
        if self.primitive_type != GpPrimitiveType::Triangle {
            return None;
        }
        if let Some(feature) = self.triangle_face_feature(counter_clockwise, direction) {
            return Some(feature);
        }

        let point_projection = self.points.map(|point| dot(direction, point));
        let selected_point = if point_projection[0] > point_projection[1]
            && point_projection[0] > point_projection[2]
        {
            0
        } else if point_projection[1] > point_projection[2] {
            1
        } else {
            2
        };
        let edge_alignment = self.edge_directions.map(|edge| dot(direction, edge).abs());
        let selected_edge = match selected_point {
            0 if edge_alignment[0] < FEATURE_SIMPLIFICATION_THRESHOLD
                && edge_alignment[2] > edge_alignment[0] =>
            {
                Some(0)
            }
            1 if edge_alignment[1] < FEATURE_SIMPLIFICATION_THRESHOLD
                && edge_alignment[2] > edge_alignment[1] =>
            {
                Some(1)
            }
            0 | 1 if edge_alignment[2] < FEATURE_SIMPLIFICATION_THRESHOLD => Some(2),
            2 if edge_alignment[0] < FEATURE_SIMPLIFICATION_THRESHOLD
                && edge_alignment[1] > edge_alignment[0] =>
            {
                Some(0)
            }
            2 if edge_alignment[1] < FEATURE_SIMPLIFICATION_THRESHOLD => Some(1),
            _ => None,
        };

        let mut feature = GpFeature::default();
        if let Some(edge_index) = selected_edge {
            let edge_start = [self.points[0], self.points[2], self.points[1]];
            feature.edges[0] = stored_feature_edge(
                edge_start[edge_index],
                self.edge_directions[edge_index],
                self.dimensions[edge_index],
            );
            feature.edge_count = 1;
        } else {
            feature.point = self.points[selected_point];
        }
        feature.build_edge_planes(counter_clockwise, direction);
        Some(feature)
    }

    /// Reconstructs the standalone triangle feature classifier at
    /// `0x82acb278`. The comparison order and strict `.05` boundaries are
    /// significant because overlapping boundary conditions keep the first
    /// matching region.
    #[must_use]
    pub fn triangle_feature_type_from_normal(
        &self,
        normal_direction: [f32; 3],
    ) -> Option<TriangleFeatureType> {
        if self.primitive_type != GpPrimitiveType::Triangle {
            return None;
        }
        Some(triangle_feature_type_from_projections(
            self.edge_directions.map(|edge| dot(normal_direction, edge)),
        ))
    }

    fn maximum_feature_for_overlap(
        &self,
        counter_clockwise: bool,
        direction: [f32; 3],
    ) -> Option<GpFeature> {
        match self.primitive_type {
            GpPrimitiveType::Triangle => {
                self.triangle_maximum_feature(counter_clockwise, direction)
            }
            GpPrimitiveType::Box => self.box_maximum_feature(counter_clockwise, direction),
            GpPrimitiveType::Cylinder => {
                self.cylinder_maximum_feature(counter_clockwise, direction)
            }
            _ => self.maximum_feature(direction),
        }
    }
}

/// Candidate list generated by `FindBestSepDir_FindCandidates` at
/// `0x82aca230`. Face normals are intentionally appended in reverse index
/// order; edge cross products use forward nested order.
#[must_use]
pub fn generic_separating_direction_candidates(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Vec<[f32; 3]> {
    let mut candidates = Vec::with_capacity(17);
    append_face_normals_reverse(&mut candidates, first);
    append_face_normals_reverse(&mut candidates, second);

    for first_edge in first
        .edge_directions
        .iter()
        .take(usize::from(first.edge_direction_count))
    {
        for second_edge in second
            .edge_directions
            .iter()
            .take(usize::from(second.edge_direction_count))
        {
            append_normalized_above(
                &mut candidates,
                cross(*first_edge, *second_edge),
                EDGE_CROSS_MINIMUM_LENGTH_SQUARED,
            );
        }
    }

    if first.edge_direction_count == 1 && second.edge_direction_count == 1 {
        let first_edge = first.edge_directions[0];
        let second_edge = second.edge_directions[0];
        let mutual_normal = cross(first_edge, second_edge);
        if length_squared(mutual_normal) > DIRECTION_MINIMUM_LENGTH_SQUARED {
            let mutual_normal = normalize_or_zero(mutual_normal);
            append_normalized_above(
                &mut candidates,
                cross(mutual_normal, first_edge),
                DIRECTION_MINIMUM_LENGTH_SQUARED,
            );
            append_normalized_above(
                &mut candidates,
                cross(mutual_normal, second_edge),
                DIRECTION_MINIMUM_LENGTH_SQUARED,
            );
        }
    }

    if candidates.is_empty() && first.edge_direction_count == 1 && second.edge_direction_count == 1
    {
        append_center_delta_perpendicular(&mut candidates, first, second, first.edge_directions[0]);
    }

    if first.edge_direction_count + second.edge_direction_count == 1 {
        let edge = if first.edge_direction_count == 1 {
            first.edge_directions[0]
        } else {
            second.edge_directions[0]
        };
        append_center_delta_perpendicular(&mut candidates, first, second, edge);
    }

    if candidates.is_empty() {
        append_normalized_above(
            &mut candidates,
            subtract(second.position(), first.position()),
            DIRECTION_MINIMUM_LENGTH_SQUARED,
        );
    }
    candidates
}

/// Reconstructs the generic separator at `0x82aca8a0`. This is not the
/// specialized sphere/sphere, box/box, triangle/box, or cylinder dispatch and
/// is not the complete overlap predicate.
#[must_use]
pub fn generic_separating_direction(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> SeparatingDirection {
    let candidates = generic_separating_direction_candidates(first, second);
    best_separating_direction_from_candidates(first, second, &candidates)
        .unwrap_or_else(|| separation_on_direction(first, second, [0.0, 0.0, 1.0]))
}

/// Reconstructs the 15 separating axes evaluated by
/// `FindBestSeparatingDirectionBoxBox` at `0x82ad2c58`.
///
/// The vectorized reference stores both boxes' face normals in reverse order,
/// followed by all nine forward-nested edge crosses. Unlike the generic
/// builder, its specialized path does not apply the `.001` small-cross gate;
/// every cross is normalized, including degenerate parallel pairs.
#[must_use]
pub fn box_box_separating_candidates(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if first.primitive_type != GpPrimitiveType::Box || second.primitive_type != GpPrimitiveType::Box
    {
        return None;
    }

    let mut candidates = Vec::with_capacity(15);
    append_face_normals_reverse(&mut candidates, first);
    append_face_normals_reverse(&mut candidates, second);
    for first_edge in first.edge_directions {
        for second_edge in second.edge_directions {
            candidates.push(normalize_unchecked(cross(first_edge, second_edge)));
        }
    }
    Some(candidates)
}

/// Runs the specialized strict first-best policy over the box/box axes.
#[must_use]
pub fn box_box_separating_direction(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<SeparatingDirection> {
    let candidates = box_box_separating_candidates(first, second)?;
    best_separating_direction_from_candidates(first, second, &candidates)
}

/// Reconstructs the 13 separating axes evaluated by
/// `FindBestSeparatingDirectionTriBox` at `0x82ad31a0`.
///
/// Its first axis is the triangle face normal. Three box normals follow in
/// reverse storage order, then all nine normalized triangle-edge/box-edge
/// crosses in forward nested order. The specialized code deliberately omits
/// the generic candidate builder's `.001` cross-length filter.
#[must_use]
pub fn triangle_box_separating_candidates(
    triangle: &GpPrimitive,
    box_primitive: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if triangle.primitive_type != GpPrimitiveType::Triangle
        || box_primitive.primitive_type != GpPrimitiveType::Box
    {
        return None;
    }

    let mut candidates = Vec::with_capacity(13);
    candidates.push(triangle.face_normals[0]);
    append_face_normals_reverse(&mut candidates, box_primitive);
    for triangle_edge in triangle.edge_directions {
        for box_edge in box_primitive.edge_directions {
            candidates.push(normalize_unchecked(cross(triangle_edge, box_edge)));
        }
    }
    Some(candidates)
}

/// Runs the specialized strict first-best policy over the triangle/box axes.
#[must_use]
pub fn triangle_box_separating_direction(
    triangle: &GpPrimitive,
    box_primitive: &GpPrimitive,
) -> Option<SeparatingDirection> {
    let candidates = triangle_box_separating_candidates(triangle, box_primitive)?;
    best_separating_direction_from_candidates(triangle, box_primitive, &candidates)
}

/// Reconstructs `AddLineToEdgeCandidate` at `0x82acac68` as one typed
/// candidate result.
///
/// The reference first uses `cross(line_direction, edge_direction)`. A cross
/// whose squared length is strictly below `f32::MIN_POSITIVE` enters the
/// parallel double-cross path and falls back to the line direction only when
/// that second squared length is also strictly below the same constant. For a
/// non-parallel pair, the infinite-edge parameter is clamped only when it lies
/// strictly outside `middle +/- half_length`; exact endpoints retain the
/// original line/edge cross candidate.
#[must_use]
pub fn line_edge_separating_candidate(
    line_base: [f32; 3],
    line_direction: [f32; 3],
    edge_base: [f32; 3],
    edge_direction: [f32; 3],
    edge_middle: f32,
    edge_half_length: f32,
) -> [f32; 3] {
    let line_edge_cross = cross(line_direction, edge_direction);
    if f32::MIN_POSITIVE > length_squared(line_edge_cross) {
        let base_delta = subtract(line_base, edge_base);
        let parallel_candidate = cross(cross(line_direction, base_delta), line_direction);
        if f32::MIN_POSITIVE > length_squared(parallel_candidate) {
            return line_direction;
        }
        return normalize_or_zero(parallel_candidate);
    }

    let edge_parameter_plane = cross(line_direction, line_edge_cross);
    let edge_parameter = dot(edge_parameter_plane, subtract(line_base, edge_base))
        / dot(edge_parameter_plane, edge_direction);
    let edge_offset = edge_parameter - edge_middle;
    let clamped_parameter = if edge_offset > edge_half_length {
        Some(edge_middle + edge_half_length)
    } else if -edge_half_length > edge_offset {
        Some(edge_middle - edge_half_length)
    } else {
        None
    };

    if let Some(parameter) = clamped_parameter {
        let endpoint = add(edge_base, scale(edge_direction, parameter));
        // This intentionally mirrors the executable: the endpoint is used
        // directly in the double cross; line_base is not subtracted here.
        normalize_or_zero(cross(line_direction, cross(endpoint, line_direction)))
    } else {
        normalize_or_zero(line_edge_cross)
    }
}

/// Reconstructs `RimToEdge` at `0x82ad3710` as one typed separating-axis
/// candidate for a circular rim and a finite edge.
///
/// The reference orders the edge endpoints by their signed rim-normal
/// projection, seeds the search with the lower endpoint and either the
/// in-segment rim-plane projection or the upper endpoint, and refines a
/// signed stationary-point bracket for at most ten iterations. The refinement
/// uses the executable's decaying half-weight rather than an unconstrained
/// closest-points solve. Final pair selection is strict, so equal derivative
/// magnitudes retain the upper pair.
#[must_use]
pub fn rim_to_edge_separating_candidate(
    center: [f32; 3],
    normal: [f32; 3],
    radius: f32,
    edge_start: [f32; 3],
    edge_end: [f32; 3],
) -> [f32; 3] {
    let edge_direction = subtract(edge_end, edge_start);
    let (lower_edge, upper_edge) = if dot(edge_direction, normal) < 0.0 {
        (edge_end, edge_start)
    } else {
        (edge_start, edge_end)
    };

    let edge_in_rim_plane = cross(cross(normal, edge_direction), normal);
    let projection_denominator = dot(edge_direction, edge_in_rim_plane);
    let projection_numerator = dot(subtract(center, edge_start), edge_in_rim_plane);
    let projection_parameter = if projection_denominator > 0.0 {
        projection_numerator / projection_denominator
    } else {
        projection_numerator * projection_denominator
    };
    let projected_edge = std::array::from_fn(|axis| {
        edge_direction[axis].mul_add(projection_parameter, edge_start[axis])
    });
    let second_edge = if projection_parameter > 0.0 && 1.0 > projection_parameter {
        projected_edge
    } else {
        upper_edge
    };

    let mut upper_edge = second_edge;
    let mut upper_rim = nearest_rim_point(center, normal, radius, upper_edge);
    let mut upper_metric = dot(subtract(upper_rim, upper_edge), edge_direction);
    let mut lower_edge = lower_edge;
    let mut lower_rim = nearest_rim_point(center, normal, radius, lower_edge);
    let mut lower_metric = dot(subtract(lower_rim, lower_edge), edge_direction);

    if upper_metric * lower_metric > 0.0 {
        let (rim, edge) = if lower_metric.abs() > upper_metric.abs() {
            (upper_rim, upper_edge)
        } else {
            (lower_rim, lower_edge)
        };
        return rim_edge_pair_candidate(center, normal, edge_direction, rim, edge);
    }

    if lower_metric > upper_metric {
        std::mem::swap(&mut upper_edge, &mut lower_edge);
        std::mem::swap(&mut upper_rim, &mut lower_rim);
        std::mem::swap(&mut upper_metric, &mut lower_metric);
    }

    let mut weight = 1.0_f32;
    for _ in 0..10 {
        if upper_metric.partial_cmp(&lower_metric) != Some(std::cmp::Ordering::Greater) {
            break;
        }

        let metric_span = upper_metric - lower_metric;
        let weight_denominator = weight + 1.0;
        weight *= 0.5;
        let interpolation = (upper_metric / metric_span + weight) / weight_denominator;
        let complement = 1.0 - interpolation;
        let edge = std::array::from_fn(|axis| {
            lower_edge[axis].mul_add(interpolation, upper_edge[axis] * complement)
        });
        let rim = nearest_rim_point(center, normal, radius, edge);
        let metric = dot(subtract(rim, edge), edge_direction);
        if 0.0 > metric {
            lower_edge = edge;
            lower_rim = rim;
            lower_metric = metric;
        } else {
            upper_edge = edge;
            upper_rim = rim;
            upper_metric = metric;
        }
    }

    let (rim, edge) = if upper_metric > -lower_metric {
        (lower_rim, lower_edge)
    } else {
        (upper_rim, upper_edge)
    };
    rim_edge_pair_candidate(center, normal, edge_direction, rim, edge)
}

/// Reconstructs `NearestRimToEdge` at `0x82ad4a00`.
///
/// The helper chooses one cylinder cap before delegating to `RimToEdge`. Its
/// signed test uses the unnormalized edge double cross
/// `cross(edge, cross(edge, edge_start-cylinder_center))`. A strictly negative
/// projection on the cylinder axis selects the positive cap; zero, positive,
/// and unordered values select the negative cap.
#[must_use]
pub fn nearest_cylinder_rim_to_edge_separating_candidate(
    cylinder_center: [f32; 3],
    cylinder_axis: [f32; 3],
    cylinder_half_length: f32,
    cylinder_radius: f32,
    edge_start: [f32; 3],
    edge_end: [f32; 3],
) -> [f32; 3] {
    let edge = subtract(edge_end, edge_start);
    let edge_to_center = cross(edge, cross(edge, subtract(edge_start, cylinder_center)));
    let cap_normal = if 0.0 > dot(edge_to_center, cylinder_axis) {
        cylinder_axis
    } else {
        negate(cylinder_axis)
    };
    let cap_center = std::array::from_fn(|axis| {
        cap_normal[axis].mul_add(cylinder_half_length, cylinder_center[axis])
    });
    rim_to_edge_separating_candidate(
        cap_center,
        cap_normal,
        cylinder_radius,
        edge_start,
        edge_end,
    )
}

fn nearest_rim_point(center: [f32; 3], normal: [f32; 3], radius: f32, point: [f32; 3]) -> [f32; 3] {
    let center_delta = subtract(point, center);
    let axial = dot(center_delta, normal);
    let radial = std::array::from_fn(|axis| normal[axis].mul_add(-axial, center_delta[axis]));
    let radial_squared = length_squared(radial);
    let radial_direction = if f32::MIN_POSITIVE > radial_squared {
        radial
    } else {
        scale(radial, radial_squared.sqrt().recip())
    };
    std::array::from_fn(|axis| radial_direction[axis].mul_add(radius, center[axis]))
}

fn rim_edge_pair_candidate(
    center: [f32; 3],
    normal: [f32; 3],
    edge_direction: [f32; 3],
    rim: [f32; 3],
    edge: [f32; 3],
) -> [f32; 3] {
    let pair_delta = subtract(edge, rim);
    let pair_delta_squared = length_squared(pair_delta);
    if pair_delta_squared > f32::MIN_POSITIVE {
        return scale(pair_delta, pair_delta_squared.sqrt().recip());
    }

    let fallback = cross(cross(subtract(rim, center), normal), edge_direction);
    let fallback_squared = length_squared(fallback);
    if fallback_squared > f32::MIN_POSITIVE {
        scale(fallback, fallback_squared.sqrt().recip())
    } else {
        normal
    }
}

/// Reconstructs `RimToRim` at `0x82ad3b98` as an optional separating-axis
/// candidate between two circular rims.
///
/// `line_direction` is the normalized intersection direction supplied by
/// `AddRimToRimCandidates`. The reference can reject a rim pair before
/// emitting an axis, handles its near-coplanar case separately, and otherwise
/// refines a signed stationary-point bracket for at most ten iterations.
#[must_use]
#[allow(clippy::too_many_lines, clippy::manual_midpoint)]
pub fn rim_to_rim_separating_candidate(
    center_first: [f32; 3],
    normal_first: [f32; 3],
    radius_first: f32,
    center_second: [f32; 3],
    normal_second: [f32; 3],
    radius_second: f32,
    line_direction: [f32; 3],
) -> Option<[f32; 3]> {
    let tolerance = radius_first.min(radius_second) * 1.0e-7;
    if f32::MIN_POSITIVE > tolerance {
        return None;
    }

    let normal_cross = cross(normal_second, normal_first);
    let first_shadow = subtract(center_first, scale(normal_cross, radius_first));
    let first_gate = cross(
        line_direction,
        cross(subtract(first_shadow, center_second), line_direction),
    );
    if radius_second * radius_second > length_squared(first_gate) {
        return None;
    }

    let second_seed = add(
        center_second,
        scale(cross(normal_second, line_direction), radius_second),
    );
    let second_gate = cross(
        normal_first,
        cross(subtract(second_seed, center_first), normal_first),
    );
    if radius_first * radius_first > length_squared(second_gate) {
        return None;
    }

    if tolerance > dot(subtract(center_first, center_second), normal_second).abs() {
        let first_rim = nearest_rim_point(center_first, normal_first, radius_first, second_seed);
        let second_rim = nearest_rim_point(center_second, normal_second, radius_second, first_rim);
        let delta = subtract(second_rim, first_rim);
        let delta_squared = length_squared(delta);
        return Some(if delta_squared > f32::MIN_POSITIVE {
            scale(delta, delta_squared.sqrt().recip())
        } else {
            normalize_unchecked(cross(add(normal_first, normal_second), normal_second))
        });
    }

    let center_delta = subtract(center_first, center_second);
    let center_distance_squared = length_squared(center_delta);
    let radius_sum = radius_first + radius_second;
    let mut second_projection_seed = center_first;
    if radius_sum * radius_sum > center_distance_squared
        && center_distance_squared > f32::MIN_POSITIVE
    {
        let center_distance = center_distance_squared.sqrt();
        // Preserve the reference add-then-multiply ordering; `midpoint`
        // deliberately has different overflow behavior.
        let semiperimeter = (center_distance + radius_sum) * 0.5;
        let factor = (2.0 * (radius_second * (semiperimeter - radius_second)).sqrt())
            / (center_distance * radius_first);
        second_projection_seed = std::array::from_fn(|axis| {
            (first_shadow[axis] - center_first[axis]).mul_add(factor, center_first[axis])
        });
    }

    let mut lower_second = second_seed;
    let mut lower_first = nearest_rim_point(center_first, normal_first, radius_first, lower_second);
    let second_seed_delta = subtract(second_projection_seed, center_second);
    let second_seed_perpendicular = std::array::from_fn(|axis| {
        line_direction[axis].mul_add(
            -dot(second_seed_delta, line_direction),
            second_seed_delta[axis],
        )
    });
    let mut upper_second = add(
        center_second,
        scale(
            normalize_preserving_subnormal(second_seed_perpendicular),
            radius_second,
        ),
    );
    let mut upper_first = nearest_rim_point(center_first, normal_first, radius_first, upper_second);
    let mut lower_metric =
        rim_pair_metric(center_second, line_direction, lower_first, lower_second);
    let mut upper_metric =
        rim_pair_metric(center_second, line_direction, upper_first, upper_second);

    if lower_metric * upper_metric > 0.0 {
        lower_second = std::array::from_fn(|axis| {
            center_second[axis] + center_second[axis] - lower_second[axis]
        });
        lower_first = nearest_rim_point(center_first, normal_first, radius_first, lower_second);
        lower_metric = rim_pair_metric(center_second, line_direction, lower_first, lower_second);
        if lower_metric * upper_metric > 0.0 {
            return None;
        }
    }

    if 0.0 > upper_metric {
        std::mem::swap(&mut upper_first, &mut lower_first);
        std::mem::swap(&mut upper_second, &mut lower_second);
        std::mem::swap(&mut upper_metric, &mut lower_metric);
    }

    let mut weight = 1.0_f32;
    for _ in 0..10 {
        if upper_metric > -lower_metric {
            if lower_metric > -tolerance {
                return Some(rim_pair_candidate(
                    center_first,
                    normal_first,
                    center_second,
                    line_direction,
                    lower_first,
                    lower_second,
                ));
            }
        } else if tolerance > upper_metric {
            return Some(rim_pair_candidate(
                center_first,
                normal_first,
                center_second,
                line_direction,
                upper_first,
                upper_second,
            ));
        }

        let metric_span = upper_metric - lower_metric;
        let weight_denominator = weight + 1.0;
        weight *= 0.5;
        let interpolation = (upper_metric / metric_span + weight) / weight_denominator;
        let complement = 1.0 - interpolation;
        let interpolated_second = std::array::from_fn(|axis| {
            lower_second[axis].mul_add(interpolation, upper_second[axis] * complement)
        });
        let second = add(
            center_second,
            scale(
                normalize_unchecked(subtract(interpolated_second, center_second)),
                radius_second,
            ),
        );
        let first = nearest_rim_point(center_first, normal_first, radius_first, second);
        let metric = rim_pair_metric(center_second, line_direction, first, second);
        if 0.0 > metric {
            lower_first = first;
            lower_second = second;
            lower_metric = metric;
        } else {
            upper_first = first;
            upper_second = second;
            upper_metric = metric;
        }
    }

    let (first, second) = if upper_metric > -lower_metric {
        (lower_first, lower_second)
    } else {
        (upper_first, upper_second)
    };
    Some(rim_pair_candidate(
        center_first,
        normal_first,
        center_second,
        line_direction,
        first,
        second,
    ))
}

fn normalize_preserving_subnormal(value: [f32; 3]) -> [f32; 3] {
    let squared = length_squared(value);
    if f32::MIN_POSITIVE > squared {
        value
    } else {
        scale(value, squared.sqrt().recip())
    }
}

fn normalize_unchecked(value: [f32; 3]) -> [f32; 3] {
    scale(value, length_squared(value).sqrt().recip())
}

fn rim_pair_metric(
    center_second: [f32; 3],
    line_direction: [f32; 3],
    first: [f32; 3],
    second: [f32; 3],
) -> f32 {
    dot(
        subtract(second, center_second),
        cross(line_direction, subtract(first, center_second)),
    )
}

fn rim_pair_candidate(
    center_first: [f32; 3],
    normal_first: [f32; 3],
    center_second: [f32; 3],
    line_direction: [f32; 3],
    first: [f32; 3],
    second: [f32; 3],
) -> [f32; 3] {
    let delta = subtract(second, first);
    let delta_squared = length_squared(delta);
    if delta_squared > f32::MIN_POSITIVE {
        return scale(delta, delta_squared.sqrt().recip());
    }

    let fallback = cross(
        cross(subtract(first, center_first), normal_first),
        cross(subtract(second, center_second), line_direction),
    );
    let fallback_squared = length_squared(fallback);
    if fallback_squared > f32::MIN_POSITIVE {
        scale(fallback, fallback_squared.sqrt().recip())
    } else {
        normal_first
    }
}

/// Reconstructs `AddRimToRimCandidates` at `0x82ad46d0` for two cylinders.
///
/// Parallel axes emit no rim/rim entries. Otherwise the executable visits the
/// four signed cap pairs in `(+,+)`, `(-,+)`, `(+,-)`, `(-,-)` order and only
/// appends pairs accepted by `RimToRim`. A single cap-sign change reverses the
/// supplied rim-intersection direction; changing both restores it.
#[must_use]
pub fn cylinder_rim_to_rim_candidates(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if first.primitive_type != GpPrimitiveType::Cylinder
        || second.primitive_type != GpPrimitiveType::Cylinder
    {
        return None;
    }

    let first_axis = first.edge_directions[0];
    let second_axis = second.edge_directions[0];
    let axis_cross = cross(second_axis, first_axis);
    let axis_cross_squared = length_squared(axis_cross);
    if axis_cross_squared.partial_cmp(&f32::MIN_POSITIVE) != Some(std::cmp::Ordering::Greater) {
        return Some(Vec::new());
    }
    let line_direction = scale(axis_cross, axis_cross_squared.sqrt().recip());

    let first_offset = scale(first_axis, first.dimensions[0]);
    let second_offset = scale(second_axis, second.dimensions[0]);
    let first_positive = add(first.position(), first_offset);
    let first_negative = subtract(first.position(), first_offset);
    let second_positive = add(second.position(), second_offset);
    let second_negative = subtract(second.position(), second_offset);
    let negative_first_axis = negate(first_axis);
    let negative_second_axis = negate(second_axis);
    let negative_line_direction = negate(line_direction);

    let cap_pairs = [
        (
            first_positive,
            first_axis,
            second_positive,
            second_axis,
            line_direction,
        ),
        (
            first_negative,
            negative_first_axis,
            second_positive,
            second_axis,
            negative_line_direction,
        ),
        (
            first_positive,
            first_axis,
            second_negative,
            negative_second_axis,
            negative_line_direction,
        ),
        (
            first_negative,
            negative_first_axis,
            second_negative,
            negative_second_axis,
            line_direction,
        ),
    ];
    let mut candidates = Vec::with_capacity(4);
    for (first_center, first_normal, second_center, second_normal, line) in cap_pairs {
        if let Some(candidate) = rim_to_rim_separating_candidate(
            first_center,
            first_normal,
            first.dimensions[1],
            second_center,
            second_normal,
            second.dimensions[1],
            line,
        ) {
            candidates.push(candidate);
        }
    }
    Some(candidates)
}

/// Reconstructs the candidate-selection policy used by the specialized
/// cylinder `ChooseBestCandidate` routine at `0x82adafb0`.
///
/// Each candidate is first oriented by its larger interval gap; the original
/// direction owns an equal-gap tie. A candidate replaces the current winner
/// only for strictly greater separation, so the earliest equal candidate is
/// retained. The executable's cylinder generators always provide at least one
/// candidate; this typed boundary returns `None` for an empty input.
#[must_use]
pub fn best_separating_direction_from_candidates(
    first: &GpPrimitive,
    second: &GpPrimitive,
    candidates: &[[f32; 3]],
) -> Option<SeparatingDirection> {
    let (first_candidate, remaining) = candidates.split_first()?;
    let mut best = separation_on_direction(first, second, *first_candidate);
    for candidate in remaining {
        let current = separation_on_direction(first, second, *candidate);
        if current.separation > best.separation {
            best = current;
        }
    }
    Some(best)
}

fn separation_on_direction(
    first: &GpPrimitive,
    second: &GpPrimitive,
    candidate: [f32; 3],
) -> SeparatingDirection {
    let first_interval = first.projection_interval(candidate);
    let second_interval = second.projection_interval(candidate);
    let first_to_second = second_interval.minimum - first_interval.maximum;
    let second_to_first = first_interval.minimum - second_interval.maximum;
    if second_to_first > first_to_second {
        SeparatingDirection {
            direction: negate(candidate),
            separation: second_to_first,
        }
    } else {
        SeparatingDirection {
            direction: candidate,
            separation: first_to_second,
        }
    }
}

/// Complete sphere/sphere result, including the point/point feature-prism
/// path. The comparison is strict on rejection, matching `Overlap`.
#[must_use]
pub fn spheres_overlap(first: &GpPrimitive, second: &GpPrimitive, padding: f32) -> Option<bool> {
    if first.primitive_type != GpPrimitiveType::Sphere
        || second.primitive_type != GpPrimitiveType::Sphere
    {
        return None;
    }
    let center_delta = subtract(second.position(), first.position());
    let separation = length_squared(center_delta).sqrt();
    Some(separation <= (first.fatness + second.fatness) + padding)
}

/// Reconstructs the sphere-specific branch of the cylinder candidate builder
/// at `0x82ad4ac0` and its strict first-best selection at `0x82adafb0`.
/// The returned direction is oriented from the cylinder toward the sphere.
#[must_use]
pub fn cylinder_sphere_separating_direction(
    cylinder: &GpPrimitive,
    sphere: &GpPrimitive,
) -> Option<SeparatingDirection> {
    if cylinder.primitive_type != GpPrimitiveType::Cylinder
        || sphere.primitive_type != GpPrimitiveType::Sphere
    {
        return None;
    }

    let axis = cylinder.edge_directions[0];
    let mut candidates = [axis, [0.0; 3]];
    let mut candidate_count = 1;
    let center_delta = subtract(sphere.position(), cylinder.position());
    let mut rim_plane_normal = cross(center_delta, axis);
    if f32::MIN_POSITIVE > length_squared(rim_plane_normal) {
        // The executable uses this sign-sensitive comparison and chooses only
        // between global I/J. Preserve it instead of replacing it with a
        // conventional least-aligned-axis heuristic.
        let fallback = if axis[0] > 0.5 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        rim_plane_normal = cross(fallback, axis);
    }
    if length_squared(rim_plane_normal) != 0.0 {
        candidates[1] = cross(normalize_or_zero(rim_plane_normal), axis);
        candidate_count = 2;
    }
    best_separating_direction_from_candidates(cylinder, sphere, &candidates[..candidate_count])
}

/// Reconstructs the one-edge branch of cylinder `GetCandidates` at
/// `0x82ad4ac0` for a capsule, followed by `ChooseBestCandidate`.
///
/// Candidate order is observable because strictly equal separations retain
/// the first entry: cylinder axis, positive cap rim against the capsule core
/// edge, negative cap rim, then the infinite cylinder axis against the finite
/// capsule edge.
#[must_use]
pub fn cylinder_capsule_separating_direction(
    cylinder: &GpPrimitive,
    capsule: &GpPrimitive,
) -> Option<SeparatingDirection> {
    if cylinder.primitive_type != GpPrimitiveType::Cylinder
        || capsule.primitive_type != GpPrimitiveType::Capsule
    {
        return None;
    }

    let cylinder_center = cylinder.position();
    let cylinder_axis = cylinder.edge_directions[0];
    let cylinder_half_length = cylinder.dimensions[0];
    let cylinder_radius = cylinder.dimensions[1];
    let capsule_center = capsule.position();
    let capsule_axis = capsule.edge_directions[0];
    let capsule_half_length = capsule.dimensions[0];
    let positive_rim_center = std::array::from_fn(|axis| {
        cylinder_axis[axis].mul_add(cylinder_half_length, cylinder_center[axis])
    });
    let negative_rim_center = std::array::from_fn(|axis| {
        (-cylinder_axis[axis]).mul_add(cylinder_half_length, cylinder_center[axis])
    });
    let capsule_edge_start = std::array::from_fn(|axis| {
        (-capsule_axis[axis]).mul_add(capsule_half_length, capsule_center[axis])
    });
    let capsule_edge_end = std::array::from_fn(|axis| {
        capsule_axis[axis].mul_add(capsule_half_length, capsule_center[axis])
    });

    let candidates = [
        cylinder_axis,
        rim_to_edge_separating_candidate(
            positive_rim_center,
            cylinder_axis,
            cylinder_radius,
            capsule_edge_start,
            capsule_edge_end,
        ),
        rim_to_edge_separating_candidate(
            negative_rim_center,
            negate(cylinder_axis),
            cylinder_radius,
            capsule_edge_start,
            capsule_edge_end,
        ),
        line_edge_separating_candidate(
            cylinder_center,
            cylinder_axis,
            capsule_center,
            capsule_axis,
            0.0,
            capsule_half_length,
        ),
    ];
    best_separating_direction_from_candidates(cylinder, capsule, &candidates)
}

/// Reconstructs the complete cylinder/cylinder candidate list emitted by
/// `GetCandidates` at `0x82ad4ac0`.
///
/// Ordering is tie-sensitive: both cylinder axes, the second cylinder's two
/// cap rims against the first core edge, accepted rim/rim pairs, the first
/// cylinder's two cap rims against the second core edge, and finally the
/// first infinite axis against the second finite core edge. The executable
/// notably retains the positive second-axis normal for both candidates in
/// the first rim/edge pair.
#[must_use]
pub fn cylinder_cylinder_separating_candidates(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if first.primitive_type != GpPrimitiveType::Cylinder
        || second.primitive_type != GpPrimitiveType::Cylinder
    {
        return None;
    }

    let first_center = first.position();
    let first_axis = first.edge_directions[0];
    let first_half_length = first.dimensions[0];
    let first_radius = first.dimensions[1];
    let second_center = second.position();
    let second_axis = second.edge_directions[0];
    let second_half_length = second.dimensions[0];
    let second_radius = second.dimensions[1];
    let first_positive =
        std::array::from_fn(|axis| first_axis[axis].mul_add(first_half_length, first_center[axis]));
    let first_negative = subtract(first_center, scale(first_axis, first_half_length));
    let second_positive = std::array::from_fn(|axis| {
        second_axis[axis].mul_add(second_half_length, second_center[axis])
    });
    let second_negative = subtract(second_center, scale(second_axis, second_half_length));
    let first_edge_start = std::array::from_fn(|axis| {
        (-first_axis[axis]).mul_add(first_half_length, first_center[axis])
    });
    let first_edge_end = first_positive;
    let second_edge_start = std::array::from_fn(|axis| {
        (-second_axis[axis]).mul_add(second_half_length, second_center[axis])
    });
    let second_edge_end = second_positive;

    let mut candidates = Vec::with_capacity(11);
    candidates.push(first_axis);
    candidates.push(second_axis);
    candidates.push(rim_to_edge_separating_candidate(
        second_positive,
        second_axis,
        second_radius,
        first_edge_start,
        first_edge_end,
    ));
    candidates.push(rim_to_edge_separating_candidate(
        second_negative,
        second_axis,
        second_radius,
        first_edge_start,
        first_edge_end,
    ));
    candidates.extend(cylinder_rim_to_rim_candidates(first, second)?);
    candidates.push(rim_to_edge_separating_candidate(
        first_positive,
        first_axis,
        first_radius,
        second_edge_start,
        second_edge_end,
    ));
    candidates.push(rim_to_edge_separating_candidate(
        first_negative,
        negate(first_axis),
        first_radius,
        second_edge_start,
        second_edge_end,
    ));
    candidates.push(line_edge_separating_candidate(
        first_center,
        first_axis,
        second_center,
        second_axis,
        0.0,
        second_half_length,
    ));
    Some(candidates)
}

/// Runs the specialized strict first-best chooser over the complete
/// cylinder/cylinder `GetCandidates` result.
#[must_use]
pub fn cylinder_cylinder_separating_direction(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<SeparatingDirection> {
    let candidates = cylinder_cylinder_separating_candidates(first, second)?;
    best_separating_direction_from_candidates(first, second, &candidates)
}

/// Reconstructs the cylinder/triangle slice of `GetCandidates` at
/// `0x82ad587c` in its observable eight-entry order.
///
/// The triangle edges follow `CreateGPInstance` storage rather than numeric
/// vertex order: `0 -> 2`, `2 -> 1`, then `1 -> 0`. Each finite-edge call uses
/// the edge start as its base and the reference's equal middle/half-length
/// values (`stored_length * 0.5`).
#[must_use]
pub fn cylinder_triangle_separating_candidates(
    cylinder: &GpPrimitive,
    triangle: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if cylinder.primitive_type != GpPrimitiveType::Cylinder
        || triangle.primitive_type != GpPrimitiveType::Triangle
    {
        return None;
    }

    let cylinder_center = cylinder.position();
    let cylinder_axis = cylinder.edge_directions[0];
    let cylinder_half_length = cylinder.dimensions[0];
    let cylinder_radius = cylinder.dimensions[1];
    let triangle_edges = [
        (triangle.points[0], triangle.points[2]),
        (triangle.points[2], triangle.points[1]),
        (triangle.points[1], triangle.points[0]),
    ];

    let mut candidates = Vec::with_capacity(8);
    candidates.push(cylinder_axis);
    candidates.push(triangle.face_normals[0]);
    for (edge_start, edge_end) in triangle_edges {
        candidates.push(nearest_cylinder_rim_to_edge_separating_candidate(
            cylinder_center,
            cylinder_axis,
            cylinder_half_length,
            cylinder_radius,
            edge_start,
            edge_end,
        ));
    }
    for (index, (edge_start, _)) in triangle_edges.into_iter().enumerate() {
        let edge_half_length = triangle.dimensions[index] * 0.5;
        candidates.push(line_edge_separating_candidate(
            cylinder_center,
            cylinder_axis,
            edge_start,
            triangle.edge_directions[index],
            edge_half_length,
            edge_half_length,
        ));
    }
    Some(candidates)
}

/// Runs the specialized strict first-best chooser over the complete
/// cylinder/triangle candidate list.
#[must_use]
pub fn cylinder_triangle_separating_direction(
    cylinder: &GpPrimitive,
    triangle: &GpPrimitive,
) -> Option<SeparatingDirection> {
    let candidates = cylinder_triangle_separating_candidates(cylinder, triangle)?;
    best_separating_direction_from_candidates(cylinder, triangle, &candidates)
}

fn box_signed_extents_toward(box_primitive: &GpPrimitive, point: [f32; 3]) -> [f32; 3] {
    let delta = subtract(point, box_primitive.position());
    std::array::from_fn(|axis| {
        if 0.0 > dot(delta, box_primitive.edge_directions[axis]) {
            -box_primitive.dimensions[axis]
        } else {
            box_primitive.dimensions[axis]
        }
    })
}

fn box_signed_extents_along(box_primitive: &GpPrimitive, direction: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        if 0.0 > dot(direction, box_primitive.edge_directions[axis]) {
            -box_primitive.dimensions[axis]
        } else {
            box_primitive.dimensions[axis]
        }
    })
}

fn box_corner_from_signed_extents(
    box_primitive: &GpPrimitive,
    signed_extents: [f32; 3],
) -> [f32; 3] {
    let box_center = box_primitive.position();
    std::array::from_fn(|component| {
        let first_two = box_primitive.edge_directions[0][component].mul_add(
            signed_extents[0],
            box_primitive.edge_directions[1][component] * signed_extents[1],
        );
        let offset =
            box_primitive.edge_directions[2][component].mul_add(signed_extents[2], first_two);
        offset + box_center[component]
    })
}

fn rim_to_parametric_edge_separating_candidate(
    rim_center: [f32; 3],
    rim_normal: [f32; 3],
    rim_radius: f32,
    edge_base: [f32; 3],
    edge_direction: [f32; 3],
    edge_middle: f32,
    edge_half_length: f32,
) -> [f32; 3] {
    let lower = edge_middle - edge_half_length;
    let upper = edge_middle + edge_half_length;
    let edge_start = std::array::from_fn(|component| {
        edge_direction[component].mul_add(lower, edge_base[component])
    });
    let edge_end = std::array::from_fn(|component| {
        edge_direction[component].mul_add(upper, edge_base[component])
    });
    rim_to_edge_separating_candidate(rim_center, rim_normal, rim_radius, edge_start, edge_end)
}

/// Reconstructs the cylinder/box slice of `GetCandidates` at `0x82ad4f68`
/// in its complete, tie-sensitive 16-entry order.
///
/// After the cylinder axis and three stored box normals, each cylinder cap rim
/// is paired with the three incident edges at the box corner selected toward
/// that cap. The executable deliberately supplies the positive cylinder axis
/// as the rim normal for both cap groups. Its final six axes pair the cylinder
/// center line with the six box silhouette edges selected along that axis.
#[must_use]
pub fn cylinder_box_separating_candidates(
    cylinder: &GpPrimitive,
    box_primitive: &GpPrimitive,
) -> Option<Vec<[f32; 3]>> {
    if cylinder.primitive_type != GpPrimitiveType::Cylinder
        || box_primitive.primitive_type != GpPrimitiveType::Box
    {
        return None;
    }

    let cylinder_center = cylinder.position();
    let cylinder_axis = cylinder.edge_directions[0];
    let cylinder_half_length = cylinder.dimensions[0];
    let cylinder_radius = cylinder.dimensions[1];
    let positive_cap = std::array::from_fn(|component| {
        cylinder_axis[component].mul_add(cylinder_half_length, cylinder_center[component])
    });
    let negative_cap = subtract(cylinder_center, scale(cylinder_axis, cylinder_half_length));

    let mut candidates = Vec::with_capacity(16);
    candidates.push(cylinder_axis);
    candidates.extend(box_primitive.face_normals);

    for cap_center in [positive_cap, negative_cap] {
        let signed_extents = box_signed_extents_toward(box_primitive, cap_center);
        let corner = box_corner_from_signed_extents(box_primitive, signed_extents);
        for (axis, signed_extent) in signed_extents.into_iter().enumerate() {
            candidates.push(rim_to_parametric_edge_separating_candidate(
                cap_center,
                cylinder_axis,
                cylinder_radius,
                corner,
                box_primitive.edge_directions[axis],
                -signed_extent,
                box_primitive.dimensions[axis],
            ));
        }
    }

    let signed_extents = box_signed_extents_along(box_primitive, cylinder_axis);
    let silhouette_corners = [
        [-signed_extents[0], signed_extents[1], signed_extents[2]],
        [signed_extents[0], -signed_extents[1], signed_extents[2]],
        [signed_extents[0], signed_extents[1], -signed_extents[2]],
    ]
    .map(|extents| box_corner_from_signed_extents(box_primitive, extents));
    for (corner, edge_axis) in [(0, 1), (0, 2), (1, 2), (1, 0), (2, 0), (2, 1)] {
        candidates.push(line_edge_separating_candidate(
            cylinder_center,
            cylinder_axis,
            silhouette_corners[corner],
            box_primitive.edge_directions[edge_axis],
            -signed_extents[edge_axis],
            box_primitive.dimensions[edge_axis],
        ));
    }
    Some(candidates)
}

/// Runs the specialized strict first-best chooser over the complete
/// cylinder/box candidate list.
#[must_use]
pub fn cylinder_box_separating_direction(
    cylinder: &GpPrimitive,
    box_primitive: &GpPrimitive,
) -> Option<SeparatingDirection> {
    let candidates = cylinder_box_separating_candidates(cylinder, box_primitive)?;
    best_separating_direction_from_candidates(cylinder, box_primitive, &candidates)
}

/// Complete specialized cylinder/sphere overlap in either argument order.
#[must_use]
pub fn cylinder_sphere_overlap(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
) -> Option<bool> {
    let separating = match (first.primitive_type, second.primitive_type) {
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Sphere) => {
            cylinder_sphere_separating_direction(first, second)?
        }
        (GpPrimitiveType::Sphere, GpPrimitiveType::Cylinder) => {
            let mut separating = cylinder_sphere_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        _ => return None,
    };
    Some(separating.separation <= (first.fatness + second.fatness) + padding)
}

/// Complete overlap result when exactly one input is a sphere and the other
/// is a sphere, capsule, triangle, box, or cylinder. Every opposing maximum
/// feature is paired with the sphere's point feature; all three reference
/// point-feature prism paths return success after point projection. Cylinders
/// use their separate rim-aware separator.
#[must_use]
pub fn sphere_primitive_overlap(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
) -> Option<bool> {
    let supported = matches!(
        (first.primitive_type, second.primitive_type),
        (
            GpPrimitiveType::Sphere
                | GpPrimitiveType::Capsule
                | GpPrimitiveType::Triangle
                | GpPrimitiveType::Box
                | GpPrimitiveType::Cylinder,
            GpPrimitiveType::Sphere
        ) | (
            GpPrimitiveType::Sphere,
            GpPrimitiveType::Capsule
                | GpPrimitiveType::Triangle
                | GpPrimitiveType::Box
                | GpPrimitiveType::Cylinder
        )
    );
    if !supported {
        return None;
    }
    if first.primitive_type == GpPrimitiveType::Sphere
        && second.primitive_type == GpPrimitiveType::Sphere
    {
        return spheres_overlap(first, second, padding);
    }
    if first.primitive_type == GpPrimitiveType::Cylinder
        || second.primitive_type == GpPrimitiveType::Cylinder
    {
        return cylinder_sphere_overlap(first, second, padding);
    }
    let separation = generic_separating_direction(first, second).separation;
    Some(separation <= (first.fatness + second.fatness) + padding)
}

/// Complete primitive-pair overlap dispatch. Sphere/capsule/triangle pairs and
/// box against sphere or capsule use the generic separator; box/box,
/// triangle/box, and every cylinder pair use their specialized reference
/// separators before entering the common maximum-feature/prism tail.
#[must_use]
pub fn generic_overlap_proven_subset(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
) -> Option<bool> {
    if first.primitive_type == GpPrimitiveType::Sphere
        && second.primitive_type == GpPrimitiveType::Sphere
    {
        return spheres_overlap(first, second, padding);
    }
    match try_sk8_pre_triangle_fixup_candidate(first, second, padding)? {
        PreTriangleFixupOutcome::Separated => Some(false),
        PreTriangleFixupOutcome::Candidate(candidate) => {
            // This public predicate intentionally stops at the generic
            // RenderWare prism boundary. The Skate 2 ground producer must not
            // consume this candidate until its mandatory triangle fix-up has
            // independently accepted it.
            let _ = (
                candidate.separating,
                candidate.prism,
                candidate.result_normal,
            );
            Some(true)
        }
    }
}

/// `rw::collision::Overlap` 0x82ad6188: `findBestSepDirFuncs[type1][type2]`,
/// then `separation > (fatness1 + fatness2) + padding` rejects (`fcmpu`/`bgt`,
/// so an unordered separation overlaps). Both maximum features and
/// `FindFeatureIntersectionPrism` 0x82ad2350 follow, but every prism branch
/// returns nonzero (point/point 1, `FindEdgePointPrism` 0x82ac911c,
/// `FindFacePointPrism` 0x82ac9968, `FindEdgeEdgePrism`, `FindFaceEdgePrism`
/// and the general face/face tail all `li r3, 1`), so the separation decides.
#[must_use]
pub fn rw_overlap(first: &GpPrimitive, second: &GpPrimitive, padding: f32) -> Option<bool> {
    let separation = if first.primitive_type == GpPrimitiveType::Sphere
        && second.primitive_type == GpPrimitiveType::Sphere
    {
        length_squared(subtract(second.position(), first.position())).sqrt()
    } else {
        complete_separating_direction(first, second)?.separation
    };
    Some(!(separation > (first.fatness + second.fatness) + padding))
}

fn complete_separating_direction(
    first: &GpPrimitive,
    second: &GpPrimitive,
) -> Option<SeparatingDirection> {
    Some(match (first.primitive_type, second.primitive_type) {
        (GpPrimitiveType::Box, GpPrimitiveType::Box) => {
            box_box_separating_direction(first, second)?
        }
        (GpPrimitiveType::Triangle, GpPrimitiveType::Box) => {
            triangle_box_separating_direction(first, second)?
        }
        (GpPrimitiveType::Box, GpPrimitiveType::Triangle) => {
            let mut separating = triangle_box_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Sphere) => {
            cylinder_sphere_separating_direction(first, second)?
        }
        (GpPrimitiveType::Sphere, GpPrimitiveType::Cylinder) => {
            let mut separating = cylinder_sphere_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Capsule) => {
            cylinder_capsule_separating_direction(first, second)?
        }
        (GpPrimitiveType::Capsule, GpPrimitiveType::Cylinder) => {
            let mut separating = cylinder_capsule_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Cylinder) => {
            cylinder_cylinder_separating_direction(first, second)?
        }
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Triangle) => {
            cylinder_triangle_separating_direction(first, second)?
        }
        (GpPrimitiveType::Triangle, GpPrimitiveType::Cylinder) => {
            let mut separating = cylinder_triangle_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        (GpPrimitiveType::Cylinder, GpPrimitiveType::Box) => {
            cylinder_box_separating_direction(first, second)?
        }
        (GpPrimitiveType::Box, GpPrimitiveType::Cylinder) => {
            let mut separating = cylinder_box_separating_direction(second, first)?;
            separating.direction = negate(separating.direction);
            separating
        }
        _ => generic_separating_direction(first, second),
    })
}

fn find_feature_intersection_prism(
    first: &mut GpFeature,
    second: &mut GpFeature,
    separation_direction: [f32; 3],
) -> Option<FeatureIntersectionPrism> {
    if first.edge_count == 0 || second.edge_count == 0 {
        return point_involved_feature_prism(first, second, separation_direction);
    }
    if first.edge_count == 1 && second.edge_count == 1 {
        return edge_edge_feature_prism(first, second, separation_direction);
    }
    if first.edge_count > 1 && second.edge_count == 1 {
        return face_edge_feature_prism(first, second, separation_direction);
    }
    if first.edge_count == 1 && second.edge_count > 1 {
        let mut reversed = face_edge_feature_prism(second, first, separation_direction)?;
        std::mem::swap(
            &mut reversed.points_on_first,
            &mut reversed.points_on_second,
        );
        return Some(reversed);
    }
    if first.edge_count > 1 && second.edge_count > 1 {
        return face_face_feature_prism(first, second);
    }
    None
}

fn refine_reciprocal_sqrt_twice(value: f32, estimate: f32) -> f32 {
    let square_0 = estimate * estimate;
    let half_0 = estimate * 0.5;
    let residual_0 = (-value).mul_add(square_0, 1.0);
    let refined_1 = half_0.mul_add(residual_0, estimate);
    let square_1 = refined_1 * refined_1;
    let half_1 = refined_1 * 0.5;
    let residual_1 = (-value).mul_add(square_1, 1.0);
    half_1.mul_add(residual_1, refined_1)
}

/// Numeric operations whose exact result bits depend on Xenon VMX behavior.
///
/// `vmsum3fp128` reduction and `vrsqrtefp128` estimate bits remain injected
/// boundaries. The surrounding operation order is reconstructed natively.
trait TriangleFixupNumeric {
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32;
    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32;
}

struct HostTriangleFixupNumeric;

impl TriangleFixupNumeric for HostTriangleFixupNumeric {
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
        dot(left, right)
    }

    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
        value.sqrt().recip()
    }
}

#[allow(dead_code)]
struct InjectedTriangleFixupNumeric<R, S> {
    sum_three_products: R,
    reciprocal_sqrt_seed: S,
}

impl<R, S> TriangleFixupNumeric for InjectedTriangleFixupNumeric<R, S>
where
    R: FnMut([f32; 3], [f32; 3]) -> f32,
    S: FnMut(f32) -> f32,
{
    fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
        (self.sum_three_products)(left, right)
    }

    fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
        (self.reciprocal_sqrt_seed)(value)
    }
}

fn bit_negate(value: [f32; 3]) -> [f32; 3] {
    value.map(|component| f32::from_bits(component.to_bits() ^ 0x8000_0000))
}

fn bit_negate_scalar(value: f32) -> f32 {
    f32::from_bits(value.to_bits() ^ 0x8000_0000)
}

fn triangle_feature_type_from_projections(
    [toward_02, toward_21, toward_10]: [f32; 3],
) -> TriangleFeatureType {
    let threshold = FEATURE_SIMPLIFICATION_THRESHOLD;

    if toward_02 > threshold && -threshold > toward_21 {
        return TriangleFeatureType::Vertex2;
    }
    if toward_21 > threshold && -threshold > toward_10 {
        return TriangleFeatureType::Vertex1;
    }
    if toward_10 > threshold && -threshold > toward_02 {
        return TriangleFeatureType::Vertex0;
    }
    if toward_02 > -toward_10 && -toward_21 >= toward_02 {
        return TriangleFeatureType::Edge20;
    }
    if toward_21 > -toward_02 && -toward_10 >= toward_21 {
        return TriangleFeatureType::Edge12;
    }
    if toward_10 > -toward_21 && -toward_02 >= toward_10 {
        return TriangleFeatureType::Edge01;
    }
    TriangleFeatureType::Face
}

fn triangle_feature_type_from_normal_with_numeric<N: TriangleFixupNumeric>(
    triangle: &GpPrimitive,
    normal_direction: [f32; 3],
    numeric: &mut N,
) -> TriangleFeatureType {
    triangle_feature_type_from_projections([
        numeric.sum_three_products(normal_direction, triangle.edge_directions[0]),
        numeric.sum_three_products(normal_direction, triangle.edge_directions[1]),
        numeric.sum_three_products(normal_direction, triangle.edge_directions[2]),
    ])
}

fn normalize_with_estimate<N: TriangleFixupNumeric>(value: [f32; 3], numeric: &mut N) -> [f32; 3] {
    let squared_length = numeric.sum_three_products(value, value);
    let seed = numeric.reciprocal_sqrt_seed(squared_length);
    scale(value, refine_reciprocal_sqrt_twice(squared_length, seed))
}

/// Source-shaped `E x T` schedule used by `EdgeCosTest` and
/// `EdgeCosTestFull`: separately rounded multiplies, one fused negative
/// multiply-add, then the final YZX permutation.
fn triangle_edge_cross(edge: [f32; 3], triangle_normal: [f32; 3]) -> [f32; 3] {
    let partial = [
        edge[0] * triangle_normal[1],
        edge[1] * triangle_normal[2],
        edge[2] * triangle_normal[0],
    ];
    let permuted = [
        (-edge[1]).mul_add(triangle_normal[0], partial[0]),
        (-edge[2]).mul_add(triangle_normal[1], partial[1]),
        (-edge[0]).mul_add(triangle_normal[2], partial[2]),
    ];
    [permuted[1], permuted[2], permuted[0]]
}

/// `EdgeCosTest` at `0x82acb060`.
fn edge_cos_test<N: TriangleFixupNumeric>(
    edge: [f32; 3],
    triangle_normal: [f32; 3],
    contact_normal: [f32; 3],
    edge_cos: f32,
    convex: bool,
    numeric: &mut N,
) -> bool {
    let cross = triangle_edge_cross(edge, triangle_normal);
    if numeric.sum_three_products(contact_normal, cross) < 0.0 {
        return true;
    }
    if !convex {
        return false;
    }
    edge_cos_projected_test(edge, triangle_normal, contact_normal, edge_cos, numeric)
}

/// `EdgeCosTestFull` at `0x82acb138`.
fn edge_cos_test_full<N: TriangleFixupNumeric>(
    edge: [f32; 3],
    triangle_normal: [f32; 3],
    contact_normal: [f32; 3],
    edge_cos: f32,
    convex: bool,
    numeric: &mut N,
) -> bool {
    let cross = triangle_edge_cross(edge, triangle_normal);
    if numeric.sum_three_products(contact_normal, cross) < 0.0 {
        return true;
    }
    let comparison_normal = if convex {
        triangle_normal
    } else {
        bit_negate(triangle_normal)
    };
    edge_cos_projected_test(edge, comparison_normal, contact_normal, edge_cos, numeric)
}

fn edge_cos_projected_test<N: TriangleFixupNumeric>(
    edge: [f32; 3],
    comparison_normal: [f32; 3],
    contact_normal: [f32; 3],
    edge_cos: f32,
    numeric: &mut N,
) -> bool {
    let along = numeric.sum_three_products(contact_normal, edge);
    let projected = subtract(contact_normal, scale(edge, along));
    let unit_projected = normalize_with_estimate(projected, numeric);
    numeric.sum_three_products(unit_projected, comparison_normal) >= edge_cos
}

fn fused_scale_add(direction: [f32; 3], scalar: f32, base: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| direction[axis].mul_add(scalar, base[axis]))
}

/// `FixUpOneSidedTriangleEdge` at `0x82acb418`.
///
/// A malformed positive point count above the typed 16-pair capacity returns
/// `None` before mutation. Valid native callers cannot reach that host safety
/// boundary; the reference itself performs no explicit cap check.
#[allow(clippy::too_many_arguments)]
fn fix_up_one_sided_triangle_edge<N: TriangleFixupNumeric>(
    edge_is_convex: bool,
    alignment: f32,
    edge_cos: f32,
    result_normal: &mut [f32; 3],
    reverse: bool,
    contact_normal: [f32; 3],
    triangle_normal: [f32; 3],
    prism: &mut FeatureIntersectionPrism,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<u32> {
    if edge_is_convex {
        return Some(u32::from(alignment + convexity_epsilon >= edge_cos));
    }
    let object_near_flat = is_object && f64::from((edge_cos - 1.0).abs()) < 1.0e-5_f64;
    if object_near_flat {
        return Some(u32::from(alignment + convexity_epsilon >= edge_cos));
    }
    if edge_cos > bend_normal_threshold || edge_cos <= TRIANGLE_MINIMUM_CLIPPING_ANGLE {
        return Some(0);
    }

    let edge_cos_squared = edge_cos * edge_cos;
    // The raw chronology rounds the scalar square, reduces this dot, and
    // only then subtracts the square and issues the reciprocal-root estimate.
    let face_dot = numeric.sum_three_products(contact_normal, triangle_normal);
    let root_squared = 1.0 - edge_cos_squared;
    let seed = numeric.reciprocal_sqrt_seed(root_squared);
    let reciprocal_root = refine_reciprocal_sqrt_twice(root_squared, seed);
    let root = if root_squared == 0.0 {
        0.0
    } else {
        root_squared * reciprocal_root
    };
    if face_dot.partial_cmp(&root) != Some(std::cmp::Ordering::Greater) {
        return Some(0);
    }

    let point_count = if prism.point_count > 0 {
        usize::try_from(prism.point_count).ok()?
    } else {
        0
    };
    if point_count > prism.points_on_first.len() {
        return None;
    }

    *result_normal = if reverse {
        bit_negate(triangle_normal)
    } else {
        triangle_normal
    };
    for index in 0..point_count {
        if reverse {
            let delta = subtract(prism.points_on_second[index], prism.points_on_first[index]);
            let height = numeric.sum_three_products(delta, triangle_normal);
            prism.points_on_second[index] =
                fused_scale_add(triangle_normal, height, prism.points_on_first[index]);
        } else {
            let delta = subtract(prism.points_on_first[index], prism.points_on_second[index]);
            let height = numeric.sum_three_products(delta, triangle_normal);
            prism.points_on_first[index] =
                fused_scale_add(triangle_normal, height, prism.points_on_second[index]);
        }
    }
    Some(1)
}

#[derive(Clone, Copy)]
struct TriangleEdgeFixup {
    direction_index: usize,
    cosine_index: usize,
    convex_mask: u32,
}

const EDGE_01_FIXUP: TriangleEdgeFixup = TriangleEdgeFixup {
    direction_index: 2,
    cosine_index: 0,
    convex_mask: FLAG_TRIANGLE_EDGE_01_CONVEX,
};
const EDGE_12_FIXUP: TriangleEdgeFixup = TriangleEdgeFixup {
    direction_index: 1,
    cosine_index: 1,
    convex_mask: FLAG_TRIANGLE_EDGE_12_CONVEX,
};
const EDGE_20_FIXUP: TriangleEdgeFixup = TriangleEdgeFixup {
    direction_index: 0,
    cosine_index: 2,
    convex_mask: FLAG_TRIANGLE_EDGE_20_CONVEX,
};

fn direct_triangle_region_return(flags: u32, region: TriangleFeatureType) -> u32 {
    match region {
        TriangleFeatureType::Edge01 => flags & FLAG_TRIANGLE_EDGE_01_CONVEX,
        TriangleFeatureType::Edge12 => flags & FLAG_TRIANGLE_EDGE_12_CONVEX,
        TriangleFeatureType::Edge20 => flags & FLAG_TRIANGLE_EDGE_20_CONVEX,
        TriangleFeatureType::Vertex0 => u32::from(flags & FLAG_TRIANGLE_VERTEX_0_DISABLE == 0),
        TriangleFeatureType::Vertex1 => u32::from(flags & FLAG_TRIANGLE_VERTEX_1_DISABLE == 0),
        TriangleFeatureType::Vertex2 => u32::from(flags & FLAG_TRIANGLE_VERTEX_2_DISABLE == 0),
        TriangleFeatureType::Face => 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn one_sided_edge_region<N: TriangleFixupNumeric>(
    edge: TriangleEdgeFixup,
    triangle: &GpPrimitive,
    alignment: f32,
    result_normal: &mut [f32; 3],
    reverse: bool,
    contact_normal: [f32; 3],
    triangle_normal: [f32; 3],
    prism: &mut FeatureIntersectionPrism,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<u32> {
    fix_up_one_sided_triangle_edge(
        triangle.flags & edge.convex_mask != 0,
        alignment,
        triangle.edge_data[edge.cosine_index],
        result_normal,
        reverse,
        contact_normal,
        triangle_normal,
        prism,
        bend_normal_threshold,
        convexity_epsilon,
        is_object,
        numeric,
    )
}

fn vertex_edge_cos_tests<N: TriangleFixupNumeric>(
    triangle: &GpPrimitive,
    contact_normal: [f32; 3],
    triangle_normal: [f32; 3],
    first: TriangleEdgeFixup,
    second: TriangleEdgeFixup,
    full: bool,
    numeric: &mut N,
) -> bool {
    for edge in [first, second] {
        let direction = bit_negate(triangle.edge_directions[edge.direction_index]);
        let convex = triangle.flags & edge.convex_mask != 0;
        let accepted = if full {
            edge_cos_test_full(
                direction,
                triangle_normal,
                contact_normal,
                triangle.edge_data[edge.cosine_index],
                convex,
                numeric,
            )
        } else {
            edge_cos_test(
                direction,
                triangle_normal,
                contact_normal,
                triangle.edge_data[edge.cosine_index],
                convex,
                numeric,
            )
        };
        if !accepted {
            return false;
        }
    }
    true
}

/// Full `rw::collision::FixUpTriangleResult` at `0x82acb5d0`.
///
/// The native `FixupReason*` parameter is omitted because this retail build
/// never reads or writes it. `None` is reserved for invalid host-side typed
/// inputs; successful native paths preserve their exact integer return.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn fix_up_triangle_result_with_numeric<N: TriangleFixupNumeric>(
    triangle: &GpPrimitive,
    result_normal: &mut [f32; 3],
    reverse: bool,
    prism: &mut FeatureIntersectionPrism,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<u32> {
    if triangle.primitive_type != GpPrimitiveType::Triangle {
        return None;
    }

    // Native code snapshots both values before the only mutating helper can
    // update `result_normal` and either FIP point array.
    let contact_normal = if reverse {
        bit_negate(*result_normal)
    } else {
        *result_normal
    };
    let triangle_normal = triangle.face_normals[0];
    let alignment = numeric.sum_three_products(contact_normal, triangle_normal);
    let mut region = TriangleFeatureType::Face;
    if alignment.abs() < TRIANGLE_FIXUP_ALIGNMENT_CUTOFF {
        let tangent = subtract(contact_normal, scale(triangle_normal, alignment));
        let unit_tangent = normalize_with_estimate(tangent, numeric);
        region = triangle_feature_type_from_normal_with_numeric(triangle, unit_tangent, numeric);
    }

    let flags = triangle.flags;
    let one_sided = flags & FLAG_TRIANGLE_ONE_SIDED != 0;
    if flags & FLAG_TRIANGLE_USE_EDGE_COS == 0 {
        if region == TriangleFeatureType::Face {
            return Some(if one_sided {
                u32::from(alignment > 0.0)
            } else {
                1
            });
        }
        if one_sided {
            if alignment > TRIANGLE_FIXUP_ALIGNMENT_CUTOFF {
                return Some(1);
            }
            if alignment < 0.0 {
                return Some(0);
            }
        } else if alignment.abs() > TRIANGLE_FIXUP_ALIGNMENT_CUTOFF {
            return Some(1);
        }
        return Some(direct_triangle_region_return(flags, region));
    }

    if one_sided {
        return match region {
            TriangleFeatureType::Face => Some(u32::from(alignment > 0.0)),
            TriangleFeatureType::Edge01 => one_sided_edge_region(
                EDGE_01_FIXUP,
                triangle,
                alignment,
                result_normal,
                reverse,
                contact_normal,
                triangle_normal,
                prism,
                bend_normal_threshold,
                convexity_epsilon,
                is_object,
                numeric,
            ),
            TriangleFeatureType::Edge12 => one_sided_edge_region(
                EDGE_12_FIXUP,
                triangle,
                alignment,
                result_normal,
                reverse,
                contact_normal,
                triangle_normal,
                prism,
                bend_normal_threshold,
                convexity_epsilon,
                is_object,
                numeric,
            ),
            TriangleFeatureType::Edge20 => one_sided_edge_region(
                EDGE_20_FIXUP,
                triangle,
                alignment,
                result_normal,
                reverse,
                contact_normal,
                triangle_normal,
                prism,
                bend_normal_threshold,
                convexity_epsilon,
                is_object,
                numeric,
            ),
            TriangleFeatureType::Vertex0 => {
                if flags & FLAG_TRIANGLE_VERTEX_0_DISABLE != 0 && !is_object {
                    Some(0)
                } else {
                    Some(u32::from(vertex_edge_cos_tests(
                        triangle,
                        contact_normal,
                        triangle_normal,
                        EDGE_20_FIXUP,
                        EDGE_01_FIXUP,
                        false,
                        numeric,
                    )))
                }
            }
            TriangleFeatureType::Vertex1 => {
                if flags & FLAG_TRIANGLE_VERTEX_1_DISABLE != 0 && !is_object {
                    Some(0)
                } else {
                    Some(u32::from(vertex_edge_cos_tests(
                        triangle,
                        contact_normal,
                        triangle_normal,
                        EDGE_01_FIXUP,
                        EDGE_12_FIXUP,
                        false,
                        numeric,
                    )))
                }
            }
            TriangleFeatureType::Vertex2 => {
                if flags & FLAG_TRIANGLE_VERTEX_2_DISABLE != 0 && !is_object {
                    Some(0)
                } else {
                    Some(u32::from(vertex_edge_cos_tests(
                        triangle,
                        contact_normal,
                        triangle_normal,
                        EDGE_12_FIXUP,
                        EDGE_20_FIXUP,
                        false,
                        numeric,
                    )))
                }
            }
        };
    }

    match region {
        TriangleFeatureType::Face => Some(1),
        TriangleFeatureType::Edge01 | TriangleFeatureType::Edge12 | TriangleFeatureType::Edge20 => {
            let edge = match region {
                TriangleFeatureType::Edge01 => EDGE_01_FIXUP,
                TriangleFeatureType::Edge12 => EDGE_12_FIXUP,
                TriangleFeatureType::Edge20 => EDGE_20_FIXUP,
                _ => unreachable!(),
            };
            let comparison_alignment = if flags & edge.convex_mask != 0 {
                alignment
            } else {
                bit_negate_scalar(alignment)
            };
            Some(u32::from(
                comparison_alignment >= triangle.edge_data[edge.cosine_index],
            ))
        }
        TriangleFeatureType::Vertex0 => {
            if flags & FLAG_TRIANGLE_VERTEX_0_DISABLE != 0 {
                Some(0)
            } else {
                Some(u32::from(vertex_edge_cos_tests(
                    triangle,
                    contact_normal,
                    triangle_normal,
                    EDGE_20_FIXUP,
                    EDGE_01_FIXUP,
                    true,
                    numeric,
                )))
            }
        }
        TriangleFeatureType::Vertex1 => {
            if flags & FLAG_TRIANGLE_VERTEX_1_DISABLE != 0 {
                Some(0)
            } else {
                Some(u32::from(vertex_edge_cos_tests(
                    triangle,
                    contact_normal,
                    triangle_normal,
                    EDGE_01_FIXUP,
                    EDGE_12_FIXUP,
                    true,
                    numeric,
                )))
            }
        }
        TriangleFeatureType::Vertex2 => {
            if flags & FLAG_TRIANGLE_VERTEX_2_DISABLE != 0 {
                Some(0)
            } else {
                Some(u32::from(vertex_edge_cos_tests(
                    triangle,
                    contact_normal,
                    triangle_normal,
                    EDGE_12_FIXUP,
                    EDGE_20_FIXUP,
                    true,
                    numeric,
                )))
            }
        }
    }
}

/// Owning acceptance seam for the mandatory triangle fix-up. A rejected
/// candidate is consumed and cannot escape into a ground-contact producer.
#[allow(dead_code)]
fn accept_triangle_fixup_candidate_with_numeric<N: TriangleFixupNumeric>(
    triangle: &GpPrimitive,
    mut candidate: PreTriangleFixupCandidate,
    reverse: bool,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<PreTriangleFixupCandidate> {
    let accepted = fix_up_triangle_result_with_numeric(
        triangle,
        &mut candidate.result_normal,
        reverse,
        &mut candidate.prism,
        bend_normal_threshold,
        convexity_epsilon,
        is_object,
        numeric,
    )?;
    (accepted != 0).then_some(candidate)
}

/// Serializes the accepted post-fixup candidate exactly as the tail at
/// `0x827cd104..0x827cd1bc`: tags retain first/second order, the published
/// normal is the bitwise-negated result normal, and each core point is moved
/// outward by its own primitive's fatness.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
// Shared packing: Sk8ComputeContactPoints 827cd140 and ComputeContactPoints
// 82ad60bc both negate the result normal and offset points by GP fatness.
fn serialize_sk8_contact_points(
    first: &GpPrimitive,
    first_tags: GpInstanceTags,
    second: &GpPrimitive,
    second_tags: GpInstanceTags,
    candidate: &PreTriangleFixupCandidate,
) -> Option<Sk8ContactPoints> {
    let point_count = usize::try_from(candidate.prism.point_count).ok()?;
    if point_count > candidate.prism.points_on_first.len() {
        return None;
    }

    let mut point_pairs = [Sk8ContactPointPair::default(); 16];
    let first_fatness = scale(candidate.result_normal, first.fatness);
    let second_fatness = scale(candidate.result_normal, second.fatness);
    for (index, pair) in point_pairs[..point_count].iter_mut().enumerate() {
        *pair = Sk8ContactPointPair {
            first: add(candidate.prism.points_on_first[index], first_fatness),
            second: subtract(candidate.prism.points_on_second[index], second_fatness),
        };
    }

    Some(Sk8ContactPoints {
        first: first_tags,
        second: second_tags,
        point_count,
        normal: bit_negate(candidate.result_normal),
        point_pairs,
    })
}

/// Complete accepted-value path of `rw::collision::Sk8ComputeContactPoints`
/// at `0x827cce40..0x827cd1e0` for represented primitive types.
///
/// `None` covers the native zero-count routes and typed unsupported/malformed
/// host inputs. A second triangle always takes the mandatory reverse fix-up.
/// `draw_contacts` is retained as an input because it is present in the PDB
/// signature, although this retail body never reads it.
#[allow(clippy::too_many_arguments)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
pub(super) fn try_sk8_compute_contact_points(
    first: &GpPrimitive,
    first_tags: GpInstanceTags,
    second: &GpPrimitive,
    second_tags: GpInstanceTags,
    padding: f32,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    _draw_contacts: bool,
) -> Option<Sk8ContactPoints> {
    let mut numeric = HostTriangleFixupNumeric;
    let computation = try_sk8_compute_contact_points_with_numeric(
        first,
        first_tags,
        second,
        second_tags,
        padding,
        bend_normal_threshold,
        convexity_epsilon,
        is_object,
        &mut numeric,
    )?;
    (computation.returned_count != 0).then_some(computation.scratch)
}

/// `rw::collision::ComputeContactPoints` at `0x82ad5d80`, used by the
/// assembly/assembly job. Unlike Sk8ComputeContactPoints, this checks BOTH
/// triangle operands and adds the two fatness values before adding padding.
/// Uses the existing shared GP, feature-prism, triangle-fixup and packing code.
#[allow(clippy::too_many_arguments)]
pub(super) fn try_compute_contact_points(
    first: &GpPrimitive,
    first_tags: GpInstanceTags,
    second: &GpPrimitive,
    second_tags: GpInstanceTags,
    padding: f32,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
) -> Option<Sk8ContactPoints> {
    let mut numeric = HostTriangleFixupNumeric;
    // Two distinct fadds at 82ad5e0c/24. Do not reassociate to the Sk8 order.
    let contact_limit = (second.fatness + first.fatness) + padding;
    let mut candidate = match try_pre_triangle_fixup_candidate_with_limit(
        first,
        second,
        contact_limit,
        &mut numeric,
    )? {
        PreTriangleFixupOutcome::Separated => return None,
        PreTriangleFixupOutcome::Candidate(candidate) => candidate,
    };
    // 82ad6040..60b8: first rev=false, then second rev=true, with the same
    // mutable normal/prism. Either rejected triangle returns before packing.
    for (triangle, reverse) in [(first, false), (second, true)] {
        if triangle.primitive_type == GpPrimitiveType::Triangle
            && fix_up_triangle_result_with_numeric(
                triangle,
                &mut candidate.result_normal,
                reverse,
                &mut candidate.prism,
                bend_normal_threshold,
                convexity_epsilon,
                is_object,
                &mut numeric,
            )? == 0
        {
            return None;
        }
    }
    let contacts =
        serialize_sk8_contact_points(first, first_tags, second, second_tags, &candidate)?;
    (contacts.point_count != 0).then_some(contacts)
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for the Assembly/ground Contact owner")
)]
fn try_sk8_compute_contact_points_with_numeric<N: TriangleFixupNumeric>(
    first: &GpPrimitive,
    first_tags: GpInstanceTags,
    second: &GpPrimitive,
    second_tags: GpInstanceTags,
    padding: f32,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<Sk8ContactPointsComputation> {
    let candidate =
        match try_sk8_pre_triangle_fixup_candidate_with_backend(first, second, padding, numeric)? {
            PreTriangleFixupOutcome::Separated => return None,
            PreTriangleFixupOutcome::Candidate(candidate) => candidate,
        };
    finalize_sk8_contact_points_with_numeric(
        first,
        first_tags,
        second,
        second_tags,
        candidate,
        bend_normal_threshold,
        convexity_epsilon,
        is_object,
        numeric,
    )
}

#[allow(clippy::too_many_arguments)]
fn finalize_sk8_contact_points_with_numeric<N: TriangleFixupNumeric>(
    first: &GpPrimitive,
    first_tags: GpInstanceTags,
    second: &GpPrimitive,
    second_tags: GpInstanceTags,
    mut candidate: PreTriangleFixupCandidate,
    bend_normal_threshold: f32,
    convexity_epsilon: f32,
    is_object: bool,
    numeric: &mut N,
) -> Option<Sk8ContactPointsComputation> {
    let rejected = if second.primitive_type == GpPrimitiveType::Triangle {
        fix_up_triangle_result_with_numeric(
            second,
            &mut candidate.result_normal,
            true,
            &mut candidate.prism,
            bend_normal_threshold,
            convexity_epsilon,
            is_object,
            numeric,
        )? == 0
    } else {
        false
    };
    let scratch = serialize_sk8_contact_points(first, first_tags, second, second_tags, &candidate)?;
    Some(Sk8ContactPointsComputation {
        returned_count: if rejected { 0 } else { scratch.point_count },
        scratch,
    })
}

fn orient_pre_triangle_fixup_normal(
    separating: SeparatingDirection,
    prism_normal: [f32; 3],
    first_interval: ProjectionInterval,
    second_interval: ProjectionInterval,
) -> [f32; 3] {
    let first_before_second = first_interval.minimum - second_interval.maximum;
    let second_before_first = second_interval.minimum - first_interval.maximum;
    let largest_gap = if first_before_second > second_before_first {
        first_before_second
    } else {
        second_before_first
    };
    if largest_gap >= separating.separation {
        if first_before_second > second_before_first {
            negate(prism_normal)
        } else {
            prism_normal
        }
    } else {
        separating.direction
    }
}

#[allow(dead_code)]
fn try_sk8_pre_triangle_fixup_candidate_with_numeric<R, S>(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
    mut sum_three_products: R,
    reciprocal_sqrt_seed: S,
) -> Option<PreTriangleFixupOutcome>
where
    R: FnMut([f32; 3]) -> f32,
    S: FnMut(f32) -> f32,
{
    let mut numeric = InjectedTriangleFixupNumeric {
        sum_three_products: |left: [f32; 3], right: [f32; 3]| {
            let _ = right;
            sum_three_products(left)
        },
        reciprocal_sqrt_seed,
    };
    try_sk8_pre_triangle_fixup_candidate_with_backend(first, second, padding, &mut numeric)
}

fn try_sk8_pre_triangle_fixup_candidate_with_backend<N: TriangleFixupNumeric>(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
    numeric: &mut N,
) -> Option<PreTriangleFixupOutcome> {
    let contact_limit = (second.fatness + padding) + first.fatness;
    try_pre_triangle_fixup_candidate_with_limit(first, second, contact_limit, numeric)
}

// The GP feature/prism and normal-selection path is shared by 827cce40 and
// 82ad5d80; each entry point retains its own rounded contact-limit arithmetic.
fn try_pre_triangle_fixup_candidate_with_limit<N: TriangleFixupNumeric>(
    first: &GpPrimitive,
    second: &GpPrimitive,
    contact_limit: f32,
    numeric: &mut N,
) -> Option<PreTriangleFixupOutcome> {
    let separating = complete_separating_direction(first, second)?;
    if separating.separation > contact_limit {
        return Some(PreTriangleFixupOutcome::Separated);
    }
    let mut first_feature = first.maximum_feature_for_overlap(true, separating.direction)?;
    let mut second_feature =
        second.maximum_feature_for_overlap(false, negate(separating.direction))?;
    let mut prism = find_feature_intersection_prism(
        &mut first_feature,
        &mut second_feature,
        separating.direction,
    )?;
    let mut result_normal = separating.direction;

    if prism.point_count == 1 {
        let point_delta = subtract(prism.points_on_second[0], prism.points_on_first[0]);
        // `vmsum3fp128` and `vrsqrtefp128` execute even for exact zero.
        // Keeping both results injected makes the unproven Xenon reduction
        // and estimate bits explicit numeric boundaries rather than silently
        // claiming host arithmetic parity.
        let squared_length = numeric.sum_three_products(point_delta, point_delta);
        let seed = numeric.reciprocal_sqrt_seed(squared_length);
        let reciprocal_length = refine_reciprocal_sqrt_twice(squared_length, seed);
        let point_distance = if squared_length == 0.0 {
            0.0
        } else {
            squared_length * reciprocal_length
        };
        if point_distance > f32::EPSILON {
            prism.normal = scale(point_delta, reciprocal_length);
            prism.normal_override = 1;
        }
    }

    if prism.normal_override != 0 {
        let first_interval = first.projection_interval(prism.normal);
        let second_interval = second.projection_interval(prism.normal);
        result_normal = orient_pre_triangle_fixup_normal(
            separating,
            prism.normal,
            first_interval,
            second_interval,
        );
    }

    Some(PreTriangleFixupOutcome::Candidate(
        PreTriangleFixupCandidate {
            separating,
            prism,
            result_normal,
        },
    ))
}

fn try_sk8_pre_triangle_fixup_candidate(
    first: &GpPrimitive,
    second: &GpPrimitive,
    padding: f32,
) -> Option<PreTriangleFixupOutcome> {
    let mut numeric = HostTriangleFixupNumeric;
    try_sk8_pre_triangle_fixup_candidate_with_backend(first, second, padding, &mut numeric)
}

fn append_face_normals_reverse(candidates: &mut Vec<[f32; 3]>, primitive: &GpPrimitive) {
    candidates.extend(
        primitive.face_normals[..usize::from(primitive.face_normal_count)]
            .iter()
            .rev()
            .copied(),
    );
}

fn append_center_delta_perpendicular(
    candidates: &mut Vec<[f32; 3]>,
    first: &GpPrimitive,
    second: &GpPrimitive,
    edge: [f32; 3],
) {
    let delta = subtract(second.position(), first.position());
    let perpendicular = cross(edge, cross(delta, edge));
    append_normalized_above(candidates, perpendicular, DIRECTION_MINIMUM_LENGTH_SQUARED);
}

fn append_normalized_above(
    candidates: &mut Vec<[f32; 3]>,
    value: [f32; 3],
    minimum_length_squared: f32,
) {
    let squared = length_squared(value);
    if squared > minimum_length_squared {
        let inverse_length = squared.sqrt().recip();
        candidates.push(value.map(|component| component * inverse_length));
    }
}

fn point_interval(point: [f32; 3], direction: [f32; 3]) -> ProjectionInterval {
    let projection = dot(point, direction);
    ProjectionInterval {
        minimum: projection,
        maximum: projection,
    }
}

fn compose_affine(outer: [f32; 16], inner: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for column in 0..3 {
        for row in 0..3 {
            result[column * 4 + row] = (0..3)
                .map(|axis| outer[axis * 4 + row] * inner[column * 4 + axis])
                .sum();
        }
    }
    result[12..15].copy_from_slice(&transform_point(outer, affine_translation(inner)));
    result[15] = 1.0;
    result
}

fn affine_axes(transform: [f32; 16]) -> [[f32; 3]; 3] {
    std::array::from_fn(|column| affine_column(transform, column))
}

fn affine_column(transform: [f32; 16], column: usize) -> [f32; 3] {
    std::array::from_fn(|row| transform[column * 4 + row])
}

fn affine_translation(transform: [f32; 16]) -> [f32; 3] {
    affine_column(transform, 3)
}

fn transform_point(transform: [f32; 16], point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        transform[8 + row].mul_add(
            point[2],
            transform[4 + row].mul_add(
                point[1],
                transform[row].mul_add(point[0], transform[12 + row]),
            ),
        )
    })
}

fn transform_direction(transform: [f32; 16], direction: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        transform[8 + row].mul_add(
            direction[2],
            transform[4 + row].mul_add(direction[1], transform[row] * direction[0]),
        )
    })
}

fn subtract(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| left[axis] + right[axis])
}

fn scale(value: [f32; 3], scalar: f32) -> [f32; 3] {
    value.map(|component| component * scalar)
}

fn negate(value: [f32; 3]) -> [f32; 3] {
    value.map(|component| -component)
}

fn dot(left: [f32; 3], right: [f32; 3]) -> f32 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn cross(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn length_squared(value: [f32; 3]) -> f32 {
    dot(value, value)
}

fn normalize_or_zero(value: [f32; 3]) -> [f32; 3] {
    let squared = length_squared(value);
    if squared == 0.0 {
        [0.0; 3]
    } else {
        let inverse_length = squared.sqrt().recip();
        value.map(|component| component * inverse_length)
    }
}

fn normalize_above_or_zero(value: [f32; 3], minimum_squared_length: f32) -> [f32; 3] {
    let squared = length_squared(value);
    if squared > minimum_squared_length {
        scale(value, squared.sqrt().recip())
    } else {
        [0.0; 3]
    }
}

fn cylinder_local_direction_to_world(
    axis_x: [f32; 3],
    axis_y: [f32; 3],
    axis_z: [f32; 3],
    direction: [f32; 3],
) -> [f32; 3] {
    std::array::from_fn(|component| {
        axis_z[component].mul_add(
            direction[2],
            axis_y[component].mul_add(direction[1], axis_x[component] * direction[0]),
        )
    })
}

fn cylinder_local_point_to_world(
    center: [f32; 3],
    axis_x: [f32; 3],
    axis_y: [f32; 3],
    axis_z: [f32; 3],
    point: [f32; 3],
) -> [f32; 3] {
    std::array::from_fn(|component| {
        axis_z[component].mul_add(
            point[2],
            axis_y[component].mul_add(
                point[1],
                axis_x[component].mul_add(point[0], center[component]),
            ),
        )
    })
}

fn select_projected_box_point(
    center: [f32; 3],
    axes: &[[f32; 3]; 3],
    dimensions: &[f32; 3],
    direction: [f32; 3],
    signs: &[[f32; 2]],
    selected_axes: [usize; 2],
) -> [f32; 3] {
    let point_for_signs = |signs: [f32; 2]| {
        let first = scale(
            axes[selected_axes[0]],
            dimensions[selected_axes[0]] * signs[0],
        );
        let second = scale(
            axes[selected_axes[1]],
            dimensions[selected_axes[1]] * signs[1],
        );
        add(add(center, first), second)
    };
    let mut best_point = point_for_signs(signs[0]);
    let mut best_projection = dot(best_point, direction);
    for candidate_signs in &signs[1..] {
        let candidate = point_for_signs(*candidate_signs);
        let projection = dot(candidate, direction);
        if projection > best_projection {
            best_point = candidate;
            best_projection = projection;
        }
    }
    best_point
}

fn select_projected_box_vertex(
    center: [f32; 3],
    axes: &[[f32; 3]; 3],
    dimensions: &[f32; 3],
    direction: [f32; 3],
) -> [f32; 3] {
    let point_for_signs = |signs: [f32; 3]| {
        let mut point = center;
        for axis in 0..3 {
            point = add(point, scale(axes[axis], dimensions[axis] * signs[axis]));
        }
        point
    };
    let mut best_point = point_for_signs(BOX_VERTEX_SIGNS[0]);
    let mut best_projection = dot(best_point, direction);
    for signs in &BOX_VERTEX_SIGNS[1..] {
        let candidate = point_for_signs(*signs);
        let projection = dot(candidate, direction);
        if projection > best_projection {
            best_point = candidate;
            best_projection = projection;
        }
    }
    best_point
}

fn stored_feature_edge(start: [f32; 3], direction: [f32; 3], length: f32) -> GpFeatureEdge {
    GpFeatureEdge {
        start,
        direction,
        plane_normal: [0.0; 3],
        length,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    fn volume(volume_type: VolumeType) -> Volume {
        Volume {
            transform: IDENTITY,
            vtable_offset: volume_type as u32,
            shape_words: [0; 3],
            radius: 0.0,
            group_id: 0,
            surface_id: 0,
            flags: 0,
        }
    }

    struct QueuedTriangleFixupNumeric {
        sums: std::collections::VecDeque<f32>,
        seeds: std::collections::VecDeque<f32>,
        events: Vec<&'static str>,
    }

    impl QueuedTriangleFixupNumeric {
        fn new(sums: impl IntoIterator<Item = f32>, seeds: impl IntoIterator<Item = f32>) -> Self {
            Self {
                sums: sums.into_iter().collect(),
                seeds: seeds.into_iter().collect(),
                events: Vec::new(),
            }
        }

        fn assert_exhausted(&self) {
            assert!(
                self.sums.is_empty(),
                "unconsumed reductions: {:?}",
                self.sums
            );
            assert!(
                self.seeds.is_empty(),
                "unconsumed estimates: {:?}",
                self.seeds
            );
        }
    }

    impl TriangleFixupNumeric for QueuedTriangleFixupNumeric {
        fn sum_three_products(&mut self, _left: [f32; 3], _right: [f32; 3]) -> f32 {
            self.events.push("sum");
            self.sums.pop_front().expect("unexpected reduction")
        }

        fn reciprocal_sqrt_seed(&mut self, _value: f32) -> f32 {
            self.events.push("seed");
            self.seeds.pop_front().expect("unexpected estimate")
        }
    }

    struct RecordingHostTriangleFixupNumeric {
        events: Vec<&'static str>,
    }

    impl TriangleFixupNumeric for RecordingHostTriangleFixupNumeric {
        fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
            self.events.push("sum");
            dot(left, right)
        }

        fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
            self.events.push("seed");
            value.sqrt().recip()
        }
    }

    fn fixup_triangle(flags: u32, edge_data: [f32; 3]) -> GpPrimitive {
        let mut triangle =
            GpPrimitive::from_triangle_points([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        triangle.flags = flags;
        triangle.edge_data = edge_data;
        triangle
    }

    fn feature_projections(region: TriangleFeatureType) -> [f32; 3] {
        match region {
            TriangleFeatureType::Face => [0.0, 0.0, 0.0],
            TriangleFeatureType::Edge01 => [-0.02, 0.0, 0.01],
            TriangleFeatureType::Edge12 => [0.0, 0.01, -0.02],
            TriangleFeatureType::Edge20 => [0.01, -0.02, 0.0],
            TriangleFeatureType::Vertex0 => [-1.0, 0.0, 1.0],
            TriangleFeatureType::Vertex1 => [0.0, 1.0, -1.0],
            TriangleFeatureType::Vertex2 => [1.0, -1.0, 0.0],
        }
    }

    fn run_fixup_for_region(
        triangle: &GpPrimitive,
        region: TriangleFeatureType,
        alignment: f32,
        additional_sums: impl IntoIterator<Item = f32>,
    ) -> (Option<u32>, QueuedTriangleFixupNumeric) {
        let projections = feature_projections(region);
        let mut sums = vec![alignment, 1.0];
        sums.extend(projections);
        sums.extend(additional_sums);
        let mut numeric = QueuedTriangleFixupNumeric::new(sums, [1.0]);
        let mut result_normal = [0.0, 0.0, 1.0];
        let mut prism = FeatureIntersectionPrism::default();
        let result = fix_up_triangle_result_with_numeric(
            triangle,
            &mut result_normal,
            false,
            &mut prism,
            0.75,
            0.0,
            false,
            &mut numeric,
        );
        (result, numeric)
    }

    #[test]
    fn creates_box_gp_instance_with_reference_axis_and_dimension_storage() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        box_volume.radius = 0.25;
        let mut parent = IDENTITY;
        parent[0] = 0.0;
        parent[1] = 1.0;
        parent[4] = -1.0;
        parent[5] = 0.0;
        parent[12] = 10.0;

        let primitive = GpPrimitive::from_volume(&box_volume, Some(parent)).unwrap();
        assert_eq!(primitive.position(), [10.0, 0.0, 0.0]);
        assert_eq!(
            primitive.face_normals,
            [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]
        );
        assert_eq!(primitive.edge_directions, primitive.face_normals);
        assert_eq!(primitive.dimensions, [1.0, 2.0, 3.0]);
        assert_eq!(primitive.fatness, 0.25);

        let candidates = generic_separating_direction_candidates(&primitive, &primitive);
        assert_eq!(candidates[0], [0.0, 0.0, 1.0]);
        assert_eq!(candidates[1], [-1.0, 0.0, 0.0]);
        assert_eq!(candidates[2], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn box_maximum_face_preserves_corner_order_winding_and_y_axis_correction() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        let primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();

        let positive_z = primitive
            .box_maximum_feature(true, [0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(positive_z.edge_count, 4);
        assert_eq!(positive_z.own_normal, [0.0, 0.0, 1.0]);
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|index| positive_z.edges[index].start),
            [
                [1.0, 2.0, 3.0],
                [-1.0, 2.0, 3.0],
                [-1.0, -2.0, 3.0],
                [1.0, -2.0, 3.0],
            ]
        );
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|index| positive_z.edges[index].direction),
            [
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ]
        );
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|index| positive_z.edges[index].plane_normal),
            [
                [0.0, 1.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [1.0, 0.0, 0.0],
            ]
        );
        assert_eq!(
            std::array::from_fn::<_, 4, _>(|index| positive_z.edges[index].length),
            [2.0, 4.0, 2.0, 4.0]
        );

        let positive_y = primitive
            .box_maximum_feature(true, [0.0, 1.0, 0.0])
            .unwrap();
        assert_eq!(positive_y.own_normal, [0.0, 1.0, 0.0]);
        assert_eq!(positive_y.edges[0].start, [1.0, 2.0, 3.0]);
        assert_eq!(positive_y.edges[0].direction, [0.0, 0.0, -1.0]);
        assert_eq!(positive_y.edges[1].start, [1.0, 2.0, -3.0]);
        assert_eq!(positive_y.edges[1].direction, [-1.0, 0.0, 0.0]);
    }

    #[test]
    fn box_maximum_edge_vertex_and_alignment_boundary_follow_reference_tables() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        let primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();

        let edge = primitive
            .box_maximum_feature(true, [1.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(edge.edge_count, 1);
        assert_eq!(edge.edges[0].start, [1.0, -2.0, 3.0]);
        assert_eq!(edge.edges[0].direction, [0.0, 1.0, 0.0]);
        assert_eq!(edge.edges[0].length, 4.0);

        let vertex = primitive
            .box_maximum_feature(true, [1.0, 1.0, 1.0])
            .unwrap();
        assert_eq!(vertex.edge_count, 0);
        assert_eq!(vertex.point, [1.0, 2.0, 3.0]);
        let tied_vertex = primitive.box_maximum_feature(true, [0.0; 3]).unwrap();
        assert_eq!(tied_vertex.point, [-1.0, 2.0, 3.0]);

        assert_eq!(
            primitive
                .box_maximum_feature(true, [0.2, 0.0, 0.0])
                .unwrap()
                .edge_count,
            4
        );
        assert_eq!(
            primitive
                .box_maximum_feature(true, [0.199_999_99, 0.0, 0.0])
                .unwrap()
                .edge_count,
            0
        );
    }

    #[test]
    fn projects_capsule_box_and_cylinder_core_shapes_without_fatness() {
        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.shape_words[0] = 2.0_f32.to_bits();
        capsule_volume.radius = 0.5;
        let capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            capsule.projection_interval([0.0, 0.0, 1.0]),
            ProjectionInterval {
                minimum: -2.0,
                maximum: 2.0
            }
        );
        assert_eq!(
            capsule.projection_interval([1.0, 0.0, 0.0]),
            ProjectionInterval {
                minimum: 0.0,
                maximum: 0.0
            }
        );

        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        let box_primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();
        assert_eq!(
            box_primitive.projection_interval([1.0, 0.0, 0.0]),
            ProjectionInterval {
                minimum: -1.0,
                maximum: 1.0
            }
        );

        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        assert_eq!(
            cylinder.face_normals,
            [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]]
        );
        assert_eq!(
            cylinder.projection_interval([1.0, 0.0, 0.0]),
            ProjectionInterval {
                minimum: -3.0,
                maximum: 3.0
            }
        );
        assert_eq!(
            cylinder.projection_interval([0.0, 0.0, 1.0]),
            ProjectionInterval {
                minimum: -2.0,
                maximum: 2.0
            }
        );
    }

    #[test]
    fn cylinder_cap_feature_preserves_octagon_start_order_and_winding() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let diagonal = 3.0 * SQRT_HALF;

        let positive = cylinder
            .cylinder_maximum_feature(true, [0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(positive.edge_count, 8);
        assert_eq!(positive.own_normal, [0.0, 0.0, 1.0]);
        assert_eq!(positive.point, [0.0; 3]);
        assert_eq!(
            std::array::from_fn::<_, 8, _>(|index| positive.edges[index].start),
            [
                [-diagonal, diagonal, 2.0],
                [-3.0, 0.0, 2.0],
                [-diagonal, -diagonal, 2.0],
                [0.0, -3.0, 2.0],
                [diagonal, -diagonal, 2.0],
                [3.0, 0.0, 2.0],
                [diagonal, diagonal, 2.0],
                [0.0, 3.0, 2.0],
            ]
        );

        let negative = cylinder
            .cylinder_maximum_feature(true, [0.0, 0.0, -1.0])
            .unwrap();
        assert_eq!(negative.own_normal, [0.0, 0.0, -1.0]);
        assert_eq!(negative.edges[0].start, [0.0, 3.0, -2.0]);
        assert_eq!(negative.edges[1].start, [diagonal, diagonal, -2.0]);
        assert!(positive.edges[0].plane_normal[0] < 0.0);
        assert!(negative.edges[0].plane_normal[0] > 0.0);
    }

    #[test]
    fn cylinder_cap_uses_strict_axial_alignment_threshold() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let threshold = CYLINDER_CAP_ALIGNMENT_THRESHOLD;
        let radial = (1.0 - threshold * threshold).sqrt();
        assert_eq!(
            cylinder
                .cylinder_maximum_feature(true, [radial, 0.0, threshold])
                .unwrap()
                .edge_count,
            0
        );

        let above = f32::from_bits(threshold.to_bits() + 1);
        let radial = (1.0 - above * above).sqrt();
        assert_eq!(
            cylinder
                .cylinder_maximum_feature(true, [radial, 0.0, above])
                .unwrap()
                .edge_count,
            8
        );
    }

    #[test]
    fn cylinder_side_edge_keeps_full_direction_normalization_at_boundary() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let axial = CYLINDER_EDGE_ALIGNMENT_THRESHOLD;
        let radial = (1.0 - axial * axial).sqrt();

        let edge = cylinder
            .cylinder_maximum_feature(true, [radial, 0.0, axial])
            .unwrap();
        assert_eq!(edge.edge_count, 1);
        assert_eq!(edge.own_normal, [radial, 0.0, axial]);
        assert!((edge.edges[0].start[0] - radial * 3.0).abs() < 1.0e-6);
        assert_eq!(edge.edges[0].start[1..], [0.0, 2.0]);
        assert_eq!(edge.edges[0].direction, [0.0, 0.0, -1.0]);
        assert_eq!(edge.edges[0].length, 4.0);
        assert_eq!(edge.point, [0.0; 3]);

        let above = f32::from_bits(axial.to_bits() + 1);
        let radial = (1.0 - above * above).sqrt();
        let point = cylinder
            .cylinder_maximum_feature(true, [radial, 0.0, above])
            .unwrap();
        assert_eq!(point.edge_count, 0);
        assert!((point.point[0] - 3.0).abs() < 1.0e-6);
        assert_eq!(point.point[1..], [0.0, 2.0]);
    }

    #[test]
    fn cylinder_rim_point_transforms_through_stored_local_basis() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let mut parent = IDENTITY;
        parent[0] = 0.0;
        parent[1] = 1.0;
        parent[4] = -1.0;
        parent[5] = 0.0;
        parent[12..15].copy_from_slice(&[10.0, 20.0, 30.0]);
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, Some(parent)).unwrap();
        let radial = 0.75_f32.sqrt();

        let point = cylinder
            .cylinder_maximum_feature(true, [0.0, radial, 0.5])
            .unwrap();
        assert_eq!(point.edge_count, 0);
        assert_eq!(point.point, [10.0, 23.0, 32.0]);
        assert_eq!(point.own_normal, [0.0, radial, 0.5]);
    }

    #[test]
    fn cylinder_sphere_separator_keeps_axis_order_and_radial_orientation() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let mut sphere_volume = volume(VolumeType::Sphere);
        sphere_volume.transform[12] = 5.0;
        let sphere = GpPrimitive::from_volume(&sphere_volume, None).unwrap();

        assert_eq!(
            cylinder_sphere_separating_direction(&cylinder, &sphere),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 2.0,
            })
        );

        sphere_volume.transform[12] = 0.0;
        sphere_volume.transform[14] = 5.0;
        let axial_sphere = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        assert_eq!(
            cylinder_sphere_separating_direction(&cylinder, &axial_sphere),
            Some(SeparatingDirection {
                direction: [0.0, 0.0, 1.0],
                separation: 3.0,
            })
        );
    }

    #[test]
    fn specialized_candidate_chooser_preserves_both_strict_ties() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [1.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();

        assert_eq!(
            best_separating_direction_from_candidates(
                &cylinder,
                &sphere,
                &[[0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]],
            ),
            Some(SeparatingDirection {
                direction: [0.0, 0.0, 1.0],
                separation: -1.0,
            })
        );
        assert_eq!(
            best_separating_direction_from_candidates(&cylinder, &sphere, &[]),
            None
        );

        let mut moved_sphere_volume = volume(VolumeType::Sphere);
        moved_sphere_volume.transform[12] = 3.0;
        let moved_sphere = GpPrimitive::from_volume(&moved_sphere_volume, None).unwrap();
        assert_eq!(
            best_separating_direction_from_candidates(
                &cylinder,
                &moved_sphere,
                &[[-1.0, 0.0, 0.0]],
            ),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 2.0,
            })
        );
    }

    #[test]
    fn line_edge_candidate_keeps_cross_axis_at_exact_segment_endpoints() {
        let line_base = [0.0; 3];
        let line_direction = [1.0, 0.0, 0.0];
        let edge_direction = [0.0, 1.0, 0.0];
        assert_eq!(
            line_edge_separating_candidate(
                line_base,
                line_direction,
                [0.0, -1.0, 2.0],
                edge_direction,
                0.0,
                1.0,
            ),
            [0.0, 0.0, 1.0]
        );
        assert_eq!(
            line_edge_separating_candidate(
                line_base,
                line_direction,
                [0.0, 1.0, 2.0],
                edge_direction,
                0.0,
                1.0,
            ),
            [0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn line_edge_candidate_clamps_only_strictly_outside_finite_edge() {
        let line_direction = [1.0, 0.0, 0.0];
        let edge_direction = [0.0, 1.0, 0.0];
        assert_eq!(
            line_edge_separating_candidate(
                [0.0; 3],
                line_direction,
                [0.0, 2.0, 0.0],
                edge_direction,
                0.0,
                1.0,
            ),
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            line_edge_separating_candidate(
                [0.0; 3],
                line_direction,
                [0.0, -2.0, 0.0],
                edge_direction,
                0.0,
                1.0,
            ),
            [0.0, -1.0, 0.0]
        );

        // The executable's endpoint double cross uses the absolute selected
        // endpoint, not endpoint-line_base. Preserve that observable detail.
        assert_eq!(
            line_edge_separating_candidate(
                [0.0, 10.0, 0.0],
                line_direction,
                [0.0, 8.0, 0.0],
                edge_direction,
                0.0,
                1.0,
            ),
            [0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn line_edge_candidate_preserves_parallel_double_cross_and_fallback() {
        let line_direction = [1.0, 0.0, 0.0];
        assert_eq!(
            line_edge_separating_candidate(
                [0.0, 2.0, 0.0],
                line_direction,
                [0.0; 3],
                line_direction,
                0.0,
                1.0,
            ),
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            line_edge_separating_candidate(
                [0.0; 3],
                line_direction,
                [0.0; 3],
                line_direction,
                0.0,
                1.0,
            ),
            line_direction
        );
    }

    #[test]
    fn rim_edge_candidate_refines_a_plane_crossing_to_the_cylinder_side() {
        let candidate = rim_to_edge_separating_candidate(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            1.0,
            [2.0, 0.0, -2.0],
            [2.0, 0.0, 2.0],
        );
        assert_eq!(candidate, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn rim_edge_candidate_uses_the_interior_projection_for_a_parallel_edge() {
        let candidate = rim_to_edge_separating_candidate(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            1.0,
            [-2.0, 2.0, 0.0],
            [2.0, 2.0, 0.0],
        );
        assert!((candidate[0]).abs() < 1.0e-6);
        assert!((candidate[1] - 1.0).abs() < 1.0e-6);
        assert!((candidate[2]).abs() < 1.0e-6);
    }

    #[test]
    fn rim_edge_candidate_same_sign_seed_keeps_the_smaller_strict_metric() {
        let candidate = rim_to_edge_separating_candidate(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            1.0,
            [2.0, 2.0, 0.0],
            [2.0, 3.0, 0.0],
        );
        assert!((candidate[0] - SQRT_HALF).abs() < 1.0e-6);
        assert!((candidate[1] - SQRT_HALF).abs() < 1.0e-6);
        assert!((candidate[2]).abs() < 1.0e-6);
    }

    #[test]
    fn rim_edge_candidate_preserves_both_degenerate_fallback_levels() {
        let normal = [0.0, 0.0, 1.0];
        assert_eq!(
            rim_to_edge_separating_candidate(
                [0.0; 3],
                normal,
                1.0,
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 1.0],
            ),
            [-1.0, 0.0, 0.0]
        );
        assert_eq!(
            rim_to_edge_separating_candidate(
                [0.0; 3],
                normal,
                1.0,
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ),
            normal
        );
    }

    #[test]
    fn nearest_cylinder_rim_to_edge_preserves_strict_cap_selection() {
        let center = [0.0; 3];
        let axis = [0.0, 0.0, 1.0];
        let high_edge = [[-1.0, 0.0, 5.0], [1.0, 0.0, 5.0]];
        assert_eq!(
            nearest_cylinder_rim_to_edge_separating_candidate(
                center,
                axis,
                2.0,
                1.0,
                high_edge[0],
                high_edge[1],
            ),
            rim_to_edge_separating_candidate(
                [0.0, 0.0, 2.0],
                axis,
                1.0,
                high_edge[0],
                high_edge[1],
            )
        );

        let low_edge = [[-1.0, 0.0, -5.0], [1.0, 0.0, -5.0]];
        assert_eq!(
            nearest_cylinder_rim_to_edge_separating_candidate(
                center,
                axis,
                2.0,
                1.0,
                low_edge[0],
                low_edge[1],
            ),
            rim_to_edge_separating_candidate(
                [0.0, 0.0, -2.0],
                negate(axis),
                1.0,
                low_edge[0],
                low_edge[1],
            )
        );

        let tied_edge = [[-1.0, 3.0, 0.0], [1.0, 3.0, 0.0]];
        assert_eq!(
            nearest_cylinder_rim_to_edge_separating_candidate(
                center,
                axis,
                2.0,
                1.0,
                tied_edge[0],
                tied_edge[1],
            ),
            rim_to_edge_separating_candidate(
                [0.0, 0.0, -2.0],
                negate(axis),
                1.0,
                tied_edge[0],
                tied_edge[1],
            )
        );
    }

    #[test]
    fn rim_rim_candidate_preserves_radius_and_geometric_refusal_gates() {
        assert_eq!(
            rim_to_rim_separating_candidate(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                1.0e-32,
                [0.0; 3],
                [0.0, 1.0, 0.0],
                1.0e-32,
                [1.0, 0.0, 0.0],
            ),
            None
        );
        assert_eq!(
            rim_to_rim_separating_candidate(
                [0.0; 3],
                [0.0, 0.0, 1.0],
                1.0,
                [0.0; 3],
                [0.0, 1.0, 0.0],
                1.0,
                [1.0, 0.0, 0.0],
            ),
            None
        );
    }

    #[test]
    fn rim_rim_candidate_uses_the_near_coplanar_projection_path() {
        let diagonal = SQRT_HALF;
        let candidate = rim_to_rim_separating_candidate(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            1.0,
            [0.0, 3.0 * diagonal, -3.0 * diagonal],
            [0.0, diagonal, diagonal],
            1.0,
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        assert!(candidate[0].abs() < 1.0e-6);
        assert!((candidate[1] - 0.281_084_66).abs() < 1.0e-5);
        assert!((candidate[2] + 0.959_683).abs() < 1.0e-5);
    }

    #[test]
    fn rim_rim_candidate_keeps_the_general_signed_pair_selection() {
        let candidate = rim_to_rim_separating_candidate(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            1.0,
            [0.0, 3.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        assert!(candidate[0].abs() < 1.0e-6);
        assert!((candidate[1] - 0.894_427_2).abs() < 1.0e-6);
        assert!((candidate[2] + 0.447_213_6).abs() < 1.0e-6);
    }

    #[test]
    fn cylinder_rim_rim_candidates_preserve_cap_signs_and_append_order() {
        let mut first_volume = volume(VolumeType::Cylinder);
        first_volume.shape_words = [2.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();

        let mut second_volume = first_volume.clone();
        second_volume.transform[5] = 0.0;
        second_volume.transform[6] = -1.0;
        second_volume.transform[9] = 1.0;
        second_volume.transform[10] = 0.0;
        second_volume.transform[12] = 5.0;
        second_volume.transform[13] = 3.0;
        second_volume.transform[14] = 4.0;
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();

        let expected = [
            rim_to_rim_separating_candidate(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, 1.0],
                1.0,
                [5.0, 5.0, 4.0],
                [0.0, 1.0, 0.0],
                1.0,
                [1.0, 0.0, 0.0],
            ),
            rim_to_rim_separating_candidate(
                [0.0, 0.0, -2.0],
                [0.0, 0.0, -1.0],
                1.0,
                [5.0, 5.0, 4.0],
                [0.0, 1.0, 0.0],
                1.0,
                [-1.0, 0.0, 0.0],
            ),
            rim_to_rim_separating_candidate(
                [0.0, 0.0, 2.0],
                [0.0, 0.0, 1.0],
                1.0,
                [5.0, 1.0, 4.0],
                [0.0, -1.0, 0.0],
                1.0,
                [-1.0, 0.0, 0.0],
            ),
            rim_to_rim_separating_candidate(
                [0.0, 0.0, -2.0],
                [0.0, 0.0, -1.0],
                1.0,
                [5.0, 1.0, 4.0],
                [0.0, -1.0, 0.0],
                1.0,
                [1.0, 0.0, 0.0],
            ),
        ];
        assert!(expected.iter().all(Option::is_some));
        let expected = expected.into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(
            cylinder_rim_to_rim_candidates(&first, &second).unwrap(),
            expected
        );
        let all_candidates = cylinder_cylinder_separating_candidates(&first, &second).unwrap();
        assert_eq!(all_candidates.len(), 11);
        assert_eq!(all_candidates[0], first.edge_directions[0]);
        assert_eq!(all_candidates[1], second.edge_directions[0]);
        assert_eq!(&all_candidates[4..8], expected.as_slice());

        let parallel = GpPrimitive::from_volume(&first_volume, None).unwrap();
        assert_eq!(
            cylinder_rim_to_rim_candidates(&first, &parallel),
            Some(Vec::new())
        );
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(cylinder_rim_to_rim_candidates(&first, &sphere), None);
    }

    #[test]
    fn cylinder_sphere_separator_preserves_degenerate_fallback_basis() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [4.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(
            cylinder_sphere_separating_direction(&cylinder, &sphere),
            Some(SeparatingDirection {
                direction: [-1.0, 0.0, 0.0],
                separation: -1.0,
            })
        );

        let mut parent = IDENTITY;
        parent[0] = 0.0;
        parent[2] = -1.0;
        parent[8] = 1.0;
        parent[10] = 0.0;
        let rotated = GpPrimitive::from_volume(&cylinder_volume, Some(parent)).unwrap();
        assert_eq!(rotated.edge_directions[0], [1.0, 0.0, 0.0]);
        assert_eq!(
            cylinder_sphere_separating_direction(&rotated, &sphere),
            Some(SeparatingDirection {
                direction: [0.0, -1.0, 0.0],
                separation: -1.0,
            })
        );
    }

    #[test]
    fn cylinder_sphere_and_capsule_overlap_are_complete() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 3.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let mut sphere_volume = volume(VolumeType::Sphere);
        sphere_volume.radius = 1.0;
        sphere_volume.transform[12] = 4.0;
        let touching = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        assert_eq!(
            cylinder_sphere_overlap(&cylinder, &touching, 0.0),
            Some(true)
        );
        assert_eq!(
            sphere_primitive_overlap(&touching, &cylinder, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &touching, 0.0),
            Some(true)
        );

        sphere_volume.transform[12] = 4.001;
        let separated = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        assert_eq!(
            cylinder_sphere_overlap(&cylinder, &separated, 0.0),
            Some(false)
        );

        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.radius = 1.0;
        capsule_volume.shape_words[0] = 1.0_f32.to_bits();
        capsule_volume.transform[12] = 4.0;
        let capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &capsule, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&capsule, &cylinder, 0.0),
            Some(true)
        );
        capsule_volume.transform[12] = 4.001;
        let separated_capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &separated_capsule, 0.0),
            Some(false)
        );
    }

    #[test]
    fn cylinder_capsule_separator_preserves_axial_and_rim_candidate_order() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();

        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.shape_words[0] = 1.0_f32.to_bits();
        capsule_volume.transform[14] = 4.0;
        let axial = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            cylinder_capsule_separating_direction(&cylinder, &axial),
            Some(SeparatingDirection {
                direction: [0.0, 0.0, 1.0],
                separation: 2.0,
            })
        );

        capsule_volume.transform[14] = 0.0;
        capsule_volume.transform[12] = 3.0;
        let radial = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            cylinder_capsule_separating_direction(&cylinder, &radial),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 1.0,
            })
        );
    }

    #[test]
    fn cylinder_cylinder_separator_and_overlap_cover_parallel_core_edges() {
        let mut first_volume = volume(VolumeType::Cylinder);
        first_volume.shape_words = [2.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();

        let mut second_volume = first_volume.clone();
        second_volume.transform[12] = 3.0;
        let separated = GpPrimitive::from_volume(&second_volume, None).unwrap();
        assert_eq!(
            cylinder_cylinder_separating_direction(&first, &separated),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 1.0,
            })
        );
        assert_eq!(
            cylinder_cylinder_separating_direction(&separated, &first),
            Some(SeparatingDirection {
                direction: [-1.0, 0.0, 0.0],
                separation: 1.0,
            })
        );
        assert_eq!(
            generic_overlap_proven_subset(&first, &separated, 0.0),
            Some(false)
        );

        second_volume.transform[12] = 2.0;
        let touching = GpPrimitive::from_volume(&second_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &touching, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&touching, &first, 0.0),
            Some(true)
        );

        second_volume.transform[12] = 0.0;
        let coincident = GpPrimitive::from_volume(&second_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &coincident, 0.0),
            Some(true)
        );

        let mut perpendicular_volume = first_volume.clone();
        perpendicular_volume.transform[5] = 0.0;
        perpendicular_volume.transform[6] = -1.0;
        perpendicular_volume.transform[9] = 1.0;
        perpendicular_volume.transform[10] = 0.0;
        let perpendicular = GpPrimitive::from_volume(&perpendicular_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &perpendicular, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&perpendicular, &first, 0.0),
            Some(true)
        );
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(
            cylinder_cylinder_separating_direction(&first, &sphere),
            None
        );
    }

    #[test]
    fn cylinder_triangle_candidates_and_overlap_preserve_cyclic_edges() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();

        let mut triangle_volume = volume(VolumeType::Triangle);
        triangle_volume.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let triangle = GpPrimitive::from_volume(&triangle_volume, None).unwrap();
        let triangle_edges = [
            (triangle.points[0], triangle.points[2]),
            (triangle.points[2], triangle.points[1]),
            (triangle.points[1], triangle.points[0]),
        ];
        let candidates = cylinder_triangle_separating_candidates(&cylinder, &triangle).unwrap();
        assert_eq!(candidates.len(), 8);
        assert_eq!(candidates[0], cylinder.edge_directions[0]);
        assert_eq!(candidates[1], triangle.face_normals[0]);
        for (index, (edge_start, edge_end)) in triangle_edges.into_iter().enumerate() {
            assert_eq!(
                candidates[index + 2],
                nearest_cylinder_rim_to_edge_separating_candidate(
                    cylinder.position(),
                    cylinder.edge_directions[0],
                    cylinder.dimensions[0],
                    cylinder.dimensions[1],
                    edge_start,
                    edge_end,
                )
            );
            let edge_half_length = triangle.dimensions[index] * 0.5;
            assert_eq!(
                candidates[index + 5],
                line_edge_separating_candidate(
                    cylinder.position(),
                    cylinder.edge_directions[0],
                    edge_start,
                    triangle.edge_directions[index],
                    edge_half_length,
                    edge_half_length,
                )
            );
        }

        cylinder_volume.transform[14] = 3.0;
        let separated = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        assert_eq!(
            cylinder_triangle_separating_direction(&separated, &triangle),
            Some(SeparatingDirection {
                direction: [0.0, 0.0, -1.0],
                separation: 2.0,
            })
        );
        assert_eq!(
            generic_overlap_proven_subset(&separated, &triangle, 0.0),
            Some(false)
        );

        cylinder_volume.transform[14] = 1.0;
        let touching = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&touching, &triangle, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&triangle, &touching, 0.0),
            Some(true)
        );
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(
            cylinder_triangle_separating_candidates(&cylinder, &sphere),
            None
        );
    }

    #[test]
    fn cylinder_box_corner_selection_preserves_strict_positive_ties() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        let box_primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();

        assert_eq!(
            box_signed_extents_toward(&box_primitive, [0.0, 0.0, 0.0]),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            box_signed_extents_toward(&box_primitive, [-10.0, 10.0, -10.0]),
            [-1.0, 2.0, -3.0]
        );
        assert_eq!(
            box_signed_extents_along(&box_primitive, [0.0, 0.0, 1.0]),
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            box_corner_from_signed_extents(&box_primitive, [-1.0, 2.0, -3.0]),
            [-1.0, 2.0, -3.0]
        );
    }

    #[test]
    fn cylinder_box_candidates_preserve_cap_asymmetry_and_silhouette_order() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [2.0_f32.to_bits(), 0.5_f32.to_bits(), 0];
        cylinder_volume.transform[12] = 4.0;
        cylinder_volume.transform[13] = -5.0;
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(), 2.0_f32.to_bits(), 3.0_f32.to_bits()];
        let box_primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();

        let candidates = cylinder_box_separating_candidates(&cylinder, &box_primitive).unwrap();
        assert_eq!(candidates.len(), 16);
        assert_eq!(candidates[0], cylinder.edge_directions[0]);
        assert_eq!(&candidates[1..4], &box_primitive.face_normals);

        let positive_cap = [4.0, -5.0, 2.0];
        let negative_cap = [4.0, -5.0, -2.0];
        for (group, (cap, corner, signed_extents)) in [
            (positive_cap, [1.0, -2.0, 3.0], [1.0, -2.0, 3.0]),
            (negative_cap, [1.0, -2.0, -3.0], [1.0, -2.0, -3.0]),
        ]
        .into_iter()
        .enumerate()
        {
            for (axis, signed_extent) in signed_extents.into_iter().enumerate() {
                assert_eq!(
                    candidates[4 + group * 3 + axis],
                    rim_to_parametric_edge_separating_candidate(
                        cap,
                        [0.0, 0.0, 1.0],
                        0.5,
                        corner,
                        box_primitive.edge_directions[axis],
                        -signed_extent,
                        box_primitive.dimensions[axis],
                    )
                );
            }
        }

        let silhouette_corners = [[-1.0, 2.0, 3.0], [1.0, -2.0, 3.0], [1.0, 2.0, -3.0]];
        for (index, (corner, edge_axis)) in [(0, 1), (0, 2), (1, 2), (1, 0), (2, 0), (2, 1)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                candidates[10 + index],
                line_edge_separating_candidate(
                    cylinder.position(),
                    cylinder.edge_directions[0],
                    silhouette_corners[corner],
                    box_primitive.edge_directions[edge_axis],
                    -box_primitive.dimensions[edge_axis],
                    box_primitive.dimensions[edge_axis],
                )
            );
        }

        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(cylinder_box_separating_candidates(&cylinder, &sphere), None);
    }

    #[test]
    fn cylinder_box_separator_and_overlap_are_complete_in_both_orders() {
        let mut cylinder_volume = volume(VolumeType::Cylinder);
        cylinder_volume.shape_words = [1.0_f32.to_bits(), 1.0_f32.to_bits(), 0];
        let cylinder = GpPrimitive::from_volume(&cylinder_volume, None).unwrap();
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(); 3];
        box_volume.transform[12] = 3.0;
        let separated = GpPrimitive::from_volume(&box_volume, None).unwrap();

        assert_eq!(
            cylinder_box_separating_direction(&cylinder, &separated),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 1.0,
            })
        );
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &separated, 0.0),
            Some(false)
        );
        assert_eq!(
            generic_overlap_proven_subset(&separated, &cylinder, 0.0),
            Some(false)
        );

        box_volume.transform[12] = 2.0;
        let touching = GpPrimitive::from_volume(&box_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &touching, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&touching, &cylinder, 0.0),
            Some(true)
        );

        box_volume.transform[12] = 0.0;
        let coincident = GpPrimitive::from_volume(&box_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&cylinder, &coincident, 0.0),
            Some(true)
        );
    }

    #[test]
    fn creates_triangle_points_cached_normal_edges_and_interval() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        triangle.shape_words = [0.1_f32.to_bits(), 0.2_f32.to_bits(), 0.3_f32.to_bits()];

        let primitive = GpPrimitive::from_volume(&triangle, None).unwrap();
        assert_eq!(
            primitive.points,
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        );
        assert_eq!(primitive.face_normals[0], [0.0, 0.0, 1.0]);
        assert_eq!(primitive.edge_directions[0], [0.0, 1.0, 0.0]);
        assert_eq!(primitive.edge_directions[2], [-1.0, 0.0, 0.0]);
        assert_eq!(primitive.dimensions, [1.0, 2.0_f32.sqrt(), 1.0]);
        assert_eq!(primitive.edge_data, [0.1, 0.2, 0.3]);
        assert_eq!(
            primitive.projection_interval([1.0, 1.0, 0.0]),
            ProjectionInterval {
                minimum: 0.0,
                maximum: 1.0
            }
        );

        triangle.flags = 2;
        triangle.transform[12..15].copy_from_slice(&[9.0, 9.0, 9.0]);
        let rebuilt = GpPrimitive::from_volume(&triangle, None).unwrap();
        assert_eq!(rebuilt.face_normals[0], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn generic_separator_handles_sphere_against_capsule_core() {
        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.transform[12] = 5.0;
        capsule_volume.shape_words[0] = 2.0_f32.to_bits();
        let capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();

        assert_eq!(
            generic_separating_direction(&sphere, &capsule),
            SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 5.0
            }
        );
    }

    #[test]
    fn sphere_overlap_includes_fatness_padding_and_boundary_contact() {
        let mut first_volume = volume(VolumeType::Sphere);
        first_volume.radius = 1.0;
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = first_volume;
        second_volume.transform[12] = 2.25;
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();

        assert_eq!(spheres_overlap(&first, &second, 0.25), Some(true));
        assert_eq!(spheres_overlap(&first, &second, 0.249), Some(false));
        assert_eq!(spheres_overlap(&first, &first, -2.0), Some(true));
    }

    #[test]
    fn sk8_pre_fixup_stage_preserves_padding_boundary_and_one_point_normal_graph() {
        let mut first_volume = volume(VolumeType::Sphere);
        first_volume.radius = 0.5;
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = first_volume.clone();
        second_volume.transform[12] = 2.0;
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();
        let observed_operand = std::cell::Cell::new(None);

        let outcome = try_sk8_pre_triangle_fixup_candidate_with_numeric(
            &first,
            &second,
            1.0,
            length_squared,
            |value| {
                observed_operand.set(Some(value.to_bits()));
                value.sqrt().recip()
            },
        )
        .unwrap();
        let PreTriangleFixupOutcome::Candidate(candidate) = outcome else {
            panic!("exact padding equality must produce a candidate");
        };
        assert_eq!(observed_operand.get(), Some(4.0_f32.to_bits()));
        assert_eq!(candidate.separating.direction, [1.0, 0.0, 0.0]);
        assert_eq!(candidate.separating.separation, 2.0);
        assert_eq!(candidate.prism.point_count, 1);
        assert_eq!(candidate.prism.points_on_first[0], [0.0, 0.0, 0.0]);
        assert_eq!(candidate.prism.points_on_second[0], [2.0, 0.0, 0.0]);
        assert_eq!(candidate.prism.normal, [1.0, 0.0, 0.0]);
        assert_eq!(candidate.prism.normal_override, 1);
        assert_eq!(candidate.result_normal, [1.0, 0.0, 0.0]);

        assert_eq!(
            try_sk8_pre_triangle_fixup_candidate_with_numeric(
                &first,
                &second,
                0.999,
                |_| panic!("separation must precede the reduction site"),
                |_| panic!("separation must precede the estimate site"),
            ),
            Some(PreTriangleFixupOutcome::Separated)
        );
    }

    #[test]
    fn sk8_pre_fixup_stage_preserves_native_fatness_addition_association() {
        let mut first_volume = volume(VolumeType::Sphere);
        first_volume.radius = f32::from_bits(0x4086_e771);
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = volume(VolumeType::Sphere);
        second_volume.radius = f32::from_bits(0x40ff_b89e);
        second_volume.transform[12] = f32::from_bits(0x4146_d45d);
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();
        let padding = f32::from_bits(0x3e61_1547);

        assert_eq!(
            ((second.fatness + padding) + first.fatness).to_bits(),
            0x4146_d45c
        );
        assert_eq!(
            ((first.fatness + second.fatness) + padding).to_bits(),
            0x4146_d45d
        );
        assert_eq!(
            try_sk8_pre_triangle_fixup_candidate_with_numeric(
                &first,
                &second,
                padding,
                |_| panic!("native association rejects before the reduction site"),
                |_| panic!("native association rejects before the estimate site"),
            ),
            Some(PreTriangleFixupOutcome::Separated)
        );
    }

    #[test]
    fn sk8_pre_fixup_stage_executes_zero_estimate_but_retains_separating_normal() {
        let coincident = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        let observed_operand = std::cell::Cell::new(None);
        let outcome = try_sk8_pre_triangle_fixup_candidate_with_numeric(
            &coincident,
            &coincident,
            0.0,
            length_squared,
            |value| {
                observed_operand.set(Some(value.to_bits()));
                123.0
            },
        )
        .unwrap();
        let PreTriangleFixupOutcome::Candidate(candidate) = outcome else {
            panic!("coincident points must produce a candidate");
        };
        assert_eq!(observed_operand.get(), Some(0.0_f32.to_bits()));
        assert_eq!(candidate.prism.point_count, 1);
        assert_eq!(candidate.prism.normal, [0.0; 3]);
        assert_eq!(candidate.prism.normal_override, 0);
        assert_eq!(candidate.result_normal, candidate.separating.direction);
    }

    #[test]
    fn sk8_pre_fixup_newton_rounds_preserve_fused_result_bits() {
        let squared_length = f32::from_bits(0x3dcc_cccd);
        let seed = f32::from_bits(0x4048_dc22);
        assert_eq!(
            refine_reciprocal_sqrt_twice(squared_length, seed).to_bits(),
            0x404a_62c1
        );
    }

    #[test]
    fn sk8_pre_fixup_normal_selector_preserves_winner_tie_and_gate() {
        let separating = SeparatingDirection {
            direction: [1.0, 0.0, 0.0],
            separation: 2.0,
        };
        let override_normal = [0.0, 1.0, 0.0];
        let right = ProjectionInterval {
            minimum: 3.0,
            maximum: 4.0,
        };
        let left = ProjectionInterval {
            minimum: 0.0,
            maximum: 1.0,
        };
        assert_eq!(
            orient_pre_triangle_fixup_normal(separating, override_normal, right, left),
            [0.0, -1.0, 0.0]
        );
        assert_eq!(
            orient_pre_triangle_fixup_normal(separating, override_normal, left, right),
            override_normal
        );
        assert_eq!(
            orient_pre_triangle_fixup_normal(
                SeparatingDirection {
                    separation: f32::from_bits(0x4000_0001),
                    ..separating
                },
                override_normal,
                right,
                left,
            ),
            separating.direction
        );
        let tied = ProjectionInterval {
            minimum: 0.0,
            maximum: 0.0,
        };
        assert_eq!(
            orient_pre_triangle_fixup_normal(
                SeparatingDirection {
                    separation: 0.0,
                    ..separating
                },
                override_normal,
                tied,
                tied,
            ),
            override_normal
        );
    }

    #[test]
    fn triangle_fixup_without_edge_cos_preserves_raw_region_returns() {
        assert_eq!(TRIANGLE_FIXUP_ALIGNMENT_CUTOFF.to_bits(), 0x3f7f_f62b);
        assert_eq!(TRIANGLE_MINIMUM_CLIPPING_ANGLE.to_bits(), 0x3a83_126f);
        assert_eq!(
            [
                FLAG_TRIANGLE_ONE_SIDED,
                FLAG_TRIANGLE_EDGE_01_CONVEX,
                FLAG_TRIANGLE_EDGE_12_CONVEX,
                FLAG_TRIANGLE_EDGE_20_CONVEX,
                FLAG_TRIANGLE_USE_EDGE_COS,
                FLAG_TRIANGLE_VERTEX_0_DISABLE,
                FLAG_TRIANGLE_VERTEX_1_DISABLE,
                FLAG_TRIANGLE_VERTEX_2_DISABLE,
            ],
            [0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800]
        );
        assert_eq!(
            [
                (
                    EDGE_01_FIXUP.direction_index,
                    EDGE_01_FIXUP.cosine_index,
                    EDGE_01_FIXUP.convex_mask,
                ),
                (
                    EDGE_12_FIXUP.direction_index,
                    EDGE_12_FIXUP.cosine_index,
                    EDGE_12_FIXUP.convex_mask,
                ),
                (
                    EDGE_20_FIXUP.direction_index,
                    EDGE_20_FIXUP.cosine_index,
                    EDGE_20_FIXUP.convex_mask,
                ),
            ],
            [(2, 0, 0x20), (1, 1, 0x40), (0, 2, 0x80)]
        );
        let cases = [
            (TriangleFeatureType::Face, 1),
            (TriangleFeatureType::Edge01, 0x20),
            (TriangleFeatureType::Edge12, 0x40),
            (TriangleFeatureType::Vertex1, 1),
            (TriangleFeatureType::Edge20, 0x80),
            (TriangleFeatureType::Vertex0, 1),
            (TriangleFeatureType::Vertex2, 1),
        ];
        for (region, expected) in cases {
            let triangle = fixup_triangle(0x0e0, [0.0; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.0, []);
            assert_eq!(result, Some(expected), "two-sided {region:?}");
            numeric.assert_exhausted();

            let triangle = fixup_triangle(0x0f0, [0.0; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.0, []);
            let expected = if region == TriangleFeatureType::Face {
                0
            } else {
                expected
            };
            assert_eq!(result, Some(expected), "one-sided {region:?}");
            numeric.assert_exhausted();
        }

        for region in [
            TriangleFeatureType::Edge01,
            TriangleFeatureType::Edge12,
            TriangleFeatureType::Edge20,
        ] {
            let triangle = fixup_triangle(0, [0.0; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.0, []);
            assert_eq!(result, Some(0), "convex bit clear for {region:?}");
            numeric.assert_exhausted();
        }

        for (region, disable_flag) in [
            (TriangleFeatureType::Vertex0, FLAG_TRIANGLE_VERTEX_0_DISABLE),
            (TriangleFeatureType::Vertex1, FLAG_TRIANGLE_VERTEX_1_DISABLE),
            (TriangleFeatureType::Vertex2, FLAG_TRIANGLE_VERTEX_2_DISABLE),
        ] {
            let triangle = fixup_triangle(disable_flag, [0.0; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.0, []);
            assert_eq!(result, Some(0), "disabled {region:?}");
            numeric.assert_exhausted();
        }
    }

    #[test]
    fn triangle_fixup_sweeps_every_mode_and_region_cell() {
        let regions = [
            TriangleFeatureType::Face,
            TriangleFeatureType::Edge01,
            TriangleFeatureType::Edge12,
            TriangleFeatureType::Vertex1,
            TriangleFeatureType::Edge20,
            TriangleFeatureType::Vertex0,
            TriangleFeatureType::Vertex2,
        ];
        for use_edge_cos in [false, true] {
            for one_sided in [false, true] {
                for region in regions {
                    let flags = FLAG_TRIANGLE_EDGE_01_CONVEX
                        | FLAG_TRIANGLE_EDGE_12_CONVEX
                        | FLAG_TRIANGLE_EDGE_20_CONVEX
                        | if use_edge_cos {
                            FLAG_TRIANGLE_USE_EDGE_COS
                        } else {
                            0
                        }
                        | if one_sided {
                            FLAG_TRIANGLE_ONE_SIDED
                        } else {
                            0
                        };
                    let triangle = fixup_triangle(flags, [-1.0; 3]);
                    let helper_sums = if use_edge_cos
                        && matches!(
                            region,
                            TriangleFeatureType::Vertex0
                                | TriangleFeatureType::Vertex1
                                | TriangleFeatureType::Vertex2
                        ) {
                        vec![-1.0, -1.0]
                    } else {
                        Vec::new()
                    };
                    let (result, numeric) =
                        run_fixup_for_region(&triangle, region, 0.0, helper_sums);
                    let expected = if one_sided && region == TriangleFeatureType::Face {
                        0
                    } else if !use_edge_cos {
                        direct_triangle_region_return(flags, region)
                    } else {
                        1
                    };
                    assert_eq!(
                        result,
                        Some(expected),
                        "use_edge_cos={use_edge_cos} one_sided={one_sided} {region:?}"
                    );
                    numeric.assert_exhausted();
                }
            }
        }
    }

    #[test]
    fn triangle_fixup_alignment_cutoff_has_strict_classifier_entry() {
        let triangle = fixup_triangle(0, [0.0; 3]);
        for alignment in [
            TRIANGLE_FIXUP_ALIGNMENT_CUTOFF,
            -TRIANGLE_FIXUP_ALIGNMENT_CUTOFF,
            f32::from_bits(TRIANGLE_FIXUP_ALIGNMENT_CUTOFF.to_bits() + 1),
        ] {
            let mut numeric = QueuedTriangleFixupNumeric::new([alignment], []);
            let mut result_normal = [0.0, -1.0, alignment];
            let mut prism = FeatureIntersectionPrism::default();
            assert_eq!(
                fix_up_triangle_result_with_numeric(
                    &triangle,
                    &mut result_normal,
                    false,
                    &mut prism,
                    0.75,
                    0.0,
                    false,
                    &mut numeric,
                ),
                Some(1)
            );
            assert_eq!(numeric.events, ["sum"]);
            numeric.assert_exhausted();
        }

        let below = f32::from_bits(TRIANGLE_FIXUP_ALIGNMENT_CUTOFF.to_bits() - 1);
        let projections = feature_projections(TriangleFeatureType::Edge01);
        let mut numeric = QueuedTriangleFixupNumeric::new(
            [below, 1.0, projections[0], projections[1], projections[2]],
            [1.0],
        );
        let mut result_normal = [0.0, -1.0, below];
        let mut prism = FeatureIntersectionPrism::default();
        assert_eq!(
            fix_up_triangle_result_with_numeric(
                &triangle,
                &mut result_normal,
                false,
                &mut prism,
                0.75,
                0.0,
                false,
                &mut numeric,
            ),
            Some(0)
        );
        assert_eq!(numeric.events, ["sum", "sum", "seed", "sum", "sum", "sum"]);
        numeric.assert_exhausted();
    }

    #[test]
    fn triangle_fixup_edge_cos_tables_use_semantic_edge_mapping() {
        let edge_data = [0.1, 0.2, 0.3];
        for one_sided in [false, true] {
            let flags = FLAG_TRIANGLE_USE_EDGE_COS
                | FLAG_TRIANGLE_EDGE_01_CONVEX
                | FLAG_TRIANGLE_EDGE_12_CONVEX
                | FLAG_TRIANGLE_EDGE_20_CONVEX
                | if one_sided {
                    FLAG_TRIANGLE_ONE_SIDED
                } else {
                    0
                };
            let triangle = fixup_triangle(flags, edge_data);
            for (alignment, expected_by_region) in [(0.15, [1, 0, 0]), (0.25, [1, 1, 0])] {
                for (region, expected) in [
                    TriangleFeatureType::Edge01,
                    TriangleFeatureType::Edge12,
                    TriangleFeatureType::Edge20,
                ]
                .into_iter()
                .zip(expected_by_region)
                {
                    let (result, numeric) = run_fixup_for_region(&triangle, region, alignment, []);
                    assert_eq!(
                        result,
                        Some(expected),
                        "one_sided={one_sided} alignment={alignment} {region:?}"
                    );
                    numeric.assert_exhausted();
                }
            }
        }

        for (region, convex_mask) in [
            (TriangleFeatureType::Edge01, 0x20),
            (TriangleFeatureType::Edge12, 0x40),
            (TriangleFeatureType::Edge20, 0x80),
        ] {
            let triangle = fixup_triangle(FLAG_TRIANGLE_USE_EDGE_COS | convex_mask, [0.1; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.15, []);
            assert_eq!(result, Some(1), "isolated convex mask for {region:?}");
            numeric.assert_exhausted();
        }
        for region in [
            TriangleFeatureType::Edge01,
            TriangleFeatureType::Edge12,
            TriangleFeatureType::Edge20,
        ] {
            let triangle = fixup_triangle(FLAG_TRIANGLE_USE_EDGE_COS, [0.1; 3]);
            let (result, numeric) = run_fixup_for_region(&triangle, region, 0.15, []);
            assert_eq!(result, Some(0), "all-concave two-sided {region:?}");
            numeric.assert_exhausted();
        }
    }

    #[test]
    fn triangle_fixup_object_bypasses_vertex_disable_only_one_sided() {
        for (region, disable_flag) in [
            (TriangleFeatureType::Vertex0, FLAG_TRIANGLE_VERTEX_0_DISABLE),
            (TriangleFeatureType::Vertex1, FLAG_TRIANGLE_VERTEX_1_DISABLE),
            (TriangleFeatureType::Vertex2, FLAG_TRIANGLE_VERTEX_2_DISABLE),
        ] {
            let base_flags = FLAG_TRIANGLE_USE_EDGE_COS
                | FLAG_TRIANGLE_EDGE_01_CONVEX
                | FLAG_TRIANGLE_EDGE_12_CONVEX
                | FLAG_TRIANGLE_EDGE_20_CONVEX
                | disable_flag;
            let projections = feature_projections(region);
            for (one_sided, is_object, expected, helper_sums) in
                [(true, false, 0, 0), (true, true, 1, 2), (false, true, 0, 0)]
            {
                let triangle = fixup_triangle(
                    base_flags
                        | if one_sided {
                            FLAG_TRIANGLE_ONE_SIDED
                        } else {
                            0
                        },
                    [-1.0; 3],
                );
                let mut sums = vec![0.0, 1.0];
                sums.extend(projections);
                sums.extend(std::iter::repeat_n(-1.0, helper_sums));
                let mut numeric = QueuedTriangleFixupNumeric::new(sums, [1.0]);
                let mut result_normal = [0.0, 0.0, 1.0];
                let mut prism = FeatureIntersectionPrism::default();
                assert_eq!(
                    fix_up_triangle_result_with_numeric(
                        &triangle,
                        &mut result_normal,
                        false,
                        &mut prism,
                        0.75,
                        0.0,
                        is_object,
                        &mut numeric,
                    ),
                    Some(expected),
                    "{region:?} one_sided={one_sided} is_object={is_object}"
                );
                numeric.assert_exhausted();
            }
        }
    }

    #[test]
    fn triangle_fixup_accepted_concave_reverse_rewrites_second_points() {
        let triangle = fixup_triangle(
            FLAG_TRIANGLE_USE_EDGE_COS | FLAG_TRIANGLE_ONE_SIDED,
            [0.5, 0.0, 0.0],
        );
        let mut result_normal = [0.0, 0.4, -f32::from_bits(0x3f6a_a0bc)];
        let mut prism = FeatureIntersectionPrism {
            point_count: 2,
            normal: [11.0, 12.0, 13.0],
            normal_override: 0xa5a5_5a5a,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0] = [1.0, 2.0, 3.0];
        prism.points_on_second[0] = [10.0, 20.0, 30.0];
        prism.points_on_first[1] = [-4.0, 5.0, 6.0];
        prism.points_on_second[1] = [7.0, -8.0, 9.0];
        prism.points_on_first[2] = [101.0, 102.0, 103.0];
        prism.points_on_second[2] = [201.0, 202.0, 203.0];
        let original_first = prism.points_on_first;
        let original_unused_second = prism.points_on_second[2];
        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };

        assert_eq!(
            fix_up_triangle_result_with_numeric(
                &triangle,
                &mut result_normal,
                true,
                &mut prism,
                0.75,
                0.0,
                false,
                &mut numeric,
            ),
            Some(1)
        );
        assert_eq!(
            result_normal.map(f32::to_bits),
            [0x8000_0000, 0x8000_0000, 0xbf80_0000]
        );
        assert_eq!(prism.points_on_first, original_first);
        assert_eq!(prism.points_on_second[0], [1.0, 2.0, 30.0]);
        assert_eq!(prism.points_on_second[1], [-4.0, 5.0, 9.0]);
        assert_eq!(prism.points_on_second[2], original_unused_second);
        assert_eq!(prism.point_count, 2);
        assert_eq!(prism.normal, [11.0, 12.0, 13.0]);
        assert_eq!(prism.normal_override, 0xa5a5_5a5a);
        assert_eq!(
            numeric.events,
            [
                "sum", "sum", "seed", "sum", "sum", "sum", "sum", "seed", "sum", "sum"
            ]
        );
    }

    #[test]
    fn one_sided_triangle_edge_boundaries_and_host_count_safety_are_exact() {
        let triangle_normal = [0.0, 0.0, 1.0];
        let contact_normal = [0.0, 0.0, 1.0];
        let mut result_normal = [7.0, 8.0, 9.0];
        let mut prism = FeatureIntersectionPrism::default();
        let mut numeric = QueuedTriangleFixupNumeric::new([], []);
        assert_eq!(
            fix_up_one_sided_triangle_edge(
                true,
                0.4,
                0.5,
                &mut result_normal,
                false,
                contact_normal,
                triangle_normal,
                &mut prism,
                0.75,
                0.1,
                false,
                &mut numeric,
            ),
            Some(1)
        );
        assert_eq!(result_normal, [7.0, 8.0, 9.0]);
        numeric.assert_exhausted();

        prism.point_count = 17;
        let prism_before = prism.clone();
        let result_before = result_normal;
        let mut numeric = QueuedTriangleFixupNumeric::new([1.0], [1.0]);
        assert_eq!(
            fix_up_one_sided_triangle_edge(
                false,
                1.0,
                0.5,
                &mut result_normal,
                false,
                contact_normal,
                triangle_normal,
                &mut prism,
                0.75,
                0.0,
                false,
                &mut numeric,
            ),
            None
        );
        assert_eq!(numeric.events, ["sum", "seed"]);
        assert_eq!(result_normal, result_before);
        assert_eq!(prism, prism_before);
        numeric.assert_exhausted();
    }

    #[test]
    fn triangle_edge_cos_helpers_preserve_strict_side_and_full_nonconvex_rule() {
        let edge = [1.0, 0.0, 0.0];
        let triangle_normal = [0.0, 0.0, 1.0];

        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };
        assert!(edge_cos_test(
            edge,
            triangle_normal,
            [0.0, 1.0, 0.0],
            1.0,
            false,
            &mut numeric,
        ));
        assert_eq!(numeric.events, ["sum"]);

        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };
        assert!(!edge_cos_test(
            edge,
            triangle_normal,
            [0.0, 0.0, 1.0],
            0.5,
            false,
            &mut numeric,
        ));
        assert_eq!(numeric.events, ["sum"]);

        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };
        assert!(edge_cos_test_full(
            edge,
            triangle_normal,
            [0.0, 0.0, -1.0],
            1.0,
            false,
            &mut numeric,
        ));
        assert_eq!(numeric.events, ["sum", "sum", "sum", "seed", "sum"]);

        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };
        assert!(!edge_cos_test_full(
            edge,
            triangle_normal,
            edge,
            0.0,
            true,
            &mut numeric,
        ));
        assert_eq!(numeric.events, ["sum", "sum", "sum", "seed", "sum"]);
    }

    #[test]
    fn triangle_vertex_helpers_preserve_adjacent_order_and_first_failure() {
        struct SideCapture {
            results: std::collections::VecDeque<f32>,
            right_operands: Vec<[f32; 3]>,
        }

        impl TriangleFixupNumeric for SideCapture {
            fn sum_three_products(&mut self, _left: [f32; 3], right: [f32; 3]) -> f32 {
                self.right_operands.push(right);
                self.results
                    .pop_front()
                    .expect("unexpected helper reduction")
            }

            fn reciprocal_sqrt_seed(&mut self, _value: f32) -> f32 {
                panic!("nonconvex side rejection must not estimate")
            }
        }

        let triangle = fixup_triangle(FLAG_TRIANGLE_USE_EDGE_COS, [0.0; 3]);
        let triangle_normal = triangle.face_normals[0];
        for (first, second) in [
            (EDGE_01_FIXUP, EDGE_12_FIXUP),
            (EDGE_20_FIXUP, EDGE_01_FIXUP),
            (EDGE_12_FIXUP, EDGE_20_FIXUP),
        ] {
            let expected_first = triangle_edge_cross(
                bit_negate(triangle.edge_directions[first.direction_index]),
                triangle_normal,
            );
            let expected_second = triangle_edge_cross(
                bit_negate(triangle.edge_directions[second.direction_index]),
                triangle_normal,
            );
            let mut numeric = SideCapture {
                results: [-1.0, 0.0].into_iter().collect(),
                right_operands: Vec::new(),
            };
            assert!(!vertex_edge_cos_tests(
                &triangle,
                [0.0, 0.0, 1.0],
                triangle_normal,
                first,
                second,
                false,
                &mut numeric,
            ));
            assert_eq!(numeric.right_operands, [expected_first, expected_second]);

            let mut numeric = SideCapture {
                results: [0.0].into_iter().collect(),
                right_operands: Vec::new(),
            };
            assert!(!vertex_edge_cos_tests(
                &triangle,
                [0.0, 0.0, 1.0],
                triangle_normal,
                first,
                second,
                false,
                &mut numeric,
            ));
            assert_eq!(numeric.right_operands, [expected_first]);
        }
    }

    #[test]
    fn triangle_fixup_top_level_binds_each_vertex_to_native_adjacent_edges() {
        struct TopLevelCapture {
            results: std::collections::VecDeque<f32>,
            right_operands: Vec<[f32; 3]>,
            seed_count: usize,
        }

        impl TriangleFixupNumeric for TopLevelCapture {
            fn sum_three_products(&mut self, _left: [f32; 3], right: [f32; 3]) -> f32 {
                self.right_operands.push(right);
                self.results.pop_front().expect("unexpected reduction")
            }

            fn reciprocal_sqrt_seed(&mut self, _value: f32) -> f32 {
                self.seed_count += 1;
                1.0
            }
        }

        for one_sided in [false, true] {
            let triangle = fixup_triangle(
                FLAG_TRIANGLE_USE_EDGE_COS
                    | if one_sided {
                        FLAG_TRIANGLE_ONE_SIDED
                    } else {
                        0
                    },
                [0.0; 3],
            );
            let triangle_normal = triangle.face_normals[0];
            for (region, expected_indices) in [
                (TriangleFeatureType::Vertex1, [2, 1]),
                (TriangleFeatureType::Vertex0, [0, 2]),
                (TriangleFeatureType::Vertex2, [1, 0]),
            ] {
                let projections = feature_projections(region);
                let helper_results = if one_sided {
                    vec![-1.0, 0.0]
                } else {
                    vec![-1.0, 0.0, 0.0, 1.0, 1.0]
                };
                let mut results = vec![0.0, 1.0, projections[0], projections[1], projections[2]];
                results.extend(helper_results);
                let mut numeric = TopLevelCapture {
                    results: results.into_iter().collect(),
                    right_operands: Vec::new(),
                    seed_count: 0,
                };
                let mut result_normal = [0.0, 0.0, 1.0];
                let mut prism = FeatureIntersectionPrism::default();
                assert_eq!(
                    fix_up_triangle_result_with_numeric(
                        &triangle,
                        &mut result_normal,
                        false,
                        &mut prism,
                        0.75,
                        0.0,
                        false,
                        &mut numeric,
                    ),
                    Some(u32::from(!one_sided))
                );
                assert!(numeric.results.is_empty());
                assert_eq!(numeric.seed_count, if one_sided { 1 } else { 2 });
                let observed = [&numeric.right_operands[5], &numeric.right_operands[6]];
                let expected = expected_indices.map(|index| {
                    triangle_edge_cross(
                        bit_negate(triangle.edge_directions[index]),
                        triangle_normal,
                    )
                });
                assert_eq!(observed, [&expected[0], &expected[1]]);
            }
        }
    }

    #[test]
    fn one_sided_triangle_edge_preserves_object_and_bend_boundaries() {
        let triangle_normal = [0.0, 0.0, 1.0];
        let contact_normal = triangle_normal;
        for (edge_cos, uses_direct_route) in [
            (f32::from_bits(0x3f7f_ff59), true),
            (f32::from_bits(0x3f7f_ff58), false),
        ] {
            let mut result_normal = [7.0, 8.0, 9.0];
            let mut prism = FeatureIntersectionPrism::default();
            let mut numeric = if uses_direct_route {
                QueuedTriangleFixupNumeric::new([], [])
            } else {
                QueuedTriangleFixupNumeric::new([1.0], [1.0])
            };
            let result = fix_up_one_sided_triangle_edge(
                false,
                1.0,
                edge_cos,
                &mut result_normal,
                false,
                contact_normal,
                triangle_normal,
                &mut prism,
                1.0,
                0.0,
                true,
                &mut numeric,
            );
            assert_eq!(result, Some(1));
            assert_eq!(
                result_normal,
                if uses_direct_route {
                    [7.0, 8.0, 9.0]
                } else {
                    triangle_normal
                }
            );
            numeric.assert_exhausted();
        }

        for (edge_cos, threshold, expected, expected_events) in [
            (f32::from_bits(0x3f40_0001), 0.75, 0, 0),
            (TRIANGLE_MINIMUM_CLIPPING_ANGLE, 0.75, 0, 0),
            (0.5, 0.5, 1, 2),
        ] {
            let mut result_normal = [7.0, 8.0, 9.0];
            let mut prism = FeatureIntersectionPrism::default();
            let mut numeric = if expected_events == 0 {
                QueuedTriangleFixupNumeric::new([], [])
            } else {
                QueuedTriangleFixupNumeric::new([1.0], [1.0])
            };
            assert_eq!(
                fix_up_one_sided_triangle_edge(
                    false,
                    1.0,
                    edge_cos,
                    &mut result_normal,
                    false,
                    contact_normal,
                    triangle_normal,
                    &mut prism,
                    threshold,
                    0.0,
                    false,
                    &mut numeric,
                ),
                Some(expected)
            );
            assert_eq!(numeric.events.len(), expected_events);
            numeric.assert_exhausted();
        }
    }

    #[test]
    fn one_sided_triangle_edge_nonpositive_counts_still_write_normal() {
        for point_count in [0, -1] {
            let mut result_normal = [7.0, 8.0, 9.0];
            let mut prism = FeatureIntersectionPrism {
                point_count,
                ..FeatureIntersectionPrism::default()
            };
            prism.points_on_first[0] = [1.0, 2.0, 3.0];
            prism.points_on_second[0] = [4.0, 5.0, 6.0];
            let prism_before = prism.clone();
            let mut numeric = QueuedTriangleFixupNumeric::new([1.0], [1.0]);
            assert_eq!(
                fix_up_one_sided_triangle_edge(
                    false,
                    1.0,
                    0.5,
                    &mut result_normal,
                    true,
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    &mut prism,
                    0.75,
                    0.0,
                    false,
                    &mut numeric,
                ),
                Some(1)
            );
            assert_eq!(
                result_normal.map(f32::to_bits),
                [0x8000_0000, 0x8000_0000, 0xbf80_0000]
            );
            assert_eq!(prism, prism_before);
            assert_eq!(numeric.events, ["sum", "seed"]);
            numeric.assert_exhausted();
        }
    }

    #[test]
    fn one_sided_triangle_edge_face_dot_equality_rejects_without_mutation() {
        let edge_cos = 0.6_f32;
        let root_squared = 1.0 - edge_cos * edge_cos;
        let seed = root_squared.sqrt().recip();
        let root = root_squared * refine_reciprocal_sqrt_twice(root_squared, seed);
        let mut numeric = QueuedTriangleFixupNumeric::new([root], [seed]);
        let mut result_normal = [7.0, 8.0, 9.0];
        let mut prism = FeatureIntersectionPrism {
            point_count: 1,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0] = [1.0, 2.0, 3.0];
        prism.points_on_second[0] = [4.0, 5.0, 6.0];
        let prism_before = prism.clone();
        assert_eq!(
            fix_up_one_sided_triangle_edge(
                false,
                1.0,
                edge_cos,
                &mut result_normal,
                false,
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                &mut prism,
                0.75,
                0.0,
                false,
                &mut numeric,
            ),
            Some(0)
        );
        assert_eq!(result_normal, [7.0, 8.0, 9.0]);
        assert_eq!(prism, prism_before);
        numeric.assert_exhausted();
    }

    #[test]
    fn one_sided_triangle_edge_forward_mutation_keeps_ascending_panic_prefix() {
        struct PanicOnReduction {
            call: usize,
            panic_on: usize,
        }

        impl TriangleFixupNumeric for PanicOnReduction {
            fn sum_three_products(&mut self, left: [f32; 3], right: [f32; 3]) -> f32 {
                self.call += 1;
                assert_ne!(self.call, self.panic_on, "injected reduction panic");
                dot(left, right)
            }

            fn reciprocal_sqrt_seed(&mut self, value: f32) -> f32 {
                value.sqrt().recip()
            }
        }

        let mut result_normal = [7.0, 8.0, 9.0];
        let mut prism = FeatureIntersectionPrism {
            point_count: 2,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0] = [10.0, 20.0, 30.0];
        prism.points_on_second[0] = [1.0, 2.0, 3.0];
        prism.points_on_first[1] = [7.0, -8.0, 9.0];
        prism.points_on_second[1] = [-4.0, 5.0, 6.0];
        let second_before = prism.points_on_second;
        let point_one_before = prism.points_on_first[1];
        let mut numeric = PanicOnReduction {
            call: 0,
            panic_on: 3,
        };
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = fix_up_one_sided_triangle_edge(
                false,
                1.0,
                0.5,
                &mut result_normal,
                false,
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                &mut prism,
                0.75,
                0.0,
                false,
                &mut numeric,
            );
        }));
        assert!(panic.is_err());
        assert_eq!(result_normal, [0.0, 0.0, 1.0]);
        assert_eq!(prism.points_on_first[0], [1.0, 2.0, 30.0]);
        assert_eq!(prism.points_on_first[1], point_one_before);
        assert_eq!(prism.points_on_second, second_before);
    }

    #[test]
    fn triangle_fixup_owning_seam_consumes_rejected_candidates() {
        let triangle = fixup_triangle(FLAG_TRIANGLE_ONE_SIDED, [0.0; 3]);
        let candidate = PreTriangleFixupCandidate {
            separating: SeparatingDirection {
                direction: [0.0, 0.0, -1.0],
                separation: 0.0,
            },
            prism: FeatureIntersectionPrism::default(),
            result_normal: [0.0, 0.0, 1.0],
        };
        let mut numeric = HostTriangleFixupNumeric;
        assert!(
            accept_triangle_fixup_candidate_with_numeric(
                &triangle,
                candidate,
                true,
                0.75,
                0.0,
                false,
                &mut numeric,
            )
            .is_none()
        );
    }

    #[test]
    fn capsule_maximum_feature_uses_exact_simplification_boundary() {
        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.shape_words[0] = 2.0_f32.to_bits();
        let capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();

        let positive = capsule.maximum_feature([0.0, 0.0, 0.05]).unwrap();
        assert_eq!(positive.edge_count, 0);
        assert_eq!(positive.point, [0.0, 0.0, 2.0]);

        let edge = capsule.maximum_feature([1.0, 0.0, 0.0]).unwrap();
        assert_eq!(edge.edge_count, 1);
        assert_eq!(edge.edges[0].start, [0.0, 0.0, 2.0]);
        assert_eq!(edge.edges[0].direction, [0.0, 0.0, -1.0]);
        assert_eq!(edge.edges[0].length, 4.0);
    }

    #[test]
    fn feature_edge_clamp_preserves_endpoint_region_boundaries() {
        let edge = GpFeatureEdge::from_endpoints([1.0, 0.0, 0.0], [3.0, 0.0, 0.0]);
        let mut before = [0.0, 1.0, 0.0];
        let mut start = [1.0, 1.0, 0.0];
        let mut end = [3.0, 1.0, 0.0];
        let mut after = [4.0, 1.0, 0.0];
        assert_eq!(edge.constrain_point(&mut before), 1);
        assert_eq!(edge.constrain_point(&mut start), 2);
        assert_eq!(edge.constrain_point(&mut end), 2);
        assert_eq!(edge.constrain_point(&mut after), 3);
        assert_eq!(before, [1.0, 0.0, 0.0]);
        assert_eq!(start, [1.0, 0.0, 0.0]);
        assert_eq!(end, [3.0, 0.0, 0.0]);
        assert_eq!(after, [3.0, 0.0, 0.0]);
    }

    #[test]
    fn feature_edge_planes_preserve_winding_and_zero_degenerate_crosses() {
        let mut feature = GpFeature::default();
        feature.edges[0] = GpFeatureEdge::from_endpoints([0.0; 3], [1.0, 0.0, 0.0]);
        feature.edge_count = 1;

        feature.build_edge_planes(true, [0.0, 0.0, 1.0]);
        assert_eq!(feature.edges[0].plane_normal, [0.0, -1.0, 0.0]);
        feature.build_edge_planes(false, [0.0, 0.0, 1.0]);
        assert_eq!(feature.edges[0].plane_normal, [0.0, 1.0, 0.0]);

        feature.edges[0].direction = [f32::EPSILON, 0.0, 0.0];
        feature.build_edge_planes(true, [0.0, 0.0, 1.0]);
        assert_eq!(feature.edges[0].plane_normal, [0.0; 3]);
    }

    #[test]
    fn triangle_face_feature_preserves_reference_winding_regions_and_threshold() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let primitive = GpPrimitive::from_volume(&triangle, None).unwrap();

        assert!(
            primitive
                .triangle_face_feature(true, [0.0, 0.0, 0.95])
                .is_none()
        );

        let counter_clockwise = primitive
            .triangle_face_feature(true, [0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(counter_clockwise.region, 0);
        assert_eq!(counter_clockwise.edges[0].start, primitive.points[0]);
        assert_eq!(counter_clockwise.edges[0].direction, [1.0, 0.0, 0.0]);
        assert_eq!(counter_clockwise.edges[0].length, 1.0);
        assert_eq!(counter_clockwise.edges[0].plane_normal, [0.0, -1.0, 0.0]);

        let clockwise = primitive
            .triangle_face_feature(false, [0.0, 0.0, 1.0])
            .unwrap();
        assert_eq!(clockwise.region, 8);
        assert_eq!(clockwise.edges[0].start, primitive.points[0]);
        assert_eq!(clockwise.edges[0].direction, [0.0, 1.0, 0.0]);
        assert_eq!(clockwise.edges[0].length, 1.0);
        assert_eq!(clockwise.edges[0].plane_normal, [-1.0, 0.0, 0.0]);
        assert_eq!(clockwise.own_normal, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn triangle_feature_classifier_recovers_all_six_boundary_regions() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let primitive = GpPrimitive::from_volume(&triangle, None).unwrap();

        let cases = [
            ([0.0, 0.0, 1.0], TriangleFeatureType::Face),
            ([0.0, -1.0, 0.0], TriangleFeatureType::Edge01),
            ([1.0, 1.0, 0.0], TriangleFeatureType::Edge12),
            ([-1.0, 0.0, 0.0], TriangleFeatureType::Edge20),
            ([-1.0, -1.0, 0.0], TriangleFeatureType::Vertex0),
            ([1.0, -1.0, 0.0], TriangleFeatureType::Vertex1),
            ([-1.0, 1.0, 0.0], TriangleFeatureType::Vertex2),
        ];
        for (direction, expected) in cases {
            assert_eq!(
                primitive.triangle_feature_type_from_normal(direction),
                Some(expected)
            );
        }

        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(
            sphere.triangle_feature_type_from_normal([1.0, 0.0, 0.0]),
            None
        );
    }

    #[test]
    fn triangle_maximum_feature_preserves_edge_storage_vertex_ties_and_cutoff() {
        let mut triangle = volume(VolumeType::Triangle);
        triangle.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let primitive = GpPrimitive::from_volume(&triangle, None).unwrap();

        let edge_20 = primitive
            .triangle_maximum_feature(true, [-1.0, 0.0, 0.0])
            .unwrap();
        assert_eq!(edge_20.edge_count, 1);
        assert_eq!(edge_20.edges[0].start, primitive.points[0]);
        assert_eq!(edge_20.edges[0].direction, primitive.edge_directions[0]);
        assert_eq!(edge_20.edges[0].length, primitive.dimensions[0]);

        let edge_12 = primitive
            .triangle_maximum_feature(true, [1.0, 1.0, 0.0])
            .unwrap();
        assert_eq!(edge_12.edges[0].start, primitive.points[2]);
        assert_eq!(edge_12.edges[0].direction, primitive.edge_directions[1]);

        let edge_01 = primitive
            .triangle_maximum_feature(true, [0.0, -1.0, 0.0])
            .unwrap();
        assert_eq!(edge_01.edges[0].start, primitive.points[1]);
        assert_eq!(edge_01.edges[0].direction, primitive.edge_directions[2]);

        let exact_cutoff = primitive
            .triangle_maximum_feature(true, [-1.0, 0.05, 0.0])
            .unwrap();
        assert_eq!(exact_cutoff.edge_count, 0);
        assert_eq!(exact_cutoff.point, primitive.points[2]);
    }

    #[test]
    fn point_point_prism_copies_both_support_points() {
        let first = GpFeature {
            point: [1.0, 2.0, 3.0],
            ..GpFeature::default()
        };
        let second = GpFeature {
            point: [4.0, 5.0, 6.0],
            ..GpFeature::default()
        };
        let result = point_point_feature_prism(&first, &second);
        assert_eq!(result.point_count, 1);
        assert_eq!(result.points_on_first[0], first.point);
        assert_eq!(result.points_on_second[0], second.point);
    }

    #[test]
    fn edge_point_prism_projects_and_updates_endpoint_region() {
        let mut edge = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([0.0; 3], [2.0, 0.0, 0.0]);
        let point = GpFeature {
            point: [3.0, 1.0, 0.0],
            ..GpFeature::default()
        };

        let result = edge_point_feature_prism(&mut edge, &point).unwrap();
        assert_eq!(result.points_on_first[0], [2.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], point.point);
        assert_eq!(edge.region, 9);
    }

    #[test]
    fn face_point_prism_projects_to_plane_then_clamps_to_furthest_edge() {
        let corners = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        let mut face = GpFeature {
            edge_count: 4,
            ..GpFeature::default()
        };
        for index in 0..4 {
            face.edges[index] =
                GpFeatureEdge::from_endpoints(corners[index], corners[(index + 1) % 4]);
        }
        face.build_edge_planes(true, [0.0, 0.0, 1.0]);
        let point = GpFeature {
            point: [2.0, 0.0, 1.0],
            ..GpFeature::default()
        };

        let result = face_point_feature_prism(&mut face, &point, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.points_on_first[0], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], point.point);
        assert_eq!(face.region, 4);
    }

    #[test]
    fn point_prism_dispatch_preserves_original_argument_order() {
        let mut point = GpFeature {
            point: [3.0, 1.0, 0.0],
            ..GpFeature::default()
        };
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([0.0; 3], [2.0, 0.0, 0.0]);

        let result = point_involved_feature_prism(&mut point, &mut edge, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.points_on_first[0], point.point);
        assert_eq!(result.points_on_second[0], [2.0, 0.0, 0.0]);
    }

    #[test]
    fn nonparallel_edge_prism_solves_crossing_and_skew_edge_pairs() {
        let mut first = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([0.0; 3], [2.0, 0.0, 0.0]);
        let mut second = GpFeature {
            region: 20,
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([1.0, -1.0, 1.0], [1.0, 1.0, 1.0]);

        let result = nonparallel_edge_edge_feature_prism(&mut first, &mut second).unwrap();
        assert_eq!(result.point_count, 1);
        assert_eq!(result.points_on_first[0], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [1.0, 0.0, 1.0]);
        assert_eq!(first.region, 10);
        assert_eq!(second.region, 20);
    }

    #[test]
    fn nonparallel_edge_prism_preserves_epsilon_parallel_branch_boundary() {
        let mut first = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([0.0; 3], [1.0, 0.0, 0.0]);
        let mut second = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([0.0, 1.0, 0.0], [1.0, 1.0, 0.0]);

        assert!(nonparallel_edge_edge_feature_prism(&mut first, &mut second).is_none());
    }

    #[test]
    fn parallel_edge_prism_emits_strict_positive_overlap_interval() {
        let mut first = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([0.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
        let mut second = GpFeature {
            region: 20,
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([1.0, 1.0, 0.0], [3.0, 1.0, 0.0]);

        let result =
            parallel_edge_edge_feature_prism(&mut first, &mut second, [0.0, 1.0, 0.0]).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.points_on_first[0], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_first[1], [3.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [1.0, 1.0, 0.0]);
        assert_eq!(result.points_on_second[1], [3.0, 1.0, 0.0]);
        assert_eq!(first.region, 10);
        assert_eq!(second.region, 20);
    }

    #[test]
    fn parallel_edge_prism_preserves_opposed_edge_contact_order() {
        let mut first = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([0.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
        let mut second = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([3.0, 1.0, 0.0], [1.0, 1.0, 0.0]);

        let result = edge_edge_feature_prism(&mut first, &mut second, [0.0, 1.0, 0.0]).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.points_on_first[0], [3.0, 0.0, 0.0]);
        assert_eq!(result.points_on_first[1], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [3.0, 1.0, 0.0]);
        assert_eq!(result.points_on_second[1], [1.0, 1.0, 0.0]);
    }

    #[test]
    fn parallel_edge_prism_touching_interval_uses_nearest_endpoints() {
        let mut first = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let mut second = GpFeature {
            region: 20,
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([1.0, 1.0, 0.0], [2.0, 1.0, 0.0]);

        let result = edge_edge_feature_prism(&mut first, &mut second, [0.0, 1.0, 0.0]).unwrap();
        assert_eq!(result.point_count, 1);
        assert_eq!(result.points_on_first[0], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [1.0, 1.0, 0.0]);
        assert_eq!(first.region, 11);
        assert_eq!(second.region, 19);
    }

    #[test]
    fn parallel_edge_prism_disjoint_reverse_order_selects_opposite_endpoints() {
        let mut first = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        first.edges[0] = GpFeatureEdge::from_endpoints([3.0, 0.0, 0.0], [4.0, 0.0, 0.0]);
        let mut second = GpFeature {
            region: 20,
            edge_count: 1,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge::from_endpoints([0.0, 1.0, 0.0], [1.0, 1.0, 0.0]);

        let result = edge_edge_feature_prism(&mut first, &mut second, [0.0, 1.0, 0.0]).unwrap();
        assert_eq!(result.point_count, 1);
        assert_eq!(result.points_on_first[0], [3.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [1.0, 1.0, 0.0]);
        assert_eq!(first.region, 9);
        assert_eq!(second.region, 21);
    }

    fn square_face() -> GpFeature {
        let corners = [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        let mut face = GpFeature {
            edge_count: 4,
            ..GpFeature::default()
        };
        for index in 0..4 {
            face.edges[index] =
                GpFeatureEdge::from_endpoints(corners[index], corners[(index + 1) % 4]);
        }
        face
    }

    fn triangle_face(vertices: [[f32; 3]; 3]) -> GpFeature {
        let mut feature = GpFeature {
            edge_count: 3,
            own_normal: [0.0, 0.0, 1.0],
            ..GpFeature::default()
        };
        for index in 0..3 {
            feature.edges[index] =
                GpFeatureEdge::from_endpoints(vertices[index], vertices[(index + 1) % 3]);
        }
        feature.build_edge_planes(true, [0.0, 0.0, 1.0]);
        feature
    }

    #[test]
    fn feature_prism_dispatch_restores_edge_face_output_order() {
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-0.5, 0.0, 1.0], [0.5, 0.0, 1.0]);
        let mut face = triangle_face([[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]]);

        let result =
            find_feature_intersection_prism(&mut edge, &mut face, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.points_on_first[0], [-0.5, 0.0, 1.0]);
        assert_eq!(result.points_on_first[1], [0.5, 0.0, 1.0]);
        assert_eq!(result.points_on_second[0], [-0.5, 0.0, 0.0]);
        assert_eq!(result.points_on_second[1], [0.5, 0.0, 0.0]);
    }

    #[test]
    fn face_prism_edge_clip_preserves_reference_interval_orientation() {
        let face = square_face();
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-2.0, 0.0, 0.0], [2.0, 0.0, 0.0]);

        let clip = clip_edge_with_face_prism(&face, &edge, [0.0, 0.0, 1.0]).unwrap();
        assert!(clip.intersects);
        assert_eq!(clip.parameter_start, 1.0);
        assert_eq!(clip.parameter_end, 3.0);
        assert_eq!(clip.constraining_base, [0.0; 3]);
        assert_eq!(clip.constraining_direction, [0.0; 3]);
    }

    #[test]
    fn face_prism_edge_clip_retains_last_parallel_rejecting_constraint() {
        let face = square_face();
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-2.0, -2.0, 0.0], [2.0, -2.0, 0.0]);

        let clip = clip_edge_with_face_prism(&face, &edge, [0.0, 0.0, 1.0]).unwrap();
        assert!(!clip.intersects);
        assert_eq!(clip.parameter_start, 1.0);
        assert_eq!(clip.parameter_end, 3.0);
        assert_eq!(clip.constraining_base, [-1.0, -1.0, 0.0]);
        assert_eq!(clip.constraining_direction, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn face_prism_point_constraint_selects_furthest_positive_plane() {
        let face = square_face();
        let mut outside = [0.0, -2.0, 0.0];
        let region = constrain_point_to_face_prism(&face, [0.0, 0.0, 1.0], &mut outside).unwrap();
        assert_eq!(outside, [0.0, -1.0, 0.0]);
        assert_eq!(region, 2);

        let mut inside = [0.0, 0.0, 0.0];
        assert_eq!(
            constrain_point_to_face_prism(&face, [0.0, 0.0, 1.0], &mut inside),
            Some(0)
        );
        assert_eq!(inside, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn clipped_face_edge_prism_projects_both_interval_endpoints() {
        let mut face = square_face();
        face.own_normal = [0.0, 0.0, 1.0];
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-2.0, 0.0, 1.0], [2.0, 0.0, 1.0]);

        let result = clipped_face_edge_feature_prism(&face, &edge, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.points_on_first[0], [-1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_first[1], [1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [-1.0, 0.0, 1.0]);
        assert_eq!(result.points_on_second[1], [1.0, 0.0, 1.0]);
    }

    #[test]
    fn face_edge_prism_collapses_disjoint_interval_and_updates_regions() {
        let mut face = square_face();
        face.own_normal = [0.0, 0.0, 1.0];
        let mut edge = GpFeature {
            region: 10,
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-3.0, 0.0, 1.0], [-2.0, 0.0, 1.0]);

        let result = face_edge_feature_prism(&mut face, &mut edge, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.point_count, 1);
        assert_eq!(result.points_on_first[0], [-1.0, 0.0, 0.0]);
        assert_eq!(result.points_on_second[0], [-2.0, 0.0, 1.0]);
        assert_eq!(face.region, 8);
        assert_eq!(edge.region, 9);
        assert_eq!(result.normal_override, 0);
    }

    #[test]
    fn face_edge_prism_parallel_fallback_emits_override_normal() {
        let mut face = square_face();
        face.own_normal = [0.0, 0.0, 1.0];
        let mut edge = GpFeature {
            edge_count: 1,
            ..GpFeature::default()
        };
        edge.edges[0] = GpFeatureEdge::from_endpoints([-2.0, -2.0, 1.0], [2.0, -2.0, 1.0]);

        let result = face_edge_feature_prism(&mut face, &mut edge, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(result.points_on_first[0], [-1.0, -1.0, 0.0]);
        assert_eq!(result.points_on_first[1], [1.0, -1.0, 0.0]);
        assert_eq!(result.points_on_second[0], [-1.0, -2.0, 1.0]);
        assert_eq!(result.points_on_second[1], [1.0, -2.0, 1.0]);
        assert_eq!(result.normal_override, 1);
        let inverse_sqrt_two = 2.0_f32.sqrt().recip();
        assert_eq!(result.normal, [0.0, inverse_sqrt_two, -inverse_sqrt_two]);
    }

    #[test]
    fn intersecting_face_prism_preserves_contained_second_vertex_order() {
        let first = triangle_face([[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [0.0, 3.0, 0.0]]);
        let second = triangle_face([[0.5, 0.5, 1.0], [1.5, 0.5, 1.0], [0.5, 1.5, 1.0]]);

        let result = intersecting_face_face_feature_prism(&first, &second).unwrap();
        assert_eq!(result.point_count, 3);
        assert_eq!(
            result.points_on_second[..3],
            [[0.5, 0.5, 1.0], [1.5, 0.5, 1.0], [0.5, 1.5, 1.0]]
        );
        assert_eq!(
            result.points_on_first[..3],
            [[0.5, 0.5, 0.0], [1.5, 0.5, 0.0], [0.5, 1.5, 0.0]]
        );
    }

    #[test]
    fn intersecting_face_prism_emits_paired_boundary_crossings_in_reference_order() {
        let first = triangle_face([[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 4.0, 0.0]]);
        let second = triangle_face([[2.0, -1.0, 1.0], [5.0, 2.0, 1.0], [2.0, 5.0, 1.0]]);

        let result = intersecting_face_face_feature_prism(&first, &second).unwrap();
        assert_eq!(result.point_count, 4);
        let expected_first = [
            [3.0, 0.0, 0.0],
            [3.5, 0.5, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
        ];
        for (actual, expected) in result.points_on_first[..4].iter().zip(expected_first) {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() < 0.000_001);
            }
        }
        for index in 0..4 {
            assert_eq!(
                result.points_on_second[index][0..2],
                result.points_on_first[index][0..2]
            );
            assert_eq!(result.points_on_second[index][2], 1.0);
        }
    }

    #[test]
    fn face_face_nonoverlap_vertex_edge_clamps_and_sets_absolute_regions() {
        let cases = [
            ([-1.0, -1.0, 1.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0], 2),
            ([1.0, 1.0, 1.0], [2.0, 2.0, 1.0], [1.0, 1.0, 1.0], 1),
            ([-2.0, -2.0, 1.0], [-1.0, -1.0, 1.0], [-1.0, -1.0, 1.0], 3),
        ];
        for (second_start, second_end, expected_second, expected_second_region) in cases {
            let mut first = triangle_face([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]]);
            first.region = 99;
            let mut second = GpFeature {
                region: 88,
                edge_count: 2,
                ..GpFeature::default()
            };
            second.edges[0] = GpFeatureEdge::from_endpoints(second_start, second_end);

            let result = face_face_nonoverlap_feature_prism(&mut first, &mut second, 0, 0).unwrap();
            assert_eq!(result.point_count, 1);
            assert_eq!(result.points_on_first[0], [0.0, 0.0, 0.0]);
            for (actual, expected) in result.points_on_second[0].iter().zip(expected_second) {
                assert!((*actual - expected).abs() < 0.000_001);
            }
            assert_eq!(first.region, 1);
            assert_eq!(second.region, expected_second_region);
        }
    }

    #[test]
    fn face_face_nonoverlap_uses_strict_simplification_threshold() {
        let edge_direction = [0.05, (1.0_f32 - 0.05_f32.powi(2)).sqrt(), 0.0];
        let mut first = GpFeature {
            region: 77,
            edge_count: 3,
            ..GpFeature::default()
        };
        first.edges[0].start = edge_direction;
        first.edges[0].plane_normal = [0.0, 1.0, 0.0];
        first.edges[2].start = scale(edge_direction, 3.0);
        first.edges[2].direction = negate(edge_direction);
        first.edges[2].plane_normal = [1.0, 0.0, 0.0];
        first.edges[2].length = 2.0;
        let mut second = GpFeature {
            region: 88,
            edge_count: 2,
            ..GpFeature::default()
        };
        second.edges[0] = GpFeatureEdge {
            start: [0.0; 3],
            direction: edge_direction,
            plane_normal: [0.0; 3],
            length: 2.0,
        };

        let result = face_face_nonoverlap_feature_prism(&mut first, &mut second, 0, 0);
        assert!(result.is_some());
        assert_eq!(first.region, 1);
        assert_eq!(second.region, 2);

        first.region = 77;
        second.region = 88;
        first.edges[2].plane_normal = [0.999, 0.0, 0.0];
        let result = face_face_nonoverlap_feature_prism(&mut first, &mut second, 0, 0).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(first.region, 6);
        assert_eq!(second.region, 2);
    }

    #[test]
    fn face_face_nonoverlap_adjacent_edges_preserve_interval_order_and_touching_regions() {
        let cases = [
            ([0.0, 2.0, 1.0], 1, [0.0, 2.0, 0.0], [0.0, 2.0, 1.0], 5, 1),
            ([0.0, 1.0, 1.0], 2, [0.0, 1.0, 0.0], [0.0, 2.0, 1.0], 6, 2),
            ([0.0, -2.0, 1.0], 1, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 7, 3),
        ];
        for (
            second_start,
            expected_count,
            expected_first_zero,
            expected_second_last,
            expected_first_region,
            expected_second_region,
        ) in cases
        {
            let mut first = GpFeature {
                edge_count: 3,
                ..GpFeature::default()
            };
            first.edges[0].plane_normal = [0.0, -1.0, 0.0];
            first.edges[2] = GpFeatureEdge {
                start: [0.0, 2.0, 0.0],
                direction: [0.0, -1.0, 0.0],
                plane_normal: [-1.0, 0.0, 0.0],
                length: 2.0,
            };
            let mut second = GpFeature {
                edge_count: 2,
                ..GpFeature::default()
            };
            second.edges[0] = GpFeatureEdge {
                start: second_start,
                direction: [0.0, 1.0, 0.0],
                plane_normal: [0.0; 3],
                length: 2.0,
            };

            let result = face_face_nonoverlap_feature_prism(&mut first, &mut second, 0, 0).unwrap();
            assert_eq!(usize::try_from(result.point_count).unwrap(), expected_count);
            assert_eq!(result.points_on_first[0], expected_first_zero);
            assert_eq!(
                result.points_on_second[expected_count - 1],
                expected_second_last
            );
            assert_eq!(first.region, expected_first_region);
            assert_eq!(second.region, expected_second_region);
        }
    }

    #[test]
    fn face_face_prism_selects_maximin_fallback_and_restores_argument_order() {
        let mut first = square_face();
        first.own_normal = [0.0, 0.0, 1.0];
        first.build_edge_planes(true, first.own_normal);
        let mut second = square_face();
        second.own_normal = [0.0, 0.0, 1.0];
        for edge in &mut second.edges[..4] {
            edge.start[0] += 3.0;
        }
        second.build_edge_planes(true, second.own_normal);

        let result = face_face_feature_prism(&mut first, &mut second).unwrap();
        assert_eq!(result.point_count, 2);
        assert_eq!(
            result.points_on_first[..2],
            [[1.0, -1.0, 0.0], [1.0, 1.0, 0.0]]
        );
        assert_eq!(
            result.points_on_second[..2],
            [[2.0, -1.0, 0.0], [2.0, 1.0, 0.0]]
        );
        assert_eq!(first.region, 4);
        assert_eq!(second.region, 8);
        assert_eq!(result.normal, [1.0, 0.0, 0.0]);
        assert_eq!(result.normal_override, 1);
    }

    #[test]
    fn generic_overlap_subset_decides_skew_and_parallel_capsules() {
        let mut first_volume = volume(VolumeType::Capsule);
        first_volume.shape_words[0] = 1.0_f32.to_bits();
        first_volume.radius = 1.0;
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();

        let mut skew_volume = first_volume.clone();
        skew_volume.transform = [
            0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 1.0,
        ];
        let skew = GpPrimitive::from_volume(&skew_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &skew, 0.0),
            Some(true)
        );

        let mut parallel_volume = first_volume;
        parallel_volume.transform[12] = 0.5;
        let parallel = GpPrimitive::from_volume(&parallel_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &parallel, 0.0),
            Some(true)
        );

        parallel_volume.transform[12] = 3.0;
        let separated = GpPrimitive::from_volume(&parallel_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &separated, 0.0),
            Some(false)
        );
    }

    #[test]
    fn generic_overlap_decides_box_capsule() {
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(); 3];
        let box_primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();
        let mut capsule_volume = volume(VolumeType::Capsule);
        capsule_volume.shape_words[0] = 1.0_f32.to_bits();
        let capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();

        assert_eq!(
            generic_overlap_proven_subset(&box_primitive, &capsule, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&capsule, &box_primitive, 0.0),
            Some(true)
        );

        capsule_volume.transform[12] = 3.0;
        let separated_capsule = GpPrimitive::from_volume(&capsule_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&box_primitive, &separated_capsule, 0.0),
            Some(false)
        );
    }

    #[test]
    fn specialized_box_candidate_lists_preserve_face_and_cross_order() {
        let mut first_box_volume = volume(VolumeType::Box);
        first_box_volume.shape_words = [1.0_f32.to_bits(); 3];
        let first_box = GpPrimitive::from_volume(&first_box_volume, None).unwrap();
        let mut second_box_volume = first_box_volume;
        second_box_volume.transform = [
            2.0 / 3.0,
            -2.0 / 3.0,
            1.0 / 3.0,
            0.0,
            1.0 / 3.0,
            2.0 / 3.0,
            2.0 / 3.0,
            0.0,
            -2.0 / 3.0,
            -1.0 / 3.0,
            2.0 / 3.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        let second_box = GpPrimitive::from_volume(&second_box_volume, None).unwrap();

        let box_candidates = box_box_separating_candidates(&first_box, &second_box).unwrap();
        assert_eq!(box_candidates.len(), 15);
        assert_eq!(
            &box_candidates[..6],
            &[
                first_box.face_normals[2],
                first_box.face_normals[1],
                first_box.face_normals[0],
                second_box.face_normals[2],
                second_box.face_normals[1],
                second_box.face_normals[0],
            ]
        );
        for (first_axis, first_edge) in first_box.edge_directions.into_iter().enumerate() {
            for (second_axis, second_edge) in second_box.edge_directions.into_iter().enumerate() {
                assert_eq!(
                    box_candidates[6 + first_axis * 3 + second_axis],
                    normalize_unchecked(cross(first_edge, second_edge))
                );
            }
        }

        let mut triangle_volume = volume(VolumeType::Triangle);
        triangle_volume.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let triangle = GpPrimitive::from_volume(&triangle_volume, None).unwrap();
        let triangle_candidates =
            triangle_box_separating_candidates(&triangle, &second_box).unwrap();
        assert_eq!(triangle_candidates.len(), 13);
        assert_eq!(triangle_candidates[0], triangle.face_normals[0]);
        assert_eq!(
            &triangle_candidates[1..4],
            &[
                second_box.face_normals[2],
                second_box.face_normals[1],
                second_box.face_normals[0],
            ]
        );
        for (triangle_axis, triangle_edge) in triangle.edge_directions.into_iter().enumerate() {
            for (box_axis, box_edge) in second_box.edge_directions.into_iter().enumerate() {
                assert_eq!(
                    triangle_candidates[4 + triangle_axis * 3 + box_axis],
                    normalize_unchecked(cross(triangle_edge, box_edge))
                );
            }
        }

        let sphere = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        assert_eq!(box_box_separating_candidates(&first_box, &sphere), None);
        assert_eq!(triangle_box_separating_candidates(&triangle, &sphere), None);
    }

    #[test]
    fn box_box_separator_and_overlap_preserve_specialized_axis_order() {
        let mut first_volume = volume(VolumeType::Box);
        first_volume.shape_words = [1.0_f32.to_bits(); 3];
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = first_volume;
        second_volume.transform[12] = 3.0;
        let separated = GpPrimitive::from_volume(&second_volume, None).unwrap();

        assert_eq!(
            box_box_separating_direction(&first, &separated),
            Some(SeparatingDirection {
                direction: [1.0, 0.0, 0.0],
                separation: 1.0,
            })
        );
        assert_eq!(
            generic_overlap_proven_subset(&first, &separated, 0.0),
            Some(false)
        );

        second_volume.transform[12] = 2.0;
        let touching = GpPrimitive::from_volume(&second_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &touching, 0.0),
            Some(true)
        );

        second_volume.transform[12] = 0.0;
        let coincident = GpPrimitive::from_volume(&second_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&first, &coincident, 0.0),
            Some(true)
        );
    }

    #[test]
    fn triangle_box_separator_and_overlap_are_complete_in_both_orders() {
        let mut triangle_volume = volume(VolumeType::Triangle);
        triangle_volume.transform = [
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let triangle = GpPrimitive::from_volume(&triangle_volume, None).unwrap();
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [0.5_f32.to_bits(), 0.5_f32.to_bits(), 1.0_f32.to_bits()];
        box_volume.transform[14] = 2.0;
        let separated = GpPrimitive::from_volume(&box_volume, None).unwrap();

        assert_eq!(
            triangle_box_separating_direction(&triangle, &separated),
            Some(SeparatingDirection {
                direction: [0.0, 0.0, 1.0],
                separation: 1.0,
            })
        );
        assert_eq!(
            generic_overlap_proven_subset(&triangle, &separated, 0.0),
            Some(false)
        );
        assert_eq!(
            generic_overlap_proven_subset(&separated, &triangle, 0.0),
            Some(false)
        );

        box_volume.transform[14] = 1.0;
        let touching = GpPrimitive::from_volume(&box_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&triangle, &touching, 0.0),
            Some(true)
        );
        assert_eq!(
            generic_overlap_proven_subset(&touching, &triangle, 0.0),
            Some(true)
        );

        box_volume.transform[14] = 0.0;
        let coincident = GpPrimitive::from_volume(&box_volume, None).unwrap();
        assert_eq!(
            generic_overlap_proven_subset(&triangle, &coincident, 0.0),
            Some(true)
        );
    }

    #[test]
    fn sphere_box_overlap_is_complete_in_both_argument_orders() {
        let mut sphere_volume = volume(VolumeType::Sphere);
        sphere_volume.radius = 0.5;
        sphere_volume.transform[12] = 2.0;
        let sphere = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        let mut box_volume = volume(VolumeType::Box);
        box_volume.shape_words = [1.0_f32.to_bits(); 3];
        box_volume.radius = 0.25;
        let box_primitive = GpPrimitive::from_volume(&box_volume, None).unwrap();

        assert_eq!(
            sphere_primitive_overlap(&sphere, &box_primitive, 0.25),
            Some(true)
        );
        assert_eq!(
            sphere_primitive_overlap(&box_primitive, &sphere, 0.249),
            Some(false)
        );

        let cylinder = GpPrimitive::from_volume(&volume(VolumeType::Cylinder), None).unwrap();
        assert_eq!(
            sphere_primitive_overlap(&sphere, &cylinder, 0.0),
            Some(false)
        );
    }

    #[test]
    fn sk8_contact_serializer_preserves_tags_normal_bits_and_fatness_sides() {
        let mut first_volume = volume(VolumeType::Sphere);
        first_volume.radius = 0.25;
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = volume(VolumeType::Sphere);
        second_volume.radius = 0.5;
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();
        let mut prism = FeatureIntersectionPrism {
            point_count: 2,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0] = [1.0, 2.0, 3.0];
        prism.points_on_second[0] = [4.0, 5.0, 6.0];
        prism.points_on_first[1] = [-1.0, -2.0, -3.0];
        prism.points_on_second[1] = [-4.0, -5.0, -6.0];
        let candidate = PreTriangleFixupCandidate {
            separating: SeparatingDirection {
                direction: [0.0, 0.0, 1.0],
                separation: 0.0,
            },
            prism,
            result_normal: [0.0, -0.0, 1.0],
        };

        let points = serialize_sk8_contact_points(
            &first,
            GpInstanceTags::new(0x1020_3040, 0x5060_7080),
            &second,
            GpInstanceTags::new(0x90a0_b0c0, 0xd0e0_f000),
            &candidate,
        )
        .unwrap();

        assert_eq!(
            points.first_tags(),
            GpInstanceTags::new(0x1020_3040, 0x5060_7080)
        );
        assert_eq!(
            points.second_tags(),
            GpInstanceTags::new(0x90a0_b0c0, 0xd0e0_f000)
        );
        assert_eq!(
            points.normal().map(f32::to_bits),
            [0x8000_0000, 0, (-1.0_f32).to_bits()]
        );
        assert_eq!(
            points.point_pairs(),
            [
                Sk8ContactPointPair {
                    first: [1.0, 2.0, 3.25],
                    second: [4.0, 5.0, 5.5],
                },
                Sk8ContactPointPair {
                    first: [-1.0, -2.0, -2.75],
                    second: [-4.0, -5.0, -6.5],
                },
            ]
        );
    }

    #[test]
    fn sk8_contact_serializer_keeps_separate_multiply_then_add_sub_rounding() {
        let mut first = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        first.fatness = f32::from_bits(0x410c_1346);
        let mut second = first;
        second.fatness = f32::from_bits(0x3e53_1301);
        let mut prism = FeatureIntersectionPrism {
            point_count: 1,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0][0] = f32::from_bits(0x3f2c_6b78);
        prism.points_on_second[0][1] = f32::from_bits(0x3ef9_5c59);
        let candidate = PreTriangleFixupCandidate {
            separating: SeparatingDirection {
                direction: [0.0; 3],
                separation: 0.0,
            },
            prism,
            result_normal: [
                f32::from_bits(0x4183_f9ad),
                f32::from_bits(0x3fa4_ea24),
                0.0,
            ],
        };

        let points = serialize_sk8_contact_points(
            &first,
            GpInstanceTags::new(0, 0),
            &second,
            GpInstanceTags::new(0, 0),
            &candidate,
        )
        .unwrap();

        assert_eq!(points.point_pairs()[0].first()[0].to_bits(), 0x4311_195f);
        assert_eq!(points.point_pairs()[0].second()[1].to_bits(), 0x3e62_c63e);
    }

    #[test]
    fn sk8_contact_serializer_rejects_unrepresented_point_counts_before_writes() {
        let primitive = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        for point_count in [-1, 17] {
            let candidate = PreTriangleFixupCandidate {
                separating: SeparatingDirection {
                    direction: [1.0, 0.0, 0.0],
                    separation: 0.0,
                },
                prism: FeatureIntersectionPrism {
                    point_count,
                    ..FeatureIntersectionPrism::default()
                },
                result_normal: [1.0, 0.0, 0.0],
            };
            assert_eq!(
                serialize_sk8_contact_points(
                    &primitive,
                    GpInstanceTags::new(1, 2),
                    &primitive,
                    GpInstanceTags::new(3, 4),
                    &candidate,
                ),
                None
            );
        }
    }

    #[test]
    fn sk8_contact_points_runs_complete_sphere_pair_and_ignores_draw_flag() {
        let mut first_volume = volume(VolumeType::Sphere);
        first_volume.radius = 1.0;
        let first = GpPrimitive::from_volume(&first_volume, None).unwrap();
        let mut second_volume = first_volume;
        second_volume.transform[12] = 1.5;
        let second = GpPrimitive::from_volume(&second_volume, None).unwrap();

        let build = |draw_contacts| {
            try_sk8_compute_contact_points(
                &first,
                GpInstanceTags::new(1, 2),
                &second,
                GpInstanceTags::new(3, 4),
                0.0,
                0.5,
                0.01,
                false,
                draw_contacts,
            )
            .unwrap()
        };
        let hidden = build(false);
        let drawn = build(true);
        assert_eq!(hidden, drawn);
        assert_eq!(hidden.point_pairs().len(), 1);
        assert_eq!(hidden.normal(), [-1.0, -0.0, -0.0]);
        assert_eq!(hidden.point_pairs()[0].first(), [1.0, 0.0, 0.0]);
        assert_eq!(hidden.point_pairs()[0].second(), [0.5, 0.0, 0.0]);
    }

    #[test]
    fn assembly_contact_points_check_one_sided_triangles_in_both_operand_positions() {
        let mut triangle = GpPrimitive::from_triangle_points([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        triangle.flags = FLAG_TRIANGLE_ONE_SIDED;
        let triangle_tags = GpInstanceTags::new(1, 2);
        let sphere_tags = GpInstanceTags::new(3, 4);
        for (height, accepted) in [(-0.1, false), (0.1, true)] {
            let mut sphere_volume = volume(VolumeType::Sphere);
            sphere_volume.radius = 0.5;
            sphere_volume.transform[14] = height;
            let sphere = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
            for reverse in [false, true] {
                let (first, first_tags, second, second_tags) = if reverse {
                    (&sphere, sphere_tags, &triangle, triangle_tags)
                } else {
                    (&triangle, triangle_tags, &sphere, sphere_tags)
                };
                let contacts = try_compute_contact_points(
                    first,
                    first_tags,
                    second,
                    second_tags,
                    0.0,
                    0.5,
                    0.01,
                    false,
                );
                assert_eq!(
                    contacts.is_some(),
                    accepted,
                    "height={height}, reverse={reverse}"
                );
                if let Some(contacts) = contacts {
                    assert_eq!(contacts.first_tags(), first_tags);
                    assert_eq!(contacts.second_tags(), second_tags);
                    assert_eq!(contacts.point_pairs().len(), 1);
                }
            }
        }
    }

    #[test]
    fn assembly_contact_limit_preserves_the_native_fatness_addition_order() {
        // At 2^24 the ULP is 2. Native assembly contacts add 1+1 first;
        // the Sk8 path rounds away each 1 around padding independently.
        let mut sphere_volume = volume(VolumeType::Sphere);
        sphere_volume.radius = 1.0;
        let first = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        sphere_volume.transform[12] = 16_777_218.0;
        let second = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        let tags = GpInstanceTags::new(0, 0);
        assert!(try_compute_contact_points(
            &first, tags, &second, tags, 16_777_216.0, 0.5, 0.01, false,
        ).is_some());
        assert!(
            try_sk8_compute_contact_points(
                &first,
                tags,
                &second,
                tags,
                16_777_216.0,
                0.5,
                0.01,
                false,
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn sk8_contact_points_mandatorily_applies_reverse_triangle_fixup() {
        let mut triangle = GpPrimitive::from_triangle_points([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        triangle.flags = FLAG_TRIANGLE_ONE_SIDED;
        let first_tags = GpInstanceTags::new(1, 2);
        let second_tags = GpInstanceTags::new(3, 4);

        let mut sphere_volume = volume(VolumeType::Sphere);
        sphere_volume.radius = 0.5;
        sphere_volume.transform[14] = -0.1;
        let below = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        assert_eq!(
            try_sk8_compute_contact_points(
                &below,
                first_tags,
                &triangle,
                second_tags,
                0.0,
                0.5,
                0.01,
                false,
                false,
            ),
            None
        );
        let mut numeric = HostTriangleFixupNumeric;
        let rejected = try_sk8_compute_contact_points_with_numeric(
            &below,
            first_tags,
            &triangle,
            second_tags,
            0.0,
            0.5,
            0.01,
            false,
            &mut numeric,
        )
        .expect("fix-up rejection occurs after scratch packing");
        assert_eq!(rejected.returned_count, 0);
        assert_eq!(rejected.scratch.first_tags(), first_tags);
        assert_eq!(rejected.scratch.second_tags(), second_tags);
        assert_eq!(rejected.scratch.normal(), [-0.0, -0.0, -1.0]);
        assert_eq!(rejected.scratch.point_pairs().len(), 1);
        assert_eq!(rejected.scratch.point_pairs()[0].first(), [0.0, 0.0, 0.4]);
        assert_eq!(rejected.scratch.point_pairs()[0].second(), [0.0, 0.0, 0.0]);

        sphere_volume.transform[14] = 0.1;
        let above = GpPrimitive::from_volume(&sphere_volume, None).unwrap();
        assert!(
            try_sk8_compute_contact_points(
                &above,
                first_tags,
                &triangle,
                second_tags,
                0.0,
                0.5,
                0.01,
                false,
                false,
            )
            .is_some()
        );
    }

    #[test]
    fn sk8_contact_points_packs_the_accepted_post_fixup_normal_and_points() {
        let mut first = GpPrimitive::from_volume(&volume(VolumeType::Sphere), None).unwrap();
        first.fatness = 0.25;
        let mut triangle = fixup_triangle(
            FLAG_TRIANGLE_USE_EDGE_COS | FLAG_TRIANGLE_ONE_SIDED,
            [0.5, 0.0, 0.0],
        );
        triangle.fatness = 0.5;
        let mut prism = FeatureIntersectionPrism {
            point_count: 2,
            normal: [11.0, 12.0, 13.0],
            normal_override: 0xa5a5_5a5a,
            ..FeatureIntersectionPrism::default()
        };
        prism.points_on_first[0] = [1.0, 2.0, 3.0];
        prism.points_on_second[0] = [10.0, 20.0, 30.0];
        prism.points_on_first[1] = [-4.0, 5.0, 6.0];
        prism.points_on_second[1] = [7.0, -8.0, 9.0];
        let candidate = PreTriangleFixupCandidate {
            separating: SeparatingDirection {
                direction: [0.0; 3],
                separation: 0.0,
            },
            prism,
            result_normal: [0.0, 0.4, -f32::from_bits(0x3f6a_a0bc)],
        };
        let first_tags = GpInstanceTags::new(0x1020_3040, 0x5060_7080);
        let second_tags = GpInstanceTags::new(0x90a0_b0c0, 0xd0e0_f000);
        let mut numeric = RecordingHostTriangleFixupNumeric { events: Vec::new() };

        let computation = finalize_sk8_contact_points_with_numeric(
            &first,
            first_tags,
            &triangle,
            second_tags,
            candidate,
            0.75,
            0.0,
            false,
            &mut numeric,
        )
        .unwrap();

        assert_eq!(computation.returned_count, 2);
        assert_eq!(computation.scratch.first_tags(), first_tags);
        assert_eq!(computation.scratch.second_tags(), second_tags);
        assert_eq!(
            computation.scratch.normal().map(f32::to_bits),
            [0, 0, 1.0_f32.to_bits()]
        );
        assert_eq!(
            computation.scratch.point_pairs(),
            [
                Sk8ContactPointPair {
                    first: [1.0, 2.0, 2.75],
                    second: [1.0, 2.0, 30.5],
                },
                Sk8ContactPointPair {
                    first: [-4.0, 5.0, 5.75],
                    second: [-4.0, 5.0, 9.5],
                },
            ]
        );
        assert_eq!(
            numeric.events,
            [
                "sum", "sum", "seed", "sum", "sum", "sum", "sum", "seed", "sum", "sum"
            ]
        );
    }
}
