//! WorldShadowView's entity/island culling, debug sk82_na_m.xex.
//!
//! Both callers use DoFrustumCulling828589c0 with mAreaList=null and no
//! anti-frustums (82851144, 827d4248, 828572c0). This is that specialization,
//! not a replacement for scene/area/occluder culling. Authored min.W is the
//! radius input; recomputing it from XYZ changes minimum-pixel admission.
//! Host reciprocal estimates and VMX exceptional min selection remain the
//! existing numerical boundaries; this does not claim Xenon bit equality.

use crate::{
    reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric},
    simulation_drive_jacobian::refine_drive_reciprocal_sqrt,
};
use skate2_formats::render::AxisAlignedBox;

/// CalculateBoundsInfo827f2708, for the single RModel in a WorldPresEntity.
/// The source replaces min.W with half the four-lane diagonal length before
/// saving m_localBB. SetLocalToWorldMatrix does not transform this world box.
pub fn world_entity_bounds(mut bounds: AxisAlignedBox) -> Result<AxisAlignedBox, String> {
    let delta = std::array::from_fn(|i| bounds.maximum[i] - bounds.minimum[i]);
    let square = crate::reckoning_numeric::host_dot4(delta, delta);
    let seed = HostReckoningNumeric
        .reciprocal_sqrt_estimate(square)
        .map_err(|error| format!("world entity bounds: {error}"))?;
    let root = refine_drive_reciprocal_sqrt(square, seed);
    bounds.minimum[3] = if square == 0.0 { 0.0 } else { square * root } * 0.5;
    Ok(bounds)
}

/// Frustum rendering layout +0x28/+0x54, read at828572ac/828572bc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldShadowCullSettings {
    pub minimum_pixels: f32,
    pub extra_radius: f32,
}

/// Normalized source planes and the position actually passed to ComputeFrustum.
/// In RenderShadows_CSM this position is the preliminary matrix's snapped
/// translation (82857244..64), not the gameplay camera's world position.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldShadowFrustum {
    planes: [[f32; 4]; 6],
    view_position: [f32; 3],
    settings: WorldShadowCullSettings,
}

impl WorldShadowFrustum {
    /// Mathematical clip rows, with the final row producing clip W.
    /// ComputeFrustum8282bd28 uses w+x,w-x,w-y,w+y,w+z,w-z, including
    /// the symmetric Z plane (not the host rasterizer's z>=0 plane).
    pub fn try_new(
        clip_rows: [[f32; 4]; 4],
        view_position: [f32; 3],
        settings: WorldShadowCullSettings,
    ) -> Result<Self, String> {
        let [x, y, z, w] = clip_rows;
        let add = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| a[i] + b[i]);
        let sub = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| a[i] - b[i]);
        let mut planes = [
            add(x, w),
            sub(w, x),
            sub(w, y),
            add(w, y),
            add(w, z),
            sub(w, z),
        ];
        let mut numeric = HostReckoningNumeric;
        for plane in &mut planes {
            *plane = normalize_plane(*plane, &mut numeric)
                .map_err(|error| format!("world-shadow plane normalization: {error}"))?;
        }
        Ok(Self {
            planes,
            view_position,
            settings,
        })
    }

    /// Returns the source projected-size value, or zero if rejected. Native
    /// entity/island consumers select a draw when this value is nonzero.
    pub fn try_cull(&self, bounds: &AxisAlignedBox) -> Result<f32, String> {
        self.cull_with_numeric(bounds, &mut HostReckoningNumeric)
            .map_err(|error| format!("world-shadow culling: {error}"))
    }

    fn cull_with_numeric<N: ReckoningNumeric>(
        &self,
        bounds: &AxisAlignedBox,
        numeric: &mut N,
    ) -> Result<f32, N::Error> {
        let centre: [f32; 3] =
            std::array::from_fn(|i| (bounds.minimum[i] + bounds.maximum[i]) * 0.5);
        // Source subtracts the rounded centre; (max-min)*.5 is not identical.
        let half: [f32; 3] = std::array::from_fn(|i| bounds.maximum[i] - centre[i]);
        let distances = self.planes.map(|plane| {
            let distance = centre[0].mul_add(
                plane[0],
                centre[1].mul_add(plane[1], centre[2].mul_add(plane[2], plane[3])),
            );
            let support = half[0].mul_add(
                plane[0].abs(),
                half[1].mul_add(plane[1].abs(), half[2] * plane[2].abs()),
            );
            distance + support
        });
        let closest = distances[0]
            .min(distances[1])
            .min(distances[2].min(distances[3]))
            .min(distances[4].min(distances[5]));
        let delta: [f32; 3] = std::array::from_fn(|i| centre[i] - self.view_position[i]);
        let square = delta[0].mul_add(delta[0], delta[1].mul_add(delta[1], delta[2] * delta[2]));
        // 82858f38/58: bare rsqrt estimate followed by bare reciprocal estimate.
        // Do not replace this with the refined try_length service.
        let inverse_distance = numeric.reciprocal_sqrt_estimate(square)?;
        let distance = numeric.reciprocal_estimate(inverse_distance)?;
        let radius = bounds.minimum[3] + self.settings.extra_radius;
        let gap = distance - radius;
        let seed = numeric.reciprocal_estimate(gap)?;
        // One Newton step at82858f78/80; the shared drive helper does TWO.
        let reciprocal = seed.mul_add((-seed).mul_add(gap, 1.0), seed);
        let view_height = f32::from_bits(0x44ba_3d71);
        let projected = (radius * view_height) * reciprocal;
        let pixels = if gap >= radius {
            projected
        } else {
            view_height
        };
        let accepted = if pixels >= self.settings.minimum_pixels {
            pixels
        } else {
            0.0
        };
        Ok(if closest >= 0.0 { accepted } else { 0.0 })
    }
}

/// NormalizePlane8282bc60: Y square, X FMA, Z FMA, two rsqrt refinements,
/// square*inverse-root (zero selected for square==0), then scalar reciprocal.
pub(crate) fn normalize_plane<N: ReckoningNumeric>(
    plane: [f32; 4],
    numeric: &mut N,
) -> Result<[f32; 4], N::Error> {
    let square = plane[2].mul_add(plane[2], plane[0].mul_add(plane[0], plane[1] * plane[1]));
    let seed = numeric.reciprocal_sqrt_estimate(square)?;
    let refined = refine_drive_reciprocal_sqrt(square, seed);
    let length = if square == 0.0 { 0.0 } else { square * refined };
    let reciprocal = 1.0 / length;
    Ok(plane.map(|value| value * reciprocal))
}

/// WorldShadowView.Cull82851100..82851348 updates Y/Z from admitted ENTITY
/// boxes (even when all of their mesh islands are later rejected), then pads
/// by fifteen percent. X/W retain the native initial values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldShadowBounds {
    pub min: [f32; 4],
    pub max: [f32; 4],
}

impl Default for WorldShadowBounds {
    fn default() -> Self {
        Self {
            min: [f32::MAX, f32::MAX, f32::MAX, 0.0],
            max: [-f32::MAX, -f32::MAX, -f32::MAX, 0.0],
        }
    }
}

impl WorldShadowBounds {
    pub fn include(&mut self, bounds: &AxisAlignedBox) {
        for lane in [1, 2] {
            if self.min[lane] > bounds.minimum[lane] {
                self.min[lane] = bounds.minimum[lane];
            }
            if bounds.maximum[lane] > self.max[lane] {
                self.max[lane] = bounds.maximum[lane];
            }
        }
    }

    #[must_use]
    pub fn padded(self) -> Self {
        let extent: [f32; 4] =
            std::array::from_fn(|i| (self.max[i] - self.min[i]) * f32::from_bits(0x3e19_999a));
        Self {
            min: std::array::from_fn(|i| self.min[i] - extent[i]),
            max: std::array::from_fn(|i| self.max[i] + extent[i]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_frustum(minimum_pixels: f32) -> WorldShadowFrustum {
        WorldShadowFrustum::try_new(
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            [0.0; 3],
            WorldShadowCullSettings {
                minimum_pixels,
                extra_radius: 0.0,
            },
        )
        .unwrap()
    }

    #[test]
    fn uses_authored_radius_and_symmetric_near_plane() {
        let frustum = unit_frustum(1.0);
        let mut bounds = AxisAlignedBox {
            minimum: [-0.1, -0.1, -0.6, 1.0],
            maximum: [0.1, 0.1, -0.4, 999.0],
        };
        assert_eq!(
            frustum.try_cull(&bounds).unwrap(),
            f32::from_bits(0x44ba_3d71)
        );
        bounds.minimum[3] = 0.0;
        assert_eq!(frustum.try_cull(&bounds).unwrap(), 0.0);
    }

    #[test]
    fn touches_plane_and_pixel_threshold_are_inclusive() {
        let frustum = unit_frustum(f32::from_bits(0x44ba_3d71));
        let mut bounds = AxisAlignedBox {
            minimum: [1.0, -0.1, -0.1, 2.0],
            maximum: [1.2, 0.1, 0.1, 0.0],
        };
        assert!(frustum.try_cull(&bounds).unwrap() > 0.0);
        bounds.minimum[0] = 1.1;
        assert_eq!(frustum.try_cull(&bounds).unwrap(), 0.0);
    }

    #[test]
    fn depth_bounds_use_entity_height_and_fifteen_percent_padding() {
        let mut bounds = WorldShadowBounds::default();
        bounds.include(&AxisAlignedBox {
            minimum: [-99.0, 10.0, 20.0, 1.0],
            maximum: [99.0, 30.0, 40.0, 0.0],
        });
        let padded = bounds.padded();
        assert_eq!([padded.min[1], padded.max[1]], [7.0, 33.0]);
        assert_eq!([padded.min[2], padded.max[2]], [17.0, 43.0]);
        assert_eq!(padded.min[0], f32::INFINITY);
        assert_eq!(padded.max[0], f32::NEG_INFINITY);
        assert_eq!(padded.min[3], 0.0);
    }
}
