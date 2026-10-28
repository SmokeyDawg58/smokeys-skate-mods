//! challenge/killedit (asset 40, fingerprint c36cd0da): `FrontEndState_NIS`'s
//! killed it banner (`eSk8FEScreenState_SpotKilled`, the `KilledIt*` overlay
//! states, opened into `mKilledItScreenId`). Its InitAction (character 67,
//! `0x1fb5c`) defines the `killedit` class translated here; root action
//! `0x1fb48` constructs it and stops the root. The screen manager creates it
//! in its own layer (template type 0), drawn flat.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 40;

pub(super) struct KilledItMovie {
    pub root: Clip,
    /// `Audio.PlaySound` names the timeline asked for.
    pub sounds: Vec<&'static str>,
}

impl KilledItMovie {
    /// The screen's movie runs its first frame: `new killedit(this)`
    /// (`RegisterScreen`), then `stop()`. The screen manager's
    /// `PlayClip(id, 'intro')` finds no `intro` label.
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
                "killedit" => self.root.playback.stop(),
                "FE_SOUND_KILLEDIT_INTRO" | "FE_SOUND_KILLEDIT_FLOURISH" | "FE_SOUND_KILLEDIT_OUTRO" => self.sounds.push(call),
                _ => return Err(format!("untranslated killed it callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: `kIntro_TopLeft` (1), `kIntro_TopRight` (2),
    /// `kIntro_BottomLeft` (3) and `kIntro_BottomRight` (4) play their
    /// corner, which runs its flourish and outro by itself; their
    /// `logoAnimation.gotoAndPlay('intro')`, like `kOutro`'s (5) `'outro'`,
    /// finds no such child (the movie names no instances).
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        let label = match state {
            1 => "topleft_intro",
            2 => "topright_intro",
            3 => "bottomleft_intro",
            4 => "bottomright_intro",
            _ => return Ok(()),
        };
        self.root.goto(assets, label)?;
        self.calls()
    }
}

/// killedit.apt's InitAction defines the guarded global class translated
/// above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 67, 0x1fb5c))
}

/// Root action `0x1fb48` (`var killit = new killedit(this); stop();`) and the
/// corners' sound frames: intro, flourish and outro of bottom left, bottom
/// right, top left and top right.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x1fb48 => {
            clip.calls.push("killedit");
            clip.playback.stop();
        }
        0x1fe4c | 0x1fec0 | 0x1ff34 | 0x1ffa4 => clip.calls.push("FE_SOUND_KILLEDIT_INTRO"),
        0x1fe78 | 0x1feec | 0x1ff5c | 0x1ffcc => clip.calls.push("FE_SOUND_KILLEDIT_FLOURISH"),
        0x1fe9c | 0x1ff10 | 0x1ff80 | 0x1fff0 => clip.calls.push("FE_SOUND_KILLEDIT_OUTRO"),
        _ => return Err(format!("untranslated killed it APT action {offset:#x}").into()),
    }
    Ok(())
}
