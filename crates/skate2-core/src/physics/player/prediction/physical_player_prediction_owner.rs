//! Actual World-owned Predictor constructor82e4b6c8, published827b1174/11a8.
#![expect(
    dead_code,
    reason = "retained predictor awaits complete Air launch and frame scheduling"
)]
use super::SimulationAssemblyConstraintSession;
use crate::{
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimePhysics, AttributeRuntimeVault},
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
};
use std::sync::{Arc, Mutex};

#[path = "physical_player_predictor_runtime.rs"]
pub(crate) mod runtime;

/// Fixed native capacity90; no request exists before actual source submission.
pub(crate) struct PredictorQueue {
    slots: [Option<runtime::PendingPredictionQuery>; 90],
    length: usize,
}
pub(crate) struct PhysicalPredictionOwner {
    manager: SimulationManagerContactAuthority,
    physics_attributes: AttributeRuntimePhysics,
    pub(crate) gravity: [f32; 4],
    queue: Mutex<PredictorQueue>,
}
impl PhysicalPredictionOwner {
    pub(crate) fn belongs_to(&self, manager: SimulationManagerContactAuthority) -> bool {
        self.manager == manager
    }
    pub(crate) fn pending_count(&self) -> Result<usize, PredictionInitError> {
        Ok(self
            .queue
            .lock()
            .map_err(|_| PredictionInitError::Poisoned)?
            .length)
    }
}
#[derive(Debug)]
pub(crate) enum PredictionInitError {
    AlreadyAttempted,
    Attributes(AttributeRuntimeError),
    Poisoned,
}
impl std::fmt::Display for PredictionInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("World predictor construction already attempted"),
            Self::Attributes(e) => write!(f, "World predictor gravity: {e}"),
            Self::Poisoned => f.write_str("World predictor query lock poisoned"),
        }
    }
}
#[derive(Default)]
pub(crate) struct PhysicalPredictionSlot {
    attempted: bool,
    pub(crate) value: Option<Arc<PhysicalPredictionOwner>>,
}
impl<M, C> SimulationAssemblyConstraintSession<M, C> {
    /// `World.Init827b114c` allocation is distinct from each skater constructor.
    /// A null allocation publishes null; successful allocation reads the actual
    /// retained global physics layout then constructs before publication.
    pub(crate) fn try_initialize_prediction_owner(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        allocated: bool,
    ) -> Result<(), PredictionInitError> {
        if self.predictor.attempted {
            return Err(PredictionInitError::AlreadyAttempted);
        }
        self.predictor.attempted = true;
        if !allocated {
            return Ok(());
        }
        let physics_attributes = vault.physics(0xd7ed_bd36_2d7d_2152);
        let layout = physics_attributes
            .try_physics_layout()
            .map_err(PredictionInitError::Attributes)?;
        let gravity = std::array::from_fn(|lane| {
            f32::from_be_bytes(
                layout.raw_bytes[lane * 4..lane * 4 + 4]
                    .try_into()
                    .expect("four exact gravity lanes"),
            )
        });
        self.predictor.value = Some(Arc::new(PhysicalPredictionOwner {
            manager: self.contacts.assembly_pool_authority(),
            physics_attributes,
            gravity,
            queue: Mutex::new(PredictorQueue {
                slots: std::array::from_fn(|_| None),
                length: 0,
            }),
        }));
        Ok(())
    }
    pub(crate) fn prediction_owner(&self) -> Option<&Arc<PhysicalPredictionOwner>> {
        self.predictor.value.as_ref()
    }
}
