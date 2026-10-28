//! Original hud2/hudabd (asset 56, fingerprint f7b8bcef; template entry `ABDHUD` 0x82fdb878,
//! placement 5, the HUD layer under `Focus_Left`) and its `hudabd` class (InitAction character
//! 33, `0x61d4`), driven by `ABDHUD` (`skate2_core::challenge::turn_hud`): the S.K.A.T.E. list
//! of tricks already done, the trick to copy and the copy attempt. `SetScreenState` runs the
//! class's `SetState` at once, so each state is applied with the data it carries. The two
//! gestures are controls/gesture_item instances in the challenge's stance mode.
use super::{assets::Assets, clips::Clip, localizer, tutorial_movie};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::{leaderboard_hud::AbdHud, turn_hud::TurnHudView};
use skate2_formats::localization::string_hash;

pub(super) const ASSET: usize = 56;
/// The panels `SetState` 100 and 101 show and hide.
const PANELS: [&str; 3] = ["mcTitle", "mcTrick_set", "mcTrick_copy"];

pub(super) struct AbdMovie {
    pub root: Clip,
    /// `mNumActiveTricks`.
    num_active_tricks: i32,
    /// `mMasterGestureVisibility`.
    master_gesture_visibility: bool,
    pub sounds: Vec<&'static str>,
}

impl AbdMovie {
    pub fn new(assets: &Assets) -> RenderResult<Self> {
        let mut movie =
            Self { root: Clip::new(assets, ASSET, 0)?, num_active_tricks: 0, master_gesture_visibility: true, sounds: Vec::new() };
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
                "hudabd" => self.construct(assets)?,
                _ => return Err(format!("untranslated ABD HUD callback {call}").into()),
            }
        }
        Ok(())
    }

    /// Root action `0x6188`'s `new hudabd(this)`, then `Stop`: no tricks, `mcABD` at `low`, the
    /// gestures shown by the master visibility, `Show(false)`, the screen registered, and both
    /// gestures hidden and put in the challenge's stance mode (`SetUseSkateChallengeSettings`).
    /// `mScreen._x += Screen_EdgeOffset` adds zero.
    fn construct(&mut self, assets: &Assets) -> RenderResult<()> {
        self.num_active_tricks = 0;
        self.root.named(&["mcABD"])?.goto(assets, "low")?;
        self.master_gesture_visibility = true;
        self.root.visible = false;
        self.root.named(&["mcTrick_set", "mGestureComp"])?.visible = false;
        self.root.named(&["mcTrick_copy", "mGestureCompX"])?.visible = false;
        Ok(())
    }

    /// `SetState`: 1 shows the screen and updates everything, 2 hides it, 3 updates
    /// everything, 4 the list, 5 the trick set and 6 the copy attempt; 7 and 8 show and hide
    /// the list; 100 and 101 show and hide the trick panels. `ABD_GetMoveToUpperPosition`
    /// (`0x826bb6a8`), which 101 asks before moving `mcABD` to `high`, is false: only
    /// `HUD2.ShowTrickDisplayContext` clears the trick display's `mContextTrickVisible`, and no
    /// S.K.A.T.E. script calls it.
    pub fn set_state(&mut self, assets: &Assets, state: i32, view: &TurnHudView) -> RenderResult<()> {
        let data = || match view {
            TurnHudView::Abd { data, stance_regular } => Ok((data, *stance_regular)),
            _ => Err(format!("ABD HUD state {state} without ABD data")),
        };
        match state {
            1 => {
                self.root.visible = true;
                let (data, regular) = data()?;
                self.update_all(assets, data, regular)?;
            }
            2 => self.root.visible = false,
            3 => {
                let (data, regular) = data()?;
                self.update_all(assets, data, regular)?;
            }
            4 => self.update_list(assets, data()?.0)?,
            5 => {
                let (data, regular) = data()?;
                self.update_set_trick(assets, data, regular)?;
            }
            6 => {
                let (data, regular) = data()?;
                self.update_copy_trick(assets, data, regular)?;
            }
            7 | 8 => self.root.named(&["mcABD", "mPositionHolder"])?.visible = state == 7,
            100 | 101 => {
                self.master_gesture_visibility = state == 100;
                for panel in PANELS {
                    self.root.named(&[panel])?.visible = state == 100;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn update_all(&mut self, assets: &Assets, data: &AbdHud, regular: bool) -> RenderResult<()> {
        self.update_list(assets, data)?;
        self.update_set_trick(assets, data, regular)?;
        self.update_copy_trick(assets, data, regular)
    }

    /// `UpdateList`: a new count plays `mPositionHolder` from `<count>tricks`; each listed trick
    /// names its row (`ABD_GetTrickHalNames` `0x826bb3b0`).
    fn update_list(&mut self, assets: &Assets, data: &AbdHud) -> RenderResult<()> {
        let count = data.num_tricks_in_list;
        if self.num_active_tricks != count {
            self.root.named(&["mcABD", "mPositionHolder"])?.goto(assets, &format!("{count}tricks"))?;
        }
        self.num_active_tricks = count;
        for (i, item) in data.list.iter().enumerate().take(usize::try_from(count).unwrap_or(0)) {
            let text = trick_text(assets, &item.hal_name, item.spin_degrees)?;
            let row = self.root.named(&["mcABD", "mPositionHolder", &format!("mcTrickName{i}"), "mcAbdtext"])?;
            row.named(&["labelText"])?.text = Some(text.clone());
            row.named(&["labelTextGlow"])?.text = Some(text);
        }
        Ok(())
    }

    /// `UpdateSetTrick`: the set trick's gesture shows unless it is `none` (and while the master
    /// visibility allows), its name fills `mcTrick_set`, and the gesture is set in the local
    /// skater's stance (`ABD_GetTrickSetData` `0x826bb548`).
    fn update_set_trick(&mut self, assets: &Assets, data: &AbdHud, regular: bool) -> RenderResult<()> {
        if data.set_gesture == "none" {
            self.root.named(&["mcTrick_set", "mGestureComp"])?.visible = false;
        } else if self.master_gesture_visibility {
            self.root.named(&["mcTrick_set", "mGestureComp"])?.visible = true;
        }
        let text = trick_text(assets, &data.set_hal_name, data.set_spin)?;
        self.root.named(&["mcTrick_set", "mcTrick", "labeltext"])?.text = Some(text);
        set_gesture(self.root.named(&["mcTrick_set", "mGestureComp"])?, assets, regular, &data.set_gesture)
    }

    /// `UpdateCopyTrick`: with no set trick the title asks for one and the copy panel hides;
    /// otherwise (while the master visibility allows) the title asks for the copy and the panel
    /// shows. A copied gesture plays `success` or `failed` with its sound by whether the name
    /// and spin match the set trick's, else `normal`; the copy's name and gesture follow
    /// (`ABD_GetTricCopyData` `0x826bb5f8`). `SkateChallenegeSetGest` reaches the set gesture
    /// too, which faces by it at its next `SetGestureType`.
    fn update_copy_trick(&mut self, assets: &Assets, data: &AbdHud, regular: bool) -> RenderResult<()> {
        let title = if data.set_gesture == "none" {
            Some(("ID_SKATE_SET_TRICK", false))
        } else if self.master_gesture_visibility {
            Some(("ID_CHALLENGE_CENTRAL_MESSAGE_NAMECOPYTRICKNAME", true))
        } else {
            None
        };
        if let Some((label, visible)) = title {
            let text = assets.initial_text(label);
            self.root.named(&["mcTitle", "label"])?.text = Some(text.clone());
            self.root.named(&["mcTitle", "labelGlow"])?.text = Some(text);
            self.root.named(&["mcTrick_copy"])?.visible = visible;
            self.root.named(&["mcTrick_copy", "mGestureCompX"])?.visible = visible;
        }
        let copied = data.copy_hal_name == data.set_hal_name && data.copy_spin == data.set_spin;
        let panel = self.root.named(&["mcTrick_copy"])?;
        if data.copy_gesture == "none" {
            panel.goto(assets, "normal")?;
        } else if copied {
            self.sounds.push("FE_SOUND_GESTURE_COPY_SUCCESS");
            panel.goto(assets, "success")?;
        } else {
            self.sounds.push("FE_SOUND_GESTURE_COPY_FAIL");
            panel.goto(assets, "failed")?;
        }
        let text = trick_text(assets, &data.copy_hal_name, data.copy_spin)?;
        self.root.named(&["mcTrick_copy", "mcTrick", "labeltext"])?.text = Some(text);
        set_gesture(self.root.named(&["mcTrick_copy", "mGestureCompX"])?, assets, regular, &data.copy_gesture)
    }
}

/// A trick's text: its HAL name, which the text field localizes, or with a spin
/// `FELanguage.GetLocalizedFormatString(name, 0)` (`0x82622c00`, `FormatString` without
/// arguments), a space and the degrees.
fn trick_text(assets: &Assets, name: &str, spin: i32) -> RenderResult<Vec<u16>> {
    if spin <= 0 {
        return Ok(assets.initial_text(name));
    }
    let mut text = localizer::format_hal(assets, string_hash(name.as_bytes()))?;
    text.extend(format!(" {spin}").encode_utf16());
    Ok(text)
}

/// `SkateChallenegeSetGest(regular)` then `SetGestureType(gesture)` on a gesture item in the
/// challenge's stance mode (`mUseSkateSettings`): `UpdateGestureForStance` faces the new
/// gesture by `mCurrentStanceIsReg`.
fn set_gesture(item: &mut Clip, assets: &Assets, regular: bool, gesture: &str) -> RenderResult<()> {
    tutorial_movie::set_gesture_type(item, assets, gesture)?;
    tutorial_movie::face_gesture(item, regular);
    Ok(())
}

/// The movie's InitAction: the `hudabd` class.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 33, 0x61d4))
}

/// Timeline actions besides `Stop`: the root constructs the class.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x6188 => {
            clip.calls.push("hudabd");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated ABD HUD APT action {offset:#x}").into()),
    }
    Ok(())
}
