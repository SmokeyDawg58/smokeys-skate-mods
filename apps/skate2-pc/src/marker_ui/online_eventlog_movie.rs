//! Original hud2/hud_online_eventlog2 (screen 86, asset 104, fingerprint 33ac45c2; template entry
//! `HUD2_OnlineEventLog` 0x82fdb8b8, placement 5) and its `hud_online_eventlog2` class
//! (InitAction character 13, `0x1ea4`), driven by `FE::OnlineEventLog`
//! (`skate2_core::online::event_log`): the five lines of online notices at the top left, each
//! formatted by `LocalizerManager::FormatString`. The root plays its own fade-in once loaded.
use super::{assets::Assets, clips::Clip, edit_skater_movie::autosize_text, localizer};
use crate::gpu_renderer::RenderResult;
use skate2_core::online::event_log::{EventLogSnapshot, LineState};

pub(super) const ASSET: usize = 104;
/// `NUM_LOG_ENTRIES`.
const LINES: usize = 5;

pub(super) struct OnlineEventLogMovie {
    pub root: Clip,
}

impl OnlineEventLogMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)? };
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
                "hud_online_eventlog2" => self.refresh(assets, &EventLogSnapshot::default())?,
                _ => return Err(format!("untranslated online event log callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `Refresh` (the constructor's last call and `SetState(1)`): each entry's line plays its
    /// intro or outro, or stops on `shown`, with its text left auto-sized and shown in both
    /// fields; the lines past the entries hide their text.
    pub fn refresh(&mut self, assets: &Assets, log: &EventLogSnapshot) -> RenderResult<()> {
        for index in 0..LINES {
            let line = self.root.named(&["mcEventLog", &format!("mcEventLine{index}")])?;
            let Some((text, state)) = log.lines.get(index) else {
                for field in ["labelEventLine", "labelEventLineGlow"] {
                    line.named(&["mcEventLine", field])?.visible = false;
                }
                continue;
            };
            match state {
                LineState::Intro => line.goto(assets, "intro")?,
                LineState::Outro => line.goto(assets, "outro")?,
                LineState::Shown => line.goto_and_stop(assets, "shown")?,
            }
            let shown = if let Some(line) = &text.literal {
                line.encode_utf16().collect()
            } else {
                // AddLocationChangeMessage formats its HAL arguments first.
                let arguments = if text.localized {
                    text.arguments.iter().map(|argument| assets.localize(argument)).collect::<RenderResult<Vec<_>>>()?
                } else {
                    text.arguments.clone()
                };
                localizer::format_string_text(assets, text.key, &arguments)?
            };
            let string = String::from_utf16_lossy(&shown);
            for field in ["labelEventLine", "labelEventLineGlow"] {
                let label = line.named(&["mcEventLine", field])?;
                label.text = Some(shown.clone());
                // autoSize "left": the field takes its text's width from its left edge.
                autosize_text(label, assets, &string)?;
                label.visible = true;
            }
        }
        Ok(())
    }
}

/// The movie's InitAction: the `hud_online_eventlog2` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 13, 0x1ea4))
}

/// Root action `0x1e84`: `new hud_online_eventlog2(this)`, whose constructor moves the screen by
/// `Screen_EdgeOffset` (zero), registers it (`Initialise` keeps the five lines and shows the
/// screen) and runs `Refresh` over an empty log. The other actions are `Stop`s.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1e84 => clip.calls.push("hud_online_eventlog2"),
        _ => return Err(format!("untranslated online event log APT action {offset:#x}").into()),
    }
    Ok(())
}
