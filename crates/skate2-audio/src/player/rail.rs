//! `Sk8::Audio::SFXObj_Rail` (64 bytes, PLAYER object 3): the grind and slide loop
//! (`Class_grind`, `GRINDS.abk`). A grind starts one instance for the board, and a 50-50 a second
//! one for the other truck.

use super::physics::PlayerPhysics;
use super::SfxCtx;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::MATERIAL_MAX;
use crate::surface::grind_material;
use crate::vault::{class, Collection, Vault};

/// `GlobalAttributeCollection::aud_rails_default`.
const RAILS: Collection = Collection::new(class::AUD_RAILS, 0xD7ED_BD36_2D7D_2152);
/// `aud_rails::MaxSpeed`, and the grind type that replaces the skater's unless it is
/// `GRIND_TYPE_NONE`.
const MAX_SPEED: u64 = 0x4890_392C_9182_9954;
const GRIND_TYPE_OVERRIDE: u64 = 0x6710_834A_D5E0_847F;
/// The per-surface arrays for parts 0, 1 and 2 behind `GetRailVolumeMultiplier` 0x82521850,
/// `GetSweetenerVolume` 0x82521b50, `GetEnterVolume` 0x82521e68 and `GetExitVolume` 0x82522180.
const RAIL_VOLUME: [u64; 3] = [0x6996_9AF1_BE6B_B367, 0x0555_484D_6D4A_3128, 0xC219_83A2_160E_D3AC];
const SWEETENER_VOLUME: [u64; 3] = [0x0ECE_CDAC_28B2_B979, 0x5807_0BF5_1180_9903, 0x72BA_0780_A8FA_25D6];
const ENTER_VOLUME: [u64; 3] = [0x3D63_FBC1_4F6A_6CEA, 0x0BD0_F347_C3AC_6D00, 0x18A9_4AE8_DEED_0D27];
const EXIT_VOLUME: [u64; 3] = [0x958D_61F2_EC11_8BFF, 0x94E3_254C_D727_BEA6, 0x751C_968D_0992_C231];

/// `Sk8::Physics::eGrindType`.
const GRIND_TYPE_NONE: i32 = -1;
const GRIND_TYPE_5050: i32 = 0;
const GRIND_TYPE_BOARDSLIDE: i32 = 1;
const GRIND_TYPE_TIPSLIDE: i32 = 2;
const GRIND_TYPE_BACKSLASH: i32 = 4;
/// `eGrindMaterial_Rail1` and `eGrindMaterial_Untagged`.
const GRIND_MATERIAL_RAIL1: i32 = 4;
const GRIND_MATERIAL_UNTAGGED: i32 = 14;

pub struct Rail {
    /// `m_pDMixIO` (objectID 0x40010030 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mpRailSound`: the grind, and a 50-50's second truck.
    rail_sound: [Option<ClassObject>; 2],
    /// `mCurrentGrindType`.
    current_grind_type: i32,
}

impl Default for Rail {
    /// The constructor 0x82514508.
    fn default() -> Self {
        Self { io: None, rail_sound: [None, None], current_grind_type: GRIND_TYPE_NONE }
    }
}

/// The surface's element of the parts' array. The surface stops at the parts 0 array's last
/// element; parts past 2 have none.
fn rail_value(vault: &Vault, keys: [u64; 3], surface: i32, parts: u32) -> Option<f32> {
    let length = vault.length(RAILS, keys[0]) as i32;
    let surface = if surface >= length { length - 1 } else { surface };
    let key = *keys.get(parts as usize)?;
    Some(vault.f32_at(RAILS, key, surface as u32))
}

/// `GetRailVolumeMultiplier` 0x82521850 (1 past parts 2).
fn rail_volume_multiplier(vault: &Vault, surface: i32, parts: u32) -> f32 {
    rail_value(vault, RAIL_VOLUME, surface, parts).unwrap_or(1.0)
}

/// `GetSweetenerVolume`, `GetEnterVolume` and `GetExitVolume`: the value times 32767 (1 past
/// parts 2).
fn rail_volume(vault: &Vault, keys: [u64; 3], surface: i32, parts: u32) -> i32 {
    fctiwz(rail_value(vault, keys, surface, parts).unwrap_or(1.0) * 32767.0)
}

/// The `speed` member: `(mBoardSpeed - 0.5) / MaxSpeed * 3.6` limited to 0..1 by two `fsel`s, in
/// ten-thousandths, at most 9000.
fn grind_speed(vault: &Vault, board_speed: f32) -> i32 {
    let scaled = (board_speed - 0.5) / vault.f32(RAILS, MAX_SPEED) * f32::from_bits(0x4066_6666);
    let low = if -scaled >= 0.0 { 0.0 } else { scaled };
    let unit = if 1.0 - low >= 0.0 { low } else { 1.0 };
    fctiwz(unit * 10000.0).min(9000)
}

/// `mGrindMaterial` through `SurfaceMapping::GetGrindMaterial`. An untagged surface grinds as
/// `eGrindMaterial_Rail1`, or `eGrindMaterial_Untagged` under `kNoRailWhenUntagged`.
fn grind_surface(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> i32 {
    if physics.grind_material == MATERIAL_MAX {
        if ctx.cvars.no_rail_when_untagged { GRIND_MATERIAL_UNTAGGED } else { GRIND_MATERIAL_RAIL1 }
    } else {
        grind_material(ctx.vault, physics.grind_material) as i32
    }
}

/// The board's parts: 0 on the slides, else 2.
fn grind_parts(grind_type: i32) -> i32 {
    if matches!(grind_type, GRIND_TYPE_BOARDSLIDE | GRIND_TYPE_TIPSLIDE | GRIND_TYPE_BACKSLASH) { 0 } else { 2 }
}

impl Rail {
    /// The shared `InitSFX` 0x82515d80 its `SetupLoadData` runs on attach: the mixer output on.
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `Destroy` 0x825217f8: both instances.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for grind in self.rail_sound.iter_mut().filter_map(Option::take) {
            ctx.aems.release_object(grind);
        }
    }

    /// `UpdateParams` 0x82541978: the grind and grind-ended flags to the mixer. A grind's
    /// instances start on its first frame on a tagged surface and stop when it ends.
    pub fn update_params(&mut self, _t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        ctx.set_input(self.io, 0, if physics.is_rail_grind { 0x7FFF } else { 0 });
        ctx.set_input(self.io, 1, if physics.was_rail_grind && !physics.is_rail_grind { 0x7FFF } else { 0 });
        if self.rail_sound[0].is_some() {
            if !physics.is_rail_grind {
                self.destroy(ctx);
                self.current_grind_type = GRIND_TYPE_NONE;
            }
            return;
        }
        if !physics.is_rail_grind {
            return;
        }
        let speed = grind_speed(ctx.vault, physics.board_speed);
        let surface = if physics.grind_material == MATERIAL_MAX {
            if ctx.cvars.no_rail_when_untagged {
                return;
            }
            GRIND_MATERIAL_RAIL1
        } else {
            let surface = grind_material(ctx.vault, physics.grind_material) as i32;
            if surface == GRIND_MATERIAL_UNTAGGED {
                return;
            }
            surface
        };
        let grind_type = physics.grind_type;
        self.current_grind_type = grind_type;
        self.rail_sound[0] = Some(self.new_grind(ctx, speed, surface, grind_parts(grind_type)));
        if grind_type == GRIND_TYPE_5050 {
            self.rail_sound[1] = Some(self.new_grind(ctx, speed, surface, 1));
        }
    }

    /// `ProcessUpdate` 0x82542028: while grinding, a 50-50's second truck follows the grind type,
    /// and both instances take the speed, the mixer values and the parts.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !physics.is_rail_grind || self.rail_sound[0].is_none() {
            return;
        }
        let forced = ctx.vault.i32(RAILS, GRIND_TYPE_OVERRIDE);
        let grind_type = if forced == GRIND_TYPE_NONE { physics.grind_type } else { forced };
        let parts = grind_parts(grind_type);
        if grind_type != self.current_grind_type {
            if grind_type != GRIND_TYPE_5050 {
                if self.current_grind_type == GRIND_TYPE_5050 {
                    if let Some(second) = self.rail_sound[1].take() {
                        ctx.aems.release_object(second);
                    }
                }
            } else if self.rail_sound[1].is_none() {
                let surface = grind_surface(physics, ctx);
                let speed = grind_speed(ctx.vault, physics.board_speed);
                self.rail_sound[1] = Some(self.new_grind(ctx, speed, surface, 1));
            }
            self.current_grind_type = grind_type;
        }
        let speed = grind_speed(ctx.vault, physics.board_speed);
        let multiplier = rail_volume_multiplier(ctx.vault, grind_surface(physics, ctx), parts as u32);
        let io = self.io;
        let dry = fctiwz(ctx.output(io, 1, PresetType::Volume) as f32 * multiplier);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        let pitch = ctx.output(io, 2, PresetType::Pitch);
        for (index, slot) in self.rail_sound.iter_mut().enumerate() {
            let Some(grind) = slot.as_mut() else { continue };
            let data = &mut grind.data;
            data[7] = speed.clamp(0, 10000);
            data[0] = 0x7FFF;
            data[1] = dry.clamp(0, 0x7FFF);
            data[2] = ctx.output(io, 5, PresetType::Volume).clamp(0, 0x7FFF);
            data[15] = if ctx.local { ctx.output(io, 6, PresetType::Volume) } else { 0 }.clamp(0, 0x7FFF);
            data[5] = ctx.output(io, 3, PresetType::Frequency).clamp(0, 25000);
            data[6] = ctx.output(io, 4, PresetType::Frequency).clamp(0, 25000);
            data[3] = azimuth.clamp(0, 0x1_0000);
            data[4] = pitch.clamp(0, 0x2000);
            data[10] = if index == 0 { parts.clamp(0, 2) } else { i32::from(grind_type == GRIND_TYPE_5050) };
            ctx.aems.set_member_data(grind);
        }
    }

    /// The `Class_grind` `UpdateParams` and `ProcessUpdate` build, logged as
    /// "Grind. Parts = %d Surface = %d".
    fn new_grind(&self, ctx: &mut SfxCtx<'_>, speed: i32, surface: i32, parts: i32) -> ClassObject {
        let vault = ctx.vault;
        let volume = |keys| rail_volume(vault, keys, surface, parts as u32);
        let (sweetener, enter, exit) = (volume(SWEETENER_VOLUME), volume(ENTER_VOLUME), volume(EXIT_VOLUME));
        let (local, flange) = if ctx.local { (1, ctx.output(self.io, 6, PresetType::Volume)) } else { (0, 0) };
        let data = vec![
            0,
            0x7FFF,
            0,
            0,
            0,
            25000,
            0,
            speed.clamp(0, 10000),
            1024,
            surface.clamp(0, 14),
            parts.clamp(0, 2),
            sweetener.clamp(0, 0x7FFF),
            enter.clamp(0, 0x7FFF),
            exit.clamp(0, 0x7FFF),
            local,
            flange.clamp(0, 0x7FFF),
        ];
        let grind = ctx.aems.create_object("Class_grind", data);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} rail grind parts {parts} surface {surface}");
        }
        grind
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::AttribSource;

    /// `MaxSpeed` 45 and a 14-element rail volume array holding each element's index.
    struct Rails;

    impl AttribSource for Rails {
        fn attribute(&self, _class: u64, _collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
            match key {
                MAX_SPEED => Some(45f32.to_be_bytes().to_vec()),
                _ if RAIL_VOLUME.contains(&key) && index < 14 => Some((index as f32).to_be_bytes().to_vec()),
                _ => None,
            }
        }

        fn layout_word(&self, _class: u64, _collection: u64, _index: u32) -> u32 {
            0
        }

        fn has_collection(&self, _class: u64, _collection: u64) -> bool {
            true
        }

        fn length(&self, _class: u64, _collection: u64, key: u64) -> u32 {
            if RAIL_VOLUME.contains(&key) { 14 } else { 0 }
        }
    }

    #[test]
    fn speed_follows_the_fsel_limits() {
        let vault = Vault::new(Some(Box::new(Rails)));
        assert_eq!(grind_speed(&vault, 0.5), 0);
        assert_eq!(grind_speed(&vault, -3.0), 0);
        assert!((1..9000).contains(&grind_speed(&vault, 5.5)));
        assert_eq!(grind_speed(&vault, 100.0), 9000);
        assert_eq!(grind_speed(&vault, f32::NAN), 9000);
    }

    #[test]
    fn surfaces_stop_at_the_last_element() {
        let vault = Vault::new(Some(Box::new(Rails)));
        assert_eq!(rail_volume_multiplier(&vault, GRIND_MATERIAL_UNTAGGED, 0), 13.0);
        assert_eq!(rail_volume_multiplier(&vault, GRIND_MATERIAL_RAIL1, 0), 4.0);
        assert_eq!(rail_volume_multiplier(&vault, GRIND_MATERIAL_RAIL1, 3), 1.0);
        assert_eq!(rail_volume(&vault, SWEETENER_VOLUME, GRIND_MATERIAL_RAIL1, 0), 0);
        assert_eq!(rail_volume(&vault, SWEETENER_VOLUME, GRIND_MATERIAL_RAIL1, 3), 32767);
    }
}
