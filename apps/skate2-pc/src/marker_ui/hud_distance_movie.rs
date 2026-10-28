//! Original hud2/huddistance (asset 72, fingerprint 1aa3f3be; template entry `DistanceMeter`
//! `0x82fdb808`, placement 0, drawn flat) and its `huddistance` class (InitAction character 7,
//! `0xcf8`), driven by `DistanceMeter` (`skate2_core::challenge::turn_hud`). `SetScreenState`
//! runs the class's `SetState` at once, so each state is applied with the view it carries;
//! `DistanceMeter_GetData` (`0x826b7c48`) answers `mBuffer` and `mColour`.
use super::{assets::Assets, clips::Clip, localizer};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::turn_hud::{DistanceText, TurnHudView};

pub(super) const ASSET: usize = 72;
/// `_global.ScreenManager.mColour_Red`; a `textColor` set keeps the field opaque.
const RED: u32 = 0xff97_230b;
/// `_global.ScreenManager.mColour_White`.
const WHITE: u32 = 0xffff_ffff;

pub(super) struct DistanceMovie {
    pub root: Clip,
    /// `huddistance.mVisible`, undefined until the first `Show`.
    visible: Option<bool>,
}

impl DistanceMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, visible: None };
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
                "huddistance" => self.construct(assets)?,
                _ => return Err(format!("untranslated distance HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0xcc4`'s `new huddistance(this)`: the constructor blanks both labels (`#`)
    /// and registers the screen.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.visible = None;
        for name in ["labelText", "labelTextGlow"] {
            self.root.named(&["mcTextTimer", "mcTimer", name])?.text = Some(assets.initial_text("#"));
        }
        Ok(())
    }

    /// `SetState`: 1 and 2 `Show(true)` and `Show(false)`, 3 `UpdateTimerHUD`, 4 the flash and
    /// 5 its end.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView) -> RenderResult<()> {
        match state {
            1 => self.show(assets, true),
            2 => self.show(assets, false),
            3 => {
                let TurnHudView::Distance(text) = view else { return Err("distance HUD state 3 without meter data".into()) };
                self.update(assets, *text)
            }
            4 => self.root.named(&["mcTextTimer"])?.goto(assets, "flash"),
            5 => self.root.named(&["mcTextTimer"])?.goto_and_stop(assets, "normal"),
            _ => Ok(()),
        }
    }

    /// `Show(bVisible)`: nothing for the current visibility, else `hudintro` or `hudoutro`.
    fn show(&mut self, assets: &Assets, visible: bool) -> RenderResult<()> {
        if self.visible == Some(visible) {
            return Ok(());
        }
        self.root.goto(assets, if visible { "hudintro" } else { "hudoutro" })?;
        self.visible = Some(visible);
        Ok(())
    }

    /// `UpdateTimerHUD`: `mBuffer` in both labels, red while `mColour` is `eMeterColour_Red`.
    fn update(&mut self, assets: &Assets, text: DistanceText) -> RenderResult<()> {
        let buffer = match text.distance {
            Some(distance) => localizer::format_custom(assets, "ID_COMMON_DISTANCE", &format!("{:.1}", f64::from(distance)))?,
            None => Vec::new(),
        };
        let timer = self.root.named(&["mcTextTimer", "mcTimer"])?;
        timer.named(&["labelText"])?.text = Some(buffer.clone());
        timer.named(&["labelTextGlow"])?.text = Some(buffer);
        let colour = if text.red { RED } else { WHITE };
        timer.named(&["labelText"])?.properties.insert("text_colour".into(), colour.to_string());
        Ok(())
    }
}

/// The movie's InitAction: the `huddistance` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 7, 0xcf8))
}

/// Root timeline actions besides `Stop`: frame 0 constructs the class and hides the movie,
/// frame 9 (`hudintro`) shows it and frame 30 hides it and stops.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0xcc4 => {
            clip.calls.push("huddistance");
            clip.visible = false;
        }
        0xfa0 => clip.visible = true,
        0xfc8 => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated distance HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
