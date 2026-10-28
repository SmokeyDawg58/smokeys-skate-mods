//! RenderWare Audio Core (`rw::audio::core`, map library `rwaudiocore`) as Skate 2 uses it:
//! voices made of plug-in chains, a command buffer the game fills under the system lock, timers,
//! and a mixer producing 256-sample frames of 6 channels at 48 kHz.
//!
//! Threading: the game and the device thread share one [`AudioCore`] behind a mutex (the
//! `System::Lock` the game takes). Commands still queue and run at the next frame, as on the 360.

pub mod core;
pub mod decoder;
pub mod delay_line;
pub mod ealayer3;
pub mod kernels;
pub mod mixer;
pub mod plugin;
pub mod plugins;
pub mod seek;
pub mod stream_pool;
pub mod voice;

pub use self::core::{AudioCore, Command};
pub use mixer::Mixer;
pub use plugin::{BufferStatus, PlugIn, PlugInBase, PlugInDesc, PluginRef};
pub use voice::{Voice, VoiceCore, VoiceId, VoiceState};

/// Samples in one mixer frame (`Dac::XenonDownMix` 0x100).
pub const FRAME_SAMPLES: usize = 256;
/// Mixer and Dac rate (`CreateMasteringVoice` Dac event 2 = 48000).
pub const SAMPLE_RATE: f32 = 48000.0;
/// Channels of the mastering mix: FL, C, FR, SL, SR, LFE inside RWAC.
pub const MASTER_CHANNELS: usize = 6;
/// Channel capacity of a mixer sample buffer (`mBuffer[3][16384]` = 64 x 256).
pub const BUFFER_CHANNELS: usize = 64;
