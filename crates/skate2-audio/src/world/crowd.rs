//! The CROWD state (`eMAINMAPSTATES` 7): `CSTATEMGR_Crowd` (constructor 0x82554120) and its five
//! `CSTATE_Crowd`s (`EnterGamePlay` 0x82550c60, `Setup(1)`), each with an `SFXObj_Crowd`
//! (`CreateObject` 0x82532298) and its `SFXCTL_3DCrowdPos` (an `SFXCTL_3DObjPos` on the state's
//! position), whose `C_crowd_ambience` and `C_crowd_behaviour` (`SK8_AEMS_Crowds.csi`) play the
//! crowd banks of `data\audio\crowds.big`.
//!
//! A contest's countdown (`AudioSystem::TriggerMissionMusicImp`) or a loaded arena
//! (`AudioSystem::Update`) starts the crowds (`StartCrowds` 0x82557810): every free state attaches
//! as a pod of three at the local skater, or the one arena pod. Scores and wipeouts
//! (`SFXObj_Contacts::CheckForScore`, and the countdown's opening cheer) reach `PlayCrowdSound`
//! 0x825573b0, which moves the pods onto the crowd volumes of `crowds.ems` and then the nearest
//! spectator groups before each pod cheers or gasps. `StopCrowds` 0x82554210 ends them.

use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::presentation::SkaterData;
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};
use crate::world::emitter_system::{self, Sk8EmitterSystem, VOLUME_CROWD};

/// `eMAINMAPSTATES` 7 and the states `EnterGamePlay` makes.
const STATE: u32 = 7;
const STATES: u32 = 5;

/// `eCrowdType`.
pub const NO_CROWD: i32 = 0;
pub const SMALL_CROWD: i32 = 1;
pub const ARENA_CROWD: i32 = 2;

/// `Attrib::Gen::aud_crowds` and `Attrib::Gen::challenges`.
pub const AUD_CROWDS: u64 = 0xF353_66A6_1ECC_673D;
pub const CHALLENGES: u64 = 0x8876_E0B5_5674_0E36;
/// `challenges::CrowdTuning` 0x824e2e10: the challenge's `aud_crowds` collection.
pub const CROWD_TUNING: u64 = 0xBCC4_2BA1_1286_8865;
/// `aud_crowds` layout words: `npc_handicap`, `minimum_cheer_threshold`, `maximum_ohh_score`,
/// `maximum_cheer_score`.
pub const NPC_HANDICAP: u32 = 3;
pub const MINIMUM_CHEER_THRESHOLD: u32 = 8;
pub const MAXIMUM_OHH_SCORE: u32 = 9;
pub const MAXIMUM_CHEER_SCORE: u32 = 10;
/// The `aud_crowds` attributes `TuneFromAttribulator` reads: `master_volume`, `wet_volume`,
/// `dry_volume`, `murmur_volume` and `cheer_volume`, which every reaction volume reads.
const MASTER_VOLUME: u64 = 0x05B6_A504_C779_692C;
const WET_VOLUME: u64 = 0xE634_95A2_9383_8642;
const DRY_VOLUME: u64 = 0xC4D7_0B8F_313D_7F8D;
const MURMUR_VOLUME: u64 = 0x35DC_566D_C649_D77E;
const CHEER_VOLUME: u64 = 0x68CC_0EBF_C002_4609;
/// `aud_crowds::crowd_type` (`AudioSystem::GetCrowdType` 0x824f2398).
pub const CROWD_TYPE: u64 = 0x7A62_C24F_8EE3_8B07;
/// `Sk8::Audio::kIgnoreNicotine` ("Ignore Nicotine Levels") at its registered default.
const IGNORE_NICOTINE: bool = false;
const Q15: f32 = f32::from_bits(0x46ff_fe00);

/// `Csis::C_crowd_ambience` members (13 words).
mod ambience {
    pub const VOLUME: usize = 0;
    pub const AZIMUTH: usize = 1;
    pub const PITCH: usize = 2;
    pub const DRY: usize = 3;
    pub const WET: usize = 4;
    pub const LOPASS: usize = 5;
    pub const HIPASS: usize = 6;
    pub const CROWD_AMBIENCE_VOLUME: usize = 8;
    pub const INTENSITY: usize = 9;
    pub const OHH: usize = 11;
    pub const CHEER: usize = 12;
    /// The constructor 0x8250f6b8's clamps.
    pub const LIMITS: [(i32, i32); 13] =
        [(0, 0x7FFF), (0, 0xFFFF), (0, 0x2000), (0, 0x7FFF), (0, 0x7FFF), (0, 25000), (0, 25000), (0, 1000), (0, 0x7FFF), (0, 1000), (1, 7), (0, 1), (0, 1)];
}

/// `Csis::C_crowd_behaviour` members (27 words).
mod behaviour {
    pub const VOLUME: usize = 0;
    pub const AZIMUTH: usize = 1;
    pub const PITCH: usize = 2;
    pub const DRY: usize = 3;
    pub const WET: usize = 4;
    pub const LOPASS: usize = 5;
    pub const HIPASS: usize = 6;
    pub const INTENSITY: usize = 8;
    pub const CHEER: usize = 9;
    pub const OHH: usize = 10;
    pub const PC_CHEER_VOLUME: usize = 11;
    pub const PC_OHH_VOLUME: usize = 12;
    pub const NPC_CHEER_VOLUME: usize = 13;
    pub const NPC_OHH_VOLUME: usize = 14;
    pub const SKATER: usize = 15;
    pub const APPLAUSE_VOLUME: usize = 19;
    pub const NEGATIVE_REAX: usize = 20;
    pub const DISTANCE: usize = 23;
    pub const CROWD_GROUP_SIZE: usize = 24;
    pub const SMALL_SAMPLE_COUNT: usize = 25;
    pub const LARGE_SAMPLE_COUNT: usize = 26;
    /// The constructor 0x8250f8f0's clamps.
    pub const LIMITS: [(i32, i32); 27] = [
        (0, 0x7FFF),
        (0, 0xFFFF),
        (0, 0x2000),
        (0, 0x7FFF),
        (0, 0x7FFF),
        (0, 25000),
        (0, 25000),
        (0, 1000),
        (0, 1000),
        (0, 1),
        (0, 1),
        (0, 0x7FFF),
        (0, 0x7FFF),
        (0, 0x7FFF),
        (0, 0x7FFF),
        (0, 1),
        (0, 1),
        (0, 1),
        (0, 0x7FFF),
        (0, 0x7FFF),
        (0, 1),
        (1, 7),
        (0, 0x7FFF),
        (0, 1000),
        (0, 5),
        (0, 10),
        (0, 10),
    ];
}

/// The Csis constructors' member data: each argument clamped to its range.
fn clamped(values: &[i32], limits: &[(i32, i32)]) -> Vec<i32> {
    values.iter().zip(limits).map(|(&value, &(low, high))| value.clamp(low, high)).collect()
}

/// `Sk8::Audio::CrowdPodInfo`: a spectator group's centroid and the spectators standing at it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CrowdPodInfo {
    pub pod_pos: Vec3,
    pub num_spectators: i32,
}

/// What `CSTATEMGR_Crowd::PlayCrowdSound` reads besides its own state: the skater's
/// `SkaterData::skaterPos`, the first five crowd volumes `EmitterSystem::GetVolumes` reports (their
/// position and `aud_emitter::crowd_aems_id`; `None` past the last) and the nearest pods
/// `AudioSystem::GetNearestCrowdPodInfo` chose (at most five).
#[derive(Debug, Clone, Default)]
pub struct CrowdSoundInputs {
    pub skater_pos: Vec3,
    pub volumes: [Option<(Vec3, i32)>; 5],
    pub pods: Vec<CrowdPodInfo>,
}

/// What `AudioSystem::PlayCrowdSound` reads besides the crowd manager: the presentation's
/// skaters, the emitter system's crowd volumes, `mpCrowdPodInfos` and `mCrowdBanksLoaded`.
#[derive(Clone, Copy)]
pub struct CrowdEnv<'a> {
    pub skaters: &'a [SkaterData],
    pub emitter_system: &'a Sk8EmitterSystem,
    pub pods: &'a [CrowdPodInfo; 30],
    pub banks_loaded: &'a [bool; 9],
}

impl CrowdEnv<'_> {
    /// `PlayCrowdSound`'s reads for a skater: `AudioPresentation::GetSkater(skaterID)`'s
    /// position, `EmitterSystem::GetVolumes(eVolumeType_Crowd, 5)` and
    /// `GetNearestCrowdPodInfo(skaterPos, 5)`.
    #[must_use]
    pub fn inputs(&self, skater_id: i32, vault: &Vault) -> CrowdSoundInputs {
        let skater_pos = usize::try_from(skater_id).ok().and_then(|index| self.skaters.get(index)).map_or(ZERO, |skater| skater.skater_pos);
        let mut volumes = [None; 5];
        for (slot, id) in volumes.iter_mut().zip(self.emitter_system.volumes(VOLUME_CROWD, 5, vault)) {
            if let Some(view) = self.emitter_system.volume_status(id) {
                *slot = Some(([view.pos[0], view.pos[1], view.pos[2], 0.0], emitter_system::crowd_aems_id(view.key, vault)));
            }
        }
        CrowdSoundInputs { skater_pos, volumes, pods: nearest_crowd_pods(self.pods, skater_pos, 5) }
    }
}

/// `SFXObj_Crowd` (0xAC bytes).
struct CrowdObject {
    /// `m_pDMixIO` (objectID 0x40070000 + the state's instance).
    io: Option<DMixIoId>,
    enable_ambience: bool,
    behaviour: Option<ClassObject>,
    ambience: Option<ClassObject>,
    /// `mAttribCrowds`: its `aud_crowds` collection (0: `DefaultDataArea`).
    attrib_crowds: u64,
    skater_id: i32,
    pod_id: i32,
    crowd_ambience_volume: i32,
    common_volume: i32,
    common_pitch: i32,
    common_dry: i32,
    common_wet: i32,
    common_lopass: i32,
    common_hipass: i32,
    speed: i32,
    pc_cheer_volume: i32,
    pc_ohh_volume: i32,
    npc_cheer_volume: i32,
    npc_ohh_volume: i32,
    skater: i32,
    anticipation_start: i32,
    anticipation_stop: i32,
    anticipation_volume: i32,
    applause_volume: i32,
    negative_reax: i32,
    venue_size: i32,
    neg_reax_volume: i32,
    distance: i32,
    arena_volume_mult: f32,
    was_cheer: bool,
    was_ohh: bool,
    cheer_intensity: i32,
}

impl CrowdObject {
    /// The constructor 0x82517238.
    fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            enable_ambience: true,
            behaviour: None,
            ambience: None,
            attrib_crowds: 0,
            skater_id: 0,
            pod_id: 0,
            crowd_ambience_volume: 0,
            common_volume: 0,
            common_pitch: 4096,
            common_dry: 0,
            common_wet: 0,
            common_lopass: 24000,
            common_hipass: 0,
            speed: 0,
            pc_cheer_volume: 0,
            pc_ohh_volume: 0,
            npc_cheer_volume: 0,
            npc_ohh_volume: 0,
            skater: 0,
            anticipation_start: 0,
            anticipation_stop: 0,
            anticipation_volume: 0,
            applause_volume: 0,
            negative_reax: 0,
            venue_size: 7,
            neg_reax_volume: 0,
            distance: 0,
            arena_volume_mult: 1.0,
            was_cheer: false,
            was_ohh: false,
            cheer_intensity: 0,
        }
    }

    /// `TuneFromAttribulator` 0x8253d8c0: an `aud_crowds` attribute the collection has, scaled to
    /// `max_range` and truncated; else `int_val`.
    fn tune(&self, key: u64, max_range: f32, int_val: i32, vault: &Vault) -> i32 {
        vault.find_f32(Collection::new(AUD_CROWDS, self.attrib_crowds), key).map_or(int_val, |value| fctiwz(value * max_range))
    }

    /// `GetScaledVolume` 0x8250fd20: mixer volume output 2, or for a small crowd 3 (1 below fifty).
    fn scaled_volume(&self, state: &StateData, unscaled: i32, ctx: &SfxCtx<'_>) -> i32 {
        let output = match (state.crowd_type, state.crowd_size) {
            (SMALL_CROWD, size) if size < 50 => 1,
            (SMALL_CROWD, _) => 3,
            _ => 2,
        };
        let mixer = ctx.output(self.io, output, PresetType::Volume) as f32;
        fctiwz(unscaled as f32 * (mixer * f32::from_bits(0x3800_0100)))
    }

    /// `GetScaledPitch` 0x8250fdd0: mixer pitch output 4 over 4096.
    fn scaled_pitch(&self, unscaled: i32, ctx: &SfxCtx<'_>) -> i32 {
        let mixer = ctx.output(self.io, 4, PresetType::Pitch) as f32;
        fctiwz(unscaled as f32 * (mixer * f32::from_bits(0x3980_0000)))
    }

    /// `GetCrowdGroupSize` 0x82532350: none below four, the small bank's group below eight, the
    /// crowd volume's AEMS group from fifty, else the ten bank's group if loaded, the five
    /// bank's otherwise.
    fn crowd_group_size(state: &StateData, banks_loaded: &[bool; 9]) -> i32 {
        let size = state.crowd_size;
        if size < 4 {
            0
        } else if size >= 8 {
            if size >= 50 {
                state.crowd_aems_id
            } else if banks_loaded[2] {
                2
            } else {
                i32::from(banks_loaded[1])
            }
        } else {
            i32::from(banks_loaded[1])
        }
    }

    /// `StartCrowd` 0x82532428 (from `InitSFX` 0x82547170): for an attached state with a crowd, the
    /// challenge's `aud_crowds`, the venue size (seven for the arena, whose ambience fades in),
    /// and the ambience (none for a small crowd) and behaviour instances.
    fn start_crowd(&mut self, state: &mut StateData, banks_loaded: &[bool; 9], ctx: &mut SfxCtx<'_>) {
        if !state.attached || state.crowd_type < SMALL_CROWD {
            return;
        }
        if state.crowd_type == SMALL_CROWD {
            self.enable_ambience = false;
        }
        self.attrib_crowds = state.crowd_tuning_key(ctx.vault);
        let group = Self::crowd_group_size(state, banks_loaded);
        self.venue_size = 1;
        if state.crowd_type == ARENA_CROWD {
            self.venue_size = 7;
            self.arena_volume_mult = 0.0;
        }
        if self.ambience.is_none() && self.enable_ambience {
            let azimuth = ctx.output(self.io, 0, PresetType::Azimuth);
            let pitch = self.scaled_pitch(self.common_pitch, ctx);
            let values = [
                0,
                azimuth,
                pitch,
                self.common_dry,
                self.common_wet,
                self.common_lopass,
                self.common_hipass,
                self.speed,
                self.crowd_ambience_volume,
                0,
                self.venue_size,
                0,
                0,
            ];
            self.ambience = Some(ctx.aems.create_object("c_crowd_ambience", clamped(&values, &ambience::LIMITS)));
        }
        if self.behaviour.is_none() {
            let azimuth = ctx.output(self.io, 0, PresetType::Azimuth);
            let pitch = self.scaled_pitch(self.common_pitch, ctx);
            let values = [
                0,
                azimuth,
                pitch,
                self.common_dry,
                self.common_wet,
                self.common_lopass,
                self.common_hipass,
                self.speed,
                0,
                0,
                0,
                self.pc_cheer_volume,
                self.pc_ohh_volume,
                self.npc_cheer_volume,
                self.npc_ohh_volume,
                self.skater,
                self.anticipation_start,
                self.anticipation_stop,
                self.anticipation_volume,
                self.applause_volume,
                self.negative_reax,
                self.venue_size,
                self.neg_reax_volume,
                self.distance,
                group,
                0,
                0,
            ];
            self.behaviour = Some(ctx.aems.create_object("c_crowd_behaviour", clamped(&values, &behaviour::LIMITS)));
        }
    }

    /// `StopCrowd` 0x825326a0 (also `Destroy` and `Detach`): the behaviour, then the ambience, are
    /// released, and the state's challenge is cleared.
    fn stop_crowd(&mut self, state: &mut StateData, ctx: &mut SfxCtx<'_>) {
        if let Some(object) = self.behaviour.take() {
            ctx.aems.release_object(object);
        }
        if let Some(object) = self.ambience.take() {
            ctx.aems.release_object(object);
        }
        state.challenge = 0;
    }

    /// `PlayCheer` 0x82532720: the group size, the intensity, a cheer and the manager's next small
    /// and large sample counters.
    fn play_cheer(&mut self, state: &StateData, scaled_score: i32, banks_loaded: &[bool; 9], counters: &mut Counters, ctx: &mut SfxCtx<'_>) {
        let Some(object) = self.behaviour.as_mut() else { return };
        object.data[behaviour::CROWD_GROUP_SIZE] = Self::crowd_group_size(state, banks_loaded).clamp(0, 5);
        object.data[behaviour::INTENSITY] = scaled_score.clamp(0, 1000);
        object.data[behaviour::CHEER] = 1;
        object.data[behaviour::SMALL_SAMPLE_COUNT] = counters.small().clamp(0, 10);
        object.data[behaviour::LARGE_SAMPLE_COUNT] = counters.large().clamp(0, 10);
        ctx.aems.set_member_data(object);
    }

    /// `PlayOhh` 0x82532860: the group size, a gasp and the next sample counters.
    fn play_ohh(&mut self, state: &StateData, banks_loaded: &[bool; 9], counters: &mut Counters, ctx: &mut SfxCtx<'_>) {
        let Some(object) = self.behaviour.as_mut() else { return };
        object.data[behaviour::CROWD_GROUP_SIZE] = Self::crowd_group_size(state, banks_loaded).clamp(0, 5);
        object.data[behaviour::OHH] = 1;
        object.data[behaviour::SMALL_SAMPLE_COUNT] = counters.small().clamp(0, 10);
        object.data[behaviour::LARGE_SAMPLE_COUNT] = counters.large().clamp(0, 10);
        ctx.aems.set_member_data(object);
    }

    /// `PlayCrowdSound` 0x825471e8: a wipeout gasps; otherwise a score cheers at the scaled
    /// intensity the ambience then swells to.
    #[allow(clippy::too_many_arguments)]
    fn play_crowd_sound(&mut self, state: &StateData, skater_id: i32, score: i32, wiped_out_score: i32, scaled_score: i32, banks_loaded: &[bool; 9], counters: &mut Counters, ctx: &mut SfxCtx<'_>) {
        self.skater_id = skater_id;
        if wiped_out_score <= 0 {
            if score > 0 {
                self.play_cheer(state, scaled_score, banks_loaded, counters, ctx);
                self.cheer_intensity = scaled_score;
                self.was_cheer = true;
            }
        } else {
            self.play_ohh(state, banks_loaded, counters, ctx);
            self.was_ohh = true;
        }
    }

    /// `HandleBehaviour` 0x82547608: the tuned volumes through the mixer, the pod's pitch step, the
    /// player character's or another skater's reaction volumes, the distance; the one-shot
    /// reactions are cleared after the update.
    fn handle_behaviour(&mut self, state: &StateData, ctx: &mut SfxCtx<'_>) {
        if self.behaviour.is_none() {
            return;
        }
        let vault = ctx.vault;
        let mut volume = self.tune(MASTER_VOLUME, Q15, self.common_volume, vault);
        let applause = self.tune(CHEER_VOLUME, Q15, self.applause_volume, vault);
        let pc_cheer = self.tune(CHEER_VOLUME, Q15, self.pc_cheer_volume, vault);
        let npc_cheer = self.tune(CHEER_VOLUME, Q15, self.npc_cheer_volume, vault);
        let pc_ohh = self.tune(CHEER_VOLUME, Q15, self.pc_ohh_volume, vault);
        let npc_ohh = self.tune(CHEER_VOLUME, Q15, self.npc_ohh_volume, vault);
        let wet = self.tune(WET_VOLUME, Q15, self.common_wet, vault);
        let dry = self.tune(DRY_VOLUME, Q15, self.common_dry, vault);
        if !IGNORE_NICOTINE {
            volume = self.scaled_volume(state, volume, ctx);
        }
        let azimuth = ctx.output(self.io, 0, PresetType::Azimuth);
        let mut pitch = self.pod_id.wrapping_mul(230).wrapping_add(self.common_pitch).wrapping_sub(230);
        if !IGNORE_NICOTINE {
            pitch = self.scaled_pitch(pitch, ctx);
        }
        let npc = i32::from(self.skater_id != 0);
        let pc = i32::from(self.skater_id == 0);
        let Some(object) = self.behaviour.as_mut() else { return };
        let data = &mut object.data;
        data[behaviour::VOLUME] = volume.clamp(0, 0x7FFF);
        data[behaviour::WET] = wet.clamp(0, 0x7FFF);
        data[behaviour::DRY] = dry.clamp(0, 0x7FFF);
        data[behaviour::HIPASS] = self.common_hipass.clamp(0, 25000);
        data[behaviour::LOPASS] = self.common_lopass.clamp(0, 25000);
        data[behaviour::AZIMUTH] = azimuth.clamp(0, 0xFFFF);
        data[behaviour::PITCH] = pitch.clamp(0, 0x2000);
        data[behaviour::SKATER] = npc;
        data[behaviour::NPC_OHH_VOLUME] = npc.wrapping_mul(npc_ohh).clamp(0, 0x7FFF);
        data[behaviour::NPC_CHEER_VOLUME] = npc.wrapping_mul(npc_cheer).clamp(0, 0x7FFF);
        data[behaviour::PC_OHH_VOLUME] = pc.wrapping_mul(pc_ohh).clamp(0, 0x7FFF);
        data[behaviour::PC_CHEER_VOLUME] = pc.wrapping_mul(pc_cheer).clamp(0, 0x7FFF);
        data[behaviour::APPLAUSE_VOLUME] = applause.clamp(0, 0x7FFF);
        data[behaviour::DISTANCE] = self.distance.clamp(0, 1000);
        ctx.aems.set_member_data(object);
        object.data[behaviour::OHH] = 0;
        object.data[behaviour::CHEER] = 0;
        object.data[behaviour::NEGATIVE_REAX] = 0;
    }

    /// `HandleAmbience` 0x82547270: the murmur bed's tuned volumes through the mixer (an arena's
    /// fading in by 0.005 an update), the cheer intensity and the reactions since the last update.
    fn handle_ambience(&mut self, state: &StateData, ctx: &mut SfxCtx<'_>) {
        if self.ambience.is_none() {
            return;
        }
        let vault = ctx.vault;
        let mut volume = self.tune(MASTER_VOLUME, Q15, self.common_volume, vault);
        let wet = self.tune(WET_VOLUME, Q15, self.common_wet, vault);
        let dry = self.tune(DRY_VOLUME, Q15, self.common_dry, vault);
        let murmur = self.tune(MURMUR_VOLUME, Q15, self.crowd_ambience_volume, vault);
        if !IGNORE_NICOTINE {
            volume = self.scaled_volume(state, volume, ctx);
        }
        let azimuth = ctx.output(self.io, 0, PresetType::Azimuth);
        let pitch = if IGNORE_NICOTINE { self.common_pitch } else { self.scaled_pitch(self.common_pitch, ctx) };
        let arena = state.crowd_type == ARENA_CROWD;
        let Some(object) = self.ambience.as_mut() else { return };
        let data = &mut object.data;
        if arena {
            data[ambience::VOLUME] = fctiwz(volume as f32 * self.arena_volume_mult).clamp(0, 0x7FFF);
            if self.arena_volume_mult < 1.0 {
                self.arena_volume_mult = (f64::from(self.arena_volume_mult) + 0.005) as f32;
            }
        } else {
            data[ambience::VOLUME] = volume.clamp(0, 0x7FFF);
        }
        data[ambience::WET] = wet.clamp(0, 0x7FFF);
        data[ambience::DRY] = dry.clamp(0, 0x7FFF);
        data[ambience::AZIMUTH] = azimuth.clamp(0, 0xFFFF);
        data[ambience::PITCH] = pitch.clamp(0, 0x2000);
        data[ambience::HIPASS] = self.common_hipass.clamp(0, 25000);
        data[ambience::LOPASS] = self.common_lopass.clamp(0, 25000);
        data[ambience::INTENSITY] = self.cheer_intensity.clamp(0, 1000);
        data[ambience::OHH] = i32::from(self.was_ohh);
        self.was_ohh = false;
        data[ambience::CHEER] = i32::from(self.was_cheer);
        self.was_cheer = false;
        data[ambience::CROWD_AMBIENCE_VOLUME] = murmur.clamp(0, 0x7FFF);
        ctx.aems.set_member_data(object);
    }
}

/// `CSTATE_Crowd` (0x80 bytes) without its controller and object.
#[derive(Debug, Clone, Copy)]
struct StateData {
    attached: bool,
    /// `mCrowdType` (`eArenaCrowd` from the constructor), `mCrowdSize`, `mCrowdAemsId`.
    crowd_type: i32,
    crowd_size: i32,
    crowd_aems_id: i32,
    /// `mPosition`, which `SFXCTL_3DCrowdPos` follows.
    position: Vec3,
    /// `mChallengeAttrib`'s `challenges` collection (0: none).
    challenge: u64,
    /// `t_DeltaTime`.
    delta_time: f32,
}

impl StateData {
    /// `GetCrowdTuningKey` 0x82551878: the challenge's `CrowdTuning` collection, 0 without one.
    fn crowd_tuning_key(&self, vault: &Vault) -> u64 {
        crowd_tuning(self.challenge, vault).unwrap_or(0)
    }
}

/// `challenges::CrowdTuning` of a challenge: its `RefSpec`'s collection, `None` without the
/// attribute.
#[must_use]
pub fn crowd_tuning(challenge: u64, vault: &Vault) -> Option<u64> {
    let bytes = vault.find(Collection::new(CHALLENGES, challenge), CROWD_TUNING, 0)?;
    Some(bytes.get(8..16).map_or(0, |key| u64::from_be_bytes(key.try_into().expect("eight bytes"))))
}

/// One `CSTATE_Crowd` with its `SFXCTL_3DCrowdPos` and `SFXObj_Crowd`.
struct CrowdState {
    data: StateData,
    position: Obj3d,
    object: CrowdObject,
}

/// `mSmallCounter` and `mLargeCounter`.
#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    small: i32,
    large: i32,
}

impl Counters {
    /// `GetSmallCrowdCounter` 0x825502b8: the counter, which then steps through 0-5.
    fn small(&mut self) -> i32 {
        let result = self.small;
        self.small = result + 1;
        if result + 1 >= 6 {
            self.small = 0;
        }
        result
    }

    /// `GetLargeCrowdCounter` 0x825502e0: the counter, which then steps through 0-7.
    fn large(&mut self) -> i32 {
        let result = self.large;
        self.large = result + 1;
        if result + 1 >= 8 {
            self.large = 0;
        }
        result
    }
}

/// `CSTATEMGR_Crowd` (0x48 bytes).
pub struct CrowdManager {
    states: Vec<CrowdState>,
    /// `mAttribCrowds`: the started challenge's `aud_crowds` collection.
    attrib_crowds: u64,
    crowd_type: i32,
    challenge_key: u64,
    start_crowds: bool,
    counters: Counters,
}

impl CrowdManager {
    /// The constructor and `EnterGamePlay`: five states, each with object 0 and its position
    /// controller.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..STATES)
            .map(|instance| {
                let id = (STATE << 16) | (instance << 11);
                let mut position = Obj3d::new(vault);
                position.io = map.and_then(|map| map.dmixio(0x6000_0000 | id));
                CrowdState {
                    data: StateData { attached: false, crowd_type: ARENA_CROWD, crowd_size: 0, crowd_aems_id: 0, position: ZERO, challenge: 0, delta_time: 0.0 },
                    position,
                    object: CrowdObject::new(map.and_then(|map| map.dmixio(0x4000_0000 | id))),
                }
            })
            .collect();
        Self { states, attrib_crowds: 0, crowd_type: NO_CROWD, challenge_key: 0, start_crowds: false, counters: Counters::default() }
    }

    /// `mChallengeKey`: the started challenge, 0 while the crowds are stopped.
    #[must_use]
    pub fn challenge_key(&self) -> u64 {
        self.challenge_key
    }

    /// `mStartCrowds`: the crowd type and challenge `UpdateParams` starts once the banks load.
    #[must_use]
    pub fn pending_start(&self) -> Option<(i32, u64)> {
        self.start_crowds.then_some((self.crowd_type, self.challenge_key))
    }

    /// `StartCrowds` 0x82557810: the type, the challenge and its `CrowdTuning` (kept when the
    /// challenge has none); with the crowd banks loaded every free state attaches at the local
    /// skater (`AttachCrowdPod`), the arena only its first unless attached already, else the start
    /// waits for `UpdateParams`.
    pub fn start_crowds(&mut self, crowd_type: i32, challenge_key: u64, loaded: bool, local_position: Vec3, banks_loaded: &[bool; 9], ctx: &mut SfxCtx<'_>) {
        self.crowd_type = crowd_type;
        self.challenge_key = challenge_key;
        if let Some(key) = crowd_tuning(challenge_key, ctx.vault) {
            self.attrib_crowds = key;
        }
        if !loaded {
            self.start_crowds = true;
            return;
        }
        if self.crowd_type != ARENA_CROWD || !self.states[0].data.attached {
            for _ in 0..self.states.len() {
                self.attach_crowd_pod(local_position, 3, challenge_key, banks_loaded, ctx);
                if self.crowd_type == ARENA_CROWD {
                    break;
                }
            }
        }
        self.start_crowds = false;
    }

    /// `AttachCrowdPod` 0x82557300: the first free state takes the position, the size, the
    /// manager's crowd type and the challenge, then attaches (`CSTATE_Base::Attach`: the position
    /// controller's outputs, then `SFXObj_Crowd::InitSFX`).
    fn attach_crowd_pod(&mut self, position: Vec3, num_spectators: i32, challenge_key: u64, banks_loaded: &[bool; 9], ctx: &mut SfxCtx<'_>) {
        let crowd_type = self.crowd_type;
        let Some(state) = self.states.iter_mut().find(|state| !state.data.attached) else { return };
        state.data.position = position;
        state.data.crowd_size = num_spectators;
        state.data.crowd_type = crowd_type;
        state.data.challenge = challenge_key;
        state.data.attached = true;
        if let Some(io) = state.position.io {
            ctx.map.set_output_enabled(io, true);
        }
        if let Some(io) = state.object.io {
            ctx.map.set_output_enabled(io, true);
        }
        state.object.start_crowd(&mut state.data, banks_loaded, ctx);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} crowd pod attach: type {crowd_type}, {num_spectators} spectators at {position:?}, challenge {challenge_key:#018x}");
        }
    }

    /// `StopCrowds` 0x82554210: every attached state's crowd stops and the state detaches
    /// (`CSTATE_Base::Detach`: the object's `Detach`, which stops it again, its outputs off and
    /// inputs cleared, then the controller's), then the challenge clears. The caller then unloads
    /// the crowd banks (`AudioSystem::UnloadCrowdData`).
    pub fn stop_crowds(&mut self, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            if !state.data.attached {
                continue;
            }
            state.object.stop_crowd(&mut state.data, ctx);
            state.data.attached = false;
            state.object.stop_crowd(&mut state.data, ctx);
            if let Some(io) = state.object.io {
                ctx.map.set_output_enabled(io, false);
                ctx.map.clear_input(io);
            }
            state.position.detach();
            if let Some(io) = state.position.io {
                ctx.map.clear_input(io);
            }
        }
        self.challenge_key = 0;
    }

    /// `CSTATEMGR_Crowd::PlayCrowdSound` 0x825573b0. With a challenge started, a score scales to
    /// `maximum_cheer_score` (1000 without one; another skater's by `npc_handicap`) and must reach
    /// `minimum_cheer_threshold`; a wipeout always plays. The sample counters restart at random,
    /// the first five pods take the crowd volumes (fifty spectators, the volume's AEMS group) and
    /// the rest the nearest spectator groups (the arena's one pod moves to the skater), and each
    /// such pod plays the reaction.
    pub fn play_crowd_sound(&mut self, skater_id: i32, score: i32, wiped_out_score: i32, local: bool, inputs: &CrowdSoundInputs, banks_loaded: &[bool; 9], ctx: &mut SfxCtx<'_>) {
        if self.challenge_key == 0 {
            return;
        }
        let mut scaled_score = score;
        if score > 0 {
            let crowds = Collection::new(AUD_CROWDS, self.attrib_crowds);
            let max_cheer = ctx.vault.layout_word(crowds, MAXIMUM_CHEER_SCORE) as i32;
            scaled_score = if max_cheer <= 0 { 1000 } else { score.wrapping_mul(1000) / max_cheer };
            if !local {
                scaled_score = fctiwz(ctx.vault.layout_f32(crowds, NPC_HANDICAP) * scaled_score as f32);
            }
            if ctx.vault.layout_word(crowds, MINIMUM_CHEER_THRESHOLD) as i32 > scaled_score {
                return;
            }
        }
        self.counters.small = (ctx.rng.next_u32() % 6) as i32;
        self.counters.large = (ctx.rng.next_u32() & 7) as i32;
        let objects = self.states.len().min(5);
        let mut remaining = objects;
        let mut used = 0;
        if self.crowd_type != ARENA_CROWD {
            for (index, volume) in inputs.volumes.iter().enumerate() {
                let Some((position, aems_id)) = *volume else { continue };
                let Some(state) = self.states.get_mut(index) else { continue };
                state.data.position = [position[0], position[1], position[2], 0.0];
                state.data.crowd_size = 50;
                state.data.crowd_aems_id = aems_id;
                state.object.pod_id = index as i32;
                let data = state.data;
                state.object.play_crowd_sound(&data, skater_id, score, wiped_out_score, scaled_score, banks_loaded, &mut self.counters, ctx);
                used += 1;
                remaining -= 1;
            }
        }
        let mut pods = [CrowdPodInfo::default(); 5];
        for (slot, pod) in pods.iter_mut().zip(&inputs.pods) {
            *slot = *pod;
        }
        if self.crowd_type == ARENA_CROWD {
            pods[0].pod_pos = inputs.skater_pos;
        }
        remaining = remaining.min(inputs.pods.len());
        if self.crowd_type == ARENA_CROWD {
            remaining = 1;
        }
        for (pod_index, pod) in pods.iter().enumerate().take(remaining) {
            let Some(state) = self.states.get_mut(used + pod_index) else { continue };
            state.data.position = pod.pod_pos;
            if self.crowd_type != ARENA_CROWD {
                state.data.crowd_size = pod.num_spectators;
            }
            state.object.pod_id = (pod_index + used) as i32;
            let data = state.data;
            state.object.play_crowd_sound(&data, skater_id, score, wiped_out_score, scaled_score, banks_loaded, &mut self.counters, ctx);
        }
    }

    /// `CSTATEMGR_Crowd::UpdateParams` 0x825588d8 after a waiting `StartCrowds` (done by the
    /// caller), then the base pass: each attached state's position controller on its position
    /// (`SFXObj_Crowd` has no `UpdateParams`).
    pub fn update_params(&mut self, t: f32, mics: &MicrophoneMan, shared: &mut Pos3dShared, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            state.data.delta_time = t;
            if !state.data.attached {
                continue;
            }
            ctx.delta_time = t;
            let refs = ObjRefs { dir: None, pos: Some(state.data.position), vel: None };
            state.position.update_params(t, refs, mics, shared, ctx.map);
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate` 0x82a7dc40: `SFXObj_Crowd::ProcessUpdate` 0x8254e4d8 of each
    /// attached state: the camera's distance (to 1000) and, for the arena, the pod following the
    /// camera, then the behaviour while a skater is set, then the ambience.
    pub fn process_update(&mut self, mics: &MicrophoneMan, ctx: &mut SfxCtx<'_>) {
        let camera = mics.microphones[0].cur_position;
        for state in &mut self.states {
            ctx.delta_time = state.data.delta_time;
            if !state.data.attached {
                continue;
            }
            if state.object.skater_id >= 0 {
                let mut distance = vmx::length(vmx::sub(camera, state.data.position));
                if distance > 1000.0 || distance < 0.0 {
                    distance = if distance > 1000.0 { 1000.0 } else { 0.0 };
                }
                state.object.distance = fctiwz(distance);
                if state.data.crowd_type == ARENA_CROWD {
                    state.data.position = camera;
                }
                state.object.handle_behaviour(&state.data, ctx);
            }
            state.object.handle_ambience(&state.data, ctx);
        }
    }
}

/// `GetNearestCrowdPodInfo` 0x824f0298: of the thirty cached pods with spectators, the nearest
/// `requested` by distance in x and z, in the order the native's selection leaves them (each new
/// pod appends until the list is full, then replaces the farthest when nearer).
#[must_use]
pub fn nearest_crowd_pods(pods: &[CrowdPodInfo; 30], near: Vec3, requested: usize) -> Vec<CrowdPodInfo> {
    let mut distances = [0.0_f32; 30];
    let mut selected: Vec<CrowdPodInfo> = Vec::with_capacity(requested);
    let mut farthest = 0;
    for (index, pod) in pods.iter().enumerate() {
        distances[index] = -1.0;
        if pod.num_spectators == 0 {
            continue;
        }
        let dx = near[0] - pod.pod_pos[0];
        let dz = near[2] - pod.pod_pos[2];
        let distance = vmx::length_from_squared(dx * dx + dz * dz);
        distances[index] = distance;
        let slot = if selected.len() < requested {
            if !(distances[farthest] >= distance) {
                farthest = selected.len();
            }
            selected.push(*pod);
            selected.len() - 1
        } else {
            if distances[farthest] <= distance {
                continue;
            }
            let slot = farthest;
            distances[slot] = distance;
            farthest = 0;
            for candidate in 1..selected.len() {
                if !(distances[farthest] >= distances[candidate]) {
                    farthest = candidate;
                }
            }
            selected[slot] = *pod;
            slot
        };
        distances[slot] = distance;
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pod(x: f32, spectators: i32) -> CrowdPodInfo {
        CrowdPodInfo { pod_pos: [x, 0.0, 0.0, 0.0], num_spectators: spectators }
    }

    #[test]
    fn the_nearest_pods_fill_in_order_then_replace_the_farthest() {
        let mut pods = [CrowdPodInfo::default(); 30];
        for (index, x) in [10.0, 5.0, 20.0, 1.0, 7.0, 3.0, 30.0].into_iter().enumerate() {
            pods[index] = pod(x, index as i32 + 1);
        }
        let near = nearest_crowd_pods(&pods, ZERO, 5);
        let order: Vec<i32> = near.iter().map(|pod| pod.num_spectators).collect();
        assert_eq!(order, [1, 2, 6, 4, 5]);
    }

    #[test]
    fn empty_pods_are_skipped_and_a_short_list_stays_short() {
        let mut pods = [CrowdPodInfo::default(); 30];
        pods[3] = pod(2.0, 2);
        pods[9] = pod(1.0, 3);
        let near = nearest_crowd_pods(&pods, ZERO, 5);
        assert_eq!(near.iter().map(|pod| pod.num_spectators).collect::<Vec<_>>(), [2, 3]);
    }

    #[test]
    fn the_sample_counters_cycle_six_and_eight() {
        let mut counters = Counters::default();
        let small: Vec<i32> = (0..8).map(|_| counters.small()).collect();
        let large: Vec<i32> = (0..10).map(|_| counters.large()).collect();
        assert_eq!(small, [0, 1, 2, 3, 4, 5, 0, 1]);
        assert_eq!(large, [0, 1, 2, 3, 4, 5, 6, 7, 0, 1]);
    }

    #[test]
    fn group_size_follows_the_pod_and_the_loaded_banks() {
        let mut state = StateData { attached: true, crowd_type: SMALL_CROWD, crowd_size: 3, crowd_aems_id: 4, position: ZERO, challenge: 0, delta_time: 0.0 };
        let mut banks = [false; 9];
        assert_eq!(CrowdObject::crowd_group_size(&state, &banks), 0);
        state.crowd_size = 5;
        banks[1] = true;
        assert_eq!(CrowdObject::crowd_group_size(&state, &banks), 1);
        state.crowd_size = 12;
        banks[2] = true;
        assert_eq!(CrowdObject::crowd_group_size(&state, &banks), 2);
        state.crowd_size = 50;
        assert_eq!(CrowdObject::crowd_group_size(&state, &banks), 4);
    }

    #[test]
    fn the_csis_constructors_clamp_their_members() {
        let behaviour = clamped(&[-5, 0x2_0000, 0x3000, 0, 0, 30000, 0, 2000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 11, -1], &behaviour::LIMITS);
        assert_eq!(behaviour[behaviour::VOLUME], 0);
        assert_eq!(behaviour[behaviour::AZIMUTH], 0xFFFF);
        assert_eq!(behaviour[behaviour::PITCH], 0x2000);
        assert_eq!(behaviour[behaviour::LOPASS], 25000);
        assert_eq!(behaviour[21], 1, "the venue size is at least one");
        assert_eq!(behaviour[behaviour::CROWD_GROUP_SIZE], 5);
        assert_eq!(behaviour[behaviour::SMALL_SAMPLE_COUNT], 10);
        assert_eq!(behaviour[behaviour::LARGE_SAMPLE_COUNT], 0);
    }
}
