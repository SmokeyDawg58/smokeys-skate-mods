//! StepSelectorGeoAnalyzer (827c2498/827c2808), supplied-triangle calculation.
//! The world walk/box query and selected-tick future ownership belong to callers.
//! Host min/max, estimate seeds and dot reduction retain the numeric uncertainty
//! documented by reckoning_numeric; this module does not claim Xenon bit parity.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "awaits StepSelector TestTrajectory integration")
)]

use crate::reckoning_numeric::{
    ReckoningNumeric, cross_full4, length_from_squared_and_estimate, try_normalize_safe,
    try_normalize_unsafe,
};
use crate::simulation_drive_jacobian::refine_drive_reciprocal;
use std::cmp::Ordering;

type V = [f32; 4];
const ZERO: V = [0.0; 4];
const UP: V = [0.0, 1.0, 0.0, 0.0];
const SMALL: f32 = f32::from_bits(0x3780_0000);

/// Primitive live reads, invoked only at the corresponding source load.
/// Implementations must resolve the actual cVar references, not cached defaults.
pub(crate) trait StepGeometryTuning {
    type Error;
    fn minimum_bad_y(&mut self) -> Result<f32, Self::Error>;
    fn vertical_normal_y(&mut self) -> Result<f32, Self::Error>;
    fn additional_gap(&mut self) -> Result<f32, Self::Error>;
    fn display_bad_triangles(&mut self) -> Result<bool, Self::Error>;
    /// Native debug singleton assertion; not a fabricated draw operation.
    fn require_debug_owner(&mut self) -> Result<(), Self::Error>;
}

#[derive(Debug, PartialEq)]
pub(crate) enum StepGeometryError<N, T> {
    Numeric(N),
    Tuning(T),
}
type Outcome<R, N, T> = Result<R, StepGeometryError<N, T>>;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StepGeometryTriangle {
    pub(crate) vertices: [V; 3],
    pub(crate) surface: u32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StepGeometryResult {
    pub(crate) position: V,
    pub(crate) normal: V,
    pub(crate) surface: u32,
}
#[derive(Clone, Copy)]
pub(crate) struct StepGeometryInput {
    pub(crate) collision_position: V,
    pub(crate) collision_normal: V,
    pub(crate) radius: f32,
    pub(crate) step_radius: f32,
    pub(crate) normal_y_threshold: f32,
    pub(crate) maximum_y: f32,
}
#[derive(Clone, Copy, Default)]
struct Triangle {
    source: StepGeometryTriangle,
    normal: V,
}
#[derive(Clone, Copy, Default)]
struct Point {
    result: StepGeometryResult,
    direction: V,
}

/// Source capacities: 64 retained triangles, 8 ordered list nodes. The source
/// trajectory member is not consulted by any calculation in this class.
pub(crate) struct StepGeometryAnalyzer {
    input: StepGeometryInput,
    triangles: [Triangle; 64],
    triangle_count: usize,
    good: [usize; 64],
    good_count: usize,
    bad: [usize; 64],
    bad_count: usize,
    points: [Point; 8],
    point_count: usize,
}

fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn horizontal(mut v: V) -> V {
    v[1] = 0.0;
    v
}

impl StepGeometryAnalyzer {
    /// Constructor827c2498. Initial candidate is the supplied collision point.
    pub(crate) fn new(input: StepGeometryInput) -> Self {
        let mut points = [Point::default(); 8];
        points[0].result = StepGeometryResult {
            position: input.collision_position,
            normal: UP,
            surface: 0,
        };
        Self {
            input,
            triangles: [Triangle::default(); 64],
            triangle_count: 0,
            good: [0; 64],
            good_count: 0,
            bad: [0; 64],
            bad_count: 0,
            points,
            point_count: 1,
        }
    }

    /// AddTriangle827bdfd8. Rejected triangles do not consume the 64 slots.
    pub(crate) fn add_triangle<N: ReckoningNumeric, T: StepGeometryTuning>(
        &mut self,
        source: StepGeometryTriangle,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Outcome<(), N::Error, T::Error> {
        use StepGeometryError::{Numeric, Tuning};
        if self.triangle_count >= 64 {
            return Ok(());
        }
        let [a, b, c] = source.vertices;
        let normal =
            try_normalize_unsafe(cross_full4(sub(b, a), sub(c, a)), numeric).map_err(Numeric)?;
        if normal[1] >= self.input.normal_y_threshold {
            self.good[self.good_count] = self.triangle_count;
            self.good_count += 1;
        } else {
            let minimum = tuning.minimum_bad_y().map_err(Tuning)?;
            if normal[1].partial_cmp(&minimum) != Some(Ordering::Greater) {
                return Ok(());
            }
            self.bad[self.bad_count] = self.triangle_count;
            self.bad_count += 1;
        }
        self.triangles[self.triangle_count] = Triangle { source, normal };
        self.triangle_count += 1;
        Ok(())
    }

    fn erase(&mut self, index: usize) {
        self.points.copy_within(index + 1..self.point_count, index);
        self.point_count -= 1;
    }
    fn insert(&mut self, index: usize, point: Point) -> bool {
        if self.point_count == 8 {
            return false;
        }
        self.points.copy_within(index..self.point_count, index + 1);
        self.points[index] = point;
        self.point_count += 1;
        true
    }

    /// Calculate827c2808. A miss leaves the caller's result unchanged.
    pub(crate) fn calculate<N: ReckoningNumeric, T: StepGeometryTuning>(
        &mut self,
        result: &mut StepGeometryResult,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Outcome<bool, N::Error, T::Error> {
        if self.good_count == 0 {
            return Ok(false);
        }
        self.exclude_bad(numeric, tuning)?;
        self.support_points(numeric)
            .map_err(StepGeometryError::Numeric)?;
        self.filter_collision(numeric)
            .map_err(StepGeometryError::Numeric)?;
        self.double_check_bad(numeric, tuning)?;
        let mut best_y = -f32::MAX;
        let mut selected = None;
        for point in &self.points[..self.point_count] {
            let y = point.result.position[1];
            if y > best_y && self.input.maximum_y > y {
                best_y = y;
                selected = Some(point.result);
            }
        }
        if let Some(selected) = selected {
            *result = selected;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// CheckPointsAgainstBadTriangles827c2720 / ExcludePointFromTriangle827c2608.
    fn exclude_bad<N: ReckoningNumeric, T: StepGeometryTuning>(
        &mut self,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Outcome<(), N::Error, T::Error> {
        use StepGeometryError::{Numeric, Tuning};
        for bad in 0..self.bad_count {
            let triangle = self.triangles[self.bad[bad]];
            let originals = self.point_count;
            let mut cursor = 0;
            for _ in 0..originals {
                let point = self.points[cursor];
                let radius = self.input.step_radius + tuning.additional_gap().map_err(Tuning)?;
                let mut replaced = false;
                if triangle_circle(point.result.position, triangle, radius, numeric)
                    .map_err(Numeric)?
                {
                    for edge in 0..3 {
                        let (valid, edge_normal) =
                            edge_normal(triangle, edge, numeric).map_err(Numeric)?;
                        if valid {
                            if let Some(candidate) = self.consider_edge(
                                point,
                                triangle,
                                edge,
                                edge_normal,
                                numeric,
                                tuning,
                            )? {
                                if self.insert(cursor, candidate) {
                                    cursor += 1;
                                }
                                // Source reports replacement even if the list was full.
                                replaced = true;
                            }
                        }
                    }
                }
                if replaced {
                    self.erase(cursor);
                } else {
                    cursor += 1;
                }
                // Cursor points at captured original-next; inserted nodes are skipped.
            }
        }
        Ok(())
    }

    /// ConsiderPointVersusEdge827bfb58.
    fn consider_edge<N: ReckoningNumeric, T: StepGeometryTuning>(
        &self,
        mut point: Point,
        triangle: Triangle,
        edge: usize,
        edge_normal: V,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Outcome<Option<Point>, N::Error, T::Error> {
        use StepGeometryError::Numeric;
        let position = move_from_edge(
            point.result.position,
            triangle.source.vertices[edge],
            edge_normal,
            triangle.normal,
            self.input.step_radius * f32::from_bits(0x3f81_47ae),
            numeric,
            tuning,
        )?;
        let delta = sub(self.input.collision_position, position);
        let squared = numeric.dot3(delta, delta).map_err(Numeric)?;
        let allowed = self.input.radius - self.input.step_radius;
        if (allowed * allowed).partial_cmp(&squared) != Some(Ordering::Greater) {
            return Ok(None);
        }
        let direction = try_normalize_safe(sub(position, point.result.position), ZERO, numeric)
            .map_err(Numeric)?;
        if numeric.dot3(direction, point.direction).map_err(Numeric)? >= f32::from_bits(0xb8d1_b717)
        {
            point.result.position = position;
            point.direction = direction;
            Ok(Some(point))
        } else {
            Ok(None)
        }
    }

    /// CheckPointsInsideGoodTriangle827c0f20; stable first triangle wins Y ties.
    fn support_points<N: ReckoningNumeric>(&mut self, numeric: &mut N) -> Result<(), N::Error> {
        let mut cursor = 0;
        while cursor < self.point_count {
            let position = self.points[cursor].result.position;
            let mut best = StepGeometryResult {
                position: [0.0, -1_000_000.0, 0.0, 0.0],
                normal: UP,
                surface: 0,
            };
            let mut found = false;
            for good in &self.good[..self.good_count] {
                let triangle = self.triangles[*good];
                if inside_xz(position, triangle.source.vertices) {
                    let projected = project_plane(position, triangle, numeric)?;
                    if projected[1] > best.position[1] {
                        best = StepGeometryResult {
                            position: projected,
                            normal: triangle.normal,
                            surface: triangle.source.surface,
                        };
                    }
                    found = true;
                }
            }
            if found {
                self.points[cursor].result = best;
                cursor += 1;
            } else {
                self.erase(cursor);
            }
        }
        Ok(())
    }

    /// CheckPointsAgainstCollisionPos827c0da0.
    fn filter_collision<N: ReckoningNumeric>(&mut self, numeric: &mut N) -> Result<(), N::Error> {
        if self.input.collision_normal[1] < f32::from_bits(0x3f33_3333) {
            let normal = horizontal(self.input.collision_normal);
            let minimum = self.input.collision_position[1] - f32::from_bits(0x3d4c_cccd);
            let mut cursor = 0;
            while cursor < self.point_count {
                let point = self.points[cursor].result.position;
                if point[1] < minimum
                    && numeric.dot3(
                        normal,
                        horizontal(sub(point, self.input.collision_position)),
                    )? < 0.0
                {
                    self.erase(cursor);
                } else {
                    cursor += 1;
                }
            }
        }
        Ok(())
    }

    /// DoubleCheckPointsAgainstBadTriangles827c0c38.
    fn double_check_bad<N: ReckoningNumeric, T: StepGeometryTuning>(
        &mut self,
        numeric: &mut N,
        tuning: &mut T,
    ) -> Outcome<(), N::Error, T::Error> {
        use StepGeometryError::{Numeric, Tuning};
        for bad in &self.bad[..self.bad_count] {
            let triangle = self.triangles[*bad];
            if tuning.display_bad_triangles().map_err(Tuning)? {
                tuning.require_debug_owner().map_err(Tuning)?;
            }
            let mut cursor = 0;
            while cursor < self.point_count {
                if triangle_circle(
                    self.points[cursor].result.position,
                    triangle,
                    self.input.step_radius,
                    numeric,
                )
                .map_err(Numeric)?
                {
                    if tuning.display_bad_triangles().map_err(Tuning)? {
                        tuning.require_debug_owner().map_err(Tuning)?;
                    }
                    self.points
                        .copy_within(cursor + 1..self.point_count, cursor);
                    self.point_count -= 1;
                } else {
                    cursor += 1;
                }
            }
        }
        Ok(())
    }
}

/// IsPointInsideTriangleXZ827be4d0. Products are separately rounded, not FMA.
fn inside_xz(point: V, vertices: [V; 3]) -> bool {
    let determinants: [f32; 3] = std::array::from_fn(|i| {
        let a = vertices[i];
        let b = vertices[(i + 1) % 3];
        let first = (b[2] - a[2]) * (point[0] - a[0]);
        let second = (b[0] - a[0]) * (point[2] - a[2]);
        first - second
    });
    let minimum = determinants[0].min(determinants[1]).min(determinants[2]);
    let maximum = determinants[0].max(determinants[1]).max(determinants[2]);
    minimum * maximum >= 0.0
}

/// GetPointOnTrianglePlane827be5a0. Preserve X/Z/W and the unprojected fallback.
fn project_plane<N: ReckoningNumeric>(
    mut point: V,
    triangle: Triangle,
    numeric: &mut N,
) -> Result<V, N::Error> {
    let normal = triangle.normal;
    if normal[1].abs() > SMALL {
        let inverse = refine_drive_reciprocal(normal[1], numeric.reciprocal_estimate(normal[1])?);
        let vertex = triangle.source.vertices[0];
        let z_term = (point[2] - vertex[2]) * normal[2];
        let numerator = (point[0] - vertex[0]).mul_add(normal[0], z_term);
        point[1] = vertex[1] - inverse * numerator;
    }
    Ok(point)
}

/// DistBetweenPointAndLineSegmentXZ827bbc78. Both endpoints and point flatten Y.
fn segment_distance<N: ReckoningNumeric>(
    point: V,
    a: V,
    b: V,
    numeric: &mut N,
) -> Result<f32, N::Error> {
    let point = horizontal(point);
    let a = horizontal(a);
    let b = horizontal(b);
    let direction = try_normalize_safe(sub(b, a), ZERO, numeric)?;
    let from_a = sub(point, a);
    let distance = numeric.dot3(from_a, direction)?;
    let projected = std::array::from_fn(|i| distance.mul_add(direction[i], a[i]));
    let squared = if numeric.dot3(sub(projected, a), sub(projected, b))? > 0.0 {
        let from_b = sub(point, b);
        let first = numeric.dot3(from_a, from_a)?;
        let second = numeric.dot3(from_b, from_b)?;
        if second > first { first } else { second }
    } else {
        let delta = sub(projected, point);
        numeric.dot3(delta, delta)?
    };
    Ok(length_from_squared_and_estimate(
        squared,
        numeric.reciprocal_sqrt_estimate(squared)?,
    ))
}

/// TriangleVersusCircleXZ827bfcc0. The distance rejection is ordered GT.
fn triangle_circle<N: ReckoningNumeric>(
    point: V,
    triangle: Triangle,
    radius: f32,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    for edge in 0..3 {
        let distance = segment_distance(
            point,
            triangle.source.vertices[edge],
            triangle.source.vertices[(edge + 1) % 3],
            numeric,
        )?;
        if distance.partial_cmp(&radius) != Some(Ordering::Greater) {
            return Ok(true);
        }
    }
    Ok(inside_xz(point, triangle.source.vertices))
}

/// CalculateEdgeNormal827be158. Even the false arm computes its native fallback.
fn edge_normal<N: ReckoningNumeric>(
    triangle: Triangle,
    index: usize,
    numeric: &mut N,
) -> Result<(bool, V), N::Error> {
    let current = triangle.source.vertices[index];
    let third = triangle.source.vertices[(index + 2) % 3];
    let edge = horizontal(sub(triangle.source.vertices[(index + 1) % 3], current));
    if numeric.dot3(edge, edge)? > f32::from_bits(0x3727_c5ac) {
        let mut normal = try_normalize_unsafe([-edge[2], 0.0, edge[0], 0.0], numeric)?;
        if numeric.dot3(normal, sub(current, third))? >= SMALL {
            normal = normal.map(|v| f32::from_bits(v.to_bits() ^ 0x8000_0000));
        }
        Ok((true, normal))
    } else {
        Ok((
            false,
            try_normalize_safe(sub(current, third), ZERO, numeric)?,
        ))
    }
}

/// MovePointAwayFromEdge827be398; conditional extra-gap fetch stays lazy.
fn move_from_edge<N: ReckoningNumeric, T: StepGeometryTuning>(
    point: V,
    edge_point: V,
    edge_normal: V,
    triangle_normal: V,
    radius: f32,
    numeric: &mut N,
    tuning: &mut T,
) -> Outcome<V, N::Error, T::Error> {
    use StepGeometryError::{Numeric, Tuning};
    let mut distance = numeric
        .dot3(sub(point, edge_point), edge_normal)
        .map_err(Numeric)?
        - radius;
    if tuning.vertical_normal_y().map_err(Tuning)? > triangle_normal[1].abs()
        && numeric
            .dot3(triangle_normal, edge_normal)
            .map_err(Numeric)?
            > 0.0
    {
        distance -= tuning.additional_gap().map_err(Tuning)?;
    }
    Ok(if 0.0 > distance {
        std::array::from_fn(|i| point[i] - edge_normal[i] * distance)
    } else {
        point
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reckoning_numeric::HostReckoningNumeric;
    #[derive(Default)]
    struct Tuning {
        reads: Vec<&'static str>,
    }
    impl StepGeometryTuning for Tuning {
        type Error = std::convert::Infallible;
        fn minimum_bad_y(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("bad_y");
            Ok(-0.01)
        }
        fn vertical_normal_y(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("vertical");
            Ok(0.3)
        }
        fn additional_gap(&mut self) -> Result<f32, Self::Error> {
            self.reads.push("gap");
            Ok(0.1)
        }
        fn display_bad_triangles(&mut self) -> Result<bool, Self::Error> {
            self.reads.push("debug");
            Ok(false)
        }
        fn require_debug_owner(&mut self) -> Result<(), Self::Error> {
            unreachable!()
        }
    }
    fn input() -> StepGeometryInput {
        StepGeometryInput {
            collision_position: [0.0, 0.2, 0.0, 1.0],
            collision_normal: UP,
            radius: 2.0,
            step_radius: 0.1,
            normal_y_threshold: 0.7,
            maximum_y: 1.0,
        }
    }
    fn floor(y: f32, surface: u32) -> StepGeometryTriangle {
        StepGeometryTriangle {
            vertices: [
                [-3.0, y, -3.0, 1.0],
                [0.0, y, 3.0, 1.0],
                [3.0, y, -3.0, 1.0],
            ],
            surface,
        }
    }
    #[test]
    fn selects_highest_support_and_keeps_first_surface_on_ties() {
        let mut analyzer = StepGeometryAnalyzer::new(input());
        let mut numeric = HostReckoningNumeric;
        let mut tuning = Tuning::default();
        for triangle in [floor(0.0, 1), floor(0.5, 2), floor(0.5, 3)] {
            analyzer
                .add_triangle(triangle, &mut numeric, &mut tuning)
                .unwrap();
        }
        let mut result = StepGeometryResult::default();
        assert!(
            analyzer
                .calculate(&mut result, &mut numeric, &mut tuning)
                .unwrap()
        );
        assert_eq!(result.position, [0.0, 0.5, 0.0, 1.0]);
        assert_eq!(result.normal[..3], UP[..3]);
        assert_eq!(result.surface, 2);
        assert!(tuning.reads.is_empty());
    }
    #[test]
    fn strict_ceiling_leaves_output_unchanged_instead_of_falling_back_to_lower_floor() {
        let mut settings = input();
        settings.maximum_y = 0.5;
        let mut analyzer = StepGeometryAnalyzer::new(settings);
        let mut numeric = HostReckoningNumeric;
        let mut tuning = Tuning::default();
        analyzer
            .add_triangle(floor(0.0, 1), &mut numeric, &mut tuning)
            .unwrap();
        analyzer
            .add_triangle(floor(0.5, 2), &mut numeric, &mut tuning)
            .unwrap();
        let sentinel = StepGeometryResult {
            position: [9.0; 4],
            normal: [8.0; 4],
            surface: 123,
        };
        let mut result = sentinel;
        assert!(
            !analyzer
                .calculate(&mut result, &mut numeric, &mut tuning)
                .unwrap()
        );
        assert_eq!(result, sentinel);
    }
    #[test]
    fn rejected_triangles_do_not_use_capacity_and_full_capacity_skips_numeric_work() {
        let mut analyzer = StepGeometryAnalyzer::new(input());
        let mut numeric = HostReckoningNumeric;
        let mut tuning = Tuning::default();
        let mut rejected = floor(0.0, 99);
        rejected.vertices.swap(0, 1);
        for _ in 0..70 {
            analyzer
                .add_triangle(rejected, &mut numeric, &mut tuning)
                .unwrap();
        }
        assert_eq!(analyzer.triangle_count, 0);
        assert_eq!(tuning.reads.len(), 70);
        for surface in 0..70 {
            analyzer
                .add_triangle(floor(0.0, surface), &mut numeric, &mut tuning)
                .unwrap();
        }
        assert_eq!(analyzer.triangle_count, 64);
        assert_eq!(analyzer.good_count, 64);
        analyzer
            .add_triangle(rejected, &mut numeric, &mut tuning)
            .unwrap();
        assert_eq!(tuning.reads.len(), 70);
    }
    #[test]
    fn distance_ignores_all_three_y_values_and_selects_endpoint_outside_segment() {
        let mut numeric = HostReckoningNumeric;
        assert_eq!(
            segment_distance(
                [1.0, 90.0, 2.0, 1.0],
                [0.0, -40.0, 0.0, 1.0],
                [2.0, 10.0, 0.0, 1.0],
                &mut numeric
            )
            .unwrap(),
            2.0
        );
        assert_eq!(
            segment_distance(
                [5.0, 90.0, 4.0, 1.0],
                [0.0, -40.0, 0.0, 1.0],
                [2.0, 10.0, 0.0, 1.0],
                &mut numeric
            )
            .unwrap(),
            5.0
        );
    }
    #[test]
    fn failed_capacity_insert_still_replaces_original_and_skips_new_nodes() {
        let mut analyzer = StepGeometryAnalyzer::new(input());
        let mut numeric = HostReckoningNumeric;
        let mut tuning = Tuning::default();
        analyzer
            .add_triangle(
                StepGeometryTriangle {
                    vertices: [
                        [-1.0, -1.0, 0.0, 1.0],
                        [1.0, -1.0, 0.0, 1.0],
                        [0.0, 1.0, 0.0, 1.0],
                    ],
                    surface: 99,
                },
                &mut numeric,
                &mut tuning,
            )
            .unwrap();
        assert_eq!(analyzer.bad_count, 1);
        analyzer.points = [analyzer.points[0]; 8];
        analyzer.point_count = 8;
        for (i, point) in analyzer.points.iter_mut().enumerate() {
            point.result.surface = u32::try_from(i).unwrap();
        }
        analyzer.exclude_bad(&mut numeric, &mut tuning).unwrap();
        // Original0 had no free replacement node. Each later original had one.
        assert_eq!(analyzer.point_count, 7);
        assert_eq!(
            analyzer.points[..analyzer.point_count]
                .iter()
                .map(|p| p.result.surface)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6, 7]
        );
        assert!(
            analyzer.points[..7]
                .iter()
                .all(|p| p.result.position[2] > 0.2)
        );
        assert_eq!(
            tuning.reads.iter().filter(|&&s| s == "vertical").count(),
            24
        );
    }
}
