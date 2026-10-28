//! Original hud2/hudobjective2 (asset 29, fingerprint c7731a08) and its
//! `hudobjective2` class (InitAction 6fe8), driven by `ObjectiveHUD`
//! (`skate2_core::challenge::objective_hud`). `SetScreenState` runs the
//! class's `SetState` at once (`CallASFunction`), so each state is applied
//! with the component view it carries. The movie is SWF 7: `mBackground` has
//! no `mLeft`, `mRight` or `mTop*` children and `title100` no
//! `labelTypeText`, so the authored calls on them act on undefined values.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer,
    pause_movie::{bounds, child},
};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::objective_hud::{ObjectiveHudView, objective_state};
use skate2_formats::apt::Character;

/// `_global.ScreenManager.mColour_Red`, `mColour_Green` and
/// `mColour_OffWhite`; a `textColor` set keeps the field opaque.
const RED: u32 = 0xff97_230b;
const GREEN: u32 = 0xff82_be19;
const OFF_WHITE: u32 = 0xffff_feec;
/// `Objective_GetMaxNumObjectives` (`0x826b1f20`), the constructor's
/// `MAX_OBJECTIVES`.
const MAX_OBJECTIVES: i32 = 4;
/// `hudobjective2.mTitleMinWidth`.
const TITLE_MIN_WIDTH: f32 = 64.;
/// The constructor's `minBackgroundXScale` and `maxBackgroundXScale`.
const MIN_BACKGROUND_WIDTH: f32 = 250.;
const MAX_BACKGROUND_WIDTH: f32 = 440.;

pub(super) struct ObjectiveMovie {
    pub root: Clip,
    /// The instance's `mTitleScaleWidth`, undefined until `ScaleTitleSection`.
    title_scale_width: Option<f32>,
    pub sounds: Vec<&'static str>,
}

impl ObjectiveMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, 29, 0)?, title_scale_width: None, sounds: Vec::new() };
        movie.calls(assets)?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls(assets)
    }

    fn calls(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hudobjective2" => self.construct(assets)?,
                _ => return Err(format!("untranslated objective HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action 6f74's `new hudobjective2(this)`: a fresh instance (its
    /// members undefined) whose constructor right-auto-sizes every result
    /// label. `mScreen._x -= Screen_EdgeOffset` subtracts zero.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.title_scale_width = None;
        for i in 0..MAX_OBJECTIVES {
            let label = self.root.named(&[&format!("mcResult{i}"), "label"])?;
            auto_size_right(label, assets)?;
        }
        Ok(())
    }

    /// `SetState`: 1 intro, 2 outro, 3 `UpdateObjectivesX`, 4 to 7
    /// `UpdateObjectiveResultX(state - 4, true)`; others only trace.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &ObjectiveHudView) -> RenderResult<()> {
        match state {
            1 | 2 => {
                let label = match (state, view.is_modified_position) {
                    (1, false) => "introCycle",
                    (1, true) => "introCycleMod",
                    (_, false) => "outroCycle",
                    (_, true) => "outroCycleMod",
                };
                self.root.goto(assets, label)?;
                self.scale_adjust(assets, view)
            }
            3 => self.update_objectives(assets, view),
            4..=7 => self.update_result(assets, view, state - 4, true),
            _ => Ok(()),
        }
    }

    /// `UpdateObjectivesX`.
    fn update_objectives(&mut self, assets: &Assets, view: &ObjectiveHudView) -> RenderResult<()> {
        self.init_title_section(assets, view)?;
        let num = view.num_objectives;
        let background = self.root.named(&["mBackground"])?;
        // The variant text is set before the goto replaces labelTypeText.
        background.named(&["labelTypeText"])?.text = Some(assets.initial_text(&view.variant_name));
        let label = if view.variant_name == "#" { format!("{num}no") } else { num.to_string() };
        background.goto(assets, &label)?;
        let title_y = child(background, "mTitleBack")?.entry.matrix.ty + 2.;
        set_y(child(background, "mcTextTitle")?, title_y);
        for i in 0..num {
            // Shows "mcResult" + (MAX_OBJECTIVES - i): mcResult4 is absent.
            if let Ok(result) = self.root.named(&[&format!("mcResult{}", MAX_OBJECTIVES - i)]) {
                result.visible = true;
            }
            self.update_result(assets, view, i, false)?;
        }
        for i in 0..MAX_OBJECTIVES - num {
            self.root.named(&[&format!("mcResult{i}")])?.visible = false;
        }
        self.scale_adjust(assets, view)
    }

    /// `InitTitleSection`: the challenge title, formatted with the argument
    /// `" "` (`GetLocalizedFormatStringCustom`), or `ID_TRICK_CALLOUT` in
    /// the modified position; the activity visibility flags pick which
    /// title fields show.
    fn init_title_section(&mut self, assets: &Assets, view: &ObjectiveHudView) -> RenderResult<()> {
        for i in 0..MAX_OBJECTIVES {
            let label = self.root.named(&[&format!("mcResult{i}"), "label"])?;
            auto_size_right(label, assets)?;
        }
        let text = if view.is_modified_position {
            assets.initial_text("ID_TRICK_CALLOUT")
        } else {
            localizer::format_custom(assets, &view.title, " ")?
        };
        let title = self.root.named(&["mBackground", "mcTextTitle", "title100"])?;
        for name in ["labelText", "labelTextGlow"] {
            let field = title.named(&[name])?;
            auto_size_right(field, assets)?;
            field.text = Some(text.clone());
        }
        let visible = match (view.show_activity_type, view.show_activity_name) {
            (true, false) => Some(false),
            (_, true) => Some(true),
            (false, false) => None,
        };
        if let Some(visible) = visible {
            for name in ["labelText", "labelTextGlow"] {
                title.named(&[name])?.visible = visible;
            }
        }
        self.scale_title_section(assets)
    }

    /// `ScaleTitleSection`: the title back fits the title's `_width`, at
    /// least `mTitleMinWidth`, and the title sits at its right piece.
    fn scale_title_section(&mut self, assets: &Assets) -> RenderResult<()> {
        let label = self.root.named(&["mBackground", "mcTextTitle", "title100", "labelText"])?;
        let width = text_field_width(label, assets)?;
        let scale = if width > TITLE_MIN_WIDTH { width } else { TITLE_MIN_WIDTH };
        self.title_scale_width = Some(scale);
        let background = self.root.named(&["mBackground"])?;
        let back = background.named(&["mTitleBack"])?;
        three_slice_scale(back, assets, scale, true)?;
        let right_x = child(back, "mRight")?.entry.matrix.tx;
        set_x(child(background, "mcTextTitle")?, right_x);
        Ok(())
    }

    /// `UpdateObjectiveResultX(i, playAudio)` on `"mcResult" + (i +
    /// MAX_OBJECTIVES - num)`.
    fn update_result(&mut self, assets: &Assets, view: &ObjectiveHudView, i: i32, play_audio: bool) -> RenderResult<()> {
        let index = usize::try_from(i).map_err(|_| "objective row index")?;
        let row = view.rows.get(index).ok_or("ObjectiveHUD row assertion")?;
        // The row holds the objective's mTitle ("#" + FormatStringCustom,
        // 127 bytes) cut to the row's 95: 94 characters show.
        let title: Vec<u16> = if row.title.0.is_empty() && row.title.1.is_empty() {
            Vec::new()
        } else {
            localizer::format_custom(assets, &row.title.0, &row.title.1)?.into_iter().take(94).collect()
        };
        let name = format!("mcResult{}", i + MAX_OBJECTIVES - view.num_objectives);
        if let Ok(result) = self.root.named(&[&name]) {
            result.visible = true;
            let label = result.named(&["label"])?;
            label.text = Some(title);
            auto_size_right(label, assets)?;
            result.named(&["mcFraction", "labelText"])?.text = Some(assets.initial_text(&row.value));
            set_type_state(result, assets, row.state, row.result_style != 0, play_audio, &mut self.sounds)?;
        } else if row.state == objective_state::COMPLETE && play_audio {
            // SetTypeState on an undefined clip: only the sound remains.
            self.sounds.push("FE_SOUND_TRICK_SUCCESSFUL");
        }
        self.scale_adjust(assets, view)
    }

    /// `ObjectiveScaleAdjust`: the middle and bottom backs fit the widest
    /// listed label plus 100 (250 to 440), or the title back if wider.
    fn scale_adjust(&mut self, assets: &Assets, view: &ObjectiveHudView) -> RenderResult<()> {
        let num = view.num_objectives;
        let mut width = 0.0_f32;
        for i in 0..num {
            let label = self.root.named(&[&format!("mcResult{}", i + MAX_OBJECTIVES - num), "label"])?;
            let label_width = text_field_width(label, assets)?;
            if label_width > width {
                width = label_width;
            }
        }
        width += 100.;
        if width < MIN_BACKGROUND_WIDTH {
            width = MIN_BACKGROUND_WIDTH;
        }
        let background = self.root.named(&["mBackground"])?;
        // Math.max(mTitleScaleWidth + mTitleBack.mLeft._width, width), then
        // Math.max with mTitleScaleWidth + mBackground's absent mLeft and
        // mRight widths: undefined under SWF 7 Add2, toFloat 0, so ignored.
        // An undefined mTitleScaleWidth likewise loses to width.
        if let Some(title) = self.title_scale_width {
            let candidate = title + piece_width(background.named(&["mTitleBack"])?, assets, "mLeft")?;
            if candidate > width {
                width = candidate;
            }
        }
        if width >= MAX_BACKGROUND_WIDTH {
            width = MAX_BACKGROUND_WIDTH;
        }
        three_slice_scale(background.named(&["mMiddleBack"])?, assets, width, false)?;
        three_slice_scale(background.named(&["mBottomBack"])?, assets, width, false)
    }
}

/// `SetTypeState(state, mcObj, type, playAudio)`: the label and fraction
/// colours and the result clip's label; a completed objective only animates
/// (with `FE_SOUND_TRICK_SUCCESSFUL`) when the call plays audio.
fn set_type_state(result: &mut Clip, assets: &Assets, state: i32, check_box: bool, play_audio: bool,
    sounds: &mut Vec<&'static str>) -> RenderResult<()> {
    let (colour, label) = match state {
        objective_state::FAILED => (RED, Some(if check_box { "fail_check" } else { "fraction" })),
        objective_state::COMPLETE => {
            if play_audio {
                sounds.push("FE_SOUND_TRICK_SUCCESSFUL");
            }
            (GREEN, play_audio.then_some(if check_box { "success_check" } else { "fraction" }))
        }
        objective_state::INCOMPLETE | objective_state::ACTIVE => (OFF_WHITE, Some(if check_box { "checkbox" } else { "fraction" })),
        _ => return Ok(()),
    };
    for path in [&["label"][..], &["mcFraction", "labelText"]] {
        result.named(path)?.properties.insert("text_colour".into(), colour.to_string());
    }
    if let Some(label) = label {
        result.goto(assets, label)?;
    }
    Ok(())
}

/// `label.autoSize = "right"`: the field keeps its right edge and shrinks to
/// its text (`EnsureStringAllocated` `0x82a12220`). The authored result and
/// title fields are single-line and right-aligned, so they draw unchanged.
fn auto_size_right(field: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("objective HUD auto-size target is not text".into());
    };
    if text.multiline || text.alignment != 1 {
        return Err("objective HUD auto-size field differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), "1".into());
    Ok(())
}

/// A right-auto-sized field's `_width` (`objectMemberLookup` text member
/// `_width`): `textWidth + 4`, where `textWidth` is `cFont::TextExtent`
/// (`0x82849ba0`), the glyph advances at the field's font height.
pub(super) fn text_field_width(field: &Clip, assets: &Assets) -> RenderResult<f32> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("objective HUD width target is not text".into());
    };
    let Some(Character::Font { name, .. }) = assets.movies[field.asset].movie.characters.get(text.font_id as usize).and_then(Option::as_ref) else {
        return Err("objective HUD text font absent".into());
    };
    let font = assets.apt_font(name)?;
    let scale = font.scale[0] * text.height;
    let width = field.text.as_deref().unwrap_or_default().iter()
        .filter_map(|&c| font.metrics.mapped_glyph(c))
        .fold(0.0_f32, |sum, glyph| glyph.advance.mul_add(scale, sum));
    Ok(width + 4.)
}

/// A child sprite's `_width`: its bounds through its own matrix.
fn piece_width(holder: &mut Clip, assets: &Assets, name: &str) -> RenderResult<f32> {
    let piece = child(holder, name)?;
    let r = bounds(&piece.clip, assets)?.ok_or("objective HUD piece has no bounds")?;
    let m = piece.entry.matrix;
    let (left, right) = (r.left.mul_add(m.a, m.tx), r.right.mul_add(m.a, m.tx));
    Ok((right - left).abs())
}

/// `ThreeSliceScale(holder, xwidth, ScaleInnerToDimensions)`: the centre
/// piece stretches to the width (less the side pieces unless inner) and
/// extends left from the right piece; the left piece follows it.
fn three_slice_scale(holder: &mut Clip, assets: &Assets, width: f32, inner: bool) -> RenderResult<()> {
    let mut slice = if inner {
        width
    } else {
        width - piece_width(holder, assets, "mLeft")? - piece_width(holder, assets, "mRight")?
    };
    set_xscale(child(holder, "mCenter")?, 100.);
    if slice < 0. {
        slice = 0.;
    }
    let natural = piece_width(holder, assets, "mCenter")?;
    set_xscale(child(holder, "mCenter")?, 100. * (slice / natural));
    let right = child(holder, "mRight")?.entry.matrix.tx;
    let x = right - piece_width(holder, assets, "mCenter")?;
    for name in ["mCenter", "mLeft"] {
        set_x(child(holder, name)?, x);
    }
    Ok(())
}

/// `SetProceduralProperty(_x)` from ActionScript (`0x829ea608`, AS flag):
/// the instance leaves timeline control of its matrix and colour.
pub(super) fn set_x(piece: &mut Child, x: f32) {
    piece.clip.script_transform = true;
    piece.entry.matrix.tx = x;
}

/// `SetProceduralProperty(_y)` from ActionScript.
pub(super) fn set_y(piece: &mut Child, y: f32) {
    piece.clip.script_transform = true;
    piece.entry.matrix.ty = y;
}

/// `SetProceduralProperty(_xscale)` from ActionScript: `value * 0.01`
/// (zero becomes 0.0001) replaces the unrotated matrix's x scale.
pub(super) fn set_xscale(piece: &mut Child, value: f32) {
    piece.clip.script_transform = true;
    let mut scale = value * f32::from_bits(0x3c23_d70a);
    if scale == 0. {
        scale = f32::from_bits(0x38d1_b717);
    }
    let m = &mut piece.entry.matrix;
    if m.b.abs() >= f32::from_bits(0x38d1_b717) || m.c.abs() >= f32::from_bits(0x38d1_b717) {
        let inverse = 1. / m.b.mul_add(m.b, m.a * m.a).sqrt();
        m.a = inverse * m.a * scale;
        m.b = inverse * m.b * scale;
    } else {
        m.a = scale;
    }
}

/// hudobjective2.apt's sole InitAction defines the guarded global class
/// translated above; root action 6f74 constructs it.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (29, 81, 0x6fe8))
}

/// Timeline actions. `mcResult`'s frame scripts colour an undefined
/// `mcObjective`, so they do nothing.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x6f74 => {
            clip.visible = false;
            clip.calls.push("hudobjective2");
        }
        0x7dbc | 0x7dd8 => clip.visible = true,
        0x7e04 => {
            clip.visible = false;
            clip.playback.stop();
        }
        0x7f94 | 0x7ff8 | 0x8028 | 0x8068 | 0x8098 => {}
        _ => return Err(format!("untranslated objective HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
