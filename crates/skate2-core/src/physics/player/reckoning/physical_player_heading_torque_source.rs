//! Unregistered actual Heading torque reads; no outer Reckoning lock is held.
use crate::{
    attribute_runtime::AttributeRuntimeError,
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physics::PointGraph,
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
    skateboard_force_runtime::entry::heading_torque::{
        HeadingGlobal, HeadingScalar, HeadingTorqueSource, HeadingVector,
    },
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum HeadingRuntimeError {
    Missing(&'static str),
    Poisoned(&'static str),
    BoardAttribute(SkateboardAttributeError),
    HeadingAttribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
}
impl std::fmt::Display for HeadingRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "heading missing {s}"),
            Self::Poisoned(s) => write!(f, "heading {s} lock poisoned"),
            Self::BoardAttribute(e) => e.fmt(f),
            Self::HeadingAttribute(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
        }
    }
}
pub(crate) struct HeadingRuntime<'a> {
    pub(crate) owner: &'a PhysicalPlayerAssemblyOwner,
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) board_attributes: &'a mut SkateboardAttributes,
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl HeadingRuntime<'_> {
    fn read<T>(
        &self,
        field: &'static str,
        get: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, HeadingRuntimeError> {
        let p = self
            .processed
            .lock()
            .map_err(|_| HeadingRuntimeError::Poisoned("Processed"))?;
        get(&p).ok_or(HeadingRuntimeError::Missing(field))
    }
}
impl HeadingTorqueSource for HeadingRuntime<'_> {
    type Error = HeadingRuntimeError;
    type Graph = PointGraph<8>;
    fn scalar(&mut self, field: HeadingScalar) -> Result<f32, Self::Error> {
        self.read("source scalar", |p| match field {
            HeadingScalar::Balance => p.physics_state_fields().balance,
            HeadingScalar::ManualTurn => p.jump_events().spin(),
            HeadingScalar::DeckSpeed => p.prior_output().deck_speed().map(|v| v.value),
            HeadingScalar::TimeDelta => p.physics_state_fields().time_delta,
            HeadingScalar::ForwardSpeed => p.prior_output().z_speed().map(|v| v.value),
            HeadingScalar::VertTurn => p.animation_attribute_fields().raw_turn,
            HeadingScalar::HeadingFactor => p.physics_state_fields().heading_adjust_factor,
        })
    }
    fn vector(&mut self, field: HeadingVector) -> Result<SkeletonRootVector, Self::Error> {
        self.read("source vector", |p| match field {
            HeadingVector::GroundNormal => p.prior_output().ground_normal().map(|v| v.value),
            HeadingVector::ZAxisVelocity => p.toolkit_state().z_axis_velocity(),
            HeadingVector::AngularVelocity => p.prior_output().angular_velocity().map(|v| v.value),
        })
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.read("flags8c8", SkeletonProcessedContact::known_flags_8c8)
    }
    fn attribute(&mut self, global: HeadingGlobal, key: u64) -> Result<f32, Self::Error> {
        match global {
            HeadingGlobal::Manual => self
                .board_attributes
                .read_float(SkateboardAttributeClass::Manual, key)
                .map_err(Self::Error::BoardAttribute),
            HeadingGlobal::Heading => self
                .owner
                .heading_attributes()
                .torque_scalar(key)
                .map_err(Self::Error::HeadingAttribute),
        }
    }
    fn capture_graph(
        &mut self,
        global: HeadingGlobal,
        offset: usize,
    ) -> Result<Self::Graph, Self::Error> {
        match global {
            HeadingGlobal::Manual => self
                .board_attributes
                .capture_heading_manual_graph()
                .map_err(Self::Error::BoardAttribute),
            HeadingGlobal::Heading => self
                .owner
                .heading_attributes()
                .capture_torque_graph::<8>(offset)
                .map_err(Self::Error::HeadingAttribute),
        }
    }
    fn evaluate_graph(&mut self, graph: &Self::Graph, input: f32) -> Result<f32, Self::Error> {
        Ok(graph.evaluate(input))
    }
    fn ground_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let reckoning = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::Poisoned("Reckoning"))?;
        Ok(*reckoning
            .reckoning()
            .ok_or(Self::Error::Missing("actual Reckoning"))?
            .ground_transform())
    }
    fn draw_vert_turning(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_draw_vert_turning()
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
