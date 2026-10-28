//! `Sk8::Audio::SFXObj_PedestrianSFX` (540 bytes, PEDESTRIAN object 1, `CreateObject`
//! 0x82545700): each foot's `livingword_footstep` AEMS instance (`SK8_AEMS_Foley.csi`, played
//! from the `fstep_livingworld.abk` bank `LoadFoleyData` 0x824f0bf0 loads), a cloth splice per
//! step, and a whoosh-by as the pedestrian passes the camera.

use super::{PedAudioState, PedestrianPhysics, FOOTSTEP, SECURITY_GUARD};
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::physics::point_graph;
use crate::player::pos3d::MicrophoneMan;
use crate::player::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::presentation::MATERIAL_MAX;
use crate::random::RwRandom;
use crate::splicer::{SpliceId, SplicePlayParams, SpliceUpdateParams};
use crate::surface::foot_step_material;
use crate::vault::{class, Collection, Vault};
use crate::vmx;
use crate::world::whoosh::WhooshManager;

/// `GlobalAttributeCollection::aud_general_clothing` and its walk/run speed limits.
const CLOTHING: Collection = Collection::new(class::AUD_GENERAL, 0x3982_76A6_3764_95EE);
const CLOTH_SPEEDS: u64 = 0xE12A_F885_D3C3_A168;
/// The `aud_whooshby` class and `GlobalAttributeCollection::aud_whooshby_pedestrian`, whose key
/// `CheckForWhooshes` passes to `PlayWhooshSound`. Its layout's float is the whoosh distance;
/// element 2 of `WHOOSH_SPEEDS` the pedestrian's relative speed.
const AUD_WHOOSHBY: u64 = 0x4F32_993D_1936_028A;
const WHOOSHBY_PEDESTRIAN_KEY: u64 = 0xAD8D_E7E5_DA12_6B8A;
const WHOOSHBY_PEDESTRIAN: Collection = Collection::new(AUD_WHOOSHBY, WHOOSHBY_PEDESTRIAN_KEY);
const WHOOSH_SPEEDS: u64 = 0x79B8_671F_26F9_A0DD;
/// `aud_general_footstep`: the notch ranges and Q, the speed graph (pedestrian speed to AEMS
/// speed) and the `step_vol`, `rough_grit_vol` and `stumble_vol` volumes.
const NOTCH_FREQ_MAX: u64 = 0x43AA_0442_D256_3D17;
const NOTCH_FREQ_MIN: u64 = 0x859D_597E_1862_647E;
const NOTCH_GAIN_MAX: u64 = 0xB795_9E9B_E344_BA44;
const NOTCH_GAIN_MIN: u64 = 0x69F6_08EE_8A18_8CCB;
const NOTCH_Q: u64 = 0x1C23_7A44_FB97_22C8;
const SPEED_GRAPH: u64 = 0x90B4_7430_C4ED_2CCC;
const VOLUMES: u64 = 0x62A2_E642_3893_4734;
/// `SPLICE_TYPE` 23 and its `sk82_cloth_foley` `clth_wlk`, `clth_run` and `clth_sprnt`.
const SPLICE_CLOTH: usize = 23;
const CLOTH_WALK: i32 = 0x15;
const CLOTH_RUN: i32 = 0x16;
const CLOTH_SPRINT: i32 = 0x17;
/// `eAudioMaterialType_Concrete_Polished` (the `tFootStep` constructor's) and
/// `_Concrete_Rough` (a foot without a surface).
const CONCRETE_POLISHED: u32 = 2;
const CONCRETE_ROUGH: u32 = 3;
/// The `ePedestrianAudioState`s of a side step and a stumble.
const AVOID_JUMP: u32 = 4;
const AVOID_SKATER: u32 = 5;
const COLLISION_KNOCKDOWN: u32 = 6;
const COLLISION_STANDING: u32 = 7;
/// `Audio_Pedestrian` `kEnableFootSteps` at its registered default.
const ENABLE_FOOT_STEPS: bool = true;

const RIGHT: usize = 0;
const LEFT: usize = 1;

/// `Csis::Livingword_footstep` members (`Livingword_footstepStruct`, 27 words).
const MEMBERS: usize = 27;
const VOLUME: usize = 0;
const AZIMUTH: usize = 1;
const PITCH: usize = 2;
const SPEED: usize = 3;
const LOW_PASS: usize = 4;
const HIGH_PASS: usize = 5;
const WET_FX: usize = 6;
const DRY_FX: usize = 7;
const FOOT_HEEL: usize = 8;
const FOOT_TOE: usize = 9;
const SIDE_STEP: usize = 10;
const TURN_AROUND: usize = 11;
const STUMBLE: usize = 12;
const PLAYER_SPEED: usize = 13;
const SHOE_TYPE: usize = 14;
const FOOT_SURFACE: usize = 15;
const TERRAIN: usize = 16;
const STEP_VOL: usize = 17;
const LOCAL_SKATER: usize = 20;
const GAIN_0: usize = 21;
const FREQ_0: usize = 22;
const Q_0: usize = 23;
const GAIN_1: usize = 24;
const FREQ_1: usize = 25;
const Q_1: usize = 26;

/// `Sk8::PointGraphEval::Evaluate(16, x, y, speed)` over the 0x90-byte footstep speed graph (x at
/// byte 0x10, y at byte 0x50), truncated.
fn speed_graph(vault: &Vault, value: f32) -> i32 {
    let bytes = vault.bytes(FOOTSTEP, SPEED_GRAPH, 0, 0x90);
    let at = |offset: usize| f32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"));
    let x: [f32; 16] = std::array::from_fn(|i| at(0x10 + 4 * i));
    let y: [f32; 16] = std::array::from_fn(|i| at(0x50 + 4 * i));
    fctiwz(point_graph(&x, &y, value))
}

/// `FootStepUtils::GetFootStepClothSplice` 0x824f5c70 (`ble` then `bgt`: an unordered speed
/// walks).
fn foot_step_cloth_splice(vault: &Vault, speed: f32) -> i32 {
    if speed > vault.f32_at(CLOTHING, CLOTH_SPEEDS, 0) {
        CLOTH_SPRINT
    } else if speed > vault.f32_at(CLOTHING, CLOTH_SPEEDS, 1) {
        CLOTH_RUN
    } else {
        CLOTH_WALK
    }
}

/// `new Csis::Livingword_footstep` as `CheckForFootSteps` builds it; the left foot's `Speed`
/// member is 0, the right foot's 1000.
fn new_foot_step(speed: i32, aems_speed: i32, q: [i32; 2], ctx: &mut SfxCtx<'_>) -> ClassObject {
    let mut data = vec![0; MEMBERS];
    data[VOLUME] = 0x7FFF;
    data[PITCH] = 0x1000;
    data[SPEED] = speed;
    data[LOW_PASS] = 25000;
    data[PLAYER_SPEED] = aems_speed.clamp(0, 1000);
    data[SHOE_TYPE] = 1;
    data[FOOT_SURFACE] = 1;
    data[TERRAIN] = 1;
    data[GAIN_0] = 0x7FFF;
    data[FREQ_0] = 24000;
    data[Q_0] = q[0].clamp(0, 20);
    data[GAIN_1] = 0x7FFF;
    data[FREQ_1] = 24000;
    data[Q_1] = q[1].clamp(0, 20);
    ctx.aems.create_object("livingword_footstep", data)
}

/// One foot: the `tFootStep` (240 bytes) fields a pedestrian uses. Its collision and foot-push
/// splicers never play here, so `FootStepUtils::CleanUpSplices` 0x824e8940 and
/// `CleanUpSpliceVoices` 0x824f5e20 find nothing to free in `Destroy`.
struct PedFoot {
    /// `mpPedFootStep`.
    foot_step: Option<ClassObject>,
    notch_gain: [f32; 2],
    notch_gain_min: [f32; 2],
    notch_gain_max: [f32; 2],
    notch_freq: [f32; 2],
    notch_freq_min: [f32; 2],
    notch_freq_max: [f32; 2],
    /// Heel and toe (the toe stays false).
    is_touching: [bool; 2],
    was_touching: [bool; 2],
    audio_material: u32,
}

impl PedFoot {
    /// The `tFootStep` constructor 0x82516570 and the `SFXObj_PedestrianSFX` constructor
    /// 0x8252cae0, which reads the notch ranges as integer attributes.
    fn new(vault: &Vault) -> Self {
        let read = |key: u64| [0, 1].map(|index| vault.i32_at(FOOTSTEP, key, index) as f32);
        Self {
            foot_step: None,
            notch_gain: [1.0; 2],
            notch_gain_min: read(NOTCH_GAIN_MIN),
            notch_gain_max: read(NOTCH_GAIN_MAX),
            notch_freq: [96000.0; 2],
            notch_freq_min: read(NOTCH_FREQ_MIN),
            notch_freq_max: read(NOTCH_FREQ_MAX),
            is_touching: [false; 2],
            was_touching: [false; 2],
            audio_material: CONCRETE_POLISHED,
        }
    }

    /// The heel touched down this update.
    fn new_step(&self) -> bool {
        !self.was_touching[0] && self.is_touching[0]
    }

    /// `FootStepUtils::AdjustFilterParameters` 0x824e87e0: each notch frequency and gain becomes
    /// `rw::math::Random() % trunc(max - min) + min`, drawn as frequency 0, gain 0, frequency 1,
    /// gain 1. A zero range traps natively; here it gives the minimum.
    fn adjust_filter_parameters(&mut self, rng: &mut RwRandom) {
        let mut random = |min: f32, max: f32| rng.next_u32().checked_rem(fctiwz(max - min) as u32).unwrap_or(0) as f32 + min;
        for index in 0..2 {
            self.notch_freq[index] = random(self.notch_freq_min[index], self.notch_freq_max[index]);
            self.notch_gain[index] = random(self.notch_gain_min[index], self.notch_gain_max[index]);
        }
    }
}

pub(super) struct PedestrianSfx {
    /// `m_pDMixIO` (objectID 0x40050010 + the state's instance).
    pub(super) io: Option<DMixIoId>,
    /// `mRightFoot`, `mLeftFoot`.
    feet: [PedFoot; 2],
    played_whoosh: bool,
    aems_speed: i32,
    /// `mpRightFootCloth`, `mpLeftFootCloth`.
    cloth: [Option<SpliceId>; 2],
}

impl PedestrianSfx {
    /// The constructor 0x8252cae0.
    pub(super) fn new(vault: &Vault, io: Option<DMixIoId>) -> Self {
        Self { io, feet: [PedFoot::new(vault), PedFoot::new(vault)], played_whoosh: false, aems_speed: 0, cloth: [None; 2] }
    }

    /// `InitSFX` (`SFXObjBase` 0x82515d80): the mixer output on.
    pub(super) fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
    }

    /// `UpdateParams` 0x82545820: the AEMS speed of the pedestrian's speed and the heels' contact,
    /// then while it plays SFX the footsteps, cloth and whoosh. The feet, the AEMS instances and
    /// the whoosh carry over to the state's next pedestrian.
    pub(super) fn update_params(&mut self, t: f32, state: &PedAudioState, physics: &PedestrianPhysics, mics: &MicrophoneMan, whooshes: Option<&mut WhooshManager>, ctx: &mut SfxCtx<'_>) {
        self.aems_speed = speed_graph(ctx.vault, physics.speed);
        self.feet[RIGHT].is_touching = [state.is_right_foot_touching, false];
        self.feet[LEFT].is_touching = [state.is_left_foot_touching, false];
        if state.play_sfx {
            self.check_for_foot_steps(state, ctx);
            self.check_for_cloth(t, physics, ctx);
            self.check_for_whooshes(physics, mics, whooshes, ctx.vault);
        }
        for foot in &mut self.feet {
            foot.was_touching = foot.is_touching;
        }
    }

    /// `CheckForFootSteps` 0x8252d4f0: the feet's surfaces (concrete without one), the missing
    /// `livingword_footstep` instances, left first, and each new step's random notches, right
    /// first.
    fn check_for_foot_steps(&mut self, state: &PedAudioState, ctx: &mut SfxCtx<'_>) {
        let new = [self.feet[RIGHT].new_step(), self.feet[LEFT].new_step()];
        let surface = |material: u32| if material == MATERIAL_MAX { CONCRETE_ROUGH } else { material };
        self.feet[RIGHT].audio_material = surface(state.right_foot_surface);
        self.feet[LEFT].audio_material = surface(state.left_foot_surface);
        if !ENABLE_FOOT_STEPS {
            return;
        }
        for (side, speed) in [(LEFT, 0), (RIGHT, 1000)] {
            if self.feet[side].foot_step.is_none() {
                let q = [0, 1].map(|index| ctx.vault.i32_at(FOOTSTEP, NOTCH_Q, index));
                self.feet[side].foot_step = Some(new_foot_step(speed, self.aems_speed, q, ctx));
            }
        }
        for side in [RIGHT, LEFT] {
            if new[side] {
                self.feet[side].adjust_filter_parameters(ctx.rng);
            }
        }
    }

    /// `CheckForCloth` 0x8252d978: a new step replaces that foot's cloth splice (the walk, run or
    /// sprint cloth of the pedestrian's speed) and plays it on the master mix, right foot first.
    fn check_for_cloth(&mut self, t: f32, physics: &PedestrianPhysics, ctx: &mut SfxCtx<'_>) {
        let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta: t, spread: 0.0, time_stretch: 1.0 };
        let new = [self.feet[RIGHT].new_step(), self.feet[LEFT].new_step()];
        for side in [RIGHT, LEFT] {
            if !new[side] {
                continue;
            }
            ctx.release(&mut self.cloth[side]);
            let index = foot_step_cloth_splice(ctx.vault, physics.speed);
            let master = ctx.master_sub_mix;
            self.cloth[side] = ctx.splicer.create_splice(ctx.core, SPLICE_CLOTH, index, master);
            if let Some(id) = self.cloth[side] {
                ctx.splicer.play_splice(ctx.core, id, SplicePlayParams::default(), &update);
            }
        }
    }

    /// `CheckForWhooshes` 0x8252e6f8: within the `aud_whooshby_pedestrian` distance of the camera
    /// microphone and at least its speed relative to it, `PlayWhooshSound` at the pedestrian; a
    /// played whoosh waits until the pedestrian is that far again.
    fn check_for_whooshes(&mut self, physics: &PedestrianPhysics, mics: &MicrophoneMan, whooshes: Option<&mut WhooshManager>, vault: &Vault) {
        let camera = mics.microphones[0];
        let relative_speed = vmx::length(vmx::sub(camera.velocity, physics.velocity));
        let distance = vmx::length(vmx::sub(camera.cur_position, physics.position));
        let near = vault.layout_f32(WHOOSHBY_PEDESTRIAN, 0);
        if self.played_whoosh {
            self.played_whoosh = distance < near;
            return;
        }
        if distance > near || relative_speed < vault.f32_at(WHOOSHBY_PEDESTRIAN, WHOOSH_SPEEDS, 2) {
            return;
        }
        if whooshes.is_some_and(|whooshes| whooshes.play(relative_speed, WHOOSHBY_PEDESTRIAN_KEY, physics.position)) {
            self.played_whoosh = true;
        }
    }

    /// `ProcessUpdate` 0x82545978: while attached the footsteps (when it plays SFX), then the
    /// cloth.
    pub(super) fn process_update(&mut self, attached: bool, state: &PedAudioState, physics: &PedestrianPhysics, ctx: &mut SfxCtx<'_>) {
        if !attached {
            return;
        }
        if state.play_sfx {
            self.process_foot_steps(state, physics, ctx);
        }
        self.update_cloth(state, ctx);
    }

    /// `ProcessFootSteps` 0x8252dd20: both feet's `livingword_footstep` members from the mixer (dry
    /// volume output 2 for a security guard, else 1; wet 9; low and high pass 7 and 8; azimuth 0;
    /// pitch 5), the state (a side step while avoiding, a stumble after a collision) and the
    /// footstep volumes, right foot first.
    fn process_foot_steps(&mut self, state: &PedAudioState, physics: &PedestrianPhysics, ctx: &mut SfxCtx<'_>) {
        let stumble = matches!(state.state, COLLISION_KNOCKDOWN | COLLISION_STANDING);
        let side_step = matches!(state.state, AVOID_JUMP | AVOID_SKATER);
        let io = self.io;
        let dry = ctx.output(io, if state.char_type == SECURITY_GUARD { 2 } else { 1 }, PresetType::Volume);
        let wet = ctx.output(io, 9, PresetType::Volume);
        let low_pass = ctx.output(io, 7, PresetType::Frequency);
        let high_pass = ctx.output(io, 8, PresetType::Frequency);
        let azimuth = ctx.output(io, 0, PresetType::Azimuth);
        let pitch = ctx.output(io, 5, PresetType::Pitch);
        let volumes = [0, 1, 2].map(|index| ctx.vault.i32_at(FOOTSTEP, VOLUMES, index));
        for side in [RIGHT, LEFT] {
            let foot = &mut self.feet[side];
            let Some(object) = foot.foot_step.as_mut() else { continue };
            let data = &mut object.data;
            data[VOLUME] = 0x7FFF;
            data[AZIMUTH] = azimuth.clamp(0, 0xFFFF);
            data[PITCH] = pitch.clamp(0, 0x2000);
            data[LOW_PASS] = low_pass.clamp(0, 25001);
            data[HIGH_PASS] = high_pass.clamp(0, 25001);
            data[WET_FX] = wet.clamp(0, 0x7FFF);
            data[DRY_FX] = dry.clamp(0, 0x7FFF);
            data[FOOT_HEEL] = i32::from(foot.is_touching[0]);
            data[FOOT_TOE] = i32::from(foot.is_touching[1]);
            data[SIDE_STEP] = i32::from(side_step);
            data[TURN_AROUND] = 0;
            data[STUMBLE] = i32::from(stumble);
            data[PLAYER_SPEED] = self.aems_speed.clamp(0, 1000);
            data[SHOE_TYPE] = state.shoe_type.clamp(1, 5);
            data[FOOT_SURFACE] = (foot_step_material(ctx.vault, foot.audio_material) as i32).clamp(1, 7);
            data[TERRAIN] = physics.terrain.clamp(1, 5);
            for (index, volume) in volumes.into_iter().enumerate() {
                data[STEP_VOL + index] = volume.clamp(0, 0x7FFF);
            }
            data[LOCAL_SKATER] = 0;
            data[GAIN_0] = fctiwz(foot.notch_gain[0]).clamp(0, 0x7FFF);
            data[FREQ_0] = fctiwz(foot.notch_freq[0]).clamp(0, 24000);
            data[GAIN_1] = fctiwz(foot.notch_gain[1]).clamp(0, 0x7FFF);
            data[FREQ_1] = fctiwz(foot.notch_freq[1]).clamp(0, 24000);
            ctx.aems.set_member_data(object);
        }
    }

    /// `UpdateCloth` 0x8252db58: volume output 4 for a security guard, else 3; pitch output 5,
    /// azimuth output 0, the state's delta time. Both splices share the parameters, which
    /// `Splice::Update` scales in place; a finished splice is deleted.
    fn update_cloth(&mut self, state: &PedAudioState, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        let volume = if state.char_type == SECURITY_GUARD { 4 } else { 3 };
        let mut update = SpliceUpdateParams {
            vol: ctx.output(io, volume, PresetType::Volume) as f32 * VOLUME_SCALE,
            pitch: ctx.output(io, 5, PresetType::Pitch) as f32 * PITCH_SCALE,
            az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
            delta: ctx.delta_time,
            spread: 0.0,
            time_stretch: 1.0,
        };
        for side in [RIGHT, LEFT] {
            let Some(id) = self.cloth[side] else { continue };
            if ctx.is_playing(id) {
                ctx.splicer.update_splice(ctx.core, id, &mut update);
            } else {
                ctx.release(&mut self.cloth[side]);
            }
        }
    }

    /// `Destroy` 0x8252d418: both feet's AEMS instances (right first), then the cloth splices
    /// (left first).
    pub(super) fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for side in [RIGHT, LEFT] {
            if let Some(object) = self.feet[side].foot_step.take() {
                ctx.aems.release_object(object);
            }
        }
        ctx.release(&mut self.cloth[LEFT]);
        ctx.release(&mut self.cloth[RIGHT]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::AttribSource;

    /// Cloth speeds 3 and 1; notch frequencies 100..=600 and gains 10..=30.
    struct Footsteps;

    impl AttribSource for Footsteps {
        fn attribute(&self, _class: u64, _collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
            let value = match (key, index) {
                (CLOTH_SPEEDS, 0) => 3f32.to_bits(),
                (CLOTH_SPEEDS, 1) => 1f32.to_bits(),
                (NOTCH_FREQ_MIN, _) => 100,
                (NOTCH_FREQ_MAX, _) => 600,
                (NOTCH_GAIN_MIN, _) => 10,
                (NOTCH_GAIN_MAX, _) => 30,
                _ => return None,
            };
            Some(value.to_be_bytes().to_vec())
        }

        fn layout_word(&self, _class: u64, _collection: u64, _index: u32) -> u32 {
            0
        }

        fn has_collection(&self, _class: u64, _collection: u64) -> bool {
            true
        }
    }

    #[test]
    fn cloth_follows_the_speed_limits() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        assert_eq!(foot_step_cloth_splice(&vault, 3.5), CLOTH_SPRINT);
        assert_eq!(foot_step_cloth_splice(&vault, 3.0), CLOTH_RUN);
        assert_eq!(foot_step_cloth_splice(&vault, 1.0), CLOTH_WALK);
        assert_eq!(foot_step_cloth_splice(&vault, f32::NAN), CLOTH_WALK);
    }

    #[test]
    fn notches_draw_frequency_then_gain_per_filter() {
        let vault = Vault::new(Some(Box::new(Footsteps)));
        let mut foot = PedFoot::new(&vault);
        let mut rng = RwRandom::new(3);
        let mut expected = rng.clone();
        let draws: [u32; 4] = std::array::from_fn(|_| expected.next_u32());
        foot.adjust_filter_parameters(&mut rng);
        assert_eq!(foot.notch_freq, [(draws[0] % 500) as f32 + 100.0, (draws[2] % 500) as f32 + 100.0]);
        assert_eq!(foot.notch_gain, [(draws[1] % 20) as f32 + 10.0, (draws[3] % 20) as f32 + 10.0]);
    }
}
