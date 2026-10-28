//! `Sk8::Audio::SFXObj_TrafficHorn` (64 bytes, TRAFFICCAR object 2, TypeInfo 0x00040020,
//! `CreateObject` 0x8252bf48): a car's horn (`TRAFFIC_HORN`, `Traffic_Horn.abk` of `vehicle.big`)
//! and, while its horn state is `Alarm`, its car alarm (`c_car_alarm` of
//! `Sk8_Emitters_Project.csi`, `car_alarms.abk` of `emitters2.big`).

use super::traffic::TrafficCarPhysics;
use super::traffic_engine::{layout_car_id, AUD_TRAFFIC_ENGINE};
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::player::pos3d::MicrophoneMan;
use crate::player::SfxCtx;
use crate::vault::Collection;
use crate::vmx;

/// `eHornState` `Alarm`.
const HORN_ALARM: i32 = 6;
/// `kHornAudio` (`Audio_Traffic`) at its registered 1.
const HORN_AUDIO: bool = true;

/// `Csis::TRAFFIC_HORNStruct` members.
const VOLUME: usize = 0;
const HORN_ON: usize = 2;
const PITCH: usize = 3;
const DISTANCE: usize = 4;
const SPEED: usize = 5;
const REL_SPEED: usize = 6;
const AZIMUTH: usize = 7;
const LO_PASS: usize = 8;
const HI_PASS: usize = 9;
const DRY_FX: usize = 10;
const WET_FX: usize = 11;

const DISTANCE_SCALE: f32 = f32::from_bits(0x447a_0000);
const SPEED_SCALE: f32 = f32::from_bits(0x44fa_0000);

pub struct TrafficHorn {
    /// `m_pDMixIO` (objectID 0x40020 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mpTrafficHorn` and `mpCarAlarm`.
    horn: Option<ClassObject>,
    alarm: Option<ClassObject>,
    /// `mEngineAttrib`'s collection; `None` is the constructor's `DefaultDataArea` layout.
    engine: Option<Collection>,
    /// `mpLastHornState`.
    last_horn_state: i32,
}

impl TrafficHorn {
    /// The constructor 0x82515dc8.
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self { io, horn: None, alarm: None, engine: None, last_horn_state: 0 }
    }

    /// `UpdateParams` 0x82515ea0 (attached states): the horn on the first update (`horn_ID` a
    /// random 0-8), and the car alarm (`car_alarm_ID` a random 0-3) only while the horn state is
    /// `Alarm`.
    pub fn update_params(&mut self, physics: &TrafficCarPhysics, ctx: &mut SfxCtx<'_>) {
        if self.horn.is_none() {
            let id = (ctx.rng.next_u32() % 9) as i32;
            self.horn = Some(ctx.aems.create_object("TRAFFIC_HORN", vec![0, id.clamp(0, 10), 0, 0x1000, 0, 0, 0, 0, 25000, 0, 0x7FFF, 0]));
        }
        if physics.horn_state == HORN_ALARM {
            if self.alarm.is_none() {
                let id = (ctx.rng.next_u32() & 3) as i32;
                self.alarm = Some(ctx.aems.create_object("c_car_alarm", vec![0, 0x7FFF, 0x7FFF, 0, 0x1000, 25000, 0, 0, id.clamp(0, 4)]));
            }
        } else if let Some(alarm) = self.alarm.take() {
            ctx.aems.release_object(alarm);
        }
    }

    /// `ProcessUpdate` 0x8252c070. With the alarm sounding the horn is off and the alarm takes the
    /// mixer's dry, wet, azimuth, pitch and low pass. Otherwise a new horn state reloads
    /// `mEngineAttrib` from the car's engine key, a car type 0 keeps the horn silent, and the horn
    /// gets input 0, the mixer values and the distance and speeds relative to the skater
    /// microphone.
    pub fn process_update(&mut self, physics: &TrafficCarPhysics, mics: &MicrophoneMan, ctx: &mut SfxCtx<'_>) {
        if self.horn.is_none() {
            return;
        }
        let io = self.io;
        let mut state = physics.horn_state;
        if state == HORN_ALARM {
            ctx.set_input(io, 0, 0);
            if let Some(horn) = self.horn.as_mut() {
                horn.data[HORN_ON] = 0;
                ctx.aems.set_member_data(horn);
            }
            // Natively `mpCarAlarm` is not checked: `UpdateParams` made it in this state.
            let Some(alarm) = self.alarm.as_mut() else { return };
            let data = &mut alarm.data;
            data[0] = 0x7FFF;
            data[1] = ctx.output(io, 6, PresetType::Volume).clamp(0, 0x7FFF);
            data[2] = ctx.output(io, 10, PresetType::Volume).clamp(0, 0x7FFF);
            data[3] = ctx.output(io, 5, PresetType::Azimuth).clamp(0, 0xFFFF);
            data[4] = ctx.output(io, 7, PresetType::Pitch).clamp(0, 0x2000);
            data[5] = ctx.output(io, 9, PresetType::Frequency).clamp(0, 25000);
            ctx.aems.set_member_data(alarm);
            return;
        }
        if state != 0 {
            if state != self.last_horn_state {
                self.engine = Some(Collection::new(AUD_TRAFFIC_ENGINE, physics.engine_key));
            }
            if self.engine.map_or(0, |engine| layout_car_id(ctx.vault, engine)) == 0 {
                state = 0;
            }
        }
        self.last_horn_state = state;
        ctx.set_input(io, 0, if state == 0 { 0 } else { 0x7FFF });
        let dry = if HORN_AUDIO { ctx.output(io, 1, PresetType::Volume) } else { 0 };
        let wet = ctx.output(io, 8, PresetType::Volume);
        let skater = mics.microphones[1];
        let distance = vmx::length(vmx::sub(physics.car_pos, skater.cur_position)) * DISTANCE_SCALE;
        let relative_speed = vmx::length(vmx::sub(skater.velocity, physics.velocity)) * SPEED_SCALE;
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        let pitch = ctx.output(io, 2, PresetType::Pitch);
        let low_pass = ctx.output(io, 3, PresetType::Frequency);
        let high_pass = ctx.output(io, 4, PresetType::Frequency);
        let Some(horn) = self.horn.as_mut() else { return };
        let data = &mut horn.data;
        data[VOLUME] = 0x7FFF;
        data[DRY_FX] = dry.clamp(0, 0x7FFF);
        data[WET_FX] = wet.clamp(0, 0x7FFF);
        data[HORN_ON] = state.clamp(0, 6);
        data[DISTANCE] = fctiwz(distance).clamp(0, 0x7FFF);
        data[SPEED] = fctiwz(physics.speed * SPEED_SCALE).clamp(0, 0x7FFF);
        data[REL_SPEED] = fctiwz(relative_speed).clamp(0, 0x7FFF);
        data[AZIMUTH] = azimuth.clamp(0, 0xFFFF);
        data[PITCH] = pitch.clamp(0, 0x7FFF);
        data[LO_PASS] = low_pass.clamp(0, 25000);
        data[HI_PASS] = high_pass.clamp(0, 25000);
        ctx.aems.set_member_data(horn);
    }

    /// `Destroy` (the object's `Detach`, the code 0x8252c000 shares with `SFXObj_Tricks`): the
    /// horn and the alarm released. `mEngineAttrib` and `mpLastHornState` stay.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(horn) = self.horn.take() {
            ctx.aems.release_object(horn);
        }
        if let Some(alarm) = self.alarm.take() {
            ctx.aems.release_object(alarm);
        }
    }
}
