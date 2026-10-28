//! `FE::Chyron`, `FrontEndManager::mChyron`: the now-playing card main.swf's `gChyron` layer
//! (`source/screens/main/chyron.swf`) shows for a music player song while the front-end state
//! on top enables it.

/// `Chyron::eChyronState`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    Hidden,
    Intro,
    Showing,
    Outro,
}

/// An `AptIntegration::CallFunction` the chyron makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// `eASCall_SetChyronVisible`, `_global.Chyron_SetVisible` (`SetVisible` 0x82619120).
    SetVisible(bool),
    /// `eASCall_DisplayChyron`, `_global.Chyron_Display` with `MusicCore::GetSongInfo`'s
    /// "IPOD" artist, title and album for the song.
    Display(u32),
    /// `eASCall_PlayChyronOutro`, `_global.Chyron_PlayOutro` (`PlayOutro` 0x826190b8).
    PlayOutro,
}

/// `FrontEndManager::Update` 0x82640130 plays the outro once the card has shown this long.
const DISPLAY_MS: u32 = 5000;
/// `GameModule::Update`'s `FrontEndManager::Update(0x682)`, which divides by 100.
pub const UPDATE_MS: u32 = 16;

/// `gFEStateInfo` 0x82fded80's `mbEnableChyron` (byte 1 of each 20-byte row).
#[must_use]
pub fn enables_chyron(state: i32) -> bool {
    matches!(state, 3 | 6 | 18..=25 | 30 | 35 | 40 | 45 | 47..=54 | 73 | 80)
}

#[derive(Clone, Debug, Default)]
pub struct Chyron {
    /// `mbEnabled`.
    enabled: bool,
    /// `mChyronState`.
    state: State,
    /// `mTimeDisplayed`, in milliseconds.
    time_displayed: u32,
    calls: Vec<Call>,
}

impl Chyron {
    #[must_use]
    pub const fn state(&self) -> State {
        self.state
    }

    pub fn take_calls(&mut self) -> Vec<Call> {
        std::mem::take(&mut self.calls)
    }

    /// `StateManagerFrontEndBase::UpdateOnEnter` 0x82d663f0's `UpdateChyron(mbEnableChyron)`
    /// (`StateManagerFrontEnd::UpdateChyron` 0x82656a68; `kbDemoMarketing` is false). The state's
    /// `Show(true)` then finds the chyron as `UpdateFEChyron` 0x82657340 would leave it.
    pub fn enter(&mut self, state: i32) {
        self.enable(enables_chyron(state));
    }

    /// `Chyron::Enable` 0x8261d188 with `kbFEShowChyron` on: enabling shows the hidden layer;
    /// disabling plays the outro over a card in or showing, else hides the layer.
    fn enable(&mut self, enable: bool) {
        if self.enabled == enable {
            return;
        }
        self.enabled = enable;
        if enable {
            self.state = State::Hidden;
            self.calls.push(Call::SetVisible(true));
        } else if matches!(self.state, State::Intro | State::Showing) {
            self.calls.push(Call::PlayOutro);
            self.state = State::Outro;
        } else {
            self.state = State::Hidden;
            self.time_displayed = 0;
            self.calls.push(Call::SetVisible(false));
        }
    }

    /// `Chyron::Display(unsigned)` 0x82639a08 into `Display(artist, title, album)` 0x82618ff0:
    /// only while enabled (`GameMusicDisabled` is the 360's user soundtrack, never on here).
    pub fn display(&mut self, song: u32) {
        if song == 0 || !self.enabled {
            return;
        }
        if self.state != State::Showing {
            self.state = State::Intro;
        }
        self.time_displayed = 0;
        self.calls.push(Call::Display(song));
    }

    /// `FrontEndManager::Update`'s count: a card showing for five seconds plays its outro.
    pub fn update(&mut self) {
        if self.enabled && self.state == State::Showing {
            self.time_displayed += UPDATE_MS;
            if self.time_displayed >= DISPLAY_MS {
                self.calls.push(Call::PlayOutro);
                self.state = State::Outro;
            }
        }
    }

    /// `AudioCmdTranslator::sMethod_ChyronIntroDone` 0x826368e8.
    pub fn intro_done(&mut self) {
        self.state = State::Showing;
    }

    /// `AudioCmdTranslator::sMethod_ChyronOutroDone` 0x82636918.
    pub fn outro_done(&mut self) {
        self.state = State::Hidden;
        self.time_displayed = 0;
        if !self.enabled {
            self.calls.push(Call::SetVisible(false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_runs_intro_show_outro() {
        let mut c = Chyron::default();
        c.display(7);
        assert!(c.take_calls().is_empty());
        c.enter(73);
        assert_eq!(c.take_calls(), [Call::SetVisible(true)]);
        c.display(7);
        assert_eq!((c.state(), c.take_calls()), (State::Intro, vec![Call::Display(7)]));
        c.intro_done();
        for _ in 0..312 {
            c.update();
        }
        assert!(c.take_calls().is_empty());
        c.update();
        assert_eq!((c.state(), c.take_calls()), (State::Outro, vec![Call::PlayOutro]));
        c.outro_done();
        assert_eq!((c.state(), c.take_calls()), (State::Hidden, vec![]));
    }

    #[test]
    fn leaving_plays_outro_then_hides() {
        let mut c = Chyron::default();
        c.enter(22);
        c.display(3);
        c.take_calls();
        c.enter(40);
        assert!(c.take_calls().is_empty());
        c.enter(1);
        assert_eq!(c.take_calls(), [Call::PlayOutro]);
        c.outro_done();
        assert_eq!(c.take_calls(), [Call::SetVisible(false)]);
        c.enter(7);
        assert!(c.take_calls().is_empty());
    }
}
