//! nis/sanvantv (asset 45, fingerprint 45497183): `FrontEndState_NIS`'s San
//! Van TV logo (`NISSanVanTV` template 136, type 0; overlay state 84 of the
//! best trick scenes). Its InitAction (character 10, `0x3710`) defines the
//! `sanvantv` class translated here; root action `0x36f4` constructs it.
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 45;

pub(super) struct SanVanTvMovie {
    pub root: Clip,
}

impl SanVanTvMovie {
    /// The screen's movie runs its first frame: `new sanvantv(this)` stops
    /// the root and registers, `mLogo` stopping on its first frame. The
    /// screen manager's `PlayClip(id, 'intro')` plays the root's `intro`,
    /// which only runs to its stop on frame 29.
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie = Self { root: Clip::new(assets, ASSET, 0)? };
        movie.calls()?;
        movie.root.goto(assets, "intro")?;
        movie.calls()?;
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
                "sanvantv" => self.root.playback.stop(),
                _ => return Err(format!("untranslated San Van TV callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `SetState`: `kScreenState_Show` (0) plays `mLogo`'s `show`,
    /// `kScreenState_Clear` (-1) its `hide`; the overlay enters with 1,
    /// which the class ignores.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        match state {
            0 => self.root.named(&["mLogo"])?.goto(assets, "show")?,
            -1 => self.root.named(&["mLogo"])?.goto(assets, "hide")?,
            _ => {}
        }
        self.calls()
    }
}

/// sanvantv.apt's InitAction defines the guarded global class translated
/// above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 10, 0x3710))
}

/// Root action `0x36f4` (`var screen = new sanvantv(this);`), the root's
/// `intro` stop (`0x38e4`) and `mLogo`'s stops (`0x38e8` on its first
/// frame, `0x38f4` ending `show`, `0x3900` ending `hide`).
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x36f4 => clip.calls.push("sanvantv"),
        0x38e4 | 0x38e8 | 0x38f4 | 0x3900 => clip.playback.stop(),
        _ => return Err(format!("untranslated San Van TV APT action {offset:#x}").into()),
    }
    Ok(())
}
