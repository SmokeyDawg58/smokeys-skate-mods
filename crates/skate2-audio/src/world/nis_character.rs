//! The NISCHARACTER state (`eMAINMAPSTATES` 12): `CSTATEMGR_NISCharacter`, `CSTATE_NISCharacter`,
//! `SFXCTL_NISCharacterPhysics` (0x000c0000), `SFXCTL_3DNISCharacterPos` (0x000c0010) and
//! `SFXObj_NISCharacter` (0x000c0000).
//!
//! The state makes no sound itself: `SFXObj_NISCharacter` only keeps a `VoiceInfo` (mix, pan, pitch
//! and filters from its mixer outputs) that `Sk8SpeechSystem` reads through
//! `GetNISCharacterVoiceInfoFromState` 0x82550ce8 to place the cutscene characters' speech.
//! `CSTATEMGR_NISCharacter::UpdateParams` 0x82558930 attaches the states when
//! `AudioSystem::mCurrentSceneInfo.sceneType` changes to `SCENE_TYPE_RACE_START`, from the
//! `ScenePlayer` entities and the speech system's character keys; neither the scene manager's
//! audio scene info nor the speech system is ported, so no state attaches.

use crate::nicotine::{DMixIoId, MixMap, PresetType};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::system::WorldAudioState;
use crate::vault::Vault;
use crate::vmx::{Vec3, ZERO};

/// `eMAINMAPSTATES` 12.
const STATE: u32 = 12;

/// The `Sk8::Audio::VoiceInfo` fields `SFXObj_NISCharacter` fills.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct VoiceInfo {
    pub speaker_id: u32,
    pub character: i32,
    pub vol: i32,
    pub azimuth: i32,
    pub pitch: i32,
    pub high_pass: i32,
    pub low_pass: i32,
    pub wet_vol: i32,
    pub is_attached: bool,
}

/// One `CSTATE_NISCharacter` (0x78 bytes) with its controllers and object.
struct NisCharacterState {
    attached: bool,
    /// `NISCharacterState::volume_factor`.
    volume_factor: f32,
    /// `SFXCTL_NISCharacterPhysics`: the scene entity's position and the velocity since the
    /// previous one.
    position: Vec3,
    velocity: Vec3,
    position_3d: Obj3d,
    object_io: Option<DMixIoId>,
    voice_info: VoiceInfo,
}

/// `CSTATEMGR_NISCharacter` (0x34 bytes).
pub struct NisCharacterManager {
    states: Vec<NisCharacterState>,
}

impl NisCharacterManager {
    /// Five states, each with `SFXObj_NISCharacter` and its two controllers.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..5u32)
            .map(|instance| {
                let id = (STATE << 16) | (instance << 11);
                let mut position_3d = Obj3d::new(vault);
                position_3d.io = map.and_then(|map| map.dmixio(0x6000_0000 | id | 0x10));
                NisCharacterState {
                    attached: false,
                    volume_factor: 1.0,
                    position: ZERO,
                    velocity: ZERO,
                    position_3d,
                    object_io: map.and_then(|map| map.dmixio(0x4000_0000 | id)),
                    voice_info: VoiceInfo::default(),
                }
            })
            .collect();
        Self { states }
    }

    /// `GetNISCharacterVoiceInfoFromState` 0x82550ce8: the voice of an attached state.
    #[must_use]
    pub fn voice_info(&self, index: usize) -> Option<VoiceInfo> {
        self.states.get(index).filter(|state| state.attached).map(|state| state.voice_info)
    }

    /// `CSTATEMGR_Base::UpdateParams` over the states: `SFXCTL_NISCharacterPhysics::UpdateParams`
    /// 0x8251b650 (the `ScenePlayer` entity location, unported, so the position holds) and the
    /// position controller on its position and velocity.
    pub fn update_params(&mut self, t: f32, _world: &WorldAudioState, mics: &MicrophoneMan, shared: &mut Pos3dShared, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            let refs = ObjRefs { dir: None, pos: Some(state.position), vel: Some(state.velocity) };
            state.position_3d.update_params(t, refs, mics, shared, ctx.map);
        }
    }

    /// `SFXObj_NISCharacter::ProcessUpdate` 0x8253b550 / `UpdateNISCharacterVoiceInfo` 0x82518548.
    pub fn process_update(&mut self, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            let io = state.object_io;
            let voice = &mut state.voice_info;
            voice.azimuth = ctx.output(io, 0, PresetType::Azimuth);
            voice.pitch = ctx.output(io, 1, PresetType::Pitch);
            voice.vol = crate::nicotine::shape::fctiwz(ctx.output(io, 2, PresetType::Volume) as f32 * state.volume_factor);
            voice.wet_vol = crate::nicotine::shape::fctiwz(ctx.output(io, 5, PresetType::Volume) as f32 * state.volume_factor);
            voice.high_pass = ctx.output(io, 3, PresetType::Frequency);
            voice.low_pass = ctx.output(io, 4, PresetType::Frequency);
        }
    }
}
