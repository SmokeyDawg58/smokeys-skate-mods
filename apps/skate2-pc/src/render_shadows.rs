//! Light-space cascades of the original dynamic shadows.
//!
//! `PostFX::RenderShadows_CSM` 0x82855888 (debug `sk82_na_m.xex`) runs every
//! frame after `RenderMan::UpdateRenderParams`, from `RenderModeScene`
//! 0x82857b40. On the gameplay path (not in an NIS, no front-end CSM
//! attributes, `RenderMan::m_bInLooseWorld` clear, a local skater present):
//!
//! - Light basis from `g_vLightDir`: forward `f = normalize(-L)`, side
//!   `s = normalize(-f.z, 0, f.x)`, up `u = normalize(f x s)`.
//! - Cascade `c` has the half-extent `S = ScaleLevel[c] * scale`, with
//!   `ScaleLevel` 0x82ff023c = {1, 4, 16, 16} and `scale` the `rendering`
//!   collection 0xb9a92367e7ac68b9 (`PostFX::m_CSMAttribs`) attribute
//!   0xc1e51168dd06bd38.
//! - Its centre is the camera position plus `(camera z axis * S) * offset`
//!   per lane (one `vmaddfp`), `offset` being attribute 0x62c03852c5f4d2ab.
//! - `rw::math::vpu::Inverse` of the rows `[s; u; f; centre]` scaled by `1/S`
//!   gives the x and y rows; their translation is snapped with
//!   `floor(v * 256) / 256`, one texel of the 512-texel cascade.
//! - The z row is replaced by `(0, -1/R, 0, snap(maxH / R))`: the depth is
//!   the height below `maxH = m_BB.m_Max.y + 2` of the local
//!   `SkaterPresEntity`, over `R` = layout +0x24 `nis_depth_precision`.
//! - The x, y and z rows are `CSM_Mat_Row0/1/2[c]`. After both cascades
//!   0x828578a4..7994 stores into `CSM_Mat_Row0[1]` the scale (xy) and
//!   offset (zw) taking cascade 0 light space to cascade 1.
//! - `ShadowView::Cull` 0x82850ac0 keeps an entity for a cascade when
//!   `IsInLightView` 0x82829b88 is not `eCompletelyOutside` and its
//!   `GetPosition` (the hips) is within `maxShadowDistSquared` (1300) of the
//!   camera; one completely inside cascade 0 is erased from the visible map
//!   before cascade 1 culls.

use crate::gpu_renderer::ShadowCascades;
use skate2_core::presentation_shadow_culling::{
    WorldShadowBounds, WorldShadowCullSettings, WorldShadowFrustum,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldShadowAttributes {
    pub(crate) scale: f32,
    pub(crate) offset: [f32; 3],
    pub(crate) cull: WorldShadowCullSettings,
    pub(crate) enabled: bool,
}

/// World pass of RenderShadows_CSM82856d54..828576bc. Kept separate from
/// the dynamic cascades: it has its own scale and entity-fitted height range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WorldShadowProjection {
    pub(crate) preliminary: [[f32; 4]; 3],
    pub(crate) settings: WorldShadowCullSettings,
}

impl WorldShadowProjection {
    pub(crate) fn new(
        light: [f32; 4],
        attributes: &WorldShadowAttributes,
        inputs: &ShadowCasterInputs,
    ) -> Self {
        let forward = normalized([-light[0], -light[1], -light[2]]);
        let side = normalized([-forward[2], 0.0, forward[0]]);
        let up = normalized([
            forward[1] * side[2],
            (-forward[0]).mul_add(side[2], forward[2] * side[0]),
            -(forward[1] * side[0]),
        ]);
        let scale = attributes.scale * 16.0; // ScaleLevel[3], 82ff0248.
        let centre: [f32; 3] = std::array::from_fn(|i| {
            (inputs.camera_forward[i] * scale)
                .mul_add(attributes.offset[i], inputs.camera_position[i])
        });
        let inverse = inverse([
            [side[0], side[1], side[2], 0.0],
            [up[0], up[1], up[2], 0.0],
            [forward[0], forward[1], forward[2], 0.0],
            [centre[0], centre[1], centre[2], 1.0],
        ]);
        let factors = [1.0 / scale, 1.0 / scale, f32::from_bits(0x3a83_126f)];
        Self {
            preliminary: std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    let value = inverse[column][row] * factors[row];
                    if column == 3 { snap(value) } else { value }
                })
            }),
            settings: attributes.cull,
        }
    }

    pub(crate) fn frustum(&self) -> Result<WorldShadowFrustum, String> {
        let [x, y, z] = self.preliminary;
        WorldShadowFrustum::try_new(
            [x, y, z, [0.0, 0.0, 0.0, 1.0]],
            [x[3], y[3], z[3]],
            self.settings,
        )
    }

    pub(crate) fn fitted_rows(&self, bounds: WorldShadowBounds) -> [[f32; 4]; 3] {
        let bounds = bounds.padded();
        let range = bounds.max[1] - bounds.min[1];
        let depth = if range.abs() <= 0.0 {
            [0.0, 0.0, 1.0, 0.0]
        } else {
            let reciprocal = 1.0 / range;
            [0.0, -reciprocal, 0.0, snap(bounds.max[1] * reciprocal)]
        };
        [self.preliminary[0], self.preliminary[1], depth]
    }
}

/// `ScaleLevel` 0x82ff023c; the cascades read entries 0 and 1.
const SCALE_LEVEL: [f32; 2] = [1.0, 4.0];
/// `maxShadowDistSquared` 0x82feffdc.
const MAX_SHADOW_DISTANCE_SQUARED: f32 = 1300.0;
/// `RenderShadows_CSM` 0x82855cd4 adds 2.0 to the skater's bounding-box top.
const TOP_MARGIN: f32 = 2.0;

/// `PostFX::m_CSMAttribs` inputs (`rendering` 0xb9a92367e7ac68b9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CsmAttributes {
    /// Attribute 0xc1e51168dd06bd38: cascade 0 half-extent in metres.
    pub(crate) scale: f32,
    /// Attribute 0x62c03852c5f4d2ab: how far along the camera axis each
    /// cascade centre sits, in cascade half-extents, per lane.
    pub(crate) offset: [f32; 3],
    /// Layout +0x24 `nis_depth_precision`: the depth range in metres.
    pub(crate) depth_range: f32,
}

/// This frame's camera and local skater, as `RenderShadows_CSM` reads them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShadowCasterInputs {
    /// `CameraPresentation::GetMatrix` row 3.
    pub(crate) camera_position: [f32; 3],
    /// `CameraPresentation::GetMatrix` row 2.
    pub(crate) camera_forward: [f32; 3],
    /// `SkaterPresEntity::GetPosition` 0x827d79c0 (`GetHipsPos`).
    pub(crate) skater_hips: [f32; 3],
    /// The skater's world `m_BB`, min then max.
    pub(crate) skater_bounds: [[f32; 3]; 2],
}

/// `IsInLightView` result (`eShadowCullType`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LightView {
    CompletelyOutside,
    CompletelyInside,
    Clipping,
}

/// Both cascades for this frame's light, camera and skater.
pub(crate) fn cascades(
    light_direction: [f32; 4],
    attributes: &CsmAttributes,
    inputs: &ShadowCasterInputs,
) -> ShadowCascades {
    cascades_with_top(light_direction, attributes, inputs, inputs.skater_bounds[1][1] + TOP_MARGIN)
}

/// RenderShadows_CSM82855888 disables the local-player shortcut for FE/loose
/// worlds. Get_BBOX_Height82829de8 pads the listed caster height by 15%; the
/// camera's m_FEAttribsCSM supplies scale, offset and depth precision.
pub(crate) fn frontend_cascades(
    light_direction: [f32; 4], attributes: &CsmAttributes, inputs: &ShadowCasterInputs,
) -> ShadowCascades {
    let [minimum, maximum] = inputs.skater_bounds;
    let top = (maximum[1] - minimum[1]) * f32::from_bits(0x3e19999a) + maximum[1];
    let mut selected = *attributes;
    selected.depth_range = top - (top - attributes.depth_range);
    cascades_with_top(light_direction, &selected, inputs, top)
}

fn cascades_with_top(
    light_direction: [f32; 4], attributes: &CsmAttributes,
    inputs: &ShadowCasterInputs, max_height: f32,
) -> ShadowCascades {
    let forward = normalized([
        -light_direction[0],
        -light_direction[1],
        -light_direction[2],
    ]);
    let side = normalized([-forward[2], 0.0, forward[0]]);
    // f x s with s.y = 0; the y lane is one `vnmsubfp`.
    let up = normalized([
        forward[1] * side[2],
        (-forward[0]).mul_add(side[2], forward[2] * side[0]),
        -(forward[1] * side[0]),
    ]);
    // 82856434..82856530 explicitly handles zero depth precision. Several
    // shipped CAC close-up collections use it; retain world Z, not NaN/Inf.
    let depth_row = if attributes.depth_range.abs() <= 0.0 {
        [0.0, 0.0, 1.0, 0.0]
    } else {
        let depth_scale = 1.0 / attributes.depth_range;
        [0.0, -depth_scale, 0.0, snap(depth_scale * max_height)]
    };
    let rows = SCALE_LEVEL.map(|level| {
        let scale = level * attributes.scale;
        let centre: [f32; 3] = std::array::from_fn(|lane| {
            (inputs.camera_forward[lane] * scale)
                .mul_add(attributes.offset[lane], inputs.camera_position[lane])
        });
        let light = inverse([
            [side[0], side[1], side[2], 0.0],
            [up[0], up[1], up[2], 0.0],
            [forward[0], forward[1], forward[2], 0.0],
            [centre[0], centre[1], centre[2], 1.0],
        ]);
        let extent = 1.0 / scale;
        let row = |column: usize| {
            [
                extent * light[0][column],
                extent * light[1][column],
                extent * light[2][column],
                snap(extent * light[3][column]),
            ]
        };
        [row(0), row(1), depth_row]
    });
    ShadowCascades {
        second_from_first: second_from_first(&rows[0], &rows[1]),
        casts: caster_cascades(
            &rows,
            inputs.skater_hips,
            inputs.skater_bounds,
            inputs.camera_position,
        ),
        rows,
    }
}

/// `ShadowView::Cull` 0x82850ac0 for one `eShadowView` entity: it casts into
/// a cascade whose `IsInLightView` of its world `m_BB` is not completely
/// outside, within `maxShadowDistSquared` of the camera by its `GetPosition`;
/// one completely inside cascade 0 is erased before cascade 1 culls.
pub(crate) fn caster_cascades(
    rows: &[[[f32; 4]; 3]; 2],
    position: [f32; 3],
    bounds: [[f32; 3]; 2],
    camera: [f32; 3],
) -> [bool; 2] {
    let near = distance_squared(position, camera) < MAX_SHADOW_DISTANCE_SQUARED;
    let first = light_view(&rows[0], &bounds);
    let second = light_view(&rows[1], &bounds);
    [
        near && first != LightView::CompletelyOutside,
        near && first != LightView::CompletelyInside && second != LightView::CompletelyOutside,
    ]
}

/// `floor(v * 256) * (1/256)`, the translation snap (`f28`/`f27`).
fn snap(value: f32) -> f32 {
    (value * 256.0).floor() * 0.003_906_25
}

fn normalized(vector: [f32; 3]) -> [f32; 3] {
    let inverse_length = 1.0
        / vector[2]
            .mul_add(
                vector[2],
                vector[1].mul_add(vector[1], vector[0] * vector[0]),
            )
            .sqrt();
    vector.map(|lane| lane * inverse_length)
}

fn distance_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta: [f32; 3] = std::array::from_fn(|lane| a[lane] - b[lane]);
    delta[2].mul_add(delta[2], delta[1].mul_add(delta[1], delta[0] * delta[0]))
}

/// 0x828578a4..7994: cascade 1's rows times the inverse of cascade 0's
/// (each completed with the row (0, 0, 0, 1)), accumulated row element 0
/// first; the receivers keep the diagonal xy scale and the zw offset.
fn second_from_first(first: &[[f32; 4]; 3], second: &[[f32; 4]; 3]) -> [f32; 4] {
    let back = inverse([first[0], first[1], first[2], [0.0, 0.0, 0.0, 1.0]]);
    let product = |row: [f32; 4], column: usize| {
        back[3][column].mul_add(
            row[3],
            back[2][column].mul_add(
                row[2],
                back[1][column].mul_add(row[1], back[0][column] * row[0]),
            ),
        )
    };
    [
        product(second[0], 0),
        product(second[1], 1),
        product(second[0], 3),
        product(second[1], 3),
    ]
}

/// `IsInLightView` 0x82829b88: the eight corners' light x and y against
/// the unit square. Outside when every corner is past one edge, inside when
/// no corner is past any edge.
fn light_view(rows: &[[f32; 4]; 3], bounds: &[[f32; 3]; 2]) -> LightView {
    let mut all_past = [true; 4];
    let mut any_past = false;
    for corner in 0..8_usize {
        let point = [
            bounds[corner & 1][0],
            bounds[(corner >> 1) & 1][1],
            bounds[(corner >> 2) & 1][2],
        ];
        let light = |row: [f32; 4]| {
            row[3].mul_add(
                1.0,
                row[2].mul_add(point[2], row[1].mul_add(point[1], row[0] * point[0])),
            )
        };
        let (x, y) = (light(rows[0]), light(rows[1]));
        let past = [-1.0 > x, x > 1.0, -1.0 > y, y > 1.0];
        for (all, side) in all_past.iter_mut().zip(past) {
            *all &= side;
        }
        any_past |= past.contains(&true);
    }
    if all_past.contains(&true) {
        LightView::CompletelyOutside
    } else if any_past {
        LightView::Clipping
    } else {
        LightView::CompletelyInside
    }
}

/// General 4x4 inverse by cofactors (row-major), as `rw::math::vpu::Inverse`
/// computes for any matrix; a singular input yields non-finite lanes.
fn inverse(m: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let a = |row: usize, column: usize| m[row][column];
    let minor = |r: [usize; 3], c: [usize; 3]| {
        a(r[0], c[0]) * (a(r[1], c[1]) * a(r[2], c[2]) - a(r[1], c[2]) * a(r[2], c[1]))
            - a(r[0], c[1]) * (a(r[1], c[0]) * a(r[2], c[2]) - a(r[1], c[2]) * a(r[2], c[0]))
            + a(r[0], c[2]) * (a(r[1], c[0]) * a(r[2], c[1]) - a(r[1], c[1]) * a(r[2], c[0]))
    };
    let others = |skip: usize| {
        let mut rest = [0; 3];
        let mut next = 0;
        for index in (0..4).filter(|&index| index != skip) {
            rest[next] = index;
            next += 1;
        }
        rest
    };
    let cofactor = |row: usize, column: usize| {
        let sign = if (row + column) % 2 == 0 { 1.0 } else { -1.0 };
        sign * minor(others(row), others(column))
    };
    let determinant = (0..4)
        .map(|column| a(0, column) * cofactor(0, column))
        .sum::<f32>();
    let reciprocal = 1.0 / determinant;
    std::array::from_fn(|row| std::array::from_fn(|column| cofactor(column, row) * reciprocal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_shadow_uses_camera_translation_for_culling_and_world_height_for_depth() {
        let attributes = WorldShadowAttributes {
            scale: 1.5,
            offset: [0.0; 3],
            enabled: true,
            cull: WorldShadowCullSettings {
                minimum_pixels: 0.0,
                extra_radius: 5.0,
            },
        };
        let inputs = ShadowCasterInputs {
            camera_position: [48.0, 12.0, 72.0],
            camera_forward: [0.0, 0.0, 1.0],
            skater_hips: [0.0; 3],
            skater_bounds: [[0.0; 3]; 2],
        };
        let projection = WorldShadowProjection::new([0.0, 0.6, 0.8, 0.0], &attributes, &inputs);
        let bounds = WorldShadowBounds {
            min: [0.0, 10.0, 0.0, 0.0],
            max: [0.0, 20.0, 0.0, 0.0],
        };
        let fitted = projection.fitted_rows(bounds);
        assert_eq!(fitted[..2], projection.preliminary[..2]);
        assert_eq!(
            fitted[2],
            [0.0, -1.0 / 13.0, 0.0, snap(21.5 * (1.0 / 13.0))]
        );
        assert_eq!(
            projection.fitted_rows(WorldShadowBounds {
                min: [0.0; 4],
                max: [0.0; 4]
            })[2],
            [0.0, 0.0, 1.0, 0.0]
        );
    }

    const ATTRIBUTES: CsmAttributes = CsmAttributes {
        scale: 1.8,
        offset: [1.0, 1.0, 1.0],
        depth_range: 5.1,
    };

    fn inputs() -> ShadowCasterInputs {
        ShadowCasterInputs {
            camera_position: [10.0, 22.0, -30.0],
            camera_forward: [0.0, -0.2, 0.9797959],
            skater_hips: [10.0, 21.0, -27.0],
            skater_bounds: [[9.6, 20.0, -27.4], [10.4, 21.9, -26.6]],
        }
    }

    fn light() -> [f32; 4] {
        let [x, y, z] = normalized([0.3, -1.0, 0.2]);
        [x, y, z, 0.0]
    }

    fn apply(row: [f32; 4], point: [f32; 3]) -> f32 {
        row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3]
    }

    #[test]
    fn inverse_undoes_a_general_matrix() {
        let m = [
            [2.0, 0.5, 0.0, 0.0],
            [0.0, 1.5, 0.25, 0.0],
            [0.5, 0.0, 3.0, 0.0],
            [4.0, -2.0, 1.0, 1.0],
        ];
        let inverse = inverse(m);
        for row in 0..4 {
            for column in 0..4 {
                let value: f32 = (0..4).map(|k| m[row][k] * inverse[k][column]).sum();
                let expected = if row == column { 1.0 } else { 0.0 };
                assert!((value - expected).abs() < 1e-5, "{row} {column} {value}");
            }
        }
    }

    #[test]
    fn cascades_map_their_extent_to_the_unit_square() {
        let inputs = inputs();
        let cascades = cascades(light(), &ATTRIBUTES, &inputs);
        let forward = normalized([0.3, -1.0, 0.2].map(|lane: f32| -lane));
        let side = normalized([-forward[2], 0.0, forward[0]]);
        for (cascade, level) in SCALE_LEVEL.into_iter().enumerate() {
            let scale = level * ATTRIBUTES.scale;
            let centre: [f32; 3] = std::array::from_fn(|lane| {
                inputs.camera_position[lane] + inputs.camera_forward[lane] * scale
            });
            let rows = cascades.rows[cascade];
            // The snap moves the origin by less than one texel (2/512).
            assert!(apply(rows[0], centre).abs() < 2.0 / 512.0);
            assert!(apply(rows[1], centre).abs() < 2.0 / 512.0);
            let edge: [f32; 3] = std::array::from_fn(|lane| centre[lane] + side[lane] * scale);
            assert!((apply(rows[0], edge) - 1.0).abs() < 2.0 / 512.0);
        }
    }

    #[test]
    fn depth_is_the_height_below_the_skater_top_over_the_range() {
        let inputs = inputs();
        let cascades = cascades(light(), &ATTRIBUTES, &inputs);
        let top = inputs.skater_bounds[1][1] + TOP_MARGIN;
        let row = cascades.rows[0][2];
        assert!(apply(row, [0.0, top, 0.0]).abs() < 1.0 / 256.0);
        assert!((apply(row, [0.0, top - ATTRIBUTES.depth_range, 0.0]) - 1.0).abs() < 1.0 / 256.0);
        assert_eq!(row, cascades.rows[1][2]);
    }

    #[test]
    fn second_from_first_reproduces_cascade_one() {
        let cascades = cascades(light(), &ATTRIBUTES, &inputs());
        let [scale_x, scale_y, offset_x, offset_y] = cascades.second_from_first;
        for point in [[10.0, 20.0, -27.0], [12.5, 19.0, -24.0], [7.0, 23.5, -29.0]] {
            let first = [
                apply(cascades.rows[0][0], point),
                apply(cascades.rows[0][1], point),
            ];
            let second = [
                apply(cascades.rows[1][0], point),
                apply(cascades.rows[1][1], point),
            ];
            assert!((first[0].mul_add(scale_x, offset_x) - second[0]).abs() < 1e-4);
            assert!((first[1].mul_add(scale_y, offset_y) - second[1]).abs() < 1e-4);
        }
        assert!((scale_x - 0.25).abs() < 1e-5);
        assert!((scale_y - 0.25).abs() < 1e-5);
    }

    #[test]
    fn a_skater_inside_cascade_zero_is_dropped_from_cascade_one() {
        let inputs = inputs();
        assert_eq!(cascades(light(), &ATTRIBUTES, &inputs).casts, [true, false]);
        let far = ShadowCasterInputs {
            skater_hips: [60.0, 21.0, -27.0],
            ..inputs
        };
        assert_eq!(cascades(light(), &ATTRIBUTES, &far).casts, [false, false]);
    }

    #[test]
    fn corners_past_one_edge_are_outside_and_straddling_ones_clip() {
        let rows = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        assert_eq!(
            light_view(&rows, &[[2.0, 0.0, 0.0], [3.0, 0.5, 0.5]]),
            LightView::CompletelyOutside
        );
        assert_eq!(
            light_view(&rows, &[[0.5, 0.0, 0.0], [1.5, 0.5, 0.5]]),
            LightView::Clipping
        );
        assert_eq!(
            light_view(&rows, &[[-0.5, -0.5, 0.0], [0.5, 0.5, 0.5]]),
            LightView::CompletelyInside
        );
    }
}
