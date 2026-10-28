//! `Sk8::Challenge::InvisibleBoundaryManager` (`challengeinvisibleboundary.cpp`): the polygons an
//! online game keeps its skaters in. `GameModule::GameLoad` 0x8273be50 loads its challenge's
//! (`LoadBoundary` 0x82db0e78, `data\boundary\<challenges.Name>.xml`); `Update` 0x82da72f0 posts
//! the `cMsgOutOfBoundaryAmount` the screen's snow shows near an edge, and `IsPositionOk`
//! 0x82da4d30 answers `Simulation::TestForNetworkGameCollisionConstraints` 0x8289e510: `PlayerUI`
//! sends a skater outside back to a breadcrumb, and breadcrumbs are kept only well inside.

use skate2_formats::mount::MountSet;

/// `kBoundaryMinWarningDistance`: the snow starts this near an edge.
const MIN_WARNING_DISTANCE: f32 = 13.0;
/// `kBoundaryMaxWarningDistance`: and is at its strongest this near.
const MAX_WARNING_DISTANCE: f32 = 5.0;
/// `kBoundaryMinWarningAmount`.
const MIN_WARNING_AMOUNT: f32 = 0.1;
/// `kBoundaryMaxWarningAmount`.
const MAX_WARNING_AMOUNT: f32 = 0.5;

/// The loaded boundaries, each a polygon in x, z.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InvisibleBoundaries {
    boundaries: Vec<Vec<[f32; 2]>>,
}

impl InvisibleBoundaries {
    /// `LoadBoundary` of the challenge named `name`: `InvisibleBoundaryLoader` 0x82da6f48 reads
    /// `data\boundary\<name>.xml`, and a file that is missing or does not load leaves the
    /// challenge without one (`OnComplete` 0x82da71b0).
    pub(crate) fn load(mounts: &MountSet, name: &str) -> Self {
        let boundary = mounts
            .read_path(&format!("data\\boundary\\{name}.xml"))
            .ok()
            .and_then(|bytes| skate2_formats::boundary_xml::parse(&bytes).ok())
            .filter(|points| points.len() > 2);
        Self { boundaries: boundary.into_iter().collect() }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.boundaries.is_empty()
    }

    /// `IsPositionOk`: inside every boundary and at least `radius` from its edges; a negative
    /// radius is `kBoundaryMinWarningDistance`.
    #[must_use]
    pub(crate) fn is_position_ok(&self, position: [f32; 3], radius: f32) -> bool {
        let radius_squared = if radius < 0.0 { MIN_WARNING_DISTANCE * MIN_WARNING_DISTANCE } else { radius * radius };
        let point = [position[0], position[2]];
        let mut nearest = f32::MAX;
        for boundary in &self.boundaries {
            if !is_inside(point, boundary) {
                return false;
            }
            nearest = nearest.min(distance_squared(point, boundary));
        }
        nearest >= radius_squared
    }

    /// `StateOnlineFreeskateHere::LoadBoundary` 0x8274e7e0's test of a location for a skater at
    /// `position`: `None` inside (`IsInside`), else the squared distance to the nearest edge
    /// (`DistanceSquared`).
    #[must_use]
    pub(crate) fn outside_distance_squared(&self, position: [f32; 3]) -> Option<f32> {
        let point = [position[0], position[2]];
        if self.boundaries.iter().all(|boundary| is_inside(point, boundary)) {
            return None;
        }
        Some(self.boundaries.iter().fold(f32::MAX, |nearest, boundary| nearest.min(distance_squared(point, boundary))))
    }

    /// `Update`'s `cMsgOutOfBoundaryAmount` for the local skater at `position`: none farther
    /// than `kBoundaryMinWarningDistance` from every edge, else rising from
    /// `kBoundaryMinWarningAmount` there to `kBoundaryMaxWarningAmount` at
    /// `kBoundaryMaxWarningDistance` and nearer.
    #[must_use]
    pub(crate) fn warning_amount(&self, position: [f32; 3]) -> Option<f32> {
        let point = [position[0], position[2]];
        let nearest = self.boundaries.iter().fold(f32::MAX, |nearest, boundary| nearest.min(distance_squared(point, boundary)));
        if nearest >= MIN_WARNING_DISTANCE * MIN_WARNING_DISTANCE {
            return None;
        }
        let distance = nearest.sqrt();
        if distance <= MAX_WARNING_DISTANCE {
            return Some(MAX_WARNING_AMOUNT);
        }
        let along = (distance - MAX_WARNING_DISTANCE) / (MIN_WARNING_DISTANCE - MAX_WARNING_DISTANCE);
        Some(MAX_WARNING_AMOUNT - along * (MAX_WARNING_AMOUNT - MIN_WARNING_AMOUNT))
    }
}

/// `DistanceSquared` 0x82da3110: the squared distance from `point` to the nearest of the
/// polygon's edges, the last closing back to the first. A degenerate edge measures to its point.
fn distance_squared(point: [f32; 2], boundary: &[[f32; 2]]) -> f32 {
    let mut nearest = f32::MAX;
    for (index, &start) in boundary.iter().enumerate() {
        let end = boundary[(index + 1) % boundary.len()];
        let edge = [end[0] - start[0], end[1] - start[1]];
        let offset = [point[0] - start[0], point[1] - start[1]];
        let length_squared = edge[0] * edge[0] + edge[1] * edge[1];
        let along = if length_squared > 0.0 { ((offset[0] * edge[0] + offset[1] * edge[1]) / length_squared).clamp(0.0, 1.0) } else { 0.0 };
        let closest = [edge[0].mul_add(along, start[0]), edge[1].mul_add(along, start[1])];
        let away = [point[0] - closest[0], point[1] - closest[1]];
        nearest = nearest.min(away[0] * away[0] + away[1] * away[1]);
    }
    nearest
}

/// `IsInside` 0x82da3220: the crossing count of a ray towards +x, each edge counted once over
/// its z range (open below, closed above) and only where `point` lies left of the crossing.
fn is_inside(point: [f32; 2], boundary: &[[f32; 2]]) -> bool {
    let mut inside = false;
    for (index, &start) in boundary.iter().enumerate() {
        let end = boundary[(index + 1) % boundary.len()];
        if point[1] > start[1].min(end[1]) && point[1] <= start[1].max(end[1]) && point[0] <= start[0].max(end[0]) && start[1] != end[1] {
            let crossing = (point[1] - start[1]) * (end[0] - start[0]) / (end[1] - start[1]) + start[0];
            if start[0] == end[0] || point[0] <= crossing {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::InvisibleBoundaries;

    fn square() -> InvisibleBoundaries {
        InvisibleBoundaries { boundaries: vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]] }
    }

    #[test]
    fn a_location_holds_the_skater_or_measures_how_far_it_is() {
        assert_eq!(square().outside_distance_squared([50.0, 9.0, 50.0]), None);
        assert_eq!(square().outside_distance_squared([110.0, 0.0, 50.0]), Some(100.0));
    }

    #[test]
    fn the_snow_rises_towards_an_edge() {
        let area = square();
        assert_eq!(area.warning_amount([50.0, 3.0, 50.0]), None);
        assert_eq!(area.warning_amount([50.0, 3.0, 87.0]), None, "13 m from the edge");
        let at_nine = area.warning_amount([50.0, 3.0, 91.0]).unwrap();
        assert!((at_nine - 0.3).abs() < 1.0e-5, "halfway between 13 m and 5 m: {at_nine}");
        assert_eq!(area.warning_amount([50.0, 3.0, 97.0]), Some(0.5));
        assert_eq!(area.warning_amount([50.0, 3.0, 103.0]), Some(0.5), "just outside");
    }

    #[test]
    fn a_position_outside_or_near_the_edge_is_not_ok() {
        let area = square();
        assert!(area.is_position_ok([50.0, 0.0, 50.0], 0.0));
        assert!(area.is_position_ok([1.0, 0.0, 1.0], 0.0));
        assert!(!area.is_position_ok([-1.0, 0.0, 50.0], 0.0));
        assert!(!area.is_position_ok([50.0, 0.0, 101.0], 0.0));
        assert!(!area.is_position_ok([50.0, 0.0, 90.0], -1.0), "a breadcrumb stays 13 m inside");
        assert!(area.is_position_ok([50.0, 0.0, 80.0], -1.0));
        assert!(InvisibleBoundaries::default().is_position_ok([1.0e6, 0.0, 1.0e6], 0.0), "no boundary holds no one");
    }

    #[test]
    fn a_concave_boundary_counts_crossings() {
        let area = InvisibleBoundaries {
            boundaries: vec![vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [50.0, 50.0], [0.0, 100.0]]],
        };
        assert!(area.is_position_ok([50.0, 0.0, 25.0], 0.0));
        assert!(!area.is_position_ok([50.0, 0.0, 75.0], 0.0), "inside the notch");
        assert!(area.is_position_ok([90.0, 0.0, 75.0], 0.0));
    }
}
