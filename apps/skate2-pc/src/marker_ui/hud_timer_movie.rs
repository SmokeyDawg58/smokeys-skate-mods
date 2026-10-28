//! Original hud2/hudtimer (asset 54, fingerprint 19a5bcd1; template entry `TimerHUD`, placement
//! 0, drawn flat) and its `hudtimer` class (InitAction character 8, `0x1dc8`), driven by
//! `TimerHUD` (`skate2_core::challenge::turn_hud`). `SetScreenState` runs the class's `SetState`
//! at once, so each state is applied with the view it carries; `Timer_GetData` (`0x826b7b88`)
//! answers `mTimeBuffer` and `mColour`.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::turn_hud::{TimerText, TurnHudView};
use skate2_formats::apt::Character;

pub(super) const ASSET: usize = 54;
/// `_global.ScreenManager.mColour_Red`; a `textColor` set keeps the field opaque.
const RED: u32 = 0xff97_230b;
/// The `textColor` the class sets outside the distress time, `0xFFFFEC`.
const OFF_WHITE: u32 = 0xffff_ffec;

pub(super) struct TimerMovie {
    pub root: Clip,
    /// `hudtimer.mVisible`, undefined until the first `Show`.
    visible: Option<bool>,
}

impl TimerMovie {
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
                "hudtimer" => self.construct(assets)?,
                _ => return Err(format!("untranslated timer HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0x1d94`'s `new hudtimer(this)`: the constructor blanks the time (`#`) and
    /// registers the screen.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.visible = None;
        self.root.named(&["mcTextTimer", "mcTimer", "mTimeStrs", "labelText"])?.text = Some(assets.initial_text("#"));
        Ok(())
    }

    /// `SetState`: 1 and 2 `Show(true)` and `Show(false)`, 3 `UpdateTimerHUD`, 4 the distress
    /// flash and 5 its end.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView) -> RenderResult<()> {
        match state {
            1 => self.show(assets, true),
            2 => self.show(assets, false),
            3 => {
                let TurnHudView::Timer(text) = view else { return Err("timer HUD state 3 without timer data".into()) };
                self.update_timer(assets, *text)
            }
            4 => {
                self.root.named(&["mcTextTimer"])?.goto(assets, "flash")?;
                self.root.named(&["mcTextTimer", "mcTimer"])?.goto(assets, "big")
            }
            5 => {
                self.root.named(&["mcTextTimer"])?.goto_and_stop(assets, "normal")?;
                self.root.named(&["mcTextTimer", "mcTimer"])?.goto(assets, "normal")
            }
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

    /// `UpdateTimerHUD`: `Timer_GetData`'s text in both labels, red while `mColour` is
    /// `eTimerColour_Red`, then the label centre-sized.
    fn update_timer(&mut self, assets: &Assets, text: TimerText) -> RenderResult<()> {
        let time = assets.initial_text(&format!("#{}", format_time(assets, text.minutes, text.seconds)?));
        let strings = self.root.named(&["mcTextTimer", "mcTimer", "mTimeStrs"])?;
        strings.named(&["labelText"])?.text = Some(time.clone());
        strings.named(&["labelTextGlow"])?.text = Some(time);
        let label = strings.named(&["labelText"])?;
        let colour = if text.white { OFF_WHITE } else { RED };
        label.properties.insert("text_colour".into(), colour.to_string());
        auto_size_center(label, assets)
    }
}

/// `FormatTime(buffer, 0, minutes, seconds, 0, true, 16)` (`0x829812f0`) through
/// `BuildTimeFormatString` (`0x82980e30`) with the English `hh:mm:ss.ff`: no hours, two-digit
/// minutes when there are any, then two-digit seconds, and no fraction (flag 16); nothing
/// for a zero time.
pub(super) fn format_time(assets: &Assets, minutes: i32, seconds: i32) -> RenderResult<String> {
    if assets.localize("ID_COMMON_TIME_FORMAT")? != "hh:mm:ss.ff" {
        return Err("unsupported timer HUD time pattern".into());
    }
    Ok(if minutes > 0 {
        format!("{minutes:02}:{seconds:02}")
    } else if seconds > 0 {
        format!("{seconds:02}")
    } else {
        String::new()
    })
}

/// `label.autoSize = "center"`: the field keeps its centre and shrinks to its text. The
/// authored time labels are single-line and centred, so they draw unchanged.
fn auto_size_center(field: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let Character::Text(text) = field.definition(assets)? else {
        return Err("timer HUD auto-size target is not text".into());
    };
    if text.multiline || text.alignment != 2 {
        return Err("timer HUD auto-size field differs from the verified layout".into());
    }
    field.properties.insert("text_autosize".into(), "1".into());
    Ok(())
}

/// The movie's InitAction: the `hudtimer` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 8, 0x1dc8))
}

/// Root timeline actions besides `Stop`: frame 0 constructs the class and hides the movie,
/// frame 9 (`hudintro`) shows it and frame 45 hides it and stops.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1d94 => {
            clip.calls.push("hudtimer");
            clip.visible = false;
        }
        0x2094 => clip.visible = true,
        0x20bc => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated timer HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
