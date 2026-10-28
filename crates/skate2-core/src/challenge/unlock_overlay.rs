//! What `FrontEndState_Unlock`'s screen (`unlock/unlockoverlay`) reads
//! through the `FEUnlock` natives (`UnlockCmdTranslator::Initialize`
//! `0x8262c108`): the completion results, the challenge's cash awards and the
//! player's cash. The results only change in progression handlers, which do
//! not run while the overlay pauses the game, so one reading serves the
//! whole screen.

use super::{
    keys::class,
    signpost, signpost_progress,
    system::World,
};

/// `eCashCredit`.
mod credit {
    pub const BETTING: usize = 1;
    pub const CHALLENGE_AWARD: usize = 2;
    pub const CONTEST_PURSE: usize = 3;
    pub const SPONSORSHIP: usize = 4;
    pub const TRICK_CALLOUT: usize = 5;
    pub const BONUS_OBJECTIVE: usize = 6;
}

/// The CAS part types `SponsorX_Is*Unlocked` pass to `CheckUnlockResults`.
mod part {
    pub const BOARD: u16 = 1;
    pub const TRUCK: u16 = 2;
    pub const WHEEL: u16 = 4;
    pub const SHOE: u16 = 8;
    pub const CLOTHING: u16 = 16;
}

/// `throwdown_competitors`, `characters_marquee` and the asset groups the
/// lists and the sponsor text name.
mod classes {
    pub const THROWDOWN_COMPETITORS: u64 = 0x9c86_9217_c5ae_e9b4;
    pub const CHARACTERS_MARQUEE: u64 = 0x91bc_a669_3efc_9aa7;
    pub const ASSET_GROUP_UNLOCKS: u64 = 0xa1c0_cb4e_155b_407d;
    pub const ASSET_GROUP_SPONSORS: u64 = 0x1ff0_26b1_24d5_e938;
}

/// `throwdown_competitors.HALIDString` (layout +0x48).
const COMPETITOR_HAL_ID: u32 = 0x48 / 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnlockOverlayView {
    /// `Phone_GetNumbersUnlockedList` (`0x8260c128`): each unlocked phone
    /// number's competitor `HALIDString`, or `ID_UNLOCK_PHONE_NUMBER` alone.
    pub phone_numbers: Vec<String>,
    /// `Character_GetUnlockedList` (`0x8260cdb0`): each unlocked
    /// `characters_marquee` collection's layout word 1, or `#` alone.
    pub characters: Vec<String>,
    /// `Phone_IsNumbersUnlocked` (`0x8260bfa8`): `NumPhoneNumbersUnlocked > 0`.
    pub numbers_unlocked: bool,
    /// `Location_IsLocationUnlocked` (`0x8260c300`).
    pub location_unlocked: bool,
    /// `Coverage_IsCoverageUnlocked` (`0x8260c6d0`).
    pub coverage_unlocked: bool,
    /// `Medal_IsMagMinuteUnlocked` (`0x8260c788`, `IsMagMinuteUnlocked`
    /// `0x825d3198`): `CheckChallengeResults("Mag Minute")`.
    pub mag_minute: bool,
    /// `Medal_IsRobUnlocked` (`0x8260c838`, `IsRobChallengeUnlocked`
    /// `0x825d3138`): `CheckChallengeResults("RobDyrdek Path")`.
    pub rob: bool,
    /// `Medal_IsSponsorTapeUnlocked` (`0x8260c8e8`, `IsSponsorTapeCreated`
    /// `0x825c4678`): `sponsorMeCreated`.
    pub sponsor_tape: bool,
    /// `Medal_IsThrowdownUnlocked` (`0x8260c998`, `IsThrowdownLocationUnlocked`
    /// `0x825d3168`): `CheckChallengeResults("Throwdown Locations")`.
    pub throwdown: bool,
    /// `Medal_IsDWayUnlocked` (`0x8260ca48`, `IsDWayChallengeUnlocked`
    /// `0x825d3150`): `CheckChallengeResults("DannyWay Path")`.
    pub dway: bool,
    /// `Character_Unlocked` (`0x8260cce0`).
    pub character_unlocked: bool,
    /// `Property_Unlocked` (`0x8260d038`, `IsPropertyUnlocked` `0x825d3180`):
    /// `CheckChallengeResults("FE My Properties")`.
    pub property: bool,
    /// `Sponsors_GetNumSponsorsToCycle` (`0x8260d0e8`): 1 with unlocked CAC or
    /// sponsor CAC items, else 0.
    pub sponsors_to_cycle: i32,
    /// `Path_NumPathUnlocked` (`0x8260c618`).
    pub paths: i32,
    /// `GetLocationIconText` (`0x8260caf8`): the first unlocked location's
    /// `fe_locations` layout word 2, or `ID_UNLOCK_LOCATION`.
    pub location_icon_text: String,
    /// `Location_NumLocationsUnlocked` (`0x8260c3d0`).
    pub locations: i32,
    /// `IsCoverageThrasherCoverage` (`0x8260cbf8`): the photo article's layout
    /// word 6 is 0.
    pub thrasher_coverage: bool,
    /// `SponsorX_GetUnlockHalText` (`0x8260d618`): the first unlocked CAC
    /// item's (`progression_asset_group_unlocks`), else sponsor CAC item's
    /// (`progression_asset_group_sponsors`) layout word 1, else
    /// `ID_COMMON_UNKNOWN`.
    pub sponsor_unlock_text: String,
    /// `SponsorX_IsBoardUnlocked`/`Truck`/`Wheel`/`Clothing`/`Shoe`
    /// (`0x8260d1c8`..`0x8260d4a8`): `CheckUnlockResults` (`0x825c5ab0`) of
    /// the part type over the unlocked CAC and sponsor CAC items.
    pub board: bool,
    pub truck: bool,
    pub wheel: bool,
    pub clothing: bool,
    pub shoe: bool,
    /// `Cash_GetChallengeCashPrize` (`0x8260baa0`): `GetCashAwards(ChallengeAward)`.
    pub prize: i32,
    /// `Cash_GetChallengeCashPurse` (`0x8260bb58`): `ContestPurse`.
    pub purse: i32,
    /// `Cash_GetChallengeObjBonusCash` (`0x8260be38`): `BonusObjective`.
    pub objective_bonus: i32,
    /// `Cash_GetWagerPot` (`0x8260bc10`): `Betting`.
    pub wager: i32,
    /// `Cash_GetSponsorBonus` (`0x8260bcc8`): `Sponsorship`.
    pub sponsor_bonus: i32,
    /// `Cash_GetTrickCalloutBonus` (`0x8260bd80`): `TrickCallout`.
    pub trick_callout: i32,
    /// `Cash_GetPlayerCashTotal` (`0x8260b008`).
    pub cash: i32,
}

/// `cSignPostManager::CheckUnlockResults(typeCheck)` (`0x825c5ab0`): an
/// unlocked CAC or sponsor CAC item whose `CASUnlockPartDB::GetUnlockFlags`
/// has the type.
fn check_unlock_results(world: &World, types: u16) -> bool {
    let completion = &world.signpost.completion;
    completion
        .unlocked_cac_items
        .iter()
        .chain(&completion.unlocked_sponsor_cac_items)
        .any(|&key| world.cas_unlock_parts.unlock_flags(key) & types != 0)
}

/// `cSignPostManager::GetCashAwards(credit)` (`0x825c44f0`).
fn cash_awards(world: &World, credit: usize) -> i32 {
    world.signpost.progress.cash_earned.get(credit).copied().unwrap_or(0)
}

fn layout_string(world: &World, class: u64, collection: u64, index: u32) -> String {
    world.db.layout_text(class, collection, index).unwrap_or_default().to_owned()
}

/// The natives' answers when the overlay's screen reads them.
#[must_use]
pub fn view(world: &World) -> UnlockOverlayView {
    let completion = &world.signpost.completion;
    let phone_numbers = if completion.unlocked_phone_numbers.is_empty() {
        vec!["ID_UNLOCK_PHONE_NUMBER".to_owned()]
    } else {
        completion
            .unlocked_phone_numbers
            .iter()
            .map(|&k| layout_string(world, classes::THROWDOWN_COMPETITORS, k, COMPETITOR_HAL_ID))
            .collect()
    };
    let characters = if completion.unlocked_characters.is_empty() {
        vec!["#".to_owned()]
    } else {
        completion.unlocked_characters.iter().map(|&k| layout_string(world, classes::CHARACTERS_MARQUEE, k, 1)).collect()
    };
    let location_icon_text = completion
        .unlocked_locations
        .first()
        .map_or_else(|| "ID_UNLOCK_LOCATION".to_owned(), |&k| layout_string(world, class::FE_LOCATIONS, k, 2));
    let sponsor_unlock_text = if let Some(&item) = completion.unlocked_cac_items.first() {
        layout_string(world, classes::ASSET_GROUP_UNLOCKS, item, 1)
    } else if let Some(&item) = completion.unlocked_sponsor_cac_items.first() {
        layout_string(world, classes::ASSET_GROUP_SPONSORS, item, 1)
    } else {
        "ID_COMMON_UNKNOWN".to_owned()
    };
    let count = |list: &Vec<u64>| i32::try_from(list.len()).unwrap_or(i32::MAX);
    UnlockOverlayView {
        phone_numbers,
        characters,
        numbers_unlocked: completion.num_phone_numbers_unlocked > 0,
        location_unlocked: !completion.unlocked_locations.is_empty(),
        coverage_unlocked: completion.coverage_unlocked,
        mag_minute: signpost::check_challenge_results(world, "Mag Minute"),
        rob: signpost::check_challenge_results(world, "RobDyrdek Path"),
        sponsor_tape: completion.sponsor_me_created,
        throwdown: signpost::check_challenge_results(world, "Throwdown Locations"),
        dway: signpost::check_challenge_results(world, "DannyWay Path"),
        character_unlocked: !completion.unlocked_characters.is_empty(),
        property: signpost::check_challenge_results(world, "FE My Properties"),
        sponsors_to_cycle: i32::from(!completion.unlocked_cac_items.is_empty() || !completion.unlocked_sponsor_cac_items.is_empty()),
        paths: completion.num_paths_unlocked,
        location_icon_text,
        locations: count(&completion.unlocked_locations),
        thrasher_coverage: world.db.layout_word(signpost_progress::classes::PHOTO_ARTICLES, completion.photo_article, 6) == 0,
        sponsor_unlock_text,
        board: check_unlock_results(world, part::BOARD),
        truck: check_unlock_results(world, part::TRUCK),
        wheel: check_unlock_results(world, part::WHEEL),
        clothing: check_unlock_results(world, part::CLOTHING),
        shoe: check_unlock_results(world, part::SHOE),
        prize: cash_awards(world, credit::CHALLENGE_AWARD),
        purse: cash_awards(world, credit::CONTEST_PURSE),
        objective_bonus: cash_awards(world, credit::BONUS_OBJECTIVE),
        wager: cash_awards(world, credit::BETTING),
        sponsor_bonus: cash_awards(world, credit::SPONSORSHIP),
        trick_callout: cash_awards(world, credit::TRICK_CALLOUT),
        cash: signpost_progress::cash(world),
    }
}
