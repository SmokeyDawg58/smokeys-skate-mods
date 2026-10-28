//! `SFXObj_Dynamic` (0x00080000): the rolling and sliding loops of a loose dynamic object
//! (`c_dynamic_rolling_objects`, `c_dynamic_sliding_objects` in the four banks its constructor
//! 0x82533670 loads: `DMO_Starts.abk`, `Dynamics_Objects_Roll.abk`,
//! `Dynamics_Objects_Slide_A.abk`, `Dynamics_Objects_Slide_B.abk`), its impacts and its whoosh.

use super::{dynamic_object_or_default, layout_byte, layout_u64, Physics, KPH};
use crate::aems::ClassObject;
use crate::collision::{self, LEVEL_L1, LEVEL_L3, LEVEL_L4, LEVEL_NUM};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::player::pos3d::MicrophoneMan;
use crate::player::{CollisionInfo, SfxCtx, PITCH_SCALE};
use crate::presentation::MATERIAL_MAX;
use crate::utils::clamp_fsel;
use crate::world::whoosh::{self, WhooshManager};

/// The `aud_dynamicobject` roll attributes `GetAttribVariables` reads by key.
const ROLL_KEYS: [u64; 22] = [
    0xC6A2_B85A_C87A_6E6E,
    0x4006_C735_1138_4DD9,
    0xD2B6_8F51_8948_B6B1,
    0x47B1_51FB_093E_8B63,
    0x15CD_D8F9_9FD5_143D,
    0x775A_C9A0_44EB_A15B,
    0x2243_9C1E_29DF_5306,
    0x8F3F_4E34_9A86_F668,
    0x5D25_98EF_B05C_9903,
    0xA4B1_ACF5_9F9A_6AD2,
    0x0B09_639B_E4E2_6AE8,
    0xA67D_B8DB_2548_82DE,
    0xFB37_ABA8_5116_510F,
    0x4650_9B25_419F_00F8,
    0xFB7A_C542_0272_7AD1,
    0xC639_D3F0_C4DA_2BD9,
    0x905A_6733_BA0A_6992,
    0x2054_0FCA_23FA_5469,
    0xB0D7_0899_984B_85F5,
    0x4355_DF82_D13F_308A,
    0x1F61_9170_2E09_72F6,
    0x8F71_44D1_30C2_C523,
];
/// The layout words of the slide values (`mSlideAngleLFOAmp` .. `mDynamic_slide_volume_hi`).
const SLIDE_WORDS: [u32; 22] = [39, 38, 37, 36, 31, 30, 32, 35, 33, 34, 85, 83, 84, 82, 80, 81, 88, 86, 87, 28, 27, 29];

/// `Sk8::Audio::SFXObj_Dynamic` (0x120 bytes).
#[derive(Debug, Default)]
pub(super) struct Dynamic {
    pub(super) io: Option<DMixIoId>,
    sliding: Option<ClassObject>,
    rolling: Option<ClassObject>,
    max_speed: f32,
    material_id: i32,
    collision_type: u32,
    rattle_type: u32,
    pitch_offset: i32,
    /// `mSlideAngleLFOAmp` .. `mDynamic_slide_volume_hi` (0x48..0x9C), the members
    /// `c_dynamic_sliding_objects` takes from `angle_lfo_amp` on.
    slide: [i32; 22],
    /// `mRollAngleLFOAmp` .. `mDynamic_roll_volume_hi` (0xA0..0xF4).
    roll: [i32; 22],
    collision_count: i32,
    played_whoosh: bool,
    whoosh_key: u64,
    current_dynamic_key: u64,
    play_impacted_sound: bool,
    min_collision_force: f32,
    max_collision_force: f32,
    collision_delay_frames: i32,
}

impl Dynamic {
    /// The constructor 0x82533670.
    pub(super) fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            rattle_type: MATERIAL_MAX,
            play_impacted_sound: true,
            min_collision_force: 1.0,
            max_collision_force: 3.0,
            collision_delay_frames: 10,
            ..Self::default()
        }
    }

    /// `InitSFX` 0x8254f6e8 (`mDynamicObjectID`, which nothing ported reads, aside).
    pub(super) fn init_sfx(&mut self, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.played_whoosh = false;
        self.current_dynamic_key = 0;
        self.get_attrib_variables(physics, ctx);
    }

    /// `GetDynamicAttribKey` 0x82548928: the secondary key while the object is moved.
    fn dynamic_attrib_key(physics: &Physics, ctx: &SfxCtx<'_>) -> u64 {
        if physics.get_is_being_moved(ctx.challenge_type) { physics.secondary_key } else { physics.attrib_key }
    }

    /// `GetAttribVariables` 0x8254e788.
    fn get_attrib_variables(&mut self, physics: &Physics, ctx: &SfxCtx<'_>) {
        let vault = ctx.vault;
        let attrib = dynamic_object_or_default(vault, Self::dynamic_attrib_key(physics, ctx));
        self.max_speed = vault.layout_f32(attrib, 77);
        self.material_id = vault.layout_word(attrib, 79) as i32;
        self.pitch_offset = vault.layout_word(attrib, 44) as i32;
        for (value, word) in self.slide.iter_mut().zip(SLIDE_WORDS) {
            *value = vault.layout_word(attrib, word) as i32;
        }
        for (value, key) in self.roll.iter_mut().zip(ROLL_KEYS) {
            *value = vault.i32(attrib, key);
        }
        self.get_collision_info_from_attrib(physics, ctx);
    }

    /// `GetCollisionInfoFromAttrib` 0x82533908, from the primary key's collection.
    fn get_collision_info_from_attrib(&mut self, physics: &Physics, ctx: &SfxCtx<'_>) {
        if physics.attrib_key == 0 {
            return;
        }
        let vault = ctx.vault;
        let attrib = dynamic_object_or_default(vault, physics.attrib_key);
        self.collision_type = vault.layout_word(attrib, 90);
        self.rattle_type = vault.layout_word(attrib, 40);
        self.whoosh_key = layout_u64(vault, attrib, 2);
        self.play_impacted_sound = layout_byte(vault, attrib, 172) != 0;
        self.min_collision_force = vault.layout_f32(attrib, 74);
        self.max_collision_force = vault.layout_f32(attrib, 75);
        self.collision_delay_frames = vault.layout_word(attrib, 89) as i32;
    }

    /// `CalcAemsSpeed` 0x82533858: the speed (the smoothed moving speed while moved) over
    /// `aud_dynamicobject` word 77 in km/h, 0..1000.
    fn calc_aems_speed(&self, physics: &Physics, ctx: &SfxCtx<'_>) -> i32 {
        let mut speed = physics.speed;
        if physics.get_is_being_moved(ctx.challenge_type) {
            speed = physics.moving_speed;
        }
        if f64::from(speed) < 0.2 {
            speed = 0.0;
        }
        fctiwz(clamp_fsel((speed * KPH) / self.max_speed, 1.0) * 1000.0)
    }

    /// `CanPlayDynamic` 0x825489a0: the key has to hold for two updates and the object be on the
    /// ground.
    fn can_play_dynamic(&mut self, physics: &Physics, ctx: &SfxCtx<'_>) -> bool {
        let key = Self::dynamic_attrib_key(physics, ctx);
        let mut same = key == self.current_dynamic_key;
        self.current_dynamic_key = key;
        if key == 0 {
            same = false;
        }
        physics.on_ground && same
    }

    /// The `c_dynamic_*_objects` members (`CreateSliding` 0x825480d0, `CreateRolling` 0x825484e0).
    fn member_data(&self, speed: i32, values: &[i32; 22]) -> Vec<i32> {
        let mut data = vec![0x7FFF, 0, 4096, speed, 25000, 0, 0, 0, self.material_id, 0, 0, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF];
        data.extend_from_slice(values);
        data
    }

    /// `StopDynamicAudio` 0x825337e8.
    fn stop_dynamic_audio(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(object) = self.sliding.take() {
            ctx.aems.release_object(object);
        }
        if let Some(object) = self.rolling.take() {
            ctx.aems.release_object(object);
        }
    }

    /// `Destroy` 0x82548060 (`SFXObjBase::Detach` 0x82518380 calls it).
    pub(super) fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        self.stop_dynamic_audio(ctx);
    }

    /// `UpdateParams` 0x8254f728.
    pub(super) fn update_params(&mut self, physics: &Physics, mics: &MicrophoneMan, whooshes: Option<&mut WhooshManager>, ctx: &mut SfxCtx<'_>) {
        self.get_attrib_variables(physics, ctx);
        if physics.speed > 0.2 && self.can_play_dynamic(physics, ctx) {
            if self.sliding.is_none() && self.rolling.is_none() {
                self.get_attrib_variables(physics, ctx);
                let speed = self.calc_aems_speed(physics, ctx);
                if physics.sliding {
                    if speed > 0 {
                        let data = self.member_data(speed, &self.slide);
                        self.sliding = Some(ctx.aems.create_object("c_dynamic_sliding_objects", data));
                    }
                } else {
                    let data = self.member_data(speed, &self.roll);
                    self.rolling = Some(ctx.aems.create_object("c_dynamic_rolling_objects", data));
                }
            }
        } else if self.sliding.is_some() || self.rolling.is_some() {
            self.stop_dynamic_audio(ctx);
        }
        self.check_for_collisions(physics, ctx);
        self.check_for_whoosh(physics, mics, whooshes, ctx);
    }

    /// `CheckForCollisions` 0x82533a18: an impact over `mMinCollisionForce` plays the object's
    /// collision type against the surface (and the L3 duck of an L4), then its rattle while it
    /// moves; `mCollisionDelayFrames` updates pass before the next.
    fn check_for_collisions(&mut self, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if self.collision_count > 0 {
            self.collision_count -= 1;
            return;
        }
        let force = physics.collision_force;
        if !(force > self.min_collision_force) {
            return;
        }
        let vault = ctx.vault;
        let material = if self.play_impacted_sound { physics.collided_material } else { MATERIAL_MAX };
        let normalized = clamp_fsel((force - self.min_collision_force) / (self.max_collision_force - self.min_collision_force), 5.0);
        let (mut low, mut high) = (0.0, 0.0);
        let (type_level, range) = collision::collision_level(vault, self.collision_type, normalized, true);
        if let Some(range) = range {
            (low, high) = range;
        }
        let (material_level, material_range) = collision::collision_level(vault, material, normalized, true);
        let (material_low, material_high) = material_range.unwrap_or((0.0, 0.0));
        let type_volume = collision::collision_volume(vault, self.collision_type, material, type_level, low, high, normalized, true);
        let material_volume = collision::collision_volume(vault, material, self.collision_type, material_level, material_low, material_high, normalized, true);
        if type_level == LEVEL_NUM && material_level == LEVEL_NUM {
            return;
        }
        ctx.collisions.push(CollisionInfo {
            material: [self.collision_type, material],
            severity: [type_level, material_level],
            position: physics.position,
            volume: [type_volume, material_volume],
            dmo_collision: true,
        });
        self.collision_count = self.collision_delay_frames;
        let ducked = [type_level, material_level].map(|level| if level == LEVEL_L4 { LEVEL_L3 } else { LEVEL_NUM });
        if ducked[0] != LEVEL_NUM || ducked[1] != LEVEL_NUM {
            let first = fctiwz(collision::l3_volume_duck(vault, self.collision_type, true) * type_volume as f32);
            let second = fctiwz(collision::l3_volume_duck(vault, material, true) * material_volume as f32);
            ctx.collisions.push(CollisionInfo {
                material: [self.collision_type, material],
                severity: ducked,
                position: physics.position,
                volume: [first, second],
                dmo_collision: true,
            });
        }
        if self.rattle_type != MATERIAL_MAX && f64::from(physics.speed * KPH) > 0.1 {
            let (rattle_level, range) = collision::collision_level(vault, self.rattle_type, normalized, true);
            if let Some(range) = range {
                (low, high) = range;
            }
            let volume = collision::collision_volume(vault, self.rattle_type, material, rattle_level, low, high, normalized, true);
            if rattle_level != LEVEL_NUM {
                ctx.collisions.push(CollisionInfo {
                    material: [self.rattle_type, MATERIAL_MAX],
                    severity: [rattle_level, LEVEL_L1],
                    position: physics.position,
                    volume: [volume, 0],
                    dmo_collision: true,
                });
            }
        }
    }

    /// `CheckForWhoosh` 0x82533e30 at the object's position.
    fn check_for_whoosh(&mut self, physics: &Physics, mics: &MicrophoneMan, whooshes: Option<&mut WhooshManager>, ctx: &SfxCtx<'_>) {
        whoosh::check_for_whoosh(&mut self.played_whoosh, self.whoosh_key, physics.position, mics, whooshes, ctx.vault);
    }

    /// `ProcessUpdate` 0x8254e6f8.
    pub(super) fn process_update(&mut self, attached: bool, physics: &Physics, ctx: &mut SfxCtx<'_>) {
        if !attached {
            return;
        }
        let speed = self.calc_aems_speed(physics, ctx).clamp(0, 1000);
        let moved = physics.get_is_being_moved(ctx.challenge_type);
        let io = self.io;
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        let mut dry = ctx.output(io, 1, PresetType::Volume);
        let wet = ctx.output(io, 5, PresetType::Volume);
        let pitch = ctx.output(io, 2, PresetType::Pitch);
        let low_pass = ctx.output(io, 4, PresetType::Frequency);
        let high_pass = ctx.output(io, 3, PresetType::Frequency);
        let pitch = pitch - fctiwz((self.pitch_offset as f32 * pitch as f32) * -PITCH_SCALE);
        if let Some(mut object) = self.sliding.take() {
            // `UpdateSliding` 0x82548220.
            let mut dry = dry;
            if moved {
                dry = fctiwz(physics.secondary_volume * ctx.output(io, 6, PresetType::Volume) as f32);
            }
            Self::write_common(&mut object.data, speed, azimuth, dry, wet, pitch, low_pass, high_pass);
            ctx.aems.set_member_data(&object);
            self.sliding = Some(object);
        }
        if let Some(mut object) = self.rolling.take() {
            // `UpdateRolling` 0x82548628.
            let mut wet = wet;
            if moved {
                dry = fctiwz(physics.secondary_volume * ctx.output(io, 6, PresetType::Volume) as f32);
                wet = fctiwz(physics.secondary_volume * ctx.output(io, 7, PresetType::Volume) as f32);
            }
            Self::write_common(&mut object.data, speed, azimuth, dry, wet, pitch, low_pass, high_pass);
            ctx.aems.set_member_data(&object);
            self.rolling = Some(object);
        }
    }

    /// The `common_parms` both updates write.
    #[allow(clippy::too_many_arguments)]
    fn write_common(data: &mut [i32], speed: i32, azimuth: i32, dry: i32, wet: i32, pitch: i32, low_pass: i32, high_pass: i32) {
        data[3] = speed;
        data[1] = azimuth.clamp(0, 0xFFFF);
        data[7] = dry.clamp(0, 0x7FFF);
        data[6] = wet.clamp(0, 0x7FFF);
        data[2] = pitch.clamp(0, 0x2000);
        data[4] = low_pass.clamp(0, 25001);
        data[5] = high_pass.clamp(0, 25001);
    }
}
