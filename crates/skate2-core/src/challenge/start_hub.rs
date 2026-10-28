//! The challenge side of the start hub (`FrontEndState_StartHub`, FE state 7): what its hub
//! variants and the in-challenge screen read of the main challenge, the `cMsgAbandonChallenge` its
//! Quit Challenge and Quit Game send, the challenge system's receiver of that message, and Quit
//! Game's challenge end point.

use super::{
    keys::{class, hash32, hash64},
    signpost, signpost_progress, signup,
    signup_poster::{self, PosterText, challenge_field, local_field},
    system::{Event, Field, Message, World},
};
use crate::ui::pause_menu::MainChallenge;

/// `challenge_local_data` 0x23F394473183C613, which
/// `StartHubState_SinglePlayer_Challenge::IsItemEnabled` (`0x8264c560`) reads for Quit Challenge.
const QUIT_CHALLENGE_ENABLED: u64 = 0x23f3_9447_3183_c613;
/// `challenge_global_data` 0x89B3580572F41DA3, which
/// `cChallengeTeleportManager::GetChallengeEndPoint` (`0x82da2f08`) requires.
const HAS_END_POINT: u64 = 0x89b3_5805_72f4_1da3;
/// `eChallengeTypes_CreateASpot` and `eChallengeTypes_CreateASpot_Setup`.
const CREATE_A_SPOT: i32 = 35;
const CREATE_A_SPOT_SETUP: i32 = 36;

/// The main challenge (`cActivityManager::GetMainChallenge`) as the hub reads it: its
/// `ChallengeType` (`FrontEndState_StartHub::OnIntroDone` `0x826536e0`) and its Quit Challenge
/// flag.
#[must_use]
pub fn main_challenge(world: &World) -> Option<MainChallenge> {
    let main = world.activities.main?;
    Some(MainChallenge {
        challenge_type: signup::challenge_type(world, main),
        quit_enabled: signup_poster::local_bool(world, main, QUIT_CHALLENGE_ENABLED),
    })
}

/// `cMsgAbandonChallenge` (`0x85DA01E5`) on the challenge broadcaster: `IsQuitting` false from the
/// hub's Quit Challenge (`QuitChallengeCallback` `0x8265e868`), true from Quit Game
/// (`FrontEndManager::ExitToMainMenu` `0x82628990`).
pub fn abandon(world: &mut World, quitting: bool) {
    world.send(Event::Challenge(Message::new("cMsgAbandonChallenge", vec![("IsQuitting", Field::Bool(quitting))])));
}

/// `FrontEndManager::ExitToMainMenu` (`0x82628990`)'s challenge half: with a main challenge,
/// `cMsgAbandonChallenge` with `IsQuitting` true (`0x82628a40`), then (after the session's
/// `NISManager::KillNIS` while a NIS runs) `Sk8ChallengeSystem::QuittingToFE` (`0x825a2bf0`):
/// `ReevaluatePrereqActivities` does nothing until the next game load. Returns whether the message
/// was sent.
pub fn exit_to_main_menu(world: &mut World) -> bool {
    let main = world.activities.main.is_some();
    if main {
        abandon(world, true);
    }
    world.activities.is_quitting_to_fe = true;
    main
}

/// `Sk8ChallengeSystem::ReceiveAbandonChallengeMessage` (`0x825a2de0`, its constructor's handler,
/// ahead of the scripts'): a main challenge other than a Create-a-Spot one, abandoned without
/// quitting, is abandoned (`cSignPostManager::AbandonChallenge`) and the results processed.
pub(crate) fn receive(world: &mut World, event: &Event) {
    let Event::Challenge(message) = event else { return };
    if message.kind != hash32("cMsgAbandonChallenge") {
        return;
    }
    let Some(main) = world.activities.main else { return };
    let quitting = message
        .fields
        .as_ref()
        .and_then(|fields| fields.iter().find(|(name, _)| *name == "IsQuitting"))
        .is_some_and(|(_, value)| matches!(value, Field::Bool(true)));
    let challenge_type = signup::challenge_type(world, main);
    if !quitting && challenge_type != CREATE_A_SPOT_SETUP && challenge_type != CREATE_A_SPOT {
        let key = world.activities.activities[main].key;
        signpost::end_challenge_unwon(world, key, 2);
        signpost::process_challenge_results(world);
    }
}

/// `cChallengeTeleportManager::GetChallengeEndPoint` (`0x82da2f08`) of the main challenge: with its
/// global data's 0x89B3580572F41DA3 set, the `LocationName` of the first of `EndLocation`,
/// `MapStartLocation` and `Location` that the global data, then the challenge, has.
#[must_use]
pub fn challenge_end_point(world: &World) -> Option<String> {
    let main = world.activities.main?;
    let activity = &world.activities.activities[main];
    let global = activity.global_data?;
    if !world.db.attribute(global.0, global.1, HAS_END_POINT).and_then(|a| a.bool(0)).unwrap_or(false) {
        return None;
    }
    ["EndLocation", "MapStartLocation", "Location"].into_iter().find_map(|name| {
        let key = hash64(name);
        world
            .db
            .attribute(global.0, global.1, key)
            .or_else(|| world.db.attribute(class::CHALLENGES, activity.key, key))
            .map(|a| a.text(0).unwrap_or_default().to_owned())
    })
}

/// `cSignPostManager::QuittingOverride` (`0x825c4688`): the progression orientation becomes the
/// given rows and `UseOverrideBecauseQuiting` is set, so `SaveProgressionOrientation` keeps them
/// instead of asking the skater's breadcrumbs.
pub fn quitting_override(world: &mut World, rows: [[f32; 3]; 4]) {
    world.signpost.progress.use_override_because_quitting = true;
    world.signpost.progress.orientation.1 = rows;
}

/// What the start hub's screen reads of the career when `SetScreenState` sets it up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartHubView {
    /// `FEUnlock.Cash_GetPlayerCashTotal` (`0x8260b008`), the wallet's.
    pub cash: i32,
    /// `cSignPostManager::GetNewChallengeCount` (`0x825c7dd0`): the available challenges still
    /// marked new, which `StartHubState_SinglePlayer::GetOptionText` (`0x8264c788`) prints into
    /// Challenge Map's label.
    pub new_challenges: i32,
    /// `cSignPostManager::GetNewBLOGCount` (`0x825c7e38`): the BLOG entries still marked new, for
    /// Slappy's Blog's label.
    pub new_blogs: i32,
    /// The in-challenge screen's texts, while a main challenge runs.
    pub challenge: Option<ChallengeInfoView>,
}

/// The start hub's view of the career.
#[must_use]
pub fn view(world: &World) -> StartHubView {
    let count = |n: usize| i32::try_from(n).unwrap_or(i32::MAX);
    StartHubView {
        cash: signpost_progress::cash(world),
        new_challenges: count(world.signpost.available.values().filter(|&&new| new).count()),
        new_blogs: count(world.signpost.progress.blog.values().filter(|blog| blog.new).count()),
        challenge: world.activities.main.is_some().then(|| challenge_info(world)),
    }
}

/// What the in-challenge hub's `challenge_info` reads in `SetMode(1)` (`challenge_info.apt`
/// `0x4a3c`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChallengeInfoView {
    /// `Mission.GetDescription` (`0x8260e4e8`): the main challenge's 0x6B0BD2BAE508B742 through
    /// `FormatStringCustom`, or `#Missing Instance Desc...`.
    pub description: PosterText,
    /// `Mission.InChallengeIsDescriptionEnabled` (`0x8260fd40`): the main challenge's local
    /// 0x137F56D7C41456F1.
    pub description_enabled: bool,
    /// `Mission.IsObjectivesEnabled(false)` (`0x82625850`): the sign-up poster's challenge's local
    /// 0x858D73CD6085C8EC.
    pub objectives_enabled: bool,
    /// `SignUp_GetObjectiveTitle(i)` (`0x826b7f10`) for each `i` below `SignUp_GetNumObjectives`
    /// (`0x826b7ca8`), of the sign-up poster's challenge: HAL key and arguments.
    pub objectives: Vec<(String, String)>,
}

/// The in-challenge hub's `challenge_info` data. The objectives are those of the challenge the
/// sign-up poster last showed (`FrontEndState_SignUpPoster::mKey`), as the natives read them.
#[must_use]
pub fn challenge_info(world: &World) -> ChallengeInfoView {
    let main = world.activities.main;
    let description = main
        .and_then(|main| {
            let key = world.activities.activities[main].key;
            world.db.attribute(class::CHALLENGES, key, challenge_field::SIGN_UP_DESCRIPTION)?.text(0)
        })
        .map_or_else(
            || PosterText { text: "#Missing Instance Desc...".to_owned(), formatted: false },
            |text| PosterText { text: text.to_owned(), formatted: true },
        );
    let poster = world.activities.find(world.signup.poster);
    ChallengeInfoView {
        description,
        description_enabled: main.is_some_and(|main| signup_poster::local_bool(world, main, local_field::DESCRIPTION_ENABLED)),
        objectives_enabled: poster
            .is_some_and(|poster| signup_poster::local_bool(world, poster, local_field::OBJECTIVES_ENABLED)),
        objectives: poster.map(|poster| signup_poster::objective_titles(world, poster)).unwrap_or_default(),
    }
}
