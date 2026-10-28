//! `Sk8::Audio::SFXObj_TrafficSkids` (40 bytes, TRAFFICCAR object 1, TypeInfo 0x00040010,
//! `CreateObject` 0x8252c660): a car's tyre skid and squeak, one `TRAFFIC_SKID` Csis instance
//! (`Traffic_Skid.abk` of `vehicle.big`).

use super::traffic::TrafficCarPhysics;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::player::SfxCtx;

/// `kSkidAudio` (`Audio_Traffic`) at its registered 1.
const SKID_AUDIO: bool = true;

/// `Csis::TRAFFIC_SKIDStruct` members.
const VOL_SKID: usize = 0;
const VOL_SQUEAK: usize = 1;
const AZIMUTH: usize = 3;
const PITCH: usize = 4;
const SPEED: usize = 5;
const BRAKE: usize = 6;
const ACCEL_DECEL: usize = 9;
const HI_PASS: usize = 10;
const LO_PASS: usize = 11;
const DRY_FX: usize = 12;
const WET_FX: usize = 13;
const MEMBERS: usize = 14;

const SPEED_SCALE: f32 = f32::from_bits(0x44fa_0000);
const ACCEL_SCALE: f32 = f32::from_bits(0x453b_8000);

pub struct TrafficSkids {
    /// `m_pDMixIO` (objectID 0x40010 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mpTrafficSkid`.
    instance: Option<ClassObject>,
}

impl TrafficSkids {
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self { io, instance: None }
    }

    /// `UpdateParams` 0x82516108 (attached states): `TRAFFIC_SKID` on the first update.
    pub fn update_params(&mut self, ctx: &mut SfxCtx<'_>) {
        if self.instance.is_none() {
            let mut data = vec![0; MEMBERS];
            data[PITCH] = 0x1000;
            data[LO_PASS] = 25000;
            data[DRY_FX] = 0x7FFF;
            self.instance = Some(ctx.aems.create_object("TRAFFIC_SKID", data));
        }
    }

    /// `ProcessUpdate` 0x825161f0: the skid and squeak volumes, wet level, azimuth, pitch and
    /// filters from the mixer, the car's skidding (`brake`), speed and acceleration.
    pub fn process_update(&mut self, physics: &TrafficCarPhysics, ctx: &mut SfxCtx<'_>) {
        let Some(instance) = self.instance.as_mut() else { return };
        let io = self.io;
        let (skid, squeak) = if SKID_AUDIO { (ctx.output(io, 1, PresetType::Volume), ctx.output(io, 2, PresetType::Volume)) } else { (0, 0) };
        let data = &mut instance.data;
        data[VOL_SKID] = skid.clamp(0, 0x7FFF);
        data[WET_FX] = ctx.output(io, 6, PresetType::Volume).clamp(0, 0x7FFF);
        data[AZIMUTH] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        data[BRAKE] = physics.is_skidding.clamp(0, 1);
        data[SPEED] = fctiwz(physics.speed * SPEED_SCALE).clamp(0, 0x7FFF);
        let accel = physics.acceleration * ACCEL_SCALE;
        data[PITCH] = ctx.output(io, 3, PresetType::Pitch).clamp(0, 20000);
        data[HI_PASS] = ctx.output(io, 5, PresetType::Frequency).clamp(0, 25000);
        data[LO_PASS] = ctx.output(io, 4, PresetType::Frequency).clamp(0, 25000);
        data[VOL_SQUEAK] = squeak.clamp(0, 0x7FFF);
        data[ACCEL_DECEL] = fctiwz(accel).clamp(-0x7FFF, 0x7FFF);
        ctx.aems.set_member_data(instance);
    }

    /// `Destroy` (the object's `Detach`, the code 0x8252c750 shares with `SFXObj_Treatments`):
    /// `TRAFFIC_SKID` released.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(instance) = self.instance.take() {
            ctx.aems.release_object(instance);
        }
    }
}
