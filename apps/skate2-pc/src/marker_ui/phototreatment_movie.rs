//! The photo treatments, one class in three movies:
//! hudComponents/phototreatment (asset 41, fingerprint 56133c80; template 61
//! `PhotoTreatment`, overlay state 54), fixedphoto (asset 46, ba88d09d;
//! template 62 `FixedPhotoTreatment`, state 85 `Take_FixedPhoto`) and
//! nonfixedphoto (asset 47, 6218005f; template 63 `NonFixedPhotoTreatment`,
//! state 86 `Take_NonFixedPhoto`), all type 0. InitAction character 8 only
//! tests `_global.iscreenmodule`; character 9 defines the class translated
//! here; the root action constructs it.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 41;
pub(super) const FIXED: usize = 46;
pub(super) const NON_FIXED: usize = 47;

pub(super) struct PhotoTreatmentMovie {
    pub root: Clip,
    /// `Audio.PlaySound` names the timeline asked for.
    pub sounds: Vec<&'static str>,
}

impl PhotoTreatmentMovie {
    /// The screen's movie runs its first frame: the constructor stops the
    /// root, plays `snapshot_mc` from `widescreen` (`Screen_EdgeOffset` is
    /// zero) and registers. The screen manager's `PlayClip(id, 'intro')`
    /// finds no `intro` label.
    pub fn new(assets: &Assets, asset: usize) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, asset, 0)?, sounds: Vec::new() };
        movie.calls(assets)?;
        movie.root.goto(assets, "intro")?;
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
                "photo treatment" => {
                    self.root.playback.stop();
                    self.root.named(&["snapshot_mc"])?.goto(assets, "widescreen")?;
                }
                "FE_SOUND_PHOTO_FLYTHROUGH" | "FE_SOUND_PHOTO_SNAPPING" | "FE_SOUND_PHOTO_SKATE1" => self.sounds.push(call),
                _ => return Err(format!("untranslated photo treatment callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: 1 plays the root's `snapshot`; 0 does nothing.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        if state == 1 {
            self.root.goto(assets, "snapshot")?;
        }
        self.calls(assets)
    }
}

/// The InitActions: character 8's empty `iscreenmodule` test and character
/// 9's guarded global class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!(
        (asset, character, offset),
        (ASSET | NON_FIXED, 8, 0x84c) | (ASSET | NON_FIXED, 9, 0x884) | (FIXED, 8, 0x848) | (FIXED, 9, 0x880)
    )
}

/// The root action (`var screen = new phototreatment(this);`), the
/// `snapshot` frame's sound (`FE_SOUND_PHOTO_FLYTHROUGH`, `_SNAPPING` for
/// the fixed photo, `_SKATE1` for the other), and the stops of the root and
/// of `snapshot_mc`'s two layouts.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (ASSET | NON_FIXED, 0x82c) | (FIXED, 0x828) => clip.calls.push("photo treatment"),
        (ASSET, 0xa58) => clip.calls.push("FE_SOUND_PHOTO_FLYTHROUGH"),
        (FIXED, 0xa54) => clip.calls.push("FE_SOUND_PHOTO_SNAPPING"),
        (NON_FIXED, 0xa58) => clip.calls.push("FE_SOUND_PHOTO_SKATE1"),
        (ASSET | NON_FIXED, 0xa80 | 0xa90 | 0xaa0) | (FIXED, 0xa7c | 0xa8c | 0xa9c) => clip.playback.stop(),
        _ => return Err(format!("untranslated photo treatment APT action {offset:#x}").into()),
    }
    Ok(())
}
