//! `FrontEndState_Unlock`'s screen 20, unlock/unlockoverlay (asset 34,
//! fingerprint 07458651; template `UnlockOverlay`, placement 4,
//! `g3dSpinContainer`, drawn through `SetApt3DSpinMatrix`). The InitAction
//! (character 143, `0xa33c`) defines the `unlockoverlay` class translated
//! here and root action `0xa320` constructs it. What the `FEUnlock` natives
//! answer comes from `skate2_core::challenge::unlock_overlay`. FE sounds are
//! not played.
use super::{assets::Assets, clips::Clip, localizer::format_currency, pause_movie::configure_right_buttons};
use crate::gpu_renderer::RenderResult;
use skate2_core::{challenge::unlock_overlay::UnlockOverlayView, ui::apt_projection::Spin};

pub(super) const ASSET: usize = 34;
/// `unlockoverlay.mCashAnimLength`: the updates a cash line counts over.
const CASH_ANIM_LENGTH: f32 = 30.0;
/// `unlockoverlay.mInitialFramesToDelayAnimation`.
const INITIAL_DELAY: i32 = 30;
/// `unlockoverlay.greyOutColor` (10066329).
const GREY_OUT: u32 = 0xff00_0000 | 10_066_329;
/// `kANIMATIONPHASE_TOPDRAWER_4` and the sponsor phases.
const TOPDRAWER_4: i32 = 3;
const SPONSOR_LOGO: i32 = 4;
const SPONSOR_BOARD: i32 = 5;
const SPONSOR_TRUCK: i32 = 6;
const SPONSOR_WHEEL: i32 = 7;
const SPONSOR_CLOTHING: i32 = 8;
const SPONSOR_SHOE: i32 = 9;
/// The awards panel's slots.
const MAX_MAIN_REWARDS: i32 = 4;
/// `GetSponsorMeTapeText` (`0x82605658`).
const SPONSOR_ME_TAPE_TEXT: &str = "ID_UNLOCK_SPONSORTAPE";
/// The cash amount `InitCash` treats as none.
const NO_CASH: f32 = 0.01;

/// `InitMainRewards`' flags (`bNumber` .. `bProperty`), cleared as each
/// reward animates.
#[derive(Clone, Copy, Default)]
struct Rewards {
    number: bool,
    location: bool,
    trophy: bool,
    path: bool,
    coverage: bool,
    mag_min: bool,
    pro_challenge: bool,
    rob: bool,
    sponsor_tape: bool,
    throwdown: bool,
    dway: bool,
    character: bool,
    photo_path: bool,
    property: bool,
}

/// A cash line: `m*Init` counting towards `m*Goal`, and its finished flag
/// (`mPrize`, `mWager`, `mSponsorBonus`, `mTotalWallet`).
#[derive(Clone, Copy, Default)]
struct Count {
    init: f32,
    goal: f32,
    done: bool,
}

/// The `unlockoverlay` instance, with the class statics its constructor
/// resets.
struct Screen {
    view: UnlockOverlayView,
    rewards: Rewards,
    /// `mPhoneAnimState == kNUMBER_MAINANIMATION`.
    phone_main: bool,
    phone_index: usize,
    /// `mCharAnimState == kCHARACTER_MAINANIMATION`.
    character_main: bool,
    character_index: usize,
    /// `unlockoverlay.mAnimationInProgress`.
    in_progress: bool,
    phase: i32,
    finished_top_drawer: bool,
    finished_sponsors: bool,
    finished_cash: bool,
    cash_ticking_done: bool,
    initial_delay: i32,
    sponsors_to_cycle: i32,
    prize: Count,
    wager: Count,
    sponsor_bonus: Count,
    trick_callout: Count,
    wallet: Count,
    /// `mWalletGoalInitDeltaDiff`.
    wallet_delta: f32,
}

#[derive(Default)]
pub(super) struct UnlockOverlayMovie {
    pub root: Option<Clip>,
    /// The unlock state is open: `OnEnter` opened the screen and `PopState`
    /// has not closed it. The state hides the in-game HUD and pauses the game
    /// (`gFEStateInfo[10]`).
    pub open: bool,
    screen: Option<Screen>,
    /// `unlockoverlay.numMainRewards`, a class static only the class
    /// definition sets (to 0). The definition runs once (`if
    /// (!_global.unlockoverlay)`) and `_global` keeps the class after the
    /// screen unloads, so each overlay counts its rewards on top of the
    /// earlier ones.
    num_main_rewards: i32,
    intro_complete: bool,
    cash_anim_done: bool,
}

impl UnlockOverlayMovie {
    /// `OnEnter`'s `OpenScreen(20)` for a newly open overlay: a fresh
    /// screen, constructed, then its `intro`, for which the `ScreenManager`
    /// starts the spin (`Game.On3DIntroStart`); `OnLeave`'s `CloseScreen`
    /// once the state pops: the `outro` and the spin back
    /// (`On3DIntroExit`).
    pub fn update(&mut self, assets: &Assets, view: Option<&UnlockOverlayView>, spin: &mut Spin) -> RenderResult<()> {
        match view {
            Some(view) if !self.open => {
                self.open = true;
                self.intro_complete = false;
                self.cash_anim_done = false;
                let mut root = Clip::new(assets, ASSET, 0)?;
                let mut calls = Vec::new();
                root.take_calls(&mut calls);
                for call in calls {
                    match call {
                        "unlockoverlay" => {
                            self.screen = Some(construct(&mut root, assets, view, &mut self.num_main_rewards)?);
                        }
                        _ => return Err(format!("untranslated unlock overlay callback {call}").into()),
                    }
                }
                if self.screen.is_none() {
                    return Err("unlock overlay root constructed no screen".into());
                }
                root.goto(assets, "intro")?;
                spin.intro_start();
                self.root = Some(root);
                self.calls()?;
            }
            None if self.open => {
                self.open = false;
                self.screen = None;
                if let Some(root) = self.root.as_mut() {
                    root.goto(assets, "outro")?;
                }
                spin.intro_exit();
                self.calls()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// One front-end update (`FrontEndManager::Update`): the movie
    /// (`AptIntegration::Update`), then the open state's `OnUpdate`, whose
    /// `SetScreenState(50)` runs `UpdateAnimation`.
    pub fn tick(&mut self, assets: &Assets) -> RenderResult<()> {
        if let Some(root) = self.root.as_mut() {
            root.tick(assets)?;
        }
        self.calls()?;
        if self.open
            && let (Some(root), Some(screen)) = (self.root.as_mut(), self.screen.as_mut())
            && update_animation(root, assets, screen, self.num_main_rewards)?
        {
            self.cash_anim_done = true;
        }
        Ok(())
    }

    /// The screen's `ScreenManager.IntroComplete` since the last call.
    pub fn take_intro_complete(&mut self) -> bool {
        std::mem::take(&mut self.intro_complete)
    }

    /// `FEUnlock.Cash_SetCashAnimDone` since the last call.
    pub fn take_cash_anim_done(&mut self) -> bool {
        std::mem::take(&mut self.cash_anim_done)
    }

    /// The frame scripts' calls: `main_mc`'s `PlayIntroAudio`
    /// (`FE_SOUND_REWARD_SCREEN`), the reward clips'
    /// `_global.UnlockAnimationStepDone` (`AnimationStepDone`), and the root's
    /// frame 10 `IntroComplete` and frame 20 `OutroComplete`, which unloads
    /// it.
    fn calls(&mut self) -> RenderResult<()> {
        let Some(root) = self.root.as_mut() else { return Ok(()) };
        let mut calls = Vec::new();
        root.take_calls(&mut calls);
        for call in calls {
            match call {
                "PlayIntroAudio" => {}
                "UnlockAnimationStepDone" => {
                    if let Some(screen) = self.screen.as_mut() {
                        screen.in_progress = false;
                    }
                }
                "IntroComplete" => self.intro_complete = true,
                "OutroComplete" => {
                    self.root = None;
                    break;
                }
                _ => return Err(format!("untranslated unlock overlay callback {call}").into()),
            }
        }
        Ok(())
    }
}

/// The clip at `path`, or `None` where ActionScript reads `undefined` (its
/// calls and sets then do nothing). Apt finds names case-insensitively.
fn at<'a>(clip: &'a mut Clip, path: &[&str]) -> Option<&'a mut Clip> {
    let Some((first, rest)) = path.split_first() else { return Some(clip) };
    let child = clip.children.values_mut().find(|c| c.entry.name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(first)))?;
    at(&mut child.clip, rest)
}

/// `gotoAndPlay(label)`.
fn goto(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) -> RenderResult<()> {
    if let Some(clip) = at(root, path) {
        clip.goto(assets, label)?;
    }
    Ok(())
}

/// `_visible = visible`.
fn set_visible(root: &mut Clip, path: &[&str], visible: bool) {
    if let Some(clip) = at(root, path) {
        clip.visible = visible;
    }
}

/// A text field's `text = label`, resolved as `SkateAptString` does.
fn set_text(root: &mut Clip, assets: &Assets, path: &[&str], label: &str) {
    if let Some(clip) = at(root, path) {
        clip.text = Some(assets.initial_text(label));
    }
}

/// A text field's `text = FEUnlock.Cash_GetFormattedCashString(value)`
/// (`0x826055a8`): `FormatCurrency` with the `#` format prefix, so the field
/// shows the currency as written.
fn set_cash(root: &mut Clip, assets: &Assets, path: &[&str], value: f32) {
    if let Some(clip) = at(root, path) {
        clip.text = Some(format_currency(assets, value).encode_utf16().collect());
    }
}

/// A text field's `textColor`.
fn set_text_colour(root: &mut Clip, path: &[&str], colour: u32) {
    if let Some(clip) = at(root, path) {
        clip.properties.insert("text_colour".into(), colour.to_string());
    }
}

/// `new unlockoverlay(this)`: `main_mc.stop()`, the phone and character
/// lists, the reset state and statics, `RegisterScreen`, the proceed button
/// (`ID_PROCEED_FROM_UNLOCK_TO_BLOG` when coverage was unlocked), then
/// `InitLevels`. The root clip has no `UpdateAnimation`, so its
/// `onEnterFrame` is undefined: `SetState(50)` alone animates.
fn construct(root: &mut Clip, assets: &Assets, view: &UnlockOverlayView, num_main_rewards: &mut i32) -> RenderResult<Screen> {
    if let Some(main) = at(root, &["main_mc"]) {
        main.playback.stop();
    }
    let mut screen = Screen {
        view: view.clone(),
        rewards: Rewards::default(),
        phone_main: true,
        phone_index: 0,
        character_main: true,
        character_index: 0,
        in_progress: true,
        phase: 0,
        finished_top_drawer: false,
        finished_sponsors: false,
        finished_cash: false,
        cash_ticking_done: false,
        initial_delay: INITIAL_DELAY,
        sponsors_to_cycle: 0,
        prize: Count::default(),
        wager: Count::default(),
        sponsor_bonus: Count::default(),
        trick_callout: Count::default(),
        wallet: Count::default(),
        wallet_delta: 1.0,
    };
    let proceed = if view.coverage_unlocked { "ID_PROCEED_FROM_UNLOCK_TO_BLOG" } else { "ID_COMMON_PROCEED" };
    if let Some(menu) = at(root, &["main_mc", "mEndOptions", "mButtonMenu"]) {
        configure_right_buttons(menu, assets, &[("A", proceed)])?;
    }
    init_levels(root, assets, &mut screen, num_main_rewards)?;
    screen.in_progress = false;
    Ok(screen)
}

/// `InitLevels`: the rewards, sponsor items and cash, then `main_mc`'s
/// `"<rewards><items><cash>-intro"` layout (`111`, `101`, `011` or `001`).
fn init_levels(root: &mut Clip, assets: &Assets, screen: &mut Screen, num_main_rewards: &mut i32) -> RenderResult<()> {
    let main_rewards = init_main_rewards(root, assets, screen, num_main_rewards)?;
    if !main_rewards {
        screen.phase = SPONSOR_LOGO;
        screen.finished_top_drawer = true;
    }
    screen.sponsors_to_cycle = screen.view.sponsors_to_cycle;
    let unlock_items = screen.sponsors_to_cycle > 0;
    let cash = init_cash(root, assets, screen);
    set_visible(root, &["main_mc", "mUnlocks", "labeltext"], unlock_items);
    let bit = |b: bool| if b { '1' } else { '0' };
    let label = format!("{}{}{}-intro", bit(main_rewards), bit(unlock_items), bit(cash));
    goto(root, assets, &["main_mc"], &label)
}

/// `InitMainRewards`: the reward flags (the trophy and photo path are
/// overwritten false after their natives answer, the path is false and
/// `Medal_IsProChallengeUnlocked` is the constant-false native), counted
/// into `numMainRewards` (at most 4), the awards panel's layout for that
/// count and its stars hidden.
fn init_main_rewards(root: &mut Clip, assets: &Assets, screen: &mut Screen, num_main_rewards: &mut i32) -> RenderResult<bool> {
    let view = &screen.view;
    screen.rewards = Rewards {
        number: view.numbers_unlocked,
        location: view.location_unlocked,
        trophy: false,
        path: false,
        coverage: view.coverage_unlocked,
        mag_min: view.mag_minute,
        pro_challenge: false,
        rob: view.rob,
        sponsor_tape: view.sponsor_tape,
        throwdown: view.throwdown,
        dway: view.dway,
        character: view.character_unlocked,
        photo_path: false,
        property: view.property,
    };
    let r = screen.rewards;
    for flag in [
        r.number, r.location, r.trophy, r.path, r.coverage, r.mag_min, r.pro_challenge, r.rob, r.sponsor_tape, r.throwdown,
        r.dway, r.character, r.photo_path, r.property,
    ] {
        if flag {
            *num_main_rewards += 1;
        }
    }
    if *num_main_rewards > MAX_MAIN_REWARDS {
        *num_main_rewards = MAX_MAIN_REWARDS;
    }
    goto(root, assets, &["main_mc", "mAwards"], &num_main_rewards.to_string())?;
    for slot in 0..*num_main_rewards {
        set_visible(root, &["main_mc", "mAwards", &format!("mcStar{slot}")], false);
    }
    Ok(*num_main_rewards > 0)
}

/// `InitCash`: the prize line (challenge prize, contest purse and bonus
/// objective cash), the wager, sponsor bonus and trick callout goals, each
/// greyed and its value hidden at 0.01 or less (this movie has no
/// `mCashTrick`/`mCalloutVal`, so the callout's reads are `undefined`), and
/// the wallet counting from the cash before these awards (at least 0).
/// `mcSponsorCash.labeltext.autoSize = "left"` moves nothing on the
/// left-aligned field.
fn init_cash(root: &mut Clip, assets: &Assets, screen: &mut Screen) -> bool {
    let view = &screen.view;
    #[expect(clippy::cast_precision_loss, reason = "Apt numbers")]
    let number = |value: i32| value as f32;
    screen.prize.goal = number(view.prize) + number(view.purse) + number(view.objective_bonus);
    let cash = ["main_mc", "mCash"];
    if screen.prize.goal <= NO_CASH {
        set_text_colour(root, &[cash[0], cash[1], "mCashPrize", "mCashPrizeText"], GREY_OUT);
        set_visible(root, &[cash[0], cash[1], "mCashPrizeVal"], false);
    }
    if number(view.wager) <= NO_CASH {
        set_text_colour(root, &[cash[0], cash[1], "mCashWager", "mCashWagerText"], GREY_OUT);
        set_visible(root, &[cash[0], cash[1], "mWagerVal"], false);
    }
    if number(view.sponsor_bonus) <= NO_CASH {
        set_text_colour(root, &[cash[0], cash[1], "mcSponsorCash", "labeltext"], GREY_OUT);
        set_visible(root, &[cash[0], cash[1], "mSponsorVal"], false);
    }
    set_cash(root, assets, &[cash[0], cash[1], "mCashPrizeVal"], screen.prize.init);
    screen.wager.goal = number(view.wager);
    set_cash(root, assets, &[cash[0], cash[1], "mWagerVal"], screen.wager.init);
    screen.sponsor_bonus.goal = number(view.sponsor_bonus);
    set_cash(root, assets, &[cash[0], cash[1], "mSponsorVal"], screen.sponsor_bonus.init);
    screen.trick_callout.goal = number(view.trick_callout);
    screen.wallet.goal = number(view.cash);
    screen.wallet.init = screen.wallet.goal - screen.trick_callout.goal - screen.sponsor_bonus.goal - screen.wager.goal - screen.prize.goal;
    if screen.wallet.init < 0.0 {
        screen.wallet.init = 0.0;
    }
    screen.wallet_delta = screen.wallet.goal - screen.wallet.init;
    set_cash(root, assets, &[cash[0], cash[1], "mWalletVal"], screen.wallet.init);
    true
}

/// `SetState(50)` → `UpdateAnimation`: the initial delay, then the next step
/// whenever no reward animation runs. True when the step reported
/// `Cash_SetCashAnimDone`.
fn update_animation(root: &mut Clip, assets: &Assets, screen: &mut Screen, num_main_rewards: i32) -> RenderResult<bool> {
    if screen.initial_delay > 0 {
        screen.initial_delay -= 1;
        return Ok(false);
    }
    if screen.in_progress {
        return Ok(false);
    }
    if !screen.finished_top_drawer {
        process_top_drawer_next(root, assets, screen, num_main_rewards)?;
        Ok(false)
    } else if !screen.finished_sponsors {
        process_sponsor_next(root, assets, screen)?;
        Ok(false)
    } else if !screen.finished_cash {
        Ok(process_cash_next(root, assets, screen))
    } else {
        Ok(false)
    }
}

/// A reward branch of `ProcessTopDrawerNext`: the slot's icon frame, its
/// `icon_intro`, the label, then its star: hidden, or for a counted reward
/// shown with `#x<count>` when more than one was unlocked.
fn reward(root: &mut Clip, assets: &Assets, screen: &mut Screen, slot: i32, icon: &str, label: &str, count: Option<i32>) -> RenderResult<()> {
    let comp = format!("mMainComp{slot}");
    goto(root, assets, &["main_mc", "mAwards", &comp, "compIcon"], icon)?;
    goto(root, assets, &["main_mc", "mAwards", &comp], "icon_intro")?;
    screen.in_progress = true;
    set_text(root, assets, &["main_mc", "mAwards", &comp, "label_mc", "labeltext"], label);
    let star = format!("mcStar{slot}");
    match count {
        Some(count) if count > 1 => {
            set_visible(root, &["main_mc", "mAwards", &star], true);
            set_text(root, assets, &["main_mc", "mAwards", &star, "labeltext"], &format!("#x{count}"));
        }
        _ => set_visible(root, &["main_mc", "mAwards", &star], false),
    }
    Ok(())
}

/// `ProcessNumberAnimation` / `ProcessCharacterAnimation`: the first entry
/// with the icon frame (`number`/`character`), each further entry by
/// `icon_update`, then the total with `icon_intro` and the star's count; true
/// once all were shown. A single entry is done after its first step.
fn list_step(root: &mut Clip, assets: &Assets, screen: &mut Screen, slot: i32, characters: bool) -> RenderResult<bool> {
    let comp = format!("mMainComp{slot}");
    let star = format!("mcStar{slot}");
    let (list, main, index, icon, total) = if characters {
        (&screen.view.characters, &mut screen.character_main, &mut screen.character_index, "character", "ID_UNLOCK_CHARACTERS")
    } else {
        (&screen.view.phone_numbers, &mut screen.phone_main, &mut screen.phone_index, "number", "ID_UNLOCK_PHONE_NUMBER")
    };
    let entry = |i: usize| list.get(i).cloned().unwrap_or_default();
    let done = if *main {
        goto(root, assets, &["main_mc", "mAwards", &comp, "compIcon"], icon)?;
        goto(root, assets, &["main_mc", "mAwards", &comp], "icon_update")?;
        set_text(root, assets, &["main_mc", "mAwards", &comp, "label_mc", "labeltext"], &entry(*index));
        set_visible(root, &["main_mc", "mAwards", &star], false);
        *index += 1;
        if list.len() > 1 {
            *main = false;
            false
        } else {
            true
        }
    } else if *index < list.len() {
        goto(root, assets, &["main_mc", "mAwards", &comp], "icon_update")?;
        set_text(root, assets, &["main_mc", "mAwards", &comp, "label_mc", "labeltext"], &entry(*index));
        *index += 1;
        false
    } else if *index == list.len() {
        goto(root, assets, &["main_mc", "mAwards", &comp], "icon_intro")?;
        set_text(root, assets, &["main_mc", "mAwards", &comp, "label_mc", "labeltext"], total);
        set_visible(root, &["main_mc", "mAwards", &star], true);
        set_text(root, assets, &["main_mc", "mAwards", &star, "labeltext"], &format!("#x{}", list.len()));
        *index += 1;
        false
    } else {
        return Ok(true);
    };
    screen.in_progress = true;
    Ok(done)
}

/// `ProcessTopDrawerNext`: past the fourth slot the sponsor section starts;
/// otherwise the slot (one further once the rewards are all counted) shows
/// the first remaining reward in the authored order. The property branch
/// clears `bPath`, not `bProperty`, so a property reward repeats in the
/// later slots. The phone numbers and characters take several steps.
fn process_top_drawer_next(root: &mut Clip, assets: &Assets, screen: &mut Screen, num_main_rewards: i32) -> RenderResult<()> {
    if screen.phase > TOPDRAWER_4 {
        screen.phase = SPONSOR_LOGO;
        screen.finished_top_drawer = true;
        return Ok(());
    }
    if !(screen.phase < num_main_rewards) {
        screen.phase += 1;
    }
    let slot = screen.phase;
    let mut advance = true;
    let r = screen.rewards;
    if r.trophy {
        // Never set here: InitMainRewards overwrote the flag. The branch
        // would also load the award's icon into compIcon.mTrophy0.
        screen.rewards.trophy = false;
        reward(root, assets, screen, slot, "trophy", "ID_UNLOCK_TROPHY", None)?;
    } else if r.mag_min {
        screen.rewards.mag_min = false;
        reward(root, assets, screen, slot, "magmin", "ID_UNLOCK_MAGMIN", None)?;
    } else if r.pro_challenge {
        screen.rewards.pro_challenge = false;
        reward(root, assets, screen, slot, "prochallenge", "ID_UNLOCK_PROCHALLENGE", None)?;
    } else if r.rob {
        screen.rewards.rob = false;
        reward(root, assets, screen, slot, "rob", "ID_UNLOCK_ROB", None)?;
    } else if r.throwdown {
        screen.rewards.throwdown = false;
        let label = if r.sponsor_tape { "ID_UNLOCK_SHINGO " } else { "ID_UNLOCK_THROWDOWN" };
        reward(root, assets, screen, slot, "throwdown", label, None)?;
    } else if r.sponsor_tape {
        screen.rewards.sponsor_tape = false;
        reward(root, assets, screen, slot, "sponsortape", SPONSOR_ME_TAPE_TEXT, None)?;
    } else if r.dway {
        screen.rewards.dway = false;
        reward(root, assets, screen, slot, "dway", "ID_UNLOCK_DWAY", None)?;
    } else if r.path {
        screen.rewards.path = false;
        let paths = screen.view.paths;
        reward(root, assets, screen, slot, "path", "ID_UNLOCK_PATH", Some(paths))?;
    } else if r.location {
        screen.rewards.location = false;
        let (label, locations) = (screen.view.location_icon_text.clone(), screen.view.locations);
        reward(root, assets, screen, slot, "location", &label, Some(locations))?;
    } else if r.property {
        screen.rewards.path = false;
        reward(root, assets, screen, slot, "property", "ID_UNLOCK_PROPERTY", None)?;
    } else if r.coverage {
        screen.rewards.coverage = false;
        let icon = if screen.view.thrasher_coverage { "thrasher" } else { "sbm" };
        reward(root, assets, screen, slot, icon, "ID_UNLOCK_COVERAGE", None)?;
    } else if r.number {
        if list_step(root, assets, screen, slot, false)? {
            screen.rewards.number = false;
        } else {
            advance = false;
        }
    } else if r.character {
        if list_step(root, assets, screen, slot, true)? {
            screen.rewards.character = false;
        } else {
            advance = false;
        }
    } else if r.photo_path {
        screen.rewards.photo_path = false;
        reward(root, assets, screen, slot, "photostage", "ID_UNLOCK_PHOTO_STAGE", None)?;
    }
    if advance {
        screen.phase += 1;
    }
    Ok(())
}

/// `ProcessSponsorNext`: the sponsor logo's `iconintro` (and the unlocked
/// item's HAL text when the label shows), then each of the board, truck,
/// wheel, clothing and shoe clips `on` when that part type was unlocked
/// (animating) or `off`, then the section finishes.
fn process_sponsor_next(root: &mut Clip, assets: &Assets, screen: &mut Screen) -> RenderResult<()> {
    let unlocks = ["main_mc", "mUnlocks"];
    match screen.phase {
        SPONSOR_LOGO => {
            goto(root, assets, &[unlocks[0], unlocks[1], "icon_mc"], "iconintro")?;
            screen.in_progress = true;
            screen.phase += 1;
            // SponsorX_IsNewInventory (0x826056f0) is always true.
            let label_visible = at(root, &[unlocks[0], unlocks[1], "labeltext"]).is_some_and(|c| c.visible);
            if label_visible {
                let text = screen.view.sponsor_unlock_text.clone();
                let unknown = format!("#{}", assets.localize("ID_COMMON_UNKNOWN")?);
                set_text(root, assets, &[unlocks[0], unlocks[1], "labeltext"], if text == unknown { "#" } else { &text });
            }
        }
        SPONSOR_BOARD | SPONSOR_TRUCK | SPONSOR_WHEEL | SPONSOR_CLOTHING | SPONSOR_SHOE => {
            let (clip, unlocked) = match screen.phase {
                SPONSOR_BOARD => ("boards_mc", screen.view.board),
                SPONSOR_TRUCK => ("trucks_mc", screen.view.truck),
                SPONSOR_WHEEL => ("wheels_mc", screen.view.wheel),
                SPONSOR_CLOTHING => ("clothes_mc", screen.view.clothing),
                _ => ("shoes_mc", screen.view.shoe),
            };
            if unlocked {
                goto(root, assets, &[unlocks[0], unlocks[1], clip], "on")?;
                screen.in_progress = true;
            } else {
                goto(root, assets, &[unlocks[0], unlocks[1], clip], "off")?;
            }
            screen.phase += 1;
        }
        _ => {
            screen.finished_sponsors = true;
            screen.phase += 1;
        }
    }
    Ok(())
}

/// `ProcessCashNext`: `CashTick` until its lines are done, then
/// `CashTickFinish` and the section finishes. True when
/// `Cash_SetCashAnimDone` was called.
fn process_cash_next(root: &mut Clip, assets: &Assets, screen: &mut Screen) -> bool {
    if screen.cash_ticking_done {
        cash_tick_finish(root, assets, screen);
        screen.finished_cash = true;
        true
    } else {
        cash_tick(root, assets, screen)
    }
}

/// One line's `CashTick` step: `goal / mCashAnimLength` more, snapped to the
/// goal and finished once it is not below it.
fn count_up(count: &mut Count, step: f32) {
    count.init += step;
    if !(count.init < count.goal) {
        count.done = true;
        count.init = count.goal;
    }
}

/// `CashTick`: the prize, wager, sponsor bonus and wallet lines count up one
/// after another; once all are done, `Cash_SetCashAnimDone` (true).
fn cash_tick(root: &mut Clip, assets: &Assets, screen: &mut Screen) -> bool {
    let cash = ["main_mc", "mCash"];
    if !screen.prize.done {
        let step = screen.prize.goal / CASH_ANIM_LENGTH;
        count_up(&mut screen.prize, step);
        set_cash(root, assets, &[cash[0], cash[1], "mCashPrizeVal"], screen.prize.init);
    } else if !screen.wager.done {
        let step = screen.wager.goal / CASH_ANIM_LENGTH;
        count_up(&mut screen.wager, step);
        set_cash(root, assets, &[cash[0], cash[1], "mWagerVal"], screen.wager.init);
    } else if !screen.sponsor_bonus.done {
        let step = screen.sponsor_bonus.goal / CASH_ANIM_LENGTH;
        count_up(&mut screen.sponsor_bonus, step);
        set_cash(root, assets, &[cash[0], cash[1], "mSponsorVal"], screen.sponsor_bonus.init);
    } else if !screen.wallet.done {
        count_up(&mut screen.wallet, screen.wallet_delta / CASH_ANIM_LENGTH);
        set_cash(root, assets, &[cash[0], cash[1], "mWalletVal"], screen.wallet.init);
    } else {
        screen.cash_ticking_done = true;
        return true;
    }
    false
}

/// `CashTickFinish`: every line at its goal, then `Cash_SetCashAnimDone`
/// again.
fn cash_tick_finish(root: &mut Clip, assets: &Assets, screen: &mut Screen) {
    let cash = ["main_mc", "mCash"];
    screen.prize.init = screen.prize.goal;
    screen.wager.init = screen.wager.goal;
    screen.sponsor_bonus.init = screen.sponsor_bonus.goal;
    screen.wallet.init = screen.wallet.goal;
    set_cash(root, assets, &[cash[0], cash[1], "mCashPrizeVal"], screen.prize.init);
    set_cash(root, assets, &[cash[0], cash[1], "mWagerVal"], screen.wager.goal);
    set_cash(root, assets, &[cash[0], cash[1], "mSponsorVal"], screen.sponsor_bonus.goal);
    set_cash(root, assets, &[cash[0], cash[1], "mWalletVal"], screen.wallet.goal);
    screen.cash_ticking_done = true;
}

/// unlockoverlay.apt's InitAction defines the guarded global class
/// translated above.
pub(super) fn init_action(asset: usize, character: u32, offset: usize) -> bool {
    matches!((asset, character, offset), (ASSET, 143, 0xa33c))
}

/// Timeline actions besides `Stop`: root frame 0 constructs the class,
/// frame 1's script is empty, frames 10 and 20 report `IntroComplete` and
/// `OutroComplete` and stop; `main_mc` plays the intro audio at each layout's
/// first frame; the reward clips report `UnlockAnimationStepDone` when their
/// animation ends (`mMainComp` frame 26 keeps playing).
pub(super) fn action(clip: &mut Clip, offset: usize) -> RenderResult<()> {
    match offset {
        0xa320 => clip.calls.push("unlockoverlay"),
        0xc188 => {}
        0xc194 => {
            clip.calls.push("IntroComplete");
            clip.playback.stop();
        }
        0xc1c0 => {
            clip.calls.push("OutroComplete");
            clip.playback.stop();
        }
        0xc5d8 | 0xc60c | 0xc640 | 0xc674 => clip.calls.push("PlayIntroAudio"),
        0xc318 => clip.calls.push("UnlockAnimationStepDone"),
        0xc334 | 0xc3c8 | 0xc3f0 | 0xc418 | 0xc440 | 0xc468 | 0xc4a4 => {
            clip.calls.push("UnlockAnimationStepDone");
            clip.playback.stop();
        }
        _ => return Err(format!("untranslated unlock overlay APT action {offset:#x}").into()),
    }
    Ok(())
}
