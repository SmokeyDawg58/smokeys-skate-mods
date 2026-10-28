//! nis/gestollie (asset 37, fingerprint 4e5752ee): `FrontEndState_NIS`'s
//! ollie gesture tip (`eSk8FEScreenState_TutorialGesture_Ollie`, the
//! `TutorialGesture_OlliePt1`/`Pt2` overlay states). Its InitAction
//! (character 31, `0x36f0`) defines the `gestollie` class translated here and
//! root action `0x36c4` constructs it. The screen manager creates it in its
//! own layer (template type 0, `mScreen.createEmptyMovieClip`), drawn flat.
use super::{
    assets::Assets,
    clips::Clip,
    objective_movie::set_y,
    pause_movie::child,
    tutorial_movie::{max_scroll, set_height},
};
use crate::gpu_renderer::RenderResult;

pub(super) const ASSET: usize = 37;
/// `gestollie`'s backer growth per label line past two.
const LINE_HEIGHT: f32 = 22.;

pub(super) struct GestOllieMovie {
    pub root: Clip,
}

impl GestOllieMovie {
    /// The screen's movie runs its first frame: `new gestollie(this)` and its
    /// `RegisterScreen` (`ScreenManager.OnLoaded`), whose
    /// `InitialStandardScreenReady` callback plays the root's `intro`
    /// (`PlayClip`: `gotoAndPlay`).
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut root = Clip::new(assets, ASSET, 0)?;
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "gestollie" => construct(&mut root, assets)?,
                _ => return Err(format!("untranslated gesture tip callback {call}").into()),
            }
        }
        root.goto(assets, "intro")?;
        Ok(Self { root })
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        self.root.tick(assets)
    }

    /// `SetState`: `kScreenState_OlliePt1` (1) plays `mName`'s `olliedown`,
    /// `kScreenState_OlliePt2` (2) its `ollieup`; others do nothing.
    pub fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        let label = match state {
            1 => "olliedown",
            2 => "ollieup",
            _ => return Ok(()),
        };
        self.root.named(&["mName"])?.goto(assets, label)
    }
}

/// `new gestollie(screen)`: `mScreen._x -= Screen_EdgeOffset` subtracts zero;
/// `autoSize = "center"` on the multiline labels moves nothing
/// (`SkateAptString` `0x82a70b28` keeps a multiline field's box); a label
/// wrapped past two lines (`maxscroll > 2`) lowers `mBackerbottom` and
/// `mBackerreflect` 22 per extra line and stretches `mBackercore` down to
/// `mBackerbottom`; `mScreen.stop()`.
fn construct(root: &mut Clip, assets: &Assets) -> RenderResult<()> {
    let backer = root.named(&["mBacker"])?;
    let lines = max_scroll(backer.named(&["label"])?, assets)?;
    if lines > 2 {
        #[expect(clippy::cast_precision_loss, reason = "a line count")]
        let extra = (lines - 2) as f32 * LINE_HEIGHT;
        let bottom = child(backer, "mBackerbottom")?;
        let bottom_y = bottom.entry.matrix.ty + extra;
        set_y(bottom, bottom_y);
        let reflect = child(backer, "mBackerreflect")?;
        let reflect_y = reflect.entry.matrix.ty + extra;
        set_y(reflect, reflect_y);
        let core = child(backer, "mBackercore")?;
        let core_y = core.entry.matrix.ty;
        set_height(core, assets, bottom_y - core_y)?;
    }
    root.playback.stop();
    Ok(())
}

/// gestollie.apt's sole InitAction defines the guarded global class
/// translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 31, 0x36f0))
}

/// Root action `0x36c4`, `var screen = new gestollie(this)`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x36c4 => clip.calls.push("gestollie"),
        _ => return Err(format!("untranslated gesture tip APT action {offset:#x}").into()),
    }
    Ok(())
}
