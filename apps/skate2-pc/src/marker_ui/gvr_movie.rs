//! nis/gvrreg (asset 43, fingerprint e58a8c6e) and nis/gvrgoofy (asset 44,
//! fingerprint 9fa95bcf): `FrontEndState_NIS`'s stance badges
//! (`GVR_Regular` template 132, `GVR_Goofy` 133, type 0; overlay states 82
//! and 83 of the race starts). Each InitAction (gvrreg character 5 `0xdb0`,
//! gvrgoofy character 4 `0xd98`) defines the same class translated here;
//! the root action constructs it.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const REGULAR: usize = 43;
pub(super) const GOOFY: usize = 44;

pub(super) struct GvrMovie {
    pub root: Clip,
}

impl GvrMovie {
    /// The screen's movie runs its first frame: `new gvrreg(this)` (or
    /// `gvrgoofy`) moves `mScreen._x` by `Screen_EdgeOffset` (zero) and
    /// registers; nothing stops the root, which plays to its stop on frame
    /// 3 with the badge at alpha 0. The screen manager's `PlayClip(id,
    /// 'intro')` finds no `intro` label.
    pub fn new(assets: &Assets, asset: usize) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, asset, 0)? };
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
            if call != "gvr" {
                return Err(format!("untranslated stance badge callback {call}").into());
            }
        }
        Ok(())
    }

    /// `SetState`: 1 plays `gvrin`, -1 `gvrout`.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        match state {
            1 => self.root.goto(assets, "gvrin")?,
            -1 => self.root.goto(assets, "gvrout")?,
            _ => {}
        }
        self.calls()
    }
}

/// The InitActions defining each movie's guarded global class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (REGULAR, 5, 0xdb0) | (GOOFY, 4, 0xd98))
}

/// The root actions (`var screen = new gvrreg(this);`, gvrreg `0xd94`,
/// gvrgoofy `0xd7c`) and the stops ending the wait, `gvrin` and `gvrout`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match (clip.asset, offset) {
        (REGULAR, 0xd94) | (GOOFY, 0xd7c) => clip.calls.push("gvr"),
        (REGULAR, 0xf6c | 0xf78 | 0xf84) | (GOOFY, 0xf54 | 0xf60 | 0xf6c) => clip.playback.stop(),
        _ => return Err(format!("untranslated stance badge APT action {offset:#x}").into()),
    }
    Ok(())
}
