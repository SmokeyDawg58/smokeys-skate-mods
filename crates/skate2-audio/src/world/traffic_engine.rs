//! `Sk8::Audio::SFXObj_TrafficEngine` (168 bytes, TRAFFICCAR object 0, TypeInfo 0x00040000,
//! `CreateObject` 0x8252b0e0): a car's engine and exhaust, one `TRAFFIC_CAR` Csis instance
//! (`AEMS_TRAFFIC.csi`, the `C0x_*.abk` banks of `vehicle.big`) driven by an RPM simulated from
//! the car's speed and its `aud_traffic_engine` collection.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::traffic::TrafficCarPhysics;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::player::pos3d::MicrophoneMan;
use crate::player::SfxCtx;
use crate::vault::{Collection, Vault};
use crate::vmx;

/// `aud_traffic_engine`: the class key and the attributes `UpdateParams` copies.
pub const AUD_TRAFFIC_ENGINE: u64 = 0x2590_9516_3B97_4174;
const DELTA_TIME_SCALER: u64 = 0x24D1_BAA3_09A8_DABE;
const GAS_OFF_SCALER: u64 = 0xBACA_53EF_608D_56E5;
const GAS_SPEED_SCALER: u64 = 0x3794_78BE_6600_553E;
const MODULATION_AMPLITUDE: u64 = 0x2C15_86C6_D46B_89DF;
const MODULATION_PERIOD: u64 = 0xD048_F5E8_09B0_70C0;
const RPM_DOWNSHIFT_SCALER: u64 = 0xE6B3_BD54_DF5A_C0A5;
const RPM_UPSHIFT_SCALER: u64 = 0x7EA0_A898_87B3_746C;
const RPM_DROPS_PER_SECOND: u64 = 0x7FD8_4F2C_9C37_4F46;
const SPEED_DELTA_PER_GEAR: u64 = 0xE67C_4A17_326C_555D;
const NUM_GEARS: u64 = 0x07CE_76F8_BE00_66C1;
const USE_GAS_FILTER_SCALER: u64 = 0x8C5E_EDE4_36EE_A065;
const USE_GAS_PITCH_SCALER: u64 = 0xE9B0_F8D7_BFD7_D1D9;
const USE_GAS_VOL_SCALER: u64 = 0xD453_D28D_0602_41B5;
const VOLUME_ON: u64 = 0xA936_66FD_7ECC_3AE7;
const ENG_EXH_BIAS: u64 = 0x763D_B0A1_68A4_9E93;

/// The `Audio_Traffic` cvars at their registered values: `kAddDeltaTime` 0, `kEngineAudio` 1.
const ADD_DELTA_TIME: bool = false;
const ENGINE_AUDIO: bool = true;

/// `Csis::TRAFFIC_CARStruct` members.
const RPM: usize = 0;
const VOLUME_ENG: usize = 1;
const VOLUME_EXH: usize = 2;
const VOL_BY_FAST: usize = 3;
const VOL_BY_SLOW: usize = 4;
const AZIMUTH_ENG: usize = 5;
const AZIMUTH_EXH: usize = 6;
const AZIMUTH_BY: usize = 7;
const PITCH_OFFSET: usize = 8;
const LO_PASS: usize = 9;
const HI_PASS: usize = 10;
const FX_DRY: usize = 11;
const FX_WET: usize = 12;
const CAR_SPEED: usize = 13;
const REL_SPEED: usize = 14;
const CAR_ID: usize = 15;
const ACCEL_DECEL: usize = 16;
const SHIFT: usize = 18;
const MEMBERS: usize = 19;

const SPEED_INPUT: f32 = f32::from_bits(0x3d4c_cccd);
const Q15: f32 = f32::from_bits(0x46ff_fe00);
const NEG_INV_Q15: f32 = f32::from_bits(0xb800_0100);
const SPEED_SCALE: f32 = f32::from_bits(0x44fa_0000);
const ACCEL_SCALE: f32 = f32::from_bits(0x453b_8000);

/// `fsel(-x, 0, x)` then `fsel(1 - x, x, 1)`: within 0..=1, 1 for NaN.
fn unit(value: f32) -> f32 {
    let low = if -value >= 0.0 { 0.0 } else { value };
    if 1.0 - low >= 0.0 { low } else { 1.0 }
}

/// A signed 16-bit attribute (`lhz`, `extsh`).
fn short(vault: &Vault, collection: Collection, key: u64) -> i32 {
    let bytes = vault.bytes(collection, key, 0, 2);
    i32::from(i16::from_be_bytes([bytes[0], bytes[1]]))
}

/// The car type at layout byte 8 of an `aud_traffic_engine` collection (`car_ID`).
pub fn layout_car_id(vault: &Vault, collection: Collection) -> i16 {
    (vault.layout_word(collection, 2) >> 16) as u16 as i16
}

pub struct TrafficEngine {
    /// `m_pDMixIO` (objectID 0x40000 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mpTrafficCarEngExh`.
    instance: Option<ClassObject>,
    car_id: i32,
    current_rpm: f32,
    current_speed: f32,
    rpm_modulate: f32,
    rpm_direction: f32,
    gas: f32,
    delta_time: f32,
    min_rpm: f32,
    max_rpm: f32,
    delta_time_scaler: f32,
    gas_off_scaler: f32,
    gas_speed_scaler: f32,
    modulation_amplitude: f32,
    modulation_period: f32,
    rpm_downshift_scaler: f32,
    rpm_upshift_scaler: f32,
    rpm_drops_per_second: f32,
    speed_delta_per_gear: f32,
    num_gears: i32,
    use_gas_filter_scaler: i32,
    use_gas_pitch_scaler: i32,
    use_gas_vol_scaler: i32,
    volume_on: bool,
    eng_exh_bias: i32,
    /// `mDownShift`, `mUpShift`: the updates left reporting a shift (only the constructor clears
    /// them).
    down_shift: i32,
    up_shift: i32,
}

impl TrafficEngine {
    /// The constructor 0x82515b98.
    #[must_use]
    pub fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            instance: None,
            car_id: 0,
            current_rpm: 0.0,
            current_speed: 0.0,
            rpm_modulate: 0.0,
            rpm_direction: 1.0,
            gas: 1.0,
            delta_time: 0.0,
            min_rpm: 0.0,
            max_rpm: 0.0,
            delta_time_scaler: 1.0,
            gas_off_scaler: 0.125,
            gas_speed_scaler: f32::from_bits(0x3e00_ebee),
            modulation_amplitude: 100.0,
            modulation_period: 10.0,
            rpm_downshift_scaler: 1.0,
            rpm_upshift_scaler: 0.5,
            rpm_drops_per_second: 2000.0,
            speed_delta_per_gear: 3.0,
            num_gears: 5,
            use_gas_filter_scaler: 0,
            use_gas_pitch_scaler: 0,
            use_gas_vol_scaler: 0,
            volume_on: true,
            eng_exh_bias: 0,
            down_shift: 0,
            up_shift: 0,
        }
    }

    /// `UpdateParams` 0x8252b1f8 (attached states): the first update after an attach finds the
    /// car's `aud_traffic_engine` collection, picks the engine bank (`car_ID`: a 1 becomes 7 or 8
    /// two times in three, a 3 becomes 6 one time in two), creates `TRAFFIC_CAR` and restarts the
    /// RPM from the collection.
    pub fn update_params(&mut self, physics: &TrafficCarPhysics, ctx: &mut SfxCtx<'_>) {
        if self.instance.is_some() {
            return;
        }
        let engine = Collection::new(AUD_TRAFFIC_ENGINE, physics.engine_key);
        self.car_id = i32::from(layout_car_id(ctx.vault, engine));
        if self.car_id == 1 {
            let roll = ctx.rng.next_u32() % 100;
            if roll < 33 {
                self.car_id = 7;
            } else if roll < 66 {
                self.car_id = 8;
            }
        } else if self.car_id == 3 && ctx.rng.next_u32() % 100 < 50 {
            self.car_id = 6;
        }
        let mut data = vec![0; MEMBERS];
        data[PITCH_OFFSET] = 0x1000;
        data[LO_PASS] = 25000;
        data[FX_DRY] = 0x7FFF;
        data[CAR_ID] = self.car_id.clamp(0, 9);
        self.instance = Some(ctx.aems.create_object("TRAFFIC_CAR", data));
        let vault = ctx.vault;
        self.current_speed = physics.speed;
        self.delta_time = 0.0;
        self.rpm_modulate = 0.0;
        self.rpm_direction = 1.0;
        self.current_rpm = 0.0;
        self.gas = 1.0;
        self.min_rpm = vault.layout_f32(engine, 0);
        self.max_rpm = vault.layout_f32(engine, 1);
        self.delta_time_scaler = vault.f32(engine, DELTA_TIME_SCALER);
        self.gas_off_scaler = vault.f32(engine, GAS_OFF_SCALER);
        self.gas_speed_scaler = vault.f32(engine, GAS_SPEED_SCALER);
        self.modulation_amplitude = vault.f32(engine, MODULATION_AMPLITUDE);
        self.modulation_period = vault.f32(engine, MODULATION_PERIOD);
        self.rpm_downshift_scaler = vault.f32(engine, RPM_DOWNSHIFT_SCALER);
        self.rpm_upshift_scaler = vault.f32(engine, RPM_UPSHIFT_SCALER);
        self.rpm_drops_per_second = vault.f32(engine, RPM_DROPS_PER_SECOND);
        self.speed_delta_per_gear = vault.f32(engine, SPEED_DELTA_PER_GEAR);
        self.num_gears = short(vault, engine, NUM_GEARS);
        self.use_gas_filter_scaler = short(vault, engine, USE_GAS_FILTER_SCALER);
        self.use_gas_pitch_scaler = short(vault, engine, USE_GAS_PITCH_SCALER);
        self.use_gas_vol_scaler = short(vault, engine, USE_GAS_VOL_SCALER);
        self.volume_on = vault.bool(engine, VOLUME_ON);
        self.eng_exh_bias = vault.i32(engine, ENG_EXH_BIAS);
    }

    /// `ProcessUpdate` 0x8252b548: input 0 (the speed over 20 m/s), the RPM toward the speed's
    /// share of the current gear's range plus a wobble (`mRpmModulate`), falling with the gas off
    /// on an upshift and rising on a downshift, then the members: the RPM, the mixer volumes
    /// (engine and exhaust shifted against each other by `EngExhBias` and how far the car faces
    /// the camera microphone), azimuths, pitch and filters, the speeds and the shift.
    pub fn process_update(&mut self, physics: &TrafficCarPhysics, state_delta_time: f32, mics: &MicrophoneMan, ctx: &mut SfxCtx<'_>) {
        if self.instance.is_none() {
            return;
        }
        let io = self.io;
        self.current_speed = physics.speed;
        ctx.set_input(io, 0, fctiwz(unit(physics.speed * SPEED_INPUT) * Q15).clamp(0, 0x7FFF));
        self.delta_time = if ADD_DELTA_TIME { state_delta_time + self.delta_time } else { state_delta_time };
        let delta = self.delta_time_scaler * self.delta_time;
        self.delta_time = delta;
        self.rpm_modulate = (self.modulation_period * self.rpm_direction).mul_add(delta, self.rpm_modulate);
        if self.rpm_modulate > self.modulation_amplitude {
            self.rpm_direction = -1.0;
        } else if self.rpm_modulate < -self.modulation_amplitude {
            self.rpm_direction = 1.0;
        }
        let mut gear_speed = 0.0;
        let mut gear = 1;
        while gear <= self.num_gears {
            gear_speed = gear as f32 * self.speed_delta_per_gear;
            if self.current_speed < gear_speed {
                break;
            }
            gear += 1;
        }
        let drop = self.rpm_drops_per_second * delta;
        let mut target = self.current_speed * self.max_rpm / gear_speed + self.rpm_modulate;
        if !(target > self.min_rpm) {
            target = self.min_rpm;
        } else if !(target < self.max_rpm) {
            target = self.max_rpm;
        }
        let rpm = self.current_rpm;
        let difference = target - rpm;
        let mut gas_target = 1.0;
        if !(difference > -drop) {
            self.current_rpm = (-self.rpm_upshift_scaler).mul_add(drop, rpm);
            gas_target = self.gas_off_scaler;
            self.up_shift = 3;
        } else if difference < drop {
            self.current_rpm = target;
        } else {
            self.current_rpm = self.rpm_downshift_scaler.mul_add(drop, rpm);
            self.down_shift = 3;
        }
        if !(self.current_rpm > self.min_rpm) {
            self.current_rpm = self.min_rpm;
        } else if !(self.current_rpm < self.max_rpm) {
            self.current_rpm = self.max_rpm;
        }
        let gas_step = self.gas_speed_scaler * delta;
        let gas_difference = gas_target - self.gas;
        if !(gas_difference > -gas_step) {
            self.gas -= gas_step;
        } else if gas_difference < gas_step {
            self.gas = gas_target;
        } else {
            self.gas += gas_step;
        }
        let volume = |index| ctx.output(io, index, PresetType::Volume);
        let (engine, exhaust, by) = if self.use_gas_vol_scaler == 0 {
            (volume(3), volume(4), volume(5))
        } else {
            (fctiwz(volume(3) as f32 * self.gas), fctiwz(volume(4) as f32 * self.gas), fctiwz(volume(5) as f32 * self.gas))
        };
        let forward = vmx::normalize_unguarded(physics.forward);
        let to_car = vmx::normalize_unguarded(vmx::sub(physics.car_pos, mics.microphones[0].cur_position));
        let relative_speed = vmx::length(vmx::sub(mics.microphones[1].velocity, physics.velocity)) * SPEED_SCALE;
        let bias = self.eng_exh_bias as f32 * vmx::dot3(to_car, forward);
        let exhaust = exhaust.wrapping_sub(fctiwz(exhaust as f32 * bias * NEG_INV_Q15));
        let wet = volume(9);
        let azimuth = |index| ctx.output(io, index, PresetType::Azimuth);
        let azimuths = [azimuth(0), azimuth(1), azimuth(2)];
        let pitch = ctx.output(io, 6, PresetType::Pitch) as f32;
        let pitch = if self.use_gas_pitch_scaler == 0 { pitch } else { pitch * self.gas };
        let low_pass = ctx.output(io, 7, PresetType::Frequency) as f32;
        let low_pass = if self.use_gas_filter_scaler == 0 { low_pass } else { low_pass * self.gas };
        let high_pass = ctx.output(io, 8, PresetType::Frequency);
        let shift = if self.up_shift > 0 {
            self.up_shift -= 1;
            1
        } else if self.down_shift > 0 {
            self.down_shift -= 1;
            -1
        } else {
            0
        };
        let Some(instance) = self.instance.as_mut() else { return };
        let data = &mut instance.data;
        data[RPM] = fctiwz(self.current_rpm).clamp(0, 10000);
        if ENGINE_AUDIO && self.volume_on {
            data[VOLUME_ENG] = engine.wrapping_sub(fctiwz(engine as f32 * -bias * NEG_INV_Q15)).clamp(0, 0x7FFF);
            data[VOLUME_EXH] = exhaust.clamp(0, 0x7FFF);
            data[VOL_BY_FAST] = by.clamp(0, 0x7FFF);
            data[VOL_BY_SLOW] = by.clamp(0, 0x7FFF);
        } else {
            data[VOLUME_ENG] = 0;
            data[VOLUME_EXH] = 0;
            data[VOL_BY_FAST] = 0;
            data[VOL_BY_SLOW] = 0;
        }
        data[FX_WET] = wet.clamp(0, 0x7FFF);
        data[AZIMUTH_ENG] = azimuths[0].clamp(0, 0x10000);
        data[AZIMUTH_EXH] = azimuths[1].clamp(0, 0x10000);
        data[AZIMUTH_BY] = azimuths[2].clamp(0, 0xFFFF);
        data[PITCH_OFFSET] = fctiwz(pitch).clamp(0, 0x3FFF);
        data[LO_PASS] = fctiwz(low_pass).clamp(0, 25000);
        data[HI_PASS] = high_pass.clamp(0, 25000);
        data[ACCEL_DECEL] = fctiwz(physics.acceleration * ACCEL_SCALE).clamp(-0x7FFF, 0x7FFF);
        data[SHIFT] = shift;
        data[CAR_SPEED] = fctiwz(self.current_speed * SPEED_SCALE).clamp(0, 0x7FFF);
        data[REL_SPEED] = fctiwz(relative_speed).clamp(0, 0x7FFF);
        ctx.aems.set_member_data(instance);
    }

    /// `Destroy` 0x8252b198 (the object's `Detach`): `TRAFFIC_CAR` released.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(instance) = self.instance.take() {
            ctx.aems.release_object(instance);
        }
    }
}
