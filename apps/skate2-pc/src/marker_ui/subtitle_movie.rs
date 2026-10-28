//! main.swf's subtitle layer, source/screens/main/subtitle (asset 36,
//! fingerprint d847264a). main.apt (`0x12d5`) loads it into
//! `_root.gSubtitle`, a `ScreenLayer` clip of `_level0` created between
//! `gChyron` and `gPopups`: drawn flat, under the loaded levels. The
//! InitAction (character 7, `0x378`) defines the `subtitle` class translated
//! here and root action `0x34c` constructs it.
//!
//! `FESubtitle::SetSubtitleText` (`0x82615ec8`) and `ClearSubtitle`
//! (`0x82615e68`) end in `FrontEndManager::RefreshSubtitle` (`0x82614c80`),
//! which calls `ScreenManager.RefreshSubtitle` (`Subtitle.RefreshSubtitle()`)
//! only with the profile's subtitles option (Options' Video Settings) or
//! `mbOverrideProfileSetting`, which `FEMoviePlayer::Play` (`0x82638fb8`) sets
//! for a French or German movie in Europe. The lines come from
//! `SubTitler::Update` (`0x8269fbd0`): a NIS scene's, and a subtitled FMV's
//! while `FrontEndState_MoviePlayer` plays it. Options cannot open over
//! either, so a refresh is due when the line changes.
use super::{
    assets::Assets,
    clips::Clip,
    localizer,
    objective_movie::set_y,
    pause_movie::child,
    tutorial_movie::max_scroll,
};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 36;

pub(super) struct SubtitleMovie {
    pub root: Clip,
    /// The front end's line at the last refresh.
    line: Option<u32>,
}

impl SubtitleMovie {
    /// `loadMovie`: the class constructed, the line hidden.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "subtitle" => construct(&mut root)?,
                _ => return Err(format!("untranslated subtitle callback {call}").into()),
            }
        }
        Ok(Self { root, line: None })
    }

    /// The front end's line (`FESubtitle::mSubtitleText`, a HAL id) and the
    /// subtitles option.
    pub fn update(&mut self, assets: &Assets, line: Option<u32>, subtitles: bool) -> RenderResult<()> {
        if !subtitles || line == self.line {
            return Ok(());
        }
        self.line = line;
        refresh(&mut self.root, assets, line)
    }

    /// The line of the last refresh, which the screen shows (nothing for an
    /// id the localizer lacks).
    pub fn shown(&self) -> Option<u32> {
        self.line
    }
}

/// `new subtitle(this)`: `_global.Subtitle` becomes the instance and
/// `subtitle.mScreen` the movie; `labelText.autoSize = "center"` moves
/// nothing, as `SkateAptString` (`0x82a70b28`) keeps a multiline field's box
/// and `EnsureStringAllocated` (`0x82a12220`) shifts it by the width change;
/// `Clear()` hides the line; `Stop`.
fn construct(root: &mut Clip) -> RenderResult<()> {
    enable(root, false)?;
    root.playback.stop();
    Ok(())
}

/// `EnableSubtitle(b)`: `mcTextSubtitle._visible` and `mDimmer._visible`.
fn enable(root: &mut Clip, visible: bool) -> RenderResult<()> {
    root.named(&["mcTextSubtitle"])?.visible = visible;
    root.named(&["mDimmer"])?.visible = visible;
    Ok(())
}

/// `RefreshSubtitle()`: `Game.GetMovieSubtitleString()` (`0x82636e50`) is
/// `mSubtitleText`, `"#"` when that is empty. `SetSubtitleText` formats a
/// line the localizer has (`DoesStringExist`; `FormatString` with no
/// arguments, after the `#` format prefix) and clears one it lacks. `"#"`
/// hides `mcTextSubtitle` and `mDimmer`, anything else shows them; either
/// way the string becomes `labelText.text` (`SkateAptString` drops the `#`)
/// and `labelText._y` becomes `(3 - labelText.maxscroll) * 12`.
fn refresh(root: &mut Clip, assets: &Assets, line: Option<u32>) -> RenderResult<()> {
    let text = match line {
        Some(hal) if assets.localized.contains_key(&hal) => localizer::format_hal(assets, hal)?,
        _ => Vec::new(),
    };
    if std::env::var_os("SKATE2_TRACE_SUBTITLES").is_some() {
        eprintln!("subtitle: {line:x?} {:?}", String::from_utf16_lossy(&text));
    }
    enable(root, !text.is_empty())?;
    let field = child(root.named(&["mcTextSubtitle"])?, "labelText")?;
    field.clip.text = Some(text);
    #[expect(clippy::cast_precision_loss, reason = "a line count")]
    let lines = max_scroll(&field.clip, assets)? as f32;
    set_y(field, (3. - lines) * 12.);
    Ok(())
}

/// subtitle.apt's sole InitAction defines the guarded global class
/// translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 7, 0x378))
}

/// Root action `0x34c`, `var screen = new subtitle(this)`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x34c => clip.calls.push("subtitle"),
        _ => return Err(format!("untranslated subtitle APT action {offset:#x}").into()),
    }
    Ok(())
}
