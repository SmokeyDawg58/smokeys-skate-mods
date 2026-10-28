//! `Sk8::Audio::MicrophoneMan` and `SFXCTL_3DObjPos` (the player and board position controllers):
//! the listener positions and the per-object distance, azimuth, twist and Doppler inputs the
//! Nicotine 3D mix controls read (input words 0-3, 10, 11, 13-15).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap};
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};

/// `MicrophoneMan::Microphone`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Microphone {
    pub cur_position: Vec3,
    pub prev_position: Vec3,
    pub direction: Vec3,
    pub velocity: Vec3,
}

/// `Sk8::Audio::MicrophoneMan`: microphone 0 follows the camera, microphone 1 the local skater.
#[derive(Debug, Clone, Copy, Default)]
pub struct MicrophoneMan {
    pub microphones: [Microphone; 2],
    pub last_frame: u32,
}

/// The camera `CameraPresentation::GetMatrix(1)` returns: its Z row and translation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CameraMatrix {
    pub at: Vec3,
    pub position: Vec3,
}

impl MicrophoneMan {
    /// `MicrophoneMan::UpdateCameras` 0x824f2728. `frame` is
    /// `DataSystem::GetCurrentRestoredFrame`; `local` the local skater's `SkaterData`.
    pub fn update_cameras(&mut self, t: f32, frame: u32, camera: CameraMatrix, local: Option<(Vec3, Vec3, Vec3)>) {
        let last = self.last_frame;
        self.last_frame = frame;
        let changed = last != frame;
        let Some((position, look, velocity)) = local else { return };
        let camera_mic = &mut self.microphones[0];
        camera_mic.prev_position = camera_mic.cur_position;
        camera_mic.cur_position = camera.position;
        camera_mic.direction = vmx::normalize_unguarded(camera.at);
        if changed {
            let delta = vmx::sub(camera_mic.cur_position, camera_mic.prev_position);
            camera_mic.velocity = if t == 0.0 { ZERO } else { vmx::scale(delta, 1.0 / t) };
        }
        let skater_mic = &mut self.microphones[1];
        skater_mic.prev_position = skater_mic.cur_position;
        skater_mic.cur_position = position;
        skater_mic.direction = look;
        skater_mic.velocity = velocity;
    }
}

/// `SFXCTL_3DObjPos::Ref3D<Vector2>`: the x and z of a direction, position and velocity.
#[derive(Debug, Clone, Copy, Default)]
pub struct Ref2D {
    pub dir: [f32; 2],
    pub pos: [f32; 2],
    pub vel: [f32; 2],
}

/// The `SFXCTL_3DObjPos` statics every instance shares (`m_v2CameraAzimRef`, `m_v2PlayerAzimRef`,
/// `m_v2ObjCopy`, `m_CameraAngle`, `m_PlayerAngle`, `PlayerMixMode`, `POSMIXTYPE`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Pos3dShared {
    pub camera_azim_ref: Ref2D,
    pub player_azim_ref: Ref2D,
    pub obj_copy: Ref2D,
    pub camera_angle: i32,
    pub player_angle: i32,
    pub player_mix_mode: i32,
    pub pos_mix_type: u32,
}

/// The vectors a position controller's `Ref3D<Vector3*>` points at this update (`None` for a
/// null pointer).
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjRefs {
    pub dir: Option<Vec3>,
    pub pos: Option<Vec3>,
    pub vel: Option<Vec3>,
}

/// `aud_general` "camera".
const GENERAL_CAMERA: Collection = Collection::new(class::AUD_GENERAL, 0x55D8_01ED_E7E3_38B6);
const MINUS_ONE: i32 = 0xbf80_0000_u32 as i32;
const SMALL: f32 = f32::from_bits(0x38d1_b717);
const ANGLE_SCALE: f32 = f32::from_bits(0x477f_ff00);
const INV_TWO_PI: f32 = f32::from_bits(0x3e22_f983);

/// `SFXCTL_3DObjPos` (`SFXCTL_3DPlayerPos`, `SFXCTL_3DBoardPos`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Obj3d {
    pub io: Option<DMixIoId>,
    pub dist_to_ref: [[f32; 2]; 2],
    pub player_ref: i32,
    pub camera_pos: Vec3,
    pub player_pos: Vec3,
    pub dvelmag_car: [f32; 2],
    pub dvelmag_cam: [f32; 2],
    pub camera_offset: f32,
}

/// The x and z of a vector (`VectorPermuteConstant<0,6,0,0>`).
fn xz(v: Vec3) -> [f32; 2] {
    [v[0], v[2]]
}

/// `Vector2` length with the zero guard.
fn length2(v: [f32; 2]) -> f32 {
    let x = v[0] * v[0];
    let z = v[1] * v[1];
    vmx::length_from_squared(x + z)
}

impl Obj3d {
    /// `SFXCTL_3DObjPos::SFXCTL_3DObjPos` 0x825189f8.
    #[must_use]
    pub fn new(vault: &Vault) -> Self {
        Self { camera_offset: vault.f32(GENERAL_CAMERA, 0xA6F8_53B9_35E4_6E5F), ..Self::default() }
    }

    fn set(map: &mut MixMap, io: Option<DMixIoId>, index: usize, value: i32) {
        if let Some(io) = io {
            map.set_input(io, index, value);
        }
    }

    fn get(map: &MixMap, io: Option<DMixIoId>, index: usize) -> i32 {
        io.map_or(0, |io| map.input(io, index))
    }

    /// `SFXCTL_3DObjPos::UpdateParams` 0x8253d6c8 (`UpdateDoppler` takes the time but never reads
    /// it).
    pub fn update_params(&mut self, _t: f32, refs: ObjRefs, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        let io = self.io;
        if refs.pos.is_none() {
            Self::set(map, io, 3, 0);
            Self::set(map, io, 1, MINUS_ONE);
            Self::set(map, io, 2, 0);
            Self::set(map, io, 0, MINUS_ONE);
            let flags = Self::get(map, io, 15);
            Self::set(map, io, 15, flags & !1);
            return;
        }
        let flags = Self::get(map, io, 15);
        Self::set(map, io, 15, flags | 1);
        Self::set(map, io, 11, self.player_ref);
        self.generate_3d_params(0, refs, mics, shared, map);
        self.update_doppler(refs, mics, map);
    }

    /// `SFXCTL_3DObjPos::Generate3DParams` 0x8254a1c8.
    fn generate_3d_params(&mut self, player: i32, refs: ObjRefs, mics: &MicrophoneMan, shared: &mut Pos3dShared, map: &mut MixMap) {
        if refs.pos.is_none() {
            return;
        }
        self.player_ref = player;
        if shared.player_mix_mode != 0 {
            return;
        }
        shared.pos_mix_type = 0;
        self.player_ref = 0;
        let camera = mics.microphones[0];
        let dir = xz(camera.direction);
        shared.camera_azim_ref.dir = dir;
        let inverse = vmx::rsqrt(dir[0] * dir[0] + dir[1] * dir[1]);
        let pos = xz(camera.cur_position);
        shared.camera_azim_ref.pos = [pos[0] - dir[0] * inverse * self.camera_offset, pos[1] - dir[1] * inverse * self.camera_offset];
        let skater = mics.microphones[1];
        shared.player_azim_ref.pos = xz(skater.cur_position);
        shared.player_azim_ref.dir = xz(skater.direction);
        self.camera_pos = camera.cur_position;
        self.player_pos = skater.cur_position;
        self.generate_single_player_mix(refs, shared, map);
    }

    /// `SFXCTL_3DObjPos::GenerateSinglePlayerMix` 0x8253d240.
    fn generate_single_player_mix(&mut self, refs: ObjRefs, shared: &mut Pos3dShared, map: &mut MixMap) {
        let io = self.io;
        let Some(pos) = refs.pos else {
            Self::set(map, io, 3, 0);
            Self::set(map, io, 1, MINUS_ONE);
            Self::set(map, io, 2, 0);
            Self::set(map, io, 0, MINUS_ONE);
            return;
        };
        shared.obj_copy.pos = xz(pos);
        if let Some(dir) = refs.dir {
            shared.obj_copy.dir = xz(dir);
        }
        if shared.pos_mix_type < 1 {
            shared.camera_angle = calculate_angle(shared.camera_azim_ref, shared.obj_copy);
            Self::set(map, io, 3, shared.camera_angle);
            let camera_distance = vmx::length(vmx::sub(self.camera_pos, pos));
            Self::set(map, io, 1, camera_distance.to_bits() as i32);
            shared.player_angle = calculate_angle(shared.player_azim_ref, shared.obj_copy);
            Self::set(map, io, 2, shared.player_angle);
            let player_distance = vmx::length(vmx::sub(self.player_pos, pos));
            Self::set(map, io, 0, player_distance.to_bits() as i32);
        }
        if refs.dir.is_some() {
            let angle = calculate_angle(shared.camera_azim_ref, shared.obj_copy);
            let twist = calculate_twist(shared.camera_azim_ref, shared.obj_copy, angle);
            Self::set(map, io, 10, twist);
        } else {
            Self::set(map, io, 5, 0);
            Self::set(map, io, 6, 0);
            Self::set(map, io, 10, 0);
        }
    }

    /// `SFXCTL_3DObjPos::UpdateDoppler` 0x82518ad8.
    fn update_doppler(&mut self, refs: ObjRefs, mics: &MicrophoneMan, map: &mut MixMap) {
        let Some(velocity) = refs.vel else { return };
        let pos = refs.pos.unwrap_or(ZERO);
        let [camera, skater] = mics.microphones;
        let relative_skater = vmx::sub(velocity, skater.velocity);
        self.dist_to_ref[0][1] = self.dist_to_ref[0][0];
        let relative_camera = vmx::sub(velocity, camera.velocity);
        self.dist_to_ref[1][1] = self.dist_to_ref[1][0];
        self.dist_to_ref[0][0] = vmx::length(vmx::sub(skater.cur_position, pos));
        self.dvelmag_car[1] = self.dvelmag_car[0];
        self.dvelmag_cam[1] = self.dvelmag_cam[0];
        let camera_speed = vmx::length(relative_camera);
        let skater_speed = vmx::length(relative_skater);
        let camera_distance = vmx::length(vmx::sub(camera.cur_position, pos));
        self.dvelmag_car[0] = skater_speed;
        self.dvelmag_cam[0] = camera_speed;
        self.dist_to_ref[1][0] = camera_distance;
        if self.dist_to_ref[0][1] > self.dist_to_ref[0][0] {
            self.dvelmag_car[0] = skater_speed * -1.0;
        }
        if self.dist_to_ref[1][1] > camera_distance {
            self.dvelmag_cam[0] = camera_speed * -1.0;
        }
        let io = self.io;
        if crossed(self.dvelmag_car) {
            let flags = Self::get(map, io, 15);
            Self::set(map, io, 15, (flags as u32 | 0x8000_0000) as i32);
        }
        if crossed(self.dvelmag_cam) {
            let flags = Self::get(map, io, 15);
            Self::set(map, io, 15, (flags as u32 | 0x4000_0000) as i32);
        }
        Self::set(map, io, 13, self.dvelmag_car[0].to_bits() as i32);
        Self::set(map, io, 14, self.dvelmag_cam[0].to_bits() as i32);
    }

    /// `SFXCTL_3DObjPos::Detach` 0x8250f370 clears the position and direction pointers; the owner
    /// stops passing them.
    pub fn detach(&mut self) {}
}

/// The sign change test of `UpdateDoppler` (`bge`/`ble` against 0, NaN takes neither side).
fn crossed(values: [f32; 2]) -> bool {
    let [now, previous] = values;
    if previous < 0.0 {
        return now > 0.0;
    }
    previous > 0.0 && now < 0.0
}

/// Two `Vector2` lengths `CalculateAngle`/`CalculateTwist` compare against 1e-4, then normalise
/// with the refined `vrefp` reciprocals.
fn unit_pair(delta: [f32; 2], dir: [f32; 2]) -> Option<([f32; 2], [f32; 2])> {
    let delta_length = length2(delta);
    let dir_length = length2(dir);
    if !(delta_length >= SMALL) || !(dir_length >= SMALL) {
        return None;
    }
    let delta_inverse = vmx::recip(delta_length);
    let dir_inverse = vmx::recip(dir_length);
    Some(([delta_inverse * delta[0], delta_inverse * delta[1]], [dir_inverse * dir[0], dir_inverse * dir[1]]))
}

/// `XMVectorACos` of the clamped dot product, as a 0..0xFFFF fraction of a turn.
fn turn_fraction(a: [f32; 2], b: [f32; 2]) -> i32 {
    let x = a[0] * b[0];
    let z = a[1] * b[1];
    let dot = vmx::vmin(1.0, vmx::vmax(-1.0, x + z));
    fctiwz(INV_TWO_PI * (vmx::xm_acos(dot) * ANGLE_SCALE))
}

/// `SFXCTL_3DObjPos::CalculateAngle` 0x8253cc98: the object's bearing from `reference`.
fn calculate_angle(reference: Ref2D, obj: Ref2D) -> i32 {
    let delta = [obj.pos[0] - reference.pos[0], obj.pos[1] - reference.pos[1]];
    let Some((delta, dir)) = unit_pair(delta, reference.dir) else { return 0 };
    let angle = turn_fraction(delta, dir);
    let cross = delta[0] * dir[1] - delta[1] * dir[0];
    if cross > 0.0 { 0xFFFF - angle } else { angle }
}

/// `SFXCTL_3DObjPos::CalculateTwist` 0x8253cf58: the object's facing against the reference
/// bearing.
fn calculate_twist(reference: Ref2D, obj: Ref2D, angle: i32) -> i32 {
    let delta = [reference.pos[0] - obj.pos[0], reference.pos[1] - obj.pos[1]];
    let mut facing = 0;
    if let Some((delta, dir)) = unit_pair(delta, obj.dir) {
        facing = turn_fraction(delta, dir);
        let cross = delta[0] * dir[1] - delta[1] * dir[0];
        if 0.0 > cross {
            facing = 0xFFFF - facing;
        }
    }
    let mut twist = if facing > angle { facing - angle } else { angle - facing };
    if twist < 0 {
        twist += 0x10000;
    }
    twist
}
