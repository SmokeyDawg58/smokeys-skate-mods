//! Collision sounds: the `AudioSystem` collision helpers (`GetCollisionLevel` 0x824efa58,
//! `CalculateCollisionVolume` 0x824fd500, `GetSplicerInfo` 0x824fd388, `GetSplicerIndex`
//! 0x824ef360, `GetVolumeMappingKey` 0x824ef718, `GetL3VolumeDuck` 0x824ef850, `HasLandSound`
//! 0x824ef8c8, `GetAdjustedHardness` 0x824f0ff0, `GetNicotineType`) and the COLLISION state
//! (`eMAINMAPSTATES` 3): `CSTATEMGR_Collision`, `CSTATE_Collision`, `SFXObj_Collision`,
//! `SFXCTL_3DColPos`.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::{CollisionInfo, SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::rwac::core::{PlugInConfig, PlugInKind};
use crate::rwac::{AudioCore, PluginRef, VoiceId};
use crate::splicer::{SpliceId, SpliceUpdateParams};
use crate::surface::{material_collection, material_splice, material_hardness};
use crate::vault::{Collection, Vault};

/// `eCollisionLevel`.
pub const LEVEL_L1: u32 = 0;
pub const LEVEL_L3: u32 = 1;
pub const LEVEL_L4: u32 = 2;
pub const LEVEL_NUM: u32 = 3;
/// `eMaterialHardness`.
pub const HARDNESS_SOFT: u32 = 0;
pub const HARDNESS_MEDIUM: u32 = 1;
pub const HARDNESS_HARD: u32 = 2;
/// `SPLICE_TYPE_METAL`, `SPLICE_TYPE_HOM`.
const SPLICE_METAL: i32 = 6;
const SPLICE_HOM: i32 = 7;
/// `aud_volumemapping` and `aud_intensitymapping` classes, `aud_volumemapping` "default".
const AUD_VOLUMEMAPPING: u64 = 0x13E2_0D39_8E38_5A56;
const AUD_INTENSITYMAPPING: u64 = 0x7DAF_F70B_3A91_CD5D;
const DEFAULT_KEY: u64 = 0xD7ED_BD36_2D7D_2152;
/// `aud_material` attributes.
const DMO_INTENSITY_MAPPING: u64 = 0xF64F_891C_312E_FD5E;
const DMO_VOLUME_MAPPING: u64 = 0xE007_F3FC_90E8_87D0;
const DMO_PITCH: u64 = 0x3A3D_D47E_8DAF_E796;
const HOM_L4: u64 = 0x3EA2_579C_2F3B_B23B;
const HOM_SOFT_L3: u64 = 0x5432_B352_24E4_B1C1;
const HOM_MEDIUM_L3: u64 = 0x3FCB_E040_7F83_3721;
const HOM_HARD_L3: u64 = 0x5331_1C67_61F1_35A1;
const HOM_SOFT_L1: u64 = 0x137C_683B_FB50_6ECA;
const HOM_MEDIUM_L1: u64 = 0x2A28_3013_7430_BB02;
const HOM_HARD_L1: u64 = 0x79BD_E00B_04DF_51B9;

fn layout_byte(vault: &Vault, collection: Collection, offset: u32) -> u8 {
    (vault.layout_word(collection, offset / 4) >> (24 - 8 * (offset % 4))) as u8
}

fn layout_u64(vault: &Vault, collection: Collection, word: u32) -> u64 {
    (u64::from(vault.layout_word(collection, word)) << 32) | u64::from(vault.layout_word(collection, word + 1))
}

/// The collection key of a `RefSpec` attribute (bytes 8..16).
fn ref_spec_collection(vault: &Vault, collection: Collection, key: u64) -> u64 {
    let bytes = vault.bytes(collection, key, 0, 16);
    u64::from_be_bytes(bytes[8..16].try_into().expect("eight bytes"))
}

/// `AudioSystem::HasLandSound` 0x824ef8c8 (layout byte 0x7C).
#[must_use]
pub fn has_land_sound(vault: &Vault, material: u32) -> bool {
    material < 0x84 && material_collection(material).is_some_and(|collection| layout_byte(vault, collection, 0x7C) != 0)
}

/// `AudioSystem::GetNicotineType` (layout word 18; 8 for no material).
#[must_use]
pub fn nicotine_type(vault: &Vault, material: u32) -> u32 {
    if material >= 0x84 {
        return 8;
    }
    material_collection(material).map_or(0, |collection| vault.layout_word(collection, 18))
}

/// `AudioSystem::GetAdjustedHardness` 0x824f0ff0.
#[must_use]
pub fn adjusted_hardness(vault: &Vault, material: u32) -> u32 {
    match material {
        93 | 95 | 97 | 98 | 99 | 100 | 102 | 103 | 104 | 111 => HARDNESS_MEDIUM,
        94 => HARDNESS_HARD,
        96 | 101 | 105 | 106 | 107 => HARDNESS_SOFT,
        _ => material_hardness(vault, material),
    }
}

/// `AudioSystem::GetCollisionLevel` 0x824efa58: the level and the (low, high) force range of
/// that level (unchanged when the force is below the first threshold).
#[must_use]
pub fn collision_level(vault: &Vault, material: u32, force: f32, dmo: bool) -> (u32, Option<(f32, f32)>) {
    if material >= 0x84 {
        return (LEVEL_NUM, None);
    }
    let Some(material) = material_collection(material) else { return (LEVEL_NUM, None) };
    let mut key = layout_u64(vault, material, 8);
    if dmo && layout_byte(vault, material, 0x39) != 0 {
        key = ref_spec_collection(vault, material, DMO_INTENSITY_MAPPING);
    }
    let intensity = Collection::new(AUD_INTENSITYMAPPING, key);
    let value = |index| vault.layout_f32(intensity, index);
    if force < value(3) {
        return (LEVEL_NUM, None);
    }
    let (mut low, mut high, mut level) = (value(3), value(2), 0);
    if force > value(0) {
        low = value(0);
        level = 2;
        high = value(1);
    } else if force > value(2) {
        low = value(2);
        level = 1;
        high = value(0);
    }
    (level, Some((low, high)))
}

/// `AudioSystem::GetVolumeMappingKey` 0x824ef718.
#[must_use]
pub fn volume_mapping_key(vault: &Vault, material: u32, dmo: bool) -> u64 {
    if material >= 0x84 {
        return DEFAULT_KEY;
    }
    let Some(material) = material_collection(material) else { return DEFAULT_KEY };
    if dmo && layout_byte(vault, material, 0x39) != 0 {
        return ref_spec_collection(vault, material, DMO_VOLUME_MAPPING);
    }
    layout_u64(vault, material, 2)
}

/// `AudioSystem::GetL3VolumeDuck` 0x824ef850.
#[must_use]
pub fn l3_volume_duck(vault: &Vault, material: u32, dmo: bool) -> f32 {
    if material >= 0x84 {
        return 1.0;
    }
    vault.layout_f32(Collection::new(AUD_VOLUMEMAPPING, volume_mapping_key(vault, material, dmo)), 10)
}

/// `AudioSystem::CalculateCollisionVolume` 0x824fd500.
#[must_use]
pub fn collision_volume(vault: &Vault, material: u32, other: u32, severity: u32, low: f32, high: f32, current: f32, dmo: bool) -> i32 {
    if material >= 0x84 || severity == LEVEL_NUM {
        return 0;
    }
    let mapping = Collection::new(AUD_VOLUMEMAPPING, volume_mapping_key(vault, material, dmo));
    let current = if current < high { current } else { high };
    let hardness = if material < 95 && other != 0x84 { adjusted_hardness(vault, other) } else { HARDNESS_SOFT };
    let words = |a: u32, b: u32| (vault.layout_word(mapping, a) as i32, vault.layout_word(mapping, b) as i32);
    let (minimum, maximum) = match severity {
        LEVEL_L1 => match hardness {
            0 => words(2, 3),
            1 => words(6, 7),
            _ => words(13, 14),
        },
        LEVEL_L3 => match hardness {
            0 => words(0, 1),
            1 => words(4, 5),
            _ => words(11, 12),
        },
        2 => words(8, 9),
        _ => (0, 0x7FFF),
    };
    if !(high > low) {
        return maximum;
    }
    let slope = (maximum.wrapping_sub(minimum) as f32) / (high - low);
    fctiwz(slope.mul_add(current - low, minimum as f32))
}

/// `AudioSystem::GetSplicerIndex` 0x824ef360.
fn splicer_index(vault: &Vault, material: u32, hardness: u32, level: u32) -> i32 {
    let Some(collection) = material_collection(material) else { return 0 };
    let word = |index: u32| vault.layout_word(collection, index) as i32;
    let attribute = |key: u64| vault.i32(collection, key);
    match material_splice(material) {
        SPLICE_METAL => match (level, hardness) {
            (LEVEL_L4, _) => word(23),
            (LEVEL_L1, 0) => word(20),
            (LEVEL_L1, 1) => word(22),
            (LEVEL_L1, 2) => word(25),
            (_, 0) if level != LEVEL_L1 => word(19),
            (_, 1) if level != LEVEL_L1 => word(21),
            (_, 2) if level != LEVEL_L1 => word(24),
            _ => 0,
        },
        SPLICE_HOM => match (level, hardness) {
            (LEVEL_L4, _) => attribute(HOM_L4),
            (LEVEL_L1, 0) => attribute(HOM_SOFT_L1),
            (LEVEL_L1, 1) => attribute(HOM_MEDIUM_L1),
            (LEVEL_L1, 2) => attribute(HOM_HARD_L1),
            (_, 0) => attribute(HOM_SOFT_L3),
            (_, 1) => attribute(HOM_MEDIUM_L3),
            (_, 2) => attribute(HOM_HARD_L3),
            _ => 0,
        },
        _ => match (level, hardness) {
            (LEVEL_L4, _) => word(28),
            (LEVEL_L1, 0) => word(16),
            (LEVEL_L1, 1) => word(27),
            (LEVEL_L1, 2) => word(30),
            (_, 0) => word(15),
            (_, 1) => word(26),
            (_, 2) => word(29),
            _ => 0,
        },
    }
}

/// `AudioSystem::GetSplicerInfo` 0x824fd388: (bank, index, pitch, volume).
#[must_use]
pub fn splicer_info(vault: &Vault, material: u32, other: u32, level: u32, dmo: bool) -> Option<(i32, i32, i32, i32)> {
    if material >= 132 {
        return None;
    }
    let splice = material_splice(material);
    if splice == -1 {
        return None;
    }
    let hardness = if other == 132 { HARDNESS_SOFT } else { adjusted_hardness(vault, other) };
    let collection = material_collection(material)?;
    let index = splicer_index(vault, material, hardness, level);
    let volume = (vault.layout_word(collection, 13) as i32).clamp(0, 0x7FFF);
    let mut pitch = vault.layout_word(collection, 17) as i32;
    if dmo && layout_byte(vault, collection, 0x38) != 0 {
        pitch = vault.i32(collection, DMO_PITCH);
    }
    Some((splice, index, pitch, volume))
}

/// `SFXObj_Collision::tCollisionData`.
#[derive(Debug, Clone, Copy)]
struct CollisionData {
    splice: Option<SpliceId>,
    volume: i32,
    pitch: i32,
    voice: Option<VoiceId>,
}

impl Default for CollisionData {
    fn default() -> Self {
        Self { splice: None, volume: 0, pitch: 4096, voice: None }
    }
}

/// One `CSTATE_Collision` with its `SFXObj_Collision` and `SFXCTL_3DColPos`.
struct CollisionState {
    attached: bool,
    info: Option<CollisionInfo>,
    tick: u32,
    delta_time: f32,
    object_io: Option<DMixIoId>,
    position: Obj3d,
    data: [CollisionData; 2],
}

/// `CSTATEMGR_Collision`.
pub struct CollisionManager {
    states: Vec<CollisionState>,
}

impl CollisionManager {
    /// `CSTATEMGR_Collision::EnterGamePlay` 0x8254fa68: ten states.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..10u32)
            .map(|instance| {
                let id = (3 << 16) | (instance << 11);
                let mut position = Obj3d::new(vault);
                position.io = map.and_then(|map| map.dmixio(0x6000_0000 | id));
                CollisionState {
                    attached: false,
                    info: None,
                    tick: 0,
                    delta_time: 0.0,
                    object_io: map.and_then(|map| map.dmixio(0x4000_0000 | id)),
                    position,
                    data: [CollisionData::default(); 2],
                }
            })
            .collect();
        Self { states }
    }

    /// `AudioSystem::PlayCollisionSound` 0x824ef148: `GetFreeState` 0x8254fad0 (stealing the
    /// oldest attachment when none is free) and `CSTATE_Collision::Attach`.
    pub fn play(&mut self, info: CollisionInfo, tick: u32, ctx: &mut SfxCtx<'_>) {
        let index = match self.states.iter().position(|state| !state.attached) {
            Some(index) => index,
            None => {
                let mut oldest = None;
                let mut best = u32::MAX;
                for (index, state) in self.states.iter().enumerate() {
                    if state.tick < best {
                        best = state.tick;
                        oldest = Some(index);
                    }
                }
                let Some(index) = oldest else { return };
                self.detach(index, ctx);
                index
            }
        };
        let state = &mut self.states[index];
        state.info = Some(info);
        state.tick = tick;
        state.attached = true;
        for io in [state.position.io, state.object_io].into_iter().flatten() {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `CSTATE_Collision::Detach` (`CSTATE_Base::Detach`, `SFXObj_Collision::Destroy`).
    fn detach(&mut self, index: usize, ctx: &mut SfxCtx<'_>) {
        let state = &mut self.states[index];
        state.attached = false;
        for data in &mut state.data {
            ctx.release(&mut data.splice);
            if let Some(voice) = data.voice.take() {
                ctx.core.release_voice(voice);
            }
        }
        if let Some(io) = state.object_io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        if let Some(io) = state.position.io {
            ctx.map.clear_input(io);
        }
        state.info = None;
    }

    /// `CSTATEMGR_Base::UpdateParams` over the collision states: `SFXCTL_3DColPos` then
    /// `SFXObj_Collision::UpdateParams` 0x82515810.
    pub fn update_params(&mut self, t: f32, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        for state in &mut self.states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { pos: state.info.map(|info| info.position), ..ObjRefs::default() };
            state.position.update_params(t, refs, mics, shared, map);
            let io = state.object_io;
            let set = |map: &mut MixMap, index, value| {
                if let Some(io) = io {
                    map.set_input(io, index, value);
                }
            };
            set(map, 0, 0);
            set(map, 1, 0);
            if state.data[0].splice.is_some() || state.data[1].splice.is_some() {
                set(map, 0, 0x7FFF);
                let severity = state.info.map_or(0, |info| if info.severity[0] > info.severity[1] { info.severity[0] } else { info.severity[1] });
                let value = match severity {
                    1 => 0x4E20,
                    2 => 0x7FFF,
                    _ => 0x2710,
                };
                set(map, 1, value);
            }
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate` over the collision states:
    /// `SFXObj_Collision::ProcessUpdate` 0x82544cb8.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        for index in 0..self.states.len() {
            if !self.states[index].attached {
                continue;
            }
            ctx.delta_time = self.states[index].delta_time;
            if self.states[index].data.iter().all(|data| data.splice.is_none()) {
                self.start_collision(index, ctx);
                continue;
            }
            let state = &mut self.states[index];
            let info = state.info.expect("attached collision");
            let io = state.object_io;
            let material_volume = info.material.map(|material| material_volume(ctx, io, material));
            let pitch = info.material.map(|material| collision_pitch(ctx, io, material));
            for slot in 0..2 {
                let Some(splice) = state.data[slot].splice else { continue };
                if !ctx.is_playing(splice) {
                    ctx.release(&mut state.data[slot].splice);
                    continue;
                }
                let data = state.data[slot];
                let base = fctiwz((info.volume[slot] as f32 * VOLUME_SCALE) * material_volume[slot] as f32);
                let volume = fctiwz(base as f32 * (data.volume as f32 * VOLUME_SCALE));
                let azimuth = ctx.output(io, 0, PresetType::Azimuth);
                let pitch = fctiwz((data.pitch as f32 * PITCH_SCALE) * pitch[slot] as f32);
                let update = SpliceUpdateParams {
                    vol: volume as f32 * VOLUME_SCALE,
                    pitch: pitch as f32 * PITCH_SCALE,
                    az: azimuth as f32 * AZIMUTH_SCALE,
                    delta: ctx.delta_time,
                    spread: 0.0,
                    time_stretch: 1.0,
                };
                ctx.update_splice(splice, update);
            }
            if state.data.iter().all(|data| data.splice.is_none()) {
                self.detach(index, ctx);
            }
        }
    }

    /// `SFXObj_Collision::StartCollision` 0x82544ae8.
    fn start_collision(&mut self, index: usize, ctx: &mut SfxCtx<'_>) {
        let Some(info) = self.states[index].info else { return };
        let play = [ctx.cvars.play_impactor_collision, ctx.cvars.play_impacted_collision];
        for slot in 0..2 {
            if info.material[slot] == 132 || info.severity[slot] == LEVEL_NUM {
                continue;
            }
            let other = info.material[1 - slot];
            let Some((bank, splice_index, pitch, volume)) = splicer_info(ctx.vault, info.material[slot], other, info.severity[slot], info.dmo_collision) else {
                continue;
            };
            let state = &mut self.states[index];
            state.data[slot].pitch = pitch;
            state.data[slot].volume = volume;
            if splice_index == -1 || !play[slot] {
                continue;
            }
            let io = state.object_io;
            let wet = material_wet_level(ctx, io, info.material[slot]);
            let voice = create_collision_voice(ctx.core, ctx.master_sub_mix, ctx.global_effects.reverb_sub_mix(), wet);
            self.states[index].data[slot].voice = voice;
            let Some(voice) = voice else { continue };
            self.states[index].data[slot].splice = ctx.new_splice(bank, splice_index, PluginRef { voice, index: 0 });
        }
    }
}

/// `SFXObj_Collision::GetMaterialVolume` 0x825290c8.
fn material_volume(ctx: &SfxCtx<'_>, io: Option<DMixIoId>, material: u32) -> i32 {
    let output = match nicotine_type(ctx.vault, material) {
        0 => 14,
        1 => 15,
        2 => 16,
        3 => 17,
        4 => 18,
        5 => 19,
        6 => 13,
        7 => 20,
        9 => 22,
        _ => 21,
    };
    ctx.output(io, output, PresetType::Volume)
}

/// `SFXObj_Collision::GetWetLevel` 0x825291c0.
fn material_wet_level(ctx: &SfxCtx<'_>, io: Option<DMixIoId>, material: u32) -> i32 {
    let output = match nicotine_type(ctx.vault, material) {
        0 => 4,
        1 => 5,
        2 => 6,
        3 => 7,
        4 => 8,
        5 => 9,
        6 => 3,
        7 => 10,
        9 => 12,
        _ => 11,
    };
    ctx.output(io, output, PresetType::Volume)
}

/// `SFXObj_Collision::GetPitch` 0x825292b8.
fn collision_pitch(ctx: &SfxCtx<'_>, io: Option<DMixIoId>, material: u32) -> i32 {
    let output = if nicotine_type(ctx.vault, material) == 9 { 23 } else { 1 };
    ctx.output(io, output, PresetType::Pitch)
}

/// `SFXObj_Collision::CreateCollisionVoice` 0x82529328: stage 4, `SubMix` "Collision SubMix"
/// (one channel) and two `Send`s; send 1 goes to `GlobalEffects`' reverb at the material's wet
/// level, send 2 to the master mix.
fn create_collision_voice(core: &mut AudioCore, master: PluginRef, reverb: PluginRef, wet: i32) -> Option<VoiceId> {
    let voice = core.create_voice(
        4,
        "Collision SubMix",
        &[
            PlugInConfig { kind: PlugInKind::SubMix { name: "Collision SubMix".into() }, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 1 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 1 },
        ],
    )?;
    core.connect_send(PluginRef { voice, index: 1 }, Some(reverb));
    core.set_attribute(PluginRef { voice, index: 1 }, 0, wet as f32 * VOLUME_SCALE);
    core.connect_send(PluginRef { voice, index: 2 }, Some(master));
    Some(voice)
}

/// `AudioSystem::CreateSplicerReverbVoice` 0x824f13c8: stage 4, `SubMix` "Splicer Wet SubMix" and
/// two `Send`s (six channels); send 1 goes to `GlobalEffects`' environment send at `wet`, send 2
/// to the master mix.
pub fn create_splicer_reverb_voice(core: &mut AudioCore, master: PluginRef, reverb: PluginRef, wet: f32) -> Option<VoiceId> {
    let voice = core.create_voice(
        4,
        "Splicer Wet SubMix",
        &[
            PlugInConfig { kind: PlugInKind::SubMix { name: "Splicer Wet SubMix".into() }, output_channels: 6 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
            PlugInConfig { kind: PlugInKind::Send, output_channels: 6 },
        ],
    )?;
    core.connect_send(PluginRef { voice, index: 1 }, Some(reverb));
    core.set_attribute(PluginRef { voice, index: 1 }, 0, wet);
    core.connect_send(PluginRef { voice, index: 2 }, Some(master));
    Some(voice)
}
