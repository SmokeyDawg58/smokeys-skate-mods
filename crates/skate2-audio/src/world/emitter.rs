//! The EMITTER state (`eMAINMAPSTATES` 6): `CSTATEMGR_Emitter` and its five `CSTATE_Emitter`s,
//! each with an `SFXObj_Emitter` (TypeInfo 0x00060000, `CreateObject` 0x825306b0) and the
//! `SFXCTL_3DEmitterPos` (`CreateObject` 0x8251a9e8, an `SFXCTL_3DObjPos`) that places it.
//! `Sk8EmitterSystem` (`super::emitter_system`) attaches them: each volume or random emitter
//! plays one `Csis::C_emitter` (`Sk8_Emitters_Project.csi`), which the `c_emitter` module of
//! the loaded `emitters.big` bank answers.

use crate::aems::{Aems, ClassObject};
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::vault::Vault;
use crate::vmx::Vec3;

/// `eVolState`.
pub const VOL_STATE_RANDOM: i32 = 0;
pub const VOL_STATE_VOLUME: i32 = 1;
/// `eAzimState`.
pub const AZIM_STATE_NICOTINE: i32 = 0;
pub const AZIM_STATE_EXTERNAL: i32 = 1;

/// `Csis::C_emitterStruct` members.
const DRY_LEVEL: usize = 1;
const WET_LEVEL: usize = 2;
const AZIMUTH: usize = 3;
const PITCH: usize = 4;
const LOW_PASS: usize = 5;

/// `Sk8::Audio::tEmitterInfo` (48 bytes), which the emitter system allocates for an attach and
/// `CSTATE_Emitter::Detach` frees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmitterInfo {
    pub aems_id: i32,
    pub vol_state: i32,
    pub vol: f32,
    pub azim_state: i32,
    pub azimuth: i32,
    pub expire: bool,
    pub timeout: f32,
    pub position: Vec3,
}

/// One `CSTATE_Emitter` with its `SFXObj_Emitter` and `SFXCTL_3DEmitterPos`.
struct EmitterState {
    /// `CSTATE_Base::bIsAttached` and `CSTATE_Emitter::mpEmitterInfo`.
    attached: bool,
    info: Option<EmitterInfo>,
    /// `CSTATE_Base::t_DeltaTime`.
    delta_time: f32,
    /// The object's DMixIO (0x40000000 | objectID).
    io: Option<DMixIoId>,
    position: Obj3d,
    /// `mpEmitter` and `mCurrentTime`.
    emitter: Option<ClassObject>,
    current_time: f32,
}

/// `CSTATEMGR_Emitter`.
pub struct EmitterManager {
    states: Vec<EmitterState>,
}

impl EmitterManager {
    /// `CSTATEMGR_Emitter::EnterGamePlay` (0x82550c60, shared): five states, `Setup(1)` (object
    /// 0, whose `GetController` 0x82512f18 names controller 0).
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..5u32)
            .map(|instance| {
                let id = (6 << 16) | (instance << 11);
                let mut position = Obj3d::new(vault);
                position.io = map.and_then(|map| map.dmixio(0x6000_0000 | id));
                EmitterState {
                    attached: false,
                    info: None,
                    delta_time: 0.0,
                    io: map.and_then(|map| map.dmixio(0x4000_0000 | id)),
                    position,
                    emitter: None,
                    current_time: 0.0,
                }
            })
            .collect();
        Self { states }
    }

    /// `CSTATEMGR_Base::GetFreeState` 0x82a7db90: the first detached state.
    #[must_use]
    pub fn free_state(&self) -> Option<usize> {
        self.states.iter().position(|state| !state.attached)
    }

    #[must_use]
    pub fn is_attached(&self, index: usize) -> bool {
        self.states.get(index).is_some_and(|state| state.attached)
    }

    /// The attached state's `mpEmitterInfo` (`None` once detached).
    pub fn info_mut(&mut self, index: usize) -> Option<&mut EmitterInfo> {
        self.states.get_mut(index)?.info.as_mut()
    }

    /// `CSTATE_Emitter::Attach` 0x825517f8 (`CSTATE_Base::Attach` 0x82a7e2d0): the controller's
    /// `InitSFX` (0x82515d80, its outputs on), then `LoadData`'s `SFXObj_Emitter::InitSFX`
    /// 0x82516e58: outputs on, the controller on `info.position`, and a `C_emitter` at full
    /// volume, pitch 4096, low pass 25000 and `emitter_ID` = `aemsID` clamped to 0..500.
    pub fn attach(&mut self, index: usize, info: EmitterInfo, aems: &mut Aems, map: &mut MixMap) {
        let Some(state) = self.states.get_mut(index) else { return };
        state.info = Some(info);
        state.attached = true;
        if let Some(io) = state.position.io {
            map.set_output_enabled(io, true);
        }
        if let Some(io) = state.io {
            map.set_output_enabled(io, true);
        }
        let data = vec![0x7FFF, 0, 0, 0, 4096, 25000, 0, 0, info.aems_id.clamp(0, 500)];
        state.emitter = Some(aems.create_object("c_emitter", data));
        state.current_time = 0.0;
    }

    /// `CSTATE_Emitter::Detach` 0x82551800 (`CSTATE_Base::Detach` 0x82a7e350): the object's
    /// `Detach` (`Destroy` 0x82530768 releases the `C_emitter`), its outputs off and inputs
    /// cleared, the controller's inputs cleared, then the info freed.
    pub fn detach(&mut self, index: usize, aems: &mut Aems, map: &mut MixMap) {
        let Some(state) = self.states.get_mut(index) else { return };
        state.attached = false;
        if let Some(emitter) = state.emitter.take() {
            aems.release_object(emitter);
        }
        if let Some(io) = state.io {
            map.set_output_enabled(io, false);
            map.clear_input(io);
        }
        if let Some(io) = state.position.io {
            map.clear_input(io);
        }
        state.info = None;
    }

    /// `CSTATEMGR_Base::UpdateParams` 0x82a7dbc0 (`CSTATE_Base::UpdateParams` 0x82a7e0e0): each
    /// attached state's position controller on its info's position (no direction or velocity);
    /// `SFXObj_Emitter` has no `UpdateParams`.
    pub fn update_params(&mut self, t: f32, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        for state in &mut self.states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { pos: state.info.map(|info| info.position), ..ObjRefs::default() };
            state.position.update_params(t, refs, mics, shared, map);
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate` 0x82a7dc40: `SFXObj_Emitter::ProcessUpdate` 0x825307b0 of
    /// every state.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        for index in 0..self.states.len() {
            self.process_emitter(index, ctx);
        }
    }

    /// `SFXObj_Emitter::ProcessUpdate` 0x825307b0: a volume reads mixer outputs 4, 8, 5, 6 and a
    /// random emitter 2, 7, 1, 3 (dry, wet, pitch, low pass), the levels scaled by the info's
    /// volume; the azimuth is the info's for an external one, else mixer output 0. An expiring
    /// emitter detaches past its timeout.
    fn process_emitter(&mut self, index: usize, ctx: &mut SfxCtx<'_>) {
        let state = &mut self.states[index];
        if !state.attached {
            return;
        }
        let Some(info) = state.info else { return };
        let io = state.io;
        let level = |output: i32| fctiwz(ctx.output(io, output, PresetType::Volume) as f32 * info.vol);
        // `AudioCore::StartSampleCapture` (+0x50) zeroes a random emitter's dry level; the game
        // never captures.
        let (dry, wet, pitch, frequency) = if info.vol_state != VOL_STATE_RANDOM {
            (level(4), level(8), ctx.output(io, 5, PresetType::Pitch), ctx.output(io, 6, PresetType::Frequency))
        } else {
            (level(2), level(7), ctx.output(io, 1, PresetType::Pitch), ctx.output(io, 3, PresetType::Frequency))
        };
        let azimuth = if info.azim_state == AZIM_STATE_EXTERNAL { info.azimuth } else { ctx.output(io, 0, PresetType::Azimuth) };
        if let Some(emitter) = state.emitter.as_mut() {
            emitter.data[DRY_LEVEL] = dry.clamp(0, 0x7FFF);
            emitter.data[WET_LEVEL] = wet.clamp(0, 0x7FFF);
            emitter.data[PITCH] = pitch.clamp(0, 0x2000);
            emitter.data[LOW_PASS] = frequency.clamp(0, 25000);
            emitter.data[AZIMUTH] = azimuth.clamp(0, 0xFFFF);
            ctx.aems.set_member_data(emitter);
        }
        if info.expire {
            state.current_time += state.delta_time;
            if state.current_time > info.timeout {
                self.detach(index, ctx.aems, ctx.map);
            }
        }
    }
}
