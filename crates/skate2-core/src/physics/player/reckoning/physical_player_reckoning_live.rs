//! Ground's live processed reader uses the SAME retained Reckoning and filters.
use super::{
    PhysicalPlayerReckoningOwner, PhysicalPlayerReckoningPhase, PhysicalPlayerReckoningUpdateError,
};
use crate::{
    reckoning::ReckoningCvars, reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
impl PhysicalPlayerReckoningOwner {
    /// Skeleton::UpdateOffBoard uses the same retained Reckoning and
    /// Processed owner for UpdateGrindStates after NextTrajectory reset.
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn try_update_grind_live<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        normal: SkeletonRootVector,
        heading: SkeletonRootVector,
        smoothing: f32,
        in_air: bool,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<std::convert::Infallible, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_grind_states(&*input, normal, heading, smoothing, in_air, cvars, numeric)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
    pub(crate) fn try_update_grounded_reset_systems<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        heading: SkeletonRootVector,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<std::convert::Infallible, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_grounded_reset_systems(&*input, heading, cvars, numeric)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }

    pub(crate) fn try_update_grounded_live<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<std::convert::Infallible, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_grounded_live(&*input, cvars, numeric)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
    /// FollowPath's `UpdateGroundStates`: gJ for the ground.
    pub(crate) fn try_update_grounded_follow_path<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<std::convert::Infallible, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_grounded_follow_path(&*input, cvars, numeric)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
    pub(crate) fn try_update_grounded_fixed_heading_live<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        cvars: &mut C,
        numeric: &mut N,
    ) -> Result<
        PhysicalPlayerReckoningPhase,
        PhysicalPlayerReckoningUpdateError<std::convert::Infallible, C::Error, N::Error>,
    > {
        use PhysicalPlayerReckoningUpdateError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_grounded_fixed_heading_read(&*input, &*input, cvars, numeric)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
}
