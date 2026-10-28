//! `Sk8::FE::FrontEndState_SignUpPoster` (FE state 32, screen 19
//! challenge/challenge_signup): what its screen's `challenge_info` sign-up
//! mode reads through the `SignUpPoster`, `Mission`, `HUDComponents` and
//! `FEUnlock` natives, and the challenge side of its accept and decline
//! (`ReceiveInput` `0x82657bf8`). The FE state's own flags (`mActionsBlocked`,
//! the input block) live with the session.

use super::{
    keys::class,
    objectives, signpost_progress, signup,
    system::{ChallengeSystem, Event, Output, World},
};

/// `challenges` fields the poster natives read.
pub(crate) mod challenge_field {
    /// `Title` (`GetSignUpTitle`).
    pub const TITLE: u64 = 0xe2a5_0788_b6c2_0c92;
    /// The sign-up description (`GetSignUpDescription`).
    pub const SIGN_UP_DESCRIPTION: u64 = 0x6b0b_d2ba_e508_b742;
    /// `TutorialNISHalTitle`.
    pub const TUTORIAL_NIS_HAL_TITLE: u64 = 0x59cd_0b94_49d1_9c3c;
    /// `Objectives`: RefSpecs to the objectives.
    pub const OBJECTIVES: u64 = 0xc4b3_4541_5e85_37be;
    /// `IsDynamicCompetitorChallenge`, which `IsCompetitorChallenge` reads.
    pub const IS_DYNAMIC_COMPETITOR: u64 = 0xf3a5_0805_8ae5_148d;
    /// `IsAThrowdownChallenge`.
    pub const IS_A_THROWDOWN: u64 = 0x4263_ed15_8a50_063e;
}

/// `challenge_local_data` flags the `Mission` natives read for the poster
/// (`GetBoolParam(0)` false: the poster's challenge).
pub(crate) mod local_field {
    /// `Mission.IsDescriptionEnabled` (`0x82625b40`).
    pub const DESCRIPTION_ENABLED: u64 = 0x137f_56d7_c414_56f1;
    /// `Mission.IsObjectivesEnabled` (`0x82625850`).
    pub const OBJECTIVES_ENABLED: u64 = 0x858d_73cd_6085_c8ec;
    /// `Mission.IsCompetitorsEnabled` (`0x826259c8`).
    pub const COMPETITORS_ENABLED: u64 = 0x0abd_c707_fc5d_df63;
}

/// `eChallengeTypes` 19, a betting challenge.
const BETTING: i32 = 19;

/// A string a poster native returns: a HAL key the localizer formats
/// (`FormatStringCustom` without arguments), or literal text (`#` prefix).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PosterText {
    pub text: String,
    pub formatted: bool,
}

/// What challenge_signup's `challenge_info` reads in `SetMode(Signup)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[expect(clippy::struct_excessive_bools, reason = "native queries")]
pub struct PosterView {
    /// `GetSignUpTitle` (`0x82626240`): `Title`, or `#Missing Title...`.
    pub title: PosterText,
    /// `GetSignUpDescription` (`0x826263e0`): the sign-up description, or
    /// `#Missing SignUp Description...`.
    pub description: PosterText,
    /// `EntryFee_GetFee` (`0x82627658`).
    pub entry_fee: i32,
    /// `EntryFee_HasPlayerPaidFee` (`0x82627390`).
    pub entry_fee_paid: bool,
    /// `FEUnlock.Cash_GetPlayerCashTotal` (`0x8260b008`).
    pub cash: i32,
    pub description_enabled: bool,
    pub objectives_enabled: bool,
    pub competitors_enabled: bool,
    /// `IsBettingChallenge` (`0x826481d8`).
    pub betting: bool,
    /// `HasEnoughForMinBet` (`0x826482f8`).
    pub enough_for_min_bet: bool,
    /// `HasTutorialNIS` (`0x826480d8`) with `GetTutorialNISHalName`
    /// (`0x82626a60`), a HAL key.
    pub tutorial_nis: Option<String>,
    /// `IsCompetitorChallenge` (`0x82627008`).
    pub competitor_challenge: bool,
    /// `SignUp_GetObjectiveTitle(i)` (`0x826b7f10`) for each `i` below
    /// `SignUp_GetNumObjectives` (`0x826b7ca8`): HAL key and arguments.
    pub objectives: Vec<(String, String)>,
    /// The betting panel's natives.
    pub bet: BetView,
}

/// What the poster's betting panel reads: `GetPlayerBet` (`0x82610138`) and `GetPotTotal`
/// (`0x82610230`), which it shows as currency, and `CanBetMore` (`0x8260fef0`) and `CanBetLess`
/// (`0x82610020`), which show its arrows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BetView {
    pub bet: i32,
    pub pot: i32,
    pub can_bet_more: bool,
    pub can_bet_less: bool,
}

/// What the poster's accept (keycode 2) did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosterAccept {
    /// `cMsgHUDYes` and `PlayerAcceptChallenge`; the state pops.
    Accepted,
    /// A betting challenge without the minimum bet: the error sound only.
    NotEnoughForMinBet,
    /// The fee is not covered: the error sound and the fee-failed popup
    /// (`eSk8FEPopup_ChallengeInfo_FeeFailed`, 203: `ID_COMMON_WARNING_TITLE`,
    /// `ID_CHALLENGEINFO_FEE_FAILED`, `ID_COMMON_OK`).
    FeeFailed,
    /// The fee needs a system the port lacks (reported as a script
    /// error); nothing happens.
    Unported,
}

fn challenge_attribute(world: &World, activity: usize, key: u64) -> Option<super::attrib_db::Attribute<'_>> {
    world.db.attribute(class::CHALLENGES, world.activities.activities[activity].key, key)
}

fn challenge_bool(world: &World, activity: usize, key: u64) -> bool {
    challenge_attribute(world, activity, key).and_then(|a| a.bool(0)).unwrap_or(false)
}

pub(crate) fn local_bool(world: &World, activity: usize, key: u64) -> bool {
    world.activities.activities[activity]
        .local_data
        .and_then(|(c, k)| world.db.attribute(c, k, key))
        .and_then(|a| a.bool(0))
        .unwrap_or(false)
}

/// A HAL key read through `FormatStringCustom(key, null)`, or the native's
/// literal when the attribute is empty.
fn formatted(world: &World, activity: usize, key: u64, missing: &str) -> PosterText {
    match challenge_attribute(world, activity, key).and_then(|a| a.text(0)) {
        Some(text) => PosterText { text: text.to_owned(), formatted: true },
        None => PosterText { text: missing.to_owned(), formatted: false },
    }
}

/// `FrontEndState_SignUpPoster::IsBettingChallenge` (`0x826481d8`): type 19
/// or a throwdown.
fn is_betting(world: &World, activity: usize) -> bool {
    signup::challenge_type(world, activity) == BETTING || challenge_bool(world, activity, challenge_field::IS_A_THROWDOWN)
}

/// `HasEnoughCashForMinBet` (`0x826482f8`): before the boot flow, always;
/// otherwise the cash covers the `MinimumBet` setting (default 0).
fn has_enough_for_min_bet(world: &World) -> bool {
    if !world.boot_flow_executed {
        return true;
    }
    let minimum = world.signpost.message_setting_value("MinimumBet").unwrap_or(0).cast_signed();
    signpost_progress::cash(world) >= minimum
}

/// A betting message setting (`MinimumBet`, `MaximumBet`, `BettingIncrement`; default 0).
fn bet_setting(world: &World, name: &str) -> i32 {
    world.signpost.message_setting_value(name).unwrap_or(0).cast_signed()
}

/// The betting panel's natives: `CanBetMore` while another `BettingIncrement` keeps the bet
/// within `MaximumBet` and the cash, `CanBetLess` while the bet is an increment above
/// `MinimumBet`.
pub(crate) fn bet_view(world: &World) -> BetView {
    let (bet, pot) = (world.signpost.betting.bet, world.signpost.betting.pot);
    let increment = bet_setting(world, "BettingIncrement");
    BetView {
        bet,
        pot,
        can_bet_more: bet <= bet_setting(world, "MaximumBet").wrapping_sub(increment)
            && increment.wrapping_add(bet) <= signpost_progress::cash(world),
        can_bet_less: bet >= bet_setting(world, "MinimumBet").wrapping_add(increment),
    }
}

/// `SignUpPoster.BetMore()` (`0x82626b98`): while the bet plus a `BettingIncrement` is below the
/// cash, the bet rises by it, capped at `MaximumBet` (`SetPlayersBet` of the poster's challenge),
/// with `core_nav_right`; otherwise `core_error`. Answers whether the bet, before the cap, is
/// below `MaximumBet`, and the sound.
pub(crate) fn bet_more(world: &mut World) -> (bool, &'static str) {
    let maximum = bet_setting(world, "MaximumBet");
    let increment = bet_setting(world, "BettingIncrement");
    let mut bet = world.signpost.betting.bet;
    let sound = if increment.wrapping_add(bet) >= signpost_progress::cash(world) {
        "FE_SOUND_CORE_ERROR"
    } else {
        bet = bet.wrapping_add(increment);
        signpost_progress::set_players_bet(world, world.signup.poster, bet.min(maximum));
        "FE_SOUND_CORE_NAV_RIGHT"
    };
    (bet < maximum, sound)
}

/// `SignUpPoster.BetLess()` (`0x82626db8`): `core_error` at `MinimumBet`, else `core_nav_left`;
/// the bet less a `BettingIncrement`, at least `MinimumBet` (`SetPlayersBet` of the poster's
/// challenge). Answers whether the lowered bet is above `MinimumBet`, and the sound.
pub(crate) fn bet_less(world: &mut World) -> (bool, &'static str) {
    let minimum = bet_setting(world, "MinimumBet");
    let increment = bet_setting(world, "BettingIncrement");
    let bet = world.signpost.betting.bet;
    let sound = if minimum == bet { "FE_SOUND_CORE_ERROR" } else { "FE_SOUND_CORE_NAV_LEFT" };
    let lowered = bet.wrapping_sub(increment);
    signpost_progress::set_players_bet(world, world.signup.poster, lowered.max(minimum));
    (lowered > minimum, sound)
}

/// `GetHasTutorialVideo` (`0x826480d8`): `kbAllowSignUpVideoToBePlayed`
/// (default true) and a `TutorialNISHalTitle` other than `ID_COMMON_NONE`.
fn tutorial_nis(world: &World, activity: usize) -> Option<String> {
    let title = challenge_attribute(world, activity, challenge_field::TUTORIAL_NIS_HAL_TITLE).and_then(|a| a.text(0))?;
    (title != "ID_COMMON_NONE").then(|| title.to_owned())
}

/// The objectives the sign-up lists: `SignUp_GetNumObjectives` gives 1 for
/// an `ObjectiveFlow` of type 1, else those with `DisplayInHUD` set or
/// absent; `SignUp_GetObjectiveTitle(i)` walks the same objectives.
pub(crate) fn objective_titles(world: &World, activity: usize) -> Vec<(String, String)> {
    let db = &world.db;
    let shown: Vec<(u64, u64)> = challenge_attribute(world, activity, challenge_field::OBJECTIVES)
        .map(|a| (0..a.len()).filter_map(|i| a.ref_spec(i)).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|&(c, k)| db.has_collection(c, k))
        .filter(|&(c, k)| db.attribute(c, k, objectives::field::DISPLAY_IN_HUD).is_none_or(|a| a.bool(0).unwrap_or(false)))
        .collect();
    let flow = challenge_attribute(world, activity, objectives::challenge_field::OBJECTIVE_FLOW)
        .and_then(|a| a.value_bytes(0))
        .filter(|b| b.len() >= 4)
        .map_or(0, |b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let count = if flow == 1 { 1 } else { shown.len() };
    (0..count)
        .map(|i| match shown.get(i) {
            Some(&(c, k)) => objectives::title(db, c, k),
            None => ("#OBJECTIVE NOT VALID!".to_owned(), String::new()),
        })
        .collect()
}

/// The poster's natives for a challenge. An extra fee the port cannot
/// compute is reported as a script error and counted as 0.
pub(crate) fn view(world: &mut World, activity: usize) -> PosterView {
    let key = world.activities.activities[activity].key;
    let sponsor = signpost_progress::has_board_sponsor(world);
    let paid = signpost_progress::challenge_save_data(world, key, "Paid Entry Fee");
    let extra = signpost_progress::extra_entry_fee(world, activity).unwrap_or_else(|error| {
        let context = world.activities.activities[activity].name.clone();
        world.output(Output::ScriptError { context, error });
        0
    });
    // EntryFee_GetFee: 0 with a board sponsor; the fee is 0 once paid.
    let entry_fee = if sponsor {
        0
    } else {
        extra + if paid == 1 { 0 } else { signpost_progress::challenge_economy_data(world, key, "Entry Fee") }
    };
    // EntryFee_HasPlayerPaidFee: a sponsor pays; an extra fee is never paid.
    let entry_fee_paid = sponsor || (extra <= 0 && paid != 0);
    PosterView {
        title: formatted(world, activity, challenge_field::TITLE, "#Missing Title..."),
        description: formatted(world, activity, challenge_field::SIGN_UP_DESCRIPTION, "#Missing SignUp Description..."),
        entry_fee,
        entry_fee_paid,
        cash: signpost_progress::cash(world),
        description_enabled: local_bool(world, activity, local_field::DESCRIPTION_ENABLED),
        objectives_enabled: local_bool(world, activity, local_field::OBJECTIVES_ENABLED),
        competitors_enabled: local_bool(world, activity, local_field::COMPETITORS_ENABLED),
        betting: is_betting(world, activity),
        enough_for_min_bet: has_enough_for_min_bet(world),
        tutorial_nis: tutorial_nis(world, activity),
        competitor_challenge: challenge_bool(world, activity, challenge_field::IS_DYNAMIC_COMPETITOR),
        objectives: objective_titles(world, activity),
        bet: bet_view(world),
    }
}

impl ChallengeSystem {
    /// challenge_info's `BetMore` (`0x3a7f`): `SignUpPoster.BetMore`; the sound it played.
    pub fn sign_up_bet_more(&mut self) -> &'static str {
        bet_more(&mut self.world).1
    }
    /// challenge_info's `BetLess` (`0x3ad9`): `SignUpPoster.BetLess`; the sound it played.
    pub fn sign_up_bet_less(&mut self) -> &'static str {
        bet_less(&mut self.world).1
    }
    /// The betting panel's natives after a bet change.
    #[must_use]
    pub fn sign_up_bet_view(&self) -> BetView {
        bet_view(&self.world)
    }
}

/// `ReceiveInput` (`0x82657bf8`) accept, keycode 2, once the intro is done.
/// The entry fee: a board sponsor marks it paid; otherwise the cash must
/// cover `Entry Fee` plus `GetExtraEntryFee`, and after the boot flow a fee
/// that is not paid and not covered opens the fee-failed popup instead, while
/// a covered one is charged (`ProcessEntryFee`) and marked paid. Then
/// `cMsgHUDYes` and `PlayerAcceptChallenge`. The FE sounds, the audio's
/// `NotifyChallengeLoad` and the announcer stop are not presented.
pub(crate) fn accept(world: &mut World) -> PosterAccept {
    let Some(activity) = world.signup.selected else { return PosterAccept::Accepted };
    if is_betting(world, activity) && !has_enough_for_min_bet(world) {
        return PosterAccept::NotEnoughForMinBet;
    }
    let key = world.activities.activities[activity].key;
    let mut paid = signpost_progress::challenge_save_data(world, key, "Paid Entry Fee") != 0;
    let extra = match signpost_progress::extra_entry_fee(world, activity) {
        Ok(extra) => extra,
        Err(error) => {
            let context = world.activities.activities[activity].name.clone();
            world.output(Output::ScriptError { context, error });
            return PosterAccept::Unported;
        }
    };
    let fee = signpost_progress::challenge_economy_data(world, key, "Entry Fee") + extra;
    let cash = signpost_progress::cash(world);
    let sponsor = signpost_progress::has_board_sponsor(world);
    if sponsor && !paid {
        signpost_progress::set_challenge_save_data(world, key, "Paid Entry Fee", 1);
        paid = true;
    }
    let can_pay = !paid && !sponsor && cash >= fee;
    if !paid && !can_pay && world.boot_flow_executed {
        return PosterAccept::FeeFailed;
    }
    if can_pay && world.boot_flow_executed {
        signpost_progress::process_entry_fee(world, key);
        signpost_progress::set_challenge_save_data(world, key, "Paid Entry Fee", 1);
    }
    world.send(Event::HudYes);
    signup::player_accept(world);
    PosterAccept::Accepted
}

/// `ReceiveInput` decline, keycode 3, once the intro is done: `cMsgHUDNo`,
/// `PlayerDeclineChallenge`, then for a throwdown or dynamic-competitor
/// challenge `ResetDynamicallyChosenAICompetitorInfo` (`0x825af2d8`), otherwise
/// `SetHUDState(1, -1)`, the in-game HUD state the port's HUD is always in.
pub(crate) fn decline(world: &mut World) {
    let selected = world.signup.selected;
    world.send(Event::HudNo);
    signup::player_decline(world);
    if let Some(activity) = selected
        && (challenge_bool(world, activity, challenge_field::IS_A_THROWDOWN)
            || challenge_bool(world, activity, challenge_field::IS_DYNAMIC_COMPETITOR))
    {
        super::competitors::reset(world);
    }
}
