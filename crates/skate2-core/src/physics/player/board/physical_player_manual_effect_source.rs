//! Unregistered live manual adapter. Caller verifies the same board/owner pair.
use crate::{
    debug_overlay::{DebugOverlayError, DebugOverlaySlot},
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalPlayerAssemblyOwner,
    physics::PointGraph,
    skateboard_assembly::{SkateboardAssemblySession, SkateboardSimulationBackend},
    skateboard_attributes::{
        SkateboardAttributeClass, SkateboardAttributeError, SkateboardAttributeRead,
        SkateboardAttributes,
    },
    skateboard_force_runtime::entry::manual_effect::{ManualEffectSource, ManualScalar},
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootMatrix,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum ManualRuntimeError {
    Missing(&'static str),
    Poisoned(&'static str),
    Attribute(SkateboardAttributeError),
    Cvar(InputCvarError),
    Overlay(DebugOverlayError),
    Transform(crate::dynamic_assembly_session::part_transform::DynamicAssemblyPartTransformError),
}
impl std::fmt::Display for ManualRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(s) => write!(f, "manual missing {s}"),
            Self::Poisoned(s) => write!(f, "manual {s} lock poisoned"),
            Self::Attribute(e) => e.fmt(f),
            Self::Cvar(e) => e.fmt(f),
            Self::Overlay(e) => e.fmt(f),
            Self::Transform(e) => e.fmt(f),
        }
    }
}
pub(crate) struct ManualRuntime<'a> {
    pub(crate) owner: &'a PhysicalPlayerAssemblyOwner,
    pub(crate) processed: &'a Arc<Mutex<SkeletonProcessedContact>>,
    pub(crate) board: &'a SkateboardAssemblySession,
    pub(crate) backend: &'a SkateboardSimulationBackend,
    pub(crate) attributes: &'a mut SkateboardAttributes,
    pub(crate) references: &'a InputCvarReferences,
    pub(crate) overlay: &'a DebugOverlaySlot,
}
impl ManualRuntime<'_> {
    fn read<T>(
        &self,
        name: &'static str,
        get: impl FnOnce(&SkeletonProcessedContact) -> Option<T>,
    ) -> Result<T, ManualRuntimeError> {
        let p = self
            .processed
            .lock()
            .map_err(|_| ManualRuntimeError::Poisoned("Processed"))?;
        get(&p).ok_or(ManualRuntimeError::Missing(name))
    }
}
impl ManualEffectSource for ManualRuntime<'_> {
    type Error = ManualRuntimeError;
    type Graph = PointGraph<8>;
    fn scalar(&mut self, field: ManualScalar) -> Result<f32, Self::Error> {
        self.read("source scalar", |p| match field {
            ManualScalar::FlippedControls => p.toolkit_state().flipped_controls_scalar(),
            ManualScalar::Balance => p.physics_state_fields().balance,
            ManualScalar::DeckZyDelta => p.process_data_output().deck_zy_delta,
            ManualScalar::StateTimer => p.physics_state_fields().state_timer,
            ManualScalar::AbsoluteSpeed => p.toolkit_state().abs_body_speed(),
            ManualScalar::TimeDelta => p.physics_state_fields().time_delta,
        })
    }
    fn attribute(&mut self, key: u64) -> Result<f32, Self::Error> {
        self.attributes
            .read_float(SkateboardAttributeClass::Manual, key)
            .map_err(Self::Error::Attribute)
    }
    fn capture_speed_graph(&mut self) -> Result<Self::Graph, Self::Error> {
        self.attributes
            .capture_manual_effect_speed_graph()
            .map_err(Self::Error::Attribute)
    }
    fn evaluate_graph(&mut self, graph: &Self::Graph, input: f32) -> Result<f32, Self::Error> {
        Ok(graph.evaluate(input))
    }
    fn ground_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        let owner = self
            .owner
            .reckoning()
            .lock()
            .map_err(|_| Self::Error::Poisoned("Reckoning"))?;
        Ok(*owner
            .reckoning()
            .ok_or(Self::Error::Missing("actual Reckoning"))?
            .ground_transform())
    }
    fn part_six_transform(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.board
            .try_live_part_transform_typed(self.backend, 6)
            .map_err(Self::Error::Transform)
    }
    fn manual_flag(&mut self) -> Result<bool, Self::Error> {
        self.read("manual8c4/20000000", |p| {
            let (bits, known) = p.known_flags_8c4();
            (known & 0x2000_0000 != 0).then_some(bits & 0x2000_0000 != 0)
        })
    }
    fn flags_8c8(&mut self) -> Result<u32, Self::Error> {
        self.read("flags8c8", SkeletonProcessedContact::known_flags_8c8)
    }
    fn show_manual(&mut self) -> Result<bool, Self::Error> {
        self.references
            .try_show_manual_info()
            .map_err(Self::Error::Cvar)
    }
    fn wrong_wheel_scalar(&mut self) -> Result<f32, Self::Error> {
        self.references
            .try_wrong_wheel_scalar()
            .map_err(Self::Error::Cvar)
    }
    fn require_overlay(&mut self) -> Result<(), Self::Error> {
        self.overlay
            .require()
            .map(|_| ())
            .map_err(Self::Error::Overlay)
    }
}
