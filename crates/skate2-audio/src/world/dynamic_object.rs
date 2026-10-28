//! The DYNAMICOBJECT state (`eMAINMAPSTATES` 8): `CSTATEMGR_DynamicObject`, `CSTATE_DynamicObject`,
//! the `SFXCTL_3DDynamicObjectPos` (0x00080000) and `SFXCTL_DynamicObjectPhysics` (0x00080010)
//! controllers, and `SFXObj_Dynamic` (0x00080000) / `SFXObj_Moving` (0x00080010), which
//! `EnterGamePlay` 0x82557a18 sets up in five states (`Setup(3)`).
//!
//! `CSTATE_Base::SortSFXCtl` 0x82a7de80 orders controllers by index, so the position controller
//! updates before the physics one and reads the previous update's position and velocity.

mod dynamic;
mod moving;

use std::collections::HashMap;
use std::sync::Arc;

use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::presentation::SkaterData;
use crate::random::RwRandom;
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::system::WorldAudioState;
use crate::utils::{clamp_fsel, fsel};
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};
use crate::world::whoosh::WhooshManager;
use dynamic::Dynamic;
use moving::Moving;

/// `eMAINMAPSTATES` 8.
const STATE: u32 = 8;
/// `aud_dynamicobject`, `aud_dynamicobjectnotch`, `aud_dynamicobjectrandom` and
/// `livingworld_dynamicobject_characteristics` class keys, and the shared "default" collection.
const AUD_DYNAMICOBJECT: u64 = 0xB05F_7C74_C63A_CFB8;
const AUD_DYNAMICOBJECTNOTCH: u64 = 0xDCD3_64A8_78BA_0AA3;
const AUD_DYNAMICOBJECTRANDOM: u64 = 0x74F2_B413_2A1D_39EA;
const LW_DMO_CHARACTERISTICS: u64 = 0xCCB7_C068_68EB_1C87;
const DEFAULT_KEY: u64 = 0xD7ED_BD36_2D7D_2152;
/// `livingworld_dynamicobject_characteristics` attributes `SFXCTL_DynamicObjectPhysics::UpdateParams`
/// reads when the object is not sliding (and the bool that says it never slides).
const CHAR_NO_SLIDE: u64 = 0x7E98_38A5_9060_2713;
const CHAR_PRIMARY_KEY: u64 = 0x64B6_475E_A79A_904D;
const CHAR_SECONDARY_KEY: u64 = 0x1DD2_8458_327D_DE7B;
const CHAR_PRIMARY_VOLUME: u64 = 0xB7B8_E216_6895_035E;
const CHAR_SECONDARY_VOLUME: u64 = 0xE119_794F_275D_9260;
/// `eMoveType`.
const MOVE_NONE: i32 = 0;
const MOVE_PUSH: i32 = 1;
const MOVE_PULL: i32 = 2;
/// `eChallengeTypes_Point2PointRace`, `DeathRace`, `SpotRace`.
const RACES: [i32; 3] = [18, 19, 20];
/// `CSTATEMGR_DynamicObject::mSwapBank` slots (`eSwapBank_Moving`, `eSwapBank_Random`).
const SWAP_MOVING: usize = 0;
const SWAP_RANDOM: usize = 1;
/// `SPLICE_TYPE` 22, the bank `SFXObj_Moving` loads its random splices into.
const SPLICE_DMO_RANDOM: usize = 22;
/// `3.5999999f`, metres per second to km/h.
const KPH: f32 = f32::from_bits(0x4066_6666);

/// `Sk8::AudioPresentation::DynamicData` as `AudioPresentation::DoSimSubmit` 0x827fb2f8 fills it
/// for each dynamic object `GetDynamicObjectsWithinRange` returns within 30 m of the local
/// skater: `objectID`, `DynamicObjectStateOut::GetVelocity`, `grabbedObject` (the local skater's
/// grab spline belongs to it), the `livingworld_dynamicobject_characteristics` collection key,
/// part 0's angular velocity, its strongest contact's force and `GetAudioMaterial` of the contact
/// surface (`onGround` when it has any contact) and `onSide` (|up . y| < 0.3). `bounds_center` is
/// `cLivingWorldPresEntityManager::GetPresDynamicObject(id)->GetBoundsCenter()`, `None` without a
/// presentation object.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DynamicData {
    pub object_id: u32,
    pub object_vel: Vec3,
    pub grabbed_object: bool,
    pub dynamic_type: u64,
    pub angular_vel: Vec3,
    pub collision_force: f32,
    pub on_ground: bool,
    pub collision_material: u32,
    pub on_side: bool,
    pub bounds_center: Option<Vec3>,
}

/// The files `AudioAssetManager::AsyncLoadToSwapSlot` 0x82a7a340 resolves for the DYNAMICOBJECT
/// swap banks, keyed by the path `SFXObj_Moving::UpdateParams` passes: `data\audio\splicerdmo.big|`
/// and the `aud_dynamicobject` bank name (after `xcore::ConvertSlashes`), and the bare
/// `aud_dynamicobjectrandom` splice bank names. `rw::core::BigHandler::Open` 0x82bb87c0 looks for a
/// `big|name` path only in that BIG and for a bare name in every mounted one; a path the host
/// leaves out stays loading, as a failed native load does.
#[derive(Debug, Clone, Default)]
pub struct DmoSwapFiles(pub HashMap<String, Arc<[u8]>>);

/// The ObjectID of a DYNAMICOBJECT object or controller (`kind` 0x40000000 or 0x60000000).
fn dmix_io(map: Option<&MixMap>, kind: u32, instance: u32, sfx: u32) -> Option<DMixIoId> {
    map.and_then(|map| map.dmixio(kind | (STATE << 16) | (instance << 11) | (sfx << 4)))
}

fn layout_byte(vault: &Vault, collection: Collection, offset: u32) -> u8 {
    (vault.layout_word(collection, offset / 4) >> (24 - 8 * (offset % 4))) as u8
}

fn layout_u64(vault: &Vault, collection: Collection, word: u32) -> u64 {
    (u64::from(vault.layout_word(collection, word)) << 32) | u64::from(vault.layout_word(collection, word + 1))
}

/// The collection key of a 24-byte `Attrib::RefSpec` attribute (bytes 8..16).
fn ref_spec_key(vault: &Vault, collection: Collection, key: u64) -> u64 {
    let bytes = vault.bytes(collection, key, 0, 24);
    u64::from_be_bytes(bytes[8..16].try_into().expect("eight bytes"))
}

/// `Attrib::Gen::aud_dynamicobject` of `key`, or the "default" collection when it is missing (the
/// fallback `GetAttribVariables` and `CalcRotSpeed` take).
fn dynamic_object_or_default(vault: &Vault, key: u64) -> Collection {
    let collection = Collection::new(AUD_DYNAMICOBJECT, key);
    if vault.has_collection(collection) { collection } else { Collection::new(AUD_DYNAMICOBJECT, DEFAULT_KEY) }
}

/// `rw::math::Random() % (max - min) + min`, or `min` for an empty range
/// (`CSTATEMGR_DynamicObject::GetRand` 0x825503d0).
fn rand_range(rng: &mut RwRandom, min: i32, max: i32) -> i32 {
    if max <= min {
        return min;
    }
    (rng.next_u32() % max.wrapping_sub(min) as u32) as i32 + min
}

/// `XMVectorSin` 0x824b0060 of one lane: the angle reduced by `vrfin(x / 2pi)`, then the
/// 23rd-order odd polynomial in fused steps.
fn xm_sin(x: f32) -> f32 {
    const INV_TWO_PI: f32 = f32::from_bits(0x3E22_F983);
    const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
    const C: [u32; 11] = [
        0xBE2A_AAAB, 0x3C08_8889, 0xB950_0D01, 0x3638_EF1D, 0xB2D7_322B, 0x2F30_9231, 0xAB57_3F9F, 0x274A_963C, 0xA317_A4DA, 0x1EB8_DC78,
        0x9A3B_0DA1,
    ];
    let n = (x * INV_TWO_PI).round_ties_even();
    let v = (-TWO_PI).mul_add(n, x);
    let v2 = v * v;
    let mut power = v2 * v;
    let mut result = v;
    for coefficient in C {
        result = f32::from_bits(coefficient).mul_add(power, result);
        power *= v2;
    }
    result
}

/// `Sk8::Audio::SFXCTL_DynamicObjectPhysics` (0xB0 bytes).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Physics {
    pub(crate) io: Option<DMixIoId>,
    pub(crate) position: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) speed: f32,
    pub(crate) moving_speed: f32,
    pub(crate) rotational_velocity: Vec3,
    pub(crate) rotational_speed: f32,
    pub(crate) is_being_moved: bool,
    pub(crate) move_type: i32,
    pub(crate) attrib_key: u64,
    pub(crate) secondary_key: u64,
    pub(crate) collision_force: f32,
    pub(crate) collided_material: u32,
    pub(crate) primary_volume: f32,
    pub(crate) secondary_volume: f32,
    pub(crate) ground_change_count: i32,
    pub(crate) on_ground: bool,
    pub(crate) sliding: bool,
}

impl Physics {
    /// The constructor 0x8251ad78.
    fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            position: ZERO,
            velocity: ZERO,
            speed: 0.0,
            moving_speed: 0.0,
            rotational_velocity: ZERO,
            rotational_speed: 0.0,
            is_being_moved: false,
            move_type: MOVE_PUSH,
            attrib_key: 0,
            secondary_key: 0,
            collision_force: 0.0,
            collided_material: 0,
            primary_volume: 0.0,
            secondary_volume: 0.0,
            ground_change_count: 0,
            on_ground: true,
            sliding: false,
        }
    }

    /// `InitSFX` (`SFXCTL_SpeakerPhysics::InitSFX` 0x82512de0, shared): the mixer output on, the
    /// position and velocity zeroed and handed to the position controller.
    fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.velocity = ZERO;
        self.position = ZERO;
    }

    /// `Detach` 0x82512ea0.
    fn detach(&mut self) {
        self.is_being_moved = false;
        self.speed = 0.0;
        self.move_type = MOVE_PUSH;
        self.moving_speed = 0.0;
        self.on_ground = true;
        self.ground_change_count = 0;
    }

    /// `GetIsBeingMoved` 0x8251af90: never in the race challenges. The `MusicSystem` target state's
    /// `MUSIC_CHASE` mode also clears it; the music system is not ported, so it never applies.
    pub(crate) fn get_is_being_moved(&self, challenge_type: i32) -> bool {
        !RACES.contains(&challenge_type) && self.is_being_moved
    }

    /// `CalcRotSpeed` 0x8251ae88.
    fn calc_rot_speed(&self, vault: &Vault) -> i32 {
        let attrib = dynamic_object_or_default(vault, self.attrib_key);
        let ratio = clamp_fsel((self.rotational_speed * KPH) / vault.layout_f32(attrib, 78), 1.0);
        fctiwz(ratio * 32767.0)
    }

    /// `UpdateParams` 0x8253e4c8. Returns the key it gives the state (`mMoveableObjectKey`), or
    /// `None` when the object is not in `AudioPresentation::mDynamicObject` (nothing updates).
    fn update_params(&mut self, object_id: u32, world: &WorldAudioState, local: Option<&SkaterData>, vault: &Vault, map: &mut MixMap) -> Option<u64> {
        let objects = &world.dynamic_objects;
        let data = objects.iter().find(|data| data.object_id == object_id)?;
        if let Some(center) = data.bounds_center {
            self.position = center;
            if data.grabbed_object
                && let Some(local) = local
            {
                self.position = local.skater_pos;
            }
        }
        self.velocity = data.object_vel;
        self.speed = vmx::length(self.velocity);
        self.rotational_velocity = [0.0, data.angular_vel[1], 0.0, data.angular_vel[3]];
        self.rotational_speed = vmx::length(self.rotational_velocity);
        if data.grabbed_object && self.rotational_speed > self.speed {
            self.speed = self.rotational_speed;
        }
        let threshold = f32::from_bits(0x3E4C_CCCD);
        if data.grabbed_object && self.speed > threshold {
            self.is_being_moved = true;
        } else if self.is_being_moved && !(self.speed > threshold) {
            self.is_being_moved = false;
        }
        if self.is_being_moved && objects.iter().any(|other| other.object_id != object_id && other.grabbed_object) {
            self.is_being_moved = false;
        }
        self.move_type = MOVE_PUSH;
        if self.is_being_moved
            && let Some(local) = local
            && vmx::dot3(self.velocity, local.skater_look) < 0.0
        {
            self.move_type = MOVE_PULL;
        }
        self.attrib_key = 0;
        self.secondary_key = 0;
        self.secondary_volume = 1.0;
        self.primary_volume = 1.0;
        let characteristics = Collection::new(LW_DMO_CHARACTERISTICS, data.dynamic_type);
        self.sliding = true;
        if vault.bool(characteristics, CHAR_NO_SLIDE) {
            self.sliding = false;
        }
        if layout_byte(vault, characteristics, 0x7C) != 0 && data.on_side {
            self.sliding = false;
        }
        if vault.has_collection(characteristics) {
            if self.sliding {
                self.attrib_key = layout_u64(vault, characteristics, 16);
                self.secondary_key = layout_u64(vault, characteristics, 10);
                self.primary_volume = vault.layout_f32(characteristics, 24);
                self.secondary_volume = vault.layout_f32(characteristics, 22);
            } else {
                self.attrib_key = ref_spec_key(vault, characteristics, CHAR_PRIMARY_KEY);
                self.secondary_key = ref_spec_key(vault, characteristics, CHAR_SECONDARY_KEY);
                self.primary_volume = vault.f32(characteristics, CHAR_PRIMARY_VOLUME);
                self.secondary_volume = vault.f32(characteristics, CHAR_SECONDARY_VOLUME);
            }
        }
        let attrib = Collection::new(AUD_DYNAMICOBJECT, self.attrib_key);
        if vault.has_collection(attrib) && self.speed > self.moving_speed {
            self.moving_speed = vault.layout_f32(attrib, 76) + self.moving_speed;
        } else {
            self.moving_speed = self.speed;
        }
        let low = fsel(-self.moving_speed, 0.0, self.moving_speed);
        self.moving_speed = fsel(self.speed - low, low, self.speed);
        self.collision_force = data.collision_force;
        self.collided_material = data.collision_material;
        if data.on_ground == self.on_ground {
            self.ground_change_count = 0;
        } else {
            self.ground_change_count += 1;
            let mut game_speed = world.player.game_speed;
            if f64::from(game_speed) < 0.1 {
                game_speed = f32::from_bits(0x3DCC_CCCD);
            }
            if self.ground_change_count >= fctiwz(10.0 / game_speed) {
                self.on_ground = data.on_ground;
            }
        }
        let key = if data.grabbed_object { self.attrib_key } else { 0 };
        let rot = if self.is_being_moved { self.calc_rot_speed(vault) } else { 0 };
        if let Some(io) = self.io {
            map.set_input(io, 0, rot.clamp(0, 0x7FFF));
        }
        Some(key)
    }
}

/// `tNotchAttrib`.
#[derive(Debug, Clone, Copy, Default)]
struct NotchAttrib {
    gain: f32,
    gain_rand_min: i32,
    gain_rand_max: i32,
    gain_rand_rate_min: i32,
    gain_rand_rate_max: i32,
    q: f32,
    min_speed: f32,
    max_speed: f32,
    min_freq: f32,
    max_freq: f32,
    lfo_depth: f32,
    lfo_depth_rand_min: i32,
    lfo_depth_rand_max: i32,
    lfo_depth_rand_rate_min: i32,
    lfo_depth_rand_rate_max: i32,
    lfo_frequency: f32,
    freq_rand_min: i32,
    freq_rand_max: i32,
    freq_rand_rate_min: i32,
    freq_rand_rate_max: i32,
    lfo_time: f32,
    lfo_value: f32,
    cur_depth_rand: f32,
    cur_depth_rand_rate: f32,
    cur_gain_rand: f32,
    cur_gain_rand_rate: f32,
    cur_freq_rand: f32,
    cur_freq_rand_rate: f32,
}

/// `tVolModulation`.
#[derive(Debug, Clone, Copy, Default)]
struct VolModulation {
    min_speed: f32,
    max_speed: f32,
    min_vol_at_min: i32,
    max_vol_at_min: i32,
    min_rate_at_min: i32,
    max_rate_at_min: i32,
    min_vol_at_max: i32,
    max_vol_at_max: i32,
    min_rate_at_max: i32,
    max_rate_at_max: i32,
    cur_time: f32,
    cur_vol: i32,
}

/// `tPitchModulation`.
#[derive(Debug, Clone, Copy, Default)]
struct PitchModulation {
    min_speed: f32,
    max_speed: f32,
    min_duck: i32,
    max_duck: i32,
    min_rate_at_min: i32,
    max_rate_at_min: i32,
    min_rate_at_max: i32,
    max_rate_at_max: i32,
    pitch_duration_min: i32,
    pitch_duration_max: i32,
    cur_time: f32,
    cur_pitch_duck: i32,
    ducking: bool,
}

/// The `CSTATEMGR_DynamicObject` half `SFXObj_Moving` drives: the two moveable object voices
/// (`CreateMoveableObjectVoice` 0x825548f0: stage 3, `SubMix`, `Gain`, `FrequencyShiftSsb`, two
/// `PeakingIir2`, `Pan2D1` to six channels and a `Send` to the master mix) and their modulation.
/// `SkateSamplePlayerFactory::CreateInstance` routes AEMS patch 2048 into voice 1's sub mix and
/// patch 512 into voice 2's.
#[derive(Debug, Default)]
pub(crate) struct MoveableChains {
    chain1: Option<VoiceId>,
    chain2: Option<VoiceId>,
    moveable_object_key: u64,
    frequency_shift_push: f32,
    frequency_shift_pull: f32,
    current_move_type: i32,
    pub(crate) vel_kph: f32,
    notch: [NotchAttrib; 4],
    delta_time: f32,
    vol_mod: [VolModulation; 2],
    pitch_mod: PitchModulation,
    chain2_volume_scale: f32,
    chain2_pitch_offset: i32,
}

impl MoveableChains {
    fn plugin(voice: Option<VoiceId>, index: u8) -> Option<PluginRef> {
        voice.map(|voice| PluginRef { voice, index })
    }

    fn set(core: &mut AudioCore, voice: Option<VoiceId>, index: u8, attribute: usize, value: f32) {
        if let Some(plugin) = Self::plugin(voice, index) {
            core.set_attribute(plugin, attribute, value);
        }
    }

    /// `CreateMoveableObjectVoice` 0x825548f0, which `EnterGamePlay` runs; the port runs it on the
    /// first update, when the audio core is at hand. Voice 2 starts with its gain at 0.
    fn create_voices(&mut self, core: &mut AudioCore, master: PluginRef) {
        let configs = [
            PlugInConfig { kind: PlugInKind::SubMix { name: String::new() }, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Gain, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::FrequencyShiftSsb { constructor: 0.0 }, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::PeakingIir2, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::PeakingIir2, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Pan2D1 { constructor: None }, output_channels: 6 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
        ];
        self.chain1 = core.create_voice(3, "STATEMGR_DynamicObject.cpp", &configs);
        if let Some(send) = Self::plugin(self.chain1, 6) {
            core.connect_send(send, Some(master));
        }
        self.chain2 = core.create_voice(3, "STATEMGR_DynamicObject.cpp", &configs);
        if let Some(send) = Self::plugin(self.chain2, 6) {
            core.connect_send(send, Some(master));
        }
        Self::set(core, self.chain2, 1, 0, 0.0);
    }

    /// `GetMoveableObjectVoice` 0x825503b0 / `GetMoveableObjectVoice2` 0x825503c0: the sub mixes
    /// AEMS patches 2048 and 512 play into.
    #[must_use]
    pub fn moveable_object_voices(&self) -> [Option<PluginRef>; 2] {
        [Self::plugin(self.chain1, 0), Self::plugin(self.chain2, 0)]
    }

    /// `SetMoveType` 0x82554d30.
    pub(crate) fn set_move_type(&mut self, move_type: i32, core: &mut AudioCore) {
        if move_type == self.current_move_type {
            return;
        }
        self.current_move_type = move_type;
        let shift = match move_type {
            MOVE_PUSH => self.frequency_shift_push,
            MOVE_PULL => self.frequency_shift_pull,
            _ => return,
        };
        Self::set(core, self.chain1, 2, 0, shift);
        Self::set(core, self.chain2, 2, 0, shift);
    }

    /// `SetMoveableObjectKey` 0x82557bf8: a new key reloads the attributes and restarts the
    /// modulation; the frequency shifts come from the key's collection either way.
    pub(crate) fn set_moveable_object_key(&mut self, key: u64, vault: &Vault) {
        if self.moveable_object_key != key {
            self.moveable_object_key = key;
            self.get_attributes(vault);
            for notch in &mut self.notch {
                notch.lfo_time = 0.0;
                notch.lfo_value = 0.0;
                notch.cur_depth_rand = 0.0;
                notch.cur_depth_rand_rate = 0.0;
                notch.cur_freq_rand_rate = 0.0;
                notch.cur_gain_rand = 0.0;
                notch.cur_gain_rand_rate = 0.0;
                notch.cur_freq_rand = 0.0;
            }
            for vol in &mut self.vol_mod {
                vol.cur_time = 0.0;
                vol.cur_vol = 0x7FFF;
            }
            self.pitch_mod.cur_time = 0.0;
            self.pitch_mod.cur_pitch_duck = 4096;
            self.pitch_mod.ducking = false;
        }
        let attrib = Collection::new(AUD_DYNAMICOBJECT, key);
        if vault.has_collection(attrib) {
            self.frequency_shift_push = vault.layout_f32(attrib, 41);
            self.frequency_shift_pull = vault.layout_f32(attrib, 42);
        }
    }

    /// `GetAttributes` 0x82554e18: the `aud_dynamicobjectnotch` collection the key's
    /// `aud_dynamicobject` names (`RefSpec` at layout 48).
    fn get_attributes(&mut self, vault: &Vault) {
        let attrib = Collection::new(AUD_DYNAMICOBJECT, self.moveable_object_key);
        if !vault.has_collection(attrib) {
            return;
        }
        let notch = Collection::new(AUD_DYNAMICOBJECTNOTCH, layout_u64(vault, attrib, 14));
        let f = |word| vault.layout_f32(notch, word);
        let i = |word| vault.layout_word(notch, word) as i32;
        let ms = |word| fctiwz(f(word) * 1000.0);
        let n = &mut self.notch[0];
        n.gain = f(73);
        n.q = f(72);
        n.min_speed = f(74);
        n.max_speed = f(76);
        n.min_freq = f(75);
        n.max_freq = f(77);
        n.lfo_depth = f(83);
        n.lfo_depth_rand_min = ms(81);
        n.lfo_depth_rand_max = ms(82);
        n.lfo_depth_rand_rate_min = i(79);
        n.lfo_depth_rand_rate_max = i(80);
        n.lfo_frequency = f(78);
        let n = &mut self.notch[1];
        n.gain = f(63);
        n.gain_rand_min = ms(61);
        n.gain_rand_max = ms(62);
        n.gain_rand_rate_min = i(59);
        n.gain_rand_rate_max = i(60);
        n.q = f(58);
        n.min_speed = f(64);
        n.max_speed = f(66);
        n.min_freq = f(65);
        n.max_freq = f(67);
        n.lfo_frequency = f(70);
        n.freq_rand_min = ms(70);
        n.freq_rand_max = ms(71);
        n.freq_rand_rate_min = i(68);
        n.freq_rand_rate_max = i(69);
        let n = &mut self.notch[2];
        n.gain = f(27);
        n.q = f(26);
        n.min_speed = f(28);
        n.max_speed = f(30);
        n.min_freq = f(29);
        n.max_freq = f(31);
        n.lfo_depth = f(37);
        n.lfo_depth_rand_min = ms(35);
        n.lfo_depth_rand_max = ms(36);
        n.lfo_depth_rand_rate_min = i(33);
        n.lfo_depth_rand_rate_max = i(34);
        n.lfo_frequency = f(32);
        let n = &mut self.notch[3];
        n.gain = f(17);
        n.gain_rand_min = ms(15);
        n.gain_rand_max = ms(16);
        n.gain_rand_rate_min = i(13);
        n.gain_rand_rate_max = i(14);
        n.q = f(12);
        n.min_speed = f(18);
        n.max_speed = f(20);
        n.min_freq = f(19);
        n.max_freq = f(21);
        n.lfo_frequency = f(24);
        n.freq_rand_min = ms(24);
        n.freq_rand_max = ms(25);
        n.freq_rand_rate_min = i(22);
        n.freq_rand_rate_max = i(23);
        for (vol, base) in self.vol_mod.iter_mut().zip([[46, 47, 42, 43, 38, 39, 44, 45, 40, 41], [9, 10, 5, 6, 1, 2, 7, 8, 3, 4]]) {
            vol.min_speed = f(base[0]);
            vol.max_speed = f(base[1]);
            if vol.max_speed <= vol.min_speed {
                vol.max_speed = vol.min_speed + 1.0;
            }
            vol.min_vol_at_min = i(base[2]);
            vol.max_vol_at_min = i(base[3]);
            vol.min_rate_at_min = i(base[4]);
            vol.max_rate_at_min = i(base[5]);
            vol.min_vol_at_max = i(base[6]);
            vol.max_vol_at_max = i(base[7]);
            vol.min_rate_at_max = i(base[8]);
            vol.max_rate_at_max = i(base[9]);
        }
        let pitch = &mut self.pitch_mod;
        pitch.min_speed = f(52);
        pitch.max_speed = f(53);
        if pitch.max_speed <= pitch.min_speed {
            pitch.max_speed = pitch.min_speed + 1.0;
        }
        pitch.min_duck = i(56);
        pitch.max_duck = i(57);
        pitch.min_rate_at_min = i(48);
        pitch.max_rate_at_min = i(49);
        pitch.min_rate_at_max = i(50);
        pitch.max_rate_at_max = i(51);
        pitch.pitch_duration_min = i(54);
        pitch.pitch_duration_max = i(55);
        self.chain2_volume_scale = i(0) as f32 * crate::player::VOLUME_SCALE;
        self.chain2_pitch_offset = i(11);
    }

    /// `GetPitchDuck` 0x82550ad8.
    pub(crate) fn pitch_duck(&self) -> i32 {
        if self.moveable_object_key != 0 { self.pitch_mod.cur_pitch_duck } else { 0 }
    }

    /// `GetChain2PitchDuck` 0x82550af8.
    pub(crate) fn chain2_pitch_duck(&self) -> i32 {
        if self.moveable_object_key != 0 { self.chain2_pitch_offset + self.pitch_mod.cur_pitch_duck } else { 0 }
    }

    /// `UpdateNotchSweep` 0x82555248 (`MainNotch1` and `ModNotch1`): a 16.666666 ms LFO step over
    /// `lfoFrequency` ms, its depth randomised on a timer, around the speed-mapped frequency.
    fn update_notch_sweep(&mut self, index: usize, rng: &mut RwRandom) {
        let vel_kph = self.vel_kph;
        let delta_time = self.delta_time;
        let n = &mut self.notch[index];
        let advanced = n.lfo_time + f32::from_bits(0x4185_5555);
        let period = n.lfo_frequency;
        n.lfo_time = advanced;
        if advanced > period {
            n.lfo_time = 0.0;
        }
        if period <= 0.0 {
            n.lfo_frequency = 500.0;
        }
        let sin = xm_sin((n.lfo_time / n.lfo_frequency) * f32::from_bits(0x40C9_0FDB));
        let rate = n.cur_depth_rand_rate - delta_time;
        n.cur_depth_rand_rate = rate;
        let depth = n.lfo_depth;
        if rate <= 0.0 {
            n.cur_depth_rand = rand_range(rng, n.lfo_depth_rand_min, n.lfo_depth_rand_max) as f32 * 0.001;
            if rng.next_u32() & 1 == 1 {
                n.cur_depth_rand *= -1.0;
            }
            n.cur_depth_rand_rate = rand_range(rng, n.lfo_depth_rand_rate_min, n.lfo_depth_rand_rate_max) as f32 * 0.001;
        }
        let mut amount = n.cur_depth_rand + depth;
        if amount < 0.0 {
            amount = 0.0;
        }
        let swing = (sin * amount) * 0.5;
        let mut freq = n.min_freq;
        if vel_kph <= n.max_speed {
            if vel_kph > n.min_speed {
                let fraction = if n.max_speed > n.min_speed { (vel_kph - n.min_speed) / (n.max_speed - n.min_speed) } else { 1.0 };
                freq = (n.max_freq - n.min_freq).mul_add(fraction, n.min_freq);
            }
        } else {
            freq = n.max_freq;
        }
        n.lfo_value = if freq + swing >= n.min_freq { freq + swing } else { n.min_freq };
    }

    /// `UpdateNotchGain` 0x82550470 (`MainNotch2` and `ModNotch2`): a random gain around
    /// `mNotchAttrib[1].gain` (for both), clamped to 0.1..20, on a random timer.
    fn update_notch_gain(&mut self, index: usize, rng: &mut RwRandom) {
        let base_gain = self.notch[1].gain;
        let delta_time = self.delta_time;
        let n = &mut self.notch[index];
        let rate = n.cur_gain_rand_rate - delta_time;
        n.cur_gain_rand_rate = rate;
        if rate <= 0.0 {
            n.cur_gain_rand = rand_range(rng, n.gain_rand_min, n.gain_rand_max) as f32 * 0.001;
            if rng.next_u32() & 1 == 1 {
                n.cur_gain_rand *= -1.0;
            }
            let gain = base_gain + n.cur_gain_rand;
            let low = 0.1_f32;
            let clamped = fsel(low - gain, low, gain);
            n.cur_gain_rand = fsel(20.0 - clamped, clamped, 20.0);
            n.cur_gain_rand_rate = rand_range(rng, n.gain_rand_rate_min, n.gain_rand_rate_max) as f32 * 0.001;
        }
    }

    /// `UpdateNotchFrequency` 0x825505d8: a random frequency on a timer whose range follows the
    /// speed.
    fn update_notch_frequency(&mut self, index: usize, rng: &mut RwRandom) {
        let vel_kph = self.vel_kph;
        let delta_time = self.delta_time;
        let n = &mut self.notch[index];
        let rate = n.cur_freq_rand_rate - delta_time;
        n.cur_freq_rand_rate = rate;
        if rate <= 0.0 {
            n.cur_freq_rand = rand_range(rng, n.freq_rand_min, n.freq_rand_max) as f32 * 0.001;
            let mut limit = n.freq_rand_rate_min;
            if vel_kph <= n.max_speed {
                if vel_kph > n.min_speed {
                    let fraction = if n.max_speed > n.min_speed { (vel_kph - n.min_speed) / (n.max_speed - n.min_speed) } else { 1.0 };
                    limit = fctiwz((n.freq_rand_rate_max - n.freq_rand_rate_min) as f32 * fraction) + n.freq_rand_rate_min;
                }
            } else {
                limit = n.freq_rand_rate_max;
            }
            let wait = if limit > 0 { rng.next_u32() % limit as u32 } else { 0 };
            n.cur_freq_rand_rate = wait as f32 * 0.001;
        }
    }

    /// `UpdateVolumeModulation` 0x82550758: a new random volume and hold time whenever the hold
    /// runs out, both ranges following the speed.
    fn update_volume_modulation(&mut self, index: usize, rng: &mut RwRandom) {
        let vel_kph = self.vel_kph;
        let delta_time = self.delta_time;
        let m = &mut self.vol_mod[index];
        let left = m.cur_time - delta_time;
        m.cur_time = left;
        if left < 0.0 {
            let (mut min_vol, mut max_vol, mut min_rate, mut max_rate) = (m.min_vol_at_min, m.max_vol_at_min, m.min_rate_at_min, m.max_rate_at_min);
            if vel_kph < m.max_speed {
                if vel_kph > m.min_speed {
                    let fraction = (vel_kph - m.min_speed) / (m.max_speed - m.min_speed);
                    min_vol = fctiwz((m.min_vol_at_max - m.min_vol_at_min) as f32 * fraction) + m.min_vol_at_min;
                    min_rate = fctiwz((m.min_rate_at_max - m.min_rate_at_min) as f32 * fraction) + m.min_rate_at_min;
                    max_vol = fctiwz((m.max_vol_at_max - m.max_vol_at_min) as f32 * fraction) + m.max_vol_at_min;
                    max_rate = fctiwz((m.max_rate_at_max - m.max_rate_at_min) as f32 * fraction) + m.max_rate_at_min;
                }
            } else {
                (min_vol, max_vol, min_rate, max_rate) = (m.min_vol_at_max, m.max_vol_at_max, m.min_rate_at_max, m.max_rate_at_max);
            }
            if min_vol > max_vol {
                max_vol += (rng.next_u32() % min_vol.wrapping_sub(max_vol) as u32) as i32;
            }
            m.cur_vol = max_vol;
            if max_rate > min_rate {
                min_rate += (rng.next_u32() % max_rate.wrapping_sub(min_rate) as u32) as i32;
            }
            m.cur_time = min_rate as f32 * 0.001;
        }
    }

    /// `UpdatePitchModulation` 0x82550940: alternates a random pitch duck held for a random
    /// duration with an undocked gap whose length follows the speed.
    fn update_pitch_modulation(&mut self, rng: &mut RwRandom) {
        let vel_kph = self.vel_kph;
        let m = &mut self.pitch_mod;
        let left = m.cur_time - self.delta_time;
        m.cur_time = left;
        if left < 0.0 {
            if m.ducking {
                m.cur_pitch_duck = 0;
                let (mut low, mut high) = (m.min_rate_at_min, m.max_rate_at_min);
                if vel_kph < m.max_speed {
                    if vel_kph > m.min_speed {
                        let fraction = (vel_kph - m.min_speed) / (m.max_speed - m.min_speed);
                        low = fctiwz((m.min_rate_at_max - m.min_rate_at_min) as f32 * fraction) + m.min_rate_at_min;
                        high = fctiwz((m.max_rate_at_max - m.max_rate_at_min) as f32 * fraction) + m.max_rate_at_min;
                    }
                } else {
                    (low, high) = (m.min_rate_at_max, m.max_rate_at_max);
                }
                m.cur_time = rand_range(rng, low, high) as f32 * 0.001;
            } else {
                m.cur_pitch_duck = rand_range(rng, m.min_duck, m.max_duck);
                m.cur_time = rand_range(rng, m.pitch_duration_min, m.pitch_duration_max) as f32 * 0.001;
            }
            m.ducking = !m.ducking;
        }
    }

    /// `UpdateNotchPlugins` 0x825554a0. The mod chain's second notch takes `MainNotch2`'s
    /// frequency update too, as natively (`UpdateNotchFrequency(eNotchType_MainNotch2)` twice).
    fn update_notch_plugins(&mut self, core: &mut AudioCore, rng: &mut RwRandom) {
        if self.moveable_object_key == 0 {
            return;
        }
        self.update_notch_sweep(0, rng);
        Self::set(core, self.chain1, 3, 1, self.notch[0].gain);
        Self::set(core, self.chain1, 3, 2, self.notch[0].q);
        Self::set(core, self.chain1, 3, 0, self.notch[0].lfo_value);
        self.update_notch_gain(1, rng);
        self.update_notch_frequency(1, rng);
        Self::set(core, self.chain1, 4, 1, self.notch[1].cur_gain_rand);
        Self::set(core, self.chain1, 4, 2, self.notch[1].q);
        Self::set(core, self.chain1, 4, 0, self.notch[1].cur_freq_rand);
        self.update_notch_sweep(2, rng);
        Self::set(core, self.chain2, 3, 1, self.notch[2].gain);
        Self::set(core, self.chain2, 3, 2, self.notch[2].q);
        Self::set(core, self.chain2, 3, 0, self.notch[2].lfo_value);
        self.update_notch_gain(3, rng);
        self.update_notch_frequency(1, rng);
        Self::set(core, self.chain2, 4, 1, self.notch[3].cur_gain_rand);
        Self::set(core, self.chain2, 4, 2, self.notch[3].q);
        Self::set(core, self.chain2, 4, 0, self.notch[3].cur_freq_rand);
    }

    /// `UpdateGainPlugins` 0x82555660.
    fn update_gain_plugins(&mut self, core: &mut AudioCore, rng: &mut RwRandom) {
        if self.moveable_object_key == 0 {
            return;
        }
        self.update_volume_modulation(0, rng);
        let main = self.vol_mod[0].cur_vol as f32 * crate::player::VOLUME_SCALE;
        Self::set(core, self.chain1, 1, 0, main);
        self.update_volume_modulation(1, rng);
        let modulated = ((self.vol_mod[1].cur_vol as u32 as f32 * crate::player::VOLUME_SCALE) * main) * self.chain2_volume_scale;
        Self::set(core, self.chain2, 1, 0, modulated);
    }
}

/// `AudioAssetManager`'s status of a swap slot: empty, loading the file at a path, or loaded.
#[derive(Debug, Clone, Default)]
pub(crate) enum SwapBank {
    #[default]
    Empty,
    Loading(String),
    Loaded,
}

/// `eAssetType_AEMSBank` (the moveable bank) or `eAssetType_SplicerData` (the random splices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwapAsset {
    AemsBank,
    Splicer,
}

/// `CSTATEMGR_DynamicObject::mSwapBank`: the 409600- and 122880-byte swap banks `EnterGamePlay`
/// creates, one slot each.
#[derive(Debug, Default)]
pub(crate) struct SwapBanks {
    banks: [SwapBank; 2],
    assets: [Option<SwapAsset>; 2],
}

impl SwapBanks {
    /// `SwapSpaceEmpty` 0x82554708.
    pub(crate) fn empty(&self, bank: usize) -> bool {
        matches!(self.banks[bank], SwapBank::Empty)
    }

    /// `SwapDataLoaded` 0x82554768.
    pub(crate) fn loaded(&self, bank: usize) -> bool {
        matches!(self.banks[bank], SwapBank::Loaded)
    }

    /// `AsyncLoadToSwapSpace` 0x825547d0.
    pub(crate) fn async_load(&mut self, path: String, bank: usize) {
        self.banks[bank] = SwapBank::Loading(path);
        self.assets[bank] = Some(if bank == SWAP_MOVING { SwapAsset::AemsBank } else { SwapAsset::Splicer });
    }

    /// `UnloadSwapSpace` 0x82554890.
    pub(crate) fn unload(&mut self, bank: usize, ctx: &mut SfxCtx<'_>) {
        if matches!(self.banks[bank], SwapBank::Loaded) && self.assets[bank] == Some(SwapAsset::Splicer) {
            ctx.splicer.unload_bank(SPLICE_DMO_RANDOM);
        }
        self.banks[bank] = SwapBank::Empty;
    }

    /// `AudioAssetManager::Update`'s completion of the pending loads, one update after the request:
    /// a resolved file finishes (`ResolveData`: the AEMS bank is added, the splice bank loaded into
    /// `SPLICE_TYPE` 22); a missing one stays loading.
    fn complete(&mut self, files: &DmoSwapFiles, ctx: &mut SfxCtx<'_>) {
        for bank in 0..2 {
            let SwapBank::Loading(path) = &self.banks[bank] else { continue };
            let Some(file) = files.0.get(path) else { continue };
            let path = path.clone();
            let resolved = match self.assets[bank] {
                Some(SwapAsset::AemsBank) => ctx.aems.add_module_bank(ctx.core, &path, Arc::clone(file)).is_some(),
                Some(SwapAsset::Splicer) => ctx.splicer.load_bank(Arc::clone(file), SPLICE_DMO_RANDOM).is_ok(),
                None => false,
            };
            if resolved {
                self.banks[bank] = SwapBank::Loaded;
            }
        }
    }
}

/// One `CSTATE_DynamicObject` (0x50 bytes) with its controllers and objects.
struct DynamicObjectState {
    attached: bool,
    object_id: u32,
    moveable_object_key: u64,
    delta_time: f32,
    position: Obj3d,
    physics: Physics,
    dynamic: Dynamic,
    moving: Moving,
}

impl DynamicObjectState {
    fn new(instance: u32, vault: &Vault, map: Option<&MixMap>) -> Self {
        let mut position = Obj3d::new(vault);
        position.io = dmix_io(map, 0x6000_0000, instance, 0);
        Self {
            attached: false,
            object_id: 0,
            moveable_object_key: 0,
            delta_time: 0.0,
            position,
            physics: Physics::new(dmix_io(map, 0x6000_0000, instance, 1)),
            dynamic: Dynamic::new(dmix_io(map, 0x4000_0000, instance, 0)),
            moving: Moving::new(dmix_io(map, 0x4000_0000, instance, 1)),
        }
    }

    /// `CSTATE_DynamicObject::Attach` 0x825567c0 (`mTick`, which nothing ported reads, and
    /// `mObjectID`) and `CSTATE_Base::Attach`: the controllers' `InitSFX` in index order, then the
    /// objects' (`SetupLoadData`).
    fn attach(&mut self, object_id: u32, ctx: &mut SfxCtx<'_>) {
        self.object_id = object_id;
        self.attached = true;
        if let Some(io) = self.position.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.physics.init_sfx(ctx.map);
        self.dynamic.init_sfx(&self.physics, ctx);
        self.moving.init_sfx(&self.physics, ctx);
    }

    /// `CSTATE_DynamicObject::Detach` 0x82551910 / `CSTATE_Base::Detach` 0x82a7e350: the objects'
    /// `Destroy` with their mixer outputs off and inputs cleared, then the controllers'.
    fn detach(&mut self, swap: &mut SwapBanks, ctx: &mut SfxCtx<'_>) {
        self.attached = false;
        self.dynamic.destroy(ctx);
        self.moving.destroy(swap, ctx);
        for io in [self.dynamic.io, self.moving.io].into_iter().flatten() {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        self.position.detach();
        self.physics.detach();
        for io in [self.position.io, self.physics.io].into_iter().flatten() {
            ctx.map.clear_input(io);
        }
        self.object_id = 0;
    }
}

/// `CSTATEMGR_DynamicObject` (0x2B8 bytes).
pub struct DynamicObjectManager {
    states: Vec<DynamicObjectState>,
    num_dynamic_objects_around: i32,
    chains: MoveableChains,
    swap: SwapBanks,
    cur_time: f32,
}

impl DynamicObjectManager {
    /// `EnterGamePlay` 0x82557a18: five states, each with `SFXObj_Dynamic` and `SFXObj_Moving`.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        Self {
            states: (0..5).map(|instance| DynamicObjectState::new(instance, vault, map)).collect(),
            num_dynamic_objects_around: 0,
            chains: MoveableChains { current_move_type: MOVE_NONE, ..MoveableChains::default() },
            swap: SwapBanks::default(),
            cur_time: 0.0,
        }
    }

    /// `GetGrabbedObjectKey` 0x82550430 (`AudioSystem::GetGrabbedObjectKey` 0x824e69e8): the first
    /// attached state's non-zero `mMoveableObjectKey`, which the physics controller sets to the
    /// grabbed object's `aud_dynamicobject` key.
    #[must_use]
    pub fn grabbed_object_key(&self) -> u64 {
        self.states.iter().find(|state| state.attached && state.moveable_object_key != 0).map_or(0, |state| state.moveable_object_key)
    }

    /// The moveable object voices' sub mixes (`GetMoveableObjectVoice`, `GetMoveableObjectVoice2`).
    #[must_use]
    pub fn moveable_object_voices(&self) -> [Option<PluginRef>; 2] {
        self.chains.moveable_object_voices()
    }

    /// `UpdateParams` 0x825544a0: the first five objects of `AudioPresentation::mDynamicObject`
    /// (the grabbed one replacing the fifth when it is further out), states of objects no longer
    /// listed detached, free states attached to the new ones; then `CSTATEMGR_Base::UpdateParams`.
    pub fn update_params(
        &mut self,
        t: f32,
        world: &WorldAudioState,
        mics: &MicrophoneMan,
        shared: &mut Pos3dShared,
        mut whooshes: Option<&mut WhooshManager>,
        ctx: &mut SfxCtx<'_>,
    ) {
        if self.chains.chain1.is_none() {
            self.chains.create_voices(ctx.core, ctx.master_sub_mix);
        }
        self.swap.complete(&world.dmo_swap_files, ctx);
        let objects = &world.dynamic_objects;
        self.num_dynamic_objects_around = objects.len() as i32;
        let count = self.num_dynamic_objects_around.clamp(0, 5) as usize;
        let mut ids = [0u32; 5];
        let mut grabbed = false;
        for (slot, data) in objects.iter().take(count).enumerate() {
            ids[slot] = data.object_id;
            if data.grabbed_object {
                grabbed = true;
            }
        }
        if !grabbed {
            for data in objects {
                if data.grabbed_object {
                    ids[count - 1] = data.object_id;
                }
            }
        }
        for index in 0..self.states.len() {
            if !self.states[index].attached {
                continue;
            }
            match ids[..count].iter().position(|&id| id == self.states[index].object_id) {
                Some(slot) => ids[slot] = 0,
                None => self.states[index].detach(&mut self.swap, ctx),
            }
        }
        if self.num_dynamic_objects_around > 0 {
            for &id in &ids[..count] {
                if id == 0 {
                    continue;
                }
                if let Some(state) = self.states.iter_mut().find(|state| !state.attached) {
                    state.attach(id, ctx);
                }
            }
        }
        self.chains.delta_time = t;
        self.cur_time += t;
        let local = world.player.skaters.iter().find(|skater| skater.local);
        let Self { states, chains, swap, .. } = self;
        for state in states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { dir: None, pos: Some(state.physics.position), vel: Some(state.physics.velocity) };
            state.position.update_params(t, refs, mics, shared, ctx.map);
            if let Some(key) = state.physics.update_params(state.object_id, world, local, ctx.vault, ctx.map) {
                state.moveable_object_key = key;
            }
            ctx.delta_time = state.delta_time;
            state.dynamic.update_params(&state.physics, mics, whooshes.as_deref_mut(), ctx);
            state.moving.update_params(&state.physics, chains, swap, ctx);
        }
    }

    /// `ProcessUpdate` 0x82557af0: the moveable voices' pan (`SkateSamplePlayerFactory::mSendInfo`,
    /// written by the unported patch 512/2048 players, so 0), notches, gains and pitch duck; then
    /// every state's objects.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        if self.chains.chain1.is_some() && self.chains.chain2.is_some() {
            let send_info = 0.0;
            MoveableChains::set(ctx.core, self.chains.chain1, 5, 0, send_info);
            MoveableChains::set(ctx.core, self.chains.chain2, 5, 0, send_info);
            self.chains.update_notch_plugins(ctx.core, ctx.rng);
            self.chains.update_gain_plugins(ctx.core, ctx.rng);
            if self.chains.moveable_object_key != 0 {
                self.chains.update_pitch_modulation(ctx.rng);
            }
        }
        let Self { states, chains, .. } = self;
        for state in states {
            ctx.delta_time = state.delta_time;
            state.dynamic.process_update(state.attached, &state.physics, ctx);
            state.moving.process_update(state.attached, &state.physics, chains, ctx);
        }
    }
}
