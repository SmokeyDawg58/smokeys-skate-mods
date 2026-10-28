//! The TRAFFICCAR state (`eMAINMAPSTATES` 4): `CSTATEMGR_TrafficCar`, `CSTATE_TrafficCar`, the
//! `SFXCTL_TrafficCarPhysics` controller with its three `SFXCTL_3DObjPos` positions (car, engine,
//! exhaust) and the `SFXObj_TrafficEngine`, `SFXObj_TrafficSkids`, `SFXObj_TrafficHorn` and
//! `SFXObj_TrafficWoosh` objects. `EnterGamePlay` 0x8254ff90 makes four states (`Setup(15)`:
//! objects 0-3 in id order, controllers sorted by index); after four seconds of updates they
//! follow the first four cars `AudioPresentation::DoSimSubmit` 0x827fb2f8 found around the camera.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::traffic_engine::TrafficEngine;
use super::traffic_horn::TrafficHorn;
use super::traffic_skids::TrafficSkids;
use super::whoosh::WhooshManager;
use crate::nicotine::shape::fctiwz;
use crate::nicotine::{DMixIoId, MixMap};
use crate::player::pos3d::{MicrophoneMan, Obj3d, ObjRefs, Pos3dShared};
use crate::player::SfxCtx;
use crate::system::WorldAudioState;
use crate::vault::{class, Collection, Vault};
use crate::vmx::{self, Vec3, ZERO};

/// `AudioPresentation::VehicleData` (32 bytes) of one car `DoSimSubmit` finds with
/// `LWVehicleMan::GetVehiclesWithinRange` (the camera's x and z, 40 m, at most ten), in the
/// query's order, with the matrix `SFXCTL_TrafficCarPhysics::UpdateParams` reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct VehicleData {
    /// `tVehicleStateOut::mAcceleration` and `mSpeed`.
    pub acceleration: f32,
    pub speed: f32,
    /// The range query's distance.
    pub distance_to_camera: f32,
    /// `tVehicleStateOut::GetID`.
    pub vehicle_id: u32,
    /// `mHornState` (`eHornState`); `DoSimSubmit` keeps its low four bits (`insrwi`).
    pub horn_state: i32,
    /// `tVehicleStateOut::IsSkidding`.
    pub is_skidding: bool,
    /// `tVehicleStateOut::GetEngineKey` 0x82d1e6d8: the `aud_traffic_engine` collection key at byte
    /// 8 of the car's `livingworld_vehicle_characteristics` layout.
    pub engine_key: u64,
    /// `cLivingWorldPresEntityManager::GetPresVehicle(vehicleID)->GetLocalToWorldMatrix()` rows;
    /// `None` while `GetPresVehicle` returns null.
    pub matrix: Option<[Vec3; 4]>,
}

/// `aud_general` 0x3213338D2D2C817F: `max_relative_velocity_vehicle` and the relative speed's
/// largest change per second.
const GENERAL_VEHICLE: Collection = Collection::new(class::AUD_GENERAL, 0x3213_338D_2D2C_817F);
const MAX_RELATIVE_VELOCITY_VEHICLE: u64 = 0x11FA_A9AA_DDC7_8EC0;
const RELATIVE_VELOCITY_RATE: u64 = 0xAB85_397C_101B_0752;
/// `GlobalAttributeCollection::aud_whooshby_vehicle` (`aud_whooshby` "vehicle"): its first layout
/// float is the whoosh distance, element 2 of 0x79B8671F26F9A0DD the lowest relative speed.
const WHOOSHBY_VEHICLE: Collection = Collection::new(0x4F32_993D_1936_028A, 0x39FC_10A1_DEBD_34F8);
const WHOOSHBY_SPEEDS: u64 = 0x79B8_671F_26F9_A0DD;
const Q15: f32 = f32::from_bits(0x46ff_fe00);

/// `SFXCTL_TrafficCarPhysics` (192 bytes, TRAFFICCAR controller 0, ctor 0x82510d58): the car
/// the state follows, as the objects and the position controllers read it.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrafficCarPhysics {
    pub io: Option<DMixIoId>,
    pub car_pos: Vec3,
    pub prev_car_pos: Vec3,
    pub eng_pos: Vec3,
    pub exh_pos: Vec3,
    pub forward: Vec3,
    pub velocity: Vec3,
    pub acceleration: f32,
    pub speed: f32,
    pub distance_to_camera: f32,
    pub horn_state: i32,
    pub is_skidding: i32,
    pub engine_key: u64,
    /// `mLastRelativeSpeed`, which only the constructor clears.
    pub last_relative_speed: f32,
}

impl TrafficCarPhysics {
    /// `InitSFX` 0x82510df8: the output on and every vector zero; the position controllers then
    /// point at `mvCarPos`, `mvEngPos` and `mvExhPos`, each with `mvForward` and `mvVelocity`.
    fn init_sfx(&mut self, map: &mut MixMap) {
        if let Some(io) = self.io {
            map.set_output_enabled(io, true);
        }
        self.velocity = ZERO;
        self.forward = ZERO;
        self.exh_pos = ZERO;
        self.eng_pos = ZERO;
        self.prev_car_pos = ZERO;
        self.car_pos = ZERO;
    }

    /// `UpdateParams` 0x82519d70: the car's matrix and `VehicleData`, then input 0, the speed
    /// relative to the camera microphone (at most `max_relative_velocity_vehicle`, changing by at
    /// most the rate times `t`) as a fraction of that maximum. Nothing changes without the car or
    /// its presentation entity.
    fn update_params(&mut self, t: f32, vehicle: Option<&VehicleData>, mics: &MicrophoneMan, vault: &Vault, map: &mut MixMap) {
        let Some(vehicle) = vehicle else { return };
        let Some(matrix) = vehicle.matrix else { return };
        self.prev_car_pos = self.car_pos;
        self.forward = vmx::normalize_unguarded(matrix[2]);
        self.car_pos = matrix[3];
        self.eng_pos = std::array::from_fn(|i| self.forward[i].mul_add(1.0, self.car_pos[i]));
        self.exh_pos = std::array::from_fn(|i| self.forward[i].mul_add(-1.0, self.car_pos[i]));
        self.velocity = vmx::scale(self.forward, vehicle.speed);
        self.speed = vehicle.speed;
        self.acceleration = vehicle.acceleration;
        self.distance_to_camera = vehicle.distance_to_camera;
        self.horn_state = (vehicle.horn_state << 28) >> 28;
        self.is_skidding = i32::from(vehicle.is_skidding);
        self.engine_key = vehicle.engine_key;
        let mut relative = vmx::length(vmx::sub(self.velocity, mics.microphones[0].velocity));
        let maximum = vault.f32(GENERAL_VEHICLE, MAX_RELATIVE_VELOCITY_VEHICLE);
        if relative > maximum {
            relative = maximum;
        }
        let step = vault.f32(GENERAL_VEHICLE, RELATIVE_VELOCITY_RATE) * t;
        let last = self.last_relative_speed;
        if relative > last {
            if relative - last > step {
                relative = last + step;
            }
        } else if relative < last && last - relative > step {
            relative = last - step;
        }
        self.last_relative_speed = relative;
        if let Some(io) = self.io {
            map.set_input(io, 0, fctiwz(relative / maximum * Q15).clamp(0, 0x7FFF));
        }
    }

    /// A position controller's `Ref3D` pointers: `pos`, `mvForward` and `mvVelocity`.
    fn refs(&self, pos: Vec3) -> ObjRefs {
        ObjRefs { dir: Some(self.forward), pos: Some(pos), vel: Some(self.velocity) }
    }
}

/// `SFXObj_TrafficWoosh` (44 bytes, TRAFFICCAR object 3, TypeInfo 0x00040030, ctor 0x825164f8).
struct TrafficWoosh {
    io: Option<DMixIoId>,
    /// `mPlayedWhoosh`, which outlives a detach.
    played_whoosh: bool,
}

impl TrafficWoosh {
    /// `UpdateParams` 0x825456e8 (attached states), `CheckForWhooshes` 0x8252c868: within the
    /// whoosh distance of the camera microphone and at least the lowest relative speed,
    /// `PlayWhooshSound` 0x824f0b30 (`aud_whooshby` "vehicle", the car's position) until one
    /// attaches a WHOOSH state, then nothing until the car leaves that distance again.
    fn check_for_whooshes(&mut self, physics: &TrafficCarPhysics, mics: &MicrophoneMan, vault: &Vault, whooshes: Option<&mut WhooshManager>) {
        let camera = mics.microphones[0];
        let relative_speed = vmx::length(vmx::sub(camera.velocity, physics.velocity));
        let distance = vmx::length(vmx::sub(camera.cur_position, physics.car_pos));
        let radius = vault.layout_f32(WHOOSHBY_VEHICLE, 0);
        if self.played_whoosh {
            self.played_whoosh = distance < radius;
            return;
        }
        if distance > radius || relative_speed < vault.f32_at(WHOOSHBY_VEHICLE, WHOOSHBY_SPEEDS, 2) {
            return;
        }
        if whooshes.is_some_and(|whooshes| whooshes.play(relative_speed, WHOOSHBY_VEHICLE.key, physics.car_pos)) {
            self.played_whoosh = true;
        }
    }
}

/// One `CSTATE_TrafficCar` (0x44 bytes) with its controllers and objects in list order.
struct TrafficState {
    attached: bool,
    /// `mTrafficCarStateID`.
    vehicle_id: u32,
    /// `t_DeltaTime`.
    delta_time: f32,
    physics: TrafficCarPhysics,
    /// `SFXCTL_3DTrafficCarPos`, `SFXCTL_3DTrafficEngPos`, `SFXCTL_3DTrafficExhPos` (controllers
    /// 1-3, plain `SFXCTL_3DObjPos`).
    positions: [Obj3d; 3],
    engine: TrafficEngine,
    skids: TrafficSkids,
    horn: TrafficHorn,
    woosh: TrafficWoosh,
}

impl TrafficState {
    /// `CSTATE_TrafficCar::Attach` 0x825516d0 (`CSTATE_Base::Attach` 0x82a7e2d0): the controllers'
    /// `InitSFX` (0x82515d80 for the positions: the output on), then `LoadData`, whose
    /// `SetupLoadData` is each object's `InitSFX`.
    fn attach(&mut self, vehicle_id: u32, map: &mut MixMap) {
        self.vehicle_id = vehicle_id;
        self.attached = true;
        self.physics.init_sfx(map);
        let outputs = [self.positions[0].io, self.positions[1].io, self.positions[2].io, self.engine.io, self.skids.io, self.horn.io, self.woosh.io];
        for io in outputs.into_iter().flatten() {
            map.set_output_enabled(io, true);
        }
    }

    /// `CSTATE_TrafficCar::Detach` 0x825516e0 (`CSTATE_Base::Detach` 0x82a7e350): each object's
    /// `Detach` (its `Destroy`) with its output off and inputs cleared, then each controller's
    /// `Detach` with its inputs cleared.
    fn detach(&mut self, ctx: &mut SfxCtx<'_>) {
        self.attached = false;
        self.engine.destroy(ctx);
        disconnect(self.engine.io, ctx.map);
        self.skids.destroy(ctx);
        disconnect(self.skids.io, ctx.map);
        self.horn.destroy(ctx);
        disconnect(self.horn.io, ctx.map);
        disconnect(self.woosh.io, ctx.map);
        if let Some(io) = self.physics.io {
            ctx.map.clear_input(io);
        }
        for position in &mut self.positions {
            position.detach();
            if let Some(io) = position.io {
                ctx.map.clear_input(io);
            }
        }
    }
}

/// An object's output off and its inputs cleared.
fn disconnect(io: Option<DMixIoId>, map: &mut MixMap) {
    if let Some(io) = io {
        map.set_output_enabled(io, false);
        map.clear_input(io);
    }
}

/// `CSTATEMGR_TrafficCar::FindVehicleByID` 0x82552d58: the car among the first
/// `mNumVehiclesAround`.
fn find_vehicle_by_id(vehicles: &[VehicleData], around: i32, id: u32) -> Option<&VehicleData> {
    vehicles.iter().take(usize::try_from(around).unwrap_or(0)).find(|vehicle| vehicle.vehicle_id == id)
}

/// `CSTATEMGR_TrafficCar` (36 bytes, `CreateStateMan` 0x82552c40).
pub struct TrafficManager {
    states: Vec<TrafficState>,
    /// `mNumVehiclesAround`.
    num_vehicles_around: i32,
    /// `mLoadBuffer`: the update time since `EnterGamePlay`.
    load_buffer: f32,
}

impl TrafficManager {
    /// `CSTATEMGR_TrafficCar::EnterGamePlay` 0x8254ff90: four states.
    #[must_use]
    pub fn new(vault: &Vault, map: Option<&MixMap>) -> Self {
        let states = (0..4u32)
            .map(|instance| {
                let io = |kind: u32, sfx: u32| map.and_then(|map| map.dmixio(kind | (4 << 16) | (instance << 11) | (sfx << 4)));
                let position = |sfx: u32| {
                    let mut position = Obj3d::new(vault);
                    position.io = io(0x6000_0000, sfx);
                    position
                };
                TrafficState {
                    attached: false,
                    vehicle_id: 0,
                    delta_time: 0.0,
                    physics: TrafficCarPhysics { io: io(0x6000_0000, 0), ..TrafficCarPhysics::default() },
                    positions: [position(1), position(2), position(3)],
                    engine: TrafficEngine::new(io(0x4000_0000, 0)),
                    skids: TrafficSkids::new(io(0x4000_0000, 1)),
                    horn: TrafficHorn::new(io(0x4000_0000, 2)),
                    woosh: TrafficWoosh { io: io(0x4000_0000, 3), played_whoosh: false },
                }
            })
            .collect();
        Self { states, num_vehicles_around: 0, load_buffer: 0.0 }
    }

    /// `CSTATEMGR_TrafficCar::UpdateParams` 0x82552df0: from four seconds on, the states whose car
    /// is no longer among the first `IntClamp(mVehicleCount, 0, 4)` detach and the new cars take
    /// free states (`GetFreeState`: the first detached), then `CSTATEMGR_Base::UpdateParams`: every
    /// state's `t_DeltaTime`, and for attached ones `SFXCTL_TrafficCarPhysics`, the positions and
    /// the objects' `UpdateParams`.
    pub fn update_params(&mut self, t: f32, world: &WorldAudioState, mics: &MicrophoneMan, shared: &mut Pos3dShared, mut whooshes: Option<&mut WhooshManager>, ctx: &mut SfxCtx<'_>) {
        self.load_buffer = t + self.load_buffer;
        if self.load_buffer < 4.0 {
            return;
        }
        self.num_vehicles_around = i32::try_from(world.vehicles.len()).unwrap_or(i32::MAX);
        let count = self.num_vehicles_around.clamp(0, 4) as usize;
        let mut ids = [0u32; 4];
        for (id, vehicle) in ids.iter_mut().zip(&world.vehicles).take(count) {
            *id = vehicle.vehicle_id;
        }
        for state in &mut self.states {
            if !state.attached {
                continue;
            }
            match ids[..count].iter_mut().find(|id| **id == state.vehicle_id) {
                Some(id) => *id = 0,
                None => state.detach(ctx),
            }
        }
        if self.num_vehicles_around > 0 {
            for id in ids[..count].iter().copied().filter(|&id| id != 0) {
                if let Some(state) = self.states.iter_mut().find(|state| !state.attached) {
                    state.attach(id, ctx.map);
                }
            }
        }
        for state in &mut self.states {
            state.delta_time = t;
            if !state.attached {
                continue;
            }
            let vehicle = find_vehicle_by_id(&world.vehicles, self.num_vehicles_around, state.vehicle_id);
            state.physics.update_params(t, vehicle, mics, ctx.vault, ctx.map);
            let targets = [state.physics.car_pos, state.physics.eng_pos, state.physics.exh_pos];
            for (position, target) in state.positions.iter_mut().zip(targets) {
                position.update_params(t, state.physics.refs(target), mics, shared, ctx.map);
            }
            state.engine.update_params(&state.physics, ctx);
            state.skids.update_params(ctx);
            state.horn.update_params(&state.physics, ctx);
            state.woosh.check_for_whooshes(&state.physics, mics, ctx.vault, whooshes.as_deref_mut());
        }
    }

    /// `CSTATEMGR_Base::ProcessUpdate` over every state: the objects' `ProcessUpdate` in list
    /// order (`SFXObj_TrafficWoosh` has none). A detached state's objects hold no instances.
    pub fn process_update(&mut self, mics: &MicrophoneMan, ctx: &mut SfxCtx<'_>) {
        for state in &mut self.states {
            state.engine.process_update(&state.physics, state.delta_time, mics, ctx);
            state.skids.process_update(&state.physics, ctx);
            state.horn.process_update(&state.physics, mics, ctx);
        }
    }
}
