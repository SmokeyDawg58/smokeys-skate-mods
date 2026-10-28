//! `FrontEndState_Sponsor` (FE state 11) on the career: the overlay `cMsgOpenSponsorOverlay`
//! opens, its input (`ReceiveInput` `0x82674800`, then the screen's `UpdateInput` for every key),
//! the sponsor confirmation popup, `NextSelected` (`0x82667fa0`) and the `cMsgHUDContinue` that
//! `OnOutroDone` (`0x82659920`) sends once the screen's outro ends.

use super::{CareerSession, DialogOwner, PausingState, SelectedLiveSession};
use crate::{
    challenge::{Event, sponsor_overlay::state},
    ui::{pause_menu::PauseMenuInput, sponsor_browser::Key},
};

impl CareerSession {
    /// `ReceiveOpenSponsorOverlayMsg` (`0x82614450`): `mActiveState`, then `PushState(11)`, whose
    /// `OnEnter` (`0x826427d0`) opens screen 21.
    pub(super) fn open_sponsor_overlay(&mut self, sponsor: i32) {
        let hud = self.system.sponsor_overlay(sponsor, &mut self.sponsor_selected);
        if std::env::var_os("SKATE2_TRACE_CAREER").is_some() {
            let view = &hud.view;
            eprintln!("career: sponsor overlay state {} board {} icon {:?} logos {}", view.state, view.board_sponsor_hal_id, view.board_sponsor_icon, view.swfs.len());
        }
        self.hud.sponsor_overlay = Some(hud);
        self.pausing.push(PausingState::Sponsor);
        self.sponsor_ready = false;
    }

    /// The state's input once its screen is ready. `ReceiveInput` takes A: `NextSelected` in the
    /// tape and board states, else the `Skateshop_SponsorConfirmation` popup (177: No and Yes,
    /// default No, not cancellable). It returns 0, so every key also reaches the screen
    /// (`sponsoroverlay::UpdateInput`), whose `mSponsorSelector` steps on the directions and
    /// sounds `FE_SOUND_SPONSOR_NAV`.
    pub(super) fn sponsor_input(&mut self, input: PauseMenuInput) {
        if !self.sponsor_ready {
            return;
        }
        let Some(hud) = self.hud.sponsor_overlay.as_mut() else { return };
        if input.select {
            if hud.view.state < state::TRUCK_SELECTABLE {
                self.sponsor_next_selected();
            } else {
                self.dialog_owner = DialogOwner::SponsorConfirm;
                let (title, text) = ("ID_SPONSORSHIPBROWSER_CONFIRM_TITLE", "ID_SPONSORSHIPBROWSER_CONFIRM_TEXT");
                self.hud.dialog_options = None;
                self.hud.dialog = Some((title.to_owned(), text.to_owned(), 0));
                self.sounds.push("FE_SOUND_CORE_POPUP");
            }
            return;
        }
        for (pressed, key) in [(input.right, Key::Right), (input.left, Key::Left), (input.down, Key::Down), (input.up, Key::Up)] {
            if pressed {
                hud.browser.update_input(key);
                self.sounds.push("FE_SOUND_SPONSOR_NAV");
            }
        }
    }

    /// `NextSelected`: `SetScreenState(mActiveState)`, whose `SetState` hands the selector's
    /// `GetSelectedIndex` to `SetIndexPicked` in the board and selectable states and plays the
    /// state's outro; the choice; then `PopState`, whose `OnLeave` (`0x826427e8`) closes the
    /// screen.
    pub(super) fn sponsor_next_selected(&mut self) {
        let Some(hud) = self.hud.sponsor_overlay.take() else { return };
        let state = hud.view.state;
        if (state::BOARD_SPONSORED..=state::SHOE_SELECTABLE).contains(&state)
            && let Ok(selected) = usize::try_from(hud.browser.selected_index())
        {
            self.sponsor_selected = selected;
        }
        self.system.sponsor_next_selected(state, self.sponsor_selected);
        if let Some(at) = self.pausing.iter().rposition(|pausing| *pausing == PausingState::Sponsor) {
            self.pausing.remove(at);
        }
        self.sponsor_ready = false;
        self.sponsor_outro_pending = true;
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `FrontEndState_Sponsor`'s `OnScreenReady` when its screen's intro reports `IntroComplete`.
    pub fn acknowledge_career_sponsor_intro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && career.hud.sponsor_overlay.is_some()
        {
            career.sponsor_ready = true;
        }
    }

    /// Diagnostics: the progression message `message` sent, after the text message setting
    /// `setting` (its name and text) stored. False without a career.
    pub fn diagnostic_send_signpost_message(&mut self, setting: Option<(&str, &str)>, message: &str) -> bool {
        self.career.as_mut().map(|career| career.system.diagnostic_send_message(setting, message)).is_some()
    }

    /// `OnOutroDone` (`0x82659920`) when the closed screen's outro reports `OutroComplete`:
    /// `cMsgHUDContinue`.
    pub fn acknowledge_career_sponsor_outro(&mut self) {
        if let Some(career) = self.career.as_mut()
            && std::mem::take(&mut career.sponsor_outro_pending)
        {
            career.system.post(Event::HudContinue);
        }
    }
}
