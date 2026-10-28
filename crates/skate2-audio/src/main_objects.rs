//! The MAIN state objects `system.rs` does not keep inline: `SFXObj_Bloom` (sfx 9), `SFXObj_HOM`
//! (12), `SFXObj_Jitter` (14) and `SFXObj_NIS`'s mixer volumes (6); the Hall of Meat splice bank
//! `UpdateHOMBankStatus` keeps loaded; and the `Sk8_moments.csi` classes `PlayNISSfx` and
//! `PlayMenuSoundImp` create (`killedit_bloom`, `killedit_logo`, `ownedit_logo`), whose modules
//! are in `moments.big`.

use crate::aems::{Aems, ClassObject};
use crate::math::fmadd;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::random::RwRandom;
use crate::system::AssetStatus;
use crate::utils::fsel;
use crate::vault::{class, Collection, Vault};

/// `32767.0f`.
const Q15: f32 = f32::from_bits(0x46FF_FE00);
/// `100.0f`.
const PERCENT: f32 = f32::from_bits(0x42C8_0000);
/// `Sk8::Scene::SCENE_TYPE_NONE`.
const SCENE_TYPE_NONE: i32 = 0;
/// `Sk8::FE::eSk8HUDState_HOMResults`.
pub const HUD_STATE_HOM_RESULTS: i32 = 9;
/// `Sk8::NIS_SPECIALFLAG_HALLOFMEAT`.
pub const SPECIAL_FLAG_HALL_OF_MEAT: i32 = 3;

fn set_input(map: &mut MixMap, io: Option<DMixIoId>, index: usize, value: i32) {
    if let Some(io) = io {
        map.set_input(io, index, value);
    }
}

fn flag(value: bool) -> i32 {
    if value { 0x7FFF } else { 0 }
}

/// `SFXObjBase::GetVolumeFromMixer` 0x82514560 (`GetDMixOutput(index, DMX_VOL)`).
fn volume(map: &MixMap, io: Option<DMixIoId>, index: i32) -> i32 {
    io.map_or(0, |io| map.output(io, index, PresetType::Volume))
}

/// `SFXObjBase::GetPitchFromMixer` 0x82515210 (`GetDMixOutput(index, DMX_PITCH)`).
fn pitch(map: &MixMap, io: Option<DMixIoId>, index: i32) -> i32 {
    io.map_or(0, |io| map.output(io, index, PresetType::Pitch))
}

/// The parameters every `Sk8_moments.csi` class starts with: Volume, Azimuth 0, Pitch 4096,
/// LowPass 25000, HighPass 0, WetFX 0, DryFX 0x7FFF.
fn moment_parameters(volume: i32) -> Vec<i32> {
    vec![volume.clamp(0, 0x7FFF), 0, 4096, 25000, 0, 0, 0x7FFF]
}

/// `Csis::Fade_to_white`.
const FADE_TO_WHITE: &str = "fade_to_white";
/// `Csis::Killedit_bloom`.
const KILLEDIT_BLOOM: &str = "killedit_bloom";
/// `Csis::Killedit_logo`.
const KILLEDIT_LOGO: &str = "killedit_logo";
/// `Csis::Ownedit_logo`.
const OWNEDIT_LOGO: &str = "ownedit_logo";
/// `common_parms_Volume`.
const MEMBER_VOLUME: usize = 0;
/// `bloom_amount` / `bloom_value` / `logo_value`, after the seven common parameters.
const MEMBER_VALUE: usize = 7;

/// `(int)(fade * 100.0f)` clamped to 0..=100.
fn bloom_amount(fade: f32) -> i32 {
    fctiwz(fade * PERCENT).clamp(0, 100)
}

/// `Sk8::Audio::SFXObj_Bloom` (MAIN sfx 9, 0x24 bytes): `fade_to_white` while the screen fades.
#[derive(Debug, Default)]
pub struct Bloom {
    io: Option<DMixIoId>,
    /// +0x1C `mFadeAmount`.
    fade_amount: f32,
    /// +0x20 `mpFadeToWhite`.
    fade_to_white: Option<ClassObject>,
}

impl Bloom {
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self { io, fade_amount: 0.0, fade_to_white: None }
    }

    /// `UpdateParams` 0x82549fa0, which branches to `CheckForBloom` 0x8253b698. The fade is the
    /// `VisualDirector`'s (`mOutData.bFadeInfo`, `sFadeInfo.mNormalizedFadeValue`) outside a scene,
    /// raised to `AudioSystem::mNISFadeAmount`; input 0 is its Q15. A positive input creates
    /// `fade_to_white` once; zero releases it.
    pub fn update_params(&mut self, scene_type: i32, fade_info: Option<f32>, nis_fade_amount: f32, map: &mut MixMap, aems: &mut Aems) {
        let mut fade = 0.0;
        if scene_type == SCENE_TYPE_NONE {
            if let Some(value) = fade_info {
                fade = value;
            }
        }
        if nis_fade_amount > fade {
            fade = nis_fade_amount;
        }
        self.fade_amount = fade;
        let value = fctiwz(fade * Q15).clamp(0, 0x7FFF);
        set_input(map, self.io, 0, value);
        if value <= 0 {
            if let Some(object) = self.fade_to_white.take() {
                aems.release_object(object);
            }
        } else if self.fade_to_white.is_none() {
            let mut data = moment_parameters(0);
            data.push(bloom_amount(self.fade_amount));
            self.fade_to_white = Some(aems.create_object(FADE_TO_WHITE, data));
        }
    }

    /// `ProcessUpdate` 0x8253b690, which branches to `UpdateBloom` 0x825186d8: the instance's
    /// volume is output 0 and its amount the fade.
    pub fn process_update(&mut self, map: &MixMap, aems: &mut Aems) {
        let Some(object) = self.fade_to_white.as_mut() else { return };
        object.data[MEMBER_VOLUME] = volume(map, self.io, 0).clamp(0, 0x7FFF);
        object.data[MEMBER_VALUE] = bloom_amount(self.fade_amount);
        aems.set_member_data(object);
    }
}

/// `Sk8::Audio::SFXObj_HOM` (MAIN sfx 12).
#[derive(Debug, Default)]
pub struct Hom {
    io: Option<DMixIoId>,
}

impl Hom {
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self { io }
    }

    /// `UpdateParams` 0x8253c598: input 0 is `mIsHallOfMeat`, 1 the Hall of Meat results screen
    /// (`mSk8HUDState`), 2 a Hall of Meat scene (`mCurrentSceneInfo.specialFlags`).
    pub fn update_params(&self, hall_of_meat: bool, hud_state: i32, special_flags: i32, map: &mut MixMap) {
        set_input(map, self.io, 0, flag(hall_of_meat));
        set_input(map, self.io, 1, flag(hud_state == HUD_STATE_HOM_RESULTS));
        set_input(map, self.io, 2, flag(special_flags == SPECIAL_FLAG_HALL_OF_MEAT));
    }
}

/// `Sk8::Audio::Jitterer` (0x1C bytes): a value wandering between `mid_point - range` and
/// `mid_point + range` by random steps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jitterer {
    /// +0x0 `mCurrentPos`.
    pub current_pos: f32,
    /// +0x4 `mCurrentDelta`.
    pub current_delta: f32,
    /// +0x8 `mMaxDelta`.
    pub max_delta: f32,
    /// +0xC `mMinDelta`.
    pub min_delta: f32,
    /// +0x10 `mMidPoint`.
    pub mid_point: f32,
    /// +0x14 `mRange`.
    pub range: f32,
    /// +0x18 `mMultiplier`.
    pub multiplier: f32,
}

impl Default for Jitterer {
    /// The values `SFXObj_Jitter`'s constructor 0x8253c860 writes.
    fn default() -> Self {
        Self {
            current_pos: 0.0,
            current_delta: 0.0,
            max_delta: f32::from_bits(0x3DCC_CCCD),
            min_delta: 0.0,
            mid_point: 0.0,
            range: 1.0,
            multiplier: 1.0,
        }
    }
}

impl Jitterer {
    /// `Update` 0x82518858: a step of `(Random() % 2001 - 1000) / 1000` of the delta span, pushed
    /// away from zero by `mMinDelta`, joins the delta, which stays within `mMaxDelta`; a position
    /// past either bound reflects back and reverses the delta.
    pub fn update(&mut self, rng: &mut RwRandom) {
        let draw = (rng.next_u32() % 2001) as i32 - 1000;
        let step = (self.max_delta - self.min_delta) * (draw as f32 * f32::from_bits(0x3A83_126F));
        let step = if step < 0.0 { step - self.min_delta } else { self.min_delta + step };
        let delta = step + self.current_delta;
        self.current_delta = delta;
        let low = -self.max_delta;
        let delta = fsel(low - delta, low, delta);
        let delta = fsel(self.max_delta - delta, delta, self.max_delta);
        self.current_delta = delta;
        let upper = self.mid_point + self.range;
        let mut position = delta + self.current_pos;
        if !(position <= upper) {
            let over = position - upper;
            self.current_delta = delta * -1.0;
            position = upper - over;
        } else {
            let lower = self.mid_point - self.range;
            if position < lower {
                let under = lower - position;
                self.current_delta = delta * -1.0;
                position = under + lower;
            }
        }
        let lower = self.mid_point - self.range;
        let position = fsel(lower - position, lower, position);
        self.current_pos = fsel(upper - position, position, upper);
    }

    /// `SFXObj_Jitter::UpdateParams`' input: the position scaled about the midpoint by
    /// `mMultiplier` (`fmadds`), held within the range and 0..=32767.
    #[must_use]
    pub fn input(&self) -> i32 {
        let lower = self.mid_point - self.range;
        let upper = self.range + self.mid_point;
        let value = fmadd(self.current_pos - self.mid_point, self.multiplier, self.mid_point);
        let value = fsel(lower - value, lower, value);
        let value = fsel(upper - value, value, upper);
        let value = fsel(-value, 0.0, value);
        let value = fsel(Q15 - value, value, Q15);
        fctiwz(value).clamp(0, 0x7FFF)
    }
}

/// The `aud_general` collection `SFXObj_Jitter::InitSFX` reads.
const JITTER: Collection = Collection::new(class::AUD_GENERAL, 0x6F05_8488_8913_307B);
/// Its array of 20-byte jitterers: midpoint, range, max delta, min delta, multiplier.
const JITTERERS: u64 = 0xB66A_AD95_7873_A8B3;
const MAX_JITTERERS: usize = 10;

/// `Sk8::Audio::SFXObj_Jitter` (MAIN sfx 14, 0x138 bytes): one mixer input per jitterer.
#[derive(Debug, Default)]
pub struct Jitter {
    io: Option<DMixIoId>,
    /// +0x1C `mJitterer`.
    jitterers: [Jitterer; MAX_JITTERERS],
    /// +0x134 `mNumJitterers`.
    count: usize,
}

impl Jitter {
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self { io, jitterers: [Jitterer::default(); MAX_JITTERERS], count: 0 }
    }

    /// `InitSFX` 0x8253ca38: the output on, then up to ten jitterers from the collection, each
    /// starting at its midpoint with no delta.
    pub fn init_sfx(&mut self, vault: &Vault, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.count = (vault.length(JITTER, JITTERERS) as usize).min(MAX_JITTERERS);
        for (index, jitterer) in self.jitterers[..self.count].iter_mut().enumerate() {
            let bytes = vault.bytes(JITTER, JITTERERS, index as u32, 20);
            let word = |at: usize| f32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            *jitterer = Jitterer {
                current_pos: word(0),
                current_delta: 0.0,
                max_delta: word(8),
                min_delta: word(12),
                mid_point: word(0),
                range: word(4),
                multiplier: word(16),
            };
        }
    }

    /// `UpdateParams` 0x8254a0b0: each jitterer steps and sets the input of its index.
    pub fn update_params(&mut self, rng: &mut RwRandom, map: &mut MixMap) {
        for (index, jitterer) in self.jitterers[..self.count].iter_mut().enumerate() {
            jitterer.update(rng);
            set_input(map, self.io, index, jitterer.input());
        }
    }

    #[must_use]
    pub fn jitterers(&self) -> &[Jitterer] {
        &self.jitterers[..self.count]
    }
}

/// `SFXObj_NIS` +0x1C..+0x58: the volumes `ProcessUpdate` 0x82517348 reads from the mixer for
/// the speech system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NisVolumes {
    /// `mScriptedSpeechVol` (output 0) and `mScriptedSpeechPitch` (pitch 13).
    pub scripted_speech: i32,
    pub scripted_speech_pitch: i32,
    /// `mOtherNISVol` (1).
    pub other: i32,
    /// `mStoryNISVol` (2).
    pub story: i32,
    /// `mPhotoHighlightVol` (3).
    pub photo_highlight: i32,
    /// `mProHighlightVol` (4).
    pub pro_highlight: i32,
    /// `mRaceNISVol` (5).
    pub race: i32,
    /// `mFlythroughVol` (6).
    pub flythrough: i32,
    /// `mOTSSucessVol` (7).
    pub ots_success: i32,
    /// `mContestHighlightVol` (8).
    pub contest_highlight: i32,
    /// `mFilmHighlightVol` (9).
    pub film_highlight: i32,
    /// `mNisPhotographerVol` (10).
    pub nis_photographer: i32,
    /// `mCellPhoneVol` (11).
    pub cell_phone: i32,
    /// `mNISProVol` (12).
    pub nis_pro: i32,
    /// `mTutorialVol` (14).
    pub tutorial: i32,
    /// `mTutorialSucessVol` (15).
    pub tutorial_success: i32,
}

impl NisVolumes {
    /// `SFXObj_NIS::ProcessUpdate` 0x82517348.
    #[must_use]
    pub fn read(map: &MixMap, io: Option<DMixIoId>) -> Self {
        Self {
            scripted_speech: volume(map, io, 0),
            scripted_speech_pitch: pitch(map, io, 13),
            other: volume(map, io, 1),
            story: volume(map, io, 2),
            photo_highlight: volume(map, io, 3),
            pro_highlight: volume(map, io, 4),
            race: volume(map, io, 5),
            flythrough: volume(map, io, 6),
            ots_success: volume(map, io, 7),
            contest_highlight: volume(map, io, 8),
            film_highlight: volume(map, io, 9),
            tutorial: volume(map, io, 14),
            tutorial_success: volume(map, io, 15),
            nis_photographer: volume(map, io, 10),
            cell_phone: volume(map, io, 11),
            nis_pro: volume(map, io, 12),
        }
    }

    /// `GetNISVol` 0x82547e80: the volume of `GetNISType`'s `eNISType`, `mOtherNISVol` for none.
    #[must_use]
    pub fn nis_vol(&self, nis_type: i32) -> i32 {
        match nis_type {
            0 => self.photo_highlight,
            1 => self.pro_highlight,
            2 => self.race,
            3 => self.flythrough,
            4 => self.ots_success,
            5 => self.story,
            6 => self.contest_highlight,
            7 => self.film_highlight,
            8 => self.tutorial,
            9 => self.tutorial_success,
            _ => self.other,
        }
    }
}

/// `sHOMBankNames`, all in `data\audio\splicercollision.big`.
pub const HOM_BANK_NAMES: [&str; 5] = ["HOM_Set_1.bnk", "HOM_Set_2.bnk", "HOM_Set_3.bnk", "HOM_Set_4.bnk", "HOM_Set_5.bnk"];

/// The Hall of Meat splice bank (`SPLICE_TYPE` 7) `UpdateHOMBankStatus` 0x824f1650 keeps in a
/// 0x40000-byte swap bank: `mHOMBankStatus` (+0x79C), `mWasHallOfMeat` (+0x795) and
/// `mbHOMBankLoaded` (+0x7A0).
#[derive(Debug)]
pub struct HomBank {
    pub status: AssetStatus,
    was_hall_of_meat: bool,
    loaded: [bool; 5],
    /// The bank `AsyncLoadToSwapSlot` is reading.
    reading: Option<usize>,
}

impl Default for HomBank {
    fn default() -> Self {
        Self { status: AssetStatus::Empty, was_hall_of_meat: false, loaded: [false; 5], reading: None }
    }
}

impl HomBank {
    /// `GetHOMBank` 0x824e6a08: one of the banks not loaded yet, by `Random() % count` (all five
    /// again once every one has been); `sHOMBankNames[0]` when the draw finds none.
    pub fn get_hom_bank(&mut self, rng: &mut RwRandom) -> usize {
        let mut count = self.loaded.iter().filter(|&&loaded| !loaded).count() as u32;
        if count == 0 {
            self.loaded = [false; 5];
            count = 5;
        }
        let draw = rng.next_u32() % count;
        let mut seen = 0;
        for index in 0..5 {
            if self.loaded[index] {
                continue;
            }
            if seen == draw {
                self.loaded[index] = true;
                return index;
            }
            seen += 1;
        }
        0
    }

    /// `UpdateHOMBankStatus` 0x824f1650: an empty bank starts reading `GetHOMBank`'s file; the
    /// read completes on the next update (`AsyncToSwapSlotStatus`) once `install` takes the bank
    /// as splice type 7, and a read that cannot install never completes; a loaded bank notes a
    /// Hall of Meat (`mWasHallOfMeat`). Nothing sets `eAssetStatus_Unloading`, so its reload
    /// branch never runs.
    pub fn update(&mut self, hall_of_meat: bool, rng: &mut RwRandom, install: impl FnOnce(usize) -> bool) {
        match self.status {
            AssetStatus::Empty => {
                self.reading = Some(self.get_hom_bank(rng));
                self.status = AssetStatus::Loading;
            }
            AssetStatus::Loading => {
                if let Some(bank) = self.reading.take() {
                    if install(bank) {
                        self.status = AssetStatus::Loaded;
                    }
                }
            }
            AssetStatus::Loaded => {
                if hall_of_meat {
                    self.was_hall_of_meat = true;
                }
            }
            AssetStatus::Unloading => {}
        }
    }
}

/// `AudioSystem::mpKilledIt` (+0x78C) and `mpOwnedIt` (+0x790): the `killedit_bloom` instances
/// `PlayNISSfx` plays under a "killed it" or "owned it" highlight.
#[derive(Debug, Default)]
pub struct Moments {
    killed_it: Option<ClassObject>,
    owned_it: Option<ClassObject>,
}

impl Moments {
    /// `PlayNISSfx` 0x82504540 for ids 1 to 5 (an `ICEUserChannelAudioEffect` command 2). Id 1
    /// replaces the killed-it bloom at `mKilledBloomVol`, 4 the owned-it one at `mOwnedBloomVol`
    /// (both are `killedit_bloom`); 2, 3 and 5 move the running one's `bloom_value` on. The value
    /// is the id clamped to 1..=5.
    pub fn play_nis_sfx(&mut self, id: i32, killed_bloom_vol: i32, owned_bloom_vol: i32, aems: &mut Aems) {
        let value = id.clamp(1, 5);
        let (slot, volume) = match id {
            1 => (&mut self.killed_it, killed_bloom_vol),
            4 => (&mut self.owned_it, owned_bloom_vol),
            2 | 3 | 5 => {
                let running = if id == 5 { &mut self.owned_it } else { &mut self.killed_it };
                if let Some(object) = running.as_mut() {
                    object.data[MEMBER_VALUE] = value;
                    aems.set_member_data(object);
                }
                return;
            }
            _ => return,
        };
        if let Some(object) = slot.take() {
            aems.release_object(object);
        }
        let mut data = moment_parameters(volume);
        data.push(value);
        *slot = Some(aems.create_object(KILLEDIT_BLOOM, data));
    }

    /// `NotifyNISPlaybackFinish` 0x824fe558's releases of both blooms.
    pub fn release(&mut self, aems: &mut Aems) {
        for object in [self.killed_it.take(), self.owned_it.take()].into_iter().flatten() {
            aems.release_object(object);
        }
    }
}

/// `PlayMenuSoundImp` 0x8250c738's logo for an `fe` sound with a logo type: `ownedit_logo` for
/// type 4, else `killedit_logo` with `logo_value` the type clamped to 1..=5; the volume is the
/// master's logo volume times the sound's. The instance is released before the call returns, so
/// its module plays out alone.
pub fn play_logo(logo_type: i32, sound_volume: f32, killed_logo_vol: i32, owned_logo_vol: i32, aems: &mut Aems) {
    let object = if logo_type == 4 {
        aems.create_object(OWNEDIT_LOGO, moment_parameters(fctiwz(owned_logo_vol as f32 * sound_volume)))
    } else {
        let mut data = moment_parameters(fctiwz(killed_logo_vol as f32 * sound_volume));
        data.push(logo_type.clamp(1, 5));
        aems.create_object(KILLEDIT_LOGO, data)
    };
    aems.release_object(object);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jitterer(mid: f32, range: f32, max: f32, min: f32) -> Jitterer {
        Jitterer { current_pos: mid, current_delta: 0.0, max_delta: max, min_delta: min, mid_point: mid, range, multiplier: 1.0 }
    }

    #[test]
    fn jitterer_steps_stay_in_range_and_reflect() {
        let mut rng = RwRandom::new(0x1234_5678);
        let mut j = jitterer(16384.0, 16383.0, 29000.0, 21000.0);
        for _ in 0..1000 {
            j.update(&mut rng);
            assert!(j.current_pos >= 1.0 && j.current_pos <= 32767.0, "{j:?}");
            assert!(j.current_delta.abs() <= 29000.0, "{j:?}");
            assert!((0..=0x7FFF).contains(&j.input()));
        }
    }

    #[test]
    fn jitterer_step_matches_a_hand_computed_draw() {
        let mut probe = RwRandom::new(7);
        let draw = (probe.next_u32() % 2001) as i32 - 1000;
        let mut rng = RwRandom::new(7);
        let mut j = jitterer(0.0, 10.0, 2.0, 1.0);
        j.update(&mut rng);
        let step = 1.0 * (draw as f32 * f32::from_bits(0x3A83_126F));
        let step = if step < 0.0 { step - 1.0 } else { 1.0 + step };
        assert_eq!(j.current_delta, step.clamp(-2.0, 2.0));
        assert_eq!(j.current_pos, step.clamp(-2.0, 2.0));
    }

    #[test]
    fn jitterer_reflects_off_the_upper_bound() {
        // A zero span makes every step `0.0 * draw` (-0.0 for negative draws, which is not below
        // zero), so the step is always +mMinDelta.
        let mut j = jitterer(0.0, 1.0, 5.0, 5.0);
        j.current_pos = 0.5;
        let mut rng = RwRandom::new(1);
        j.update(&mut rng);
        assert_eq!(j.current_delta, -5.0);
        assert_eq!(j.current_pos, -1.0);
    }

    #[test]
    fn jitter_input_scales_about_the_midpoint_and_clamps() {
        let mut j = jitterer(100.0, 50.0, 1.0, 0.0);
        j.current_pos = 120.0;
        j.multiplier = 2.0;
        assert_eq!(j.input(), 140);
        j.multiplier = 4.0;
        assert_eq!(j.input(), 150);
        let mut low = jitterer(10.0, 40.0, 1.0, 0.0);
        low.current_pos = -30.0;
        assert_eq!(low.input(), 0);
        let mut nan = jitterer(10.0, 40.0, 1.0, 0.0);
        nan.current_pos = f32::NAN;
        assert_eq!(nan.input(), 50);
    }

    #[test]
    fn hom_bank_draws_each_bank_once_before_repeating() {
        let mut bank = HomBank::default();
        let mut rng = RwRandom::new(99);
        let mut seen = [0; 5];
        for _ in 0..5 {
            seen[bank.get_hom_bank(&mut rng)] += 1;
        }
        assert_eq!(seen, [1; 5]);
        let again = bank.get_hom_bank(&mut rng);
        assert!(again < 5);
        assert_eq!(bank.loaded.iter().filter(|&&loaded| loaded).count(), 1);
    }

    #[test]
    fn hom_bank_loads_on_the_update_after_the_draw() {
        let mut bank = HomBank::default();
        let mut rng = RwRandom::new(3);
        bank.update(false, &mut rng, |_| panic!("no read yet"));
        assert_eq!(bank.status, AssetStatus::Loading);
        let mut installed = None;
        bank.update(false, &mut rng, |index| {
            installed = Some(index);
            true
        });
        assert!(installed.is_some());
        assert_eq!(bank.status, AssetStatus::Loaded);
        bank.update(true, &mut rng, |_| panic!("loaded"));
        assert!(bank.was_hall_of_meat);
    }

    #[test]
    fn hom_bank_read_that_cannot_install_never_completes() {
        let mut bank = HomBank::default();
        let mut rng = RwRandom::new(3);
        bank.update(false, &mut rng, |_| true);
        bank.update(false, &mut rng, |_| false);
        assert_eq!(bank.status, AssetStatus::Loading);
        bank.update(false, &mut rng, |_| panic!("no read pending"));
        assert_eq!(bank.status, AssetStatus::Loading);
    }

    #[test]
    fn nis_vol_follows_the_nis_type() {
        let volumes = NisVolumes { other: 1, photo_highlight: 2, story: 3, tutorial_success: 4, ..NisVolumes::default() };
        assert_eq!(volumes.nis_vol(0), 2);
        assert_eq!(volumes.nis_vol(5), 3);
        assert_eq!(volumes.nis_vol(9), 4);
        assert_eq!(volumes.nis_vol(10), 1);
        assert_eq!(volumes.nis_vol(-1), 1);
    }

    #[test]
    fn moments_replace_and_advance_their_blooms() {
        let mut aems = Aems::new(None);
        let mut moments = Moments::default();
        moments.play_nis_sfx(2, 100, 200, &mut aems);
        assert!(moments.killed_it.is_none());
        moments.play_nis_sfx(1, 0x9000, 200, &mut aems);
        let killed = moments.killed_it.as_ref().map(|object| object.data.clone());
        assert_eq!(killed, Some(vec![0x7FFF, 0, 4096, 25000, 0, 0, 0x7FFF, 1]));
        moments.play_nis_sfx(3, 100, 200, &mut aems);
        assert_eq!(moments.killed_it.as_ref().map(|object| object.data[MEMBER_VALUE]), Some(3));
        moments.play_nis_sfx(4, 100, -5, &mut aems);
        assert_eq!(moments.owned_it.as_ref().map(|object| object.data.clone()), Some(vec![0, 0, 4096, 25000, 0, 0, 0x7FFF, 4]));
        moments.play_nis_sfx(5, 100, 200, &mut aems);
        assert_eq!(moments.owned_it.as_ref().map(|object| object.data[MEMBER_VALUE]), Some(5));
        moments.release(&mut aems);
        assert!(moments.killed_it.is_none() && moments.owned_it.is_none());
    }

    #[test]
    fn bloom_amount_truncates_and_clamps() {
        assert_eq!(bloom_amount(0.505), 50);
        assert_eq!(bloom_amount(2.0), 100);
        assert_eq!(bloom_amount(-0.5), 0);
        assert_eq!(bloom_amount(f32::NAN), 0);
    }
}
