//! Splicer (`splicer` 3.00.00, 0x82a7aa90-0x82a7d468). A splice plays one sample ref from each of
//! its sample clusters; every sample gets its own RWAC voice at processing stage 0
//! (`SndPlayer1`, `Resample`, `Gain`, `Pan2D1` for mono, `Send` to the splice's SubMix). Samples
//! queue when their trigger time comes; `SpliceManager::Update` starts the queued ones once per
//! game frame, culling by priority beyond 60. The owner of a splice updates it every frame.
//!
//! The game uses the base `SpliceManager` (`AudioCore::InitializeAudioCore` 0x82a7f094): its user
//! plug-in hooks (vtable slots 1-3) are empty, so no user plug-ins are ever added.

use crate::formats::splc::{SampleRef, SplcError, SpliceBank};
use crate::math::{fmadd, fnmsub};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::decoder::DataRef;
use crate::rwac::plugins::snd_player::PlayParams;
use crate::rwac::{AudioCore, PluginRef, VoiceId, VoiceState};
use std::sync::Arc;

/// Native 30 slots plus an isolated PC mod bank for original Skate 3 catches.
pub const SPLICE_TYPES: usize = 31;
pub const SKATE3_FOOT_CATCH_BANK: usize = 30;
const MAX_QUEUED: usize = 40;
const MAX_ACTIVE: usize = 60;
/// `Splice::mSampleRefs[20]`.
const MAX_SAMPLE_REFS: usize = 20;
/// `rand() * (1 / 32768)`, the scale every caller applies (`__real_38000000`).
const RAND_SCALE: f32 = 1.0 / 32768.0;

/// `Splicer::SpliceUpdateParams`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpliceUpdateParams {
    pub vol: f32,
    pub pitch: f32,
    pub az: f32,
    pub delta: f32,
    pub spread: f32,
    pub time_stretch: f32,
}

impl Default for SpliceUpdateParams {
    /// The values `SpliceSamplePair`'s constructor 0x82a7b848 starts from.
    fn default() -> Self {
        Self { vol: 1.0, pitch: 1.0, az: 0.0, delta: 0.016, spread: 1.0, time_stretch: 1.0 }
    }
}

/// `Splicer::SplicePlayParams`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SplicePlayParams {
    pub allow_time_stretch: bool,
}

/// `Splicer::eSampleState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SampleState {
    Off = 0,
    WaitingToBeQueued = 1,
    Queued = 2,
    Playing = 3,
    Finished = 4,
    Culled = 5,
}

/// `mppPlugins[]` slots (`Splicer::PlugInIndex`).
const SND_PLAYER: usize = 0;
const RESAMPLE: usize = 1;
const TIME_STRETCH: usize = 2;
const GAIN: usize = 3;
const PAN: usize = 4;
const ROUTE: usize = 5;
const SEND: usize = 6;

/// `Splicer::SpliceSample`.
struct SpliceSample {
    voice: Option<VoiceId>,
    plugins: [Option<PluginRef>; 7],
    previous_parameters: [f32; 7],
    local_pitch: f32,
    local_random_volume: f32,
    last_pitch: f32,
    state: SampleState,
    time_to_trigger: f32,
    time_into_playback: f32,
    data: SampleRef,
    sub_mix: PluginRef,
    is_stereo: bool,
}

impl SpliceSample {
    /// The construction in `Splice::Construct` 0x82a7c2f8.
    fn new(data: SampleRef, sub_mix: PluginRef) -> Self {
        Self {
            voice: None,
            plugins: [None; 7],
            previous_parameters: [-1.0; 7],
            local_pitch: 0.0,
            local_random_volume: 0.0,
            last_pitch: 0.0,
            state: SampleState::Off,
            time_to_trigger: -1.0,
            time_into_playback: 0.0,
            data,
            sub_mix,
            is_stereo: false,
        }
    }

    /// `GetEnvelopeVolume` 0x82a7bd88.
    fn envelope_volume(&self, volume: &mut f32) {
        let data = &self.data;
        let curve_type = data.fade_curve_types & 0xF;
        if data.fade_in != 0.0 && self.time_into_playback < data.fade_in {
            let value = (self.time_into_playback - data.seek_point) / (data.fade_in - data.seek_point);
            *volume *= curve(curve_type, value);
        } else if data.fade_out != 0.0 && self.time_into_playback > data.fade_out {
            let value = 1.0 - (self.time_into_playback - data.fade_out) / (data.duration - data.fade_out);
            *volume *= curve(curve_type, value);
        }
    }
}

/// `Splicer::Splice`.
pub struct Splice {
    play_params: SplicePlayParams,
    samples: [Option<SpliceSample>; MAX_SAMPLE_REFS],
    num_sample_clusters: usize,
    volume: f32,
    pitch: f32,
    rnd_pitch: f32,
    local_pitch: f32,
}

/// A splice owned by the [`SpliceManager`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpliceId(u32);

/// A sample of a splice: the native `SpliceSample*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleId {
    splice: SpliceId,
    index: u8,
}

/// `SpliceManager::SpliceSamplePair`.
#[derive(Debug, Clone, Copy)]
struct SpliceSamplePair {
    sample: Option<SampleId>,
    priority: i8,
    update: SpliceUpdateParams,
}

impl SpliceSamplePair {
    const EMPTY: Self = Self {
        sample: None,
        priority: -1,
        update: SpliceUpdateParams { vol: 1.0, pitch: 1.0, az: 0.0, delta: 0.016, spread: 1.0, time_stretch: 1.0 },
    };
}

/// `Splicer::SpliceManager`.
pub struct SpliceManager {
    containers: Vec<Option<SpliceBank>>,
    queued: [SpliceSamplePair; MAX_QUEUED],
    active: [SpliceSamplePair; MAX_ACTIVE],
    num_queued: usize,
    num_active: usize,
    splices: Vec<Option<Splice>>,
}

impl Default for SpliceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// `Splicer::GenerateRandomIndex` 0x82a7ade8: picks one of `count` entries. Type 0 is random,
/// type 2 cycles through the halves of the set without repeats (the history holds a 16-bit mask
/// of the unused entries of the current half and the half in bit 0), any other type is
/// sequential.
fn generate_random_index(rand: &mut crate::math::CrtRand, count: u8, kind: u8, history: &mut u32) -> u8 {
    if count == 1 {
        return 0;
    }
    match kind {
        0 => {
            let r = rand.rand() as f32;
            ((r * RAND_SCALE) * f32::from(count)) as i32 as u8
        }
        2 => {
            let mask = *history >> 16;
            let half = u32::from(count) >> 1;
            let bit = *history & 1;
            let size = half + (bit & u32::from(count) & 1);
            let r = rand.rand() as f32;
            let start = ((r * RAND_SCALE) * (size + 1) as f32) as i32 as u32;
            if size == 0 {
                return 0;
            }
            let Some(slot) = (0..size).map(|i| i.wrapping_add(start) % size).find(|slot| (1 << slot) & mask != 0) else {
                return 0;
            };
            let mut remaining = mask & !(1 << slot);
            let index = (u32::from(bit != 0) * half).wrapping_add(slot);
            let mut new_bit = bit;
            if remaining == 0 {
                new_bit = u32::from(bit == 0);
                let next_size = half + (new_bit & u32::from(count) & 1);
                let full = 2f64.powi(next_size as i32) as f32 as i64 as u32;
                remaining = full.wrapping_sub(1);
            }
            *history = u32::from(new_bit != 0) | (remaining << 16);
            index as u8
        }
        _ => {
            let next = (*history as i32).wrapping_add(1).wrapping_rem(i32::from(count));
            *history = next as u32;
            next as u8
        }
    }
}

/// `XMVectorCos` 0x824b0130 (XNA Math, one lane): the angle reduced by whole turns, then the
/// 22nd-degree polynomial with the VMX code's fused multiply-adds.
fn xm_vector_cos(v: f32) -> f32 {
    const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
    const RECIPROCAL_TWO_PI: f32 = f32::from_bits(0x3e22_f983);
    const C: [u32; 11] = [
        0xbf00_0000, 0x3d2a_aaab, 0xbab6_0b61, 0x37d0_0d01, 0xb493_f27e, 0x310f_76c8, 0xad49_cba5, 0x2957_3f9f,
        0xa534_13c3, 0x20f2_a15d, 0x9c86_71cb,
    ];
    let c = |i: usize| f32::from_bits(C[i]);
    let turns = (v * RECIPROCAL_TWO_PI).round_ties_even();
    let x = fnmsub(TWO_PI, turns, v);
    let x2 = x * x;
    let x4 = x2 * x2;
    let r = fmadd(c(0), x2, 1.0);
    let x6 = x4 * x2;
    let r = fmadd(c(1), x4, r);
    let x8 = x4 * x4;
    let x10 = x6 * x4;
    let r = fmadd(c(2), x6, r);
    let x12 = x6 * x6;
    let x14 = x8 * x6;
    let x16 = x8 * x8;
    let x18 = x10 * x8;
    let r = fmadd(c(3), x8, r);
    let x20 = x10 * x10;
    let x22 = x12 * x10;
    let r = fmadd(c(4), x10, r);
    let r = fmadd(c(5), x12, r);
    let r = fmadd(c(6), x14, r);
    let r = fmadd(c(7), x16, r);
    let r = fmadd(c(8), x18, r);
    let r = fmadd(c(9), x20, r);
    fmadd(c(10), x22, r)
}

/// `Splicer::Curve::Get` 0x82a7b058 (`eCurve`): 0 one minus equal power, 1 its square, 2 linear,
/// 3 power, 4 its square. The value is clamped to [0, 1].
pub fn curve(kind: u8, value: f32) -> f32 {
    const HALF_PI: f32 = f32::from_bits(0x3fc9_0fd0);
    let v = if value >= 0.0 {
        if value <= 1.0 { value } else { 1.0 }
    } else {
        0.0
    };
    match kind {
        0 => 1.0 - xm_vector_cos(v * HALF_PI),
        1 => {
            let t = 1.0 - xm_vector_cos(v * HALF_PI);
            t * t
        }
        3 => xm_vector_cos((v - 1.0) * HALF_PI),
        4 => {
            let c = xm_vector_cos((v - 1.0) * HALF_PI);
            c * c
        }
        _ => v,
    }
}

/// Seconds since the first call while `SKATE2_TRACE_AUDIO` is set (a diagnostic log of what
/// plays, on stderr).
pub fn trace_elapsed() -> Option<f64> {
    static START: std::sync::OnceLock<Option<std::time::Instant>> = std::sync::OnceLock::new();
    START.get_or_init(|| std::env::var_os("SKATE2_TRACE_AUDIO").map(|_| std::time::Instant::now())).map(|start| start.elapsed().as_secs_f64())
}

impl SpliceManager {
    /// `SpliceManager::SpliceManager` 0x82a7be70.
    pub fn new() -> Self {
        Self {
            containers: (0..SPLICE_TYPES).map(|_| None).collect(),
            queued: [SpliceSamplePair::EMPTY; MAX_QUEUED],
            active: [SpliceSamplePair::EMPTY; MAX_ACTIVE],
            num_queued: 0,
            num_active: 0,
            splices: Vec::new(),
        }
    }

    /// `LoadSpliceBank` 0x82a7cd08 with `AddToContainer` 0x82a7bf60.
    pub fn load_bank(&mut self, bytes: Arc<[u8]>, splice_type: usize) -> Result<(), SplcError> {
        let bank = SpliceBank::parse(bytes, splice_type as i8)?;
        self.containers[splice_type] = Some(bank);
        Ok(())
    }

    /// Drops a bank whose asset was unloaded.
    pub fn unload_bank(&mut self, splice_type: usize) {
        self.containers[splice_type] = None;
    }

    pub fn bank(&self, splice_type: usize) -> Option<&SpliceBank> {
        self.containers.get(splice_type).and_then(Option::as_ref)
    }

    /// `SpliceManager::FindSplice` 0x82a7c138: indices past the splices address splice clusters,
    /// which pick one of their splices; anything beyond is the last splice.
    fn find_splice(&mut self, core: &mut AudioCore, splice_type: usize, index: i32) -> Option<usize> {
        let bank = self.containers.get_mut(splice_type)?.as_mut()?;
        let splices = bank.splices.len() as i32;
        let mut index = index;
        if index >= splices {
            if index < bank.clusters.len() as i32 + splices {
                let cluster = &mut bank.clusters[(index - splices) as usize];
                let count = cluster.splice_indices.len() as u8;
                let pick = generate_random_index(&mut core.rand, count, cluster.randomization_type, &mut cluster.random_history);
                index = i32::from(cluster.splice_indices.get(usize::from(pick)).copied().unwrap_or(0));
            } else {
                index = splices - 1;
            }
        }
        (0..splices).contains(&index).then_some(index as usize)
    }

    /// `Splice::Splice` 0x82a7cf78 with `Splice::Construct` 0x82a7c2f8: chooses one sample ref per
    /// sample cluster, each kept only if `rand()` passes its probability of playing.
    pub fn create_splice(&mut self, core: &mut AudioCore, splice_type: usize, index: i32, sub_mix: PluginRef) -> Option<SpliceId> {
        let id = self.create_splice_inner(core, splice_type, index, sub_mix);
        if let Some(elapsed) = trace_elapsed() {
            let refs: Vec<String> = id
                .and_then(|id| self.splices.get(id.0 as usize).and_then(Option::as_ref))
                .map(|splice| splice.samples.iter().flatten().map(|s| format!("{}+{:.3}s", s.data.sample_index, s.data.offset)).collect())
                .unwrap_or_default();
            eprintln!("{elapsed:9.3} audio splice bank {splice_type} index {index} samples [{}]", refs.join(" "));
        }
        id
    }

    fn create_splice_inner(&mut self, core: &mut AudioCore, splice_type: usize, index: i32, sub_mix: PluginRef) -> Option<SpliceId> {
        let splice_index = self.find_splice(core, splice_type, index)?;
        let bank = self.containers[splice_type].as_mut()?;
        let data = &mut bank.splices[splice_index];
        let mut samples: [Option<SpliceSample>; MAX_SAMPLE_REFS] = Default::default();
        let num_sample_clusters = data.clusters.len();
        // A skipped sample leaves the cluster pointer where it is: the next sample cluster
        // chooses from the same refs.
        let mut cluster = 0;
        for slot in samples.iter_mut().take(num_sample_clusters) {
            let Some(current) = data.clusters.get_mut(cluster) else {
                break;
            };
            let count = current.refs.len() as u8;
            let pick = generate_random_index(&mut core.rand, count, current.randomization_type, &mut current.randomization_field);
            let Some(sample_ref) = current.refs.get(usize::from(pick)).cloned() else {
                continue;
            };
            let roll = core.rand.rand() as f32 * RAND_SCALE;
            if roll <= sample_ref.probability_of_playing {
                *slot = Some(SpliceSample::new(sample_ref, sub_mix));
                cluster += 1;
            }
        }
        let splice = Splice {
            play_params: SplicePlayParams::default(),
            samples,
            num_sample_clusters,
            volume: data.volume,
            pitch: data.pitch,
            rnd_pitch: data.rnd_pitch,
            local_pitch: 0.0,
        };
        let slot = self.splices.iter().position(Option::is_none).unwrap_or(self.splices.len());
        if slot == self.splices.len() {
            self.splices.push(Some(splice));
        } else {
            self.splices[slot] = Some(splice);
        }
        Some(SpliceId(slot as u32))
    }

    fn splice_mut(&mut self, id: SpliceId) -> Option<&mut Splice> {
        self.splices.get_mut(id.0 as usize).and_then(Option::as_mut)
    }

    fn sample_mut(&mut self, id: SampleId) -> Option<&mut SpliceSample> {
        self.splice_mut(id.splice)?.samples.get_mut(usize::from(id.index))?.as_mut()
    }

    /// `Splice::Play` 0x82a7cfd8.
    pub fn play_splice(&mut self, core: &mut AudioCore, id: SpliceId, play: SplicePlayParams, update: &SpliceUpdateParams) {
        self.play_splice_inner(core, id, play, update, false);
    }

    /// PC mod addition: preserve a short confirmed catch's initial transient with
    /// its authored splice gain/pitch and the supplied SFX volume from playback start.
    /// Native S2 callers retain their original zero-volume Play/Update ordering.
    pub fn play_splice_with_initial_mix(&mut self, core: &mut AudioCore, id: SpliceId, play: SplicePlayParams, update: &SpliceUpdateParams) {
        self.play_splice_inner(core, id, play, update, true);
    }

    fn play_splice_inner(&mut self, core: &mut AudioCore, id: SpliceId, play: SplicePlayParams, update: &SpliceUpdateParams, initial_mix: bool) {
        let Some(splice) = self.splice_mut(id) else {
            return;
        };
        let r = core.rand.rand() as f32 * RAND_SCALE;
        splice.local_pitch = fmadd(r, splice.rnd_pitch, splice.pitch);
        splice.play_params = play;
        let mut update = *update;
        if initial_mix {
            update.vol *= splice.volume;
            update.pitch *= splice.local_pitch;
        }
        for index in 0..splice.num_sample_clusters as u8 {
            self.play_sample(core, SampleId { splice: id, index }, &update);
        }
    }

    /// `SpliceSample::Play` 0x82a7c520: the random volume, pitch and trigger time.
    fn play_sample(&mut self, core: &mut AudioCore, id: SampleId, update: &SpliceUpdateParams) {
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        sample.state = SampleState::WaitingToBeQueued;
        let r = core.rand.rand() as f32 * RAND_SCALE;
        let rnd_vol = sample.data.rnd_vol;
        let one_minus = 1.0 - rnd_vol;
        let x = fmadd(r, 2.0, -1.0);
        sample.local_random_volume = 1.0;
        sample.local_random_volume = if x <= 0.0 {
            fnmsub(-x, one_minus, 1.0)
        } else if rnd_vol == 0.0 {
            4.0
        } else {
            let range = 1.0 / (1.0 - one_minus) - 1.0;
            fmadd(range, x, 1.0)
        };
        let r = core.rand.rand() as f32 * RAND_SCALE;
        sample.local_pitch = fmadd(r, sample.data.rnd_pitch, sample.data.pitch);
        sample.last_pitch = sample.local_pitch;
        sample.time_to_trigger = if sample.data.offset == 0.0 { 0.0 } else { sample.data.offset };
        if sample.data.rnd_offset != 0.0 {
            let r = core.rand.rand() as f32 * RAND_SCALE;
            sample.time_to_trigger = fmadd(r, sample.data.rnd_offset, sample.time_to_trigger);
        }
        if sample.time_to_trigger == 0.0 {
            let priority = sample.data.priority as i8;
            self.add_queued_sample(core, SpliceSamplePair { sample: Some(id), priority, update: *update });
        }
    }

    /// `SpliceManager::AddQueuedSample` 0x82a7c218.
    fn add_queued_sample(&mut self, core: &mut AudioCore, pair: SpliceSamplePair) -> bool {
        let Some(id) = pair.sample else {
            return false;
        };
        if self.num_queued < MAX_QUEUED {
            self.queued[self.num_queued] = pair;
            self.num_queued += 1;
            if let Some(sample) = self.sample_mut(id) {
                sample.state = SampleState::Queued;
            }
            true
        } else {
            if let Some(sample) = self.sample_mut(id) {
                sample.state = SampleState::Culled;
            }
            self.begin_destroy(core, id);
            false
        }
    }

    /// `SpliceManager::RemoveEntry` 0x82a7b5c0.
    fn remove_entry(&mut self, id: SampleId) {
        if let Some(slot) = self.active.iter_mut().find(|pair| pair.sample == Some(id)) {
            *slot = SpliceSamplePair { sample: None, priority: -1, update: slot.update };
            self.num_active -= 1;
            return;
        }
        if let Some(position) = self.queued[..self.num_queued].iter().position(|pair| pair.sample == Some(id)) {
            self.queued[position].sample = None;
            self.queued[position].priority = -1;
            self.queued.copy_within(position + 1..self.num_queued, position);
            self.num_queued -= 1;
        }
    }

    /// `SpliceSample::ReleaseRWACVoice` 0x82a7ad50.
    fn release_voice(&mut self, core: &mut AudioCore, id: SampleId) {
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        if let Some(voice) = sample.voice.take() {
            core.release_voice(voice);
            sample.plugins = [None; 7];
        }
    }

    /// `SpliceSample::BeginDestory` 0x82a7b960.
    fn begin_destroy(&mut self, core: &mut AudioCore, id: SampleId) {
        let Some(state) = self.sample_mut(id).map(|s| s.state) else {
            return;
        };
        let finished = match state {
            SampleState::Off | SampleState::WaitingToBeQueued => true,
            SampleState::Queued => {
                self.remove_entry(id);
                true
            }
            SampleState::Playing => {
                self.remove_entry(id);
                self.release_voice(core, id);
                true
            }
            SampleState::Culled => {
                self.release_voice(core, id);
                false
            }
            SampleState::Finished => false,
        };
        if finished {
            if let Some(sample) = self.sample_mut(id) {
                sample.state = SampleState::Finished;
            }
        }
    }

    /// `SpliceSample::CreateVoiceInstance` 0x82a7ba80.
    fn create_voice_instance(&mut self, core: &mut AudioCore, id: SampleId, play: SplicePlayParams) {
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        let stereo = sample.data.az == -127.0;
        sample.is_stereo = stereo;
        let channels = if stereo { 2 } else { 1 };
        let mut map = [None::<u8>; 7];
        map[SND_PLAYER] = Some(0);
        let mut configs = vec![PlugInConfig { kind: PlugInKind::SndPlayer1 { max_requests: 1.0 }, output_channels: channels }];
        if sample.data.lfe_only {
            configs.push(PlugInConfig { kind: PlugInKind::Gain, output_channels: channels });
            configs.push(PlugInConfig { kind: PlugInKind::Route, output_channels: 1 });
            map[GAIN] = Some(1);
            map[ROUTE] = Some(2);
        } else {
            configs.push(PlugInConfig { kind: PlugInKind::Resample, output_channels: channels });
            map[RESAMPLE] = Some(1);
            if play.allow_time_stretch || sample.data.time_stretch != 1.0 {
                // `TimeStretch` ('TSt0') is not ported; no bank sample or caller needs it.
                map[TIME_STRETCH] = None;
            }
            map[GAIN] = Some(configs.len() as u8);
            configs.push(PlugInConfig { kind: PlugInKind::Gain, output_channels: channels });
            if !stereo {
                map[PAN] = Some(configs.len() as u8);
                configs.push(PlugInConfig { kind: PlugInKind::Pan2D1 { constructor: None }, output_channels: 6 });
            }
            map[SEND] = Some(configs.len() as u8);
            configs.push(PlugInConfig { kind: PlugInKind::Send, output_channels: if stereo { 2 } else { 6 } });
        }
        let voice = core.create_voice(0, "Splice", &configs);
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        sample.voice = voice;
        for (plugin, index) in sample.plugins.iter_mut().zip(map) {
            *plugin = voice.zip(index).map(|(voice, index)| PluginRef { voice, index });
        }
    }

    fn sample_data(&self, sample: &SampleRef) -> Option<(DataRef, DataRef)> {
        let bank = self.containers.get(sample.splice_type as usize)?.as_ref()?;
        let toc = bank.toc.get(usize::from(sample.sample_index))?;
        let base = DataRef { bytes: Arc::clone(&bank.bytes), offset: bank.sample_base };
        Some((base.at(toc.sample_data as usize), base.at(toc.seek_table as usize)))
    }

    /// `SpliceSample::StartVoice` 0x82a7c6e0.
    fn start_voice(&mut self, core: &mut AudioCore, id: SampleId, play: SplicePlayParams, update: &SpliceUpdateParams) {
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        sample.time_into_playback = sample.data.seek_point;
        sample.state = SampleState::Playing;
        self.create_voice_instance(core, id, play);
        let Some(data) = self.sample_mut(id).map(|s| s.data.clone()) else {
            return;
        };
        let (ram_sample, seek_table) = self.sample_data(&data).unzip();
        let mut params = PlayParams { ram_sample, expel_mode: 1.0, ..PlayParams::default() };
        if data.seek_point != 0.0 {
            params.seek_time = f64::from((data.pitch / data.time_stretch) * data.seek_point);
            params.seek_table = seek_table;
        }
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        let plugins = sample.plugins;
        if let Some(player) = plugins[SND_PLAYER] {
            core.play(player, params);
        }
        if let Some(resample) = plugins[RESAMPLE] {
            let pitch = sample.local_pitch * update.pitch;
            core.set_attribute(resample, 0, pitch);
            sample.previous_parameters[RESAMPLE] = pitch;
        }
        if let Some(send) = plugins[SEND] {
            core.connect_send(send, Some(sample.sub_mix));
            core.set_attribute(send, 0, 1.0);
            sample.previous_parameters[SEND] = 1.0;
        }
        if let Some(route) = plugins[ROUTE] {
            core.connect_route(route, Some(sample.sub_mix), 0.0, 5.0, 1.0);
        }
        let mut volume = (sample.local_random_volume * sample.data.volume) * update.vol;
        sample.envelope_volume(&mut volume);
        if let Some(gain) = plugins[GAIN] {
            core.set_attribute(gain, 0, volume);
        }
        sample.previous_parameters[GAIN] = volume;
        if sample.data.splice_type == 16 && let Some(elapsed) = trace_elapsed() {
            eprintln!("{elapsed:9.3} foot catch start sample={} voice={:?} gain={volume} input={} pitch={} target={:?} priority={}",
                sample.data.sample_index, sample.voice, update.vol, update.pitch, sample.sub_mix, sample.data.priority);
        }
        if let Some(pan) = plugins[PAN] {
            let az = fmadd(update.spread, sample.data.az, update.az);
            core.set_attribute(pan, 0, az);
            sample.previous_parameters[PAN] = az;
        }
        if let Some(stretch) = plugins[TIME_STRETCH] {
            let value = (update.time_stretch * sample.data.time_stretch).clamp(0.5, 2.0);
            core.set_attribute(stretch, 0, value);
            sample.previous_parameters[TIME_STRETCH] = value;
        }
    }

    /// `SpliceSample::Update` 0x82a7ca00.
    fn update_sample(&mut self, core: &mut AudioCore, id: SampleId, input: &SpliceUpdateParams) {
        let Some(sample) = self.sample_mut(id) else {
            return;
        };
        let mut params = *input;
        if sample.time_to_trigger > 0.0 || sample.time_to_trigger.is_nan() {
            sample.time_to_trigger -= params.delta;
            if sample.time_to_trigger <= 0.0 {
                let priority = sample.data.priority as i8;
                self.add_queued_sample(core, SpliceSamplePair { sample: Some(id), priority, update: *input });
            }
            return;
        }
        if sample.state != SampleState::Playing {
            return;
        }
        let pitch = sample.local_pitch * params.pitch;
        let last_pitch = sample.last_pitch;
        sample.last_pitch = params.pitch;
        sample.time_into_playback = fmadd(last_pitch, params.delta, sample.time_into_playback);
        let time_stretch = sample.data.time_stretch * params.time_stretch;
        params.vol *= sample.data.volume * sample.local_random_volume;
        params.az = fmadd(sample.data.az, params.spread, params.az);
        params.pitch = pitch;
        params.time_stretch = time_stretch;
        sample.envelope_volume(&mut params.vol);
        let length = (sample.data.pitch / sample.local_pitch) * sample.data.duration;
        let expelled = sample.voice.and_then(|voice| core.voice_state(voice)) == Some(VoiceState::Expelled);
        let plugins = sample.plugins;
        if length + 0.16 < sample.time_into_playback || expelled {
            if let Some(resample) = plugins[RESAMPLE] {
                core.set_attribute(resample, 0, 0.0);
            }
            if let Some(gain) = plugins[GAIN] {
                core.set_attribute(gain, 0, 0.0);
            }
            if let Some(player) = plugins[SND_PLAYER] {
                core.stop(player);
            }
            self.begin_destroy(core, id);
            return;
        }
        if let Some(pan) = plugins[PAN] {
            if sample.previous_parameters[PAN] != params.az {
                sample.previous_parameters[PAN] = params.az;
                core.set_attribute(pan, 0, params.az);
            }
        }
        if let Some(stretch) = plugins[TIME_STRETCH] {
            let value = params.time_stretch.clamp(0.5, 2.0);
            core.set_attribute(stretch, 0, value);
            sample.previous_parameters[TIME_STRETCH] = value;
        }
        if let Some(resample) = plugins[RESAMPLE] {
            if sample.previous_parameters[RESAMPLE] != params.pitch {
                sample.previous_parameters[RESAMPLE] = params.pitch;
                core.set_attribute(resample, 0, params.pitch);
            }
        }
        if sample.previous_parameters[GAIN] != params.vol {
            sample.previous_parameters[GAIN] = params.vol;
            if let Some(gain) = plugins[GAIN] {
                core.set_attribute(gain, 0, params.vol);
            }
        }
    }

    /// `Splice::Update` 0x82a7d088: applies the splice's volume and pitch to `update` (the
    /// caller's parameters, as natively) and updates the samples; finished samples are deleted.
    pub fn update_splice(&mut self, core: &mut AudioCore, id: SpliceId, update: &mut SpliceUpdateParams) {
        let Some(splice) = self.splice_mut(id) else {
            return;
        };
        update.vol *= splice.volume;
        update.pitch *= splice.local_pitch;
        for index in 0..splice.num_sample_clusters as u8 {
            let sample = SampleId { splice: id, index };
            if self.sample_mut(sample).is_none() {
                continue;
            }
            self.update_sample(core, sample, update);
            if self.sample_mut(sample).is_some_and(|s| s.state >= SampleState::Finished) {
                self.begin_destroy(core, sample);
                if let Some(splice) = self.splice_mut(id) {
                    splice.samples[usize::from(index)] = None;
                }
            }
        }
    }

    /// `Splice::IsPlaying` 0x82a7b8f8.
    pub fn is_playing(&self, id: SpliceId) -> bool {
        self.splices
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .is_some_and(|splice| splice.samples.iter().take(splice.num_sample_clusters).flatten().any(|s| s.state < SampleState::Finished))
    }

    /// `Splice::~Splice` 0x82a7ac68: destroys the remaining samples.
    pub fn release_splice(&mut self, core: &mut AudioCore, id: SpliceId) {
        let Some(count) = self.splice_mut(id).map(|s| s.num_sample_clusters) else {
            return;
        };
        for index in 0..count as u8 {
            let sample = SampleId { splice: id, index };
            if self.sample_mut(sample).is_some() {
                self.begin_destroy(core, sample);
            }
        }
        self.splices[id.0 as usize] = None;
    }

    /// `SpliceManager::AddToActivelyPlaying` 0x82a7ce68: starts the sample's voice and takes the
    /// first free active slot from `start`.
    fn add_to_actively_playing(&mut self, core: &mut AudioCore, pair: SpliceSamplePair, start: usize) {
        let Some(id) = pair.sample else {
            return;
        };
        let play = self.splice_mut(id.splice).map(|s| s.play_params).unwrap_or_default();
        self.start_voice(core, id, play, &pair.update);
        if let Some(slot) = self.active.iter_mut().skip(start).find(|slot| slot.priority as u8 == 255) {
            *slot = pair;
            self.num_active += 1;
        }
    }

    /// `SpliceManager::Partition` 0x82a7aa90: moves entries of higher priority than the pivot
    /// (signed compare) to the front.
    fn partition(array: &mut [SpliceSamplePair], left: usize, right: usize, pivot: usize) -> usize {
        let pivot_priority = array[pivot].priority;
        array.swap(pivot, right);
        let mut store = left;
        for i in left..=right {
            if array[i].priority > pivot_priority {
                array.swap(i, store);
                store += 1;
            }
        }
        array.swap(store, right);
        store
    }

    /// `SpliceManager::QuickSort` 0x82a7b568: descending by priority.
    fn quick_sort(array: &mut [SpliceSamplePair], left: isize, right: isize) {
        let mut left = left;
        while right > left {
            let pivot = Self::partition(array, left as usize, right as usize, left as usize) as isize;
            Self::quick_sort(array, left, pivot - 1);
            left = pivot + 1;
        }
    }

    fn cull(&mut self, core: &mut AudioCore, pair: SpliceSamplePair) {
        if let Some(id) = pair.sample {
            if let Some(sample) = self.sample_mut(id) {
                if sample.data.splice_type == 16 && let Some(elapsed) = trace_elapsed() {
                    eprintln!("{elapsed:9.3} foot catch culled sample={} priority={}", sample.data.sample_index, pair.priority);
                }
                sample.state = SampleState::Culled;
            }
            self.begin_destroy(core, id);
        }
    }

    /// `SpliceManager::Update` 0x82a7d430 = `ProcessQueued` 0x82a7d1d8: starts the queued samples;
    /// beyond 60 playing, the queue and the active table are sorted by priority and the lowest
    /// are culled.
    pub fn update(&mut self, core: &mut AudioCore) {
        let queued = self.num_queued;
        if queued == 0 {
            return;
        }
        if queued + self.num_active <= MAX_ACTIVE {
            for i in 0..self.num_queued {
                let pair = self.queued[i];
                self.add_to_actively_playing(core, pair, 0);
                self.queued[i].sample = None;
                self.queued[i].priority = -1;
            }
            self.num_queued = 0;
            return;
        }
        Self::quick_sort(&mut self.queued, 0, queued as isize - 1);
        let free = MAX_ACTIVE - self.num_active;
        let mut next = 0;
        for i in 0..free {
            let pair = self.queued[i];
            self.add_to_actively_playing(core, pair, 0);
            self.queued[i].sample = None;
            self.queued[i].priority = -1;
            next = i + 1;
        }
        Self::quick_sort(&mut self.active, 0, MAX_ACTIVE as isize - 1);
        let mut last = MAX_ACTIVE - 1;
        while next != self.num_queued {
            if last == 0 || self.active[last].priority >= self.queued[next].priority {
                break;
            }
            let victim = self.active[last];
            self.cull(core, victim);
            self.active[last] = SpliceSamplePair { sample: None, priority: -1, update: victim.update };
            self.num_active -= 1;
            let pair = self.queued[next];
            self.add_to_actively_playing(core, pair, last);
            self.queued[next].sample = None;
            self.queued[next].priority = -1;
            next += 1;
            last -= 1;
        }
        for i in next..self.num_queued {
            let pair = self.queued[i];
            self.cull(core, pair);
            self.queued[i].sample = None;
            self.queued[i].priority = -1;
        }
        self.num_queued = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::CrtRand;

    #[test]
    fn history_selection_uses_each_half_without_repeats() {
        let mut rand = CrtRand::default();
        let mut history = 0x1_0000;
        let picks: Vec<u8> = (0..9).map(|_| generate_random_index(&mut rand, 3, 2, &mut history)).collect();
        assert_eq!(picks[0], 0);
        let mut second: Vec<u8> = picks[1..3].to_vec();
        second.sort_unstable();
        assert_eq!(second, [1, 2]);
        assert_eq!(picks[3], 0);
    }

    #[test]
    fn sequential_selection_counts_from_the_history() {
        let mut rand = CrtRand::default();
        let mut history = 0x1_0000;
        let picks: Vec<u8> = (0..4).map(|_| generate_random_index(&mut rand, 2, 1, &mut history)).collect();
        assert_eq!(picks, [1, 0, 1, 0]);
    }

    #[test]
    fn curves_span_zero_to_one() {
        // SPLICER_HALF_PI is 1.5707951, a little short of pi/2, so the ends miss by up to 2.5e-6.
        for kind in 0..5 {
            assert!(curve(kind, 0.0).abs() < 3e-6, "{kind}");
            assert!((curve(kind, 1.0) - 1.0).abs() < 3e-6, "{kind}");
        }
        assert_eq!(curve(2, 0.25), 0.25);
        assert_eq!(curve(2, 2.0), 1.0);
        assert_eq!(curve(2, -1.0), 0.0);
        let c = xm_vector_cos(0.5);
        assert!((c - 0.5f32.cos()).abs() < 1e-6);
    }
}
