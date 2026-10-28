//! Original letterbox APT's `_global.LetterBox` class (constructor bd67).
//! The bars and the page title only animate when their state changes: a
//! screen that enables an already shown letterbox or title replays nothing,
//! so it plays no `LETTERBOX_IN` or `PAGETITLE_FLICKER`. `mbEnabled` feeds
//! `CycleLetterBox`, which freeskate_options' outro calls.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

#[derive(Clone)]
pub(super) struct LetterBox {
    /// `mMovieClip`.
    pub clip: Clip,
    /// The timelines' `Audio.PlaySound` requests.
    pub sounds: Vec<&'static str>,
    /// `mLetterBoxInfo`.
    info: bool,
    /// `mbEnabled`.
    enabled: bool,
}

impl LetterBox {
    /// The constructor stops and hides the movie; the prototype's flags are false.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut clip = Clip::new(assets, 11, 0)?;
        clip.playback.stop();
        clip.visible = false;
        Ok(Self {
            clip,
            sounds: Vec::new(),
            info: false,
            enabled: false,
        })
    }

    /// `EnableLetterBox` bfa6: `PlayIntro` bfee plays `niceintro` only while
    /// hidden; `PlayOutro` c034 takes the title out, then plays `niceoutro`.
    pub fn enable(&mut self, assets: &Assets, enable: bool) -> RenderResult<()> {
        self.enabled = enable;
        if enable {
            if !self.clip.visible {
                self.clip.visible = true;
                self.clip
                    .named(&["mcLetterbox"])?
                    .goto(assets, "niceintro")?;
            }
        } else {
            self.enable_info(assets, false)?;
            self.clip
                .named(&["mcLetterbox"])?
                .goto(assets, "niceoutro")?;
        }
        self.callbacks(assets)
    }

    /// `EnableLetterBoxInfo` be9d: `infointro`/`infooutro` only when
    /// `mLetterBoxInfo` changes.
    pub fn enable_info(&mut self, assets: &Assets, enable: bool) -> RenderResult<()> {
        if self.info != enable {
            self.info = enable;
            self.clip
                .named(&["mcLetterbox", "mInfo"])?
                .goto(assets, if enable { "infointro" } else { "infooutro" })?;
        }
        self.callbacks(assets)
    }

    /// `CycleLetterBox` bf0a: `cycle` while `mbEnabled`.
    pub fn cycle(&mut self, assets: &Assets) -> RenderResult<()> {
        if self.enabled {
            self.clip.named(&["mcLetterbox"])?.goto(assets, "cycle")?;
        }
        self.callbacks(assets)
    }

    /// `SetTitle` c06a: the text alone, no animation.
    pub fn set_title(&mut self, title: &str) -> RenderResult<()> {
        self.clip
            .named(&["mcLetterbox", "mInfo", "mTitle", "mTitleText", "labeltext"])?
            .text = Some(title.encode_utf16().collect());
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.clip.tick(assets)?;
        self.callbacks(assets)
    }

    /// mcLetterbox frame 58 (c21c) calls `EnableLetterBoxInfo(true)` before
    /// its stop and the empty `IntroDone`; frame 86 (c25c) stops and calls
    /// `OutroDone`, which hides the movie.
    fn callbacks(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.clip.take_calls(&mut calls);
        for call in calls {
            match call {
                "LetterIntroComplete" => self.enable_info(assets, true)?,
                "LetterOutroComplete" => self.clip.visible = false,
                "FE_SOUND_PAGETITLE_FLICKER" | "FE_SOUND_LETTERBOX_IN" | "FE_SOUND_LETTERBOX_OUT" => {
                    self.sounds.push(call)
                }
                _ => return Err(format!("untranslated LetterBox callback {call}").into()),
            }
        }
        Ok(())
    }
}
