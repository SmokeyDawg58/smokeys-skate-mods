//! SceneRenderView's optimesh island area rule, debug sk82_na_m.xex.
//!
//! `WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8 culls every visible
//! entity's islands under `kCullOptimeshIslands` (default on) through
//! `Frustum::Cull_VMX_NEW` 0x82825948 and `DoFrustumCulling` 0x828589c0, with
//! the RModel's `m_pIslandAreas` as `mAreaList` (ctor 0x827d4040; the shadow
//! cull passes none). After the frustum test, a group of four islands whose
//! area W lanes are not all zero drops each island whose nearest box corner
//! lies at least `minDist` beyond the near plane and whose view-projected area,
//! scaled by `optimalCullDist` over that depth squared, is below `minAreaCull`
//! (82859234..82859304). The shipped `minPixelSize` is zero, so the projected
//! pixel test keeps everything. Host reciprocal estimates stand in for Xenon
//! `vrefp`; this does not claim Xenon bit equality.

use crate::{
    presentation_shadow_culling::normalize_plane,
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
};
use skate2_formats::render::AxisAlignedBox;

/// The `rendering` frustum collection values `Frustum::UpdateCullParams`
/// 0x82853970 copies: layout +0x30 `minDist`, +0x34 `minAreaCull` and +0x1c
/// `optimalCullDist`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IslandAreaCullSettings {
    pub min_dist: f32,
    pub min_area: f32,
    pub optimal_dist: f32,
}

/// One view's area rule: its normalized near plane and the absolute
/// components of `mViewDirection`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IslandAreaCull {
    near: [f32; 4],
    direction: [f32; 3],
    min_dist: f32,
    min_area: f32,
    inverse_optimal_dist: f32,
}

impl IslandAreaCull {
    /// Mathematical clip rows, with the final row producing clip W.
    /// `ComputeFrustum` 0x8282bd28 makes plane four from w+z; `UpdateCamera`
    /// 0x82848b88 normalizes it. `view_direction` is `mViewDirection`, the
    /// view matrix's Z column (`UpdateCullParams` 82853ab4..ad8).
    pub fn try_new(
        clip_rows: [[f32; 4]; 4],
        view_direction: [f32; 3],
        settings: IslandAreaCullSettings,
    ) -> Result<Self, String> {
        let [_, _, z, w] = clip_rows;
        let near = normalize_plane(std::array::from_fn(|i| w[i] + z[i]), &mut HostReckoningNumeric)
            .map_err(|error| format!("scene island near plane: {error}"))?;
        let mut numeric = HostReckoningNumeric;
        // 82858bb8..be0: reciprocal estimate, one Newton step.
        let seed = numeric
            .reciprocal_estimate(settings.optimal_dist)
            .map_err(|error| format!("scene island optimal distance: {error}"))?;
        let inverse_optimal_dist = seed.mul_add((-seed).mul_add(settings.optimal_dist, 1.0), seed);
        Ok(Self {
            near,
            direction: view_direction.map(f32::abs),
            min_dist: settings.min_dist,
            min_area: settings.min_area,
            inverse_optimal_dist,
        })
    }

    /// Whether the island at `island` of a model's area table survives the
    /// rule, for its authored `bounds`. The source tests four islands at a
    /// time and skips the rule when all four W lanes are zero; lanes past the
    /// table are absent here.
    pub fn keeps(
        &self,
        bounds: &AxisAlignedBox,
        areas: &[[f32; 4]],
        island: usize,
    ) -> Result<bool, String> {
        let group = island - island % 4;
        if areas
            .iter()
            .skip(group)
            .take(4)
            .all(|area| area[3].to_bits() == 0)
        {
            return Ok(true);
        }
        let area = areas
            .get(island)
            .ok_or("scene island area beyond the model's table")?;
        let corner = |axis: usize| {
            if self.near[axis] >= 0.0 {
                bounds.minimum[axis]
            } else {
                bounds.maximum[axis]
            }
        };
        // 828592a8..dc: Z product, Y then X fused, plus the plane's W, at least one.
        let depth = corner(0)
            .mul_add(
                self.near[0],
                corner(1).mul_add(self.near[1], corner(2) * self.near[2]),
            )
            + self.near[3];
        let depth = depth.max(1.0);
        if depth < self.min_dist {
            return Ok(true);
        }
        // 8285927c..cc: Z product, then Y and X fused.
        let projected = area[0].mul_add(
            self.direction[0],
            area[1].mul_add(self.direction[1], area[2] * self.direction[2]),
        );
        let square = depth * (depth * self.inverse_optimal_dist);
        let seed = HostReckoningNumeric
            .reciprocal_estimate(square)
            .map_err(|error| format!("scene island depth: {error}"))?;
        let reciprocal = seed.mul_add((-seed).mul_add(square, 1.0), seed);
        Ok(projected * reciprocal >= self.min_area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: IslandAreaCullSettings = IslandAreaCullSettings {
        min_dist: 120.0,
        min_area: 0.2,
        optimal_dist: 900.0,
    };

    /// A view down +Z: clip W is the camera Z, so the near plane is z >= 0.
    fn forward() -> IslandAreaCull {
        IslandAreaCull::try_new(
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            [0.0, 0.0, 1.0],
            SETTINGS,
        )
        .unwrap()
    }

    fn island_at(depth: f32) -> AxisAlignedBox {
        AxisAlignedBox {
            minimum: [-1.0, -1.0, depth, 0.0],
            maximum: [1.0, 1.0, depth + 2.0, 0.0],
        }
    }

    #[test]
    fn near_islands_and_large_far_islands_stay() {
        let cull = forward();
        let small = [[0.0, 0.0, 1.0, 1.0]];
        assert!(cull.keeps(&island_at(100.0), &small, 0).unwrap());
        assert!(!cull.keeps(&island_at(200.0), &small, 0).unwrap());
        let large = [[0.0, 0.0, 10.0, 1.0]];
        assert!(cull.keeps(&island_at(200.0), &large, 0).unwrap());
    }

    #[test]
    fn the_nearest_corner_sets_the_depth() {
        let cull = forward();
        let small = [[0.0, 0.0, 1.0, 1.0]];
        let spanning = AxisAlignedBox {
            minimum: [-1.0, -1.0, 110.0, 0.0],
            maximum: [1.0, 1.0, 400.0, 0.0],
        };
        assert!(cull.keeps(&spanning, &small, 0).unwrap());
    }

    #[test]
    fn a_group_with_zero_w_lanes_skips_the_rule() {
        let cull = forward();
        let mut areas = [[0.0, 0.0, 1.0, 0.0]; 5];
        assert!(cull.keeps(&island_at(200.0), &areas, 1).unwrap());
        areas[3][3] = 2.0;
        assert!(!cull.keeps(&island_at(200.0), &areas, 1).unwrap());
        assert!(cull.keeps(&island_at(200.0), &areas, 4).unwrap());
    }
}
