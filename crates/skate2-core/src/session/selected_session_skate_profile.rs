//! The `StartHub`'s skate.Profile: Career (`FrontEndState_SkateProfile`, FE state 45): its screen,
//! the screen's intro, and its input over the paused hub.
use super::SelectedLiveSession;
use crate::ui::{pause_menu::PauseMenuInput, skate_profile::View};

impl<T, C> SelectedLiveSession<T, C> {
    /// FE state 45's screen, while the state is on the stack.
    #[must_use]
    pub fn skate_profile_view(&self) -> Option<&View> {
        self.skate_profile.snapshot()
    }

    /// The screen's `IntroComplete`, from which the state takes input.
    pub fn skate_profile_intro_complete(&mut self) {
        self.skate_profile.acknowledge_intro();
    }

    /// One front-end frame of the state; its `PopState` enters the `StartHub` below again.
    pub(super) fn update_skate_profile(&mut self, input: PauseMenuInput, has_focus: bool) {
        if self.skate_profile.update(input, has_focus) {
            self.pause_menu.reenter();
        }
    }
}
