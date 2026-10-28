//! `Sk8::Audio::SFXObj_Treatments` (40 bytes, PLAYER object 8, TypeInfo 0x00010080,
//! `CreateObject` 0x82530b18): the local skater's presentation treatment (`Class_Treatment`,
//! `Treatments.abk`), driven by the air time, the board speed, the game speed and the bail,
//! out-of-bounds and Own the Spot events.

use super::physics::PlayerPhysics;
use super::SfxCtx;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::vault::{class, Collection};

/// `GlobalAttributeCollection`'s `aud_general` treatments collection (+0x60).
const TREATMENTS: Collection = Collection::new(class::AUD_GENERAL, 0xEE7B_8A8A_893A_4E30);
/// `aud_general::volume_prelandwhoosh`, `volume_bigvrb`, `volume_outofbound`, `volume_wind`,
/// `volume_bail`, `volume_ownspot` and `volume_rumble`: members 31 to 37.
const VOLUMES: [u64; 7] = [
    0x32F9_111C_BF74_6F34,
    0x198C_DF44_C971_C07C,
    0xE4BC_6FE0_30A5_53C4,
    0x1F11_951C_2AF5_8CC7,
    0xFAA7_C671_17AB_DCA4,
    0x7F8D_9033_6A95_3ADF,
    0xA949_BAF1_C239_C13B,
];

/// `Csis::Class_Treatment::Class_Treatment` 0x82513bd8: each member's clamp.
const TREATMENT_RANGES: [i32; 38] = [
    0x7FFF, 0x7FFF, 0x7FFF, 0xFFFF, 0x2000, 25000, 25000, 20, 100, 1, 20, 1, 1, 2, 1000, 10000, 10000, 10000, 1000, 1,
    1000, 1, 1000, 1000, 1, 10000, 10000, 1, 1, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF, 0x7FFF,
];

/// `Class_TreatmentStruct` members.
const VOLUME: usize = 0;
const DRY_LEVEL: usize = 1;
const WET_LEVEL: usize = 2;
const AZIMUTH: usize = 3;
const PITCH: usize = 4;
const LOW_PASS: usize = 5;
const HI_PASS: usize = 6;
const EVENT_TYPE: usize = 7;
const LANDING_VELOCITY: usize = 14;
const TIME_IN_AIR: usize = 15;
const TIME_IN_AIR_REMAINING: usize = 16;
const JUMP_HEIGHT: usize = 17;
const GAME_SPEED: usize = 18;
const OWN_THE_SPOT: usize = 21;
const FUN_METER: usize = 22;
const BAIL_METER: usize = 23;
const OUT_OF_BOUND: usize = 24;
const OUT_OF_BOUND_PROXIMITY: usize = 25;
const SPEED: usize = 26;
const REPLAY_FORWARD: usize = 27;
const CELEBRATION: usize = 28;
const VOLUME_PRE_LAND_WHSH: usize = 31;

/// `event_Type` values `ProcessUpdate` writes.
const EVENT_OUT_OF_BOUND: i32 = 1;
const EVENT_OWN_THE_SPOT: i32 = 4;
const EVENT_BAIL: i32 = 5;

/// `__real_4326aaab` (166.66667) and `__real_3da3d70a` (0.08).
const JUMP_HEIGHT_SCALE: f32 = f32::from_bits(0x4326_AAAB);
const SPEED_SCALE: f32 = f32::from_bits(0x3DA3_D70A);

/// The constructor (`CreateObject` 0x82530b18) clears `mpTreatments`.
#[derive(Default)]
pub struct Treatments {
    /// `m_pDMixIO` (objectID 0x40010080 + the state's instance).
    pub io: Option<DMixIoId>,
    treatment: Option<ClassObject>,
}

/// `fsel(-x, 0, x)`, then `fsel(max - x, x, max)`: NaN reads as `max`.
fn clamp_fsel(value: f32, max: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if max - low >= 0.0 { low } else { max }
}

impl Treatments {
    /// The `InitSFX` (0x82515d80, shared) `SetupLoadData` runs on attach: the mixer outputs on.
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `Destroy` 0x8252c750, which `Detach` runs.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(treatment) = self.treatment.take() {
            ctx.aems.release_object(treatment);
        }
    }

    /// `UpdateParams` 0x82530c08: the local skater's instance, created once with the treatments'
    /// volumes, pitch 0x1000, low pass 25000 and game speed 500.
    pub fn update_params(&mut self, _t: f32, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.local || self.treatment.is_some() {
            return;
        }
        let mut data = vec![0; TREATMENT_RANGES.len()];
        data[PITCH] = 0x1000;
        data[LOW_PASS] = 25000;
        data[GAME_SPEED] = 500;
        for (member, key) in (VOLUME_PRE_LAND_WHSH..).zip(VOLUMES) {
            data[member] = ctx.vault.i32(TREATMENTS, key);
        }
        let data = data.iter().zip(TREATMENT_RANGES).map(|(&value, max)| value.clamp(0, max)).collect();
        self.treatment = Some(ctx.aems.create_object("Class_Treatment", data));
    }

    /// `ProcessUpdate` 0x82530da0: `kEnableTreatments` gates the mixer volume; the bail, then
    /// out-of-bounds, then Own the Spot event wins `event_Type`.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let Some(mut treatment) = self.treatment.take() else { return };
        let io = self.io;
        let data = &mut treatment.data;
        data[VOLUME] = if ctx.cvars.enable_treatments { ctx.output(io, 2, PresetType::Volume).clamp(0, 0x7FFF) } else { 0 };
        data[DRY_LEVEL] = 0x7FFF;
        data[WET_LEVEL] = 0;
        data[AZIMUTH] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        data[PITCH] = ctx.output(io, 1, PresetType::Pitch).clamp(0, 0x2000);
        data[LOW_PASS] = 25000;
        data[HI_PASS] = 0;
        data[GAME_SPEED] = fctiwz(physics.game_speed * 500.0).clamp(0, 1000);
        data[REPLAY_FORWARD] = i32::from(!physics.is_rewinding);
        data[TIME_IN_AIR] = fctiwz(physics.time_in_air * 1000.0).clamp(0, 10000);
        data[TIME_IN_AIR_REMAINING] = fctiwz(physics.time_to_land * 1000.0).clamp(0, 10000);
        data[JUMP_HEIGHT] = fctiwz(clamp_fsel(physics.jump_height * JUMP_HEIGHT_SCALE, 1000.0)).clamp(0, 10000);
        let speed = fctiwz(clamp_fsel(physics.board_speed * SPEED_SCALE, 1.0) * 10000.0);
        data[SPEED] = speed.clamp(0, 10000);
        data[LANDING_VELOCITY] = speed.clamp(0, 1000);
        data[EVENT_TYPE] = 0;
        data[OUT_OF_BOUND] = 0;
        data[OWN_THE_SPOT] = 0;
        data[CELEBRATION] = 0;
        if ctx.bail {
            data[EVENT_TYPE] = EVENT_BAIL;
            data[BAIL_METER] = fctiwz(ctx.bail_severity * 1000.0).clamp(0, 1000);
        }
        if ctx.out_of_bounds {
            data[EVENT_TYPE] = EVENT_OUT_OF_BOUND;
            data[OUT_OF_BOUND] = 1;
            data[OUT_OF_BOUND_PROXIMITY] = fctiwz(ctx.out_of_bounds_amount * 10000.0).clamp(0, 10000);
        }
        data[FUN_METER] = 0;
        if ctx.in_own_the_spot {
            data[EVENT_TYPE] = EVENT_OWN_THE_SPOT;
            data[OWN_THE_SPOT] = 1;
        }
        ctx.aems.set_member_data(&treatment);
        self.treatment = Some(treatment);
    }
}
