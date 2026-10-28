//! Bounded API-2 audio commands and canonical PCM WAV validation.
//! Only local, mod-relative PCM16 WAV files are accepted by extension 1.
use serde::{Deserialize, Serialize};

pub const MAX_WAV_BYTES: u64 = 8 * 1024 * 1024;
pub fn default_fade() -> f32 {
    0.03
}
fn one() -> f32 {
    1.0
}
fn spatial_scale() -> f32 {
    0.1
}
fn yes() -> bool {
    true
}
fn fade_in() -> f32 {
    0.01
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioPlayOptions {
    pub path: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default, rename = "loop")]
    pub looping: bool,
    #[serde(default = "one")]
    pub volume: f32,
    #[serde(default = "one")]
    pub pitch: f32,
    #[serde(default = "yes")]
    pub spatial: bool,
    #[serde(default = "spatial_scale")]
    pub spatial_scale: f32,
    #[serde(default)]
    pub paused: bool,
    #[serde(default = "fade_in")]
    pub fade_in: f32,
    /// Audio extension 3: play through the game's own (native) mixer: a per-mod bank, the retail
    /// emitter distance law (MixMap Emitter outputs), the environment (reverb) send, the retail
    /// panner. `None` (the default, user decision 2026-10-04): native while the native audio runs
    /// and a native voice is free, else a Bevy voice; `Some(true)`: native only (an error
    /// otherwise); `Some(false)`: always the Bevy voice.
    #[serde(default)]
    pub native: Option<bool>,
    /// Native only: the reach of a positional sound (the `.ems` record shape: a sphere of
    /// `radius` m with an inner `core`, and the falloff curve). Default [`DEFAULT_REACH`].
    #[serde(default)]
    pub falloff: Option<NativeFalloff>,
    /// Native only: send into the environment (reverb) bus (default true, as retail emitters).
    #[serde(default)]
    pub reverb: Option<bool>,
    /// Native only: the volume group, `world` (the Ambience volume, as world emitters; default)
    /// or `player` (the Effects volume).
    #[serde(default)]
    pub group: Option<String>,
}

/// The reach of a native positional sound: retail's emitter record test (sphere of `radius`, an
/// inner `core` fraction at full level) and falloff curve (`eVolumeFalloffType`).
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NativeFalloff {
    pub radius: f32,
    #[serde(default)]
    pub core: f32,
    #[serde(default)]
    pub curve: FalloffCurve,
}

/// `eVolumeFalloffType`: 0 = (1 − d)², 1 = 1 − d, other = flat.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FalloffCurve {
    #[default]
    Squared,
    Linear,
    Flat,
}

impl FalloffCurve {
    /// The retail `eVolumeFalloffType` number.
    pub fn retail_type(self) -> i32 {
        match self {
            Self::Squared => 0,
            Self::Linear => 1,
            Self::Flat => 2,
        }
    }
}

impl NativeFalloff {
    pub fn validate(&self) -> bool {
        between(self.radius, 0.1, 10_000.0) && between(self.core, 0.0, 1.0)
    }
}

/// The reach of a positional native sound that gives none (native routing became the default on
/// 2026-10-04, so a sound written for the Bevy voice needs one): a 40 m sphere, no core, the
/// squared curve: retail's audible reach of a traffic car (its vehicle list is cut at 40 m;
/// `game_audio::world_sources::TRAFFIC_LIST_RADIUS`, doc 15).
pub const DEFAULT_REACH: NativeFalloff = NativeFalloff {
    radius: 40.0,
    core: 0.0,
    curve: FalloffCurve::Squared,
};

/// The volume groups a native mod sound can play in.
pub const NATIVE_GROUPS: [&str; 2] = ["world", "player"];

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioUpdateOptions {
    pub volume: Option<f32>,
    pub pitch: Option<f32>,
    pub paused: Option<bool>,
    pub position: Option<[f32; 3]>,
    pub offset: Option<[f32; 3]>,
}

fn between(x: f32, lo: f32, hi: f32) -> bool {
    x.is_finite() && (lo..=hi).contains(&x)
}
pub(crate) fn point(v: &[f32; 3]) -> bool {
    v.iter().all(|x| between(*x, -100_000.0, 100_000.0))
}
pub(crate) fn offset(v: &[f32; 3]) -> bool {
    v.iter().all(|x| between(*x, -100.0, 100.0))
}

pub fn valid_audio_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 256
        && path.to_ascii_lowercase().ends_with(".wav")
        && !path.chars().any(|c| matches!(c, '\\' | ':' | '#' | '?'))
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}
/// A front-end sound name (`sdk.audio.frontend`): a retail `fe` record name, lower-case ASCII
/// letters, digits and `_`, 1..=64 bytes. Unknown names are accepted here and play nothing.
pub fn valid_frontend_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
impl AudioPlayOptions {
    pub fn validate(&self) -> bool {
        valid_audio_path(&self.path)
            && self.body.as_deref().is_none_or(crate::schema::valid_id)
            && self.position.as_ref().is_none_or(point)
            && !(self.body.is_some() && self.position.is_some())
            && offset(&self.offset)
            && between(self.volume, 0.0, 1.0)
            && between(self.pitch, 0.25, 4.0)
            && between(self.spatial_scale, 0.001, 1.0)
            && between(self.fade_in, 0.0, 2.0)
            && self.validate_native()
    }

    /// The native-only fields: rejected on a voice that asks for Bevy (`native = false`); checked
    /// otherwise (a positional native sound without `falloff` gets [`DEFAULT_REACH`]).
    fn validate_native(&self) -> bool {
        if self.native == Some(false) {
            return self.falloff.is_none() && self.reverb.is_none() && self.group.is_none();
        }
        self.falloff.as_ref().is_none_or(NativeFalloff::validate)
            && self
                .group
                .as_deref()
                .is_none_or(|g| NATIVE_GROUPS.contains(&g))
    }

    /// Whether the sound goes to the native mixer when it can (everything but `native = false`).
    pub fn wants_native(&self) -> bool {
        self.native != Some(false)
    }

    /// The reach of a positional native sound: its `falloff`, else [`DEFAULT_REACH`].
    pub fn reach(&self) -> NativeFalloff {
        self.falloff.unwrap_or(DEFAULT_REACH)
    }
}
impl AudioUpdateOptions {
    pub fn validate(&self) -> bool {
        self.volume.is_none_or(|v| between(v, 0.0, 1.0))
            && self.pitch.is_none_or(|v| between(v, 0.25, 4.0))
            && self.position.as_ref().is_none_or(point)
            && self.offset.as_ref().is_none_or(offset)
    }
}

/// A MixMap output to watch (`sdk.audio.watch`): slot by name, object, instance, output id.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MixMapKey {
    pub slot: String,
    #[serde(default)]
    pub object: u32,
    #[serde(default)]
    pub instance: u32,
    pub output: u32,
}

/// The MixMap slots a watch can name.
pub const MIXMAP_SLOTS: [&str; 7] = [
    "global",
    "player",
    "ambience",
    "collision",
    "traffic",
    "pedestrian",
    "emitter",
];
/// Payload words of a post, globals / MixMap outputs per watch list.
pub const MAX_WORDS: usize = 32;
pub const MAX_WATCH: usize = 16;

/// A retail class / global name (`c_emitter`, `g_snd`).
pub fn valid_symbol(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

impl MixMapKey {
    pub fn validate(&self) -> bool {
        MIXMAP_SLOTS.contains(&self.slot.as_str())
            && self.object <= 127
            && self.instance <= 31
            && self.output <= 31
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WavInfo {
    pub seconds: f64,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Validate bounded, uncompressed PCM16 and rebuild a minimal, canonical WAV.
/// Unknown metadata chunks never reach the backend decoder. No resampling here.
pub fn canonical_pcm_wav(bytes: &[u8]) -> Result<(Vec<u8>, WavInfo), String> {
    canonical_pcm_wav_limited(bytes, MAX_WAV_BYTES, 30.0)
}

/// [`canonical_pcm_wav`] with other limits (audio content overlays: ambience beds and streams are
/// longer than a sample).
pub fn canonical_pcm_wav_limited(
    bytes: &[u8],
    max_bytes: u64,
    max_seconds: f64,
) -> Result<(Vec<u8>, WavInfo), String> {
    let bad = || {
        format!("Audio requires a valid PCM16 WAV: 1-2 channels, 8-48 kHz, 0-{max_seconds} seconds")
    };
    if bytes.len() < 44
        || bytes.len() as u64 > max_bytes
        || &bytes[0..4] != b"RIFF"
        || &bytes[8..12] != b"WAVE"
    {
        return Err(bad());
    }
    let u16le = |b: &[u8]| u16::from_le_bytes([b[0], b[1]]);
    let u32le = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    if u32le(&bytes[4..8]) as u64 + 8 != bytes.len() as u64 {
        return Err(bad());
    }
    let mut fmt = None;
    let mut data = None;
    let mut cursor = 12usize;
    while cursor < bytes.len() {
        let header_end = cursor.checked_add(8).ok_or_else(bad)?;
        if header_end > bytes.len() {
            return Err(bad());
        }
        let count = u32le(&bytes[cursor + 4..header_end]) as usize;
        let end = header_end.checked_add(count).ok_or_else(bad)?;
        if end > bytes.len() {
            return Err(bad());
        }
        let chunk = &bytes[header_end..end];
        match &bytes[cursor..cursor + 4] {
            b"fmt " => {
                if fmt.is_some() || !(count == 16 || count == 18) {
                    return Err(bad());
                }
                if u16le(chunk) != 1
                    || u16le(&chunk[14..]) != 16
                    || (count == 18 && u16le(&chunk[16..]) != 0)
                {
                    return Err(bad());
                }
                let channels = u16le(&chunk[2..]);
                let sample_rate = u32le(&chunk[4..]);
                if !(1..=2).contains(&channels)
                    || !(8000..=48000).contains(&sample_rate)
                    || u16le(&chunk[12..]) != channels * 2
                    || u32le(&chunk[8..]) != sample_rate * u32::from(channels) * 2
                {
                    return Err(bad());
                }
                fmt = Some((channels, sample_rate));
            }
            b"data" => {
                if data.is_some() {
                    return Err(bad());
                }
                data = Some(chunk);
            }
            _ => {}
        }
        cursor = end.checked_add(count & 1).ok_or_else(bad)?;
        if cursor > bytes.len() {
            return Err(bad());
        }
    }
    let (channels, sample_rate) = fmt.ok_or_else(bad)?;
    let data = data.ok_or_else(bad)?;
    let block = usize::from(channels) * 2;
    if data.is_empty() || data.len() % block != 0 {
        return Err(bad());
    }
    let seconds = (data.len() / block) as f64 / f64::from(sample_rate);
    if seconds > max_seconds {
        return Err(bad());
    }
    let mut out = Vec::with_capacity(data.len() + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(data.len() as u32 + 36).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * u32::from(channels) * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    Ok((
        out,
        WavInfo {
            seconds,
            channels,
            sample_rate,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn wav() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&196u32.to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&8000u32.to_le_bytes());
        b.extend_from_slice(&16000u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&160u32.to_le_bytes());
        b.resize(204, 0);
        b
    }
    #[test]
    fn safe_paths() {
        assert!(valid_audio_path("audio/engine.wav"));
        for p in [
            "/a.wav",
            "../a.wav",
            "a/../b.wav",
            "C:/a.wav",
            "a\\b.wav",
            "https://x/a.wav",
            "a//b.wav",
            "./a.wav",
            "a.ogg",
            "a.wav#b",
        ] {
            assert!(!valid_audio_path(p), "accepted {p}");
        }
    }
    #[test]
    fn options_and_commands() {
        let p: AudioPlayOptions =
            serde_json::from_value(json!({"path":"a.wav","loop":true,"volume":0.0})).unwrap();
        assert!(p.validate() && p.looping && p.volume == 0.0);
        for extra in [
            json!({"pitch":0}),
            json!({"volume":1.1}),
            json!({"body":"x","position":[0,0,0]}),
        ] {
            let mut value = json!({"path":"a.wav"});
            value
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let p: AudioPlayOptions = serde_json::from_value(value).unwrap();
            assert!(!p.validate());
        }
        assert!(
            serde_json::from_value::<AudioPlayOptions>(json!({"path":"a.wav","typo":true}))
                .is_err()
        );
        assert!(valid_frontend_name("cellphone_place_marker") && valid_frontend_name("1up_1_user"));
        for bad in ["", "Cellphone", "a b", "../x", &"x".repeat(65)] {
            assert!(!valid_frontend_name(bad), "accepted {bad}");
        }
        for bad in [json!(-0.1), json!(1.5), json!("x")] {
            let c = serde_json::from_value::<crate::Command>(
                json!({"kind":"audio_teleport_effect","amount":bad}),
            );
            assert!(
                c.map_or(true, |c| !c.validate()),
                "accepted teleport amount {bad}"
            );
        }
        for kind in [
            "audio_preload",
            "audio_play",
            "audio_update",
            "audio_stop",
            "audio_stop_all",
            "audio_frontend",
            "audio_teleport_effect",
        ] {
            let value = match kind {
                "audio_preload" => json!({"kind":kind,"path":"a.wav"}),
                "audio_play" => json!({"kind":kind,"key":"engine","options":{"path":"a.wav"}}),
                "audio_update" => {
                    json!({"kind":kind,"key":"engine","options":{"volume":0.0,"paused":false}})
                }
                "audio_stop" => json!({"kind":kind,"key":"engine","fade_out":0.0}),
                "audio_frontend" => json!({"kind":kind,"name":"cellphone_goto_marker"}),
                "audio_teleport_effect" => json!({"kind":kind,"amount":0.5}),
                _ => json!({"kind":kind}),
            };
            let c: crate::Command = serde_json::from_value(value).unwrap();
            assert!(c.validate());
        }
        let update = AudioUpdateOptions {
            pitch: Some(f32::NAN),
            ..Default::default()
        };
        assert!(!update.validate());
    }
    /// Audio extension 3: the native routing options cross the serde boundary; native-only fields
    /// are rejected on a voice that asks for Bevy (`native = false`); native is the default
    /// (user decision 2026-10-04), and a positional native sound without a reach gets the default
    /// one (40 m, squared).
    #[test]
    fn native_play_options() {
        for ok in [
            json!({"path":"a.wav","native":true,"position":[1,2,3],"falloff":{"radius":30}}),
            json!({"path":"a.wav","native":true,"position":[1,2,3],"falloff":{"radius":12,"core":0.25,"curve":"linear"},"reverb":false,"group":"player"}),
            json!({"path":"a.wav","native":true,"spatial":false}),
            json!({"path":"a.wav","native":true,"spatial":false,"group":"world","reverb":true}),
            json!({"path":"a.wav","native":true,"position":[0,0,0]}),
            json!({"path":"a.wav","falloff":{"radius":30}}),
            json!({"path":"a.wav","reverb":true,"group":"world"}),
            json!({"path":"a.wav","native":false,"spatial_scale":0.2}),
        ] {
            let p: AudioPlayOptions = serde_json::from_value(ok.clone()).unwrap();
            assert!(p.validate(), "{ok}");
            let c: crate::Command =
                serde_json::from_value(json!({"kind":"audio_play","key":"k","options":ok}))
                    .unwrap();
            assert!(c.validate());
        }
        // The defaults: native unless `native = false`; the default reach.
        let plain: AudioPlayOptions =
            serde_json::from_value(json!({"path":"a.wav","position":[1,2,3]})).unwrap();
        assert!(plain.native.is_none() && plain.wants_native());
        assert_eq!(plain.reach(), DEFAULT_REACH);
        assert_eq!(
            (
                DEFAULT_REACH.radius,
                DEFAULT_REACH.core,
                DEFAULT_REACH.curve
            ),
            (40.0, 0.0, FalloffCurve::Squared)
        );
        let bevy: AudioPlayOptions =
            serde_json::from_value(json!({"path":"a.wav","native":false})).unwrap();
        assert!(!bevy.wants_native());
        let given: AudioPlayOptions =
            serde_json::from_value(json!({"path":"a.wav","falloff":{"radius":12,"curve":"flat"}}))
                .unwrap();
        assert_eq!(
            given.reach(),
            NativeFalloff {
                radius: 12.0,
                core: 0.0,
                curve: FalloffCurve::Flat
            }
        );
        for bad in [
            json!({"path":"a.wav","native":false,"falloff":{"radius":30}}),
            json!({"path":"a.wav","native":false,"reverb":true}),
            json!({"path":"a.wav","native":false,"group":"world"}),
            json!({"path":"a.wav","group":"music"}),
            json!({"path":"a.wav","native":true,"spatial":false,"group":"music"}),
            json!({"path":"a.wav","native":true,"falloff":{"radius":0},"position":[0,0,0]}),
            json!({"path":"a.wav","native":true,"falloff":{"radius":10,"core":1.5},"position":[0,0,0]}),
        ] {
            let p: AudioPlayOptions = serde_json::from_value(bad.clone()).unwrap();
            assert!(!p.validate(), "accepted {bad}");
        }
        for typo in [
            json!({"path":"a.wav","native":true,"falloff":{"radius":3,"shape":"box"}}),
            json!({"path":"a.wav","native":true,"spatial":false,"falloff":{"radius":3,"curve":"cubic"}}),
        ] {
            assert!(
                serde_json::from_value::<AudioPlayOptions>(typo.clone()).is_err(),
                "{typo}"
            );
        }
        assert_eq!(
            [
                FalloffCurve::Squared,
                FalloffCurve::Linear,
                FalloffCurve::Flat
            ]
            .map(FalloffCurve::retail_type),
            [0, 1, 2]
        );
    }
    #[test]
    fn pcm_round_trip_and_corruption() {
        let b = wav();
        let (out, info) = canonical_pcm_wav(&b).unwrap();
        assert_eq!(b, out);
        assert_eq!(info.channels, 1);
        assert!((info.seconds - 0.01).abs() < 1e-9);
        for n in 0..b.len() {
            assert!(canonical_pcm_wav(&b[..n]).is_err());
        }
        for (index, value) in [(20, 3), (22, 0), (32, 4), (34, 24), (40, 255)] {
            let mut bad = b.clone();
            bad[index] = value;
            assert!(canonical_pcm_wav(&bad).is_err());
        }
    }
}
