//! challenge/ownthespot (asset 38, fingerprint 065eb3bf): `FrontEndState_NIS`'s
//! spot owned banner (`eSk8FEScreenState_SpotOwned`, the `SpotOwned_*`
//! overlay states). Its InitAction (character 30, `0x6ea4`) defines the
//! `ownthespot` class translated here; root action `0x6e90` constructs it and
//! stops the root. The screen manager creates it in its own layer (template
//! type 0), drawn flat.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 38;
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

pub(super) struct OwnTheSpotMovie {
    pub root: Clip,
    /// `Audio.PlaySound` names the timeline asked for.
    pub sounds: Vec<&'static str>,
}

impl OwnTheSpotMovie {
    /// The screen's movie runs its first frame: `new ownthespot(this)`
    /// (`mScreen.stop()`, `RegisterScreen`), then `stop()`. The screen
    /// manager's `PlayClip(id, 'intro')` finds no `intro` label, which
    /// `_gotoAndX` ignores.
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
                "ownthespot" => self.root.playback.stop(),
                "FE_SOUND_CHALLENGE_OTS" => self.sounds.push(call),
                _ => return Err(format!("untranslated spot owned callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: states 0 to 7 play their corner's intro or outro on the
    /// root; others do nothing.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        if let Some(label) = usize::try_from(state).ok().and_then(|s| LABELS.get(s)) {
            self.root.goto(assets, label)?;
        }
        self.calls()
    }
}

/// ownthespot.apt's InitAction defines the guarded global class translated
/// above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 30, 0x6ea4))
}

/// Root action `0x6e90` (`var ownSpot = new ownthespot(this); stop();`) and
/// the corners' frame actions: each intro plays
/// `_global.FE_SOUND_CHALLENGE_OTS` and shows the root, each end hides and
/// stops it.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x6e90 => {
            clip.calls.push("ownthespot");
            clip.playback.stop();
        }
        0x7224 | 0x7288 | 0x72e4 | 0x733c => {
            clip.calls.push("FE_SOUND_CHALLENGE_OTS");
            clip.visible = true;
        }
        0x727c | 0x72d8 | 0x7330 | 0x738c => {
            clip.visible = false;
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated spot owned APT action {offset:#x}").into()),
    }
    Ok(())
}
