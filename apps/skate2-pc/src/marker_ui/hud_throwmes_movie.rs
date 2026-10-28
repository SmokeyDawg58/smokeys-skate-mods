//! Original hud2/hudthrowmes (asset 55, fingerprint 0c72e72c; template entry
//! `ThrowDownMessageHUD`, placement 0, drawn flat) and its `hudthrowmes` class (InitAction
//! character 68, `0x6200`), driven by `ThrowDownMessageHUD` (`skate2_core::challenge::turn_hud`):
//! the S.K.A.T.E. letter banner and the one-up count. Its screen reads no data.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 55;

pub(super) struct ThrowDownMessageMovie {
    pub root: Clip,
    /// The instance's `mNextPlayState`: the `mSkate` label `PlayNextLetter` plays.
    next_play_state: &'static str,
    pub sounds: Vec<&'static str>,
}

impl ThrowDownMessageMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)?, next_play_state: "0", sounds: Vec::new() };
        movie.calls(assets)?;
        Ok(movie)
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)?;
        self.calls(assets)
    }

    /// The constructor, `mHolder`'s `PlayNextLetter` frames, `mSkate`'s letter sounds and its
    /// `_parent.gotoAndPlay("skateoutro")`.
    fn calls(&mut self, assets: &Assets) -> RenderResult<()> {
        let mut calls = Vec::new();
        self.root.take_calls(&mut calls);
        for call in calls {
            match call {
                "hudthrowmes" => self.construct(),
                "PlayNextLetter" => self.root.named(&["mHolder", "mSkate"])?.goto(assets, self.next_play_state)?,
                "SkateOutro" => self.root.named(&["mHolder"])?.goto(assets, "skateoutro")?,
                "SkateLetter" => self.sounds.push("FE_SOUND_SKATE_LETTER"),
                "SkateAllLetters" => self.sounds.push("FE_SOUND_SKATE_ALL_LETTERS"),
                _ => return Err(format!("untranslated throw-down message callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0x61d8`'s `new hudthrowmes(this)`, then `Stop`: the S.K.A.T.E. state, no
    /// letter queued, `Show(false)` (`mScreen._visible` set when it differs) and the screen
    /// registered.
    fn construct(&mut self) {
        self.next_play_state = "0";
        self.root.visible = false;
    }

    /// `SetState`: every state but -2 first runs `Show(true)`; -1 and -2 then end; 1 to 5 play
    /// the `skate` banner over the letters already earned and queue the next; 6, 7 and 8 play
    /// the `1up` banner's count.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        if state != -2 {
            self.root.visible = true;
        }
        let (stop, next) = match state {
            1 => (None, "1"),
            2 => (Some("1end"), "2"),
            3 => (Some("2end"), "3"),
            4 => (Some("3end"), "4"),
            5 => (Some("4end"), "5"),
            6..=8 => {
                self.root.named(&["mHolder"])?.goto(assets, "1up")?;
                let count = ["1", "2", "3"][usize::try_from(state - 6).unwrap_or(0)];
                return self.root.named(&["mHolder", "m1up"])?.goto(assets, count);
            }
            _ => return Ok(()),
        };
        self.root.named(&["mHolder"])?.goto(assets, "skate")?;
        let skate = self.root.named(&["mHolder", "mSkate"])?;
        match stop {
            None => skate.goto_number_and_stop(assets, 1)?,
            Some(label) => skate.goto_and_stop(assets, label)?,
        }
        self.next_play_state = next;
        Ok(())
    }
}

/// The movie's InitAction: the `hudthrowmes` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 68, 0x6200))
}

/// Timeline actions besides `Stop`: the root constructs the class; `mSkate` (character 43)
/// sounds each letter (`Audio.PlaySound`), the last as all letters, and sends its parent to
/// `skateoutro`; `mHolder` (character 67) calls `screen.PlayNextLetter`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x61d8 => {
            clip.calls.push("hudthrowmes");
            clip.playback.stop();
        }
        0x6588 | 0x65cc | 0x6610 | 0x6654 => clip.calls.push("SkateLetter"),
        0x6698 => clip.calls.push("SkateAllLetters"),
        0x65a4 | 0x65e8 | 0x662c | 0x6670 => clip.calls.push("SkateOutro"),
        0x66b4 => {
            clip.playback.stop();
            clip.calls.push("SkateOutro");
        }
        0x6700 | 0x6734 => clip.calls.push("PlayNextLetter"),
        _ => return Err(format!("untranslated throw-down message APT action {offset:#x}").into()),
    }
    Ok(())
}
