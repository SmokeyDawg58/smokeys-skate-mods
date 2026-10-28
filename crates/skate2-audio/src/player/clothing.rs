//! `Sk8::Audio::SFXObj_Clothing` (68 bytes, PLAYER object 6, TypeInfo 0x00010060, `CreateObject`
//! 0x8252f800): the wipeout cloth falls (`c_cloth_falls`, `Foley_Cloth.abk`), the body slides
//! (`c_body_slide`, `Bodyslide.abk`) and the push leg foley (`sk82_cloth` splices 32 and 33).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::PlayerPhysics;
use super::{SfxCtx, AZIMUTH_SCALE, PITCH_SCALE, VOLUME_SCALE};
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::{audio_material, MATERIAL_MAX};
use crate::splicer::{SpliceId, SplicePlayParams, SpliceUpdateParams};
use crate::surface;
use crate::vault::{class, Collection};

/// `GlobalAttributeCollection`'s `aud_general` clothing (+0x58) and `aud_collisions` default
/// (+0x28) collections.
const CLOTHING: Collection = Collection::new(class::AUD_GENERAL, 0x3982_76A6_3764_95EE);
const COLLISIONS: Collection = Collection::new(class::AUD_COLLISIONS, 0xD7ED_BD36_2D7D_2152);

/// The clothing switches `HandleLegFoley` and `HandleFalls` read, and `Max_Tumble_Speed`.
const PLAY_LEG_FOLEY: u64 = 0x91E6_9D3A_C71F_1D7F;
const PLAY_FALLS: u64 = 0xE1EA_210F_3CB3_F8B8;
const MAX_TUMBLE_SPEED: u64 = 0x0A9A_9BD1_150F_E838;
/// The body slide switch, the speed and tumble that read as 1000, `Slide_TimeIncrement`, the
/// speed that ends a slide and `Slide_MinSpeedForSlideStart`.
const PLAY_BODY_SLIDES: u64 = 0x76BE_385B_AB31_82B7;
const SLIDE_MAX_SPEED: u64 = 0x7871_71EC_D02D_BBC3;
const SLIDE_MAX_TUMBLE_SPEED: u64 = 0x5D2F_244E_82CF_7255;
const SLIDE_TIME_INCREMENT: u64 = 0x4435_464D_195C_3526;
const SLIDE_STOP_SPEED: u64 = 0x746E_A8EF_187E_1571;
const SLIDE_MIN_SPEED_FOR_SLIDE_START: u64 = 0xB021_DB33_8B89_D0F2;

/// `SPLICE_TYPE` 23 (`sk82_cloth`): the push up and push down leg foley.
const CLOTH_SPLICES: i32 = 23;
const PUSH_UP: i32 = 32;
const PUSH_DOWN: i32 = 33;

/// `eBodySlideSurface_RoughConcrete` and `eBodySlideSurface_Face`.
const SLIDE_ROUGH_CONCRETE: i32 = 2;
const SLIDE_FACE: i32 = 4;
/// `SurfaceMapping::GetBodySlideMaterial` 0x8251c030: word 10 of the mapping record.
const BODY_SLIDE_MATERIAL: usize = 10;

/// The constructor 0x82516c10 clears every member.
#[derive(Default)]
pub struct Clothing {
    /// `m_pDMixIO` (objectID 0x40010060 + the state's instance).
    pub io: Option<DMixIoId>,
    was_wiping_out: bool,
    cloth_falls: Option<ClassObject>,
    max_cloth_fall_speed: i32,
    push_up: Option<SpliceId>,
    push_down: Option<SpliceId>,
    was_generally_pushing: bool,
    /// `mTheBodySlide`: the instance and its duration.
    body_slide: Option<ClassObject>,
    body_slide_duration: i32,
}

impl Clothing {
    /// The `InitSFX` (0x82515d80, shared) `SetupLoadData` runs on attach: the mixer outputs on.
    pub fn init_sfx(&mut self, _physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
    }

    /// `Destroy` 0x8252f8b8, which `Detach` runs.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        if let Some(falls) = self.cloth_falls.take() {
            ctx.aems.release_object(falls);
        }
        ctx.release(&mut self.push_up);
        ctx.release(&mut self.push_down);
        if let Some(slide) = self.body_slide.take() {
            ctx.aems.release_object(slide);
            self.body_slide_duration = 0;
        }
    }

    /// `UpdateParams` 0x8254e0f0.
    pub fn update_params(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.handle_falls(physics, ctx);
        self.handle_leg_foley(t, physics, ctx);
        self.check_for_body_slides(physics, ctx);
    }

    /// `ProcessUpdate` 0x82530670.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        self.update_body_slides(physics, ctx);
        self.update_falls(ctx);
        self.update_leg_foley(ctx);
    }

    /// `HandleFalls` 0x8252fc20: a falls instance from the start of a wipeout until the wipeout is
    /// over, at the torso spin in thousandths of `Max_Tumble_Speed`; the larger of the torso and
    /// limb speeds is what `UpdateFalls` sends.
    fn handle_falls(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CLOTHING, PLAY_FALLS) {
            return;
        }
        let wiping_out = physics.is_wiping_out;
        let started = !self.was_wiping_out && wiping_out;
        let scale = 1.0 / ctx.vault.f32(CLOTHING, MAX_TUMBLE_SPEED);
        let torso = fctiwz(scale * physics.torso_ang_vel * 1000.0);
        let limb = fctiwz(physics.limb_speed * scale * 1000.0);
        self.max_cloth_fall_speed = if torso > limb { torso } else { limb };
        match self.cloth_falls.take() {
            Some(falls) if physics.is_wipe_out_over || !wiping_out => ctx.aems.release_object(falls),
            Some(falls) => self.cloth_falls = Some(falls),
            None if started => {
                let data = vec![0, 0, 0, torso.clamp(0, 1000), 25000, 0, 0, 0, 0];
                self.cloth_falls = Some(ctx.aems.create_object("c_cloth_falls", data));
            }
            None => {}
        }
        self.was_wiping_out = wiping_out;
    }

    /// `HandleLegFoley` 0x8252f968: the push up foley when pushing begins, cut by the next push
    /// start or a wipeout, and the push down foley on every push start, cut by a wipeout.
    fn handle_leg_foley(&mut self, t: f32, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(CLOTHING, PLAY_LEG_FOLEY) {
            return;
        }
        let pushing = physics.is_generally_pushing;
        let push_started = physics.push_started;
        let began = !self.was_generally_pushing && pushing;
        if self.push_up.is_some() {
            if push_started || physics.is_wiping_out {
                ctx.release(&mut self.push_up);
            }
        } else if began {
            self.push_up = Self::play(t, PUSH_UP, ctx);
        }
        self.was_generally_pushing = pushing;
        if push_started {
            ctx.release(&mut self.push_down);
            self.push_down = Self::play(t, PUSH_DOWN, ctx);
        } else if self.push_down.is_some() && physics.is_wiping_out {
            ctx.release(&mut self.push_down);
        }
    }

    /// `new Splicer::Splice(23, index, AudioCore::GetMasterSubMix())` and its silent `Play`, the
    /// spread 1 for the local skater and 0 otherwise.
    fn play(t: f32, index: i32, ctx: &mut SfxCtx<'_>) -> Option<SpliceId> {
        let bank = usize::from(CLOTH_SPLICES as i8 as u8);
        let splice = ctx.splicer.create_splice(ctx.core, bank, index, ctx.master_sub_mix)?;
        let spread = if ctx.local { 1.0 } else { 0.0 };
        let update = SpliceUpdateParams { vol: 0.0, pitch: 1.0, az: 0.0, delta: t, spread, time_stretch: 1.0 };
        ctx.splicer.play_splice(ctx.core, splice, SplicePlayParams::default(), &update);
        Some(splice)
    }

    /// `CheckForBodySlides` 0x82546710: a body slide instance once a sliding contact passes
    /// `Slide_MinSpeedForSlideStart`, released at or below the stop speed or out of contact.
    fn check_for_body_slides(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if !ctx.vault.bool(COLLISIONS, PLAY_BODY_SLIDES) {
            return;
        }
        let (material, speed, in_collision) = body_slide_info(physics, ctx);
        if let Some(slide) = self.body_slide.take() {
            if speed <= ctx.vault.i32(COLLISIONS, SLIDE_STOP_SPEED) || !in_collision {
                ctx.aems.release_object(slide);
                self.body_slide_duration = 0;
            } else {
                self.body_slide = Some(slide);
            }
        } else if in_collision && speed > ctx.vault.i32(COLLISIONS, SLIDE_MIN_SPEED_FOR_SLIDE_START) {
            let ground_slide = i32::from(physics.is_wiping_out);
            let data = vec![0, 0, 0, speed.clamp(0, 1000), 25000, 0, 0, 0, material.clamp(0, 4), ground_slide, 0, 0];
            self.body_slide = Some(ctx.aems.create_object("c_body_slide", data));
        }
    }

    /// `UpdateBodySlides` 0x825300d0: the slide follows the contacts, the torso spin and the mixer,
    /// its duration growing by `Slide_TimeIncrement` each update.
    fn update_body_slides(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let Some(mut slide) = self.body_slide.take() else { return };
        let (material, speed, _) = body_slide_info(physics, ctx);
        let tumble = fctiwz(physics.torso_ang_vel / ctx.vault.f32(COLLISIONS, SLIDE_MAX_TUMBLE_SPEED) * 1000.0);
        self.body_slide_duration = self.body_slide_duration.wrapping_add(ctx.vault.i32(COLLISIONS, SLIDE_TIME_INCREMENT));
        let io = self.io;
        let data = &mut slide.data;
        data[0] = 0x7FFF;
        data[1] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        data[2] = ctx.output(io, 10, PresetType::Pitch).clamp(0, 0x2000);
        data[3] = speed.clamp(0, 1000);
        data[7] = ctx.output(io, 9, PresetType::Volume).clamp(0, 0x7FFF);
        data[8] = material.clamp(0, 4);
        data[11] = tumble.clamp(0, 1000);
        data[9] = i32::from(physics.is_wiping_out);
        data[10] = self.body_slide_duration.clamp(0, 7000);
        ctx.aems.set_member_data(&slide);
        self.body_slide = Some(slide);
    }

    /// `UpdateFalls` 0x82516c78.
    fn update_falls(&mut self, ctx: &mut SfxCtx<'_>) {
        let Some(mut falls) = self.cloth_falls.take() else { return };
        let io = self.io;
        let data = &mut falls.data;
        data[1] = ctx.output(io, 0, PresetType::Azimuth).clamp(0, 0xFFFF);
        data[2] = ctx.output(io, 5, PresetType::Pitch).clamp(0, 0x2000);
        data[3] = self.max_cloth_fall_speed.clamp(0, 1000);
        data[0] = 0x7FFF;
        data[7] = ctx.output(io, 6, PresetType::Volume).clamp(0, 0x7FFF);
        ctx.aems.set_member_data(&falls);
        self.cloth_falls = Some(falls);
    }

    /// `UpdateLegFoley` 0x825303b8: each leg foley splice follows the mixer until it ends.
    fn update_leg_foley(&mut self, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        for splice in [&mut self.push_up, &mut self.push_down] {
            let Some(id) = *splice else { continue };
            if !ctx.is_playing(id) {
                ctx.release(splice);
                continue;
            }
            let update = SpliceUpdateParams {
                vol: ctx.output(io, 8, PresetType::Volume) as f32 * VOLUME_SCALE,
                pitch: ctx.output(io, 7, PresetType::Pitch) as f32 * PITCH_SCALE,
                az: ctx.output(io, 0, PresetType::Azimuth) as f32 * AZIMUTH_SCALE,
                delta: ctx.delta_time,
                spread: if ctx.local { 1.0 } else { 0.0 },
                time_stretch: 1.0,
            };
            ctx.update_splice(id, update);
        }
    }
}

/// `GetBodySlideInfo` 0x8252fea8: whether any of the first six skeleton contacts slides, the player
/// speed in thousandths of the slide's maximum, and the body slide surface. `kSlideSurface` forces
/// the surface; otherwise a face contact slides on the face, a contact surface on its
/// `GetBodySlideMaterial` (contact 1's, else the first found), and no surface on rough concrete.
fn body_slide_info(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> (i32, i32, bool) {
    let info = &physics.skeleton_collision_info;
    let mut contact_surface = 0;
    let mut face = false;
    let mut in_collision = false;
    for index in 0..6 {
        if !(info.slide_speed[index].abs() <= 0.0) {
            in_collision = true;
            if info.is_face_contact {
                face = true;
            }
            let surface = info.audio_surface_type[index];
            if surface != 0 && (index == 1 || contact_surface == 0) {
                contact_surface = surface;
            }
        }
    }
    let speed = fctiwz(physics.player_speed / ctx.vault.f32(COLLISIONS, SLIDE_MAX_SPEED) * 1000.0);
    let material = audio_material(contact_surface);
    let surface = if ctx.cvars.slide_surface != -1 {
        ctx.cvars.slide_surface
    } else if material == MATERIAL_MAX {
        SLIDE_ROUGH_CONCRETE
    } else if face {
        SLIDE_FACE
    } else {
        surface::mapping(ctx.vault, material, BODY_SLIDE_MATERIAL) as i32
    };
    (surface, speed, in_collision)
}
