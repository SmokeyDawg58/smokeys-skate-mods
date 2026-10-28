//! The PEDESTRIAN state (`eMAINMAPSTATES` 5): `CSTATEMGR_Pedestrian` and its fifteen
//! `CSTATE_Pedestrian`s (`EnterGamePlay` 0x825500c8 creates them with `Setup(3)`), each with the
//! `SFXCTL_PedestrianPhysics` and `SFXCTL_3DPedestrianPos` controllers (0 and 1),
//! `SFXObj_PedestrianSpeech` (object 0, [`crate::speech::pedestrian`]) and `SFXObj_PedestrianSFX`
//! (object 1, `pedestrian_sfx.rs`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

#[path = "pedestrian_sfx.rs"]
mod sfx;

use crate::nicotine::{DMixIoId, MixMap};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::presentation::{audio_material, MATERIAL_MAX};
use crate::random::RwRandom;
use crate::speech::pedestrian::{PedestrianSpeech, PedestrianSpeechInput};
use crate::speech::{PedestrianSpeechState, Sk8SpeechSystem, SpeechCtx};
use crate::system::WorldAudioState;
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};
use crate::world::whoosh::WhooshManager;
use sfx::PedestrianSfx;

/// `CSTATEMGR_Pedestrian::EnterGamePlay`'s states, also the most pedestrians an update reads.
const STATES: usize = 15;
/// `eMAINMAPSTATES` PEDESTRIAN.
const STATE_PEDESTRIAN: u32 = 5;
/// `eSk8Character_Max` and `ePedestrianAudioState_Max`.
const CHARACTER_MAX: u32 = 100;
const AUDIO_STATE_MAX: u32 = 72;
/// `kSPCH1_CharType_securityguard`: a guard takes a random guard character (75..=79) on attach.
const SECURITY_GUARD: u32 = 0x40;
/// `eChallengeTypes_Point2PointRace`, `_DeathRace` and `_SpotRace`, which silence pedestrians.
const RACES: [i32; 3] = [0x12, 0x13, 0x14];
/// `Audio_Pedestrian` `kNumPedestrians` at its registered default: every pedestrian reported.
const NUM_PEDESTRIANS: i32 = -1;
/// `eAudioSurfaceType_AsphaltSmooth`, the surface `DoSimSubmit` gives both feet.
const ASPHALT_SMOOTH: u32 = 1;
/// `eFootStepTerrain` `eFootStep_Flat`, `_StairUp` and `_InclineUp` (down is one more).
const TERRAIN_FLAT: i32 = 1;
const TERRAIN_STAIR_UP: i32 = 2;
const TERRAIN_INCLINE_UP: i32 = 4;
/// The `aud_pedestrian` class (`Attrib::Gen::aud_pedestrian` 0x828b0a50), its `type` and
/// `shoetype`.
pub(crate) const AUD_PEDESTRIAN: u64 = 0x2AAA_AB33_6154_DF15;
const CHAR_TYPE: u64 = 0x4929_64E7_1634_DA6D;
const SHOE_TYPE: u64 = 0x871B_DC66_9F2B_1844;
/// The `aud_pedestrian` attributes only `SFXObj_PedestrianSpeech` reads: the voice volume
/// factor, the distance below which a pedestrian is near, its class as a greet target, and its
/// main-cast, cameraman and announcer ids.
const VOLUME_FACTOR: u64 = 0x2087_A329_0483_BB4F;
const NEAR_FAR_DISTANCE: u64 = 0xA272_15A9_0913_5B62;
const LW_CLASS: u64 = 0x02D9_BFDA_94A8_B35E;
pub(crate) const MC_PRO_ID: u64 = 0x161E_7AD7_43C8_3C3C;
pub(crate) const MC_IP_ID: u64 = 0x14FD_437D_1906_77C8;
pub(crate) const MC_PRO_CLASS: u64 = 0x6F29_33E9_77CF_40DD;
pub(crate) const MC_IP_CLASS: u64 = 0xD6EA_428C_2B43_E23A;
pub(crate) const MC_LW_CLASS: u64 = 0xAF7F_088C_DBEA_A429;
const CM_LW_TYPE: u64 = 0x0CE4_A571_7E9D_B02C;
const CM_LW_GENDER: u64 = 0x68BB_61E5_0884_1729;
const CM_PRO_ID: u64 = 0x8777_95E7_633B_441E;
const CM_IP_ID: u64 = 0xBDF5_EAF6_AC88_0F6A;
pub(crate) const AN_CHAR_ID: u64 = 0x6F9C_8A27_E4CD_37DC;
/// `aud_speech` "default": `GlobalAttributeCollection::aud_speech_default`, and the collection
/// `Sk8SpeechSystem`'s constructor 0x82508df0 gives `mSpeechAttrib`. `MAX_SFX_PEDESTRIANS` is how
/// many of the reported pedestrians play SFX; `CHARACTERS` holds a 0x38-byte entry per character
/// whose `aud_pedestrian` collection key is at byte 40.
const SPEECH_DEFAULT: Collection = Collection::new(class::AUD_SPEECH, 0xD7ED_BD36_2D7D_2152);
const MAX_SFX_PEDESTRIANS: u64 = 0x5336_4DFA_09A4_99DD;
/// The distance within which `playSpeech` is set.
const PLAY_SPEECH_DISTANCE: u64 = 0x7F0F_FDF7_A905_77C6;
const CHARACTERS: u64 = 0x60CA_5F2E_959E_C361;
/// `GlobalAttributeCollection::aud_general_footstep` and the height between the feet's last
/// touch-downs (in centimetres) above which a pedestrian is on stairs or an incline.
const FOOTSTEP: Collection = Collection::new(class::AUD_GENERAL, 0x1ABD_2984_D724_8589);
const TERRAIN_THRESHOLD: u64 = 0xFEA5_0103_D963_7FAA;

/// `Sk8::PedStateOut` (24 bytes) as `Pedestrian::GetStateOut` returns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PedStateOut {
    pub state: u32,
    pub length: u32,
    pub topic: u32,
    pub target_greet_character: u32,
    /// The byte at 0x14: `mbRightFootDown` 0x10, `mbLeftFootDown` 0x20, `mbWasPlayerCollision`
    /// 0x40, `mbIsStopped` 0x80.
    pub flags: u8,
}

/// `AudioPresentation::PedestrianData` (16 bytes), one per pedestrian `DoSimSubmit` reports, and
/// the pres pedestrian matrix `SFXCTL_PedestrianPhysics` reads with it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PedestrianData {
    /// `ePedestrianAudioState` (7 bits).
    pub state: u32,
    /// `eSk8Characters` (7 bits).
    pub character: u32,
    pub was_player_collision: bool,
    pub is_right_foot_in_contact: bool,
    /// `eAudioMaterialTypes` (7 bits each).
    pub right_foot_surface: u32,
    pub left_foot_surface: u32,
    pub is_left_foot_in_contact: bool,
    pub is_stopped: bool,
    /// The greet target's `eSk8Characters` (7 bits), the conversation topic (5) and length (2).
    pub target_character: u32,
    pub topic: u32,
    pub length: u32,
    pub distance: f32,
    pub pedestrian_id: u32,
    /// `cLivingWorldPresEntityManager::GetPresPedestrian(pedestrianID)->GetLocalToWorldMatrix()`
    /// rows 2 (facing) and 3 (position); `None` without a pres pedestrian.
    pub local_to_world: Option<[Vec3; 2]>,
}

impl PedestrianData {
    /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's record of a pedestrian
    /// `GetPedestriansWithinRange` found with an initialized core: the state out truncated to its
    /// bit fields, `PedStateOut::GetAudioCharacter` 0x82e78490, both feet on
    /// `AudioSystem::GetAudioMaterial(eAudioSurfaceType_AsphaltSmooth)`.
    #[must_use]
    pub fn from_state_out(pedestrian_id: u32, out: PedStateOut, audio_character: u32, distance: f32, local_to_world: Option<[Vec3; 2]>) -> Self {
        let surface = audio_material(ASPHALT_SMOOTH) & 0x7F;
        Self {
            state: out.state & 0x7F,
            character: audio_character & 0x7F,
            was_player_collision: out.flags & 0x40 != 0,
            is_right_foot_in_contact: out.flags & 0x10 != 0,
            right_foot_surface: surface,
            left_foot_surface: surface,
            is_left_foot_in_contact: out.flags & 0x20 != 0,
            is_stopped: out.flags & 0x80 != 0,
            target_character: out.target_greet_character & 0x7F,
            topic: out.topic & 0x1F,
            length: out.length & 3,
            distance,
            pedestrian_id,
            local_to_world,
        }
    }
}

/// `Sk8SpeechSystem::GetSpeechCharacteristicsKey` 0x824ec7a8 as the `aud_pedestrian` collection
/// `UpdateParams` changes `mPedestrianAttrib` to (reads of a missing one are `DefaultDataArea`'s).
fn characteristics(vault: &Vault, character: u32) -> Collection {
    let entry = vault.bytes(SPEECH_DEFAULT, CHARACTERS, character, 0x38);
    Collection::new(AUD_PEDESTRIAN, u64::from_be_bytes(entry[40..48].try_into().expect("eight bytes")))
}

/// `PedAudioState` (104 bytes), at `PedAudioStateInit` 0x82550000's values by default.
/// `playSpeech` is written but never read natively.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PedAudioState {
    pedestrian_id: u32,
    grunt_index: i32,
    play_sfx: bool,
    play_speech: bool,
    /// `attatch`: the row's pedestrian has no state yet.
    attach: bool,
    was_player_collision: bool,
    is_left_foot_touching: bool,
    is_right_foot_touching: bool,
    character: u32,
    char_spch_id: u32,
    lw_class: u32,
    /// `type` (`SPCH1Type_CharType`).
    char_type: u32,
    mc_pro_id: u32,
    mc_ip_id: u32,
    mc_pro_class: u32,
    mc_ip_class: u32,
    mc_lw_class: u32,
    topic: u32,
    cm_lw_type: u32,
    cm_lw_gender: u32,
    cm_pro_id: u32,
    cm_ip_id: u32,
    an_char_id: u32,
    shoe_type: i32,
    state: u32,
    right_foot_surface: u32,
    left_foot_surface: u32,
    distance: f32,
    volume_factor: f32,
    near_far_distance: f32,
}

impl Default for PedAudioState {
    fn default() -> Self {
        Self {
            pedestrian_id: 0,
            grunt_index: 0,
            play_sfx: false,
            play_speech: false,
            attach: false,
            was_player_collision: false,
            is_left_foot_touching: false,
            is_right_foot_touching: false,
            character: CHARACTER_MAX,
            char_spch_id: 0,
            lw_class: 0,
            char_type: 0,
            mc_pro_id: 0,
            mc_ip_id: 0,
            mc_pro_class: 0,
            mc_ip_class: 0,
            mc_lw_class: 0,
            topic: 0,
            cm_lw_type: 0,
            cm_lw_gender: 0,
            cm_pro_id: 0,
            cm_ip_id: 0,
            an_char_id: 0,
            shoe_type: 1,
            state: AUDIO_STATE_MAX,
            right_foot_surface: MATERIAL_MAX,
            left_foot_surface: MATERIAL_MAX,
            distance: 0.0,
            volume_factor: 1.0,
            near_far_distance: 0.0,
        }
    }
}

impl PedAudioState {
    /// `UpdateParams`' row for the pedestrian at `index`: one with an id and a character below
    /// `eSk8Character_Max`, playing SFX while `index` is below the `aud_speech` count.
    /// The topic (`1 << (topic - 1)` below 18), the character's characteristics, the distances,
    /// and the greet target's class (a target below `eSk8Character_Max`) beside the SFX fields.
    fn row(data: &PedestrianData, index: usize, max_sfx: i32, play_speech_distance: f32, vault: &Vault) -> Option<Self> {
        if data.pedestrian_id == 0 || data.character >= CHARACTER_MAX {
            return None;
        }
        let mut row = Self {
            pedestrian_id: data.pedestrian_id,
            attach: true,
            state: data.state,
            topic: if (1..0x12).contains(&data.topic) { 1 << (data.topic - 1) } else { 0 },
            character: data.character,
            ..Self::default()
        };
        row.read_characteristics(vault);
        row.was_player_collision = data.was_player_collision;
        row.is_left_foot_touching = data.is_left_foot_in_contact;
        row.is_right_foot_touching = data.is_right_foot_in_contact;
        row.left_foot_surface = data.left_foot_surface;
        row.right_foot_surface = data.right_foot_surface;
        row.play_sfx = (index as i32) < max_sfx;
        row.play_speech = data.distance < play_speech_distance;
        row.distance = data.distance;
        row.near_far_distance = vault.f32(characteristics(vault, data.character), NEAR_FAR_DISTANCE);
        if data.target_character < CHARACTER_MAX {
            row.lw_class = vault.i32(characteristics(vault, data.target_character), LW_CLASS) as u32;
        }
        Some(row)
    }

    /// The fields read from the character's `aud_pedestrian` collection, as the row and the
    /// security-guard re-pick read them.
    fn read_characteristics(&mut self, vault: &Vault) {
        let collection = characteristics(vault, self.character);
        let read = |key: u64| vault.i32(collection, key) as u32;
        self.shoe_type = vault.i32(collection, SHOE_TYPE);
        self.char_type = read(CHAR_TYPE);
        self.char_spch_id = vault.layout_word(collection, 2);
        self.volume_factor = vault.f32(collection, VOLUME_FACTOR);
        self.grunt_index = vault.layout_word(collection, 0) as i32;
        self.mc_pro_id = read(MC_PRO_ID);
        self.mc_ip_id = read(MC_IP_ID);
        self.mc_pro_class = read(MC_PRO_CLASS);
        self.mc_ip_class = read(MC_IP_CLASS);
        self.mc_lw_class = read(MC_LW_CLASS);
        self.cm_lw_type = read(CM_LW_TYPE);
        self.cm_lw_gender = read(CM_LW_GENDER);
        self.cm_pro_id = read(CM_PRO_ID);
        self.cm_ip_id = read(CM_IP_ID);
        self.an_char_id = read(AN_CHAR_ID);
    }

    /// A security guard about to attach becomes guard character `75 + rw::math::Random() % 5`
    /// and takes that character's characteristics.
    fn pick_security_guard(&mut self, vault: &Vault, rng: &mut RwRandom) {
        self.character = rng.next_u32() % 5 + 75;
        self.read_characteristics(vault);
    }

    /// What the speech object reads.
    fn speech_input(&self) -> PedestrianSpeechInput {
        PedestrianSpeechInput {
            pedestrian_id: self.pedestrian_id,
            character: self.character,
            state: self.state,
            was_player_collision: self.was_player_collision,
            char_type: self.char_type,
            char_spch_id: self.char_spch_id,
            lw_class: self.lw_class,
            topic: self.topic,
            mc_pro_id: self.mc_pro_id,
            mc_ip_id: self.mc_ip_id,
            mc_pro_class: self.mc_pro_class,
            mc_ip_class: self.mc_ip_class,
            mc_lw_class: self.mc_lw_class,
            cm_lw_type: self.cm_lw_type,
            cm_lw_gender: self.cm_lw_gender,
            cm_pro_id: self.cm_pro_id,
            cm_ip_id: self.cm_ip_id,
            an_char_id: self.an_char_id,
            distance: self.distance,
            volume_factor: self.volume_factor,
            near_far_distance: self.near_far_distance,
        }
    }
}

/// `SFXCTL_PedestrianPhysics` (160 bytes, `CreateObject` 0x8251a1c0): the pres pedestrian's
/// position, facing, velocity and speed, and the terrain of its steps.
struct PedestrianPhysics {
    io: Option<DMixIoId>,
    position: Vec3,
    prev_position: Vec3,
    forward: Vec3,
    velocity: Vec3,
    prev_velocity: Vec3,
    speed: f32,
    was_right_foot_touching: bool,
    was_left_foot_touching: bool,
    last_right_touch_position: f32,
    last_left_touch_position: f32,
    /// `eFootStepTerrain`.
    terrain: i32,
}

impl PedestrianPhysics {
    /// The constructor 0x82511020.
    fn new(io: Option<DMixIoId>) -> Self {
        Self {
            io,
            position: ZERO,
            prev_position: ZERO,
            forward: ZERO,
            velocity: ZERO,
            prev_velocity: ZERO,
            speed: 0.0,
            was_right_foot_touching: false,
            was_left_foot_touching: false,
            last_right_touch_position: 0.0,
            last_left_touch_position: 0.0,
            terrain: TERRAIN_FLAT,
        }
    }

    /// `InitSFX` 0x82511108: the mixer output on and the vectors cleared; the position controller
    /// (`AttachController` 0x825136c0) refers to the position, facing and velocity. The speed,
    /// the feet and the terrain keep the previous pedestrian's values.
    fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.prev_velocity = ZERO;
        self.velocity = ZERO;
        self.forward = ZERO;
        self.prev_position = ZERO;
        self.position = ZERO;
    }

    /// `UpdateParams` 0x8251a2d8, without a pres pedestrian nothing: the matrix's position and
    /// normalised facing, the velocity over `t` (zero for no time), the speed as the mean of this
    /// and the last velocity's lengths. A foot touching down keeps the height it touched at; when
    /// the right foot's last touch-down is more than the threshold above or below the left's, the
    /// touching foot (the right first) reports stairs or an incline, up when it is the higher one,
    /// the kind drawn from `rw::math::Random() & 1` every update.
    fn update_params(&mut self, t: f32, state: &PedAudioState, local_to_world: Option<[Vec3; 2]>, vault: &Vault, rng: &mut RwRandom) {
        let Some([at, translation]) = local_to_world else { return };
        self.prev_position = self.position;
        self.forward = vmx::normalize_unguarded(at);
        self.position = translation;
        self.prev_velocity = self.velocity;
        self.velocity = if t > 0.0 { vmx::scale(vmx::sub(self.position, self.prev_position), vmx::recip(t)) } else { ZERO };
        self.speed = (vmx::length(self.velocity) + vmx::length(self.prev_velocity)) * 0.5;
        let (left, right) = (state.is_left_foot_touching, state.is_right_foot_touching);
        if left && !self.was_left_foot_touching {
            self.last_left_touch_position = self.position[1];
        }
        if right && !self.was_right_foot_touching {
            self.last_right_touch_position = self.position[1];
        }
        self.terrain = TERRAIN_FLAT;
        let rise = (self.last_right_touch_position - self.last_left_touch_position) * 100.0;
        let up = if rng.next_u32() & 1 != 0 { TERRAIN_STAIR_UP } else { TERRAIN_INCLINE_UP };
        if rise.abs() > vault.f32(FOOTSTEP, TERRAIN_THRESHOLD) && (right || left) {
            let down = if right { !(rise > 0.0) } else { rise > 0.0 };
            self.terrain = up + i32::from(down);
        }
        self.was_left_foot_touching = left;
        self.was_right_foot_touching = right;
    }
}

/// The objectID of a PEDESTRIAN-state object or controller.
fn object_id(instance: u32, sfx: u32) -> u32 {
    (STATE_PEDESTRIAN << 16) | (instance << 11) | (sfx << 4)
}

/// One `CSTATE_Pedestrian` (176 bytes, `CreateState` 0x825563c0) with its controllers,
/// `SFXObj_PedestrianSpeech` and `SFXObj_PedestrianSFX`.
struct PedestrianState {
    attached: bool,
    /// `mState`.
    data: PedAudioState,
    /// `t_DeltaTime`.
    delta_time: f32,
    physics: PedestrianPhysics,
    /// `SFXCTL_3DPedestrianPos` (`CreateObject` 0x8253e330), an `SFXCTL_3DObjPos`.
    position: Obj3d,
    speech: PedestrianSpeech,
    sfx: PedestrianSfx,
}

impl PedestrianState {
    fn new(instance: u32, vault: &Vault, map: Option<&MixMap>) -> Self {
        let io = |kind: u32, sfx: u32| map.and_then(|map| map.dmixio(kind | object_id(instance, sfx)));
        let mut position = Obj3d::new(vault);
        position.io = io(0x6000_0000, 1);
        Self {
            attached: false,
            data: PedAudioState::default(),
            delta_time: 0.0,
            physics: PedestrianPhysics::new(io(0x6000_0000, 0)),
            position,
            speech: PedestrianSpeech::new(instance as usize, io(0x4000_0000, 0)),
            sfx: PedestrianSfx::new(vault, io(0x4000_0000, 1)),
        }
    }

    /// `CSTATE_Pedestrian::Attach` 0x82551728 (`mState` is the row), then `CSTATE_Base::Attach`
    /// 0x82a7e2d0: the controllers' `InitSFX`, then `LoadData`, whose `SetupLoadData` is each
    /// object's `InitSFX`.
    fn attach(&mut self, data: PedAudioState, speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        self.data = data;
        self.attached = true;
        self.physics.init_sfx(ctx.map);
        if let Some(io) = self.position.io {
            ctx.map.set_output_enabled(io, true);
        }
        if let Some(speech) = speech {
            self.speech.init_sfx(&self.data.speech_input(), speech, ctx.map);
        }
        self.sfx.init_sfx(ctx.map);
    }

    /// What `UpdateParams` copies from the row of an attached state's pedestrian; the row then
    /// attaches nothing.
    fn refresh(&mut self, row: &mut PedAudioState) {
        row.attach = false;
        self.data.state = row.state;
        self.data.lw_class = row.lw_class;
        self.data.topic = row.topic;
        self.data.distance = row.distance;
        self.data.is_left_foot_touching = row.is_left_foot_touching;
        self.data.is_right_foot_touching = row.is_right_foot_touching;
        self.data.left_foot_surface = row.left_foot_surface;
        self.data.right_foot_surface = row.right_foot_surface;
        self.data.was_player_collision = row.was_player_collision;
        self.data.play_sfx = row.play_sfx;
        self.data.play_speech = row.play_speech;
    }

    /// `CSTATE_Base::Detach` 0x82a7e350: the objects' `Detach` (`SFXObj_PedestrianSFX`'s calls
    /// `Destroy`) with their mixer outputs off and inputs cleared, then the controllers' (only the
    /// position controller's does anything) with their inputs cleared.
    fn detach(&mut self, speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        self.attached = false;
        if let Some(speech) = speech {
            self.speech.detach(speech, ctx.splicer, ctx.core);
        }
        if let Some(io) = self.speech.io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        self.sfx.destroy(ctx);
        if let Some(io) = self.sfx.io {
            ctx.map.set_output_enabled(io, false);
            ctx.map.clear_input(io);
        }
        self.position.detach();
        for io in [self.physics.io, self.position.io].into_iter().flatten() {
            ctx.map.clear_input(io);
        }
    }

    /// `CSTATE_Base::UpdateParams` 0x82a7e0e0: while attached the controllers, then the object.
    #[allow(clippy::too_many_arguments)]
    fn update_params(&mut self, t: f32, world: &WorldAudioState, mics: &MicrophoneMan, shared: &mut Pos3dShared, whooshes: Option<&mut WhooshManager>, speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        self.delta_time = t;
        if !self.attached {
            return;
        }
        ctx.delta_time = t;
        let id = self.data.pedestrian_id;
        let local_to_world = world.pedestrians.iter().find(|data| data.pedestrian_id == id).and_then(|data| data.local_to_world);
        self.physics.update_params(t, &self.data, local_to_world, ctx.vault, ctx.rng);
        let refs = ObjRefs { dir: Some(self.physics.forward), pos: Some(self.physics.position), vel: Some(self.physics.velocity) };
        self.position.update_params(t, refs, mics, shared, ctx.map);
        if let Some(speech) = speech {
            let input = self.data.speech_input();
            let reverb_sub_mix = ctx.global_effects.reverb_sub_mix();
            let mut speech_ctx = SpeechCtx { core: &mut *ctx.core, vault: ctx.vault, rng: &mut *ctx.rng, master_sub_mix: ctx.master_sub_mix, reverb_sub_mix };
            self.speech.update_params(t, &input, speech, &mut *ctx.splicer, &mut speech_ctx);
        }
        self.sfx.update_params(t, &self.data, &self.physics, mics, whooshes, ctx);
    }

    /// `CSTATE_Base::ProcessUpdate` 0x82a7e090.
    fn process_update(&mut self, speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        ctx.delta_time = self.delta_time;
        if let Some(speech) = speech {
            let input = self.data.speech_input();
            let reverb_sub_mix = ctx.global_effects.reverb_sub_mix();
            let mut speech_ctx = SpeechCtx { core: &mut *ctx.core, vault: ctx.vault, rng: &mut *ctx.rng, master_sub_mix: ctx.master_sub_mix, reverb_sub_mix };
            self.speech.process_update(self.attached, self.delta_time, &input, speech, &mut *ctx.splicer, &*ctx.map, &mut speech_ctx);
        }
        self.sfx.process_update(self.attached, &self.data, &self.physics, ctx);
    }
}

/// `CSTATEMGR_Pedestrian` (48 bytes).
pub struct PedestrianManager {
    states: Vec<PedestrianState>,
    /// `mNumPedestriansAround`.
    num_pedestrians_around: i32,
    /// The `SFXCTL_3DObjPos` statics: `Generate3DParams` writes every value it reads in the same
    /// call (`PlayerMixMode` stays 0), so this copy behaves as the shared one.
    shared: Pos3dShared,
}

impl PedestrianManager {
    /// `CSTATEMGR_Pedestrian::EnterGamePlay` 0x825500c8: fifteen states with objects 0 and 1.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        Self {
            states: (0..STATES as u32).map(|instance| PedestrianState::new(instance, vault, map)).collect(),
            num_pedestrians_around: 0,
            shared: Pos3dShared::default(),
        }
    }

    /// `CSTATEMGR_Pedestrian::UpdateParams` 0x825530d0: a row for each of the first fifteen
    /// pedestrians (none during races), an attached state whose pedestrian has a row takes its
    /// state, feet and whether it plays SFX and the others detach, each remaining row attaches the
    /// first free state; then `CSTATEMGR_Base::UpdateParams` 0x82a7dbc0 updates every state.
    /// `whooshes` is the WHOOSH manager `AudioSystem::PlayWhooshSound` attaches to; the speech
    /// system's view of the states follows the update.
    pub fn update_params(&mut self, t: f32, world: &WorldAudioState, mics: &MicrophoneMan, mut whooshes: Option<&mut WhooshManager>, mut speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        let around = if NUM_PEDESTRIANS <= -1 { i32::try_from(world.pedestrians.len()).unwrap_or(i32::MAX) } else { NUM_PEDESTRIANS };
        self.num_pedestrians_around = if RACES.contains(&ctx.challenge_type) { 0 } else { around };
        let count = self.num_pedestrians_around.clamp(0, STATES as i32) as usize;
        let max_sfx = ctx.vault.i32(SPEECH_DEFAULT, MAX_SFX_PEDESTRIANS);
        let play_speech_distance = ctx.vault.f32(SPEECH_DEFAULT, PLAY_SPEECH_DISTANCE);
        let mut rows = [PedAudioState::default(); STATES];
        for (index, row) in rows.iter_mut().enumerate().take(count) {
            if let Some(filled) = world.pedestrians.get(index).and_then(|data| PedAudioState::row(data, index, max_sfx, play_speech_distance, ctx.vault)) {
                *row = filled;
            }
        }
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            match rows[..count].iter().position(|row| row.pedestrian_id == state.data.pedestrian_id) {
                Some(index) => state.refresh(&mut rows[index]),
                None => state.detach(speech.as_deref_mut(), ctx),
            }
        }
        if self.num_pedestrians_around > 0 {
            for row in &mut rows[..count] {
                if !row.attach {
                    continue;
                }
                let Some(state) = self.states.iter_mut().find(|state| !state.attached) else { continue };
                if row.char_type == SECURITY_GUARD {
                    row.pick_security_guard(ctx.vault, ctx.rng);
                }
                state.attach(*row, speech.as_deref_mut(), ctx);
            }
        }
        for state in &mut self.states {
            state.update_params(t, world, mics, &mut self.shared, whooshes.as_deref_mut(), speech.as_deref_mut(), ctx);
        }
        if let Some(speech) = speech {
            for (index, state) in self.states.iter().enumerate() {
                let data = &state.data;
                let view = PedestrianSpeechState {
                    attached: state.attached,
                    pedestrian_id: data.pedestrian_id,
                    character: data.character,
                    char_id: data.char_spch_id,
                    lw_class: data.lw_class,
                    char_type: data.char_type,
                    mc_pro_id: data.mc_pro_id,
                    mc_ip_id: data.mc_ip_id,
                    mc_pro_class: data.mc_pro_class,
                    mc_ip_class: data.mc_ip_class,
                    mc_lw_class: data.mc_lw_class,
                    distance: data.distance,
                    near_far_distance: data.near_far_distance,
                };
                speech.set_pedestrian_state(index, view);
            }
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate` 0x82a7dc40.
    pub fn process_update(&mut self, mut speech: Option<&mut Sk8SpeechSystem>, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            state.process_update(speech.as_deref_mut(), ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::AttribSource;

    const GUARD_KEY: u64 = 0x4000;

    /// Character `c` has characteristics collection `0x1000 + c` (guards' are `GUARD_KEY`) whose
    /// `type` is `c` (0x40 for guards) and whose `shoetype` is `c + 1`; three pedestrians play
    /// SFX and the terrain threshold is 10 cm.
    struct Characters;

    impl AttribSource for Characters {
        fn attribute(&self, class_key: u64, collection: u64, key: u64, index: u32) -> Option<Vec<u8>> {
            let word = |value: u32| Some(value.to_be_bytes().to_vec());
            match (class_key, key) {
                (class::AUD_SPEECH, MAX_SFX_PEDESTRIANS) => word(3),
                (class::AUD_SPEECH, CHARACTERS) => {
                    let key = if (75..80).contains(&index) { GUARD_KEY + u64::from(index) } else { 0x1000 + u64::from(index) };
                    let mut entry = vec![0; 0x38];
                    entry[40..48].copy_from_slice(&key.to_be_bytes());
                    Some(entry)
                }
                (AUD_PEDESTRIAN, CHAR_TYPE) if collection >= GUARD_KEY => word(1000 + (collection - GUARD_KEY) as u32),
                (AUD_PEDESTRIAN, CHAR_TYPE) => word((collection as u32).wrapping_sub(0x1000)),
                (AUD_PEDESTRIAN, SHOE_TYPE) => word((collection as u32 & 0xFFF) + 1),
                (AUD_PEDESTRIAN, LW_CLASS) => word((collection as u32 & 0xFFF) << 4),
                (AUD_PEDESTRIAN, VOLUME_FACTOR) => word(0.5f32.to_bits()),
                (AUD_PEDESTRIAN, NEAR_FAR_DISTANCE) => word(6f32.to_bits()),
                (class::AUD_GENERAL, TERRAIN_THRESHOLD) => word(10f32.to_bits()),
                _ => None,
            }
        }

        fn layout_word(&self, _class: u64, _collection: u64, _index: u32) -> u32 {
            0
        }

        fn has_collection(&self, _class: u64, _collection: u64) -> bool {
            true
        }
    }

    fn pedestrian(id: u32, character: u32) -> PedestrianData {
        PedestrianData::from_state_out(id, PedStateOut { state: 3, ..PedStateOut::default() }, character, 4.0, None)
    }

    #[test]
    fn the_state_out_packs_into_bit_fields() {
        let out = PedStateOut { state: 0x83, length: 7, topic: 0x32, target_greet_character: 0xE4, flags: 0xA0 };
        let data = PedestrianData::from_state_out(9, out, 0x81, 2.5, None);
        assert_eq!((data.state, data.character, data.length, data.topic, data.target_character), (3, 1, 3, 0x12, 0x64));
        assert!(data.is_left_foot_in_contact && data.is_stopped);
        assert!(!data.is_right_foot_in_contact && !data.was_player_collision);
        assert_eq!((data.right_foot_surface, data.left_foot_surface), (0, 0));
        assert_eq!((data.pedestrian_id, data.distance), (9, 2.5));
    }

    #[test]
    fn rows_take_the_characteristics_of_valid_pedestrians() {
        let vault = Vault::new(Some(Box::new(Characters)));
        assert_eq!(PedAudioState::row(&pedestrian(0, 5), 0, 3, 5.0, &vault), None);
        assert_eq!(PedAudioState::row(&pedestrian(7, CHARACTER_MAX), 0, 3, 5.0, &vault), None);
        let row = PedAudioState::row(&pedestrian(7, 12), 2, 3, 5.0, &vault).expect("valid pedestrian");
        assert_eq!((row.pedestrian_id, row.char_type, row.shoe_type, row.state), (7, 12, 13, 3));
        assert!(row.attach && row.play_sfx);
        assert!(!PedAudioState::row(&pedestrian(7, 12), 3, 3, 5.0, &vault).expect("valid pedestrian").play_sfx);
    }

    #[test]
    fn rows_take_the_speech_fields() {
        let vault = Vault::new(Some(Box::new(Characters)));
        let out = PedStateOut { state: 3, topic: 3, target_greet_character: 9, ..PedStateOut::default() };
        let row = PedAudioState::row(&PedestrianData::from_state_out(7, out, 12, 4.0, None), 0, 3, 5.0, &vault).expect("valid pedestrian");
        assert_eq!((row.character, row.topic, row.lw_class), (12, 4, 9 << 4));
        assert_eq!((row.distance, row.volume_factor, row.near_far_distance), (4.0, 0.5, 6.0));
        assert!(row.play_speech);
        assert!(!PedAudioState::row(&pedestrian(7, 12), 0, 3, 4.0, &vault).expect("valid pedestrian").play_speech);
        let out = PedStateOut { state: 3, topic: 18, target_greet_character: 100, ..PedStateOut::default() };
        let row = PedAudioState::row(&PedestrianData::from_state_out(7, out, 12, 4.0, None), 0, 3, 5.0, &vault).expect("valid pedestrian");
        assert_eq!((row.topic, row.lw_class), (0, 0));
    }

    #[test]
    fn security_guards_draw_a_guard_character() {
        let vault = Vault::new(Some(Box::new(Characters)));
        let mut rng = RwRandom::new(77);
        let character = rng.clone().next_u32() % 5 + 75;
        let mut row = PedAudioState::row(&pedestrian(7, 12), 0, 3, 5.0, &vault).expect("valid pedestrian");
        row.pick_security_guard(&vault, &mut rng);
        assert_eq!((row.character, row.char_type, row.shoe_type), (character, 1000 + character, character as i32 + 1));
    }

    #[test]
    fn physics_follows_the_matrix_and_the_steps() {
        let vault = Vault::new(Some(Box::new(Characters)));
        let mut rng = RwRandom::new(5);
        let mut physics = PedestrianPhysics::new(None);
        let mut state = PedAudioState { is_left_foot_touching: true, ..PedAudioState::default() };
        let matrix = |y: f32| Some([[0.0, 0.0, 2.0, 0.0], [1.0, y, 0.0, 1.0]]);
        physics.update_params(0.5, &state, matrix(0.0), &vault, &mut rng);
        assert_eq!(physics.forward, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!((physics.velocity[0], physics.speed, physics.terrain), (2.0, 1.0, TERRAIN_FLAT));
        state = PedAudioState { is_right_foot_touching: true, ..PedAudioState::default() };
        let up = if rng.clone().next_u32() & 1 != 0 { TERRAIN_STAIR_UP } else { TERRAIN_INCLINE_UP };
        physics.update_params(0.5, &state, matrix(0.25), &vault, &mut rng);
        assert_eq!((physics.speed, physics.terrain), (1.25, up));
        physics.update_params(0.5, &state, None, &vault, &mut rng);
        assert_eq!(physics.speed, 1.25);
        state = PedAudioState { is_left_foot_touching: true, ..PedAudioState::default() };
        let up = if rng.clone().next_u32() & 1 != 0 { TERRAIN_STAIR_UP } else { TERRAIN_INCLINE_UP };
        physics.update_params(0.0, &state, matrix(-0.5), &vault, &mut rng);
        assert_eq!((physics.velocity, physics.terrain), (ZERO, up + 1));
    }
}
