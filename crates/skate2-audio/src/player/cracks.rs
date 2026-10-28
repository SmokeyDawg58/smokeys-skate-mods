//! `Sk8::Audio::SFXObj_Cracks` (156 bytes, PLAYER object 4): a `Class_Seams` instance per wheel
//! (`Seams_Bank.abk`) the wheels trigger crossing the surface pattern's cracks or slats, or
//! rolling onto another material.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::physics::PlayerPhysics;
use super::SfxCtx;
use crate::aems::ClassObject;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, PresetType};
use crate::presentation::MATERIAL_MAX;
use crate::surface::{seam_type, transition_type};
use crate::vault::{class, Collection};
use crate::vmx::Vec3;

const DEFAULT: u64 = 0xD7ED_BD36_2D7D_2152;
/// `GlobalAttributeCollection`'s `aud_cracks_default` (+0x1C).
const CRACKS_DEFAULT: Collection = Collection::new(class::AUD_CRACKS, DEFAULT);

const MIN_SPEED: u64 = 0xF7BC_A67F_0A1F_C92E;
const SPEED_VOLUME: u64 = 0x5F75_F995_2EBA_482C;
const TRANSITION_VOLUME: u64 = 0x2F29_F403_8486_3C8C;
const PATTERN_OVERRIDE: u64 = 0x2314_D2C0_724A_C82F;
const CRACK_TYPE: u64 = 0xCA81_764B_F5A8_5E34;
const MIN_FRAMES_BETWEEN_LEFT_RIGHT: u64 = 0xDAE8_03A0_CBD2_86D1;
const SLAT_WIDTH: u64 = 0xA3BC_1976_039F_A00A;
const WOOD_SIDEWALK_MULT: u64 = 0x1911_1761_87FB_9B1F;

/// `aud_cracks` layout words: `RattleVol`, `GridAngle`, `Grid_Width`, `Grid_Length`, `AEMS_ID`.
const RATTLE_VOL: u32 = 0;
const GRID_ANGLE: u32 = 1;
const GRID_WIDTH: u32 = 2;
const GRID_LENGTH: u32 = 3;
const AEMS_ID: u32 = 4;

/// `CRACK_TYPE` values: none, crack lines and slats.
const CRACKS_LINES: i32 = 1;
const CRACKS_SLATS: i32 = 2;
/// The wood seam type and the sidewalk pattern, where `Wood_Sidewalk_Mult` widens the grid.
const SEAM_WOOD: u32 = 3;
const PATTERN_SIDEWALK: i32 = 0xA;

/// `SlatReset`, and the back wheels' slat distance once the front wheels hit one.
const SLAT_RESET: f32 = 999.0;
const SLAT_BACK: f32 = 50.0;

/// `AudioSystem::SurfacePatternToKey` 0x824e6858: the `aud_cracks` collections of surface
/// patterns 1..=15.
const PATTERNS: [&str; 15] = [
    "spidercrack",
    "square_2_x_2",
    "square_4_x_4",
    "square_8_x_8",
    "square_12_x_12",
    "square_24_x_24",
    "irregular_small",
    "irregular_medium",
    "irregular_large",
    "slats",
    "sidewalk",
    "brick_tile_random_size",
    "mini_tile",
    "special_1",
    "special_2",
];

fn pattern_key(pattern: u32) -> u64 {
    pattern.wrapping_sub(1).try_into().ok().and_then(|index: usize| PATTERNS.get(index)).map_or(0, |name| skate2_formats::attrib::string_hash64(name))
}

/// `GetRotatedWheelPos` 0x82521558: X and Z turned by the grid angle in degrees.
fn rotated_wheel_pos(position: Vec3, angle: i32) -> Vec3 {
    let radians = angle as f32 * f32::from_bits(0x3C8E_FA35);
    let (cos, sin) = (f64::from(radians).cos() as f32, f64::from(radians).sin() as f32);
    let (x, z) = (position[0], position[2]);
    [cos.mul_add(x, -(sin * z)), position[1], cos.mul_add(z, sin * x), position[3]]
}

pub struct Cracks {
    /// `m_pDMixIO` (objectID 0x40010040 + the state's instance).
    pub io: Option<DMixIoId>,
    /// `mCracksAttrib`'s collection.
    collection: u64,
    seams: [Option<ClassObject>; 4],
    seam_flipper: [bool; 4],
    last_crack_in_direction: [[i32; 4]; 2],
    distance_since_last_slat: [f32; 2],
    previous_wheel_material: [u32; 4],
    valid_previous_data: bool,
    current_tick: i32,
    seam_tick: [i32; 4],
    last_turn: i32,
}

impl Default for Cracks {
    /// The constructor 0x825213e0.
    fn default() -> Self {
        Self {
            io: None,
            collection: DEFAULT,
            seams: [None, None, None, None],
            seam_flipper: [false; 4],
            last_crack_in_direction: [[i32::MAX; 4]; 2],
            distance_since_last_slat: [0.0, SLAT_RESET],
            previous_wheel_material: [MATERIAL_MAX; 4],
            valid_previous_data: false,
            current_tick: 0,
            seam_tick: [0; 4],
            last_turn: 0,
        }
    }
}

impl Cracks {
    fn cracks(&self) -> Collection {
        Collection::new(class::AUD_CRACKS, self.collection)
    }

    /// `SFXCTL_PlayerPhysics::GetWheelType` 0x825199d0.
    fn wheel_type(physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> i32 {
        if ctx.local { physics.wheel_hardness as i32 } else { ctx.npc_wheel_type as i32 }
    }

    /// `GetCrackSurface` 0x82521650: the wheel material's seam type, or its transition type
    /// past the five seam types.
    fn crack_surface(wheel: usize, transition: bool, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> u32 {
        let material = physics.wheel_material[wheel];
        if material >= MATERIAL_MAX {
            0
        } else if transition {
            transition_type(ctx.vault, material) + 5
        } else {
            seam_type(ctx.vault, material)
        }
    }

    /// `InitSFX` 0x82540dd8: the mixer outputs on and a seam instance per wheel.
    pub fn init_sfx(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        if let Some(io) = self.io {
            ctx.map.set_output_enabled(io, true);
        }
        for wheel in 0..4 {
            let material = physics.wheel_material[wheel];
            let surface = if material < MATERIAL_MAX { seam_type(ctx.vault, material) } else { 0 };
            let wheel_type = Self::wheel_type(physics, ctx);
            let data = vec![0, 0x7FFF, 0, 0, 0x1000, 25000, 0, 0, 0, 0, (surface as i32).clamp(0, 8), wheel_type.clamp(0, 1), 0, 0, (wheel as i32).clamp(0, 3), 0, 0, 0, 0];
            self.seams[wheel] = Some(ctx.aems.create_object("Class_Seams", data));
            self.seam_flipper[wheel] = true;
        }
    }

    /// `Destroy` 0x82521500.
    pub fn destroy(&mut self, ctx: &mut SfxCtx<'_>) {
        for seam in &mut self.seams {
            if let Some(seam) = seam.take() {
                ctx.aems.release_object(seam);
            }
        }
    }

    /// `UpdateParams` 0x8254f3f0: the seam triggers cleared, then a material change or, above
    /// the pattern's minimum speed, the cracks along and across the board.
    pub fn update_params(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        ctx.set_input(self.io, 0, 0);
        for seam in self.seams.iter_mut().flatten() {
            seam.data[7] = 0;
        }
        if !self.check_for_surface_changes(physics, ctx) {
            let min_speed = ctx.vault.f32(self.cracks(), MIN_SPEED);
            if !(physics.board_speed <= min_speed) {
                self.process_wheels(0, physics, ctx);
                self.process_wheels(1, physics, ctx);
            }
        }
        for seam in self.seams.iter().flatten() {
            ctx.aems.set_member_data(seam);
        }
    }

    /// `CheckForSurfaceChanges` 0x825418b8: a transition seam on each wheel whose material
    /// changed since the last grounded frame.
    fn check_for_surface_changes(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) -> bool {
        let mut changed = false;
        if physics.in_air {
            self.valid_previous_data = false;
        }
        if self.valid_previous_data {
            for wheel in 0..4 {
                if self.previous_wheel_material[wheel] != physics.wheel_material[wheel] {
                    self.play_seam(wheel, false, true, physics, ctx);
                    changed = true;
                }
            }
        }
        self.previous_wheel_material = physics.wheel_material;
        self.valid_previous_data = true;
        changed
    }

    /// `ProcessWheels` 0x8254c2b8: the front then back wheel pair crossing a crack line in
    /// `direction` (along, across), or the slats passing under the board.
    fn process_wheels(&mut self, direction: usize, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let manual = physics.is_manual || physics.is_manual_braking;
        let forced = ctx.vault.i32(CRACKS_DEFAULT, PATTERN_OVERRIDE);
        let pattern = if forced != -1 {
            forced as u32
        } else if manual && !physics.landed[0] {
            physics.surface_pattern[3]
        } else {
            physics.surface_pattern[0]
        };
        if pattern == 0 {
            self.clear_patterns();
            return;
        }
        self.collection = pattern_key(pattern);
        let crack_type = ctx.vault.i32(self.cracks(), CRACK_TYPE);
        if crack_type == 0 {
            self.clear_patterns();
            return;
        }
        self.current_tick += 1;
        if self.current_tick > 0x7FFF {
            self.current_tick = 0;
            self.seam_tick = [0; 4];
        }
        if crack_type == CRACKS_LINES {
            let front = [self.in_contact_with_crack(0, direction, physics, ctx), self.in_contact_with_crack(1, direction, physics, ctx)];
            let back = if manual { [false; 2] } else { [self.in_contact_with_crack(2, direction, physics, ctx), self.in_contact_with_crack(3, direction, physics, ctx)] };
            for (pair, contact) in [(0, front), (2, back)] {
                let both = contact[0] && contact[1];
                let (wheel, other) = if contact[0] {
                    (pair, pair + 1)
                } else if contact[1] {
                    (pair + 1, pair)
                } else {
                    continue;
                };
                let min_frames = ctx.vault.i32(self.cracks(), MIN_FRAMES_BETWEEN_LEFT_RIGHT);
                if self.current_tick - self.seam_tick[other] > min_frames {
                    self.play_seam(wheel, !both, false, physics, ctx);
                    self.seam_tick[wheel] = self.current_tick;
                }
            }
        } else if crack_type == CRACKS_SLATS && !physics.in_air && direction == 0 {
            let mut game_speed = physics.game_speed;
            if !(game_speed <= 2.0) {
                game_speed = 2.0;
            }
            let step = game_speed * (physics.board_speed * 100.0) * ctx.delta_time;
            let [front, back] = self.distance_since_last_slat;
            self.distance_since_last_slat[0] = front + step;
            if !(back > SLAT_BACK) {
                self.distance_since_last_slat[1] = back - step;
            }
            let width = ctx.vault.f32(self.cracks(), SLAT_WIDTH);
            if !(self.distance_since_last_slat[0] <= width) {
                self.play_seam(0, false, false, physics, ctx);
                self.play_seam(1, false, false, physics, ctx);
                self.distance_since_last_slat = [0.0, SLAT_BACK];
            }
            if !(self.distance_since_last_slat[1] > 0.0) {
                self.play_seam(2, false, false, physics, ctx);
                self.play_seam(3, false, false, physics, ctx);
                self.distance_since_last_slat[1] = SLAT_RESET;
            }
        }
    }

    fn clear_patterns(&mut self) {
        for seam in self.seams.iter_mut().flatten() {
            seam.data[13] = 0;
        }
    }

    /// `InContactWithCrack` 0x82540fc8: whether the wheel moved into another grid cell along
    /// `direction` since the last check.
    fn in_contact_with_crack(&mut self, wheel: usize, direction: usize, physics: &PlayerPhysics, ctx: &SfxCtx<'_>) -> bool {
        if physics.in_air || physics.is_rail_grind {
            return false;
        }
        let cracks = self.cracks();
        let position = rotated_wheel_pos(physics.wheel_pos[wheel], ctx.vault.layout_word(cracks, GRID_ANGLE) as i32);
        let mut multiplier = 1.0;
        if Self::crack_surface(wheel, false, physics, ctx) == SEAM_WOOD && ctx.vault.layout_word(cracks, AEMS_ID) as i32 == PATTERN_SIDEWALK {
            multiplier = ctx.vault.f32(CRACKS_DEFAULT, WOOD_SIDEWALK_MULT);
        }
        let (along, size) = if direction == 1 {
            (position[2], ctx.vault.layout_f32(cracks, GRID_WIDTH) * multiplier)
        } else {
            (position[0], ctx.vault.layout_f32(cracks, GRID_LENGTH) * multiplier)
        };
        let cell = fctiwz((along / size).floor());
        let last = &mut self.last_crack_in_direction[direction][wheel];
        if *last == cell {
            return false;
        }
        *last = cell;
        true
    }

    /// `PlaySeam` 0x82541118: the wheel's seam triggered (alternating 1 and 2), with the
    /// crossing angle, the seam or transition surface and the wheel type, and mixer input 0 up.
    fn play_seam(&mut self, wheel: usize, angle: bool, transition: bool, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let surface = Self::crack_surface(wheel, transition, physics, ctx);
        let wheel_type = Self::wheel_type(physics, ctx);
        let flipper = self.seam_flipper[wheel];
        self.seam_flipper[wheel] = !flipper;
        let Some(seam) = self.seams[wheel].as_mut() else { return };
        seam.data[9] = i32::from(angle);
        seam.data[7] = if flipper { 1 } else { 2 };
        seam.data[10] = (surface as i32).clamp(0, 8);
        seam.data[11] = wheel_type.clamp(0, 1);
        ctx.set_input(self.io, 0, 0x7FFF);
        if let Some(elapsed) = crate::splicer::trace_elapsed() {
            eprintln!("{elapsed:9.3} cracks seam wheel {wheel} angle {angle} transition {transition} surface {surface}");
        }
    }

    /// `ProcessUpdate` 0x825412a0: each seam instance follows the mixer, the board speed, the
    /// turn (100 per wheel at most) and the pattern.
    pub fn process_update(&mut self, physics: &PlayerPhysics, ctx: &mut SfxCtx<'_>) {
        let io = self.io;
        for wheel in 0..4 {
            let Some(mut seam) = self.seams[wheel].take() else { continue };
            let mut volume = ctx.output(io, 1, PresetType::Volume);
            if Self::wheel_type(physics, ctx) == 1 {
                volume = ctx.output(io, 6, PresetType::Volume);
            }
            let azimuth = ctx.output(io, 0, PresetType::Azimuth);
            let pitch = ctx.output(io, 2, PresetType::Pitch);
            let wet = ctx.output(io, 5, PresetType::Volume);
            let low_pass = ctx.output(io, 3, PresetType::Frequency);
            let high_pass = ctx.output(io, 4, PresetType::Frequency);
            let speed = physics.board_speed;
            let ramp = (speed - 0.5) * f32::from_bits(0x3DA3_D70A);
            let ramp = if -ramp >= 0.0 { 0.0 } else { ramp };
            let ramp = if 1.0 - ramp >= 0.0 { ramp } else { 1.0 };
            let speed_param = fctiwz(ramp * 10000.0);
            if ctx.vault.bool(self.cracks(), SPEED_VOLUME) {
                volume = fctiwz(volume as f32 * speed * f32::from_bits(0x3DCC_CCCD));
                if volume > 0x7FFF {
                    volume = 0x7FFF;
                }
            }
            let mut turn = fctiwz(physics.turn * 1000.0);
            if turn > self.last_turn {
                if turn - self.last_turn > 100 {
                    turn = self.last_turn + 100;
                }
            } else if turn < self.last_turn && self.last_turn - turn > 100 {
                turn = self.last_turn - 100;
            }
            self.last_turn = turn;
            let cracks = self.cracks();
            let data = &mut seam.data;
            data[0] = 0x7FFF;
            data[1] = volume.clamp(0, 0x7FFF);
            data[2] = wet.clamp(0, 0x7FFF);
            data[5] = low_pass.clamp(0, 25000);
            data[6] = high_pass.clamp(0, 25000);
            data[3] = azimuth.clamp(0, 0x1_0000);
            data[4] = pitch.clamp(0, 0x2000);
            data[8] = speed_param.clamp(0, 10000);
            data[11] = Self::wheel_type(physics, ctx).clamp(0, 1);
            data[15] = if turn < 0 { turn.wrapping_neg() } else { turn }.clamp(0, 1000);
            data[16] = fctiwz(ctx.vault.layout_f32(cracks, RATTLE_VOL) * 32767.0).clamp(0, 0x7FFF);
            data[17] = i32::from(physics.is_manual || physics.is_manual_braking);
            data[18] = fctiwz(ctx.vault.f32(cracks, TRANSITION_VOLUME) * 32767.0).clamp(0, 0x7FFF);
            data[12] = i32::from(physics.is_pushing_left || physics.is_pushing_right);
            data[13] = (ctx.vault.layout_word(cracks, AEMS_ID) as i32).clamp(0, 15);
            ctx.aems.set_member_data(&seam);
            self.seams[wheel] = Some(seam);
        }
    }
}
