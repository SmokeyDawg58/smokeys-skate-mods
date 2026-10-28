//! challenge/spotfound (asset 39, fingerprint 7ced72b1): `FrontEndState_NIS`'s
//! spot found banner (`eSk8FEScreenState_SpotFound`, the `SpotFound_*`
//! overlay states). InitAction character 27 (`0x614c`) only tests
//! `_global.iscreenmodule`; character 28 (`0x6160`) defines the `spotfound`
//! class translated here; root action `0x6110` constructs it and stops the
//! root. The screen manager creates it in its own layer (template type 0),
//! drawn flat.
use super::{assets::Assets, clips::Clip, localizer};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::system::MissionTitle;
use skate2_formats::apt::Character;

pub(super) const ASSET: usize = 39;
/// `SetState`'s labels for `kScreenState_IntroBottomRight` (0) through
/// `kScreenState_OutroTopLeft` (7).
const LABELS: [&str; 8] = [
    "introBottomRight",
    "outroBottomRight",
    "introTopRight",
    "outroTopRight",
    "introBottomLeft",
    "outroBottomLeft",
    "introTopLeft",
    "outroTopLeft",
];

pub(super) struct SpotFoundMovie {
    pub root: Clip,
    /// `Audio.PlaySound` names the timeline asked for.
    pub sounds: Vec<&'static str>,
}

impl SpotFoundMovie {
    /// The screen's movie runs its first frame: `new spotfound(this)`
    /// (`RegisterScreen`, `mOriginalX = mScreen._x`), then `stop()`. The
    /// screen manager's `PlayClip(id, 'intro')` finds no `intro` label.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, sounds: Vec::new() };
        movie.calls()?;
        movie.root.goto(assets, "intro")?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls()
    }

    fn calls(&mut self) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "spotfound" => self.root.playback.stop(),
                "FE_SOUND_SPOT_FOUND" => self.sounds.push(call),
                _ => return Err(format!("untranslated spot found callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState(state)`: offline (`Mission.GetIsOnlineMission` and
    /// `GetIsPartyPlayMission` false) both spot name labels take
    /// `Mission.GetTitle()` and both headers `ID_EVENT_SPOT_FOUND_CAPS`, the
    /// `mcOwned` labels auto-size right and the `mcTextLeftJustified` ones
    /// left; the `mcPoints`/`mcABDPoints` it hides are not in the movie. An
    /// intro moves `mScreen._x` by `Screen_EdgeOffset` (zero) when it is
    /// still at `mOriginalX`; states 0 to 7 play their corner's label.
    pub fn set_state(&mut self, assets: &Assets, state: i32, title: &MissionTitle) -> RenderResult<()> {
        let title = localizer::mission_title(assets, title)?;
        let header = assets.initial_text("ID_EVENT_SPOT_FOUND_CAPS");
        for (holder, alignment) in [("mcOwned", RIGHT), ("mcTextLeftJustified", LEFT)] {
            let name = self.root.named(&[holder, "mcSpotName", "labelText"])?;
            name.text = Some(title.clone());
            auto_size(name, assets, alignment)?;
            for field in ["labelText", "labelText_glow"] {
                let field = self.root.named(&[holder, "mcSpotFound", field])?;
                field.text = Some(header.clone());
                auto_size(field, assets, alignment)?;
            }
        }
        if let Some(label) = usize::try_from(state).ok().and_then(|s| LABELS.get(s)) {
            self.root.goto(assets, label)?;
        }
        self.calls()
    }
}

/// `TextAlignment` of a right- and a left-aligned field.
const RIGHT: u32 = 1;
const LEFT: u32 = 0;

/// `autoSize = "right"` / `"left"`: the field keeps that edge and shrinks to
/// its text (`EnsureStringAllocated` `0x82a12220`); a single line aligned to
/// the same edge draws unchanged.
fn auto_size(field: &mut Clip, assets: &Assets, alignment: u32) -> RenderResult<()> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("spot found auto-size target is not text".into());
    };
    if text.multiline || text.alignment != alignment {
        return Err("spot found auto-size field differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), if alignment == RIGHT { "1" } else { "left" }.into());
    Ok(())
}

/// spotfound.apt's InitActions: character 27's empty `iscreenmodule` test
/// and character 28's guarded global class translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 27, 0x614c) | (ASSET, 28, 0x6160))
}

/// Root action `0x6110` (`var screen = new spotfound(this); stop();`) and the
/// corners' frame actions: each intro shows the root and plays
/// `_global.FE_SOUND_SPOT_FOUND`, each end hides and stops it.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x6110 => {
            clip.calls.push("spotfound");
            clip.playback.stop();
        }
        0x6714 | 0x6778 | 0x67d4 | 0x6830 => {
            clip.visible = true;
            clip.calls.push("FE_SOUND_SPOT_FOUND");
        }
        0x676c | 0x67c8 | 0x6824 | 0x687c => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated spot found APT action {offset:#x}").into()),
    }
    Ok(())
}
