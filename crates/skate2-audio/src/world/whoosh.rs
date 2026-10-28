//! The WHOOSH state (`eMAINMAPSTATES` 10): `CSTATEMGR_Whoosh`, `CSTATE_Whoosh`,
//! `SFXCTL_3DWhooshPos` (0x000a0000) and `SFXObj_Whoosh` (0x000a0000). `AudioSystem::PlayWhooshSound`
//! attaches a state when the camera passes a dynamic object or a world instance with an
//! `aud_whooshby` key fast enough; the state plays one whoosh splice from `Sk82_Whsh_Bys.bnk`
//! (`SPLICE_TYPE` 21) and lets go when it ends.

use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::splicer::{SpliceId, SpliceUpdateParams};
use crate::system::WorldAudioState;
use crate::vault::{Collection, Vault};
use crate::vmx::{self, Vec3};

/// `eMAINMAPSTATES` 10.
const STATE: u32 = 10;
/// `SPLICE_TYPE` 21.
const SPLICE_WHOOSH: i32 = 21;
/// `aud_whooshby`: the class, its trigger distance (layout word 0), `Volume`, the three relative
/// speeds (fast to slow) and the splice index and delay of each.
pub(crate) const AUD_WHOOSHBY: u64 = 0x4F32_993D_1936_028A;
const VOLUME: u64 = 0x875B_A753_41DC_8391;
const SPEEDS: u64 = 0x79B8_671F_26F9_A0DD;
const SPLICES: u64 = 0x119A_6E4B_FE71_1569;
const DELAYS: u64 = 0x8172_0390_A8CE_11FC;

/// `tWhooshInfo` (0x30 bytes): the key, the position and the relative speed.
#[derive(Debug, Clone, Copy)]
struct WhooshInfo {
    key: u64,
    position: Vec3,
    relative_speed: f32,
}

/// `CheckForWhoosh` (`SFXObj_Dynamic` 0x82533e30, `SFXObj_ObjectInstance` 0x8253b250): within the
/// `aud_whooshby` distance of the camera microphone, moving at least its slowest speed, a whoosh
/// plays once; leaving the distance arms it again.
pub(crate) fn check_for_whoosh(played: &mut bool, key: u64, position: Vec3, mics: &MicrophoneMan, whooshes: Option<&mut WhooshManager>, vault: &Vault) {
    if key == 0 {
        return;
    }
    let whoosh = Collection::new(AUD_WHOOSHBY, key);
    if !vault.has_collection(whoosh) {
        return;
    }
    if vault.f32(whoosh, VOLUME) <= 0.0 {
        return;
    }
    let camera = &mics.microphones[0];
    let speed = vmx::length(camera.velocity);
    let distance = vmx::length(vmx::sub(camera.cur_position, position));
    let reach = vault.layout_f32(whoosh, 0);
    if *played {
        *played = distance < reach;
        return;
    }
    if distance > reach || speed < vault.f32_at(whoosh, SPEEDS, 2) {
        return;
    }
    if whooshes.is_some_and(|whooshes| whooshes.play(speed, key, position)) {
        *played = true;
    }
}

/// One `CSTATE_Whoosh` (0x48 bytes) with `SFXCTL_3DWhooshPos` and `SFXObj_Whoosh`.
struct WhooshState {
    attached: bool,
    info: Option<WhooshInfo>,
    delta_time: f32,
    position: Obj3d,
    object_io: Option<DMixIoId>,
    splice_index: i32,
    delay: f32,
    volume: f32,
    splice: Option<SpliceId>,
}

/// `CSTATEMGR_Whoosh`.
pub struct WhooshManager {
    states: Vec<WhooshState>,
    /// The attachments `PlayWhooshSound` made since the last update, whose `InitSFX` runs with the
    /// vault at the next update (`CSTATE_Base::Attach` runs it at once; nothing reads the object
    /// in between).
    pending_init: Vec<usize>,
}

impl WhooshManager {
    /// `EnterGamePlay` 0x8254fb90: three states.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..3u32)
            .map(|instance| {
                let id = (STATE << 16) | (instance << 11);
                let mut position = Obj3d::new(vault);
                position.io = map.and_then(|map| map.dmixio(0x6000_0000 | id));
                WhooshState {
                    attached: false,
                    info: None,
                    delta_time: 0.0,
                    position,
                    object_io: map.and_then(|map| map.dmixio(0x4000_0000 | id)),
                    splice_index: 0,
                    delay: 0.0,
                    volume: 0.0,
                    splice: None,
                }
            })
            .collect();
        Self { states, pending_init: Vec::new() }
    }

    /// `AudioSystem::PlayWhooshSound` 0x824f0b30: the first free state (`GetFreeState`) attaches
    /// to the whoosh (`CSTATE_Whoosh::Attach` 0x82555e10, which keeps `Simulation::mTick` as
    /// `mTick`; nothing reads it); false when all three are busy.
    pub fn play(&mut self, relative_speed: f32, key: u64, position: Vec3) -> bool {
        let Some(index) = self.states.iter().position(|state| !state.attached) else { return false };
        let state = &mut self.states[index];
        state.info = Some(WhooshInfo { key, position, relative_speed });
        state.attached = true;
        self.pending_init.push(index);
        true
    }

    /// `SFXObj_Whoosh::InitSFX` 0x82545010 (and the position controller's, which turns its mixer
    /// output on): the splice and delay of the speed band (`GetWhooshIndex` 0x82529a10), the volume.
    fn init_sfx(state: &mut WhooshState, ctx: &mut SfxCtx<'_>) {
        for io in [state.position.io, state.object_io].into_iter().flatten() {
            ctx.map.set_output_enabled(io, true);
        }
        let Some(info) = state.info else { return };
        let vault = ctx.vault;
        let whoosh = Collection::new(AUD_WHOOSHBY, info.key);
        let index = if info.relative_speed < vault.f32_at(whoosh, SPEEDS, 0) {
            if info.relative_speed < vault.f32_at(whoosh, SPEEDS, 1) { 2 } else { 1 }
        } else {
            0
        };
        state.splice_index = vault.i32_at(whoosh, SPLICES, index);
        state.delay = vault.f32_at(whoosh, DELAYS, index);
        state.volume = vault.f32(whoosh, VOLUME);
        state.splice = None;
    }

    /// `CSTATE_Whoosh::Detach` 0x82551460 / `CSTATE_Base::Detach`.
    fn detach(state: &mut WhooshState, ctx: &mut SfxCtx<'_>) {
        state.attached = false;
        ctx.release(&mut state.splice);
        if let Some(io) = state.object_io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        state.position.detach();
        if let Some(io) = state.position.io {
            ctx.map.clear_input(io);
        }
        state.info = None;
    }

    /// `CSTATEMGR_Base::UpdateParams` over the states: `SFXCTL_3DObjPos::UpdateParams` at the
    /// whoosh position, then `SFXObj_Whoosh::UpdateParams` 0x82529818, which starts the splice
    /// once the delay has run out.
    pub fn update_params(&mut self, t: f32, _world: &WorldAudioState, mics: &MicrophoneMan, shared: &mut Pos3dShared, ctx: &mut SfxCtx<'_>) {
        for index in std::mem::take(&mut self.pending_init) {
            Self::init_sfx(&mut self.states[index], ctx);
        }
        for state in &mut self.states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { pos: state.info.map(|info| info.position), ..ObjRefs::default() };
            state.position.update_params(t, refs, mics, shared, ctx.map);
            if state.delay > 0.0 {
                state.delay -= t;
            }
            if state.splice.is_none() && state.delay <= 0.0 {
                // `StartWhooshSplice` 0x82529758.
                ctx.delta_time = state.delta_time;
                state.splice = ctx.new_splice(SPLICE_WHOOSH, state.splice_index, ctx.master_sub_mix);
            }
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate`: `SFXObj_Whoosh::ProcessUpdate` 0x82529868 mixes the splice
    /// while it plays and detaches the state when it has ended.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            let Some(splice) = state.splice else { continue };
            ctx.delta_time = state.delta_time;
            if ctx.is_playing(splice) {
                let io = state.object_io;
                let update = SpliceUpdateParams {
                    vol: state.volume * (ctx.output(io, 1, PresetType::Volume) as f32 * VOLUME_SCALE),
                    pitch: ctx.output(io, 2, PresetType::Pitch) as f32 * PITCH_SCALE,
                    az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
                    delta: state.delta_time,
                    spread: 0.0,
                    time_stretch: 1.0,
                };
                ctx.update_splice(splice, update);
            } else {
                Self::detach(state, ctx);
            }
        }
    }
}
