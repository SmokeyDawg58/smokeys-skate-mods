//! `Sk8::Audio::SFXObj_HandGrabs` (52 bytes, PLAYER object 11, TypeInfo 0x000100b0,
//! `CreateObject` 0x8253a568): the local skater's hand contacts. Skitching, a hand plant or a
//! grabbed dynamic object starts the hand grab splice and the surface's grab splice; letting go of
//! a hand plant or an object plays the surface's grab-off splice.

use super::physics::PlayerPhysics;
use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::MATERIAL_MAX;
use crate::splicer::{SpliceId, SpliceUpdateParams};
use crate::surface::{material_collection, material_splice};
use crate::vault::{Collection, Vault};

/// The hand grab splice, and skitching's grab splice in the metal bank.
const GRAB: (i32, i32) = (5, 0x302);
const SKITCH: (i32, i32) = (6, 0x1C0);
/// `aud_dynamicobject`, whose layout word 90 is the object's audio material.
const AUD_DYNAMICOBJECT: u64 = 0xB05F_7C74_C63A_CFB8;
const DMO_MATERIAL: u32 = 90;
/// `aud_material::Grab_Off`, `Metal_Grab_Off` and `HOM_Soft_L1`, and the grab and grab-off volumes.
const GRAB_OFF: u64 = 0x897F_422F_517D_B598;
const METAL_GRAB_OFF: u64 = 0xCD45_84FA_CA7B_D43C;
const HOM_SOFT_L1: u64 = 0x137C_683B_FB50_6ECA;
const GRAB_VOLUME: u64 = 0x9149_686A_DC03_1389;
const GRAB_OFF_VOLUME: u64 = 0x3972_02D0_AA48_2601;

/// `AudioSystem::GetGrabCollision` 0x824efc40: the splice bank is the material's
/// `tAudioSurfaceMapping` type; the index its grab (layout word 16, metal word 20, Hall of Meat
/// `HOM_Soft_L1`) or grab-off attribute. A material outside the table reads material 0.
fn grab_collision(vault: &Vault, material: u32, grab: bool) -> (i32, i32) {
    let material = if material >= MATERIAL_MAX { 0 } else { material };
    let bank = material_splice(material);
    let Some(collection) = material_collection(material) else { return (bank, 0) };
    let index = match (bank, grab) {
        (5, true) => vault.layout_word(collection, 16) as i32,
        (6, true) => vault.layout_word(collection, 20) as i32,
        (7, true) => vault.i32(collection, HOM_SOFT_L1),
        (5 | 7, false) => vault.i32(collection, GRAB_OFF),
        (6, false) => vault.i32(collection, METAL_GRAB_OFF),
        _ => 0,
    };
    (bank, index)
}

/// `AudioSystem::GetGrabVolume` 0x824efdc0 and `GetGrabOffVolume` 0x824efeb8: 0 outside the
/// table.
fn material_volume(vault: &Vault, material: u32, key: u64) -> i32 {
    if material >= MATERIAL_MAX {
        return 0;
    }
    material_collection(material).map_or(0, |collection| vault.i32(collection, key))
}

/// The `aud_dynamicobject` material of `mDMOKey`'s collection, when it exists.
fn dmo_material(vault: &Vault, key: u64) -> Option<u32> {
    let collection = Collection::new(AUD_DYNAMICOBJECT, key);
    vault.has_collection(collection).then(|| vault.layout_word(collection, DMO_MATERIAL))
}

pub struct HandGrabs {
    /// `m_pDMixIO` (objectID 0x400100b0 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mbSkitching`, `mbHandPlant`, `mbGrabbing`.
    skitching: bool,
    hand_plant: bool,
    grabbing: bool,
    /// `mpGrab`, `mpMaterial`, `mpRelease`.
    grab: Option<SpliceId>,
    material: Option<SpliceId>,
    release: Option<SpliceId>,
    /// `mReleaseAudioMaterial`: the planted hand's surface.
    release_material: u32,
}

impl Default for HandGrabs {
    /// The constructor 0x82518260.
    fn default() -> Self {
        Self {
            io: None,
            skitching: false,
            hand_plant: false,
            grabbing: false,
            grab: None,
            material: None,
            release: None,
            release_material: MATERIAL_MAX,
        }
    }
}

impl HandGrabs {
    /// The shared `InitSFX` 0x82515d80: the mixer outputs on.
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `UpdateParams` 0x82549dd0: the local skater's checks, then mixer input 0 while skitching.
    pub fn update_params(&mut self, _t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if ctx.local {
            self.check_for_skitch(physics, ctx);
            self.check_for_hand_plants(physics, ctx);
            self.check_for_grab(physics, ctx);
        }
        ctx.set_input(self.io, 0, if self.skitching { 0x7FFF } else { 0 });
    }

    /// `ProcessUpdate` 0x82549e60 (local skater only).
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if ctx.local {
            self.update_splices(physics, ctx);
        }
    }

    /// `Destroy` 0x825182c0, which `Detach` runs: the three splices (the flags stay).
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        ctx.release(&mut self.grab);
        ctx.release(&mut self.material);
        ctx.release(&mut self.release);
    }

    /// The hand grab splice, replacing the grab and material splices.
    fn start_grab(&mut self, ctx: &mut SfxCtx<'_>) {
        ctx.release(&mut self.grab);
        ctx.release(&mut self.material);
        let sub_mix = ctx.master_sub_mix;
        self.grab = ctx.new_splice(GRAB.0, GRAB.1, sub_mix);
    }

    /// A surface's grab (or grab-off) splice.
    fn surface_splice(ctx: &mut SfxCtx<'_>, material: u32, grab: bool) -> Option<SpliceId> {
        let (bank, index) = grab_collision(ctx.vault, material, grab);
        let sub_mix = ctx.master_sub_mix;
        ctx.new_splice(bank, index, sub_mix)
    }

    /// `CheckForSkitch` 0x8253a620: taking hold plays the hand grab and the metal grab.
    fn check_for_skitch(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.skitching {
            if !physics.is_skitching {
                self.skitching = false;
            }
        } else if physics.is_skitching {
            self.start_grab(ctx);
            let sub_mix = ctx.master_sub_mix;
            self.material = ctx.new_splice(SKITCH.0, SKITCH.1, sub_mix);
            self.skitching = true;
        }
    }

    /// `CheckForHandPlants` 0x8253a7a0: a hand plant plays the hand grab and its surface's grab
    /// splice; lifting the hand plays that surface's grab-off splice.
    fn check_for_hand_plants(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.hand_plant {
            if !physics.is_hand_plant {
                ctx.release(&mut self.release);
                if self.release_material != MATERIAL_MAX {
                    self.release = Self::surface_splice(ctx, self.release_material, false);
                }
                self.hand_plant = false;
            }
        } else if physics.is_hand_plant {
            self.start_grab(ctx);
            let material = physics.hand_plant_audio_surface;
            if material != MATERIAL_MAX {
                self.material = Self::surface_splice(ctx, material, true);
            }
            self.release_material = material;
            self.hand_plant = true;
        }
    }

    /// `CheckForGrab` 0x8253aa28: the same for a grabbed dynamic object's material.
    fn check_for_grab(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if self.grabbing {
            if !physics.grabbing_dmo {
                ctx.release(&mut self.release);
                if let Some(material) = dmo_material(ctx.vault, physics.dmo_key).filter(|&material| material != MATERIAL_MAX) {
                    self.release = Self::surface_splice(ctx, material, false);
                }
                self.grabbing = false;
            }
        } else if physics.grabbing_dmo {
            self.start_grab(ctx);
            if let Some(material) = dmo_material(ctx.vault, physics.dmo_key).filter(|&material| material != MATERIAL_MAX) {
                self.material = Self::surface_splice(ctx, material, true);
            }
            self.grabbing = true;
        }
    }

    /// `UpdateSplices` 0x8253ad00: each splice follows the mixer (volume outputs 1, 2 and 3,
    /// pitch 4, azimuth 0) until it ends. The grabbed object's material, whatever the splice's
    /// cause, scales the material splice by its grab volume and the release by its grab-off volume.
    fn update_splices(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let dmo = dmo_material(ctx.vault, physics.dmo_key);
        let io = self.io;
        update_splice(io, &mut self.grab, 1, None, ctx);
        let grab = dmo.map(|material| material_volume(ctx.vault, material, GRAB_VOLUME));
        update_splice(io, &mut self.material, 2, grab, ctx);
        let grab_off = dmo.map(|material| material_volume(ctx.vault, material, GRAB_OFF_VOLUME));
        update_splice(io, &mut self.release, 3, grab_off, ctx);
    }
}

/// One `UpdateSplices` block: released once it stops playing.
fn update_splice(io: Option<DMixIoId>, splice: &mut Option<SpliceId>, output: i32, volume: Option<i32>, ctx: &mut SfxCtx<'_>) {
    let Some(id) = *splice else { return };
    if !ctx.is_playing(id) {
        ctx.release(splice);
        return;
    }
    let mut vol = ctx.output(io, output, PresetType::Volume) as f32 * VOLUME_SCALE;
    if let Some(volume) = volume {
        vol *= volume as f32 * VOLUME_SCALE;
    }
    let update = SpliceUpdateParams {
        vol,
        pitch: ctx.output(io, 4, PresetType::Pitch) as f32 * PITCH_SCALE,
        az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
        delta: ctx.delta_time,
        spread: 1.0,
        time_stretch: 1.0,
    };
    ctx.update_splice(id, update);
}
