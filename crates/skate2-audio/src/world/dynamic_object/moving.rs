//! `SFXObj_Moving` (0x00080010): the sounds of a dynamic object the skater moves by hand. The
//! start (`c_sliding_start` / `c_rolling_start`, `DMO_Starts.abk`) plays at once; the object's
//! own moveable bank and random splice bank load into the manager's swap banks, then its
//! `c_moveable_*_objects` loop and the random splices (`SPLICE_TYPE` 22) play.

use super::{dynamic_object_or_default, layout_u64, MoveableChains, Physics, SwapBanks, AUD_DYNAMICOBJECTRANDOM, KPH, SPLICE_DMO_RANDOM, SWAP_MOVING, SWAP_RANDOM};
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::player::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::random::RwRandom;
use crate::splicer::{SpliceId, SpliceUpdateParams};
use crate::utils::clamp_fsel;
use crate::vault::Collection;

/// The layout words of `mSlideAngleLFOAmp` .. `mSlideSweetenerOut`, the slide crossfades
/// (`msxms_*`, `msaxmsa_*`), `mMoveable_slide_stop`, the six slide volumes, and the start and
/// stop volumes.
const SLIDE_WORDS: [u32; 10] = [73, 72, 62, 61, 56, 55, 57, 60, 58, 59];
const SLIDE_XFADE_WORDS: [u32; 9] = [51, 49, 50, 48, 46, 47, 54, 52, 53];
const SLIDE_ANGLE_XFADE_WORDS: [u32; 9] = [68, 66, 67, 65, 63, 64, 71, 69, 70];
const SLIDE_VOLUME_WORDS: [u32; 6] = [22, 21, 23, 25, 24, 26];
/// The roll attributes, in the same order (`GetAttribVariables` reads them by key).
const ROLL_KEYS: [u64; 10] = [
    0xEF6D_5AFC_BB9E_77D1,
    0x100F_2278_3E95_5CCB,
    0xDC86_01C0_F2CF_D377,
    0xC066_67B2_D737_42A9,
    0x2F85_4235_5CFA_BE9C,
    0x034E_2678_A154_7AB5,
    0xC848_E2A3_1D8B_4388,
    0x8D5F_8F74_3569_2C63,
    0x4F83_7B7F_BD0F_8C0B,
    0xFDCC_49BD_07A8_C29C,
];
const ROLL_XFADE_KEYS: [u64; 9] = [
    0xBE6F_283B_2906_6754,
    0x6EAD_CAE5_611A_C954,
    0x8D4F_FC6C_78B6_3FC2,
    0xAA61_57F0_C59F_EC7A,
    0x468C_8187_DB55_3DE0,
    0x5666_83AE_4847_D270,
    0x75AA_A4C5_645F_E970,
    0x9CCD_964E_5D35_7264,
    0xA88B_6FEF_BF37_6A65,
];
const ROLL_ANGLE_XFADE_KEYS: [u64; 9] = [
    0xD11A_3A52_7B06_8243,
    0x5C25_3F37_6573_54B1,
    0xA945_B0A7_0E81_A9FB,
    0x8E01_02F0_0272_C985,
    0x2123_7F59_A408_17C8,
    0x6FB1_F1D3_084A_D4B9,
    0x662F_9353_B313_EF73,
    0x0C62_3FD6_2BED_CEC1,
    0x2DE0_BB7C_51C4_5B97,
];
const ROLL_STOP_KEY: u64 = 0xC767_AE54_A4B2_141B;
const ROLL_VOLUME_KEYS: [u64; 6] = [
    0x5A40_AE51_F18D_7BDA,
    0xAFB7_ED7E_3E21_6814,
    0x581C_2D89_164B_EF45,
    0x8729_CA6B_14F5_D983,
    0x1C4D_5773_FAB9_9FE0,
    0xF4EA_C394_BCB7_FDA7,
];
const ROLL_VOLUME_START_KEY: u64 = 0x9BA3_BB6B_50E9_C63A;
const ROLL_VOLUME_STOP_KEY: u64 = 0x262E_68E0_665B_1879;

/// The slide or roll half of the attribute variables.
#[derive(Debug, Clone, Copy, Default)]
struct Moveable {
    /// Angle, speed and volume LFO amplitude and frequency, then the sweetener volume, in, peak
    /// and out.
    lfo: [i32; 10],
    xfades: [i32; 9],
    angle_xfades: [i32; 9],
    /// lo, med, hi, then the angle lo, med, hi volumes.
    volumes: [i32; 6],
    volume_start: i32,
    volume_stop: i32,
}

/// `Sk8::Audio::SFXObj_Moving` (0x238 bytes).
#[derive(Debug)]
pub(super) struct Moving {
    pub(super) io: Option<DMixIoId>,
    max_speed: f32,
    max_rotational_speed: f32,
    material_id: i32,
    pitch_offset: i32,
    sliding_start: Option<ClassObject>,
    rolling_start: Option<ClassObject>,
    sliding: Option<ClassObject>,
    rolling: Option<ClassObject>,
    file_name: String,
    data_loaded: [bool; 2],
    loading: [bool; 2],
    random_splice_file_name: String,
    random_splice_cluster: i32,
    splice_volume: f32,
    splice_pitch: f32,
    splice_max_speed: f32,
    min_time_at_zero: i32,
    max_time_at_zero: i32,
    min_time_at_max: i32,
    max_time_at_max: i32,
    random_probability: i32,
    volume_at_zero: i32,
    volume_at_max: i32,
    random_duck_at_zero: i32,
    random_duck_at_max: i32,
    slide: Moveable,
    slide_stop: i32,
    roll: Moveable,
    roll_stop: i32,
    random_splice: Option<SpliceId>,
    time_to_next_random_splice: f32,
}

impl Moving {
    /// The constructor 0x82517610.
    pub(super) fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            max_speed: 0.0,
            max_rotational_speed: 0.0,
            material_id: 0,
            pitch_offset: 0,
            sliding_start: None,
            rolling_start: None,
            sliding: None,
            rolling: None,
            file_name: String::new(),
            data_loaded: [false; 2],
            loading: [false; 2],
            random_splice_file_name: String::new(),
            random_splice_cluster: 0,
            splice_volume: 0.0,
            splice_pitch: 0.0,
            splice_max_speed: 0.0,
            min_time_at_zero: 0,
            max_time_at_zero: 0,
            min_time_at_max: 0,
            max_time_at_max: 0,
            random_probability: 0,
            volume_at_zero: 0,
            volume_at_max: 0,
            random_duck_at_zero: 0,
            random_duck_at_max: 0,
            slide: Moveable::default(),
            slide_stop: 0,
            roll: Moveable::default(),
            roll_stop: 0,
            random_splice: None,
            time_to_next_random_splice: 0.0,
        }
    }

    /// `InitSFX` 0x8254edd8 (`mpDynamicObjectID`, which nothing ported reads, aside).
    pub(super) fn init_sfx(&mut self, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.get_attrib_variables(physics, ctx);
    }

    /// `GetAttribVariables` 0x82548b40, from the physics controller's primary key.
    fn get_attrib_variables(&mut self, physics: &Physics, ctx: &SfxCtx<'_>) {
        let vault = ctx.vault;
        let attrib = dynamic_object_or_default(vault, physics.attrib_key);
        let word = |index: u32| vault.layout_word(attrib, index) as i32;
        self.max_speed = vault.layout_f32(attrib, 77);
        self.max_rotational_speed = vault.layout_f32(attrib, 78);
        self.material_id = word(79);
        self.pitch_offset = word(44);
        self.file_name = vault.layout_text(attrib, 18).unwrap_or_default();
        self.get_random_splice_attrib(layout_u64(vault, attrib, 8), ctx);
        self.slide.lfo = SLIDE_WORDS.map(word);
        self.slide.xfades = SLIDE_XFADE_WORDS.map(word);
        self.slide.angle_xfades = SLIDE_ANGLE_XFADE_WORDS.map(word);
        self.slide_stop = word(45);
        self.slide.volumes = SLIDE_VOLUME_WORDS.map(word);
        self.slide.volume_start = word(20);
        self.slide.volume_stop = word(19);
        let attribute = |key: u64| vault.i32(attrib, key);
        self.roll.lfo = ROLL_KEYS.map(attribute);
        self.roll.xfades = ROLL_XFADE_KEYS.map(attribute);
        self.roll.angle_xfades = ROLL_ANGLE_XFADE_KEYS.map(attribute);
        self.roll_stop = attribute(ROLL_STOP_KEY);
        self.roll.volumes = ROLL_VOLUME_KEYS.map(attribute);
        self.roll.volume_start = attribute(ROLL_VOLUME_START_KEY);
        self.roll.volume_stop = attribute(ROLL_VOLUME_STOP_KEY);
    }

    /// `GetRandomSpliceAttrib` 0x825351d0: the `aud_dynamicobjectrandom` collection the
    /// `aud_dynamicobject` names (`RefSpec` at layout 24).
    fn get_random_splice_attrib(&mut self, key: u64, ctx: &SfxCtx<'_>) {
        let vault = ctx.vault;
        let random = Collection::new(AUD_DYNAMICOBJECTRANDOM, key);
        if !vault.has_collection(random) {
            self.random_splice_file_name.clear();
            self.splice_volume = 1.0;
            self.random_splice_cluster = -1;
            self.splice_pitch = 1.0;
            self.min_time_at_zero = 0;
            self.splice_max_speed = 10.0;
            self.max_time_at_zero = 1000;
            self.min_time_at_max = 0;
            self.max_time_at_max = 1000;
            self.random_probability = 100;
            self.volume_at_zero = 0x7FFF;
            self.volume_at_max = 0x7FFF;
            self.random_duck_at_zero = 0;
            self.random_duck_at_max = 0;
            return;
        }
        let word = |index: u32| vault.layout_word(random, index) as i32;
        self.random_splice_file_name = vault.layout_text(random, 0).unwrap_or_default();
        self.random_splice_cluster = word(6);
        self.splice_volume = word(3) as f32;
        self.splice_pitch = word(8) as f32;
        self.splice_max_speed = vault.layout_f32(random, 13);
        self.min_time_at_zero = word(9);
        self.max_time_at_zero = word(11);
        if self.max_time_at_zero <= self.min_time_at_zero {
            self.max_time_at_zero = self.min_time_at_zero + 1;
        }
        self.min_time_at_max = word(10);
        self.max_time_at_max = word(12);
        if self.max_time_at_max <= self.min_time_at_max {
            self.max_time_at_max = self.min_time_at_max + 1;
        }
        self.random_probability = word(7);
        self.volume_at_zero = word(1);
        self.volume_at_max = word(2);
        self.random_duck_at_zero = word(4);
        self.random_duck_at_max = word(5);
    }

    /// The `c_sliding_start` / `c_rolling_start` members (`CreateSlidingStart` 0x825176a0,
    /// `CreateRollingStart` 0x825179f8).
    fn start_data(&self) -> Vec<i32> {
        vec![0x7FFF, 0, 4096, 0, 25000, 0, 0, 0, self.material_id.clamp(0, 1000)]
    }

    /// The `c_moveable_*_objects` members (`CreateSliding` 0x82534370, `CreateRolling`
    /// 0x82534aa0). `nicotine_input_1` takes the first angle crossfade value, as natively.
    fn moveable_data(&self, physics: &Physics, moveable: &Moveable) -> Vec<i32> {
        let speed = fctiwz(clamp_fsel((physics.moving_speed * KPH) / self.max_speed, 1.0) * 1000.0);
        let mut data = vec![0x7FFF, 0, 4096, speed, 25000, 0, 0, 0, self.material_id, 0, 0, 0, 0, moveable.angle_xfades[0], 4096, 4096];
        data.extend_from_slice(&moveable.lfo);
        data.extend_from_slice(&moveable.xfades);
        data.extend_from_slice(&moveable.angle_xfades);
        data.push(self.roll_stop);
        data.push(self.slide_stop);
        data.extend_from_slice(&moveable.volumes);
        data.push(moveable.volume_stop);
        data
    }

    fn create_loop(&mut self, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if physics.sliding {
            let data = self.moveable_data(physics, &self.slide);
            self.sliding = Some(ctx.aems.create_object("c_moveable_sliding_objects", data));
        } else {
            let data = self.moveable_data(physics, &self.roll);
            self.rolling = Some(ctx.aems.create_object("c_moveable_rolling_objects", data));
        }
    }

    /// `StopMovingAudio` 0x825341f8: every class and the random splice released, the swap banks
    /// this object loads emptied.
    fn stop_moving_audio(&mut self, swap: &mut SwapBanks, ctx: &mut SfxCtx<'_>) {
        for object in [self.sliding_start.take(), self.rolling_start.take(), self.sliding.take(), self.rolling.take()].into_iter().flatten() {
            ctx.aems.release_object(object);
        }
        ctx.release(&mut self.random_splice);
        if self.loading[0] || self.data_loaded[0] {
            swap.unload(SWAP_MOVING, ctx);
        }
        if self.loading[1] || self.data_loaded[1] {
            swap.unload(SWAP_RANDOM, ctx);
        }
        self.data_loaded = [false; 2];
        self.loading = [false; 2];
    }

    /// `Destroy` 0x82548a10.
    pub(super) fn destroy(&mut self, swap: &mut SwapBanks, ctx: &mut SfxCtx<'_>) {
        self.stop_moving_audio(swap, ctx);
    }

    /// `UpdateParams` 0x8254ee08: while moved on the ground, the manager follows this object's key
    /// and move type, the start plays once and the banks load; letting go stops everything.
    pub(super) fn update_params(&mut self, physics: &Physics, chains: &mut MoveableChains, swap: &mut SwapBanks, ctx: &mut SfxCtx<'_>) {
        if !(physics.get_is_being_moved(ctx.challenge_type) && physics.on_ground) {
            if self.sliding_start.is_some() || self.rolling_start.is_some() {
                self.stop_moving_audio(swap, ctx);
            }
            return;
        }
        self.get_attrib_variables(physics, ctx);
        chains.set_moveable_object_key(physics.attrib_key, ctx.vault);
        chains.set_move_type(physics.move_type, ctx.core);
        chains.vel_kph = physics.speed * KPH;
        if self.sliding_start.is_none() && self.rolling_start.is_none() {
            let data = self.start_data();
            if physics.sliding {
                self.sliding_start = Some(ctx.aems.create_object("c_sliding_start", data));
            } else {
                self.rolling_start = Some(ctx.aems.create_object("c_rolling_start", data));
            }
            self.data_loaded = [false; 2];
            self.loading = [false; 2];
            if swap.empty(SWAP_MOVING) {
                swap.async_load(format!("data\\audio\\splicerdmo.big|{}", self.file_name).replace('/', "\\"), SWAP_MOVING);
                self.loading[0] = true;
            }
            if !self.random_splice_file_name.is_empty() && swap.empty(SWAP_RANDOM) {
                swap.async_load(self.random_splice_file_name.clone(), SWAP_RANDOM);
                self.loading[1] = true;
            }
        }
        if self.loading[0] && swap.loaded(SWAP_MOVING) {
            self.loading[0] = false;
            self.data_loaded[0] = true;
            self.create_loop(physics, ctx);
        }
        if self.loading[1] && swap.loaded(SWAP_RANDOM) {
            self.loading[1] = false;
            self.data_loaded[1] = true;
            self.time_to_next_random_splice = self.get_time_to_next_random(physics, ctx.rng);
        }
    }

    /// `ProcessUpdate` 0x82548a48.
    pub(super) fn process_update(&mut self, attached: bool, physics: &Physics, chains: &MoveableChains, ctx: &mut SfxCtx<'_>) {
        if !attached {
            return;
        }
        if let Some(mut object) = self.sliding_start.take() {
            self.update_start(&mut object.data, self.slide.volume_start, ctx);
            ctx.aems.set_member_data(&object);
            self.sliding_start = Some(object);
        }
        if let Some(mut object) = self.rolling_start.take() {
            self.update_start(&mut object.data, self.roll.volume_start, ctx);
            ctx.aems.set_member_data(&object);
            self.rolling_start = Some(object);
        }
        if self.data_loaded[0] {
            if let Some(mut object) = self.sliding.take() {
                self.update_loop(&mut object.data, physics, chains, ctx);
                ctx.aems.set_member_data(&object);
                self.sliding = Some(object);
            }
            if let Some(mut object) = self.rolling.take() {
                self.update_loop(&mut object.data, physics, chains, ctx);
                ctx.aems.set_member_data(&object);
                self.rolling = Some(object);
            }
        }
        self.update_random_samples(physics, ctx);
    }

    /// `UpdateSlidingStart` 0x82517778 / `UpdateRollingStart` 0x82517ad0.
    fn update_start(&self, data: &mut [i32], start_volume: i32, ctx: &SfxCtx<'_>) {
        let io = self.io;
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        let volume = ctx.output(io, 1, PresetType::Volume);
        let wet = ctx.output(io, 5, PresetType::Volume);
        let pitch = ctx.output(io, 2, PresetType::Pitch);
        let low_pass = ctx.output(io, 4, PresetType::Frequency);
        let high_pass = ctx.output(io, 3, PresetType::Frequency);
        let pitch = pitch - fctiwz((self.pitch_offset as f32 * pitch as f32) * -PITCH_SCALE);
        let dry = fctiwz((start_volume as f32 * volume as f32) * VOLUME_SCALE);
        data[1] = azimuth.clamp(0, 0xFFFF);
        data[7] = dry.clamp(0, 0x7FFF);
        data[6] = wet.clamp(0, 0x7FFF);
        data[2] = pitch.clamp(0, 0x2000);
        data[4] = low_pass.clamp(0, 25001);
        data[5] = high_pass.clamp(0, 25001);
    }

    /// `UpdateSliding` 0x82534660 / `UpdateRolling` 0x82534d90: speed and push angle from the
    /// smoothed and rotational speeds, the mix from outputs 6-11, the pitch and the second
    /// voice's pitch (`mod_pitch_value`) ducked by the manager.
    fn update_loop(&self, data: &mut [i32], physics: &Physics, chains: &MoveableChains, ctx: &SfxCtx<'_>) {
        let speed = fctiwz(clamp_fsel((physics.moving_speed * KPH) / self.max_speed, 1.0) * 1000.0);
        let push_angle = fctiwz(clamp_fsel((physics.rotational_speed * KPH) / self.max_rotational_speed, 1.0) * 1024.0);
        data[3] = speed.clamp(0, 1000);
        let io = self.io;
        let azimuth = ctx.output(io, 6, PresetType::Azimuth);
        let volume = ctx.output(io, 7, PresetType::Volume);
        let wet = ctx.output(io, 11, PresetType::Volume);
        let pitch = ctx.output(io, 8, PresetType::Pitch);
        let low_pass = ctx.output(io, 10, PresetType::Frequency);
        let high_pass = ctx.output(io, 9, PresetType::Frequency);
        let offset = self.pitch_offset - chains.pitch_duck();
        let dry = fctiwz(physics.primary_volume * volume as f32);
        let pitch = pitch - fctiwz((offset as f32 * pitch as f32) * -PITCH_SCALE);
        data[1] = azimuth.clamp(0, 0xFFFF);
        data[7] = dry.clamp(0, 0x7FFF);
        data[6] = wet.clamp(0, 0x7FFF);
        data[2] = pitch.clamp(0, 0x2000);
        data[4] = low_pass.clamp(0, 25001);
        data[5] = high_pass.clamp(0, 25001);
        data[10] = push_angle.clamp(0, 1024);
        let mod_pitch = ctx.output(io, 8, PresetType::Pitch);
        let offset = self.pitch_offset - chains.chain2_pitch_duck();
        data[15] = (mod_pitch - fctiwz((offset as f32 * mod_pitch as f32) * -PITCH_SCALE)).clamp(0, 0x2000);
    }

    /// `GetTimeToNextRandom` 0x82517d50: a random wait between the time ranges at rest and at
    /// `mSpliceMaxSpeed`, in seconds.
    fn get_time_to_next_random(&self, physics: &Physics, rng: &mut RwRandom) -> f32 {
        let ratio = clamp_fsel((physics.moving_speed * KPH) / self.splice_max_speed, 1.0);
        let low = fctiwz((self.min_time_at_max as f32 - self.min_time_at_zero as f32).mul_add(ratio, self.min_time_at_zero as f32));
        let mut high = fctiwz((self.max_time_at_max as f32 - self.max_time_at_zero as f32).mul_add(ratio, self.max_time_at_zero as f32));
        if high <= low {
            high = low + 1;
        }
        let wait = (rng.next_u32() % high.wrapping_sub(low) as u32) as i32 + low;
        wait as f32 * 0.001
    }

    /// `GetRandomVolume` 0x82517e88: the speed's volume less a random duck.
    fn get_random_volume(&self, physics: &Physics, rng: &mut RwRandom) -> f32 {
        let ratio = clamp_fsel((physics.moving_speed * KPH) / self.splice_max_speed, 1.0);
        let volume = fctiwz((self.volume_at_max as f32 - self.volume_at_zero as f32).mul_add(ratio, self.volume_at_zero as f32));
        let mut duck = fctiwz((self.random_duck_at_max as f32 - self.random_duck_at_zero as f32).mul_add(ratio, self.random_duck_at_zero as f32)) as u32;
        if duck == 0 {
            duck = 1;
        }
        let mut level = volume.wrapping_sub((rng.next_u32() % duck) as i32);
        if level < 0 {
            level = 0;
        }
        level as f32 * VOLUME_SCALE
    }

    /// `UpdateRandomSamples` 0x825353a0: once the random splice bank is in, a splice from
    /// `mRandomSpliceCluster` every random wait with `mRandomProbability` percent, panned and
    /// mixed by outputs 6, 12 and 13.
    fn update_random_samples(&mut self, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if !self.data_loaded[1] {
            return;
        }
        if let Some(splice) = self.random_splice {
            if ctx.is_playing(splice) {
                let io = self.io;
                let azimuth = ctx.output(io, 6, PresetType::Azimuth) as f32 * AZIMUTH_SCALE;
                let volume = (self.splice_volume * VOLUME_SCALE) * (ctx.output(io, 12, PresetType::Volume) as f32 * VOLUME_SCALE);
                let volume = self.get_random_volume(physics, ctx.rng) * volume;
                let pitch = ctx.output(io, 13, PresetType::Pitch) as f32;
                let pitch = (self.splice_pitch * PITCH_SCALE) * (pitch * PITCH_SCALE);
                let update = SpliceUpdateParams { vol: volume, pitch, az: azimuth, delta: ctx.delta_time, spread: 0.0, time_stretch: 1.0 };
                ctx.update_splice(splice, update);
                return;
            }
            ctx.release(&mut self.random_splice);
            self.time_to_next_random_splice = self.get_time_to_next_random(physics, ctx.rng);
            return;
        }
        if self.time_to_next_random_splice > 0.0 {
            self.time_to_next_random_splice -= ctx.delta_time;
            return;
        }
        if (ctx.rng.next_u32() % 100) as i32 >= self.random_probability {
            self.time_to_next_random_splice = self.get_time_to_next_random(physics, ctx.rng);
            return;
        }
        if self.random_splice_cluster >= 0 {
            self.random_splice = ctx.new_splice(SPLICE_DMO_RANDOM as i32, self.random_splice_cluster, ctx.master_sub_mix);
        }
    }
}
