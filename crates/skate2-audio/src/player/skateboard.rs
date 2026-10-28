//! `Sk8::Audio::SFXObj_SkateBoard` (1872 bytes, PLAYER object 0): the board's rolling sound.
//!
//! Two wheel slots roll (`mFrontLeft` and the other one; only the local skater's state runs the
//! second). Each slot plays its surface: plywood, the asphalts and concretes, the wood ramp,
//! aggregate and metal through two `GrainPlayer::AudBoard`s (rolling, and carving) and their
//! RWAC voices, the other surfaces through AEMS. The pushes, foot braking, turning, the surface
//! pattern's ducking, the sense-of-speed distortion and volume modulation shape the grains.
//!
//! The AEMS instances (rolling, rattle, squeaks, skid, grit, rocks, spider cracks, board slide)
//! run through `crate::aems`. The stage-2 voice sends to `GlobalEffects`' reverb and flange 0, and
//! `ProcessUpdate` points both flanges at the board's azimuth. The non-local whoosh
//! (`CheckForWhoosh`) is not ported.

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp, clippy::neg_multiply)]

use super::physics::{PlayerPhysics, WHEEL_SOFT};
use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::aems::ClassObject;
use crate::grain_player::{AudBoard, GrainData, GrainParams, UpdateParams};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::random::RwRandom;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::surface::{power_slide_material, rolling_material};
use crate::utils::{clamp_fsel, fsel, PathLine, Slope, SlopeParams, CURVE_LINEAR};
use crate::vault::{class, Collection, Vault};
use crate::vmx;
use std::collections::HashMap;

/// The `.grain` files `AudioAssetManager::SyncLoad` finds, by lower-case name.
pub type GrainFiles = HashMap<String, GrainData>;

const DEFAULT: u64 = 0xD7ED_BD36_2D7D_2152;
/// `GlobalAttributeCollection`'s `aud_skateboard_default` (+0x04), `aud_cracks_default` (+0x1C),
/// `aud_rails_default` (+0x40), `aud_general_wheelslide` (+0x70), `aud_senseofspeed_default`
/// (+0x8C), and the `aud_general` collection `CalcAzimRate` finds.
const SKATEBOARD_DEFAULT: Collection = Collection::new(class::AUD_SKATEBOARD, DEFAULT);
const CRACKS_DEFAULT: Collection = Collection::new(class::AUD_CRACKS, DEFAULT);
const RAILS_DEFAULT: Collection = Collection::new(class::AUD_RAILS, DEFAULT);
const WHEELSLIDE: Collection = Collection::new(class::AUD_GENERAL, 0xBA98_37A6_CF4C_26ED);
const SENSE_OF_SPEED: Collection = Collection::new(class::AUD_SENSEOFSPEED, DEFAULT);
/// `GlobalAttributeCollection`'s `aud_general_aems_rolling` (+0x50) and `aud_general_boardslide`
/// (+0x7C).
const AEMS_ROLLING: Collection = Collection::new(class::AUD_GENERAL, 0x7B0E_D922_C779_B74C);
const BOARDSLIDE: Collection = Collection::new(class::AUD_GENERAL, 0x6210_9062_0F4F_936A);
const AZIM_RATE: Collection = Collection::new(class::AUD_GENERAL, 0x1C20_B475_CE21_8459);

/// `aud_skateboard` attributes.
const GRAIN_PARAMS: u64 = 0xD18D_1174_735E_5CDE;
const FOOT_BREAK_SLOPE: u64 = 0x7473_E140_345A_1A57;
const SURFACE_OVERRIDE: u64 = 0x86E3_6319_FEC6_9C47;
const WHEEL_TYPE_OVERRIDE: u64 = 0x739F_4795_E242_51FD;
const TURN_RISE: u64 = 0x281F_F010_8147_5899;
const TURN_FALL: u64 = 0x6376_4B8C_7EB9_EC9B;
const MAX_BOWL_COMPRESSION: u64 = 0x57A7_8D3B_E8D4_7BB3;
const MAX_HILL_COMPRESSION: u64 = 0x8DD4_C3FC_8DAF_4059;
const MANUAL_VOLUME: u64 = 0x6BDC_44AE_7C3C_79D0;
const MANUAL_FREQUENCY_SHIFT: u64 = 0xF62B_C5EB_D8E5_DDE8;
const GFORCE_CARVE_FREQUENCY_SHIFT: u64 = 0x7FFF_3A8A_D448_09EF;
const PUSH_MAX_SPEED: u64 = 0x2D75_1DEB_89BB_5E33;
const PUSH_SCALE_MIN: u64 = 0xE239_B03F_0E89_0686;
const PUSH_SCALE_MAX: u64 = 0xB87E_CDDA_AB0F_8404;
const PUSH_SCALE_TIMES: [u64; 3] = [0xB3D7_4688_20AF_C661, 0xDAC9_DA91_0EF0_316C, 0x0C3D_5DBC_262E_D276];
const PUSH_FREQUENCY_MIN: u64 = 0xC658_A792_3FC7_B99E;
const PUSH_FREQUENCY_MAX: u64 = 0xA15A_D56E_225A_DBA6;
const PUSH_FREQUENCY_TIMES: [u64; 3] = [0x09A5_CC79_BA21_78E7, 0x3206_FD96_427E_A4D2, 0xDB59_7F67_2CA4_7138];
/// `aud_skateboard` layout words: `Speed_Ramp`'s second column, `GrainFile`, `MaxSpeed`,
/// `GForce_Volume_Mult`, `GForce_Slow_Down_Velocity`, `GForce_Frequency_Shift`,
/// `Carve_MaxTurn`, `Carve_FrequencyShift`.
const SPEED_RAMP: [u32; 4] = [1, 5, 9, 13];
const GRAIN_FILE: u32 = 16;
const MAX_SPEED: u32 = 17;
const GFORCE_VOLUME_MULT: u32 = 18;
const GFORCE_SLOW_DOWN_VELOCITY: u32 = 19;
const GFORCE_FREQUENCY_SHIFT: u32 = 20;
const CARVE_MAX_TURN: u32 = 21;
const CARVE_FREQUENCY_SHIFT: u32 = 22;
/// `aud_cracks`.
const PATTERN_OVERRIDE: u64 = 0x2314_D2C0_724A_C82F;
const DUCKER_MIN: u64 = 0xFA3A_5780_1765_A2F8;
const DUCKER_MAX: u64 = 0x32A9_692F_1B82_6274;
const DUCKER_TIME_MIN: u64 = 0xF713_CB54_7B1D_F920;
const DUCKER_TIME_MAX: u64 = 0x0608_B312_9FF8_1F12;
/// `aud_rails` and `aud_general`.
const GRIND_TYPE_OVERRIDE: u64 = 0x6710_834A_D5E0_847F;
const POWERSLIDE_OVERRIDE: u64 = 0x7016_8C56_1A47_F5F2;
const AZIM_RATE_MAX: u64 = 0x780F_5C81_6E00_BDFC;
const AZIM_RATE_SLEW: u64 = 0x6875_6CFE_B1FF_3428;
/// `aud_general`: `MaxRollingSpeed[]` (aems_rolling); the wheel slide's `EnterVolume`,
/// `ExitVolume` and `Debug_PowerslideTestWheels`; the board slide's maximum speed and distance
/// and its volumes (the second for `eBoardSlideType` 2).
const MAX_ROLLING_SPEED: u64 = 0x880C_82E8_EF64_7EC4;
const ENTER_VOLUME: u64 = 0xFB10_048C_CDD6_ADFA;
const EXIT_VOLUME: u64 = 0x8CE4_2E5A_9388_A4C8;
const POWERSLIDE_TEST_WHEELS: u64 = 0x8FAE_F7A3_DDF3_8313;
const BOARDSLIDE_MAX_SPEED: u64 = 0x9635_B780_C747_2A6E;
const BOARDSLIDE_MAX_DISTANCE: u64 = 0xCCF6_B170_72A4_B410;
const BOARDSLIDE_VOLUME: u64 = 0x662C_EE73_D2E3_FE2F;
const BOARDSLIDE_VOLUME_TYPE_2: u64 = 0xAB87_C3D1_EDDD_DCBC;
/// `aud_skateboard`: `Rattle_MaxSpeed` (per surface), the squeaks switch (an inline bool),
/// `Squeak_Min_Angle` and `Squeak_Max_Angular_Vel` (default collection).
const RATTLE_MAX_SPEED: u64 = 0x1227_5AA8_AC4A_63FB;
const PLAY_SQUEAKS: u64 = 0x9B94_B7D6_6F8D_E44B;
const SQUEAK_MIN_ANGLE: u64 = 0xA129_B33B_4A2C_7961;
const SQUEAK_MAX_ANGULAR_VEL: u64 = 0xAC87_D592_E760_1134;
/// `aud_senseofspeed`, in `UpdateAttribValues` order.
const DISTORTION_KEYS: [u64; 9] = [
    0x0D66_5393_E2ED_C605,
    0x28E7_0878_2445_747F,
    0xE64C_04ED_542D_ABC8,
    0xD900_C07B_E7C5_450F,
    0x88AA_96B0_8FD1_6914,
    0x3FFB_5107_C82B_A3E0,
    0xD3E8_894C_A25A_4F71,
    0x55BE_B303_53F2_44A9,
    0x4551_6395_725E_D16B,
];
const MODULATION_MIN_SPEED: u64 = 0x281A_501B_22B6_CCDF;
const MODULATION_MAX_SPEED: u64 = 0x54CD_E019_E31F_C04E;
/// Per modulation: min time, max time, min duck, max duck.
const MODULATION_KEYS: [[u64; 4]; 2] = [
    [0x36AE_4181_7640_FE04, 0x71EE_2731_3BD3_0F21, 0x437D_128B_5366_9C34, 0x0288_5338_DD5D_7DCA],
    [0x2055_BBF3_9C15_2FA9, 0xF524_0AFA_DA3B_3FFC, 0xF916_E153_393C_5F24, 0x0A36_F907_3201_6D85],
];

/// `eSurfaceTypes`.
const SURFACE_SMOOTH_ASPHALT: i32 = 3;
const SURFACE_METAL: i32 = 9;
const SURFACE_NONE: i32 = 14;
/// `eRollingType`.
const ROLLING_AEMS: i32 = 0;
const ROLLING_GRAIN: i32 = 1;
/// `eSurfacePatternType`: the spider-cracked pavement `CheckForSpiderCracks` listens for.
const PATTERN_SPIDER_CRACKS: i32 = 1;
/// `eBoardSlideType` 2, whose volume is `BOARDSLIDE_VOLUME_TYPE_2`.
const BOARD_SLIDE_TYPE_2: u32 = 2;
/// `eAudioMaterialTypes`: materials past the mapping roll as smooth asphalt.
const MATERIAL_MAX: u32 = 132;
/// `Sk8::Physics::eGrindType` value `IsWheelSlide` hears on rails.
const GRIND_WHEEL_SLIDE: i32 = 4;
/// `ePSMaterial` below which a wheel slide creates its skid.
const PS_GRASS: u32 = 3;

/// `LoadDataForSurface`'s calls in the constructor, last to first wins `mSkateboardAttrib[0]`.
const SURFACES: [(u64, usize, usize); 13] = [
    (0x7EB8_015B_4C02_405E, 0, 1),
    (0x943B_1CB0_5BA9_A6BA, 0xE, 0xF),
    (0x7C59_12FC_2DAB_F98C, 4, 5),
    (0xDC10_09D5_0E32_7F8F, 0x12, 0x13),
    (0xB303_AED8_2415_30E2, 0xA, 0xB),
    (0x863C_58AC_34BA_D599, 0x18, 0x19),
    (0x0372_1D0F_A99A_03C8, 2, 3),
    (0xB29F_ADBB_BC2F_39C2, 0x10, 0x11),
    (0xFFB5_E3E6_2E0B_4943, 6, 7),
    (0x607A_6BC3_D427_DA49, 0x14, 0x15),
    (0x1C9C_52CC_0E1C_D4CF, 0xC, 0xD),
    (0x7947_A259_F181_FDB4, 8, 9),
    (0x382B_1263_6ED9_D8DA, 0x16, 0x17),
];

/// The plug-ins of `CreateSkateBoardVoice`'s voices.
const MAIN_HIGH_PASS: u8 = 1;
const MAIN_LOW_PASS: u8 = 2;
const MAIN_FREQUENCY_SHIFT: u8 = 3;
const MAIN_DISTORTION_SEND: u8 = 4;
const MAIN_GAIN: u8 = 5;
const MAIN_SEND: u8 = 6;
const STAGE2_FLANGE_SEND: u8 = 1;
const STAGE2_REVERB_SEND: u8 = 2;
const STAGE2_PAN: u8 = 3;
const STAGE2_SEND: u8 = 4;
const DISTORTION_CLIP: u8 = 1;
const DISTORTION_GAIN: u8 = 2;
const DISTORTION_HIGH_SHELF: u8 = 3;
const DISTORTION_SEND: u8 = 4;

/// `AudioSystem::SurfacePatternToKey` 0x824e6858: the `aud_cracks` collection of a pattern.
fn surface_pattern_key(pattern: i32) -> u64 {
    match pattern {
        1 => 0x705F_4C5A_AB64_3C59,
        2 => 0xCF0A_7EF7_2038_3F8F,
        3 => 0xD540_C7D9_629A_252A,
        4 => 0x1F01_AA5A_A4AB_B4F0,
        5 => 0x67BB_1C22_E71E_BD51,
        6 => 0x845B_649A_3B15_53F6,
        7 => 0x5A4B_A8B8_D554_9E6A,
        8 => 0x1A11_7689_CE5E_6086,
        9 => 0x1C7B_E670_7F0A_CB78,
        10 => 0xF69A_81BF_FE8F_DBB6,
        11 => 0x216A_369A_0184_462D,
        12 => 0x6557_4DE6_9270_5EFB,
        13 => 0x48F5_F7BC_62B4_62B1,
        14 => 0x2BC6_909B_0699_FB8A,
        15 => 0x136F_34A2_0B20_5537,
        _ => 0,
    }
}

/// `fnmsubs`: -(a * c - b) with one rounding.
fn fnmsubs(a: f32, c: f32, b: f32) -> f32 {
    -a.mul_add(c, -b)
}

/// `rw::math::Random() % divisor` (`divwu`); the native `twllei` traps on a zero divisor.
fn random_below(rng: &mut RwRandom, divisor: u32) -> u32 {
    rng.next_u32().checked_rem(divisor).unwrap_or(0)
}

/// `tSkateBoardVoices`: the voice, its stage-2 voice and the local skater's distortion voice.
#[derive(Debug, Clone, Copy, Default)]
struct SkateBoardVoices {
    main: Option<VoiceId>,
    stage2: Option<VoiceId>,
    distortion: Option<VoiceId>,
}

impl SkateBoardVoices {
    /// `FreeVoices` 0x82522498.
    fn free(&mut self, core: &mut AudioCore) {
        for voice in [self.main, self.stage2, self.distortion].into_iter().flatten() {
            core.release_voice(voice);
        }
        *self = Self::default();
    }
}

fn plugin(voice: Option<VoiceId>, index: u8) -> Option<PluginRef> {
    voice.map(|voice| PluginRef { voice, index })
}

fn set_attribute(core: &mut AudioCore, plugin: Option<PluginRef>, index: usize, value: f32) {
    if let Some(plugin) = plugin {
        core.set_attribute(plugin, index, value);
    }
}

/// `tModulationValues`.
#[derive(Debug, Clone, Copy, Default)]
struct ModulationValues {
    min_time: i32,
    max_time: i32,
    min_duck: f32,
    max_duck: f32,
    cur_target: f32,
    cur_value: f32,
    cur_last_value: f32,
    cur_unducked: f32,
    scale: PathLine,
}

pub struct SkateBoard {
    /// `m_pDMixIO`.
    pub io: Option<DMixIoId>,
    /// The id of `mBoardGrainPlayer[0][0]`'s timer; the others follow.
    timer_base: u32,
    /// `pBoardData`: the grain files `LoadDataForSurface` loaded, by `eSkateBoardSFX`.
    board_data: [Option<GrainData>; 26],
    /// `mSkateboardAttrib[0..2]` (the slots the code changes).
    attrib: [Collection; 2],
    surface_type: [i32; 2],
    push_scale: PathLine,
    push_frequency: PathLine,
    last_turn_intensity: f32,
    turn_intensity: f32,
    breaking_intensity: f32,
    slope_foot_break: Slope,
    /// `mBoardGrainPlayer`, allocated by `InitSFX` and deleted by `Destroy`.
    grain_players: Option<[[AudBoard; 2]; 2]>,
    voices: [[SkateBoardVoices; 2]; 2],
    /// `mpWheelSkid`, `mpSqueaks` and `mPrevSign`, `mpRattle`, `mpGrit`, `mpRocks`,
    /// `mpRolling[2]` with `mAEMSRollingID[2]`, `mpSpiderCracks`, `mpBoardSlide`.
    wheel_skid: Option<ClassObject>,
    squeaks: Option<ClassObject>,
    prev_sign: bool,
    rattle: Option<ClassObject>,
    grit: Option<ClassObject>,
    rocks: Option<ClassObject>,
    rolling: [Option<ClassObject>; 2],
    aems_rolling_id: [i32; 2],
    spider_cracks: Option<ClassObject>,
    board_slide: Option<ClassObject>,
    /// `mSurfaceKey`: the `aud_skateboard` collection of the last surface change.
    surface_key: u64,
    current_rolling_type: [i32; 2],
    grains_initialized: [bool; 2],
    current_surface_pattern: i32,
    pattern_scale: PathLine,
    ducking_for_surface_pattern: bool,
    ducking_target: f32,
    volume_ducker_min: f32,
    volume_ducker_max: f32,
    volume_ducker_min_time: i32,
    volume_ducker_max_time: i32,
    is_rolling: [bool; 2],
    front_left: usize,
    was_in_manual: bool,
    gforce_bowl: f32,
    gforce_hill: f32,
    add_to_angle: i32,
    /// `mDistortionMinSpeed` .. `mDistortionHighShelfGain`, in `DISTORTION_KEYS` order.
    distortion: [f32; 9],
    current_distortion: f32,
    current_distortion_duck_multiplier: f32,
    modulation_min_speed: f32,
    modulation_max_speed: f32,
    modulation: [ModulationValues; 2],
    cur_azim: i32,
    last_azim: i32,
    last_azim_rate: f32,
}

impl SkateBoard {
    /// The constructor 0x825427f0 (`CreateObject` 0x8254c880): each surface's grain file
    /// (`LoadDataForSurface` 0x825225d8 changes `mSkateboardAttrib[mFrontLeft]` to the surface and
    /// loads its `GrainFile` into both of its slots), then `UpdateAttribValues`.
    #[must_use]
    pub fn new(vault: &Vault, grains: &GrainFiles, timer_base: u32) -> Self {
        let mut board = Self {
            io: None,
            timer_base,
            board_data: std::array::from_fn(|_| None),
            attrib: [Collection::new(class::AUD_SKATEBOARD, 0); 2],
            surface_type: [0; 2],
            push_scale: PathLine::default(),
            push_frequency: PathLine::default(),
            last_turn_intensity: 0.0,
            turn_intensity: 0.0,
            breaking_intensity: 0.0,
            slope_foot_break: Slope::default(),
            grain_players: None,
            voices: [[SkateBoardVoices::default(); 2]; 2],
            wheel_skid: None,
            squeaks: None,
            prev_sign: false,
            rattle: None,
            grit: None,
            rocks: None,
            rolling: [None, None],
            aems_rolling_id: [0; 2],
            spider_cracks: None,
            board_slide: None,
            surface_key: DEFAULT,
            current_rolling_type: [0; 2],
            grains_initialized: [false; 2],
            current_surface_pattern: 0,
            pattern_scale: PathLine::default(),
            ducking_for_surface_pattern: false,
            ducking_target: 0.0,
            volume_ducker_min: 0.0,
            volume_ducker_max: 0.0,
            volume_ducker_min_time: 0,
            volume_ducker_max_time: 0,
            is_rolling: [false; 2],
            front_left: 0,
            was_in_manual: false,
            gforce_bowl: 0.0,
            gforce_hill: 0.0,
            add_to_angle: 0,
            distortion: [0.0; 9],
            current_distortion: 0.0,
            current_distortion_duck_multiplier: 1.0,
            modulation_min_speed: 0.0,
            modulation_max_speed: 0.0,
            modulation: [ModulationValues::default(); 2],
            cur_azim: 0,
            last_azim: 0,
            last_azim_rate: 0.0,
        };
        for (key, first, second) in SURFACES {
            board.attrib[board.front_left] = Collection::new(class::AUD_SKATEBOARD, key);
            let data = vault
                .layout_text(board.attrib[board.front_left], GRAIN_FILE)
                .and_then(|name| grains.get(&name.to_lowercase()).cloned());
            board.board_data[first].clone_from(&data);
            board.board_data[second] = data;
        }
        board.update_attrib_values(vault);
        board
    }

    /// `UpdateAttribValues` 0x82525270 (`aud_senseofspeed` "default").
    fn update_attrib_values(&mut self, vault: &Vault) {
        for (value, key) in self.distortion.iter_mut().zip(DISTORTION_KEYS) {
            *value = vault.f32(SENSE_OF_SPEED, key);
        }
        self.modulation_min_speed = vault.f32(SENSE_OF_SPEED, MODULATION_MIN_SPEED);
        self.modulation_max_speed = vault.f32(SENSE_OF_SPEED, MODULATION_MAX_SPEED);
        for (modulation, keys) in self.modulation.iter_mut().zip(MODULATION_KEYS) {
            modulation.min_time = vault.i32(SENSE_OF_SPEED, keys[0]);
            modulation.max_time = vault.i32(SENSE_OF_SPEED, keys[1]);
            modulation.min_duck = vault.f32(SENSE_OF_SPEED, keys[2]);
            modulation.max_duck = vault.f32(SENSE_OF_SPEED, keys[3]);
        }
    }

    fn layout(&self, vault: &Vault, slot: usize, word: u32) -> f32 {
        vault.layout_f32(self.attrib[slot], word)
    }

    fn attr(&self, vault: &Vault, slot: usize, key: u64) -> f32 {
        vault.f32(self.attrib[slot], key)
    }

    /// `InitSFX` 0x82542e78, which `SetupLoadData` runs on attach: four new grain players, no
    /// surface, the default surface key, the push paths cleared, the local skater's grit and
    /// rocks started, pattern and modulation cleared.
    pub fn init_sfx(&mut self, physics: &PlayerPhysics, local: bool, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.grain_players = Some(std::array::from_fn(|_| std::array::from_fn(|_| AudBoard::new())));
        self.surface_type = [SURFACE_NONE; 2];
        self.current_rolling_type = [ROLLING_GRAIN; 2];
        self.turn_intensity = 0.0;
        self.surface_key = DEFAULT;
        self.last_turn_intensity = 0.0;
        self.breaking_intensity = 0.0;
        self.push_scale.clear_stages();
        self.push_frequency.clear_stages();
        if local {
            self.start_active_sounds(physics, ctx);
        }
        self.current_surface_pattern = 0;
        self.pattern_scale.clear_stages();
        self.ducking_for_surface_pattern = false;
        self.init_volume_modulation();
        self.current_distortion = 0.0;
        self.current_distortion_duck_multiplier = 1.0;
    }

    /// `Destroy` 0x82542c28, which `Detach` runs: each slot's grains, voices and AEMS rolling,
    /// then the squeaks, skid, rattle, grit, rocks, board slide and spider cracks.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for slot in 0..2 {
            if self.grains_initialized[slot] {
                if let Some(players) = &mut self.grain_players {
                    for player in &mut players[slot] {
                        player.destroy(ctx.core);
                    }
                }
                self.grains_initialized[slot] = false;
            }
            for voices in &mut self.voices[slot] {
                voices.free(ctx.core);
            }
            if let Some(rolling) = self.rolling[slot].take() {
                ctx.aems.release_object(rolling);
            }
            self.is_rolling[slot] = false;
        }
        self.grain_players = None;
        let objects = [
            self.squeaks.take(),
            self.wheel_skid.take(),
            self.rattle.take(),
            self.grit.take(),
            self.rocks.take(),
            self.board_slide.take(),
            self.spider_cracks.take(),
        ];
        for object in objects.into_iter().flatten() {
            ctx.aems.release_object(object);
        }
    }

    /// `InitVolumeModulation` 0x82514c78.
    fn init_volume_modulation(&mut self) {
        for modulation in &mut self.modulation {
            modulation.scale.clear_stages();
            modulation.cur_target = 0.0;
            modulation.cur_value = 1.0;
            modulation.cur_last_value = 1.0;
            modulation.cur_unducked = 1.0;
        }
    }

    /// The `RwAudioCoreTimer` 0x829c9d30 of grain player `index` (slot * 2 + type).
    pub fn grain_timer(&mut self, index: usize, core: &mut AudioCore, rng: &mut RwRandom, period: f32) {
        if let Some(players) = &mut self.grain_players {
            players[index / 2][index % 2].process(core, rng, period);
        }
    }

    /// The inlined `mbWasInManual` update: set in a manual, cleared once every wheel is down or up.
    fn in_manual(&mut self, physics: &PlayerPhysics) -> bool {
        if physics.is_manual {
            self.was_in_manual = true;
        } else if self.was_in_manual && matches!(physics.num_wheels_on_ground, 0 | 4) {
            self.was_in_manual = false;
        }
        self.was_in_manual
    }

    /// `UpdateParams` 0x8254cff8. `CheckForWhoosh` (the whoosh-by of skaters other than the local
    /// one) is not ported.
    pub fn update_params(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.update_gforce(physics, ctx);
        self.update_surface_type(physics, ctx);
        self.update_rolling(t, physics, ctx);
        self.update_voices(physics, ctx);
        self.check_for_skids(physics, ctx);
        self.check_for_squeaks(physics, ctx);
        self.check_for_spider_cracks(physics, ctx);
        self.check_for_pattern_scaling(t, physics, ctx);
        self.update_distortion(physics, ctx);
        self.update_volume_modulation(t, physics, ctx);
        self.check_for_board_slide(physics, ctx);
        self.calc_azim_rate(t, ctx);
    }

    /// `UpdateGForce` 0x82514ac0: the compression against the bowl (negative) or hill (positive)
    /// maximum, into inputs 2 and 3.
    fn update_gforce(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.gforce_bowl = 0.0;
        self.gforce_hill = 0.0;
        if self.current_rolling_type[self.front_left] == ROLLING_GRAIN {
            let compression = physics.compression;
            if !(compression >= 0.0) {
                let max = self.attr(ctx.vault, self.front_left, MAX_BOWL_COMPRESSION);
                self.gforce_bowl = clamp_fsel(compression / max, 1.0);
            } else if !(compression <= 0.0) {
                let max = self.attr(ctx.vault, self.front_left, MAX_HILL_COMPRESSION);
                self.gforce_hill = clamp_fsel(compression / max, 1.0);
            }
        }
        ctx.set_input(self.io, 2, fctiwz(self.gforce_bowl * 32767.0).clamp(0, 0x7FFF));
        ctx.set_input(self.io, 3, fctiwz(self.gforce_hill * 32767.0).clamp(0, 0x7FFF));
    }

    /// `GetWheelType` 0x825251b8.
    fn wheel_type(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> u32 {
        match ctx.vault.i32(SKATEBOARD_DEFAULT, WHEEL_TYPE_OVERRIDE) {
            0 => 0,
            1 => 1,
            _ if ctx.local => physics.wheel_hardness,
            _ => ctx.npc_wheel_type,
        }
    }

    /// `GetSurfaceType` 0x825235c0: none for a lifted wheel in a manual or on a rail; the
    /// surface override; the wheel's rolling material (front: wheel 0, back: wheel 3).
    fn surface_type_of(&mut self, wheel: usize, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> i32 {
        if ctx.local && self.in_manual(physics) {
            let landed = if wheel == self.front_left { physics.landed[0] } else { physics.landed[3] };
            if !landed {
                return SURFACE_NONE;
            }
        }
        if physics.is_rail_grind {
            return SURFACE_NONE;
        }
        let forced = ctx.vault.i32(self.attrib[self.front_left], SURFACE_OVERRIDE);
        if forced != -1 {
            return forced;
        }
        let material = if wheel == self.front_left { physics.wheel_material[0] } else { physics.wheel_material[3] };
        if material >= MATERIAL_MAX {
            return SURFACE_SMOOTH_ASPHALT;
        }
        rolling_material(ctx.vault, material) as i32
    }

    /// `GetSurfaceKey` 0x825438c0: the surface's `aud_skateboard` collection for the wheel type
    /// and its two `pBoardData` slots (0 for surfaces without a grain).
    fn surface_key(surface: i32, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> (u64, usize, usize) {
        let soft = Self::wheel_type(physics, ctx) == WHEEL_SOFT;
        match (surface, soft) {
            (1, true) => (0x943B_1CB0_5BA9_A6BA, 0xE, 0xF),
            (1, false) => (0x7EB8_015B_4C02_405E, 0, 1),
            (2, true) => (0xB29F_ADBB_BC2F_39C2, 0x10, 0x11),
            (2, false) => (0x0372_1D0F_A99A_03C8, 2, 3),
            (3, true) => (0xDC10_09D5_0E32_7F8F, 0x12, 0x13),
            (3, false) => (0x7C59_12FC_2DAB_F98C, 4, 5),
            (4, true) => (0x607A_6BC3_D427_DA49, 0x14, 0x15),
            (4, false) => (0xFFB5_E3E6_2E0B_4943, 6, 7),
            (5, true) => (0x382B_1263_6ED9_D8DA, 0x16, 0x17),
            (5, false) => (0x7947_A259_F181_FDB4, 8, 9),
            (6, true) => (0x863C_58AC_34BA_D599, 0x18, 0x19),
            (6, false) => (0xB303_AED8_2415_30E2, 0xA, 0xB),
            (9, _) => (0x1C9C_52CC_0E1C_D4CF, 0xC, 0xD),
            _ => (DEFAULT, 0, 0),
        }
    }

    /// `UpdateSurfaceType` 0x8254ca10. Per wheel slot (the second only for the local skater), a
    /// new surface replaces the slot's sound; input 0 pulses on a change away from a surface.
    /// The local skater hands a change to the other slot when that one is idle. Input 6 is
    /// whether wheel 0 rolls on metal.
    fn update_surface_type(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        ctx.set_input(self.io, 0, 0);
        let mut pass = 0;
        loop {
            let mut wheel = self.front_left;
            if pass == 1 {
                wheel = usize::from(self.front_left == 0);
            }
            let mut other = usize::from(wheel == 0);
            let surface = self.surface_type_of(wheel, physics, ctx);
            if surface != self.surface_type[wheel] {
                let mut handed_over = false;
                if self.surface_type[wheel] != SURFACE_NONE {
                    ctx.set_input(self.io, 0, 0x7FFF);
                    if ctx.local && !self.is_rolling[other] {
                        std::mem::swap(&mut wheel, &mut other);
                        self.front_left = usize::from(self.front_left == 0);
                        handed_over = true;
                    }
                    if !handed_over {
                        if self.current_rolling_type[wheel] == ROLLING_AEMS {
                            if let Some(rolling) = self.rolling[wheel].take() {
                                ctx.aems.release_object(rolling);
                            }
                        } else if self.grains_initialized[wheel] {
                            if let Some(players) = &mut self.grain_players {
                                for player in &mut players[wheel] {
                                    player.destroy(ctx.core);
                                }
                            }
                            self.grains_initialized[wheel] = false;
                            for voices in &mut self.voices[wheel] {
                                voices.free(ctx.core);
                            }
                        }
                        self.is_rolling[wheel] = false;
                    }
                }
                self.surface_type[wheel] = surface;
                if surface != SURFACE_NONE {
                    let (key, load, no_load) = Self::surface_key(surface, physics, ctx);
                    self.surface_key = key;
                    self.attrib[wheel] = Collection::new(class::AUD_SKATEBOARD, key);
                    let rolling_type =
                        if matches!(surface, 7 | 8 | 10..=13) { ROLLING_AEMS } else { ROLLING_GRAIN };
                    if surface != self.surface_type[other] {
                        if rolling_type == ROLLING_AEMS {
                            let id = rolling_surface_to_aems_index(surface);
                            self.aems_rolling_id[wheel] = id;
                            let data = vec![0, 0, 0x1000, 0, id.clamp(0, 15), 0, surface.clamp(0, 13), 0, 0, 25000, 0, 0x7FFF];
                            self.rolling[wheel] = Some(ctx.aems.create_object("Class_rolling", data));
                        } else {
                            for (kind, data) in [(0, load), (1, no_load)] {
                                self.create_skateboard_voice(wheel, kind, ctx);
                                self.start_grain_player(wheel, kind, data, ctx);
                            }
                            self.grains_initialized[wheel] = true;
                        }
                        self.is_rolling[wheel] = true;
                        if wheel == self.front_left {
                            let bytes = ctx.vault.bytes(self.attrib[wheel], FOOT_BREAK_SLOPE, 0, 16);
                            let word = |i: usize| f32::from_bits(u32::from_be_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]));
                            self.slope_foot_break.initialize(SlopeParams {
                                min_input: word(0),
                                max_input: word(1),
                                min_output: word(2),
                                max_output: word(3),
                            });
                        }
                    }
                    self.current_rolling_type[wheel] = rolling_type;
                }
            }
            if !ctx.local {
                break;
            }
            pass += 1;
            if pass == 2 {
                break;
            }
        }
        let surface = self.surface_type_of(0, physics, ctx);
        ctx.set_input(self.io, 6, if surface == SURFACE_METAL { 0x7FFF } else { 0 });
    }

    /// `AudBoard::Initialize` on the voice's sub mix and the slot's grain data, the surface's
    /// `GrainParams` element `kind`, then `Start`.
    fn start_grain_player(&mut self, slot: usize, kind: usize, data: usize, ctx: &mut SfxCtx<'_>) {
        let bytes = ctx.vault.bytes(self.attrib[slot], GRAIN_PARAMS, kind as u32, 20);
        let word = |i: usize| f32::from_bits(u32::from_be_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]));
        let params = GrainParams { fade_in_time: word(0), sustain_time: word(1), fade_out_time: word(2), window_size: word(3), percentage_drift: word(4) };
        let timer = self.timer_base + (2 * slot + kind) as u32;
        let empty = GrainData { bytes: std::sync::Arc::from(&[][..]) };
        let data = self.board_data[data].clone().unwrap_or(empty);
        let (Some(sub_mix), Some(players)) = (plugin(self.voices[slot][kind].main, 0), &mut self.grain_players) else { return };
        let player = &mut players[slot][kind];
        player.initialize(ctx.core, sub_mix, &data, timer);
        player.grain_params = params;
        player.start(ctx.core, ctx.rng);
    }

    /// `CreateSkateBoardVoice` 0x825239e8: the grains' sub mix through high and low pass and the
    /// frequency shift (stage 2), into a stage-5 voice that pans to the master mix. The local
    /// skater adds the distortion voice (stage 3) between them.
    fn create_skateboard_voice(&mut self, slot: usize, kind: usize, ctx: &mut SfxCtx<'_>) {
        let mono = |kind: PlugInKind| PlugInConfig { kind, output_channels: 1 };
        let sub_mix = || PlugInKind::SubMix { name: String::new() };
        let main = ctx.core.create_voice(
            2,
            "SkateBoard",
            &[
                mono(sub_mix()),
                mono(PlugInKind::HighPassIir2),
                mono(PlugInKind::LowPassIir2),
                mono(PlugInKind::FrequencyShiftSsb { constructor: 0.0 }),
                mono(PlugInKind::Send),
                mono(PlugInKind::Gain),
                mono(PlugInKind::Send),
            ],
        );
        let stage2 = ctx.core.create_voice(
            5,
            "SkateBoard",
            &[
                mono(sub_mix()),
                mono(PlugInKind::Send),
                mono(PlugInKind::Send),
                PlugInConfig { kind: PlugInKind::Pan2D1 { constructor: None }, output_channels: 6 },
                PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
            ],
        );
        let voices = &mut self.voices[slot][kind];
        voices.main = main;
        voices.stage2 = stage2;
        if let Some(send) = plugin(main, MAIN_SEND) {
            ctx.core.connect_send(send, plugin(stage2, 0));
        }
        if let Some(send) = plugin(stage2, STAGE2_SEND) {
            ctx.core.connect_send(send, Some(ctx.master_sub_mix));
        }
        if let Some(send) = plugin(stage2, STAGE2_REVERB_SEND) {
            ctx.core.connect_send(send, Some(ctx.global_effects.reverb_sub_mix()));
        }
        if let Some(send) = plugin(stage2, STAGE2_FLANGE_SEND) {
            ctx.core.connect_send(send, Some(ctx.global_effects.flange_sub_mix(0x2000)));
        }
        set_attribute(ctx.core, plugin(stage2, STAGE2_FLANGE_SEND), 0, 0.0);
        if ctx.local {
            let distortion = ctx.core.create_voice(
                3,
                "SkateBoard",
                &[
                    mono(sub_mix()),
                    mono(PlugInKind::DistortionClip),
                    mono(PlugInKind::Gain),
                    mono(PlugInKind::HighShelfIir2),
                    mono(PlugInKind::Send),
                ],
            );
            self.voices[slot][kind].distortion = distortion;
            if let Some(send) = plugin(main, MAIN_DISTORTION_SEND) {
                ctx.core.connect_send(send, plugin(distortion, 0));
            }
            set_attribute(ctx.core, plugin(main, MAIN_DISTORTION_SEND), 0, self.current_distortion);
            if let Some(send) = plugin(distortion, DISTORTION_SEND) {
                ctx.core.connect_send(send, plugin(stage2, 0));
            }
            set_attribute(ctx.core, plugin(distortion, DISTORTION_CLIP), 0, self.distortion[2]);
            set_attribute(ctx.core, plugin(distortion, DISTORTION_HIGH_SHELF), 0, self.distortion[7]);
            set_attribute(ctx.core, plugin(distortion, DISTORTION_HIGH_SHELF), 1, self.distortion[8]);
        }
    }

    /// `UpdateRolling` 0x82542fc8: input 4 while pushing; a push start rebuilds the push scale
    /// (1 or its current value, to a speed-dependent level and back to 1) and the push frequency
    /// shift (0 to a level and back); then the turn and foot-brake intensities.
    fn update_rolling(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let pushing = physics.is_pushing_left || physics.is_pushing_right;
        ctx.set_input(self.io, 4, if pushing { 0x7FFF } else { 0 });
        if physics.push_started {
            let start = if self.push_scale.complete { 1.0 } else { self.push_scale.cur_value };
            let slot = self.front_left;
            let vault = ctx.vault;
            let excess = physics.board_speed - 1.0;
            let max_speed = self.attr(vault, slot, PUSH_MAX_SPEED);
            let speed = excess * 3.6;
            let t_push = clamp_fsel(speed / max_speed, 1.0);
            let length = |key| vault.i32(self.attrib[slot], key);
            let low = self.attr(vault, slot, PUSH_SCALE_MIN);
            let high = self.attr(vault, slot, PUSH_SCALE_MAX);
            let target = (high - self.attr(vault, slot, PUSH_SCALE_MIN)).mul_add(t_push, low);
            self.push_scale.clear_stages();
            self.push_scale.add_stage(start, target, length(PUSH_SCALE_TIMES[0]), CURVE_LINEAR);
            self.push_scale.add_linked_stage(target, length(PUSH_SCALE_TIMES[1]), CURVE_LINEAR);
            self.push_scale.add_linked_stage(1.0, length(PUSH_SCALE_TIMES[2]), CURVE_LINEAR);
            let low = self.attr(vault, slot, PUSH_FREQUENCY_MIN);
            let high = self.attr(vault, slot, PUSH_FREQUENCY_MAX);
            let target = (high - self.attr(vault, slot, PUSH_FREQUENCY_MIN)).mul_add(t_push, low);
            self.push_frequency.clear_stages();
            self.push_frequency.add_stage(0.0, target, length(PUSH_FREQUENCY_TIMES[0]), CURVE_LINEAR);
            self.push_frequency.add_linked_stage(target, length(PUSH_FREQUENCY_TIMES[1]), CURVE_LINEAR);
            self.push_frequency.add_linked_stage(0.0, length(PUSH_FREQUENCY_TIMES[2]), CURVE_LINEAR);
            if let Some(rattle) = self.rattle.take() {
                ctx.aems.release_object(rattle);
            }
            if self.current_rolling_type[slot] == ROLLING_GRAIN && !ctx.cvars.turn_off_dry_rolling {
                let rattle_speed = fctiwz(clamp_fsel(speed / self.attr(vault, slot, RATTLE_MAX_SPEED), 1.0) * 10000.0);
                let surface = self.rattle_surface();
                let data = vec![0, 0, 0x1000, rattle_speed.clamp(0, 10000), surface.clamp(0, 8), 0, 1, 0, 25000, 0, 0x7FFF];
                self.rattle = Some(ctx.aems.create_object("Rolling_Rattle_Class", data));
            }
        }
        if !self.push_scale.complete {
            self.push_scale.update(t);
        }
        if !self.push_frequency.complete {
            self.push_frequency.update(t);
        }
        self.turn_intensity = self.turn_intensity(physics, ctx.vault);
        let step = f32::from_bits(0x3D4C_CCCD);
        let braking = if physics.is_foot_braking {
            let along = vmx::dot3(vmx::normalize_unguarded(physics.acceleration), vmx::normalize_unguarded(physics.velocity));
            (!(along >= 0.0)).then(|| along.abs() * vmx::length(physics.acceleration))
        } else {
            None
        };
        let current = self.breaking_intensity;
        self.breaking_intensity = match braking {
            Some(input) => {
                let mut value = self.slope_foot_break.value(input);
                if value - current <= step {
                    if !(current - value <= step) {
                        value = current - step;
                    }
                    value
                } else {
                    current + step
                }
            }
            None if !(-current <= step) => current + step,
            None if !(current <= step) => current - step,
            None => 0.0,
        };
    }

    /// `GetTurnIntensity` 0x82523710: the turn scaled by speed and limited to `Carve_MaxTurn`
    /// (none in a manual), slewed at the rise or fall rate. Returns its magnitude.
    fn turn_intensity(&mut self, physics: &PlayerPhysics, vault: &Vault) -> f32 {
        let slot = self.front_left;
        let speed = vmx::length(physics.velocity);
        let max_turn = self.layout(vault, slot, CARVE_MAX_TURN);
        let limited = clamp_fsel(speed * f32::from_bits(0x3E75_C28F), max_turn);
        let mut turn = limited * physics.turn;
        if !(turn <= max_turn) {
            turn = max_turn;
        }
        let min = max_turn * -1.0;
        if !(turn >= min) {
            turn = min;
        }
        if self.in_manual(physics) {
            turn = 0.0;
        }
        let last = self.last_turn_intensity;
        let rate = self.attr(vault, slot, if turn <= last { TURN_FALL } else { TURN_RISE });
        if !(turn - last <= rate) {
            turn = last + rate;
        } else if !(last - turn <= rate) {
            turn = last - rate;
        }
        self.last_turn_intensity = turn;
        turn.abs()
    }

    /// `UpdateVoices` 0x825240b0: the grain voices' filters (outputs 12 and 11), frequency shifts
    /// (manual, push and bowl for rolling; carve, push and bowl for carving), pan (output 0),
    /// reverb send (output 13) and the local skater's flange sends (outputs 21 and 22).
    fn update_voices(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        for slot in 0..2 {
            if !(self.grains_initialized[slot] && self.current_rolling_type[slot] == ROLLING_GRAIN) {
                continue;
            }
            let low_pass = ctx.output(io, 11, PresetType::Frequency);
            let high_pass = ctx.output(io, 12, PresetType::Frequency) as f32;
            let low_pass = low_pass as f32;
            let [rolling, carve] = self.voices[slot];
            set_attribute(ctx.core, plugin(rolling.main, MAIN_HIGH_PASS), 0, high_pass);
            set_attribute(ctx.core, plugin(rolling.main, MAIN_LOW_PASS), 0, low_pass);
            set_attribute(ctx.core, plugin(carve.main, MAIN_HIGH_PASS), 0, high_pass);
            set_attribute(ctx.core, plugin(carve.main, MAIN_LOW_PASS), 0, low_pass);
            let mut shift = 0.0;
            if self.in_manual(physics) {
                shift = self.attr(ctx.vault, slot, MANUAL_FREQUENCY_SHIFT);
            }
            if !self.push_frequency.complete {
                shift += self.push_frequency.cur_value;
            }
            if self.gforce_bowl > 0.0 {
                shift = self.layout(ctx.vault, self.front_left, GFORCE_FREQUENCY_SHIFT).mul_add(self.gforce_bowl, shift);
            }
            set_attribute(ctx.core, plugin(rolling.main, MAIN_FREQUENCY_SHIFT), 0, shift);
            let mut carve_shift = self.layout(ctx.vault, slot, CARVE_FREQUENCY_SHIFT);
            if !self.push_frequency.complete {
                carve_shift += self.push_frequency.cur_value;
            }
            if self.gforce_bowl > 0.0 {
                carve_shift = self.attr(ctx.vault, self.front_left, GFORCE_CARVE_FREQUENCY_SHIFT).mul_add(self.gforce_bowl, carve_shift);
            }
            set_attribute(ctx.core, plugin(carve.main, MAIN_FREQUENCY_SHIFT), 0, carve_shift);
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                eprintln!("{elapsed:9.3} board voices slot {slot} shift {shift:.2} carve_shift {carve_shift:.2} bowl {:.3} distortion {:.3}", self.gforce_bowl, self.current_distortion);
            }
            let pan = ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE;
            set_attribute(ctx.core, plugin(rolling.stage2, STAGE2_PAN), 0, pan);
            set_attribute(ctx.core, plugin(carve.stage2, STAGE2_PAN), 0, pan);
            let reverb = ctx.output(io, 13, PresetType::Volume) as f32 * VOLUME_SCALE;
            set_attribute(ctx.core, plugin(rolling.stage2, STAGE2_REVERB_SEND), 0, reverb);
            set_attribute(ctx.core, plugin(carve.stage2, STAGE2_REVERB_SEND), 0, reverb);
            if ctx.local {
                let rolling_flange = ctx.output(io, 21, PresetType::Volume);
                let carve_flange = ctx.output(io, 22, PresetType::Volume) as f32 * VOLUME_SCALE;
                set_attribute(ctx.core, plugin(rolling.stage2, STAGE2_FLANGE_SEND), 0, rolling_flange as f32 * VOLUME_SCALE);
                set_attribute(ctx.core, plugin(carve.stage2, STAGE2_FLANGE_SEND), 0, carve_flange);
            }
        }
    }

    /// `IsWheelSlide` 0x825226b0.
    fn is_wheel_slide(&self, physics: &PlayerPhysics, vault: &Vault) -> bool {
        if !(physics.wheel_slide <= 0.0) {
            if !physics.is_rail_grind {
                return true;
            }
            let forced = vault.i32(RAILS_DEFAULT, GRIND_TYPE_OVERRIDE);
            if forced == -1 {
                return physics.grind_type == GRIND_WHEEL_SLIDE;
            }
            return forced.wrapping_add(1) == GRIND_WHEEL_SLIDE;
        }
        physics.is_reverting || self.add_to_angle > 0
    }

    /// `CheckForSkids` 0x825227b8: input 1 while the wheels slide; a slide off grass and dirt
    /// creates the skid instance.
    fn check_for_skids(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !self.is_wheel_slide(physics, ctx.vault) {
            ctx.set_input(self.io, 1, 0);
            return;
        }
        ctx.set_input(self.io, 1, 0x7FFF);
        if self.wheel_skid.is_some() {
            return;
        }
        let material = Self::power_slide_material(physics, ctx.vault);
        if material >= PS_GRASS as i32 {
            return;
        }
        let speed = fctiwz(clamp_fsel(physics.board_speed * f32::from_bits(0x3DA3_D70A), 1.0) * 10000.0);
        let enter_volume = ctx.vault.i32(WHEELSLIDE, ENTER_VOLUME);
        let exit_volume = ctx.vault.i32(WHEELSLIDE, EXIT_VOLUME);
        let mut wheel_type = ctx.vault.i32(WHEELSLIDE, POWERSLIDE_TEST_WHEELS);
        if wheel_type == -1 {
            wheel_type = if ctx.local { physics.wheel_hardness } else { ctx.npc_wheel_type } as i32;
        }
        let (local, flange) = if ctx.local { (1, ctx.output(self.io, 20, PresetType::Volume)) } else { (0, 0) };
        let angle = fctiwz(physics.wheel_slide * 90.0);
        let data = vec![
            0,
            0x7FFF,
            0,
            0,
            0,
            25000,
            0,
            speed.clamp(0, 10000),
            wheel_type.clamp(0, 1),
            material.clamp(0, 2),
            angle.clamp(0, 90),
            enter_volume.clamp(0, 0x7FFF),
            exit_volume.clamp(0, 0x7FFF),
            local,
            flange.clamp(0, 0x7FFF),
        ];
        self.wheel_skid = Some(ctx.aems.create_object("Class_wheels_skid", data));
    }

    /// The power-slide material: the `aud_general` wheelslide override, else wheel 0's material
    /// through `SurfaceMapping::GetPowerSlideMaterial` (0 past the table).
    fn power_slide_material(physics: &PlayerPhysics, vault: &Vault) -> i32 {
        let mut material = vault.i32(WHEELSLIDE, POWERSLIDE_OVERRIDE);
        if material == -1 {
            let wheel = physics.wheel_material[0];
            material = if wheel >= MATERIAL_MAX { 0 } else { power_slide_material(vault, wheel) as i32 };
        }
        material
    }

    /// `UpdateSkids` 0x82522eb0: the skid ends with the slide; a revert turns `mAddToAngle` up by
    /// 5 (to 45), otherwise it falls by 15; the class follows the speed, mixer and slide angle.
    fn update_skids(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.wheel_skid.is_none() {
            return;
        }
        if !self.is_wheel_slide(physics, ctx.vault) {
            if let Some(skid) = self.wheel_skid.take() {
                ctx.aems.release_object(skid);
            }
            return;
        }
        let speed = fctiwz(clamp_fsel(physics.board_speed * f32::from_bits(0x3DA3_D70A), 1.0) * 10000.0);
        if physics.is_reverting {
            self.add_to_angle = (self.add_to_angle + 5).min(45);
        } else if self.add_to_angle != 0 {
            if self.add_to_angle > 0 {
                self.add_to_angle -= 15;
            }
            self.add_to_angle = self.add_to_angle.max(0);
        }
        let io = self.io;
        let pitch = ctx.output(io, 3, PresetType::Pitch).clamp(0, 0x2000);
        let dry = ctx.output(io, 4, PresetType::Volume).clamp(0, 0x7FFF);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x1_0000);
        let wet = ctx.output(io, 13, PresetType::Volume).clamp(0, 0x7FFF);
        let low_pass = ctx.output(io, 11, PresetType::Frequency).clamp(0, 25000);
        let high_pass = ctx.output(io, 12, PresetType::Frequency).clamp(0, 25000);
        let angle = self.add_to_angle.wrapping_sub(fctiwz(physics.wheel_slide * -90.0)).min(90).clamp(0, 90);
        let material = Self::power_slide_material(physics, ctx.vault).clamp(0, 2);
        let flange = if ctx.local { ctx.output(io, 20, PresetType::Volume) } else { 0 }.clamp(0, 0x7FFF);
        let Some(skid) = self.wheel_skid.as_mut() else { return };
        let data = &mut skid.data;
        data[7] = speed.clamp(0, 10000);
        data[4] = pitch;
        data[0] = 0x7FFF;
        data[1] = dry;
        data[3] = azimuth;
        data[2] = wet;
        data[5] = low_pass;
        data[6] = high_pass;
        data[10] = angle;
        data[9] = material;
        data[14] = flange;
        let skid = skid.clone();
        ctx.aems.set_member_data(&skid);
    }

    /// `CalcDuckerValues` 0x82524db0: a random level between the pattern's ducker minimum and
    /// maximum (in hundredths) and a random time between its times.
    fn ducker_values(&self, rng: &mut RwRandom) -> (f32, i32) {
        let mut ducker = self.volume_ducker_min;
        if self.volume_ducker_min != self.volume_ducker_max {
            let range = fctiwz((self.volume_ducker_max - self.volume_ducker_min) * 100.0);
            let (divisor, step) = if range > 0 { (range, 0.01) } else { (range.wrapping_abs(), -0.01f32) };
            let pick = random_below(rng, divisor as u32) as i32 as f32;
            ducker = pick * step + self.volume_ducker_min;
        }
        let mut timer = self.volume_ducker_min_time;
        if self.volume_ducker_max_time != self.volume_ducker_min_time {
            let range = self.volume_ducker_max_time.wrapping_sub(self.volume_ducker_min_time).wrapping_abs();
            timer = self.volume_ducker_min_time.wrapping_add(random_below(rng, range as u32) as i32);
        }
        (ducker, timer)
    }

    /// `CheckForPatternScaling` 0x82524ed0 (local skater): a new surface pattern (wheel 0's, or the
    /// `aud_cracks` override) loads its ducker and starts ducking when it has one.
    fn check_for_pattern_scaling(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        let mut pattern = ctx.vault.i32(CRACKS_DEFAULT, PATTERN_OVERRIDE);
        if pattern == -1 {
            pattern = physics.surface_pattern[0] as i32;
        }
        if pattern == self.current_surface_pattern {
            if !self.pattern_scale.complete {
                self.pattern_scale.update(t);
            }
            return;
        }
        self.pattern_scale.clear_stages();
        self.current_surface_pattern = pattern;
        self.ducking_for_surface_pattern = false;
        if pattern == 0 {
            return;
        }
        let cracks = Collection::new(class::AUD_CRACKS, surface_pattern_key(pattern));
        self.volume_ducker_min = ctx.vault.f32(cracks, DUCKER_MIN);
        self.volume_ducker_max = ctx.vault.f32(cracks, DUCKER_MAX);
        self.volume_ducker_min_time = ctx.vault.i32(cracks, DUCKER_TIME_MIN);
        self.volume_ducker_max_time = ctx.vault.i32(cracks, DUCKER_TIME_MAX);
        if self.volume_ducker_min != 1.0 || self.volume_ducker_max != 1.0 {
            self.ducking_for_surface_pattern = true;
            let (ducker, timer) = self.ducker_values(ctx.rng);
            self.pattern_scale.clear_stages();
            self.pattern_scale.add_stage(1.0, ducker, timer, CURVE_LINEAR);
            self.ducking_target = ducker;
        }
    }

    /// `UpdatePatternScaling` 0x825250e0: a finished duck turns around (down to a new level, or
    /// back up to 1).
    fn update_pattern_scaling(&mut self, rng: &mut RwRandom) {
        if !(self.ducking_for_surface_pattern && self.pattern_scale.complete) {
            return;
        }
        let (ducker, timer) = self.ducker_values(rng);
        self.pattern_scale.clear_stages();
        if self.ducking_target >= 1.0 {
            self.pattern_scale.add_stage(1.0, ducker, timer, CURVE_LINEAR);
            self.ducking_target = ducker;
        } else {
            self.pattern_scale.add_stage(self.ducking_target, 1.0, timer, CURVE_LINEAR);
            self.ducking_target = 1.0;
        }
    }

    /// `UpdateDistortion` 0x82525968 (local skater): the distortion send rises with the speed in
    /// km/h between its speeds; the dry rolling ducks between the duck speeds.
    fn update_distortion(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        let [min_speed, max_speed, _, max_volume, duck_min_speed, duck_max_speed, max_duck, ..] = self.distortion;
        let speed = physics.board_speed * 3.6;
        let mut distortion = 0.0;
        if !(speed < max_speed) {
            distortion = max_volume;
        } else if !(speed < min_speed) {
            distortion = ((speed - min_speed) / (max_speed - min_speed)) * max_volume;
        }
        if !(speed < duck_max_speed) {
            self.current_distortion_duck_multiplier = max_duck;
        } else if !(speed < duck_min_speed) {
            let t = (speed - duck_min_speed) / (duck_max_speed - duck_min_speed);
            self.current_distortion_duck_multiplier = fnmsubs(t, 1.0 - max_duck, 1.0);
        }
        if ctx.cvars.play_distortion && distortion != self.current_distortion {
            self.current_distortion = distortion;
            for voices in self.voices.iter().flatten() {
                set_attribute(ctx.core, plugin(voices.main, MAIN_DISTORTION_SEND), 0, self.current_distortion);
            }
        }
    }

    /// `UpdateModulation` 0x82514cf0: each finished modulation moves to a random depth (the
    /// opposite sign of the last) over a random time.
    fn update_modulation(&mut self, t: f32, rng: &mut RwRandom) {
        for modulation in &mut self.modulation {
            if !modulation.scale.complete {
                modulation.scale.update(t);
                continue;
            }
            let range = modulation.max_time.wrapping_sub(modulation.min_time) as u32;
            let time = random_below(rng, range).wrapping_add(modulation.min_time as u32) as i32;
            let depth_range = fctiwz((modulation.max_duck - modulation.min_duck) * 100.0);
            let pick = random_below(rng, depth_range as u32) as i32 as f32;
            let mut target = pick.mul_add(f32::from_bits(0x3C23_D70A), modulation.min_duck);
            if !(modulation.cur_target < 0.0) {
                target *= -1.0;
            }
            modulation.scale.clear_stages();
            modulation.scale.add_stage(modulation.cur_target, target, time, CURVE_LINEAR);
            modulation.cur_target = target;
        }
    }

    /// `UpdateVolumeModulation` 0x82525af0 (local skater): the modulations scaled by speed, ducked
    /// by the distortion, onto the grain voices' gains.
    fn update_volume_modulation(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            for modulation in &mut self.modulation {
                modulation.cur_value = 1.0;
            }
            self.current_distortion_duck_multiplier = 1.0;
            return;
        }
        self.update_modulation(t, ctx.rng);
        if ctx.cvars.turn_off_dry_rolling {
            for voices in self.voices.iter().flatten() {
                if voices.main.is_some() {
                    set_attribute(ctx.core, plugin(voices.main, MAIN_GAIN), 0, 0.0);
                    for modulation in &mut self.modulation {
                        modulation.cur_last_value = 0.0;
                    }
                }
            }
            return;
        }
        if ctx.cvars.play_volume_modulation {
            let speed = physics.board_speed * 3.6;
            let mut amount = 0.0;
            if !(speed < self.modulation_max_speed) {
                amount = 1.0;
            } else if !(speed < self.modulation_min_speed) {
                amount = (speed - self.modulation_min_speed) / (self.modulation_max_speed - self.modulation_min_speed);
            }
            for modulation in &mut self.modulation {
                modulation.cur_value = modulation.scale.cur_value.mul_add(amount, 1.0);
            }
        }
        for modulation in &mut self.modulation {
            modulation.cur_unducked = modulation.cur_value;
            modulation.cur_value *= self.current_distortion_duck_multiplier;
        }
        // The last-value check is shared by both slots, so only slot 0's voices ever change.
        for slot in 0..2 {
            for kind in 0..2 {
                let modulation = &mut self.modulation[kind];
                if modulation.cur_unducked == modulation.cur_last_value {
                    continue;
                }
                modulation.cur_last_value = modulation.cur_unducked;
                let voices = self.voices[slot][kind];
                if voices.main.is_some() {
                    set_attribute(ctx.core, plugin(voices.main, MAIN_GAIN), 0, modulation.cur_value);
                    set_attribute(ctx.core, plugin(voices.distortion, DISTORTION_GAIN), 0, modulation.cur_unducked);
                }
            }
        }
    }

    /// `CalcAzimRate` 0x82526558: how fast the azimuth moves, limited and slewed, into input 5.
    fn calc_azim_rate(&mut self, t: f32, ctx: &mut SfxCtx<'_>) {
        let change = self.cur_azim.wrapping_sub(self.last_azim).wrapping_abs();
        let mut rate = change as f32 / t;
        let max = ctx.vault.f32(AZIM_RATE, AZIM_RATE_MAX);
        if !(rate <= max) {
            rate = max;
        }
        let slew = ctx.vault.f32(AZIM_RATE, AZIM_RATE_SLEW) * t;
        let last = self.last_azim_rate;
        if !(rate <= last) {
            if !(rate - last <= slew) {
                rate = last + slew;
            }
        } else if rate < last && !(last - rate <= slew) {
            rate = last - slew;
        }
        self.last_azim_rate = rate;
        ctx.set_input(self.io, 5, fctiwz((rate / max) * 32767.0).clamp(0, 0x7FFF));
    }

    /// `ProcessUpdate` 0x8254d0b0: each rolling grain slot's two grain players get their
    /// intensity (the `Speed_Ramp` Bezier of the push-scaled speed against `MaxSpeed`), volume
    /// and pitch; then the skid and pattern updates. The AEMS rolling, squeaks, rattle, grit,
    /// rocks, spider cracks and board slide are not ported.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        ctx.global_effects.set_flange_azimuth(0, (azimuth as f32 * f32::from_bits(0x3780_0000)) * 360.0);
        self.last_azim = self.cur_azim;
        self.cur_azim = azimuth;
        for slot in 0..2 {
            if !self.is_rolling[slot] {
                continue;
            }
            if self.current_rolling_type[slot] == ROLLING_AEMS {
                self.update_aems_rolling(slot, physics, ctx);
                continue;
            }
            if !self.grains_initialized[slot] {
                continue;
            }
            let mut speed = physics.board_speed;
            if !self.push_scale.complete {
                speed *= self.push_scale.cur_value;
            }
            let ramp = SPEED_RAMP.map(|word| self.layout(ctx.vault, slot, word));
            let v = clamp_fsel((speed / self.layout(ctx.vault, slot, MAX_SPEED)) * 3.6, 1.0);
            let u = 1.0 - v;
            let w = 1.0 - u;
            let near = ramp[2] * u;
            let u2 = u * u;
            let w2 = w * w;
            let middle = ramp[1].mul_add(w, near);
            let u3 = u2 * u;
            let w3 = w2 * w;
            let middle = (middle * w) * u;
            let intensity = u3.mul_add(ramp[3], middle.mul_add(3.0, w3 * ramp[0]));
            let rolling_volume = ctx.output(io, 1, PresetType::Volume) as f32 * VOLUME_SCALE;
            let turn = fsel(self.turn_intensity - self.breaking_intensity, self.turn_intensity, self.breaking_intensity);
            let mut volume = (1.0 - turn) * rolling_volume;
            if self.in_manual(physics) {
                volume *= self.attr(ctx.vault, slot, MANUAL_VOLUME);
            }
            if self.ducking_for_surface_pattern {
                volume *= self.pattern_scale.cur_value;
            }
            let pitch = ctx.output(io, 3, PresetType::Pitch) as f32 * PITCH_SCALE;
            let mut params = UpdateParams { volume, pitch, azimuth: 0, intensity };
            if let Some(players) = &mut self.grain_players {
                players[slot][0].update(params);
            }
            params.intensity -= f32::from_bits(0x3DCC_CCCD);
            if !(params.intensity >= 0.0) {
                params.intensity = 0.0;
            }
            let carve_volume = ctx.output(io, 2, PresetType::Volume) as f32 * VOLUME_SCALE;
            let mut carve = self.turn_intensity * carve_volume;
            if self.ducking_for_surface_pattern {
                carve *= self.pattern_scale.cur_value;
            }
            if !(self.gforce_bowl <= 0.0) {
                let slow_down = self.layout(ctx.vault, self.front_left, GFORCE_SLOW_DOWN_VELOCITY);
                let mult = self.layout(ctx.vault, self.front_left, GFORCE_VOLUME_MULT);
                let mut scale = 1.0;
                if !(slow_down <= 0.0) {
                    scale = clamp_fsel((speed / slow_down) * 3.6, 1.0);
                }
                carve = ((self.gforce_bowl * scale) * mult).mul_add(volume, carve);
                if !(carve <= 1.0) {
                    carve = 1.0;
                }
            }
            params.volume = carve;
            params.pitch = ctx.output(io, 3, PresetType::Pitch) as f32 * PITCH_SCALE;
            if let Some(players) = &mut self.grain_players {
                players[slot][1].update(params);
            }
            if let Some(elapsed) = crate::splicer::trace_elapsed() {
                eprintln!(
                    "{elapsed:9.3} board slot {slot} surface {} speed {speed:.3} intensity {intensity:.3} volume {volume:.4} carve {carve:.4} pitch {:.4} out1 {} out2 {} lp {} hp {} az {} turn {:.3} brake {:.3}",
                    self.surface_type[slot],
                    params.pitch,
                    ctx.output(io, 1, PresetType::Volume),
                    ctx.output(io, 2, PresetType::Volume),
                    ctx.output(io, 11, PresetType::Frequency),
                    ctx.output(io, 12, PresetType::Frequency),
                    ctx.output(io, 0, PresetType::Azimuth),
                    self.turn_intensity,
                    self.breaking_intensity,
                );
            }
        }
        self.update_skids(physics, ctx);
        self.update_squeaks(physics, ctx);
        self.update_rattle(ctx);
        self.update_active_sounds(physics, ctx);
        self.update_spider_cracks(physics, ctx);
        self.update_pattern_scaling(ctx.rng);
        self.update_board_slide(physics, ctx);
    }
}

/// `RollingSurfaceToAEMSIndex` 0x82514a48: the `Class_rolling` surface of an AEMS rolling surface.
fn rolling_surface_to_aems_index(surface: i32) -> i32 {
    match surface {
        7 => 1,
        8 => 2,
        10 => 10,
        11 => 12,
        12 => 11,
        13 => 9,
        _ => -1,
    }
}

impl SkateBoard {
    /// `GetRattleSurface` 0x82514988: the rattle's surface for the hard-wheel keys of smooth
    /// asphalt, rough and smooth concrete, wood and aggregate; rough asphalt otherwise.
    fn rattle_surface(&self) -> i32 {
        match self.surface_key {
            0x7C59_12FC_2DAB_F98C => 1,
            0x0372_1D0F_A99A_03C8 => 2,
            0xFFB5_E3E6_2E0B_4943 => 3,
            0x7947_A259_F181_FDB4 => 4,
            0xB303_AED8_2415_30E2 => 5,
            _ => 0,
        }
    }

    /// `UpdateRattle` 0x825147a8: full volume, the mixer's dry (6), azimuth, pitch (3), wet (16)
    /// and filters (14, 15).
    fn update_rattle(&mut self, ctx: &mut SfxCtx<'_>) {
        if self.rattle.is_none() {
            return;
        }
        let io = self.io;
        let pitch = ctx.output(io, 3, PresetType::Pitch);
        let dry = ctx.output(io, 6, PresetType::Volume).clamp(0, 0x7FFF);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        let wet = ctx.output(io, 16, PresetType::Volume).clamp(0, 0x7FFF);
        let low_pass = ctx.output(io, 14, PresetType::Frequency).clamp(0, 25000);
        let high_pass = ctx.output(io, 15, PresetType::Frequency).clamp(0, 25000);
        let Some(rattle) = self.rattle.as_mut() else { return };
        let data = &mut rattle.data;
        data[0] = 0x7FFF;
        data[10] = dry;
        data[1] = azimuth;
        data[2] = pitch.clamp(0, 0x2000);
        data[7] = wet;
        data[8] = low_pass;
        data[9] = high_pass;
        let rattle = rattle.clone();
        ctx.aems.set_member_data(&rattle);
    }

    /// `GetMaxRollingSpeed` 0x825244e8: `MaxRollingSpeed[material]` of `aud_general` aems_rolling.
    fn max_rolling_speed(vault: &Vault, material: u32) -> f32 {
        f32::from_bits(u32::from_be_bytes(vault.bytes(AEMS_ROLLING, MAX_ROLLING_SPEED, material, 4).try_into().unwrap_or([0; 4])))
    }

    /// The `Class_rolling` speed of grit (material 0) and rocks (3): the board speed over the
    /// material's maximum, in km/h, as 0..10000.
    fn rolling_speed(physics: &PlayerPhysics, vault: &Vault, material: u32) -> i32 {
        fctiwz(clamp_fsel((physics.board_speed / Self::max_rolling_speed(vault, material)) * 3.6, 1.0) * 10000.0)
    }

    /// `GetAEMSSpeed` 0x82543820: the board speed (times an unfinished push scale) over the
    /// material's maximum (45 for -1), in km/h, as 0..10000.
    fn aems_speed(&self, physics: &PlayerPhysics, vault: &Vault, material: i32) -> i32 {
        let mut speed = physics.board_speed;
        if !self.push_scale.complete {
            speed = self.push_scale.cur_value * speed;
        }
        let max = if material == -1 { 45.0 } else { Self::max_rolling_speed(vault, material as u32) };
        fctiwz(clamp_fsel((speed / max) * 3.6, 1.0) * 10000.0)
    }

    /// `StartActiveSounds` 0x82524578 (the local skater's `InitSFX`): the grit (surface 0) and
    /// rocks (surface 3) `Class_rolling`s.
    fn start_active_sounds(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let speed = Self::rolling_speed(physics, ctx.vault, 0).clamp(0, 10000);
        self.grit = Some(ctx.aems.create_object("Class_rolling", vec![0, 0, 0x1000, speed, 0, 0, 3, 0, 0, 25000, 0, 0x7FFF]));
        let speed = Self::rolling_speed(physics, ctx.vault, 3).clamp(0, 10000);
        self.rocks = Some(ctx.aems.create_object("Class_rolling", vec![0, 0, 0x1000, speed, 3, 0, 3, 0, 0, 25000, 0, 0x7FFF]));
    }

    /// The `rSurface` the rolling classes follow: the front-left slot's surface, else the other
    /// slot's, else the rail (13) while grinding (otherwise unchanged).
    fn update_rolling_surface(&mut self, data: &mut [i32], physics: &PlayerPhysics, ctx: &SfxCtx<'_>) {
        let mut surface = self.surface_type_of(self.front_left, physics, ctx);
        if surface == SURFACE_NONE {
            surface = self.surface_type_of(usize::from(self.front_left == 0), physics, ctx);
            if surface == SURFACE_NONE {
                if physics.is_rail_grind {
                    data[6] = 13;
                }
                return;
            }
        }
        data[6] = surface.clamp(0, 13);
    }

    /// The members `UpdateActiveSounds` and `UpdateSpiderCracks` share: full volume, the mixer's
    /// dry (`dry_output`), azimuth, pitch (8), wet (19) and filters (17, 18), the speed, no
    /// district, the surface and whether in (or braking out of) a manual.
    fn update_rolling_class(&mut self, object: &mut ClassObject, dry_output: i32, speed: i32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        let dry = ctx.output(io, dry_output, PresetType::Volume).clamp(0, 0x7FFF);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x1_0000);
        let pitch = ctx.output(io, 8, PresetType::Pitch).clamp(0, 0x2000);
        let data = &mut object.data;
        data[0] = 0x7FFF;
        data[11] = dry;
        data[1] = azimuth;
        data[2] = pitch;
        data[3] = speed.clamp(0, 10000);
        data[5] = 0;
        self.update_rolling_surface(data, physics, ctx);
        data[7] = i32::from(physics.is_manual || physics.is_manual_braking);
        data[8] = ctx.output(io, 19, PresetType::Volume).clamp(0, 0x7FFF);
        data[9] = ctx.output(io, 17, PresetType::Frequency).clamp(0, 25000);
        data[10] = ctx.output(io, 18, PresetType::Frequency).clamp(0, 25000);
        ctx.aems.set_member_data(object);
    }

    /// `UpdateActiveSounds` 0x825247b0: grit (dry 7) and rocks (dry 9).
    fn update_active_sounds(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(mut grit) = self.grit.take() {
            let speed = Self::rolling_speed(physics, ctx.vault, 0);
            self.update_rolling_class(&mut grit, 7, speed, physics, ctx);
            self.grit = Some(grit);
        }
        if let Some(mut rocks) = self.rocks.take() {
            let speed = Self::rolling_speed(physics, ctx.vault, 3);
            self.update_rolling_class(&mut rocks, 9, speed, physics, ctx);
            self.rocks = Some(rocks);
        }
    }

    /// `ProcessUpdate`'s AEMS rolling slot: full volume, the rolling volume (1) as dry, azimuth,
    /// pitch (3), the slot surface's speed, no district, wet (13, none for AEMS surface 1),
    /// filters (11, 12) and the manual flag.
    fn update_aems_rolling(&mut self, slot: usize, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.rolling[slot].is_none() {
            return;
        }
        let io = self.io;
        let id = self.aems_rolling_id[slot];
        let speed = self.aems_speed(physics, ctx.vault, id);
        let dry = ctx.output(io, 1, PresetType::Volume).clamp(0, 0x7FFF);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x1_0000);
        let pitch = ctx.output(io, 3, PresetType::Pitch).clamp(0, 0x2000);
        let wet = if id == 1 { 0 } else { ctx.output(io, 13, PresetType::Volume).clamp(0, 0x7FFF) };
        let low_pass = ctx.output(io, 11, PresetType::Frequency).clamp(0, 25000);
        let high_pass = ctx.output(io, 12, PresetType::Frequency).clamp(0, 25000);
        let Some(rolling) = self.rolling[slot].as_mut() else { return };
        let data = &mut rolling.data;
        data[0] = 0x7FFF;
        data[11] = dry;
        data[1] = azimuth;
        data[2] = pitch;
        data[3] = speed.clamp(0, 10000);
        data[5] = 0;
        data[8] = wet;
        data[9] = low_pass;
        data[10] = high_pass;
        data[7] = i32::from(physics.is_manual || physics.is_manual_braking);
        let rolling = rolling.clone();
        ctx.aems.set_member_data(&rolling);
    }

    /// `CheckForSqueaks` 0x82522b78: with squeaks on, both feet on the deck and two or more wheels
    /// down, a deck tilt of at least `Squeak_Min_Angle` degrees starts `Class_Squeaks`; a change
    /// of tilt side restarts it. Otherwise it ends.
    fn check_for_squeaks(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if ctx.vault.bytes(SKATEBOARD_DEFAULT, PLAY_SQUEAKS, 0, 1)[0] == 0 {
            return;
        }
        if !(physics.left_foot_on_deck && physics.right_foot_on_deck && physics.num_wheels_on_ground > 1) {
            if let Some(squeaks) = self.squeaks.take() {
                ctx.aems.release_object(squeaks);
            }
            return;
        }
        let min_angle = ctx.vault.i32(SKATEBOARD_DEFAULT, SQUEAK_MIN_ANGLE);
        let tilt = physics.deck_tilt;
        let angular = physics.deck_local_xyz_angular_speed[2].abs();
        if fctiwz(tilt.abs() * f32::from_bits(0x42E5_2EE0)) < min_angle {
            return;
        }
        let sign = tilt >= 0.0;
        if self.prev_sign != sign {
            if let Some(squeaks) = self.squeaks.take() {
                ctx.aems.release_object(squeaks);
            }
        }
        self.prev_sign = sign;
        if self.squeaks.is_some() {
            return;
        }
        let speed = fctiwz(clamp_fsel(physics.board_speed * f32::from_bits(0x3DA3_D70A), 1.0) * 1000.0);
        let velocity = Self::squeak_velocity(angular, ctx.vault);
        let data = vec![0, 0x7FFF, 0, 0, 0x1000, 25000, 0, speed.clamp(0, 1000), 15, velocity.clamp(0, 1000)];
        self.squeaks = Some(ctx.aems.create_object("Class_Squeaks", data));
    }

    /// The deck's angular speed over `Squeak_Max_Angular_Vel` as 0..1000, 0 below 50.
    fn squeak_velocity(angular: f32, vault: &Vault) -> i32 {
        let velocity = fctiwz((angular / vault.f32(SKATEBOARD_DEFAULT, SQUEAK_MAX_ANGULAR_VEL)) * 1000.0);
        if velocity > 1000 {
            1000
        } else if velocity < 50 {
            0
        } else {
            velocity
        }
    }

    /// `UpdateSqueaks` 0x825232e0: speed, angular velocity, pitch (3), full volume, dry (5), no wet,
    /// filters (11, 12), azimuth.
    fn update_squeaks(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.squeaks.is_none() {
            return;
        }
        let io = self.io;
        let speed = fctiwz(clamp_fsel(physics.board_speed * f32::from_bits(0x3DA3_D70A), 1.0) * 1000.0);
        let velocity = Self::squeak_velocity(physics.deck_local_xyz_angular_speed[2].abs(), ctx.vault);
        let pitch = ctx.output(io, 3, PresetType::Pitch).clamp(0, 0x2000);
        let dry = ctx.output(io, 5, PresetType::Volume).clamp(0, 0x7FFF);
        let low_pass = ctx.output(io, 11, PresetType::Frequency).clamp(0, 25000);
        let high_pass = ctx.output(io, 12, PresetType::Frequency).clamp(0, 25000);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0x1_0000);
        let Some(squeaks) = self.squeaks.as_mut() else { return };
        let data = &mut squeaks.data;
        data[7] = speed.clamp(0, 1000);
        data[9] = velocity.clamp(0, 1000);
        data[4] = pitch;
        data[0] = 0x7FFF;
        data[1] = dry;
        data[2] = 0;
        data[5] = low_pass;
        data[6] = high_pass;
        data[3] = azimuth;
        let squeaks = squeaks.clone();
        ctx.aems.set_member_data(&squeaks);
    }

    /// `CheckForSpiderCracks` 0x82543ae8: wheel 0's surface pattern (wheel 3's after a manual while
    /// the front wheels are up, or the `aud_cracks` override) of spider cracks keeps a
    /// `Class_rolling` (surface 5) alive.
    fn check_for_spider_cracks(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let mut pattern = ctx.vault.i32(CRACKS_DEFAULT, PATTERN_OVERRIDE);
        if pattern == -1 {
            pattern = physics.surface_pattern[0] as i32;
            if self.in_manual(physics) && !physics.landed[0] {
                pattern = physics.surface_pattern[3] as i32;
            }
        }
        let speed = self.aems_speed(physics, ctx.vault, 5);
        match self.spider_cracks.take() {
            None if pattern == PATTERN_SPIDER_CRACKS => {
                let data = vec![0, 0, 0x1000, speed.clamp(0, 10000), 5, 0, 3, 0, 0, 25000, 0, 0x7FFF];
                self.spider_cracks = Some(ctx.aems.create_object("Class_rolling", data));
            }
            Some(cracks) if pattern != PATTERN_SPIDER_CRACKS => ctx.aems.release_object(cracks),
            other => self.spider_cracks = other,
        }
    }

    /// `UpdateSpiderCracks` 0x82543cf8: the rolling-class members with dry 10 and the AEMS speed
    /// of surface 5.
    fn update_spider_cracks(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(mut cracks) = self.spider_cracks.take() {
            let speed = self.aems_speed(physics, ctx.vault, 5);
            self.update_rolling_class(&mut cracks, 10, speed, physics, ctx);
            self.spider_cracks = Some(cracks);
        }
    }

    /// `CheckForBoardSlide` 0x82514e10: a board slide keeps `C_board_slide` alive (board side 1 for
    /// slide type 2).
    fn check_for_board_slide(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        match self.board_slide.take() {
            None if physics.board_slide != 0 => {
                let side = i32::from(physics.board_slide == BOARD_SLIDE_TYPE_2);
                let data = vec![0, 0, 0x1000, 0, 25000, 0, 0, 0, 0, side.clamp(0, 3), 0, 0];
                self.board_slide = Some(ctx.aems.create_object("c_board_slide", data));
            }
            Some(slide) if physics.board_slide == 0 => ctx.aems.release_object(slide),
            other => self.board_slide = other,
        }
    }

    /// `UpdateBoardSlide` 0x82525d78: full volume, azimuth, pitch (23), speed over the
    /// boardslide maximum, filters (25, 26), wet (27), dry (24), the camera microphone's distance
    /// over the boardslide maximum (0..1000) and the slide type's volume.
    fn update_board_slide(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.board_slide.is_none() {
            return;
        }
        let io = self.io;
        let azimuth = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        let pitch = ctx.output(io, 23, PresetType::Pitch).clamp(0, 0x2000);
        let max_speed = ctx.vault.f32(BOARDSLIDE, BOARDSLIDE_MAX_SPEED);
        let speed = fctiwz(clamp_fsel(((physics.board_speed - 0.5) / max_speed) * 3.6, 1.0) * 10000.0);
        let low_pass = ctx.output(io, 25, PresetType::Frequency).clamp(0, 25000);
        let high_pass = ctx.output(io, 26, PresetType::Frequency).clamp(0, 25000);
        let wet = ctx.output(io, 27, PresetType::Volume).clamp(0, 0x7FFF);
        let dry = ctx.output(io, 24, PresetType::Volume).clamp(0, 0x7FFF);
        let distance = vmx::length(vmx::sub(ctx.camera_position, physics.board_position));
        let max_distance = ctx.vault.f32(BOARDSLIDE, BOARDSLIDE_MAX_DISTANCE);
        let distance = fctiwz(clamp_fsel(distance / max_distance, 1.0) * 1000.0);
        let volume_key = if physics.board_slide == BOARD_SLIDE_TYPE_2 { BOARDSLIDE_VOLUME_TYPE_2 } else { BOARDSLIDE_VOLUME };
        let volume = ctx.vault.i32(BOARDSLIDE, volume_key);
        let Some(slide) = self.board_slide.as_mut() else { return };
        let data = &mut slide.data;
        data[0] = 0x7FFF;
        data[1] = azimuth;
        data[2] = pitch;
        data[3] = speed.clamp(0, 10000);
        data[4] = low_pass;
        data[5] = high_pass;
        data[6] = wet;
        data[7] = dry;
        data[10] = distance.clamp(0, 1000);
        data[11] = volume.clamp(0, 0x7FFF);
        let slide = slide.clone();
        ctx.aems.set_member_data(&slide);
    }
}
