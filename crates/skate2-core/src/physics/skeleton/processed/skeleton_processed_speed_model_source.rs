//! Unregistered Contact child: SAME live owners for the complete speed suffix.
use super::SkeletonProcessedContact;
use crate::{
    dynamic_assembly_session::skater_pool::board_entry::{
        SkateboardEntrySessionError, try_current_board_effective_transform,
    },
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_body_mass::{SkateboardBodyMassError, try_skateboard_body_mass},
    skateboard_force_runtime::entry::speed_model_update::{
        SpeedCvar, SpeedGlobal, SpeedGraph, SpeedModelSource, SpeedScalar, SpeedVector,
    },
    skateboard_slide_collision_attributes::{
        SkateboardSelectedSurface,
        speed_model::{
            SpeedAttributeError, SpeedAttributePointer, SpeedGraphPointer, SpeedModelVault,
        },
    },
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};
pub(crate) trait SpeedModelDebugRead {
    type Error;
    type Target;
    fn capture(&mut self, field: SpeedCvar) -> Result<Self::Target, Self::Error>;
    fn integer(&mut self, target: &Self::Target) -> Result<i32, Self::Error>;
    fn boolean(&mut self, target: &Self::Target) -> Result<bool, Self::Error>;
    fn float(&mut self, target: &Self::Target) -> Result<f32, Self::Error>;
    fn require_overlay(&mut self) -> Result<(), Self::Error>;
}
#[derive(Debug)]
pub(crate) enum SpeedRuntimeError<D> {
    Missing(&'static str),
    Poisoned,
    Transform(SkateboardEntrySessionError),
    Mass(SkateboardBodyMassError),
    Attribute(SpeedAttributeError),
    Debug(D),
}
impl<D: std::fmt::Debug> std::fmt::Display for SpeedRuntimeError<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "speed model missing {s}"),
            Self::Poisoned => f.write_str("speed model processed lock poisoned"),
            Self::Transform(e) => e.fmt(f),
            Self::Mass(e) => e.fmt(f),
            Self::Attribute(e) => e.fmt(f),
            Self::Debug(e) => write!(f, "speed model cvar: {e:?}"),
        }
    }
}
pub(crate) enum PlayerSpeedCvar<T> { Native(T), Boolean(bool), Float(f32) }
pub(crate) struct SpeedModelRuntime<'a, D> {
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) board: &'a SkateboardAssemblySession,
    pub(crate) backend: &'a SkateboardSimulationBackend,
    pub(crate) owner: &'a PhysicalPlayerAssemblyOwner,
    pub(crate) attributes: SpeedModelVault<'a>,
    pub(crate) debug: &'a mut D,
}
impl<D: SpeedModelDebugRead> SpeedModelRuntime<'_, D> {
    fn read<T>(
        &self,
        field: &'static str,
        get: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, SpeedRuntimeError<D::Error>> {
        let input = self
            .processed
            .lock()
            .map_err(|_| SpeedRuntimeError::Poisoned)?;
        get(&input).ok_or(SpeedRuntimeError::Missing(field))
    }
}
impl<D: SpeedModelDebugRead> SpeedModelSource for SpeedModelRuntime<'_, D> {
    type Error = SpeedRuntimeError<D::Error>;
    type Pointer = SpeedAttributePointer;
    type Surface = SkateboardSelectedSurface;
    type Graph = SpeedGraphPointer;
    type Cvar = PlayerSpeedCvar<D::Target>;
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.read("flags8c8", |p| p.flags_8c8)
    }
    fn flags_8cc(&mut self) -> Result<u32, Self::Error> {
        self.read("flags8cc", |p| p.physics_state.flags_8cc)
    }
    fn attribute(&mut self, g: SpeedGlobal, k: u64) -> Result<Self::Pointer, Self::Error> {
        self.attributes
            .capture(g, k)
            .map_err(Self::Error::Attribute)
    }
    fn value(&mut self, p: &Self::Pointer) -> Result<f32, Self::Error> {
        self.attributes.value(p).map_err(Self::Error::Attribute)
    }
    fn effective_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        try_current_board_effective_transform(self.board, self.backend, self.owner)
            .map_err(Self::Error::Transform)
    }
    fn vector(&mut self, v: SpeedVector) -> Result<SkeletonRootVector, Self::Error> {
        match v {
            SpeedVector::SurfaceVelocity => self.read("surface velocity1a0", |p| {
                p.prior_output.surface_velocity().map(|v| v.value)
            }),
            SpeedVector::GroundNormal => self.read("ground normal1d0", |p| {
                p.prior_output.ground_normal().map(|v| v.value)
            }),
            SpeedVector::BodyZAxis => self.read("body Z axis160", |p| p.toolkit.body_z()),
            SpeedVector::ProcessedEffectiveZ => self.read("processed effective Za0", |p| {
                p.effective.map(|m| m.columns()[2])
            }),
        }
    }
    fn scalar(&mut self, s: SpeedScalar) -> Result<f32, Self::Error> {
        match s {
            SpeedScalar::Balance => self.read("balance9a0", |p| p.physics_state.balance),
            SpeedScalar::DeckSpeed => self.read("deck speed968", |p| {
                p.prior_output.deck_speed().map(|v| v.value)
            }),
            SpeedScalar::Spin => self.read("spin97c", |p| p.jump_events.spin()),
            SpeedScalar::SurfaceSpeed => self.read("surface speed96c", |p| {
                p.prior_output.surface_speed().map(|v| v.value)
            }),
            SpeedScalar::TimeSinceInput => {
                self.read("time since input9bc", |p| p.controls.time_since_last_input)
            }
        }
    }
    fn frames_spinning(&mut self) -> Result<i32, Self::Error> {
        self.read("frames spinning924", |p| {
            p.prepare_output
                .count()
                .map(|v| i32::from_ne_bytes(v.value.to_ne_bytes()))
        })
    }
    fn capture_cvar(&mut self, c: SpeedCvar) -> Result<Self::Cvar, Self::Error> {
        #[cfg(feature = "skate3-runtime")]
        if let Some(tuning) = &*self.owner.skate3_tuning.lock().map_err(|_| Self::Error::Poisoned)? {
            match c { SpeedCvar::CheatEnabled => return Ok(PlayerSpeedCvar::Boolean(tuning.motor_enabled)),
                SpeedCvar::CheatSpeed => return Ok(PlayerSpeedCvar::Float(tuning.motor_speed)), _ => {} }
        }
        self.debug.capture(c).map(PlayerSpeedCvar::Native).map_err(Self::Error::Debug)
    }
    fn cvar_int(&mut self, c: &Self::Cvar) -> Result<i32, Self::Error> {
        match c { PlayerSpeedCvar::Native(value) => self.debug.integer(value).map_err(Self::Error::Debug),
            _ => Err(Self::Error::Missing("integer speed cvar type")) }
    }
    fn cvar_bool(&mut self, c: &Self::Cvar) -> Result<bool, Self::Error> {
        match c { PlayerSpeedCvar::Native(value) => self.debug.boolean(value).map_err(Self::Error::Debug),
            PlayerSpeedCvar::Boolean(value) => Ok(*value), _ => Err(Self::Error::Missing("boolean speed cvar type")) }
    }
    fn cvar_float(&mut self, c: &Self::Cvar) -> Result<f32, Self::Error> {
        match c { PlayerSpeedCvar::Native(value) => self.debug.float(value).map_err(Self::Error::Debug),
            PlayerSpeedCvar::Float(value) => Ok(*value), _ => Err(Self::Error::Missing("float speed cvar type")) }
    }
    fn capture_surface(&mut self) -> Result<Self::Surface, Self::Error> {
        self.read("surface904", |p| p.surface_projection.selected.clone())
    }
    fn surface_graph(&mut self, s: &Self::Surface, speed: f32) -> Result<f32, Self::Error> {
        s.try_speed_model_friction(speed)
            .map_err(Self::Error::Attribute)
    }
    fn capture_graph(&mut self, g: SpeedGraph) -> Result<Self::Graph, Self::Error> {
        self.attributes
            .capture_graph(g)
            .map_err(Self::Error::Attribute)
    }
    fn evaluate_graph(&mut self, g: &Self::Graph, speed: f32) -> Result<f32, Self::Error> {
        Ok(g.evaluate(speed))
    }
    fn body_mass(&mut self) -> Result<f32, Self::Error> {
        try_skateboard_body_mass(self.board, self.backend).map_err(Self::Error::Mass)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.debug.require_overlay().map_err(Self::Error::Debug)
    }
}
