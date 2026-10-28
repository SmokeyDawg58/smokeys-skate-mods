//! One selected HiLOD+0x2bc occurrence, shared by board+0x104 and Skeleton.
//! Storage/selected construction does not execute omitted `HiLOD` constructors.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "state updates await actual processed-input and player state scheduling"
    )
)]
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    reckoning::{Reckoning, ReckoningCvars, ReckoningError, ReckoningGroundInput},
    reckoning_attributes::{ReckoningAttributeError, ReckoningVaultAttributes},
    reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use std::sync::{Arc, Mutex};
pub(crate) struct PhysicalPlayerReckoningGroundSample {
    pub(crate) input: ReckoningGroundInput,
    pub(crate) ground: [f32; 4],
    pub(crate) dynamic: [f32; 4],
}
/// Host completion occurrence, not a contact-frame ID: state precedes solve.
#[derive(Clone)]
pub(crate) struct PhysicalPlayerReckoningPhase(Arc<()>);
#[derive(Debug)]
pub(crate) enum PhysicalPlayerReckoningConstructError {
    AlreadyAttempted,
    Attribute(ReckoningAttributeError),
}
#[derive(Debug)]
pub(crate) enum PhysicalPlayerReckoningUpdateError<S, C, N> {
    Uninitialized,
    ForeignProcessed,
    PoisonedProcessed,
    Source(S),
    Update(ReckoningError<ReckoningAttributeError, C, N>),
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PhysicalPlayerReckoningStalePhase;
pub(crate) struct PhysicalPlayerReckoningOwner {
    processed: Arc<Mutex<SkeletonProcessedContact>>,
    construction_attempted: bool,
    reckoning: Option<Reckoning<ReckoningVaultAttributes>>,
    completed: Option<PhysicalPlayerReckoningPhase>,
}
impl PhysicalPlayerReckoningOwner {
    pub(crate) fn new() -> Self {
        Self {
            processed: Arc::new(Mutex::new(SkeletonProcessedContact::new())),
            construction_attempted: false,
            reckoning: None,
            completed: None,
        }
    }
    /// Same selected processed storage, not a claim of full `ProcessedPhysIn` ctor.
    pub(crate) fn processed(&self) -> &Arc<Mutex<SkeletonProcessedContact>> {
        &self.processed
    }
    /// Constructor/raw inspection only. Use `try_completed` for completed-phase reads.
    pub(crate) fn reckoning(&self) -> Option<&Reckoning<ReckoningVaultAttributes>> {
        self.reckoning.as_ref()
    }
    /// `HiLOD`3ff8 selected successful ctor projection. Failed host PID/layout reads
    /// consume this attempt; discarded partial PID internals are not claimed retained.
    pub(crate) fn try_construct(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
    ) -> Result<(), PhysicalPlayerReckoningConstructError> {
        if self.construction_attempted {
            return Err(PhysicalPlayerReckoningConstructError::AlreadyAttempted);
        }
        self.construction_attempted = true;
        self.reckoning = Some(
            Reckoning::try_new(ReckoningVaultAttributes::bind(vault))
                .map_err(PhysicalPlayerReckoningConstructError::Attribute)?,
        );
        Ok(())
    }
    pub(crate) fn try_completed(
        &self,
        phase: &PhysicalPlayerReckoningPhase,
    ) -> Result<&Reckoning<ReckoningVaultAttributes>, PhysicalPlayerReckoningStalePhase> {
        if self
            .completed
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(&current.0, &phase.0))
        {
            self.reckoning
                .as_ref()
                .ok_or(PhysicalPlayerReckoningStalePhase)
        } else {
            Err(PhysicalPlayerReckoningStalePhase)
        }
    }
    /// Selective native Reset82dd3c50; not `ResetSystems` or placement.
    pub(crate) fn try_reset(&mut self) -> Result<(), PhysicalPlayerReckoningStalePhase> {
        self.completed = None;
        self.reckoning
            .as_mut()
            .ok_or(PhysicalPlayerReckoningStalePhase)?
            .reset();
        Ok(())
    }
    /// SAME Reckoning `ResetBodySpin` 0x82dd4068, independent of update readiness.
    pub(crate) fn try_reset_flipping(&mut self) -> Result<(), PhysicalPlayerReckoningStalePhase> {
        self.completed = None;
        self.reckoning
            .as_mut()
            .ok_or(PhysicalPlayerReckoningStalePhase)?
            .reset_flipping();
        Ok(())
    }
    pub(crate) fn try_reset_body_spin(&mut self) -> Result<(), PhysicalPlayerReckoningStalePhase> {
        self.completed = None;
        self.reckoning
            .as_mut()
            .ok_or(PhysicalPlayerReckoningStalePhase)?
            .reset_body_spin();
        Ok(())
    }

    /// Wrapper82de0408 writes heading before the shared fixed-heading graph.
    pub(crate) fn try_update_grounded<S, C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        source: impl FnOnce(&SkeletonProcessedContact) -> Result<PhysicalPlayerReckoningGroundSample, S>,
        heading: [f32; 4],
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<S, C::Error, N::Error>,
    > {
        self.try_update(processed, source, Some(heading), cvars, numeric)
    }
    /// `GroundAnimation`8d94 calls `FixedHeading` directly; no wrapper heading write.
    pub(crate) fn try_update_grounded_fixed_heading<S, C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        source: impl FnOnce(&SkeletonProcessedContact) -> Result<PhysicalPlayerReckoningGroundSample, S>,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<S, C::Error, N::Error>,
    > {
        self.try_update(processed, source, None, cvars, numeric)
    }
    fn try_update<S, C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        source: impl FnOnce(&SkeletonProcessedContact) -> Result<PhysicalPlayerReckoningGroundSample, S>,
        heading: Option<[f32; 4]>,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<S, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        // Immutable selected snapshot scope is the existing Reckoning API boundary;
        // neither the callback nor successful completion proves omitted field producers.
        let sample = source(&input).map_err(E::Source)?;
        match heading {
            Some(heading) => state.try_update_grounded(
                &sample.input,
                sample.ground,
                sample.dynamic,
                heading,
                cvars,
                numeric,
            ),
            None => state.try_update_grounded_fixed_heading(
                &sample.input,
                sample.ground,
                sample.dynamic,
                cvars,
                numeric,
            ),
        }
        .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
}
impl std::fmt::Display for PhysicalPlayerReckoningConstructError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyAttempted => f.write_str("Reckoning constructor already attempted"),
            Self::Attribute(e) => write!(f, "Reckoning constructor: {e}"),
        }
    }
}
impl<S: std::fmt::Debug, C: std::fmt::Debug, N: std::fmt::Debug> std::fmt::Display
    for PhysicalPlayerReckoningUpdateError<S, C, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uninitialized => f.write_str("Reckoning not constructed"),
            Self::ForeignProcessed => f.write_str("foreign processed occurrence"),
            Self::PoisonedProcessed => f.write_str("processed lock poisoned"),
            Self::Source(e) => write!(f, "Reckoning selected source: {e:?}"),
            Self::Update(e) => write!(f, "Reckoning update: {e:?}"),
        }
    }
}
#[cfg(test)]
#[path = "physical_player_reckoning_owner_tests.rs"]
mod tests;

#[path = "physical_player_reckoning_live.rs"]
mod live;

#[path = "physical_player_reckoning_air_live.rs"]
pub(crate) mod air_live;
