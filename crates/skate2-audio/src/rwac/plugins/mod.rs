//! RWAC plug-ins with their `sPlugInDescRunTime` data (type, FourCC, attribute defaults as dumped
//! from the debug executable).

pub mod dac;
pub mod delay;
pub mod distortion_clip;
pub mod frequency_shift_ssb;
pub mod gain;
pub mod gain_fader;
pub mod iir2;
pub mod packet_player;
pub mod pan2d;
pub mod rechannel;
pub mod resample;
pub mod reverb_model1;
pub mod route;
pub mod sample_capture;
pub mod send;
pub mod snd_player;
pub mod sub_mix;

use super::plugin::PlugInDesc;

const fn fourcc(code: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*code)
}

pub static SND_PLAYER1: PlugInDesc = PlugInDesc {
    name: "SndPlayer1",
    guid: fourcc(b"SnP1"),
    plugin_type: 0,
    attribute_defaults: &[0.0, 0.0, 0.0],
};
/// `sPlugInDescRunTime` 0x8300f740: a generator with one attribute, the time played.
pub static PACKET_PLAYER: PlugInDesc = PlugInDesc {
    name: "PacketPlayer",
    guid: fourcc(b"PPl0"),
    plugin_type: 0,
    attribute_defaults: &[0.0],
};
pub static RECHANNEL: PlugInDesc = PlugInDesc {
    name: "Rechannel",
    guid: fourcc(b"Rch0"),
    plugin_type: 1,
    attribute_defaults: &[],
};
pub static RESAMPLE: PlugInDesc = PlugInDesc {
    name: "Resample",
    guid: fourcc(b"Rsp0"),
    plugin_type: 2,
    attribute_defaults: &[1.0],
};
pub static GAIN: PlugInDesc = PlugInDesc {
    name: "Gain",
    guid: fourcc(b"Gai0"),
    plugin_type: 4,
    attribute_defaults: &[1.0],
};
pub static PAN2D1: PlugInDesc = PlugInDesc {
    name: "Pan2D1",
    guid: fourcc(b"Pn21"),
    plugin_type: 4,
    attribute_defaults: &[0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 45.0, 135.0],
};
pub static SEND: PlugInDesc = PlugInDesc {
    name: "Send",
    guid: fourcc(b"Sen0"),
    plugin_type: 4,
    attribute_defaults: &[1.0],
};
pub static SUB_MIX: PlugInDesc = PlugInDesc {
    name: "SubMix",
    guid: fourcc(b"Sub0"),
    plugin_type: 4,
    attribute_defaults: &[],
};
pub static ROUTE: PlugInDesc = PlugInDesc {
    name: "Route",
    guid: fourcc(b"Rou0"),
    plugin_type: 4,
    attribute_defaults: &[],
};
pub static SAMPLE_CAPTURE: PlugInDesc = PlugInDesc {
    name: "SampleCapture",
    guid: fourcc(b"Scp0"),
    plugin_type: 4,
    attribute_defaults: &[0.0],
};
pub static DAC: PlugInDesc = PlugInDesc {
    name: "Dac",
    guid: fourcc(b"Dac0"),
    plugin_type: 4,
    attribute_defaults: &[3.0, 48000.0, 0.0],
};
pub static HIGH_PASS_IIR2: PlugInDesc = PlugInDesc {
    name: "HighPassIir2",
    guid: fourcc(b"HI20"),
    plugin_type: 4,
    attribute_defaults: &[0.0],
};
pub static LOW_PASS_IIR2: PlugInDesc = PlugInDesc {
    name: "LowPassIir2",
    guid: fourcc(b"LI20"),
    plugin_type: 4,
    attribute_defaults: &[96000.0],
};
pub static HIGH_SHELF_IIR2: PlugInDesc = PlugInDesc {
    name: "HighShelfIir2",
    guid: fourcc(b"HS20"),
    plugin_type: 4,
    attribute_defaults: &[96000.0, 1.0],
};
pub static PEAKING_IIR2: PlugInDesc = PlugInDesc {
    name: "PeakingIir2",
    guid: fourcc(b"PI20"),
    plugin_type: 4,
    attribute_defaults: &[96000.0, 1.0, 3.0],
};
pub static DISTORTION_CLIP: PlugInDesc = PlugInDesc {
    name: "DistortionClip",
    guid: fourcc(b"DCl0"),
    plugin_type: 4,
    attribute_defaults: &[100.0],
};
/// One constructor parameter (default 1.0) ahead of the attribute.
pub static FREQUENCY_SHIFT_SSB: PlugInDesc = PlugInDesc {
    name: "FrequencyShiftSsb",
    guid: fourcc(b"FSS0"),
    plugin_type: 4,
    attribute_defaults: &[0.0],
};
/// The attribute is an output (the current gain); event 0 starts a fade.
pub static GAIN_FADER: PlugInDesc = PlugInDesc {
    name: "GainFader",
    guid: fourcc(b"GaF0"),
    plugin_type: 4,
    attribute_defaults: &[1.0],
};
/// One constructor parameter (the longest delay, default 0.0) ahead of the delay and feedback.
pub static DELAY: PlugInDesc = PlugInDesc {
    name: "Delay",
    guid: fourcc(b"Del0"),
    plugin_type: 4,
    attribute_defaults: &[0.0, 0.0],
};
/// One constructor parameter (the space size the lines hold, default 2.0) ahead of the time,
/// size and brightness.
pub static REVERB_MODEL1: PlugInDesc = PlugInDesc {
    name: "ReverbModel1",
    guid: fourcc(b"RM10"),
    plugin_type: 4,
    attribute_defaults: &[0.0, 15.0, 1.0],
};
