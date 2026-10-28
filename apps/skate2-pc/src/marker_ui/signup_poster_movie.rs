//! `FrontEndState_SignUpPoster`'s screen 19, challenge/challenge_signup
//! (asset 33, fingerprint 711e9b3c, the retail game's 2600ff8b; template entry `0x82fdb488`
//! `SignUp_Poster`, placement 0, drawn flat), and its imported
//! `challenge_info` (asset 5) in `SetMode(2)`, the sign-up mode
//! (`0x4bb8`..`0x4e98`), laid out by `UpdatePositions` (`0x3ec4`). The
//! screen's InitAction (character 66, `0x5ff4`) defines the class translated
//! here; root action `0x5fc0` constructs it. What the natives answer comes
//! from `skate2_core::challenge::signup_poster`.
use super::{
    assets::Assets,
    clips::{Child, Clip},
    localizer,
    objective_movie::{set_x, set_y},
    pause_movie::{bounds, child, configure_left_buttons, set_wallet},
    tutorial_movie::max_scroll,
};
use crate::gpu_renderer::RenderResult;
use skate2_core::challenge::signup_poster::{BetView, PosterText, PosterView};

pub(super) const ASSET: usize = 33;
/// `SetProceduralProperty`'s smallest scale (`0x829ea608`).
const MIN_SCALE: f32 = f32::from_bits(0x38d1_b717);
/// The entry fee's colour when the cash does not cover it (`textColor =
/// 11221784`).
const FEE_SHORT: u32 = 0xff00_0000 | 11_221_784;
/// `UpdatePositions`' fixed rows for the sign-up mode: the button menu and
/// the tutorial-video option.
const VIEW_Y: f32 = 577.;
const NIS_OPTION_Y: f32 = 525.;

#[derive(Default)]
pub(super) struct SignUpPosterMovie {
    pub root: Option<Clip>,
    /// The poster state is open: `OnEnter` opened the screen and `PopState`
    /// has not closed it. The state hides the in-game HUD
    /// (`gFEStateInfo[32].mShowHUD` = `eHUDState_Hide`, applied by
    /// `UpdateOnEnter` `0x82d663f0`).
    pub open: bool,
    intro_complete: bool,
    /// `mVignette`'s x scale and x as the constructor left them.
    vignette: Option<(f32, f32)>,
    last_bet: Option<BetView>,
}

impl SignUpPosterMovie {
    /// `OnEnter`'s `OpenScreen(19)` for a newly open poster: a fresh screen,
    /// constructed, then its intro; `OnLeave`'s `CloseScreen` once the state
    /// pops: its outro.
    pub fn update(&mut self, assets: &Assets, view: Option<&PosterView>, window: (u32, u32)) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                self.intro_complete = false;
                self.last_bet = view.betting.then_some(view.bet);
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "challenge_signup" => construct(&mut root, assets, view)?,
                        _ => return Err(format!("untranslated sign-up poster callback {call}").into()),
                    }
                }
                let vignette = child(root.named(&["vignette_mc"])?, "mVignette")?;
                self.vignette = Some((vignette.entry.matrix.a, vignette.entry.matrix.tx));
                root.goto(assets, "intro")?;
                self.root = Some(root);
                self.calls()?;
            }
            None if self.open => {
                self.open = false;
                self.last_bet = None;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
                self.calls()?;
            }
            _ => {}
        }
        if let (Some(root), Some(view)) = (self.root.as_mut(), view)
            && view.betting && self.last_bet != Some(view.bet)
        {
            update_bet(root.named(&["mBackTop", "mcChallengeInfo"])?, assets, view.bet)?;
            self.last_bet = Some(view.bet);
        }
        if let (Some(root), Some(constructed)) = (self.root.as_mut(), self.vignette) {
            fit_vignette(root, assets, constructed, window)?;
        }
        Ok(())
    }

    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()
    }

    /// The screen's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// The frame scripts' calls: frame 1's `PlayIntroAudio`
    /// (`FE_SOUND_CHALLENGE_TRANSITION`) and frame 57's
    /// `Audio.PlaySignupOut` (FE sounds are not played), frame 56's
    /// `IntroComplete` and frame 69's `OutroComplete`, which unloads it.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "PlayIntroAudio" | "PlaySignupOut" => {}
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated sign-up poster callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// `new challenge_signup(this)`: `mScreen.stop()`, the button backer's
/// `tutorial` or `normal` frames by `HasTutorialNIS`,
/// `mcChallengeInfo.SetMode(2)`, the edge offset (zero) added to
/// `mcChallengeInfo._x` and `mBacker._x` and half of it taken from
/// `mVignette._width`; `RegisterScreen` hands the screen to the
/// `ScreenManager`.
fn construct(root: &mut Clip, assets: &Assets, view: &PosterView) -> RenderResult<()> {
    root.playback.stop();
    let backer = root.named(&["mBackTop", "mBacker", "mButtonbacker"])?;
    backer.goto(assets, if view.tutorial_nis.is_some() { "tutorial" } else { "normal" })?;
    let back_top = root.named(&["mBackTop"])?;
    set_mode_signup(&mut child(back_top, "mcChallengeInfo")?.clip, assets, view)?;
    for name in ["mcChallengeInfo", "mBacker"] {
        let piece = child(back_top, name)?;
        set_x(piece, piece.entry.matrix.tx);
    }
    let vignette = child(root.named(&["vignette_mc"])?, "mVignette")?;
    let width = width(vignette, assets)?;
    set_width(vignette, assets, width)
}

/// A window wider than 16:9 shows the flat stage pillarboxed. The full-screen `mVignette` alone
/// is widened about its centre by the stage width beyond 1280, so the screen's frame reaches the
/// window's edges; at 16:9 or narrower it keeps the constructor's scale and x.
fn fit_vignette(root: &mut Clip, assets: &Assets, (a, tx): (f32, f32), (width, height): (u32, u32)) -> RenderResult<()> {
    #[expect(clippy::cast_precision_loss, reason = "window pixels as a ratio")]
    let aspect = width as f32 / height.max(1) as f32;
    let parent = child(root, "vignette_mc")?.entry.matrix.a;
    let extra = 1280. * (aspect / (16. / 9.) - 1.).max(0.) / parent;
    let vignette = child(root.named(&["vignette_mc"])?, "mVignette")?;
    let r = bounds(&vignette.clip, assets)?.ok_or("sign-up poster vignette has no bounds")?;
    let constructed = (r.right - r.left) * a;
    if constructed <= 0. {
        return Ok(());
    }
    let (scale, x) = if extra > 0. {
        let scale = a * (constructed + extra) / constructed;
        (scale, tx + a * r.left - extra / 2. - scale * r.left)
    } else {
        (a, tx)
    };
    vignette.entry.matrix.a = scale;
    vignette.entry.matrix.tx = x;
    vignette.clip.script_transform = true;
    Ok(())
}

/// A poster native's string as its text field shows it: the localizer's
/// `FormatStringCustom` output or the native's literal.
fn poster_text(assets: &Assets, text: &PosterText) -> RenderResult<Vec<u16>> {
    if text.formatted { localizer::format_custom(assets, &text.text, "") } else { Ok(assets.initial_text(&text.text)) }
}

/// challenge_info `SetMode(2)` (`0x482e`): the screen shown and every part
/// disabled (`mHeader` stays), then the sign-up mode, then
/// `UpdatePositions`.
fn set_mode_signup(info: &mut Clip, assets: &Assets, view: &PosterView) -> RenderResult<()> {
    info.visible = true;
    for part in info.children.values_mut() {
        part.clip.visible = part.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case("mHeader"));
    }
    let title = poster_text(assets, &view.title)?;
    info.named(&["mHeader", "labelText"])?.text = Some(title.clone());
    info.named(&["mDescription", "labelText"])?.text = Some(poster_text(assets, &view.description)?);
    if view.entry_fee > 0 {
        child(info, "mEntryFee")?.clip.visible = true;
        let value = info.named(&["mEntryFee", "mEntryFeeValue"])?;
        if view.entry_fee_paid {
            value.text = Some(assets.initial_text("ID_CHALLENGEINFO_ENTRY_FEE_PAID"));
        } else {
            if view.cash < view.entry_fee {
                value.properties.insert("text_colour".into(), FEE_SHORT.to_string());
            }
            #[expect(clippy::cast_precision_loss, reason = "the native's (float) of the fee")]
            let fee = view.entry_fee as f32;
            value.text = Some(localizer::format_currency(assets, fee).encode_utf16().collect());
        }
    }
    if view.description_enabled {
        child(info, "mDescription")?.clip.visible = true;
    }
    child(info, "mView")?.clip.visible = true;
    if view.betting {
        child(info, "mBetting")?.clip.visible = true;
        update_bet(info, assets, view.bet)?;
        #[expect(clippy::cast_precision_loss, reason = "Cash_GetFormattedCashString converts the native integer to float")]
        let cash = localizer::format_currency(assets, view.cash as f32).encode_utf16().collect::<Vec<_>>();
        set_wallet(info, assets, &cash)?;
        if !view.enough_for_min_bet {
            let betting = info.named(&["mBetting"])?;
            betting.named(&["mcBetArrows", "amountText"])?.properties.insert("text_colour".into(), "4294901760".into());
            betting.named(&["mcBetArrows", "totalAmountText"])?.visible = false;
            let warning = betting.named(&["totalbetText"])?;
            warning.text = Some(assets.initial_text("ID_SIGNUP_NOT_ENOUGH_CASH_FOR_BETTING"));
            warning.properties.insert("text_colour".into(), "4294901760".into());
        }
    }
    if let Some(hal) = &view.tutorial_nis {
        child(info, "mNISOption")?.clip.visible = true;
        info.named(&["mNISOption", "labeltext"])?.text = Some(assets.initial_text(hal));
    }
    let mut items = Vec::new();
    if !view.betting || view.enough_for_min_bet {
        items.push(("A", "ID_COMMON_ACCEPT"));
    }
    items.push(("B", "ID_COMMON_DECLINE"));
    configure_left_buttons(info.named(&["mView"])?, assets, &items)?;
    // mTitleOffsetX and mTitleOffsetY are the prototype's zeros.
    let header = child(info, "mHeader")?;
    set_x(header, 0.);
    set_y(header, 0.);
    let objectives = if view.objectives_enabled { init_objectives(info, assets, view)? } else { 0 };
    if view.competitors_enabled && view.competitor_challenge {
        eprintln!("sign-up poster: the competitors list is not ported");
    }
    update_positions(info, assets, &title, objectives)?;
    Ok(())
}

/// challenge_info BetMore/BetLess (0x3a88/0x3ae0) and UpdateBettingArrows (0x3b38).
/// GetPlayerBet82610138/GetPotTotal82610230 return localized currency strings.
#[expect(clippy::cast_precision_loss, reason = "native FormatCurrency casts the signed amount to float")]
fn update_bet(info: &mut Clip, assets: &Assets, bet: BetView) -> RenderResult<()> {
    let arrows = info.named(&["mBetting", "mcBetArrows"])?;
    arrows.named(&["amountText"])?.text = Some(localizer::format_currency(assets, bet.bet as f32).encode_utf16().collect());
    arrows.named(&["totalAmountText"])?.text = Some(localizer::format_currency(assets, bet.pot as f32).encode_utf16().collect());
    arrows.named(&["mArrowRight"])?.visible = bet.can_bet_more;
    arrows.named(&["mArrowLeft"])?.visible = bet.can_bet_less;
    Ok(())
}

/// `InitObjectives` (`0x5399`): with any objective, `EnableObjectives`
/// (the objective text hidden), the four rows hidden, then each objective's
/// title, left-auto-sized, in its row. Returns `mNumOptions`.
fn init_objectives(info: &mut Clip, assets: &Assets, view: &PosterView) -> RenderResult<usize> {
    let count = view.objectives.len();
    if count == 0 {
        return Ok(0);
    }
    child(info, "mObjectiveText")?.clip.visible = false;
    child(info, "mObjectives")?.clip.visible = true;
    let list = info.named(&["mObjectives"])?;
    for i in 0..4 {
        child(list, &format!("mObjective{i}"))?.clip.visible = false;
    }
    for (i, (hal, arguments)) in view.objectives.iter().enumerate() {
        let row = child(list, &format!("mObjective{i}"))?;
        row.clip.named(&["labelText"])?.text = Some(localizer::format_custom(assets, hal, arguments)?);
        row.clip.visible = true;
    }
    Ok(count)
}

/// `UpdatePositions` (`0x3ec4`) for the parts the sign-up mode shows: below
/// the header (10 apart), a header that wraps at its large size takes the
/// `smalltext` frames and, fitting one line there, `smalltextcenter`; the
/// description 5 lower, 16 per line plus 15; the objective rows 26 per line
/// under their title; the entry fee 10 lower, its height plus 10; the button
/// menu and the tutorial-video option at fixed heights. Multiline
/// auto-sized fields keep their authored bounds (`SkateAptString`
/// `0x82a70b28`). Returns `mYOffset`.
fn update_positions(info: &mut Clip, assets: &Assets, title: &[u16], objectives: usize) -> RenderResult<f32> {
    let header = child(info, "mHeader")?;
    let mut y = header.entry.matrix.ty + height(header, assets)? + 10.;
    let header = &mut header.clip;
    if max_scroll(header.named(&["labelText"])?, assets)? > 1 {
        header.goto(assets, "smalltext")?;
        header.named(&["labelText"])?.text = Some(title.to_vec());
        if max_scroll(header.named(&["labelText"])?, assets)? == 1 {
            header.goto(assets, "smalltextcenter")?;
            header.named(&["labelText"])?.text = Some(title.to_vec());
        }
    }
    if child(info, "mDescription")?.clip.visible {
        y += 5.;
        set_y(child(info, "mDescription")?, y);
        let lines = max_scroll(info.named(&["mDescription", "labelText"])?, assets)?;
        #[expect(clippy::cast_precision_loss, reason = "a line count")]
        let lines = lines as f32;
        y += (16. * lines).floor() + 15.;
    }
    if child(info, "mObjectives")?.clip.visible {
        set_y(child(info, "mObjectives")?, y);
        let list = info.named(&["mObjectives"])?;
        let mut row_y = height(child(list, "mObjectiveTitle")?, assets)?;
        for i in 0..objectives {
            let row = child(list, &format!("mObjective{i}"))?;
            if row.clip.visible {
                set_y(row, row_y);
                #[expect(clippy::cast_precision_loss, reason = "a line count")]
                let lines = max_scroll(row.clip.named(&["labelText"])?, assets)? as f32;
                row_y += lines * 26.;
            }
        }
        y += row_y;
    }
    // UpdatePositions0x4500: betting follows objectives/killed-it and precedes the fee.
    if child(info, "mBetting")?.clip.visible {
        let betting = child(info, "mBetting")?;
        set_y(betting, y);
        y += height(betting, assets)? + 10.;
    }
    if child(info, "mEntryFee")?.clip.visible {
        y += 10.;
        let fee = child(info, "mEntryFee")?;
        set_y(fee, y);
        y += height(fee, assets)? + 10.;
    }
    set_y(child(info, "mView")?, VIEW_Y);
    let option = child(info, "mNISOption")?;
    if option.clip.visible {
        set_y(option, NIS_OPTION_Y);
    }
    Ok(y)
}

/// A piece's `_height`: its bounds through its own (unrotated) matrix.
fn height(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    let r = bounds(&piece.clip, assets)?.ok_or("sign-up poster _height target has no bounds")?;
    Ok(((r.bottom - r.top) * piece.entry.matrix.d).abs())
}

/// A piece's `_width`, likewise.
fn width(piece: &Child, assets: &Assets) -> RenderResult<f32> {
    let r = bounds(&piece.clip, assets)?.ok_or("sign-up poster _width target has no bounds")?;
    Ok(((r.right - r.left) * piece.entry.matrix.a).abs())
}

/// `SetProceduralProperty(_width)` from ActionScript (`0x829ea608`): the x
/// scale becomes `a * value / width` as an `_xscale` (at least 1.1322573
/// percent, times 0.01).
fn set_width(piece: &mut Child, assets: &Assets, value: f32) -> RenderResult<()> {
    piece.clip.script_transform = true;
    if value < 0. {
        return Ok(());
    }
    let value = if value == 0. { MIN_SCALE } else { value };
    let m = piece.entry.matrix;
    if m.b.abs() >= MIN_SCALE || m.c.abs() >= MIN_SCALE {
        return Err("sign-up poster _width target is rotated".into());
    }
    let width = width(piece, assets)?;
    if width == 0. {
        return Ok(());
    }
    let ratio = (value / width).max(MIN_SCALE);
    let scale = (m.a * ratio * 100.).max(f32::from_bits(0x3f90_edcf));
    piece.entry.matrix.a = scale * f32::from_bits(0x3c23_d70a);
    Ok(())
}

/// challenge_signup.apt's InitAction defines the guarded global class
/// translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 66, 0x5ff4))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class,
/// frame 1 shows the screen and plays the intro sound, frame 56 stops on
/// `IntroComplete`, frame 57 plays the outro sound, frame 69 stops on
/// `OutroComplete`.
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0x5fc0 => clip.calls.push("challenge_signup"),
        0x6348 => {
            clip.visible = true;
            clip.calls.push("PlayIntroAudio");
        }
        0x6374 => {
            clip.playback.stop();
            clip.calls.push("IntroComplete");
        }
        0x6398 => clip.calls.push("PlaySignupOut"),
        0x63b4 => {
            clip.playback.stop();
            clip.calls.push("OutroComplete");
        }
        _ => return Err(format!("untranslated sign-up poster APT action {offset:#x}").into()),
    }
    Ok(())
}
