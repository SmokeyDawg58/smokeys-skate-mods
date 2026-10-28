//! SAME retained Reckoning/Processed completion for actual Air evolution.
use super::{PhysicalPlayerReckoningOwner, PhysicalPlayerReckoningPhase};
use crate::{
    reckoning::ReckoningCvars, reckoning_numeric::ReckoningNumeric,
    skeleton_processed_contact::SkeletonProcessedContact,
};
use crate::{
    reckoning::air::ReckoningAirError, reckoning_body_spin::air::BodySpinAirCvars,
    reckoning_sin_cos::ReckoningSinCosNumeric, skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::{Arc, Mutex};
#[derive(Debug)]
pub(crate) enum AirOwnerError<C, N, S> {
    Uninitialized,
    ForeignProcessed,
    PoisonedProcessed,
    Update(ReckoningAirError<crate::reckoning_attributes::ReckoningAttributeError, C, N, S>),
}
impl PhysicalPlayerReckoningOwner {
    pub(crate) fn try_update_handplant_live<
        C: ReckoningCvars + BodySpinAirCvars,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        normal: SkeletonRootVector,
        blend: f32,
        heading: SkeletonRootVector,
        maximum_delta: f32,
        c: &mut C,
        n: &mut N,
        s: &mut S,
    ) -> Result<PhysicalPlayerReckoningPhase, AirOwnerError<C::Error, N::Error, S::Error>> {
        use AirOwnerError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        let input = self.processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_handplant_live(&*input, normal, blend, heading, maximum_delta, c, n, s)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
    pub(crate) fn try_start_flipping<N: ReckoningNumeric>(
        &mut self,
        forward: bool,
        n: &mut N,
    ) -> Result<(), String>
    where
        N::Error: std::fmt::Debug,
    {
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or("Reckoning uninitialized")?;
        let input = self.processed.lock().map_err(|_| "Processed poisoned")?;
        state
            .try_start_flipping(&input, forward, n)
            .map_err(|e| format!("{e:?}"))
    }
    #[expect(
        clippy::too_many_arguments,
        clippy::type_complexity,
        reason = "source call operands and independent checked service errors"
    )]
    pub(crate) fn try_update_air_live<
        C: ReckoningCvars + BodySpinAirCvars,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
    >(
        &mut self,
        processed: &Arc<Mutex<SkeletonProcessedContact>>,
        normal: SkeletonRootVector,
        blend: f32,
        spin: f32,
        flip: f32,
        c: &mut C,
        n: &mut N,
        s: &mut S,
        #[cfg(feature = "skate3-runtime")]
        mod_tuning: Option<&crate::skate3_native::physics::player::tuning::Tuning>,
    ) -> Result<PhysicalPlayerReckoningPhase, AirOwnerError<C::Error, N::Error, S::Error>> {
        use AirOwnerError as E;
        self.completed = None;
        let state = self.reckoning.as_mut().ok_or(E::Uninitialized)?;
        if !Arc::ptr_eq(processed, &self.processed) {
            return Err(E::ForeignProcessed);
        }
        let input = processed.lock().map_err(|_| E::PoisonedProcessed)?;
        state
            .try_update_air_live(&*input, normal, blend, spin, flip, c, n, s,
                #[cfg(feature = "skate3-runtime")] mod_tuning)
            .map_err(E::Update)?;
        let phase = PhysicalPlayerReckoningPhase(Arc::new(()));
        self.completed = Some(phase.clone());
        Ok(phase)
    }
}
