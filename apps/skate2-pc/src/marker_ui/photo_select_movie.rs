//! `FrontEndState_PhotoChallenge`'s screen 116, challenge/photoselect (asset 73, template entry
//! `0x82fdba98` `PhotoSelect`, placement 6, `g3dHUDRight`). The InitAction (character 52,
//! `0x3908`) defines the `photoselect` class translated here and root action `0x38ac`
//! constructs it; its `OnLoaded`
//! reads `HUDComponents.PhotoSelect_IsSelectionAvail` (`0x826bb310`) and
//! `PhotoSelect_GetNumPhotos` (`0x826bb340`), which answer from [`PhotoSelectView`]. The
//! buttons are the shared Button Item, named by their placements' load events (Y, the right
//! and the left trigger).
use super::{assets::Assets, clips::Clip};
use crate::gpu_renderer::RenderResult;
use skate2_core::PhotoSelectView;

pub(super) const ASSET: usize = 73;
/// `photoselect.kMaxNumPhotos`.
const MAX_PHOTOS: i32 = 8;

#[derive(Default)]
pub(super) struct PhotoSelectMovie {
    pub root: Option<Clip>,
    /// The photo state is open: `OnEnter` opened the screen and `PhotoDone`'s `SetState(8)`
    /// has not closed it.
    pub open: bool,
    /// The instance's `mNumPhotos`.
    num_photos: i32,
}

impl PhotoSelectMovie {
    /// `OnEnter`'s `OpenScreen(116)` for a newly open state: a fresh screen, constructed and
    /// loaded, then its `intro`; the state's `SetScreenState` calls in order; once it is gone,
    /// `CloseScreen` plays the `outro`.
    pub fn update(&mut self, assets: &Assets, view: Option<&PhotoSelectView>, states: &[i32]) -> RenderResult<()> {
        if let Some(view) = view
            && !self.open
        {
            self.open = true;
            let mut root = Clip::new(assets, ASSET, 0)?;
            let mut calls = Vec::new();
            root.take_calls(&mut calls);
            for call in calls {
                match call {
                    "photoselect" => {}
                    _ => return Err(format!("untranslated photo select callback {call}").into()),
                }
            }
            self.num_photos = 0;
            self.on_loaded(&mut root, assets, view)?;
            root.goto(assets, "intro")?;
            self.root = Some(root);
            self.calls()?;
        }
        for &state in states {
            self.set_state(assets, state)?;
        }
        if view.is_none() && self.open {
            self.open = false;
            if let Some(root) = self.root.as_mut() {
                root.goto(assets, "outro")?;
            }
            self.calls()?;
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()
    }

    /// The timeline's calls: frame 10's `IntroComplete` and frame 22's `OutroComplete`, which
    /// unloads the screen.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "IntroComplete" => {}
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated photo select callback {call}").into()),
            }
        }
        Ok(())
    }

    /// `OnLoaded`: without a choice the step buttons hide and the title reads
    /// `ID_COMMON_CONTINUE`; with one, `mNumPhotos` is the photo count and the black bar plays
    /// the label of the unused slots (`kMaxNumPhotos - mNumPhotos`).
    fn on_loaded(&mut self, root: &mut Clip, assets: &Assets, view: &PhotoSelectView) -> RenderResult<()> {
        if view.enable_picking {
            self.num_photos = view.num_photos;
            root.named(&["blackbar_mc"])?.goto(assets, &(MAX_PHOTOS - self.num_photos).to_string())?;
        } else {
            for path in [["controllerButtons_mc", "prevButton"], ["controllerButtons_mc", "nextButton"], ["mainButtons_mc", "prev_mc"], ["mainButtons_mc", "next_mc"]] {
                root.named(&path)?.visible = false;
            }
            for name in ["mcTitle", "mcTitleGlow"] {
                root.named(&["title_mc", name])?.text = Some(assets.initial_text("ID_COMMON_CONTINUE"));
            }
        }
        Ok(())
    }

    /// `SetState`: -1 and -2 play the next and previous buttons' `active`, -3 the take-photo
    /// button's `glow`, -10 the screen's `fakeoutro`; a photo index the bar's frame label one
    /// past it.
    fn set_state(&mut self, assets: &Assets, state: i32) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        match state {
            -1 => root.named(&["mainButtons_mc", "next_mc"])?.goto(assets, "active")?,
            -2 => root.named(&["mainButtons_mc", "prev_mc"])?.goto(assets, "active")?,
            -3 => root.named(&["mainButtons_mc", "takePhoto_mc"])?.goto(assets, "glow")?,
            -10 => root.goto(assets, "fakeoutro")?,
            state if (0..self.num_photos).contains(&state) => root.named(&["bar_mc"])?.goto(assets, &(state + 1).to_string())?,
            _ => {}
        }
        self.calls()
    }
}

/// photoselect.apt's InitAction defines the guarded global class translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 52, 0x3908))
}

/// Root actions besides `Stop`: frame 0's construction, frame 10's `IntroComplete` (then
/// `stop()`) and frame 22's `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x38ac => clip.calls.push("photoselect"),
        0x3c54 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0x3c98 => clip.calls.push("OutroComplete"),
        _ => return Err(format!("untranslated photo select APT action {offset:#x}").into()),
    }
    Ok(())
}
