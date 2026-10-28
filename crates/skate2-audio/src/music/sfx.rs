//! `Sk8::Audio::SFXObj_Music` (MAIN object 1) and its position controller `SFXCTL_MusicPos`
//! (MAIN control 0, an `SFXCTL_3DObjPos` on the music system's world sound source). The mixer
//! turns the object's inputs into the per-volume-type music volumes, wet volumes and low-pass
//! frequencies the music system reads back.

use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::vault::{class, Collection, Vault};
use crate::vmx::Vec3;
use crate::world::emitter_system::MusicVolumeInfo;

/// `SFXObj_Music`'s DMixIO id (state 0, object 1) and `SFXCTL_MusicPos`'s (state 0, control 0).
pub const MUSIC_OBJECT_ID: u32 = 0x4000_0010;
pub const MUSIC_POS_ID: u32 = 0x6000_0000;

/// `aud_general` "music".
const GENERAL_MUSIC: Collection = Collection::new(class::AUD_GENERAL, 0x47EC_76B4_F9FC_79F6);
/// The rockstar levels for the x1.5, x2 and x3 multipliers, and the per-second fall and rise.
const TIMES_ONE_FIVE_ROCKSTAR_LEVEL: u64 = 0x6EE4_718F_1A7E_B772;
const TIMES_TWO_ROCKSTAR_LEVEL: u64 = 0x50F6_520E_2DD3_D54C;
const TIMES_THREE_ROCKSTAR_LEVEL: u64 = 0xA6AA_0C53_4DEA_29E7;
const ROCKSTAR_FALL_RATE: u64 = 0x7C44_AE01_6D99_A9EE;
const ROCKSTAR_RISE_RATE: u64 = 0x1666_A4A4_5EC3_09AE;

/// What `UpdateParams` reads from the rest of the audio system.
#[derive(Debug, Clone, Copy, Default)]
pub struct MusicObjectInputs {
    /// `Sk8EmitterSystem::GetCurrentMusicVolume`.
    pub music_volume: Option<MusicVolumeInfo>,
    /// `AudioSystem::mbIsTimesOneFive`, `mbIsTimesTwo`, `mbIsTimesThree`.
    pub times: [bool; 3],
    /// `MusicCore::GameMusicDisabled`.
    pub game_music_disabled: bool,
    /// `MusicSystem::IsMusicPlaying`.
    pub music_playing: bool,
    /// `MusicSystem::mbChallengeLoading`.
    pub challenge_loading: bool,
}

/// `Sk8::Audio::SFXObj_Music` and its `SFXCTL_MusicPos`.
#[derive(Debug, Clone, Default)]
pub struct MusicObject {
    io: Option<DMixIoId>,
    /// `mVol[9]`, `mWetVol[9]`, `mLPF[9]` by `eMusicVolType`.
    pub vol: [i32; 9],
    pub wet_vol: [i32; 9],
    pub azi: i32,
    pub twist: i32,
    pub dist: i32,
    pub lpf: [i32; 9],
    position: Obj3d,
    was_in_inner_sphere: bool,
    last_rockstar_level: i32,
}

fn flag(value: bool) -> i32 {
    if value { 0x7FFF } else { 0 }
}

impl MusicObject {
    /// `SFXObj_Music::SFXObj_Music` 0x82515328, `SFXCTL_MusicPos::CreateObject` 0x8251a908 and
    /// `InitSFX` 0x82528a00 (`TurnOnMixOutput`, then `SetSFXObject(MAIN, SFXOBJ_MUSIC)`, which
    /// assigns the world sound source to the controller).
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&mut MixMap>) -> Self {
        let mut object = Self { position: Obj3d::new(vault), ..Self::default() };
        if let Some(map) = map {
            object.io = map.dmixio(MUSIC_OBJECT_ID);
            object.position.io = map.dmixio(MUSIC_POS_ID);
            if let Some(io) = object.io {
                map.set_output_enabled(io, true);
            }
        }
        object
    }

    fn set(map: &mut MixMap, io: Option<DMixIoId>, index: usize, value: i32) {
        if let Some(io) = io {
            map.set_input(io, index, value);
        }
    }

    /// `SFXCTL_3DObjPos::UpdateParams` 0x8253d6c8 for `SFXCTL_MusicPos`, whose direction and
    /// position are the music system's `mWorldSoundSource` (`SetSFXObj` 0x824e8ad0).
    pub fn update_position(&mut self, t: f32, dir: Vec3, pos: Vec3, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        let refs = ObjRefs { dir: Some(dir), pos: Some(pos), vel: None };
        self.position.update_params(t, refs, mics, shared, map);
    }

    /// `SFXObj_Music::UpdateParams` 0x82528a88.
    pub fn update_params(&mut self, t: f32, inputs: MusicObjectInputs, vault: &Vault, map: &mut MixMap) {
        let io = self.io;
        if let Some(volume) = inputs.music_volume {
            let distance = crate::nicotine::shape::fctiwz(volume.distance * 32767.0).clamp(0, 0x7FFF);
            Self::set(map, io, 0, distance);
            if volume.distance == 0.0 {
                Self::set(map, io, 1, 0x7FFF);
                self.was_in_inner_sphere = true;
            } else {
                Self::set(map, io, 1, 0);
            }
        } else {
            Self::set(map, io, 0, 0x7FFF);
            Self::set(map, io, 1, 0);
            self.was_in_inner_sphere = false;
        }
        Self::set(map, io, 2, flag(self.was_in_inner_sphere));
        Self::set(map, io, 3, flag(inputs.times[2]));
        let mut level = 0;
        if t > 0.0 {
            let int = |key: u64| vault.i32(GENERAL_MUSIC, key);
            if inputs.times[0] {
                level = int(TIMES_ONE_FIVE_ROCKSTAR_LEVEL);
            } else if inputs.times[1] {
                level = int(TIMES_TWO_ROCKSTAR_LEVEL);
            } else if inputs.times[2] {
                level = int(TIMES_THREE_ROCKSTAR_LEVEL);
            }
            let fall = crate::nicotine::shape::fctiwz(vault.f32(GENERAL_MUSIC, ROCKSTAR_FALL_RATE) * t);
            let rise = crate::nicotine::shape::fctiwz(vault.f32(GENERAL_MUSIC, ROCKSTAR_RISE_RATE) * t);
            let last = self.last_rockstar_level;
            if level < last {
                if last - level > fall {
                    level = last - fall;
                }
            } else if level > last && level - last > rise {
                level = last + rise;
            }
        }
        self.last_rockstar_level = level;
        Self::set(map, io, 6, level.clamp(0, 0x7FFF));
        Self::set(map, io, 4, flag(inputs.game_music_disabled));
        Self::set(map, io, 5, flag(inputs.music_playing));
        Self::set(map, io, 7, 0);
        Self::set(map, io, 8, flag(inputs.challenge_loading));
    }

    /// `SFXObj_Music::ProcessUpdate` 0x82515408: volumes from outputs 1-7, 15 and 17, the azimuth
    /// from output 0, the controller's camera distance and twist inputs (1 and 10), frequencies
    /// from outputs 8-14, 16 and 18, wet volumes from outputs 19-27.
    pub fn process_update(&mut self, map: &MixMap) {
        let Some(io) = self.io else { return };
        let volume = |index: i32| map.output(io, index, PresetType::Volume);
        let frequency = |index: i32| map.output(io, index, PresetType::Frequency);
        for (slot, index) in [1, 2, 3, 4, 5, 6, 7, 15, 17].into_iter().enumerate() {
            self.vol[slot] = volume(index);
        }
        self.azi = map.output(io, 0, PresetType::Azimuth);
        let control = |index: usize| self.position.io.map_or(0, |io| map.input(io, index));
        self.dist = control(1);
        self.twist = control(10);
        for (slot, index) in [8, 9, 10, 11, 12, 13, 14, 16, 18].into_iter().enumerate() {
            self.lpf[slot] = frequency(index);
        }
        for (slot, index) in (19..=27).enumerate() {
            self.wet_vol[slot] = volume(index);
        }
    }
}
