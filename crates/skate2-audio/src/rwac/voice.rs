//! `rw::audio::core::Voice`.

use super::plugin::PlugIn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceId(pub u32);

/// `Voice::mState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    Playing = 0,
    /// `ExpelAfterDecay`: expelled once the decay samples are eaten.
    ExpelAfterDecay = 1,
    /// `ExpelImmediate`: out of the active list; the owner still has to `Release` it.
    Expelled = 2,
}

/// The voice members other than the plug-ins.
#[derive(Debug)]
pub struct VoiceCore {
    pub id: VoiceId,
    pub name: &'static str,
    pub processing_stage: u8,
    pub priority: f32,
    pub state: VoiceState,
    /// `mDecaySamples`: the sum the plug-ins declared (Resample adds 6).
    pub decay_samples: f32,
    pub decay_eaten_samples: f32,
    pub expel_after_decay_samples: f32,
    pub pitch: f32,
    /// Index of the last input plug-in (type <= 3), 255 for none.
    pub input_start_point: u8,
    /// Plug-ins after this index get `discontinuity = true`.
    pub discontinuity_plugin: u8,
    pub creation_time_stamp: u32,
}

pub struct Voice {
    pub core: VoiceCore,
    pub plugins: Vec<Box<dyn PlugIn>>,
}

impl Voice {
    /// `Voice::ExpelAfterDecay` 0x82be38c8.
    pub fn expel_after_decay(core: &mut VoiceCore) {
        if core.state == VoiceState::Playing {
            core.expel_after_decay_samples = core.decay_samples;
            core.state = VoiceState::ExpelAfterDecay;
        }
    }
}
