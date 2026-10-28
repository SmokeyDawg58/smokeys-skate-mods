//! `FrontEndState_Sponsor` (FE state 11) and what its screen (`unlock/sponsoroverlay`, screen 21)
//! reads through the `FEUnlock` natives. `cSignPostUtil::ShowUnlocksScreen` (`0x825cca80`) sends
//! `cMsgOpenSponsorOverlay` for a message with a positive `SponsorID`, and
//! `FrontEndManager::ReceiveOpenSponsorOverlayMsg` (`0x82614450`) stores that id as the state's
//! `mActiveState` (`eSponsorScreenStates`) before pushing the state.

use super::{
    keys::{hash64, hash_value32},
    signpost,
    signpost_progress::{self, classes::SPONSORS},
    sponsors,
    system::World,
};
use crate::ui::sponsor_browser::SponsorBrowser;

/// `eSponsorScreenStates`.
pub mod state {
    pub const TAPE_UNLOCKED: i32 = 0;
    pub const BOARD_SPONSORED: i32 = 1;
    pub const TRUCK_SELECTABLE: i32 = 2;
    pub const WHEEL_SELECTABLE: i32 = 3;
    pub const SHOE_SELECTABLE: i32 = 4;
}

/// `sponsor`'s type: 1 board, 2 truck, 3 wheel, 4 shoe.
const SPONSOR_TYPE: u64 = 0x4487_6c6f_37f2_1487;
/// `sponsor`'s name (`char*`, layout +0).
const NAME: u32 = 0;
/// `sponsor`'s logo texture (`char*`, layout +4).
const RESOURCE: u32 = 1;
/// The shoe sponsors the constructor leaves out.
const SKIPPED_SHOES: [&str; 3] = ["shoebigblack", "shoenike", "shoenixon"];

/// What the screen reads when it initialises (`sponsoroverlay::InitState`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SponsorOverlayView {
    /// `Sponsor_GetInitialState` (`0x82624208`): `mActiveState`.
    pub state: i32,
    /// `Sponsor_GetBoardSponsorName` (`0x8260d760`): `cSignPostSponsorMan::GetBoardSponsorHALID`
    /// (`0x825d1358`), the beaten pro's board sponsor's HAL id.
    pub board_sponsor_hal_id: String,
    /// `Sponsor_GetBoardSponsorIcon` (`0x82624070`), asked in the board state only: the board
    /// list's logo at `mSelectedSponsorIndex` after `SetBoardSponsorIndex`.
    pub board_sponsor_icon: String,
    /// `Sponsor_Get{Truck,Wheel,Shoe}SponsorSwfs`: the selectable state's logos, the trucks' for
    /// a state `InitState` does not know.
    pub swfs: Vec<String>,
}

/// FE state 11 on the stack: what its screen initialised from, and its `mSponsorSelector`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SponsorOverlayHud {
    pub view: SponsorOverlayView,
    pub browser: SponsorBrowser,
}

/// The sponsors `FrontEndState_Sponsor`'s constructor (`0x82642468`) lists for a type: each
/// `sponsor` collection in the class's collection order whose name is not `default`, its name
/// and logo; the shoes leave out `shoebigblack`, `shoenike` and `shoenixon`.
fn sponsors_of(world: &World, kind: i32) -> Vec<(String, String)> {
    let db = &world.db;
    db.table_order(SPONSORS)
        .into_iter()
        .filter(|&key| db.layout_text(SPONSORS, key, NAME).is_some_and(|name| name != "default"))
        .filter(|&key| db.attribute(SPONSORS, key, SPONSOR_TYPE).and_then(|a| a.i32(0)).unwrap_or(0) == kind)
        .filter(|&key| kind != 4 || !SKIPPED_SHOES.iter().any(|&shoe| hash64(shoe) == key))
        .map(|key| {
            let text = |index| db.layout_text(SPONSORS, key, index).unwrap_or_default().to_owned();
            (text(NAME), text(RESOURCE))
        })
        .collect()
}

/// The list a selectable state picks from: the trucks' for any state but wheel and shoe.
const fn list_type(state: i32) -> i32 {
    match state {
        state::WHEEL_SELECTABLE => 3,
        state::SHOE_SELECTABLE => 4,
        _ => 2,
    }
}

/// The screen's reads for `state` and its `mSponsorSelector`, which `InitState` fills (`Init`)
/// in every state but the tape and the board. `selected` is the state's
/// `mSelectedSponsorIndex`: `SetBoardSponsorIndex` (`0x82642a68`) moves it to the last board
/// sponsor named as the beaten pro's, and keeps it when none is.
#[must_use]
pub fn hud(world: &World, state: i32, selected: &mut usize) -> SponsorOverlayHud {
    let board_sponsor_hal_id = sponsors::demo_sponsor_hal_id(world);
    let mut board_sponsor_icon = String::new();
    if state == state::BOARD_SPONSORED {
        let boards = sponsors_of(world, 1);
        let name = sponsors::board_sponsor_name(world);
        if let Some(index) = boards.iter().rposition(|(board, _)| *board == name) {
            *selected = index;
        }
        board_sponsor_icon = boards.get(*selected).map(|(_, logo)| logo.clone()).unwrap_or_default();
    }
    let swfs: Vec<String> = sponsors_of(world, list_type(state)).into_iter().map(|(_, logo)| logo).collect();
    let mut browser = SponsorBrowser::new();
    if state != state::TAPE_UNLOCKED && state != state::BOARD_SPONSORED {
        browser.init(i32::try_from(swfs.len()).unwrap_or(i32::MAX));
    }
    SponsorOverlayHud { view: SponsorOverlayView { state, board_sponsor_hal_id, board_sponsor_icon, swfs }, browser }
}

fn trace(state: i32, setting: &str, name: &str) {
    if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
        eprintln!("career: sponsor overlay state {state} sets {setting} to {name}");
    }
}

/// Diagnostics: the text message setting `setting` (its name and text) stored as
/// `SetMessageSetting<char const*>` (`0x82666cd0`) stores one, then `cSignPostManager::
/// SendMessage` of the progression message `message`.
pub fn diagnostic_send_message(world: &mut World, setting: Option<(&str, &str)>, message: &str) {
    if let Some((setting, text)) = setting {
        world.signpost.set_message_setting_value(setting, hash_value32(text));
    }
    signpost::send_message_by_name(world, message);
}

/// `FrontEndState_Sponsor::NextSelected` (`0x82667fa0`) after its `SetScreenState`: the board
/// sponsor stored as `Board Sponsor` and `Board Sponsor Accepted` sent, or the picked truck,
/// wheel or shoe sponsor (`GetSelectedSponsorName`, from `SetIndexPicked`) stored and
/// `Sponsorships Earned` counted. The tape state only pops.
pub fn next_selected(world: &mut World, state: i32, selected: usize) {
    match state {
        state::BOARD_SPONSORED => {
            let name = sponsors::board_sponsor_name(world);
            trace(state, "Board Sponsor", &name);
            world.signpost.set_message_setting_value("Board Sponsor", hash_value32(&name));
            signpost::send_message_by_name(world, "Board Sponsor Accepted");
        }
        state::TRUCK_SELECTABLE | state::WHEEL_SELECTABLE | state::SHOE_SELECTABLE => {
            let setting = match state {
                state::TRUCK_SELECTABLE => "Truck Sponsor",
                state::WHEEL_SELECTABLE => "Wheel Sponsor",
                _ => "Shoe Sponsor",
            };
            if let Some((name, _)) = sponsors_of(world, list_type(state)).get(selected) {
                trace(state, setting, name);
                world.signpost.set_message_setting_value(setting, hash_value32(name));
            }
            signpost_progress::increment_points_by_name(world, "Sponsorships Earned", 1);
        }
        _ => {}
    }
}
