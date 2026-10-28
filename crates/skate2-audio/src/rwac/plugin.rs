//! `rw::audio::core::PlugIn`: one stage of a voice. Plug-ins read the mixer's current buffer,
//! write the scratch buffer and swap, or (input plug-ins, type <= 3) produce a variable number of
//! samples through `PreProcess`/`Process`.

use super::mixer::Mixer;
use super::plugins::sub_mix::SubMixArena;
use super::voice::{VoiceCore, VoiceId};
use std::any::Any;

/// `rw::audio::core::BufferStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStatus {
    Unavailable = 0,
    Available = 1,
}

/// A plug-in of a voice: `PlugIn*` in the native code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginRef {
    pub voice: VoiceId,
    pub index: u8,
}

/// The fields of `PlugInDescRunTime` the port uses.
#[derive(Debug)]
pub struct PlugInDesc {
    pub name: &'static str,
    /// FourCC `guid` used by `PlugInRegistry::GetPlugInHandle`.
    pub guid: u32,
    /// 0 generator, 1 rechannel, 2 resample, 3 time stretch, 4 processor. Types <= 3 are input
    /// plug-ins (`Voice::CreateInstanceBody` sets `mInputPlugInStartPoint` from them).
    pub plugin_type: u8,
    /// Attribute defaults (`DefaultAttributes`, the parameter descriptors after the constructor
    /// parameters).
    pub attribute_defaults: &'static [f64],
}

/// `rw::audio::core::PlugIn` members.
#[derive(Debug)]
pub struct PlugInBase {
    pub desc: &'static PlugInDesc,
    /// `mAttribute[]`: floats and doubles (`Param`).
    pub attributes: Vec<f64>,
    pub latency_samples: f32,
    pub decay_samples: f32,
    pub input_channels: u8,
    pub output_channels: u8,
}

impl PlugInBase {
    pub fn new(desc: &'static PlugInDesc, input_channels: u8, output_channels: u8) -> Self {
        Self {
            desc,
            attributes: desc.attribute_defaults.to_vec(),
            latency_samples: 0.0,
            decay_samples: 0.0,
            input_channels,
            output_channels,
        }
    }

    /// `PlugIn::GetAttribute(int, float*)`.
    pub fn attr(&self, index: usize) -> f32 {
        self.attributes[index] as f32
    }
}

/// Mixer-thread services a plug-in may touch while processing, in a timer or on release.
pub struct Services<'a> {
    pub mixer: &'a mut Mixer,
    pub submixes: &'a mut SubMixArena,
    pub xma_contexts: &'a mut super::decoder::XmaContextPool,
    /// `System::mSystemTime` (seconds).
    pub system_time: f64,
    /// `gDacInterleaveOutBuffer`: 256 frames of 6 interleaved channels.
    pub dac_out: &'a mut [f32],
    /// `Dac::sStartRequested`.
    pub dac_start_requested: &'a mut bool,
}

pub trait PlugIn: Any + Send {
    fn base(&self) -> &PlugInBase;
    fn base_mut(&mut self) -> &mut PlugInBase;

    /// `pPreProcess` of input plug-ins: given the samples the next plug-in needs, returns the
    /// samples this plug-in needs from its source.
    fn pre_process(&mut self, _mixer: &mut Mixer, _discontinuity: bool, _requested: i32) -> i32 {
        0
    }

    /// `pProcess`.
    fn process(&mut self, voice: &mut VoiceCore, services: &mut Services<'_>, discontinuity: bool) -> BufferStatus;

    /// A `TIMERSTAGE_BEFOREMIXER` timer the plug-in registered at creation.
    fn timer(&mut self, _voice: &mut VoiceCore, _services: &mut Services<'_>) {}

    /// `ReleaseEvent`.
    fn release(&mut self, _voice: &mut VoiceCore, _services: &mut Services<'_>) {}

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

macro_rules! impl_plugin_any {
    () => {
        fn base(&self) -> &$crate::rwac::plugin::PlugInBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut $crate::rwac::plugin::PlugInBase {
            &mut self.base
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}
pub(crate) use impl_plugin_any;
