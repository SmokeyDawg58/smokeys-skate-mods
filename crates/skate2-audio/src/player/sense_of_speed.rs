//! `Sk8::Audio::SFXObj_SenseOfSpeed` (156 bytes, PLAYER object 9, TypeInfo 0x00010090,
//! `CreateObject` 0x8254f070): the local skater's speed layers. Three `SK8_AEMS_skateboard`
//! classes (`SenseOfSpeed_rattle`, `SenseOfSpeed_tone`, `SenseOfSpeed_wind`, in
//! `sense_of_speed.abk`) and the "rocket" grain player on `x_jet_rolling.grain`, each above the
//! minimum of its `aud_senseofspeed` speed range in km/h.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::PlayerPhysics;
use super::skateboard::GrainFiles;
use super::{SfxCtx, PITCH_SCALE, VOLUME_SCALE};
use crate::aems::ClassObject;
use crate::grain_player::{AudBoard, GrainData, GrainParams, UpdateParams};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::random::RwRandom;
use crate::rwac::AudioCore;
use crate::vault::{class, Collection, Vault};
use std::sync::Arc;

/// `GlobalAttributeCollection::aud_senseofspeed_default`.
const SENSE_OF_SPEED: Collection = Collection::new(class::AUD_SENSEOFSPEED, 0xD7ED_BD36_2D7D_2152);

/// `CheckAttribVariables` 0x825361d8's attributes: each layer's volume, minimum and maximum speed.
const RATTLE: [u64; 3] = [0xEB22_4272_A924_C135, 0xF57A_74AF_D22A_D030, 0x1227_5AA8_AC4A_63FB];
const TONE: [u64; 3] = [0x7D28_DAF2_68ED_0DA5, 0xB335_3ECF_968E_3BED, 0xF37F_E22E_22C8_F387];
const WIND: [u64; 3] = [0x1BBA_9174_BD1B_2D88, 0x8B4D_ECD6_46B1_3210, 0x73E9_A42D_88C5_1169];
const ROCKET: [u64; 3] = [0x9BC1_3FA1_9CC4_DF00, 0x7508_154F_F73D_DCED, 0x1185_E9A6_9919_B051];
/// `mRattleVol`, `mRattleMetalVol`, `mRattleHighToneVol`.
const RATTLE_VOLUMES: [u64; 3] = [0x75F1_4A57_A506_E686, 0x5F0E_3DDE_232C_2BF3, 0x9A60_CD2E_36FB_09F3];
/// `mWindLowVol`, `mWindBuffering1Vol`, `mWindBuffering2Vol`, `mWindWhistleVol`.
const WIND_VOLUMES: [u64; 4] = [0x4381_FEC9_776F_5EC9, 0xF7A8_067C_AE4C_51D3, 0xD271_5B3A_D1BB_CDB5, 0xC689_DD34_34BC_4170];
/// `mGrainParams`, five floats.
const GRAIN_PARAMS: u64 = 0xD18D_1174_735E_5CDE;

/// m/s to km/h (`__real_40666666`).
const KMH: f32 = f32::from_bits(0x4066_6666);

/// The rocket players' timer ids follow the skateboards' eight grain players.
pub const ROCKET_TIMER_BASE: u32 = 8;

/// `fsel(-x, 0, x)` then `fsel(1 - x, x, 1)`: within 0..=1, 1 for NaN.
fn unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

/// One Csis layer: its instance (`mpRattle`, `mpTone`, `mpWind`) and attributes.
struct Layer {
    class: &'static str,
    instance: Option<ClassObject>,
    /// `common_parms_Volume`: `mRattleMasterVol`, `mToneVol`, `mWindVol`.
    volume: i32,
    min_speed: f32,
    max_speed: f32,
    /// The class's own volumes after the eight common parameters.
    volumes: Vec<i32>,
    /// `ProcessUpdate`'s mixer outputs: the azimuth (the rattle only), pitch and dry volume.
    azimuth: bool,
    pitch_output: i32,
    dry_output: i32,
}

impl Layer {
    fn new(vault: &Vault, class: &'static str, keys: [u64; 3], volumes: &[u64], outputs: (bool, i32, i32)) -> Self {
        let (azimuth, pitch_output, dry_output) = outputs;
        Self {
            class,
            instance: None,
            volume: vault.i32(SENSE_OF_SPEED, keys[0]),
            min_speed: vault.f32(SENSE_OF_SPEED, keys[1]),
            max_speed: vault.f32(SENSE_OF_SPEED, keys[2]),
            volumes: volumes.iter().map(|&key| vault.i32(SENSE_OF_SPEED, key)).collect(),
            azimuth,
            pitch_output,
            dry_output,
        }
    }

    fn release(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(instance) = self.instance.take() {
            ctx.aems.release_object(instance);
        }
    }
}

pub struct SenseOfSpeed {
    /// `m_pDMixIO` (objectID 0x40010090 + the state's instance).
    pub io: Option<DMixIoId>,
    /// Rattle, tone and wind, in update order.
    layers: [Layer; 3],
    /// `mRocketGrainPlayer`, allocated by `InitSFX` and freed by `Destroy`.
    rocket: Option<AudBoard>,
    /// `pRocketData`.
    rocket_data: Option<GrainData>,
    grain_params: GrainParams,
    /// `mbRocketGrainInitialized`.
    rocket_initialized: bool,
    rocket_vol: f32,
    rocket_min_speed: f32,
    rocket_max_speed: f32,
    timer: u32,
}

impl SenseOfSpeed {
    /// The constructor 0x82549588: `CheckAttribVariables` 0x825361d8, then
    /// `AudioAssetManager::SyncLoad("x_jet_rolling.grain")` for the rocket.
    #[must_use]
    pub fn new(vault: &Vault, grains: &GrainFiles, timer: u32) -> Self {
        let bytes = vault.bytes(SENSE_OF_SPEED, GRAIN_PARAMS, 0, 20);
        let word = |i: usize| f32::from_bits(u32::from_be_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]));
        Self {
            io: None,
            layers: [
                Layer::new(vault, "SenseOfSpeed_rattle", RATTLE, &RATTLE_VOLUMES, (true, 2, 1)),
                Layer::new(vault, "SenseOfSpeed_tone", TONE, &[], (false, 3, 4)),
                Layer::new(vault, "SenseOfSpeed_wind", WIND, &WIND_VOLUMES, (false, 3, 5)),
            ],
            rocket: None,
            rocket_data: grains.get("x_jet_rolling.grain").cloned(),
            grain_params: GrainParams { fade_in_time: word(0), sustain_time: word(1), fade_out_time: word(2), window_size: word(3), percentage_drift: word(4) },
            rocket_initialized: false,
            rocket_vol: vault.i32(SENSE_OF_SPEED, ROCKET[0]) as f32,
            rocket_min_speed: vault.f32(SENSE_OF_SPEED, ROCKET[1]),
            rocket_max_speed: vault.f32(SENSE_OF_SPEED, ROCKET[2]),
            timer,
        }
    }

    /// `InitSFX` 0x82517fe8: the mixer outputs on and a new `AudBoard` ("AudBoard Rocket").
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        self.rocket = Some(AudBoard::new());
    }

    /// `UpdateParams` 0x82535700 (local skater only): each layer exists from its minimum speed,
    /// a new instance starting at the speed's fraction of its range, and the rocket runs above its
    /// minimum.
    pub fn update_params(&mut self, _t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        let kmh = physics.board_speed * KMH;
        let play = [ctx.cvars.play_rattle, ctx.cvars.play_tone, ctx.cvars.play_wind];
        for (layer, play) in self.layers.iter_mut().zip(play) {
            if kmh < layer.min_speed {
                layer.release(ctx);
            } else if layer.instance.is_none() && play {
                let speed = fctiwz(unit((kmh - layer.min_speed) / (layer.max_speed - layer.min_speed)) * 1000.0);
                let mut data = vec![0, 0, 0x1000, speed.clamp(0, 1000), 25000, 0, 0, 0];
                data.extend(layer.volumes.iter().map(|&volume| volume.clamp(0, 0x7FFF)));
                layer.instance = Some(ctx.aems.create_object(layer.class, data));
            }
        }
        if !(kmh <= self.rocket_min_speed) && ctx.cvars.play_rocket {
            if !self.rocket_initialized {
                if let Some(rocket) = self.rocket.as_mut() {
                    let data = self.rocket_data.clone().unwrap_or(GrainData { bytes: Arc::from(&[][..]) });
                    rocket.initialize(ctx.core, ctx.master_sub_mix, &data, self.timer);
                    rocket.grain_params = self.grain_params;
                    rocket.start(ctx.core, ctx.rng);
                    self.rocket_initialized = true;
                }
            }
        } else if self.rocket_initialized {
            // `Destroy` runs twice; the second call finds the player uninitialised.
            if let Some(rocket) = self.rocket.as_mut() {
                rocket.destroy(ctx.core);
            }
            self.rocket_initialized = false;
        }
    }

    /// `ProcessUpdate` 0x82535ce0 (local skater only): the instances' volumes, mixer pitch and dry
    /// level and speed fraction (here `speed * 3.6 - min` fused), and the rocket's intensity.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local {
            return;
        }
        let speed = physics.board_speed;
        let io = self.io;
        for layer in &mut self.layers {
            let Some(instance) = layer.instance.as_mut() else { continue };
            let data = &mut instance.data;
            data[0] = layer.volume.clamp(0, 0x7FFF);
            if layer.azimuth {
                data[1] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
            }
            data[2] = ctx.output(io, layer.pitch_output, PresetType::Pitch).clamp(0, 0x2000);
            data[6] = 0;
            data[7] = ctx.output(io, layer.dry_output, PresetType::Volume).clamp(0, 0x7FFF);
            let fraction = unit(speed.mul_add(KMH, -layer.min_speed) / (layer.max_speed - layer.min_speed));
            data[3] = fctiwz(fraction * 1000.0).clamp(0, 1000);
            ctx.aems.set_member_data(instance);
        }
        if !self.rocket_initialized {
            return;
        }
        let Some(rocket) = self.rocket.as_mut() else { return };
        let intensity = unit(speed.mul_add(KMH, -self.rocket_min_speed) / (self.rocket_max_speed - self.rocket_min_speed));
        let volume = ctx.output(io, 6, PresetType::Volume) as f32;
        let pitch = ctx.output(io, 3, PresetType::Pitch) as f32;
        rocket.update(UpdateParams { volume: self.rocket_vol * (volume * VOLUME_SCALE) * VOLUME_SCALE, pitch: pitch * PITCH_SCALE, azimuth: 0, intensity });
    }

    /// `Destroy` 0x82535628, which `Detach` runs: the three instances, then the rocket player.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for layer in &mut self.layers {
            layer.release(ctx);
        }
        if let Some(mut rocket) = self.rocket.take() {
            if self.rocket_initialized {
                rocket.destroy(ctx.core);
                self.rocket_initialized = false;
            }
        }
    }

    /// The rocket player's `RwAudioCoreTimer` 0x829c9d30.
    pub fn rocket_timer(&mut self, core: &mut AudioCore, rng: &mut RwRandom, period: f32) {
        if let Some(rocket) = &mut self.rocket {
            rocket.process(core, rng, period);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_follows_the_fsel_pair() {
        assert_eq!(unit(-0.5), 0.0);
        assert_eq!(unit(-0.0), 0.0);
        assert_eq!(unit(0.25), 0.25);
        assert_eq!(unit(1.5), 1.0);
        assert_eq!(unit(f32::NAN), 1.0);
    }
}
