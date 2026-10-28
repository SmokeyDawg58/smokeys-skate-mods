//! The career's side of FE state 28: the movies its progression asks for (message type 15), the
//! sign-up poster's tutorial video, and the `cMsgFMVFinishedPlaying` a movie's end sends back.

use super::CareerSession;
use crate::challenge::{
    attrib_db::AttribDb,
    keys::class,
    signup::SlowDown,
    system::{Event, Output},
};
use std::sync::Arc;

/// `challenges.TutorialVideoAsset`: the `fe_movie` the poster's X plays.
const TUTORIAL_VIDEO_ASSET: u64 = 0x4842_7451_aef3_4906;
/// `UPDATE_TIME` of the `cMsgSetSimRate` the poster's X sends (`0.016666668`).
const NORMAL_SIM_TIME: f32 = f32::from_bits(0x3c88_8889);

#[derive(Debug, Default)]
pub(super) struct CareerMovie {
    /// The `fe_movie` keys pushed since the session last took them.
    pushes: Vec<u64>,
    /// The sign-up poster left for a tutorial video (`PushState(28)` over FE state 32).
    poster_under_movie: bool,
    /// The poster's `mDisabledSignupSlowdown`.
    disabled_signup_slowdown: bool,
}

impl CareerMovie {
    /// `mMovieKey`, then `PushState(28)`.
    pub(super) fn push(&mut self, key: u64) {
        self.pushes.push(key);
    }
}

impl CareerSession {
    /// The `fe_movie` keys pushed since the last call.
    pub(crate) fn take_movie_pushes(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.movie.pushes)
    }

    /// `cMsgFMVFinishedPlaying` to the challenge system: `ReceiveFMVFinishedPlayingMessage`
    /// (`0x825d2f18`) runs every running state machine's `FMVFinishedPlaying` script.
    pub(crate) fn fmv_finished_playing(&mut self) {
        self.system.post(Event::FmvFinishedPlaying);
    }

    /// The challenge system's Attrib database.
    pub(crate) fn attrib_db(&self) -> Arc<AttribDb> {
        Arc::clone(&self.system.world().db)
    }

    /// `FrontEndState_SignUpPoster::ReceiveInput` (`0x82657bf8`) keycode 5 (X). With a tutorial
    /// video (`GetHasTutorialVideo` `0x826480d8`) and the actions free, the actions block, the
    /// sign-up's render effect and slow-down stop, one normal-rate tick is asked for
    /// (`cMsgSetSimRate`), and FE state 28 is pushed with the challenge's `TutorialVideoAsset`;
    /// the poster leaves meanwhile (`OnLeave` `0x82641ea0` closes screen 19). Otherwise
    /// `core_error` sounds. The `cMsgHUDOverlay` it also sends has no handler but the Lua
    /// binding's, and no challenge script handles it.
    pub(crate) fn poster_tutorial_video(&mut self) {
        let has_video = self
            .hud
            .poster
            .as_ref()
            .is_some_and(|poster| poster.tutorial_nis.is_some());
        if !has_video || self.poster_blocked {
            self.sounds.push("FE_SOUND_CORE_ERROR");
            return;
        }
        let world = self.system.world_mut();
        let Some(activity) = world.signup.selected else {
            return;
        };
        let challenge = world.activities.activities[activity].key;
        let key = world
            .db
            .attribute(class::CHALLENGES, challenge, TUTORIAL_VIDEO_ASSET)
            .and_then(|a| a.ref_spec(0))
            .map_or(0, |(_, key)| key);
        self.poster_blocked = true;
        world.signup.show_render_effect = false;
        world.signup.slow_down = SlowDown::default();
        world.output(Output::SetSimRate {
            sim_time: NORMAL_SIM_TIME,
            ticks: 1,
        });
        self.movie.disabled_signup_slowdown = true;
        self.movie.poster_under_movie = true;
        self.hud.poster = None;
        self.movie.pushes.push(key);
    }

    /// FE state 28 popped: a poster under it enters again (`OnEnter` `0x82647f68`). Its actions
    /// block until the screen's intro is done, and the slow-down its X disabled resumes at rate 1.
    pub(crate) fn movie_state_left(&mut self) {
        if !std::mem::take(&mut self.movie.poster_under_movie) {
            return;
        }
        self.poster_blocked = true;
        if std::mem::take(&mut self.movie.disabled_signup_slowdown) {
            let world = self.system.world_mut();
            world.signup.show_render_effect = true;
            world.signup.slow_down = SlowDown {
                enabled: true,
                rate: 1.0,
            };
        }
        self.hud.poster = self.system.sign_up_poster_view();
    }
}
