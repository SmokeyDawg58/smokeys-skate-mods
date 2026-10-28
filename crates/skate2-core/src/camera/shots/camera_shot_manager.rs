//! `ShotManager2` constructor82e3d308: three principal and 64 blend shots.
#[path = "camera_shot_blend_selection.rs"]
pub(crate) mod blend_selection;
#[path = "camera_shot_blend_value.rs"]
pub(crate) mod blend_value;
#[path = "camera_shot_field_interpolation.rs"]
pub(crate) mod field_interpolation;
#[path = "camera_shot_runtime.rs"]
pub(crate) mod runtime;
#[path = "camera_shot_runtime_math.rs"]
pub(crate) mod runtime_math;

#[path = "camera_shot_attributes.rs"]
pub(crate) mod attributes;
#[path = "camera_shot_selection.rs"]
pub(crate) mod selection;

use crate::{attribute_runtime::AttributeRuntimeVault, camera_shot::CameraShot};
use std::sync::{Arc, Mutex};

pub(crate) struct CameraShotDatabase {
    /// Same session attribute service used by the later native shot lookup.
    pub(crate) vault: Arc<AttributeRuntimeVault>,
    pub(crate) changed: bool,
}
impl CameraShotDatabase {
    /// `AttribShotDatabase82d61788` publishes only the changed flag.
    pub(crate) fn constructed(vault: Arc<AttributeRuntimeVault>) -> Self {
        Self {
            vault,
            changed: false,
        }
    }
}
pub(crate) struct CameraShotManager {
    pub(crate) choices: [[Option<u8>; 32]; 10],
    pub(crate) interpolated: CameraShot,
    pub(crate) current: CameraShot,
    pub(crate) previous: CameraShot,
    pub(crate) blends: Vec<Arc<Mutex<CameraShot>>>,
    pub(crate) database: Arc<Mutex<CameraShotDatabase>>,
    /// The manager remains an embedded child of its actual `CameraMan`. Later
    /// operations receive that owning `CameraMan` instead of a cyclic strong Arc.
    pub(crate) choice_count: u32,
    pub(crate) active_blend_count: Option<u32>,
    pub(crate) time_since_transition: f32,
    pub(crate) shot_distance: f32,
    pub(crate) transition_out_time: f32,
    pub(crate) instant_transition: bool,
}
impl CameraShotManager {
    pub(crate) fn try_construct(database: Arc<Mutex<CameraShotDatabase>>) -> Result<Self, ()> {
        let interpolated = CameraShot::constructed();
        let current = CameraShot::constructed();
        let previous = CameraShot::constructed();
        let mut blends = Vec::new();
        blends.try_reserve_exact(64).map_err(|_| ())?;
        // Host sharing wraps the SAME native64 slots, not extra native shots.
        blends.resize_with(64, || Arc::new(Mutex::new(CameraShot::constructed())));
        Ok(Self {
            choices: [[None; 32]; 10],
            interpolated,
            current,
            previous,
            blends,
            database,
            choice_count: 0,
            active_blend_count: None,
            time_since_transition: 0.0,
            shot_distance: 0.0,
            transition_out_time: -1.0,
            instant_transition: false,
        })
    }
    /// The shot-manager suffix of `CameraMan.Teleport82e45524..556c`.
    pub(crate) fn teleport(&mut self) {
        let units = self.current.transition_units.native_value();
        self.transition_out_time = -1.0;
        self.time_since_transition = match units {
            1 => self.current.transition_time * f32::from_bits(0x3c88_8889),
            2 => self.shot_distance / self.current.transition_time,
            _ => self.current.transition_time,
        };
    }
}
