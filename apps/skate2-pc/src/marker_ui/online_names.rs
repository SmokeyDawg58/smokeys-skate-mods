//! `SkaterPresEntity::DrawName` (0x827d7fe0): an online player's name over each remote skater
//! `SceneRenderView::Render` (0x828508f8) draws, after the scene's meshes and not in a replay.
//! The label is the entity's `m_MatLtoW` translation (its hips) raised by the adjust, projected
//! through the scene camera; the font, `helvetica_bold`, shrinks and fades with the camera's
//! distance and takes the player's colour, then draws centred on the 1280x720 front buffer at the
//! label's depth, tested against the scene's and writing it. The tuning is the
//! `Render_OnlineSkaterName` cvars' defaults: a name fades out 33 m from the camera.

use super::assets::Assets;
use crate::gpu_renderer::{RenderResult, marker_ui::Batch};

/// `kfSkaterLabelXAdjust` and `kfSkaterLabelYAdjust`, added to the label's world x and y.
const ADJUST: [f32; 2] = [0.0, 1.3];
/// `kfSkaterLabelFontSize` ("Text Size").
const FONT_SIZE: f32 = 20.0;
/// `kfSkaterLabelFontScale` ("Text Distance Scale").
const FONT_SCALE: f32 = 0.82;
/// `kfSkaterLabelFontFade` ("Text Distance Fade").
const FONT_FADE: f32 = 0.7;
/// The font `cFontManager::GetFontByName` finds by its file name.
const FONT: &str = "helvetica_bold";
/// `PlayersBackEnd::GetPlayerColour` (0x82564238): `kOnlineColours` by player index, white past
/// them; `DrawName` keeps its RGB.
const ONLINE_COLOURS: [[u32; 3]; 8] = [
    [0x3f1f_9fa0, 0x3cc0_c0c1, 0x3cc0_c0c1],
    [0x3efe_feff, 0x3f11_9192, 0x3e8e_8e8f],
    [0x3eba_babb, 0x3f00_8081, 0x3f20_a0a1],
    [0x3f48_c8c9, 0x3f40_c0c1, 0x3e90_9091],
    [0x3f08_8889, 0x3e8a_8a8b, 0x3f21_a1a2],
    [0x3f4a_cacb, 0x3eb4_b4b5, 0x3d10_9091],
    [0x3e4c_cccd, 0x3ed6_d6d7, 0x3ee6_e6e7],
    [0x3ee0_e0e1, 0x3ee2_e2e3, 0x3f01_8182],
];
/// The front buffer `DrawName` places the text on.
const FRONT_BUFFER: [f32; 2] = [1280.0, 720.0];

/// A named skater this frame.
pub(crate) struct NameTag<'a> {
    pub name: &'a str,
    pub index: u8,
    pub hips: [f32; 3],
}

/// The view the scene draws with: its 16:9 clip-from-world (the window's differs only in its x
/// and y rows) and the camera's position (`CameraPresentation::GetMatrix`'s translation).
pub(crate) struct NameView {
    pub clip_from_world: [f32; 16],
    pub camera: [f32; 3],
}

/// `kOnlineColours`' entry for a player index.
fn colour(index: u8) -> [f32; 3] {
    ONLINE_COLOURS.get(usize::from(index)).map_or([1.0; 3], |rgb| rgb.map(f32::from_bits))
}

/// A column-major clip-from-world applied to a world point.
#[expect(clippy::cast_possible_truncation, reason = "the f64 sum narrows to the f32 lanes")]
fn project(clip_from_world: &[f32; 16], point: [f32; 3]) -> [f32; 4] {
    std::array::from_fn(|row| {
        (0..3).fold(f64::from(clip_from_world[12 + row]), |sum, axis| {
            f64::from(clip_from_world[axis * 4 + row]).mul_add(f64::from(point[axis]), sum)
        }) as f32
    })
}

/// The names' glyph quads in the window's clip space: the 1280x720 front buffer fitted to the
/// window as the 3D view is.
#[expect(clippy::cast_precision_loss, reason = "window dimensions")]
pub(super) fn batches(assets: &Assets, tags: &[NameTag<'_>], view: &NameView, size: (u32, u32)) -> RenderResult<Vec<Batch>> {
    let font = assets.font_by_file(FONT)?;
    let atlas = assets.textures.get(&font.texture).ok_or("name font atlas missing")?;
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    let fit = [((16. / 9.) / aspect).min(1.), (aspect / (16. / 9.)).min(1.)];
    let mut vertices = Vec::new();
    for tag in tags {
        let label = [tag.hips[0] + ADJUST[0], tag.hips[1] + ADJUST[1], tag.hips[2]];
        let clip = project(&view.clip_from_world, label);
        // 0x827d8160: nothing for a label behind the near plane.
        if clip[2] < 0.0 {
            continue;
        }
        let distance = label.iter().zip(view.camera).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt();
        let near = 0.1 * distance;
        let text_size = (FONT_SCALE - 1.0).mul_add(near, 1.0) * FONT_SIZE;
        let alpha = (FONT_FADE - 1.0).mul_add(near, 1.0);
        // The colour's alpha reaches the blend clamped to [0, 1]: a name past the fade draws nothing.
        if alpha <= 0.0 {
            continue;
        }
        let [red, green, blue] = colour(tag.index);
        let tint = [red, green, blue, alpha.min(1.0)];
        // cFont::SetScale: the font's base scale times the size.
        let scale = font.scale.map(|base| base * text_size);
        let units: Vec<u16> = tag.name.encode_utf16().collect();
        // CenterText: the text's TextExtent (its advances) centred on the label; Text adds the
        // font's m_offset, (0, 0) as constructed.
        let extent = units.iter().filter_map(|&unit| font.metrics.mapped_glyph(unit)).map(|glyph| glyph.advance * scale[0]).sum::<f32>();
        let x = (clip[0] / clip[3] + 1.0) * 0.5 * FRONT_BUFFER[0];
        let y = -((clip[1] / clip[3] + 1.0).mul_add(0.5, -1.0)) * FRONT_BUFFER[1];
        let z = clip[2] / clip[3];
        for (bounds, uv) in super::glyph_quads(font, atlas, &units, scale, [extent.mul_add(-0.5, x), y]) {
            let corner = |cx: f32, cy: f32, u: f32, v: f32| {
                [(cx / 640. - 1.) * fit[0], (1. - cy / 360.) * fit[1], z, 1., u, v, tint[0], tint[1], tint[2], tint[3]]
            };
            let corners = [
                corner(bounds[0], bounds[1], uv[0], uv[1]),
                corner(bounds[2], bounds[1], uv[2], uv[1]),
                corner(bounds[2], bounds[3], uv[2], uv[3]),
                corner(bounds[0], bounds[3], uv[0], uv[3]),
            ];
            vertices.extend([0, 1, 2, 0, 2, 3].map(|index| corners[index]));
        }
    }
    Ok(if vertices.is_empty() { Vec::new() } else { vec![Batch { texture: Some(font.texture.clone()), wrapped: false, vertices }] })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_players_take_red_and_green_and_the_rest_past_eight_white() {
        let red = colour(0);
        assert!((red[0] - 159.0 / 255.0).abs() < 1e-3 && (red[1] - 6.0 / 255.0).abs() < 1e-3);
        let green = colour(1);
        assert!((green[0] - 127.0 / 255.0).abs() < 1e-3 && (green[1] - 145.0 / 255.0).abs() < 1e-3);
        assert_eq!(colour(8), [1.0; 3]);
    }

    #[test]
    fn a_name_fades_out_a_third_of_a_hundred_metres_away() {
        let alpha = |distance: f32| (FONT_FADE - 1.0).mul_add(0.1 * distance, 1.0);
        assert!(alpha(33.0) > 0.0);
        assert!(alpha(33.5) < 0.0);
        let size = |distance: f32| (FONT_SCALE - 1.0).mul_add(0.1 * distance, 1.0) * FONT_SIZE;
        assert!((size(10.0) - 16.4).abs() < 1e-4);
    }
}
